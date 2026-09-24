# Knowlu C1b — sign-in: Continue with Google, the code without a password, the wizard's Back and Next, and a promotion code at checkout

**Status: PROPOSED 2026-09-17, amended the same day after the plan review (R-C1b-2, R-C1b-3, R-C1b-5).**
Written after Quinn's first install of the released `v0.1.0`
(HANDOFF §3: "the first install of the released 0.1.0 stopped at the subscribe step"). Authority
above this document: `docs/specs/2026-09-09-knowlu-cloud-design.md` and its *Amendment 2026-09-17*
(SIGNED). Where §5.1 of the cloud design says **"No social login at launch (one path to test, one
path to support)"**, this spec supersedes it — by Quinn's ruling of 2026-09-17, quoted in §0 — and
nothing else in §5.1 changes.

Stream: **C1b**, ahead of C3', C5 and C4 (Quinn, 2026-09-17: "as soon as possible", before every
other stream).

---

## 0. Why

Quinn, 2026-09-17, after installing the released build:

> "It made me type in my actual email (and a password for no reason) and then had me type in a
> verification code from the send code button anyway. So scuffed. Plus, why are there even back and
> next buttons if they don't work? … Why can't we just have a Google login button like every other
> app?"

and, on the subscribe step:

> "if it did [work], I shouldn't have to pay for my own service. Perhaps we should use a discount
> code?"

Four things, in Quinn's order of value: a Google button, an email path with no password, Back and
Next that behave, and a promotion code at checkout.

---

## 1. Decisions

**D1 — Continue with Google is the primary sign-in.** Supabase Auth's Google provider, PKCE, opened
in the system browser, returning to the app with nothing for the student to paste. The email code
stays underneath it as the second path, never as the first thing on the panel.

**D2 — The redirect is a loopback listener on `127.0.0.1`, on an ephemeral port, alive for exactly
one sign-in.** RFC 8252 §7.3's native-app pattern, and the one GoTrue's own redirect validator was
written to allow (§4). No custom URI scheme, no registry entry, no second window.

**D3 — The session lands where the email path already puts it.** `knowlu/pending/session` in
Credential Manager (`app/src/account.rs:32`, `PENDING_TARGET`), through the same `save_session`, with
the same `Session` shape. Every panel after the account panel — subscribe, vault, calendars, Google
Calendar, Finish, `move_session` at Finish — is unchanged, because it cannot tell the two paths
apart.

**D4 — The email path loses the password.** One email field, one button ("Email me a code"), the
code row, done. `POST /auth/v1/otp` with `create_user: true` both creates and signs in; `/signup`
and `/token?grant_type=password` are no longer called by anything, and `account::sign_up` /
`account::sign_in` go with them. No password is typed, stored, hashed or hinted at anywhere in
Knowlu.

**D5 — Email confirmation goes off** (`[auth.email] enable_confirmations = false`). It is what makes
D4 true rather than half-true: see the finding in §5.2. With no password in the product, the code
sent to the address *is* the proof of the address, at the only moment that proof matters.

**D6 — Consent moves out of the auth trigger and into one endpoint both paths call after sign-in.**
A Google sign-up cannot carry `age_attested` / `tos_version` / `privacy_version` in
`raw_user_meta_data`, and today's trigger raises when they are missing (§5.1). So the trigger stops
reading metadata altogether — every new user gets an `accounts` row with the four consent columns
null and no `consents` rows — and `POST /account/consent`, behind a session whose address has been
proved, is the one place a consent row is ever written, on **both** paths (amended 2026-09-17,
R-C1b-3; §13). The 18+ gate keeps a server-side tooth: `billing-checkout` refuses an account whose
`age_attested_at` is null, so a patched client still gets an account it cannot use.

**D7 — A new Google OAuth client in a new Google Cloud project, set to "In production", with
`openid email profile` and nothing else.** Basic scopes only, so no verification review and no
100-test-user cap. The existing project keeps the restricted `gmail.readonly` and
`calendar.readonly` scopes and its review track; the two never mix (§3).

