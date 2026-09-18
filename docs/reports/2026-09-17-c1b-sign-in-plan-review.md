# C1b sign-in plan — review (2026-09-17)

Plan: `docs/plans/2026-09-17-c1b-sign-in-plan.md` (1,340 lines, 7 tasks, status WRITTEN, not started).
Spec: `docs/specs/2026-09-17-c1b-sign-in-design.md` (393 lines, PROPOSED 2026-09-17).
Weighed against `docs/specs/2026-09-09-knowlu-cloud-design.md` §4.2, §5.1, §9 and §11 R2, its
*Amendment 2026-09-17*, `CLAUDE.md`, and the controller's three rulings of 2026-09-17 (the 100% code
exists in both Stripe modes; the upgrade overlay loses the password too; `enable_confirmations = false`
is accepted because the emailed code is the proof of address).

Reviewer: Claude (opus), independent of the writer, as a security reviewer as much as a plan
reviewer. Three passes: a full read of spec and plan; a claim-by-claim check against
`app/src/{account,main}.rs`, `app/static/{index.html,console.js,console.css}`,
`app/tests/{account,static_assets}.rs`, `scripts/wizard-check.py`, `cloud/supabase/migrations/**`,
`cloud/supabase/functions/{account,billing-checkout,_shared}/**`, `cloud/supabase/{config.toml,
deno.json}`, `engine/src/ids.rs`, `site/{privacy,terms}.html`, `engine/tests/site.rs`; and a
consistency pass over interfaces, the suites the deletions touch, and the PKCE flow's properties.

## Verdict

**Execute after fix round 1 (findings C1–C6, I1–I8).** The architecture is right and is the cheapest
shape available: one `create or replace` on the trigger, one new route, two form fields, one loopback
listener, no new crate, no new window, no panel after the account panel touched. The PKCE mechanics
are correct — the RFC 7636 Appendix B vector is the real one, `b64url` is arithmetically right, the
entropy really is a CSPRNG (`ids::new_id` calls `getrandom::getrandom`), the verifier never leaves
the process, and the listener is taken by value so no port outlives a sign-in.

What blocks execution is not the design. Five defects are mechanical and each turns a task's own
stated green gate red: a pinned corpus count nobody bumps, a test assertion that can never pass, five
existing tests that pin precisely what Tasks 4-6 delete, and a required `config.toml` block the spec
writes out in full and no task ever creates. The sixth is a business ruling wearing a code change:
`payment_method_collection: "if_required"` beside `subscription_data[trial_period_days]: "7"` does
not narrowly relax §11 R2, it repeals it — for every student, not only the one with a code — and
falsifies a bolded sentence on the published terms page whose version the `consents` table records.

## Findings

### Critical

**C1. `plan.md:141-158` (Task 1 step 1) — the new migration bumps `migrations_test.ts`'s pinned
function count from 18 to 19, and nothing bumps it.**
`cloud/supabase/migrations/migrations_test.ts:303` asserts `assertEquals(parsed, 18, "today's corpus
should parse exactly 18 function creations")`, over `everyMigrationFile()` — *every* `.sql` in the
directory, not only C2's `20260911…` ones. `assertExecuteRevoked` increments `parsed` **before** the
`returns trigger` exemption (`migrations_test.ts:139-144`), so `20260917000100_oauth_consent.sql`'s
`create or replace function public.handle_new_user()` counts. Task 1 step 6's gate (`deno test`) is
red on the first run. The task's own new test is scoped to the one file and cannot see this.
*Fix:* Task 1 step 1 also edits `migrations_test.ts:303` to `19` with a sentence naming C1b's
`create or replace` as the nineteenth — the treatment the provider swap got in that same comment.
(The view pin at `:322` is unaffected, and the trigger-exemption test at `:462` uses `.includes`, so
a second `handle_new_user` entry is fine.)

**C2. `plan.md:911` (Task 4 step 1) — `assert!(!src.to_lowercase().contains("password"), "not even
the word")` can never pass.**
`app/src/account.rs:130` is `serde_json::from_str(cred.password.expose())` inside `load_session`.
`password` there is a field name on `knowlu_engine::wincred`'s credential struct — engine-owned, and
the plan's own Global Constraints say `engine/**` is not this stream's to edit. The assertion is
unsatisfiable without either renaming an engine field or deleting the one function that reads a
stored session.
*Fix:* keep the six-name loop above it and replace the catch-all with claims that are true and still
load-bearing — no `"password"` JSON key and no `password:` parameter anywhere in the file. Lines 6,
135, 173 and 181 are prose about a password the product no longer has and should be rewritten in the
same commit, but a prose ban is not a guard.