**D8 — Checkout accepts promotion codes; the card is still taken up front.**
`allow_promotion_codes: true` on the Checkout Session, and `payment_method_collection` stays
`"always"` (amended 2026-09-17, R-C1b-2; §13). A 100-percent code makes the subscription free, not
card-free: `"if_required"` beside a 7-day trial would take a card from nobody at all, which is not a
relaxation of §11 R2 but its repeal (§8).

---

## 2. The flow, in sequence

`<auth>` is `account::auth_base(api_base())` — `…/functions/v1` rewritten to `…/auth/v1`
(`app/src/account.rs:71`). Nothing below adds an HTTP client: every call is the `ureq::Agent` that
`account.rs` already builds in `agent()` (line 159), compact-serialised through `post_json`
(line 184), for the reason its own comment gives.

1. **The page** calls `invoke("google_sign_in")` — one command, no arguments, on the wizard window
   and on the console window's upgrade overlay. The page never sees a URL, so
   `static_assets.rs::no_network_reference_in_the_shipped_page` stays true.
2. **Rust binds** `std::net::TcpListener::bind("127.0.0.1:0")` and reads the port back from
   `local_addr()`. The port is chosen by Windows, is different every time, and is never written down.
3. **Rust mints the verifier**: 32 bytes from `knowlu_engine::ids` entropy, base64url without
   padding (43 characters, RFC 7636 §4.1). The challenge is `base64url(sha256(verifier))` —
   `sha2 0.10` is already an `app/Cargo.toml` dependency, and the base64url alphabet is twelve lines
   of table lookup, so **no crate is added**.
4. **Rust opens the browser** at
   `<auth>/authorize?provider=google&redirect_to=http%3A%2F%2F127.0.0.1%3A<port>%2Fcallback&code_challenge=<challenge>&code_challenge_method=s256`
   through the existing `open_in_browser` (`account.rs:363`, `explorer.exe <url>`). There is no
   `flow_type` parameter: GoTrue infers the flow from the presence of `code_challenge`
   (`getFlowFromChallenge`, `internal/api/magic_link.go`), and `state` is a **reserved** parameter
   the server owns — a client that sends one has it stripped (`reservedOAuthParams`,
   `internal/api/external.go`).
5. **Google and GoTrue do their round trip.** GoTrue redirects the browser to
   `http://127.0.0.1:<port>/callback?code=<auth_code>` (`prepPKCERedirectURL`,
   `internal/api/verify.go:535` — `q.Set("code", code)`). A refusal arrives on the same URL as
   `?error=…&error_description=…&error_code=…`.
6. **The listener accepts one connection**, with a 180-second deadline, reads the request line,
   takes `code` (or `error_description`) out of the query, answers
   `200 text/html; charset=utf-8` with one sentence — *You are signed in to Knowlu. You can close
   this window.* — and drops the listener. Exactly one request is served; a second never gets a
   socket.
7. **Rust exchanges the code**: `POST <auth>/token?grant_type=pkce`, `apikey: <anon>`,
   `content-type: application/json`, body `{"auth_code": "<code>", "code_verifier": "<verifier>"}` —
   GoTrue's `PKCEGrantParams` (`internal/api/token.go:31`), those two field names and no others.
8. **Rust stores the session** with the existing `session_from` + `save_session(PENDING_TARGET, …)`,
   then calls `POST /account/consent` (§5.1) with the compiled-in `TOS_VERSION` and
   `PRIVACY_VERSION`, and answers the page `{ok, account_id, email}` — the same envelope
   `verify_email_code` returns, so `console.js` needs one new branch and no new shape.
   **`verify_email_code` makes the same call, in the same place**, because the route is the only way
   a consent row comes to exist (D6). A call that fails is logged and retried once by `open_checkout`
   before the checkout POST — safe because the route is idempotent — and a student for whom both
   attempts fail meets `billing-checkout`'s 403 rather than a silent account.

**Timeouts and cancellation.** The listener's accept deadline is 180 s; on expiry the command
answers `{ok: false, error: "the Google sign-in was not finished — try again"}` and the socket is
gone. The command is `#[tauri::command(async)]`, so the window stays live while the browser is open.
Leaving the account panel does not cancel an in-flight sign-in; it completes or times out, and the
panel reads `WIZ.accountId` when it lands, exactly as the code path does.