**C3. `plan.md:884-980` (Tasks 4 and 5) — three existing `static_assets.rs` tests pin exactly what
these tasks delete, and neither task touches them; `cargo test --workspace` is red at both gates.**
- `static_assets.rs:890` (`the_sign_up_call_spells_its_argument_the_way_tauri_delivers_it`) asserts
  `rust.contains("pub fn sign_up(email: String, password: String, age_attested: bool)")` and
  `js.contains("args.ageAttested = ...")`. Both die at Task 4 / Task 5.
- `static_assets.rs:980` (`every_control_this_task_added_is_in_the_markup_and_named_by_the_page`) and
  `:1032` (`the_console_can_sign_an_existing_install_in_without_re_onboarding_it`) both require
  `up-pw`, `up-create` and `up-signin` in `index.html`, and the first also in `console.js`. Task 5
  deletes all three.
*Fix:* Task 4 rewrites `:890` into the same guard over `send_magic_link(email, age_attested)` — still
the crate's one multi-word command argument, R-C1-55's trap unchanged; Task 5 swaps the three overlay
ids for `up-google`, `up-magic`, `up-code`, `up-code-go` in both lists. Name each edit as a step.

**C4. `plan.md:1197-1221` (Task 6 steps 1-2) — the new test asserts the exact opposite of two
existing tests, and Task 6 updates neither.**
Task 6's pin demands `!body.contains("EL(\"wiz-next\").disabled")` inside `wizGo` and `wizFinish`.
Two shipped tests demand the literal opposite. `static_assets.rs:523`
(`the_logins_panel_maps_what_it_finds_to_a_course`) asserts both the `= true` and the `= false` write
inside `wizGo` — R-C1-55 I3's guard that a second Next cannot start a second `discover_coursework`.
`static_assets.rs:497` (`the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it`) does
`finish.find("EL(\"wiz-next\").disabled = true").expect("wizFinish disables wiz-next")` and asserts
that write sits **before** the `google_connected` await (R2-3, the double-Finish race); `.expect`
panics the moment Task 6 lands.
*Fix:* Task 6 rewrites both in the same step, keeping each claim in the new vocabulary —
`go.contains("WIZ.busy = true")`/`("WIZ.busy = false")`, and `finish.find("WIZ.busy = true")` ordered
before `finish.find("invoke(\"google_connected\"")`. Both survive `WIZ.busy` intact; only the
spelling moves. The plan also mis-counts the sites: there are **six** `EL("wiz-next").disabled`
writes (`console.js:1584, 1601, 1623, 1631, 1676, 1688`), not two owners plus three re-enables.

**C5. `plan.md` — `[auth.external.google]` is never written into `cloud/supabase/config.toml` by any
task, so P2's `config push` enables nothing.**
Spec §3 gives the four-line block in full. The fidelity ledger row D7 (`plan.md:1300`) says it is
"`[auth.external.google]` in `config.toml` pushed by P2". But Task 4 step 3 is the only step that
edits `config.toml` and it changes `enable_confirmations` alone; Task 7's Files list is
`site/privacy.html` and `app/src/account.rs`. The block exists nowhere. Without it `supabase config
push` is a no-op for the provider and `/authorize?provider=google` answers `400 Unsupported
provider` — which the plan itself names as P2's failure mode.
*Fix:* add it to Task 4 step 3 (the file is already open there) with `client_id = "env(GOOGLE_CLIENT_ID)"`,
`secret = "env(GOOGLE_SECRET)"`, `skip_nonce_check = false`, and a comment pointing at
`pass = "env(SMTP_PASSWORD)"` (`config.toml:48`) as the precedent. Neither value enters the repo.

**C6. `plan.md:381-386` (Task 2 step 2) and spec §8 — `if_required` beside a 7-day trial collects no
card from anyone, and falsifies the published auto-renew disclosure.**
Stripe collects a payment method under `payment_method_collection: "if_required"` only when there is
an amount due on the first invoice. `checkoutForm` sets `subscription_data[trial_period_days]: "7"`
(`billing-checkout/handler.ts:54`), so the first invoice is always zero and **no** Checkout Session
ever asks for a card — not only the promotion-coded one. Three consequences the plan does not carry:
1. `site/terms.html:24` says, in bold, *"Seven days free first, with your card taken at sign-up and
   not charged until the trial ends."* That sentence is the ROSCA / California ARL auto-renew
   disclosure, and `billing-checkout` writes a `consents` row of kind `auto_renew` stamped with
   `TOS_VERSION`. Task 7 step 2 explicitly refuses to move `TOS_VERSION`. The log would then point at
   a version of the terms that describes a card the flow no longer takes.
2. `app/static/index.html:95` says the same thing on the subscribe panel the student reads.
3. With no card and `trial_settings` unset, Stripe's default at trial end is `create_invoice`, so
   every trial lands in `past_due` rather than `active` — the webhook writes that into `entitlements`
   and the student silently loses access.
Spec §8's claim that "for every full-price subscription with a trial, Stripe still collects the card"
is not what Stripe does, and the new handler test asserts the form field, not the behaviour, so
nothing in the suite catches it. Exit gate item 6 would catch it — on live Stripe, at the end.
*Fix:* Quinn's ruling, one of two. **(a)** Keep `always`, keep `allow_promotion_codes`, and comp
Quinn's own subscription from the Stripe dashboard — the founder-pays problem goes away and R2 is
untouched. **(b)** Accept a card-free trial as the product, and in the same commit rewrite
`site/terms.html:24` and `index.html:95`, move `TOS_VERSION` to 2026-09-17, and set
`subscription_data[trial_settings][end_behavior][missing_payment_method]` deliberately. (b) changes
what every student is sold; it is Quinn's call, not the executor's.

### Important

**I1. `plan.md:166-192` (Task 1 step 2) — the new trigger drops the terms/privacy `raise` outright,
so the email path can now reach checkout with the §9 consent log empty.**
`20260910000100_accounts.sql:122-124` raises when `tos_version` or `privacy_version` is missing; the
replacement keeps only the age raise. An email sign-up carrying `age_attested: 'true'` but no
versions now inserts an `accounts` row with `age_attested_at = now()`, null versions and **no**
`consents` rows — and Task 2's gate reads only `age_attested_at`, so that account subscribes with
nothing logged, against cloud design §9.
*Fix:* make the raise conditional rather than absent — `if raw is not null and (tos is null or priv
is null) then raise exception 'the terms and the privacy policy must be accepted at sign-up'; end if;`
An absent `age_attested` stays the OAuth case; a client that claims 18 must still bring both versions.
Add the assertion to Task 1 step 1's file test beside the `age attestation required` one.

**I2. `plan.md:948-966` (Task 4 steps 2-3) — `create_user: true` plus `enable_confirmations = false`
lets anyone holding the public anon key mint a confirmed account, and three fabricated consent rows,
for any address they like; the plan names no mitigation.**
`DEFAULT_ANON_KEY` ships in every build (`app/src/account.rs:19`), so `POST /auth/v1/otp` is an
unauthenticated endpoint in practice. With confirmations off GoTrue signs the user up before the code
is ever typed: one `auth.users` row and, through `handle_new_user`, one `accounts` row, one
`entitlements` row and three `consents` rows — including a `kind: age_18` row asserting an
attestation the address's owner never made. That is junk in exactly the table California's ARL wants
kept, and the blast radius is capped only by Supabase's project-wide `rate_limit_email_sent`
(30/hour by default), which is itself a denial of service on real sign-ups. The controller's ruling
accepts `enable_confirmations = false`; it does not follow that the rest can go unsaid.
*Fix:* Task 4 step 3 sets `[auth.rate_limit] email_sent` explicitly, with the number and the reason,
and the exit gate adds a line for the never-verified account's consent rows. `[auth.captcha]` is the
Supabase-native next step and is a separate decision.

**I3. `plan.md:712-760` (Task 3 step 4) — the listener accepts exactly one connection and never
checks the path, so any stray local connection ends the sign-in.**
`serve_one_callback` breaks out of `accept` on the **first** connection, reads one request line, and
returns. `code_from_request_line` looks only at the query string — a request to `/` or `/anything`
carrying `?code=` is accepted, and a request with no `code` (a browser preconnect, a local port
scanner, security software, a second process that got there first) returns
`Err("the browser came back without a sign-in code")` while the real callback is still in flight and
now meets a closed port. The plan's own test pins `/favicon.ico` as an error, which is the failure
mode, not a guard against it.
*Fix:* loop until the deadline rather than returning on the first accept — answer `404` to any
request whose target is not `/callback` and keep waiting. Same single-use guarantee, still bounded.
The compensating control against a *forged* code is real and belongs in the comment: the verifier is
per-attempt, so a code minted under any other challenge fails the exchange — which is also the honest
answer to "where is `state`?", since GoTrue owns that parameter (spec §4).