**What never touches the page.** The verifier, the code, the access token and the refresh token.
The page sends one `invoke` and receives an account id and an email address.

---

## 3. Two Google Cloud projects, and why

| | Project A (exists) | Project B (new, D7) |
|---|---|---|
| Scopes | `gmail.readonly`, `calendar.readonly` — **restricted** and **sensitive** | `openid`, `email`, `profile` — **basic** |
| Review | Restricted-scope verification, CASA assessment annually (C1's P4) | **None required** |
| Cap while unverified | 100 named test users, refresh tokens expire in 7 days | No cap, no test-user list |
| Used by | `google-connect` / `google-callback` (C2, D12) | Supabase Auth's Google provider (this spec) |

They are kept apart because a single client carrying a restricted scope drags the whole client
through review — and a sign-in button that only works for 100 named testers is not a sign-in button.
Project B asks for nothing about the student except who they are, which is the definition of the
basic-scope tier, so it goes to "In production" on the day it is created and stays there.

One client id serves both Supabase projects: its **Authorised redirect URIs** list carries
`https://<staging-ref>.supabase.co/auth/v1/callback` and
`https://<prod-ref>.supabase.co/auth/v1/callback`. Google's own consent screen is what the student
sees; `127.0.0.1` never appears in it, because Google's redirect target is Supabase, not us.

**Configured in the dashboard, and declared nowhere in this repo** (amended 2026-09-17, R-C1b-5;
§13). `cloud/supabase/config.toml` carries the auth settings C1 pushed — `[auth]`, `[auth.email]`,
`[auth.email.smtp]` with `pass = "env(SMTP_PASSWORD)"` — and C1b adds two more to it
(`enable_confirmations = false`, and an explicit `[auth.rate_limit] email_sent`). It does **not** add
an `[auth.external.google]` block:

- Quinn enabled the provider in the Supabase dashboard on **both** projects on 2026-09-17, and it was
  verified without holding a key: `/auth/v1/authorize?provider=google` answers `302` to
  `accounts.google.com` on staging and on prod, each carrying that project's own `/auth/v1/callback`
  as `redirect_uri` and `email profile` as the scope.
- The Supabase CLI (2.117.0) documents `config push` as pushing *"the properties your local
  config.toml declares … Properties the file does not declare are left unchanged"*, so a push that
  carries C1b's two settings leaves the provider exactly as the dashboard has it — and `supabase
  config diff` before each push is what proves that rather than assuming it.
- So the client id and the secret never enter this repo, this file, or any shell in this stream.
  A declared block would have needed `GOOGLE_SECRET` in every pushing shell, and a push from a shell
  without it would have **disabled** a provider that works.

`config.toml` says all of this in a comment where the block would have gone, and a Deno test pins
both the absence and the comment — "we chose not to write a block" is otherwise invisible on a diff.

---

## 4. Finding: the redirect allow-list

**A wildcard entry is not needed, and a wildcard port would work if it were.** Two facts, both from
GoTrue's own source:

1. `IsRedirectURLValid` (`internal/utilities/request.go`) tests the URL's hostname before it ever
   consults the allow-list: `else if ip := net.ParseIP(refurl.Hostname()); ip != nil { return
   ip.IsLoopback() }`. `127.0.0.1` parses as an IP and is loopback, so **any** port and **any** path
   under it is accepted with no entry in `additional_redirect_urls` at all. The function's own
   comment cites RFC 8252 §7.3 — "native apps must be allowed to use variable port numbers" — which
   is exactly this case. (A hostname that is only digits, the decimal form of an IP, is refused a
   line earlier.)
2. If an entry were added anyway, `http://127.0.0.1:*/callback` **does** match: the allow-list is
   compiled as `glob.MustCompile(uri, '.', '/')` (`internal/conf/configuration.go`), so the
   separators are `.` and `/` and `*` matches any run of characters containing neither — a port
   number contains neither. Supabase's published documentation states the glob rules and the two
   separators but says nothing about ports, which is why this was settled against the source.

So `cloud/supabase/config.toml`'s `additional_redirect_urls` is left exactly as it is, and the plan
proves the behaviour live on staging rather than trusting a reading of a file.

**The relay fallback in the brief is not available as described, and is not needed.** It would have
carried the port in `state`, and `state` is in GoTrue's `reservedOAuthParams`: a client-supplied
`state` is deleted from the query before the provider is called (`internal/api/external.go`). If
finding 1 were ever to change, the fallback would have to carry the port some other way — a
per-sign-in nonce written to a local file that `site/signed-in.html` cannot read is not it — and it
would be a new decision, not this one. `site/signed-in.html` is untouched by this spec.

---

## 5. Finding: how an `accounts` row comes to exist — and why Google breaks it today

### 5.1 The trigger raises on a Google sign-up

It is a trigger, not the first `/entitlement` call:
`cloud/supabase/migrations/20260910000100_accounts.sql` ends with `create trigger
on_auth_user_created after insert on auth.users for each row execute function
public.handle_new_user()`, and that function reads three keys out of `new.raw_user_meta_data` —
`age_attested`, `tos_version`, `privacy_version` — then inserts one `accounts` row, one empty
`entitlements` row and three `consents` rows.

A Google sign-up's `raw_user_meta_data` is Google's ID-token claims (`iss`, `sub`, `email`, `name`,
`picture`, `email_verified`, …) and carries none of the three. Trace it exactly:
`attested boolean := (null) = 'true'` is **NULL**, so `if not attested then raise` does *not* fire
(plpgsql treats a NULL condition as not-true) — but `tos := nullif(null, '')` is NULL, so
**`if tos is null or priv is null then raise exception 'the terms and the privacy policy must be
accepted at sign-up'` does fire**, the insert into `auth.users` rolls back, and GoTrue answers
`Database error saving new user` and redirects with `error=server_error`. Google sign-up is a 500
until this changes. (Google sign-**in** by an *existing* verified email links an identity to the
existing `auth.users` row and inserts nothing, so it never reaches the trigger.)

**The change, in a new migration `20260917000100_oauth_consent.sql`** (amended 2026-09-17, R-C1b-3):

- `create or replace function public.handle_new_user()` reads **nothing** out of
  `raw_user_meta_data`. For every new `auth.users` row, whichever path created it, it inserts the
  `accounts` row with `tos_version`, `tos_accepted_at`, `privacy_version` and `age_attested_at`
  **null**, inserts the empty `entitlements` row, inserts **no** `consents` rows, and returns — and
  **no `raise` remains in the function**. Everything else — the three rules `migrations_test.ts` pins
  (no date-of-birth column, RLS on every table, no client write policy) — is untouched: this
  migration creates no table and no policy.
- `POST /account/consent` on the existing `account` function (`subPath()` already routes that
  function's paths). Body `{tos_version, privacy_version, age_attested}`; bearer required; `400`
  unless `age_attested === true` and both versions are non-empty. It fills the four null columns and
  inserts the three `consents` rows (`tos`, `privacy`, `age_18`) **only when the account has none**,
  so a second call is a no-op. The app calls it after every sign-in, on **both** paths, and it is the
  only writer of a consent row anywhere in the system.
- **Why the trigger stops reading the metadata rather than reading less of it.** `/otp` with
  `create_user: true` is reachable by anyone holding the public anon key, and with
  `enable_confirmations = false` (D5) GoTrue creates the user before the code is ever typed. A
  trigger that read `age_attested` out of that request's `data` would write an `age_18` consent row
  asserting an attestation the address's owner never made — junk in precisely the table California's
  ARL wants kept. Reading nothing means a never-verified address leaves an `accounts` row with a null
  attestation and **zero** `consents` rows; a consent row can then only exist behind a session whose
  address was proved, which is what the exit gate checks.
- **The tooth that replaces the raise:** `billing-checkout` selects `age_attested_at` alongside
  `email` and `stripe_customer_id` and answers `403 "the 18+ attestation is missing — sign in again"` when it is
  null — before the Stripe customer is created and before the consent row is written. That is now the
  whole of the server-side 18+ gate, and the migration's comment says exactly that in place of the
  old promise: a patched client that skips the consent call gets an account it cannot subscribe with.

### 5.2 The email path's other trap: which mail the student gets

`/otp` with `create_user: true` for an address with no confirmed user hands the request to
`a.Signup(...)`, and with `Mailer.Autoconfirm = false` GoTrue stops there — its own comment reads
*"otherwise confirmation email already contains 'magic link'"* (`internal/api/magic_link.go`). That
sends the **confirmation** template, and `cloud/supabase/templates/` holds exactly one file,
`magic_link.html`. So a brand-new student receives Supabase's stock *Confirm your signup* mail,
which carries `{{ .ConfirmationURL }}` and **no `{{ .Token }}`** — a link the desktop app can never
receive, and no code to type. With `enable_confirmations = false` the same handler takes the other
branch: sign the user up, then re-enter `MagicLink`, which sends `[auth.email.template.magic_link]`
— the Knowlu template, code first. That is D5, and it is the whole reason for it.

`params.Data` would reach the created user as `raw_user_meta_data` on both branches
(`signUpParams.Data = params.Data`) — but after R-C1b-3 the `/otp` body carries no consent keys at
all: the address and `create_user: true`, and nothing else. Both paths now take the same trigger,
which reads nothing, and both record their consent the same way once a session exists (§5.1).

---

## 6. The account panel, after

Today (`app/static/index.html:86-93`): an email field and a password field, three buttons
("Create account", "I already have one", "Email me a link"), then a hidden code row. After:

1. **Continue with Google** — the first control on the panel.
2. *or* one email field and one button, **Email me a code** (`send_magic_link`, now
   `create_user: true` and carrying **no consent keys at all**: the attestation and the two versions
   are recorded after the code is typed, by `POST /account/consent`, §5.1).
3. The code row, shown once the mail is away, unchanged (`verify_email_code`).
4. The two checkboxes stay where they are and gate **both** paths: `#wiz-18` and `#wiz-terms` must be
   ticked before either button does anything, and the refusal is still *"Tick both boxes to create an
   account."* They are the consent `POST /account/consent` records.

`#wiz-pw` and `#up-pw` are deleted, so `static_assets.rs`'s password allow-list shrinks from four ids
to two and its `seen >= 3` becomes `seen == 2` — the two coursework logins, and nothing else on the
page. The upgrade overlay (`#upgrade`) gets the same three controls for the same reasons.

---

## 7. Back and Next

**What the code actually does.** Both buttons are wired (`console.js:1710-1711`) and `wizGo` moves
the step. Three things make them read as broken, and all three are real:

1. **A disabled wizard button looks exactly like a live one.** `console.css` styles
   `[disabled]` only under `.flagpop` (line 389) and `.set-row` (line 483). `.wiz-nav` has no such
   rule, so `#wiz-back` at step 0 — and `#wiz-next` whenever something disables it — is full
   contrast, full colour, and silently inert.
2. **`#wiz-next`'s `disabled` is state that `renderWizard` does not own.** Two places set it true
   (`console.js:1584`, discovery; `:1631`, Finish) and `renderWizard` never sets it false, against the
   file's own A-5 rule that wizard state lives on `WIZ` and is painted by `renderWizard`. A Finish
   whose last promise resolves without relaunching leaves Next dead with no way back.
3. **Next is gated and the gate has no exit.** `wizValid` refuses step 1 without `WIZ.accountId` and
   step 2 without `WIZ.entitled` (`console.js:1527-1529`). Quinn met the second with no way through
   it at all: no working checkout, no code, and a red line asking them to finish a page that never
   opened. That is the "don't work" — the wall, not the wiring.

**The fix.** (a) Back is never `disabled`; at step 0 it is simply absent from the nav, so there is no
dead control to press. (b) `#wiz-next`'s disabled state becomes `WIZ.busy`, set by the two owners and
painted by `renderWizard` like every other field, so it always recovers. (c) A refused Next paints
one sentence naming what is missing and **also** flashes it, because `#wiz-error` sits at the left of
the nav row and a red line that was already there does not read as a new answer. (d) `.wiz-nav
button.b[disabled]` gets the same `opacity: .5; cursor: not-allowed` the other two scopes have, so
the one remaining disabled state is visibly one. The subscribe wall itself is opened by §8, not here.

Pinned by `app/tests/static_assets.rs` and by `scripts/wizard-check.py`, which already walks Back
five panels and forward five (its step 8).

---

## 8. Promotion codes at checkout

`checkoutForm` (`cloud/supabase/functions/billing-checkout/handler.ts:39`) gains exactly one line,
`"allow_promotion_codes": "true"`. Everything else on the form stays — `"payment_method_collection":
"always"` (`handler.ts:55`) included, along with the 7-day trial, Stripe Tax, the billing address and
the terms checkbox — and the handler test keeps
`assertEquals(form["payment_method_collection"], "always")` and gains one for the new field.

**Why `always` stays** (amended 2026-09-17, R-C1b-2; §13). Stripe collects a payment method under
`"if_required"` only when the first invoice has an amount due, and
`subscription_data[trial_period_days]` is `"7"` (`handler.ts:54`), so the first invoice is always
zero: `if_required` would ask **nobody** for a card, not merely the student holding a 100-percent
code. That is not a narrow relaxation of §11 R2, it is its repeal — and it would falsify the bolded
auto-renew disclosure on `site/terms.html:24` and on the subscribe panel
(`app/static/index.html:95`), the sentence whose `TOS_VERSION` every `auto_renew` consent row is
stamped with. It would also leave every trial with no payment method at its end, which Stripe
resolves by invoicing: `past_due` rather than `active`, written straight into `entitlements` by the
webhook. The founder's own subscription is the cheaper thing to move — a card typed once under the
100-percent code and never charged, or a comped subscription from the Stripe dashboard — and nothing
a student is sold changes.

A card-free trial **for everyone** is a product decision with a terms rewrite and a `TOS_VERSION`
move behind it, not a form value. It is recorded as an open question in the plan's resolutions
section and is Quinn's to take separately, with the lawyer.

The 100-percent code itself is created by Quinn in the live Stripe dashboard (§9). Nothing in this
repo names a code, a coupon id or a percentage.

---

## 9. What Quinn does — the plan names them, Quinn sets them

| # | What | Without it |
|---|---|---|
| **Q1** | **DONE 2026-09-17.** A **new Google Cloud project**, publishing status **In production**, one **OAuth 2.0 Web application** client. Scopes on its consent screen: `openid`, `email`, `profile` — nothing else, so no verification is asked for. Authorised redirect URIs: each project's own `/auth/v1/callback`. The provider was then enabled **in the Supabase dashboard on both projects**, and confirmed keylessly by a `302` from `/auth/v1/authorize?provider=google` to `accounts.google.com`. The client id and the secret stay with Quinn and the dashboard: **no value from this is typed into a session, a shell or this repo** (R-C1b-5). | No Google button on either project |
| **Q2** | **The controller's, not Quinn's** (R-C1b-5): `supabase config diff`, then `supabase config push`, against staging and later prod, carrying only `enable_confirmations = false` and `[auth.rate_limit] email_sent`. No `GOOGLE_*` value in the shell — the file declares no provider, and the diff is run first to prove the push leaves the dashboard's alone. | The email path sends the stock *Confirm your signup* mail, with a link and no code, and the `/otp` rate limit stays whatever the platform defaults to |
| **Q3** | A **100-percent promotion code** in the **live** Stripe dashboard (a coupon at 100% off, a promotion code on it, duration Quinn's choice), and the same in **test** mode so the exit gate can prove it. | §8's flag works and there is no code to type into it |
| **Q4** | Read the two changed privacy sentences (§10) before they are published. | The policy describes a password Knowlu no longer has |

Quinn's existing restricted-scope project, its verification track and C1's P4 are untouched.

---

## 10. Legal and privacy

`site/privacy.html` does **not** name Google sign-in — its Google section is Gmail and Calendar only
(lines 74-84). Two sentences are wrong the moment the password goes, and one is missing. All three
are in the same commit, and `account::PRIVACY_VERSION` (`app/src/account.rs:25`) and the page's own
date both move from `2026-09-16` to the publication date, because `account.rs`'s own comment requires
them to move together.

- **"What we collect → Your account"** today reads *"The email address you sign up with; your
  password, which Supabase holds as a hash and which we never see; …"*. The password clause is
  deleted and replaced with: *"and, if you choose Continue with Google, the fact that this account
  signs in with Google and the Google account id that identifies it — Knowlu never sees your Google
  password and never asks for one. There is no password on a Knowlu account at all: you sign in with
  Google, or with a code we email you."*
- **"Who else touches it → Supabase"** today reads *"Your account row, your password hash and every
  row above live there."* The password hash clause goes; the sentence becomes *"Your account row and
  every row above live there, and it is what checks the code we email you or the sign-in Google
  confirms."*
- **The Google section** gains one opening sentence, before the Gmail paragraph: *"Signing in with
  Google tells us three things and only three: your email address, your name, and a link to your
  profile picture. It asks Google for nothing else — not your mail, not your calendar, not your
  files — and it is a different permission from the Calendar and Gmail connections below, which you
  grant separately and can refuse."*

No change to `site/terms.html` is required by this spec: it describes a subscription and an account,
not an authentication method. The Google API Services User Data Policy paragraph already on the
privacy page covers the basic scopes without amendment — Limited Use binds them the same way.

---

## 11. Fidelity ledger

| Source | Says | C1b carries it by |
|---|---|---|
| Cloud design **§4.2 step 1** | *"Sign in or create account (email + password, or a magic link; 18+ attestation checkbox; ToS + privacy policy acceptance logged with version and timestamp)"* | **Amended.** "email + password" becomes "Continue with Google, or a code we email you". The attestation checkbox, the acceptance, and the version-and-timestamp logging all stay — moved off the auth trigger entirely and onto `POST /account/consent` (§5.1), which records the same three rows with the same versions on **both** paths, behind a session whose address has been proved (R-C1b-3) |
| Cloud design **§4.2** steps 2-7 | Subscribe, calendars, coursework, Gmail, slots, done | Untouched. The session lands at `PENDING_TARGET` exactly as before (D3), so no later panel changes |
| Cloud design **§5.1**, Identity | *"email + password with email confirmation, plus magic-link sign-in. No social login at launch (one path to test, one path to support)"* | **Superseded** by Quinn 2026-09-17 (§0). The count of paths does not grow: password goes as Google arrives, so there are still two — Google and the emailed code — and one of them is the one every student already has |
| Cloud design **§5.1**, Account row | The nine columns, no birthdate ever | Unchanged. §5.1's migration adds no column and removes none; `age_attested_at` is now null for the moments between a Google sign-up and the consent call, and `billing-checkout` is what will not let it stay null |
| Cloud design **§5.1**, Sessions | JWTs in Credential Manager at `knowlu/<profile_id>/session`; refresh is the client's; sign-out revokes | Unchanged. PKCE returns the same `access_token` / `refresh_token` / `expires_in` shape `session_from` already parses, so `valid_access_token_at` and `sign_out` need no edit |
| Cloud design **§5.1**, Anti-cracking | *"A patched client with no valid session gets no judgments"* | Held, and extended: a patched client that skips the attestation gets an account that cannot subscribe (§5.1) |
| Cloud design **§9**, the consent log | Every acceptance recorded with its version and its timestamp | Held on both paths, and narrowed (R-C1b-3): `POST /account/consent` is the only writer, so a consent row exists only where a session whose address was proved asked for one. A never-verified address leaves an `accounts` row with a null attestation and **no** `consents` rows — nothing fabricated in the table the ARL wants kept |
| **§11 R2** | A 7-day trial with the card up front | **Untouched** (R-C1b-2). `payment_method_collection` stays `"always"`; C1b adds `allow_promotion_codes` and nothing else, so every trial still takes a card and `site/terms.html`'s bolded disclosure stays true (§8) |
| **Amendment 2026-09-17** | Desktop only; the account is the source of truth | Held. The loopback listener is a desktop mechanism and exists for one sign-in; nothing here assumes a second device or a browser session |
| `CLAUDE.md` rule 1 | No single-user assumptions | Held. No account, email, port, code or client id is named anywhere in the repo |
| `CLAUDE.md` rule 2 | Never regenerate a frozen reference | Held. This stream reads none of the eleven references and writes none |

---

## 12. Not in this spec

- **Production deployment of C2.** Production is still at C1 level — eight functions of 2026-09-14,
  none of C2's eleven, no C2 migration (HANDOFF §4). Getting prod to parity is a separate pre-pilot
  checklist and is not C1b's; C1b deploys its own migration and its own two functions to both
  projects because its own gate needs them, and nothing else.
- **Anything about sync.** C3 is paused; C3' is unwritten. No file this stream touches is C3's.
- **Apple, Microsoft or any third provider.** One social provider, chosen because every student in
  the pilot already has it. A second is a new decision, not an extension of D1.
- **Removing the Gmail/Calendar Google client or its verification track.** Project A and C1's P4
  stand exactly as they are (§3).

---

## 13. Amendments (2026-09-17)

Three controller's rulings on the plan review (`docs/reports/2026-09-17-c1b-sign-in-plan-review.md`,
findings C6, I1/I2 and C5). The body sections above are rewritten to match them; this section records
why.

**R-C1b-2 — `payment_method_collection` stays `"always"`; only `allow_promotion_codes` is added
(§8, D8, and the §11 R2 ledger row).** The review's C6 showed that `"if_required"` collects a card
only when the first invoice has an amount due, and a 7-day trial makes that invoice zero for every
subscription — so the value would have taken the card from nobody, repealing §11 R2 rather than
relaxing it for coded sessions, falsifying the bolded auto-renew sentence on `site/terms.html:24`
(the version each `auto_renew` consent row is stamped with) and on the subscribe panel, and leaving
every trial to end in `past_due` with no payment method to charge. The founder's own subscription is
solved outside the product: a card typed once under the 100-percent code and never charged, or a
comped subscription from the Stripe dashboard. Option (b) of the review — a card-free trial as the
product — stays open for Quinn, and is a terms change with the lawyer, not this stream's default.
The cost if this ruling is wrong is one form value and one terms sentence.

**R-C1b-3 — consent is recorded by `POST /account/consent` on both paths, behind a verified session;
the trigger records nothing and never raises (D6, §5.1, §5.2, §2 step 8, and the §4.2-step-1 and §9
ledger rows).** The spec first kept a conditional raise so that an explicit `age_attested: 'false'`
still failed at the database. The review's I2 showed the other side of the same door: `/otp` with
`create_user: true` is reachable by anyone holding the public anon key, and with
`enable_confirmations = false` GoTrue creates the user before the code is typed — so a trigger that
believed the request's `data` would have written an `age_18` row asserting an attestation the
address's owner never made, for any address a stranger chose. Reading nothing from
`raw_user_meta_data` closes that: a never-verified address leaves an `accounts` row with a null
attestation and no `consents` rows at all, and every consent row in the table was written behind a
session whose address was proved. It also makes one sentence true on both sides — the route really
is the one consent path, which is what the handler, the migration comment and the app's doc comments
all say — and the tooth the raise used to be is the `billing-checkout` 403 the spec had already
chosen. The cost if this ruling is wrong is a conditional raise put back in one migration.

**R-C1b-5 — the Google provider is configured in the Supabase dashboard, and `config.toml` declares
no `[auth.external.google]` block (§3, §9 Q1 and Q2).** The review's C5 was right that D7 had no
implementing step; the answer turned out to be smaller than a block. Quinn enabled the provider in
the dashboard on both projects on 2026-09-17 and it was verified without a key — `/authorize?provider=google`
answers 302 to `accounts.google.com` on each, with that project's own callback and `email profile` —
and the CLI (2.117.0) pushes only the properties the local file declares, leaving everything else
unchanged. A declared block would have put a client id in the repo, required `GOOGLE_SECRET` in every
pushing shell, and turned a push from a shell without it into an outage of a provider that works.
What stands in its place is a comment where the block would have been, a test that pins both the
absence and the comment, and a `supabase config diff` before each push (the plan's Task 7 step 3).
The cost if this ruling is wrong is one four-line block and one push.

Two consequences carried into the plan rather than here: the `/otp` rate limit is set explicitly in
`config.toml` rather than left at the platform default (the review's I2), and `[auth.captcha]` is
recorded as a separate open decision for Quinn, not built by this stream.