**I4. `plan.md:846-852` (Task 3 step 6) — the consent call is fire-and-forget with no retry, and the
code comments claim it runs on both paths when it runs on neither but Google.**
`let _ = record_consent_at(...)` discards every failure. Task 4 leaves `verify_email_code` untouched,
so the email path never calls the route at all — it relies on the trigger. Yet the doc comment says
"Called after **both** sign-in paths", the handler comment says "The app calls it after EVERY
sign-in, on both paths", the migration comment repeats it, and Task 1's idempotence test is justified
by it. Three comments assert a behaviour the plan does not build. The functional hole is the Google
account whose consent POST fails once: `age_attested_at` stays null for ever, `billing-checkout`
answers `403 "the 18+ attestation is missing — sign in again"`, and no code path retries.
*Fix:* correct the three comments (the email path records consent through the trigger; this route is
the OAuth path's equivalent), retry `record_consent_at` once in `open_checkout` before the POST so
"sign in again" is advice the app can act on, and give the retry a test.

**I5. `plan.md:255-330` (Task 1 step 4) and `plan.md:385-401` (Task 2 step 3) — widening the two
`Deps` interfaces breaks existing typed literals the plan does not list.**
`account/handler_test.ts:6` is `function deps(over: Partial<Deps> = {}): Deps` returning a complete
literal; two new required `Deps` fields make it a type error, and `deno test` type-checks. In
`billing-checkout/handler_test.ts` the plan says "the three existing tests gain `age_attested_at`" —
there are **four** inline `getAccount` stubs (`:43`, `:79`, `:114`, `:142`), all contextually typed.
*Fix:* name `deps()` as where the two fields land, build `consentDeps` from `deps({...})` rather than
as a second full literal (the plan's only verbatim-duplication smell), and say four, not three.

**I6. `plan.md:1050-1090` (Task 5 steps 2-3) — the upgrade overlay is specified as "the same three
rows" and "replaced the same way", with no code, no ids in Produces, no busy guard and no pin.**
The overlay today (`index.html:174-182`) has no email-code path at all: no `#up-magic`, no
`#up-code-row`, no `#up-code-go`. Task 5 adds a whole second sign-in surface in the console window by
reference. Task 5's Produces lists only `#wiz-google-signin` and `#up-google`. `renderWizard` does
not paint the overlay, so the `WIZ.busy` double-click guard that protects `#wiz-google-signin` does
not exist for `#up-google` — two presses are two listeners, two ports and two browser tabs. The new
static test greps the wizard panel only; it asserts `#up-pw` is gone but never that `#up-google`
exists or leads. The controller's ruling ("the upgrade overlay loses the password too") makes this
surface mandatory, which makes the hand-waving costlier, not cheaper.
*Fix:* Task 5 writes the overlay's markup and its click branch in full, adds a busy flag of its own,
and extends the step-1 pin over `#upgrade` the way it does over `#wiz-account`.

**I7. `plan.md:1330` (exit gate 4) — identity linking for a pre-C1b account is asserted, not
arranged.** GoTrue links a Google identity to an existing user only when both sides' emails are
confirmed. C1 accounts created under `enable_confirmations = true` that never clicked the link are
**unconfirmed**, so Google sign-in makes a second `auth.users` row, a second `accounts` row with a
null attestation, and a second unentitled subscription surface — for a student whose money is on the
first one. Quinn's own C1 account is in that population. *Fix:* Task 7 step 3 proves it on staging in
**both** states and reports the unconfirmed result to Quinn before merge.

**I8. `plan.md:100-107` (H2) — the hand-off's prose clicks `#wiz-google`, which is already the Google
**Calendar** button** (`index.html`'s calendars panel, pinned at `static_assets.rs:426` and `:465`).
Task 5 step 5's actual replacement text says `#wiz-google-signin` and is right; H2's summary would
silently drive the wrong control. *Fix:* H2 says `#wiz-google-signin` throughout.

### Minor

- **M1.** Line cites drift by one or two: `checkoutForm` is `handler.ts:39` (plan: `:38`),
  `payment_method_collection` is `:55` (plan: `:56`), the account panel is `index.html:86-94`
  (spec §6: `:88-96`). Everything else I spot-checked in spec §2 and §7 is exact.
- **M2.** H1's "the six sign-in commands are in both lists" is seven; the arithmetic it supports
  (29 + 42, **61** distinct) is correct — I recounted both lists (`main.rs:107` = 30, `:186` = 43).
- **M3.** Task 7 step 2 calls the privacy-version pin "the existing cross-file pin"; there is no such
  test (`static_assets.rs:617` pins a different sentence). The plan's is a new test; say so.
- **M4.** P1's "Needed by Task 3 step 4" and P2's "Needed by Task 7 step 2" both point at steps that
  do not need them; both are first needed at Task 7 step 3. (Task 7 itself is right: P4 gates the
  privacy sentences, staging is the controller's, production is Quinn's, and the live Google proof is
  a scratch profile pointed at staging by `KNOWLU_API_BASE`/`KNOWLU_ANON_KEY` — never a real one.)
- **M5.** `record_consent_at` detects success by string-matching `"no link came back"`, a literal
  owned by the private `post_for_url` (`account.rs:795`). Give it a sibling that wants no `url`.
- **M6.** Task 1 step 5 writes the wire as raw SQL; `account/index.ts` is PostgREST throughout
  (`restSelect`/`restPatch`/`restUpsert`), and the `age_18` consent's version (the trigger uses
  `'1'`) is unstated. (`subject_hash` is fine: `sha256Hex` lower-cases, as the trigger does.)
- **M9.** `#[tauri::command(async)]` with a 180 s `std::thread::sleep` poll holds a Tauri
  async-runtime worker six times longer than any existing command. Acceptable; worth a sentence.
- **M7.** Task 6's `assert!(js.contains("function wizValid("))` asserts only that a name exists.
- **M8.** Add `.wiz-nav button.b[hidden] { display: none; }` beside the new `[disabled]` rule —
  unnecessary today (`button.b`, `console.css:180`, sets no `display`), but the file teaches that
  lesson three times already.
- **M10.** Task 4 deletes `app/tests/account.rs:102`, `:141`, `:152` and edits the `use` at `:3`.
## Fidelity

Against the C1b spec: D1-D8 are each carried by a named task and the plan's own ledger maps them
correctly. D2's "one sign-in, then the port is gone" is genuinely enforced (the listener is taken by
value). D3 is exact - `PENDING_TARGET`, `save_session`, `session_from` and `ok_account` are reused
unchanged, so `move_session`, `attach_account`, `create_vault_in` and `valid_access_token_at` need no
edit (verified at `account.rs:143`, `:287`, `:589`, `:709`). D7 is the one decision with no
implementing step (**C5**).

Against cloud design **§4.2 step 1** the amendment is honest: the checkbox, the acceptance and the
version-and-timestamp logging all survive - except on the path **I1** opens. Steps 2-7 are untouched,
correctly. Against **§5.1** the account row gains and loses no column, sessions keep their shape and
their Credential Manager home, "no social login at launch" is superseded by Quinn on the record, and
the path count really does stay at two; the anti-cracking claim moves from the database to
`billing-checkout` and is weaker only in the window **I4** describes. Against **§9** the 18+ gate keeps
a server-side tooth and the consent log is complete for the Google path and, after **I1**, for the
email path too. §11 **R2** is the one ruled item the plan reverses rather than relaxes (**C6**).

`CLAUDE.md`'s two overriding rules hold: nothing names a person, port, code or client id, and no
frozen reference is read or written. TDD ordering is observed in every task; no test reaches the
network (both loopback servers join their threads); no secret, real account or real address appears.

## Interfaces verified against the code

| Symbol the plan consumes | Where it is | State |
|---|---|---|
| `PENDING_TARGET`, `TOS_VERSION`, `PRIVACY_VERSION` | `app/src/account.rs:32`, `:24`, `:25` | present |
| `api_base`, `anon_key`, `check_api_base`, `auth_base` | `account.rs:42`, `:46`, `:53`, `:71` | present |
| `agent`, `post_json`, `provider_error`, `session_from`, `save_session`, `ok_account`, `open_in_browser` | `account.rs:159`, `:184`, `:199`, `:208`, `:115`, `:304`, `:363` | all present |
| `post_for_url` (used by `record_consent_at`) | `account.rs:782` | present; `Err("no link came back")` on a `{ok:true}` body — M5 |
| `knowlu_engine::ids::new_id` (PKCE entropy) | `engine/src/ids.rs` | present, `getrandom::getrandom`, 5 bytes/call — the 7-call loop is sound |
| `sha2::Sha256` in `app`; `loopback()`; `CREDMAN_LOCK` | `app/Cargo.toml`; `tests/account.rs:12`, `:204` | present — no new crate, and the helper joins its thread |
| `requireUser`, `json`, `fail`, `readJson`, `methodNotAllowed`, `subPath` | `functions/_shared/{auth,http}.ts` | present; `fail` returns a `Response` that callers throw |
| `Deps`, `handle`, `deps()` (account fn) | `account/handler.ts:23`, `:121`; `handler_test.ts:6` | present — widening `Deps` breaks `deps()` (I5) |
| `checkoutForm`, `Deps.getAccount` (checkout) | `billing-checkout/handler.ts:39`, `:29` | present; four inline literals to widen, not three (I5) |
| `accounts.age_attested_at`, `consents`, `handle_new_user` | `20260910000100_accounts.sql:18`, `:36`, `:112` | present; both `raise`s at `:120`, `:123`. The corpus pin `assertEquals(parsed, 18)` (`migrations_test.ts:303`) is **not** bumped by the plan (C1) |
| `[auth.external.google]` block | `cloud/supabase/config.toml` | **missing** — no task creates it (C5) |
| `#wiz-google` (Google **Calendar**) | `app/static/index.html`, pinned `static_assets.rs:426` | present — collides with H2's prose (I8) |
| `#up-magic`, `#up-code`, `#up-code-go` | `app/static/index.html` | **absent**; Task 5 adds them by reference only (I6) |
| `BEFORE_FINISH_OK`, check 2, check 8 | `scripts/wizard-check.py:32`, `:116`, `:223` | present; the panel walk is unchanged by this stream and still holds |

Every symbol a later task consumes from an earlier one appears in that task's Produces, except Task
5's three new overlay ids (I6).

## What I did not check

No build and no state-changing command: `cargo test`, `deno test`, `deno lint` and
`scripts/wizard-check.py` were **not** run, so C1-C4's red-suite claims are read from the assertions
rather than from a failing run — each names a file and line, so they are cheap to confirm. Nothing
was checked against live Supabase, Google or Stripe: GoTrue's `IsRedirectURLValid` loopback branch,
`reservedOAuthParams`, the `s256` spelling and the `/otp` template branch are the spec's reading of
GoTrue's source, which Task 7 step 3 is right to prove live, and C6 rests on Stripe's documented
`if_required` semantics. I did not read `site/terms.html` past line 24, C2's functions, or
`engine/src/` beyond `ids.rs`.

## Re-review after fix round 1 (2026-09-17)

**Verdict: Execute after fix round 2** — three lines to change, none of them structural. The round is
otherwise complete and I re-checked every cite it wrote against the code: all hold, including the four
it corrected out from under the review (`migrations_test.ts:304` not `:303`; the view pin `:320` not
`:322`; `checkoutForm` `:39` and `payment_method_collection` `:55`; `tests/account.rs:149` for the
sign-up test, whose `sign_up_at` call is at `:152`). What remains: **R1** Task 7's live proof still
demands the card-free Checkout that R-C1b-2 rejected; **R2** the overlay's `UP_BUSY` is placed inside
the click listener, where it resets on every press and the new test passes anyway; **R3**
`[auth.rate_limit]` is a sibling TOML table, not a key "in the same block" as `enable_confirmations`.
R4-R6 are cheap.

### The original findings

- **C1 resolved.** `plan.md:123-165`. `assertEquals(parsed, 18, …)` really is at
  `migrations/migrations_test.ts:304`, `assertExecuteRevoked` counts before the `returns trigger`
  exemption, the view pin at `:320` counts views this migration does not create, and `:471` is
  `exemptedTriggers.includes("handle_new_user")`. The per-file test now strips `--` lines and all six of
  its claims hold against the migration step 2 writes; nothing anywhere pins the raise text (only
  `20260910000100_accounts.sql:120` and `:123` carry it).
- **C2 resolved.** `plan.md:1133-1150`. After the four deletions the only `password` left in
  `account.rs` is `:130`'s `cred.password.expose()`; `"password"` survives only at `:233`, `:250`
  (deleted) and `:181` (rewritten in the same step), and `password: &str` / `password: String` only at
  `:226`, `:249`, `:312`, `:327` — all deleted. Both new assertions are satisfiable.
- **C3 resolved.** `plan.md:1157` rewrites `static_assets.rs:890`; `plan.md:1424-1426` swaps the id
  lists at `:980` and `:1032`, quoting both lines exactly. A fourth pin the review missed —
  `static_assets.rs:545`, `js.contains("sign_up") && js.contains("sign_in")` — is inside
  `the_account_panel_gates_on_eighteen_and_links_both_policies`, which Task 5 step 1 replaces wholesale,
  so it is covered. The relaxation from `args.ageAttested = EL("wiz-18").checked` to `ageAttested` is
  right: `console.js:1260` and `:1725` carry it today and no `age_attested` appears on the page.
- **C4 resolved.** `plan.md:1669-1671` rewrites `static_assets.rs:523-526` and the `.expect` at `:497`;
  `plan.md:1687-1696` tables all six write sites. Verified against `console.js`: `wizGo` is
  `:1546-1608` (holds `:1584`, `:1601`), `wizFinish` `:1627-1690` (`:1631`, `:1676`, `:1688`) and
  `credentialsStranded` `:1617-1626` (`:1623`) sits between them — see **R5**.
- **C5 superseded by R-C1b-5, applied consistently.** No `[auth.external.google]` anywhere; the comment
  and its pin (`plan.md:1320-1339`) land in `_shared/config_toml_test.ts`, whose `CONFIG_TOML` really is
  at `:4` and really is a `Deno.readTextFile` URL. `config.toml` declares no `[auth.external.*]` today,
  so the absence assertion is true before and after. P1/P2/Q1/Q2, §3, §13 and the D7 row all agree, and
  no `GOOGLE_*` value appears in either document.
- **C6 superseded by R-C1b-2, applied — except R1.** `handler.ts:54` and `:55` are left alone,
  `site/terms.html:24` stays true, D8, §8, §11 R2, the Architecture line and exit gate 6 all moved.
  `plan.md:1804` did not.
- **I1 superseded by R-C1b-3.** The trigger reads nothing and raises nothing; the per-file test bans
  `raise exception`, `raw_user_meta_data` and `public.consents` over comment-stripped code.
- **I2 answered twice.** R-C1b-3 removes the fabricated `age_18` row; R-C1b-4 sets `email_sent = 20`
  with the trade-off stated both ways. `email_sent` is the right Supabase key and custom SMTP is
  configured, so the number takes effect. Placement is **R3**.
- **I3 resolved.** `plan.md:831-880`. Non-blocking accept against the deadline, 404 + `continue` for
  any target whose path is not `/callback`, return on the first real one, listener still by value. The
  new test is deterministic (the probe reads to EOF before the browser connects, so accept order is
  fixed), and `write_page` keeps both existing listener tests' `HTTP/1.1 200 OK` prefix true.
- **I4 resolved.** `post_authed` mirrors `post_for_url` (`account.rs:782-796`) field for field —
  `agent()`, `UNREACHABLE`, `limit(1 << 16)`, `provider_error` — and `open_checkout`'s rewrite matches
  the shipped body (`:799-809`) exactly, `PENDING_TARGET` included. `env_pair()`'s third member is the
  API base (`:296`) and `attach_in`'s log line is at `:602`, as cited. The retry test is satisfiable:
  `loopback` serves two responses, `checkout_url_at` is not in the `api_base()`-reaching set that
  `no_test_in_this_file_can_reach_the_compiled_in_project` guards, so it needs no `ApiBase::set`.
- **I5 resolved.** `handler_test.ts:6` is the only full `Deps` literal in that file; `consentDeps` now
  builds from `deps({…})`. The four checkout stubs are at `:43`, `:79`, `:114`, `:142` as stated, and
  Task 2 step 4 remembers `billing-checkout/index.ts`'s select.
- **I6 resolved in substance.** Markup, three branches, the shared tail and a new `#upgrade` test are
  all written out; `.set-row` is `display: flex` at `console.css:470` with no `[hidden]` rule, so that
  addition is a live fix, not a pre-emptive one. The busy flag's placement is **R2**.
- **I7 resolved** (exit gate 4 + Task 7 step 3 prove both states, report before merge). **I8 resolved:**
  `plan.md:95` says `#wiz-google-signin` and why, and it cannot collide with `id="wiz-google"`.

**Minors.** **M1** every corrected cite verified above, plus `index.html:86-93` (account panel) and
`:174-182` (overlay). **M2** verified by script over both `generate_handler!` lists: 30 + 43 today, 62
distinct, 11 shared; after C1b 29 + 42, 71 registrations, 10 twice, 61 distinct — the plan's arithmetic
and its list of the seven shared sign-in commands are both right. **M3** `static_assets.rs:617` is
`the_wizards_privacy_sentence_is_the_sites_privacy_sentence` and nothing pins `PRIVACY_VERSION`; "a NEW
test" is correct, and Task 7's three edits do not touch the sentence that test reads. **M4** P1 done,
P2 the controller's, both pointed at Task 7 step 3. **M5** `post_authed`/`post_no_reply` remove the
string match on `post_for_url`'s private literal. **M6** `restSelect`/`restPatch`/`restUpsert` and
`sha256Hex` are all already imported at `account/index.ts:1-11`, and `sha256Hex` does lower-case
(`_shared/crypto.ts:31`); the new `restUpsert(rest, "consents", …)` does not disturb the purge guard at
`handler_test.ts:363`, which matches `restPatch(rest, "consents"` only. **M7** `wizValid` really does
contain both asserted gate strings (`console.js:1528`, `:1529`) and `wizGo` `!wizValid()` (`:1547`), and
Task 6 step 4 moves the step-1 sentence to `"Sign in first."` so the pin is satisfiable at its own gate.
**M8** added. **M9** one sentence at `plan.md:1087`. **M10** the three deletions and the narrowed `use`
are right — `Session` is still used at `tests/account.rs:166` without an inner import, so keeping it in
the `use` leaves no unused-import warning.

### New findings

**R1. `plan.md:1804` (Task 7 step 4) still asks the live proof to confirm the behaviour R-C1b-2
rejected.** The step ends: *"type P3's test-mode 100-percent promotion code on Stripe's page, and
confirm Checkout completes **with no card asked for**"*. Under the ruling the card is always collected
(`handler.ts:55` untouched), and exit gate 6 now says so in as many words. A controller running the gate
would read a working flow as a failure — or "fix" the form value back to `if_required`. It is the only
survivor of the (b)-world in either document (`grep` finds no other).
*Fix:* "…confirm Stripe still asks for a card, the total reads $0.00, nothing is charged, the webhook
writes `entitlements`, and the wizard's poll advances to the name panel."

**R2. `plan.md:1507-1527` — `var UP_BUSY = false;` is placed inside the click listener, which makes
I6's guard a no-op, and `plan.md:1417-1418` cannot catch it.** The sentence reads "the `#up-create`
/`#up-signin` branch goes; these three take its place **inside the same**
`EL("upgrade").addEventListener("click", …)`", and the block immediately under it declares `UP_BUSY`,
`upBusy()` and `afterUpgradeSignIn()`. Declared there, the flag is re-initialised to `false` on every
press: two presses on Continue with Google are two commands, two loopback listeners and two browser
tabs — exactly what I6 asked for a guard against. The indentation hints at IIFE scope, but the prose
outranks indentation for an executor, and both new assertions (`js.contains("function upBusy(")`,
`listener.contains("UP_BUSY")`) pass either way.
*Fix:* say the flag and the two functions go at IIFE scope beside `upgradeUnreachable()`
(`console.js:1237`), and that only the three branches go inside the listener; then pin it — the text
**before** `EL("upgrade").addEventListener` must contain `var UP_BUSY`.

**R3. `plan.md:1284` — "in the same block" is wrong for `[auth.rate_limit]`, and the new pin does not
notice.** `[auth.rate_limit]` is a sibling table of `[auth.email]`, not a key in it. Inserted where the
sentence points — straight after `enable_confirmations` (`config.toml:28`) — it captures
`double_confirm_changes = true` (`:29`), which is not a rate-limit key, and `supabase config push`
fails or, worse, pushes an `[auth.email]` that has quietly lost a setting. `config_toml_test.ts`'s new
checks (`/^[auth.rate_limit]$/m`, `/^email_sent = d+$/m`) are satisfied wherever it lands.
*Fix:* name the insertion point — after `double_confirm_changes = true` (`config.toml:29`) and before
the SMTP comment block at `:31`, at `[auth]` sibling level — and add one assertion that
`double_confirm_changes` appears before the first `[auth.rate_limit]` line.

**R4. `plan.md:1341` (Task 4 step 4) never runs the Deno test Task 4 writes.** The gate is
`cargo test --workspace` plus `deno fmt --check`; step 3b adds assertions to
`_shared/config_toml_test.ts` that are first executed at exit gate 11. A task's own test must be green
at its own gate.
*Fix:* step 4 runs `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/` and
`deno lint`, as Tasks 1 and 2 already do.

**R5. `plan.md:1687-1696` converts `console.js:1623`, but no test reaches it, and the plan says one
does.** Step 1's pin slices `wizGo` (`function wizGo(` to `function wizRegister(`) and `wizFinish`
(`function wizFinish(` to the Checkout-page comment). `credentialsStranded` is `console.js:1617-1626`,
between the two slices, so `:1623` is outside both — yet `plan.md:1696` claims the grep returning one
line "is what step 1's test asserts from the other direction". A missed `:1623` is worse than cosmetic:
`renderWizard()` fires on the next line and repaints `EL("wiz-next").disabled = WIZ.busy`, so a
stranded-credentials recovery leaves Next stuck.
*Fix:* add `assert_eq!(js.matches("EL(#wiz-next#).disabled").count(), 1, "renderWizard is the only
writer")` to step 1 (with the real quoting), and drop the "from the other direction" clause.

**R6. Two nits.** (a) `plan.md:1422`'s "`OURS` becomes `["wiz-zy-pass", "wiz-vhl-pass"]`" has to move
the length annotation too — it is `const OURS: [&str; 4]` (`static_assets.rs:384`). (b) Task 4 step 3
leaves `config.toml:31-35`'s comment reading *"`enable_confirmations = true` means every sign-up depends
on this mail arriving"*, stale the moment the line below it flips.

### What I checked, and did not

Read against the code: `app/src/account.rs`, `app/tests/{account,static_assets}.rs`,
`app/static/{index.html,console.js,console.css}`, `cloud/supabase/config.toml`, both `migrations_test.ts`
files, `20260910000100_accounts.sql`, `_shared/{db,crypto,config_toml_test}.ts`,
`account/{handler,handler_test,index}.ts`, `billing-checkout/{handler,handler_test}.ts`,
`scripts/wizard-check.py`, `app/src/main.rs`, `site/{terms,privacy}.html`, `engine/tests/site.rs`. No
command was run but `grep`, `sed` and one Python count of the two `generate_handler!` lists — no `cargo`,
`deno` or `supabase` — so every red/green claim is still read from assertions. Unchecked from here, and
ruled: that the CLI pushes only declared properties, that the dashboard provider is live on both
projects, and Stripe's `if_required` semantics.
