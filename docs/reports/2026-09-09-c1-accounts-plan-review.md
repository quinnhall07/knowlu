# C1 accounts plan — review (2026-09-09)

Reviewer: Claude (opus), independent of the writer. Passes over the plan: 4.

Pass 1 was structural (headings, task list, the arithmetic of the test counts). Pass 2 was the
preamble — Global Constraints, preconditions, fidelity ledger, file structure, *Interfaces with C2*,
the nine controller hand-offs — against `HANDOFF.md` §2, `CLAUDE.md` and the signed spec. Pass 3 was
Tasks 0–9, the cloud half, line by line against spec §5.1, §6, §9 and the legal note's consequences.
Pass 4 was Tasks 10–21, the device and page half, against the real code in `app/src` (I opened
`onboarding.rs`, `scaffold.rs`, `scheduler.rs`, `state.rs`, `credentials.rs`, `profiles.rs`,
`commands.rs`, `main.rs`, `tray.rs`, `app/capabilities/default.json`, `app/tests/static_assets.rs`,
`engine/src/uievents.rs`, `engine/src/wincred.rs`, `app/Cargo.toml` and `site/`), followed by the
cross-check against `docs/plans/2026-09-09-c2-judge-plan.md`.

**The plan is unusually good.** It is real code, not sketch; every task is TDD with a red step that
names its own error text; the Deno test counts add up exactly (7 → 12 → 21 → 25 → 37 → 43 → 48 → 56 →
64 → 70) and so do the Rust ones; every claim I spot-checked against the codebase was true —
`Settings` really has no serde defaults (H2 is right), `uievents::ACTIONS` really is those eleven in
that order, `is_token` really is that character class, `slot_argv` really takes
`Option<&JudgeArgs>` today, the console list really is 29 and the shell's 11, `app/capabilities/default.json`
really is scoped `"windows": ["main"]` so an `lms-signin` window really gets no grant, and
`jiff`/`ureq 3.4`/`serde_yaml_ng` really are already `app/` dependencies so the plan really adds no
crate. The findings below are defects in an otherwise executable plan, not a verdict on its shape.

## Verdict

**NEEDS FIXES** — three defects block the plan's own exit gate (the LMS URL never reaches
`PUT /account/sources`, the upgrade overlay covers today's page on a dead connection, and the wizard
asks for the campus two panels *after* the panel that needs it), and twelve names, columns and
signatures in *Interfaces with C2* do not agree with the C2 plan's *Interfaces with C1*.

## Fidelity to the signed spec

| Decision / ruling | Verdict | Where (task, line) | Note |
|---|---|---|---|
| **D1** — hosted backend, releases are CI's | Honoured | ledger `plan.md:66`; H7 `plan.md:326` | Nothing touches `.github/**`; the `deno test` job is a hand-off. Conflicts with C2's H7 (cross-check). |
| **D2** — no free tier, one edition, $9.99, account required | Honoured | T3 `plan.md:1391`, T4 `plan.md:1811`, T11 `plan.md:4657` | The wizard cannot be finished without a session; `judge (skipped: no entitlement)` is a named 0-exit step. |
| **D3** — all judgment in the cloud, the judge seam stays | Honoured | T11 `plan.md:4635-4667` | `JudgePlan::Cloud` runs `judge` on any vault with `cloud.yaml`; the runtime gate applies only to `NoAccount`. Argv disagrees with C2 (cross-check). |
| **D4** — deterministic core on device, internet work in the cloud | Honoured | T7 `plan.md:2990`, T15 `plan.md:5473` | Telemetry derived on-device; only non-content rows leave. |
| **D5** — (a)+(b) under the ToS, (c) behind opt-in, issue reports carry context | Honoured | T8 `plan.md:3225`, T9 `plan.md:3608`, T15, T16 | (c) is explicitly not built and is declared at `plan.md:7059`. (b) is narrower than §6 — see Minor. |
| **D6** — deterministic measures | Deferred correctly | ledger `plan.md:71` | C2's; C1 only shapes `corrections`. That shape is where C2 disagrees. |
| **D7** — Cloudflare + Supabase, no Firebase | Honoured | T0 `plan.md:413-467` | Supabase only; no SDK at all, `fetch` against REST/GoTrue. |
| **D9** — the vault is a plain folder the app creates; Obsidian dropped | Honoured | T12 `plan.md:4963-4969`, T17 `plan.md:6133-6134` | `vault_dest_in` creates `%USERPROFILE%\Knowlu` and `Backups`; `wiz-backup`, `wiz-pick-parent`, `wiz-pick-bdir` are asserted gone. |
| **D11** — portal scraping stays on the device | Honoured | T17 `plan.md:6277-6281` | Coursework panel unchanged; Credential Manager only; disclosed in the terms (T19 §9). |
| **D12** — Gmail is OAuth, `gmail.readonly`, server-side | Honoured | T17 `plan.md:6282-6285`, T20 `plan.md:6977` | Honest stub with the testing-mode sentence; the verification packet is written for Quinn. |
| **§11 R1** — start the Google track in C1 | Honoured | T20 `plan.md:6949-6993` + P4 | Justification and shot list are drafted, not gestured at. |
| **§11 R2** — 7-day trial, card up front | Honoured | T4 `plan.md:1791-1793` | `trial_period_days=7` + `payment_method_collection=always`, pinned by a test. |
| **§11 R3** — keep both the academic-year price and the June–August pause | **Weakened** | T5 `plan.md:2317-2341` | Both exist, but the pause is applied to `academic_year` subscribers too and the test pins `paused: 2`. See Important #13. |
| **§11 R4** — client-side encryption in sync | Deferred correctly | ledger `plan.md:89` | C3's; C1 does encrypt the one thing it stores (T7). |
| **§11 R6** — staging + test Stripe from day one | Honoured | `plan.md:32`, T0 `plan.md:416-419` | Every apply and deploy names staging; production is Quinn's or CI's. |
| **§11 R7** — synthetic fixture vaults before a public release | **Absent** | — | Ruled "later"; every other R has a ledger row. Minor. |
| **§11a** — the LMS link is captured by a sign-in window, not pasted | **Contradicted in the wizard** | T13 `plan.md:5041`, T14 `plan.md:5169`, T17 `plan.md:6289` | The module and the spike are right; the wizard collects the campus on panel 8 and calls `open_lms_window` on panel 5, so the window path can never run. Critical #3. |
| **§11a** — never request campus SSO credentials | Honoured | T13 `plan.md:5043`, T14 `plan.md:5232-5249`, T17 `plan.md:6155-6179`, `plan.md:7062` | Stated as a standing rule, pinned by two static tests, and named under *What is NOT in this plan*. (One of the two tests cannot pass as written — Important #9.) |
| **§11a** — grades wanted later, not designed | Honoured | `plan.md:7061` | Named under *What is NOT in this plan*, with the reason. |
| **§11a** — existing installs adopted in place | Honoured with a hole | T18 `plan.md:6573-6624` | Folder, settings, credentials and profile id untouched; exactly one file gained. But the overlay is un-dismissable offline — Critical #2. |
| **§10 VISION** — plain text on the machine; the cloud judges, syncs, bills | Honoured | T12 `plan.md:4893-4913` | `config/cloud.yaml` is four plain scalars; only the target *name* of the secret is in the vault. |
| **§10 VISION** — telemetry keyed to a pseudonymous id, never content | Honoured | T8 `plan.md:3484-3487`, T15 `plan.md:5596-5598` | `device` is not in `EventRow`; a course correction loses both names; a title correction is no row at all. |
| **§9 minors** — 18+ boolean + timestamp, no birthdate | Honoured | T1 `plan.md:724-732`, `plan.md:863-872` | A test greps five spellings of birthdate across every migration; the gate is enforced in the app *and* in the sign-up trigger. |
| **§9 subscriptions** — cancel flow, annual reminder, 3-year consent log | Honoured | T4 `plan.md:1832-1839`, T5 `plan.md:2440-2454`, T18 `plan.md:6769` | Consent logged *before* the redirect with version, price and IP; one-click Portal; `consents` survives deletion with `account_id` nulled. |
| **§9 privacy** — one policy, universal rights, no sale | Honoured | T6 `plan.md:2665`, T19 `plan.md:6907-6922` | `DELETE /account` + `GET /account/export` for everyone, no geo-fencing. |
| **R-C1-1** — `cloud.yaml` through the normal write path, no byte comparison | Honoured | `plan.md:159`, T11 `plan.md:4519-4524` | Written by `scaffold::write_file` → `pystr::write_text`; read by `pystr::read_text` + `serde_yaml_ng`. No byte compare anywhere. |
| **R-C1-2** — entitlement cache at `profiles\<id>\entitlement.json` | Honoured | H2 `plan.md:233`, T11 `plan.md:4555` | One `cache_path(data_dir)`; the scheduler gate (`plan.md:4628`), the console banner (`account_status`, `plan.md:6697`) and the wizard poll (`entitlement_now`, `plan.md:6718` — which fetches live, not from cache) all route through it or through the same fetch. `state.rs` untouched, and the hazard argument is verifiably correct. |
| **R-C1-3** — one email provider for the reminder **and** Supabase Auth's SMTP | **Half carried** | P2 `plan.md:51`, T5 `plan.md:2534` | The reminder uses Resend. Supabase Auth's built-in sender is left in place — no `[auth.email.smtp]` in `config.toml` (`plan.md:441-444`), and the word `smtp` does not appear in the plan. Important #6. |
| **R-C1-4** — the coursework round trip waits for C2 | Honoured | `plan.md:7056` | Stated verbatim under *What is NOT in this plan*. |
| **R-C1-5** — the issue report carries the view's *name* | Honoured (stale comment) | T16 `plan.md:6036` | The code writes `view {view}`. The doc comment two lines above still says "the keys of the view" — Minor. |
| **R-C1-6** — VISION §10 amendments committed | Honoured | ledger `plan.md:78-84` | Cited as `§10` rows. |

## Findings

### Critical (blocks execution)

`plan.md:163` — **The LMS URL is never sent to `PUT /account/sources`.** The contract (and the plan's
own *Interfaces with C2* item 3) says the URL is stored twice: server-side by `PUT /account/sources`,
and in `config/ingest.yaml`. Task 7 builds, tests and deploys the endpoint (`plan.md:2990-3221`), but
no Rust function and no page handler ever calls it — `lms_link::finish` (`plan.md:5447`) only
validates, `paste_calendar_link` (`plan.md:5459`) only validates, and the wizard puts `WIZ.ics`
into `WizardPlan.ics_url` (`plan.md:6410`) and nowhere else. Grep for `account/sources` in the plan
returns only the endpoint's own files. — **Why it matters:** C2's `/ingest/ics` reads that row
server-side (`c2-judge-plan.md:175`); with no writer, the `sources` table is empty for every account
and C2's entire ICS path is dead on arrival, which the C2 stream will discover only at integration.
It also means the one C1 endpoint that exercises `requireActiveEntitlement` in production
(`plan.md:2992`) is never exercised. — **Fix:** in `lms_link::finish`, after a successful `validate`,
POST `{"kind":"lms_ics","url":…}` to `{api_base}/account/sources` with the token from
`valid_access_token_at(auth, anon, PENDING_TARGET, …)`; report a 402 as "finish subscribing first"
and any other failure as a warning that does not block the panel (the vault copy still works). Add a
back-fill in Task 18's `attach_in` for an adopted vault whose `config/ingest.yaml` already has an
`ics_url`. Add one loopback test in `app/tests/lms_link.rs` asserting the request line and the body.

`plan.md:6830-6833` — **The upgrade overlay has no dismiss path, and no offline path.**
`maybeUpgrade` un-hides `#upgrade` whenever `account_status.needs_account` is true; the overlay's
markup (`plan.md:6813-6821`) has no Close button, unlike `#report` which has `report-cancel`
(`plan.md:6302`). Offline, `sign_up`/`sign_in` return `ok: false` and there is no way past it. —
**Why it matters:** this is the first launch after C1 for every existing install, including Quinn's
cut-day vault. Spec §5.1 and D4 both promise that a dead connection never hides today's page, and the
brief names blanking today's page on a dead connection as a Critical class. The 72-hour grace protects
the *scheduler* from exactly this and the overlay walks straight past it. — **Fix:** add a
"Not now" button that hides the overlay for the session (and a `#upgrade` that never covers the page
on a failed reachability check); keep the console fully usable underneath; re-show on the next launch.
Add a static test asserting the overlay carries a dismiss control, twinned with the one at
`plan.md:6634`.

`plan.md:6289` vs `plan.md:6465` — **The campus is collected on panel 8; the LMS panel is panel 5.**
`<div class="wiz-row" id="wiz-campus"></div>` sits inside `wiz-slots` (`plan.md:6286-6290`), and
`WIZ.campus` is only read by `readSlotsPanel()`, which `wizGo` calls when `leaving === 7`
(`plan.md:6386`). The LMS panel is index 4 and calls `invoke("open_lms_window", { campus: WIZ.campus })`
with `WIZ.campus` still at its initial `"none"` (`plan.md:6327`). `open_lms_window` looks the campus up
in `CAMPUSES` and refuses an empty URL (`plan.md:5120-5122`), so *every* press of "Sign in to my
school" answers "no sign-in page is known for that school yet". — **Why it matters:** §11a's headline
ruling — the whole of Tasks 13 and 14, and Quinn's time at the laptop for the spike — is unreachable
from the shipped wizard, and exit-gate item 1 ("connect their class calendar by signing in to their
school") cannot pass. — **Fix:** move the campus radio list onto the LMS panel, above the sign-in
button (it belongs there anyway: it is the question "which school?"), leave `readSlotsPanel` reading
only the timezone, slots and autostart, and add to `the_page_has_no_lms_credential_field_anywhere`
an assertion that `id="wiz-campus"` is inside the `wiz-lms` panel.

### Important (fix before execution)

`plan.md:6256` and `plan.md:6818` — **The terms and privacy links break the console window.** Both the
wizard's account panel and the upgrade overlay carry `<a href="terms.html">` and `<a href="privacy.html">`,
and `plan.md:6189-6191` asserts they are there, commenting "they are relative names the shell opens".
Nothing opens them: `app/static/` contains only `console.css`, `console.js`, `fonts/` and `index.html`;
there is no `tauri-plugin-shell` in `app/Cargo.toml`; and `app/src/main.rs` has no `on_navigation`
handler. A click navigates the one webview to a missing asset. — **Why it matters:** the user cannot
read the two documents they are ticking a box to accept, which is the legal core of §9's consent
requirements, and the app window is lost until restart. — **Fix:** give both links `id`s and a click
handler that calls a new `account::open_terms`/`open_privacy` (or one `open_policy(which)`) using the
existing `open_in_browser` (`plan.md:6734`) against `https://knowlu.com/terms.html`; keep the `href`
for the test and `preventDefault()` in the handler.

`plan.md:1633` and `plan.md:435` — **`site/subscribed.html` and `site/signed-in.html` are referenced
and never created.** Checkout's `success_url` is `https://knowlu.com/subscribed.html`
(`plan.md:1633`, `plan.md:1906`) and `config.toml`'s `additional_redirect_urls` is
`https://knowlu.com/signed-in.html` (`plan.md:435`). The plan's *Modified — site* list
(`plan.md:137-142`) and Tasks 19–20 create only `privacy.html`, `terms.html` and edits to
`index.html` and `site.css`. — **Why it matters:** every paying user lands on a 404 the moment they
finish Checkout, and every email-confirmation link does the same. Both are the first thing a new user
sees after paying. — **Fix:** add two small pages to Task 19's file list — "You're subscribed, go back
to Knowlu" and "You're signed in, go back to Knowlu" — or point both at `index.html#subscribed` and
say so in the copy.

`plan.md:51` — **R-C1-3's second half is missing: Supabase Auth still uses its built-in sender.** The
ruling was that the same provider be the custom SMTP for Auth's confirmation and magic-link mail,
because the built-in sender is rate-limited and not for production. `config.toml`'s `[auth.email]`
block (`plan.md:441-444`) has no SMTP settings and the word does not appear in the plan. — **Why it
matters:** `enable_confirmations = true` means every single sign-up depends on that mail arriving;
Supabase's default sender is a few messages an hour and is explicitly not for production. The wizard
cannot be completed by the second person to sign up in an hour. — **Fix:** add `[auth.email.smtp]`
(host, port, user, `admin_email`, `sender_name`) to `config.toml` with the values named and the
password as a project secret, extend P2's ask to include the SMTP credentials, and add a step to Task 4
that sends one real confirmation through staging.

`plan.md:64-93` and `plan.md:48-54` — **The fidelity ledger and the preconditions table point at the
wrong task numbers.** Two tasks (the spike and `lms_link.rs`) and one task (adopt-in-place) were
inserted mid-flight and the preamble was never renumbered: the ledger says "Task 15 removes the
wizard's folder step" (`plan.md:74`, it is Task 17), "Task 15's Gmail panel" (`plan.md:77`, Task 17),
"Tasks 8, 9, 13, 14" for D5 (`plan.md:70`, they are 8, 9, 15, 16), "Task 13's value allow-list"
(`plan.md:85`, Task 15), "Task 16's privacy policy" (`plan.md:76`, `plan.md:92`, Task 19), "Task 17
starts the verification track" (`plan.md:77`, Task 20); P4 says "Task 16 publishes it" and "Task 17
hands Quinn" (`plan.md:53`, Tasks 19 and 20); P5 is on Task 16 (`plan.md:54`, Task 19). Task bodies
drift too: `plan.md:3235` ("`app/src/telemetry.rs` (Task 14)"), `plan.md:3619` and `plan.md:3610`
("Task 15" for `report.rs`), `plan.md:130` ("`write_cloud_yaml_if_absent` … (Task 17)", it is 18),
`plan.md:4004` ("Tasks 10, 11 and 17", the module doc should say 18). — **Why it matters:** the plan is
executed subagent-driven, task by task, and the ledger is what a worker checks a task against.
Precondition P5 attached to Task 16 means Quinn is asked to read a privacy policy three tasks before
it is written. — **Fix:** renumber the ledger, the preconditions table and the six in-task references
in one editing pass; the offsets are +2 for telemetry/report/wizard and +3 for privacy/Google/close.

`plan.md:6235-6239` vs `plan.md:6901` — **Task 17 breaks the test suite until Task 19 lands.** Task 17
step 1 rewrites `the_wizards_privacy_sentence_is_the_sites_privacy_sentence` to look for
`"Your vault stays on this machine"` in `site/privacy.html`; Task 19 is what writes that sentence.
The test `.expect("site/privacy.html must carry the privacy sentence in one <p>")` panics until then,
so Task 17 step 8's `cargo test -p knowlu` (`plan.md:6565`) and Task 18 step 5 (`plan.md:6884`) are both
red. — **Fix:** move Task 19 step 1 (the one sentence, into `site/privacy.html` and `console.js`)
into Task 17 as its own step, leaving the rest of the prose in Task 19; or move Task 19 before Task 17.

**Three of the new tests cannot pass against the plan's own source.** Each blocks its task's green step,
and the plan forbids weakening an assertion to get past one.
- `plan.md:6176` asserts `!panel.contains("password")` over the `wiz-lms` panel, and `plan.md:6271`
  writes "…keeps only the calendar link &mdash; never your password." into that panel. — **Fix:** the
  sentence should say "never what you type into it"; or assert `!panel.contains("type=\"password\"")`,
  which is what the test actually means.
- `plan.md:5237` asserts `!src.contains("app_data_root")` over `app/src/lms_link.rs`, and
  `plan.md:5075` writes "`state::app_data_root_in`'s" into that file's doc comment. — **Fix:** reword
  the comment to "the app's own data root", or assert on `crate::state::app_data_root`.
- `plan.md:5576` computes `listed = ts.split("export const ACTIONS").nth(1).and_then(|s| s.split(']').next())`
  to count the cloud's action list, but the declaration is `export const ACTIONS: readonly string[] = [`
  (`plan.md:3463`), so the split on `]` stops at `string[]` and `listed` holds no quotes at all —
  `0 / 2 != 11`. — **Fix:** split on `"= ["` first, then on `"]"`; or count with a regex over the
  quoted literals between `= [` and the next `];`.

`plan.md:2073` and `plan.md:2628` — **`entitlementFromSubscription` reads a field Stripe removed.**
`sub.current_period_end` moved from the Subscription object onto `items.data[].current_period_end` in
API version `2025-03-31.basil`; a Stripe account created now defaults well past that. The two handler
tests pass because they feed synthetic objects with the old shape, and the staging exercise
(`plan.md:2250`) deliberately uses a test event that carries no `account_id` and takes the `ignored`
path — so nothing in the plan would catch it. — **Why it matters:** every entitlement row would carry
`current_period_end: null`, which the console banner shows and which the plan's own consumers read.
— **Fix:** `const end = sub?.items?.data?.[0]?.current_period_end ?? sub?.current_period_end;` and
add a test fixture in the item-level shape. Pin the Stripe API version explicitly in
`stripePostFrom` with a `Stripe-Version` header so this cannot move under the code again.

`plan.md:2122-2124` and `plan.md:2081-2084` — **The `invoice.payment_failed` branch is dead code.**
`accountIdFromEvent` reads `event.data.object.metadata.account_id` or `.client_reference_id`. On an
Invoice, `metadata` is the *invoice's* own metadata, which is empty; subscription metadata is not
inherited (it is on `subscription_details.metadata` in current API versions), and there is no
`client_reference_id`. Every `invoice.payment_failed` therefore returns `{ignored: "no account id on
the event"}` before `fetchSubscription` is ever reached. — **Why it matters:** the event is subscribed
to in P2's webhook list (`plan.md:1458`) and the plan believes it produces the `past_due` state.
It happens to be harmless — `customer.subscription.updated` fires with `status: past_due` too — but
the code is a promise that is not kept. — **Fix:** for invoice events, take the account id from
`o.subscription_details?.metadata?.account_id`, falling back to a customer→account lookup; or drop
the event from both the subscription list and the handler and say why.

`plan.md:2092-2130` — **The webhook has no idempotency key and no ordering guard.** Stripe retries for
days and does not guarantee order. The write is an upsert keyed on `account_id`, so a retry is
harmless, but a late `customer.subscription.updated` arriving after `customer.subscription.deleted`
overwrites `canceled` with `active`. The `updated_at` column exists (`plan.md:2160`) and is never
compared. — **Fix:** store `stripe_event_id` and skip an event already seen (a small `webhook_events`
table, or a unique constraint), and gate the write on the event's own `created` timestamp being newer
than the row's `updated_at`. One extra column and one `and` in the upsert filter.

`plan.md:2317-2341` — **The summer pause is applied to academic-year subscribers.** `shouldBePaused`
is a function of the date alone (`plan.md:2420`) and `handle` loops over every row of
`billing_subscribers` regardless of `plan`; the test at `plan.md:2338` pins `{paused: 2, …}` for one
monthly and one `academic_year` subscriber, so the behaviour is deliberate and locked in. With
`pause_collection[behavior] = "void"`, an annual renewal invoice that falls in June, July or August is
voided and the subscription continues — a free year. — **Why it matters:** R3 kept the academic-year
price *because* it already prices the summer in; pausing it as well is the revenue hole R3's own
ruling was weighing. — **Fix:** `if (s.plan !== "monthly") continue;` before the `pauseDecision`
switch, and change the test to `{paused: 1, …}` with a comment saying why the yearly plan is exempt.

`plan.md:6794-6796` — **`delete_my_data` deletes the shared backups root.** `Settings.backup_dir` is
the *root* — Task 12 says so explicitly (`plan.md:4959`: "`backup::tick` writes `<root>\<profile_id>\vault`,
so the setting is the root") and `default_folders_in` returns `%USERPROFILE%\Knowlu\Backups` for every
profile. `remove_dir_all(b)` therefore destroys every other profile's snapshots on that machine.
The same function also leaves the profile's row in `profiles.json`, so the picker keeps offering a
vault that no longer exists. — **Fix:** delete `backup_dir.join(&profile_id)`, not `backup_dir`; and
call the existing profile-removal path (or add one) so `profiles.json` no longer names it. Add a test
with two profiles under one backups root asserting the other survives.

`plan.md:5774-5777` — **The telemetry watermark can skip events permanently.** `events` and
`corrections` are truncated to 500 independently, then `high` is the maximum `ts` across *both*
truncated lists and the watermark is advanced to it on success. A vault with 600 pending events and
one correction at a later timestamp advances the watermark past events 501–600, which are never sent.
The comment at `plan.md:5766-5768` claims the opposite ("catches up over several slots"). — **Fix:**
compute `high` as the minimum of each stream's own last `ts` whenever that stream was truncated
(`min(events.last().ts, corrections.last().ts)` when either hit the cap), or keep two watermarks.

`plan.md:713-722` — **C1's migration test fails the moment C2 merges.** `migrations()` reads every
`.sql` in `cloud/supabase/migrations/` and the first test asserts `m.name.startsWith("20260910")`.
C2 puts four `20260911…` files in the same directory (`c2-judge-plan.md:110-113`). Merge order is
C0 → C1 → C2, so C1 goes green and then C2's first commit turns C1's suite red for a reason that has
nothing to do with C2's change. C2's own test already filters (`c2-judge-plan.md:705`). — **Fix:**
filter C1's `migrations()` to `20260910` and assert the stamp shape on all of them, exactly as C2 does.
The same helper feeds three other tests (`plan.md:734`, `plan.md:749`, `plan.md:3315`) whose regexes
require a `public.` prefix that C2's SQL does not use — so after the fix they must either keep
scanning only C1's files (honest) or be widened to match both spellings (better; see cross-check).

`plan.md:1632` and `plan.md:1458` — **`consent_collection[terms_of_service] = required` needs a
Checkout ToS URL that P2 never asks for.** Stripe rejects a session with that parameter unless a terms
of service URL is configured in the account's Checkout settings. P2 asks for the product, the two
prices, the Portal and the webhook, and nothing about the ToS URL. — **Why it matters:** every
Checkout session 400s, so the wizard cannot be finished end to end and the failure surfaces as
`the payment provider refused the request` with no clue. — **Fix:** add to P2's ask: "in Checkout
settings, set the terms of service URL to `https://knowlu.com/terms.html`" — which also sequences
Task 19 before Task 4's first live Checkout.

`plan.md:4186-4189` and `plan.md:6446-6452` — **Magic-link sign-in cannot complete on the device.**
`magic_link_at` posts to `/otp`; the link in the mail redirects to `https://knowlu.com/signed-in.html`
with the session in the URL fragment, in the user's *browser*. The app never sees it, and the wizard's
own copy admits as much ("Check your email, then come back and sign in"). Spec §5.1 lists magic-link
sign-in as one of the two identity paths. — **Fix:** either implement the callback (a deep-link
scheme registered to the app, or the `verify` endpoint with a token the user pastes) or cut the
button and say in the ledger that magic link is deferred, so the plan does not ship a control that
cannot do what its label says. Cutting it also removes three names from H3, H5 and H6's lists.

### Minor (fix in flight or defer)

`plan.md:6583` — `attach_in` is advertised as `attach_in(root, vault, profile_id)` and written as
`attach_in(vault, profile_id, pending_target)` (`plan.md:6663`). Fix the *Interfaces* line.

`plan.md:2954-2956` — the export's comment says the capability URL "is returned here, decrypted,
precisely because this is the access right", and the code selects `kind,added_at`. Decide which is
true; if the comment is, decrypt it here (the access right does cover it) and adjust the test at
`plan.md:2788`.

`plan.md:6028-6029` — `preview_text`'s doc comment says the report carries "the keys of the view the
user was looking at". R-C1-5 ruled the *name*, and the code writes the name (`plan.md:6036`). Reword.

`plan.md:2593` — `create extension if not exists pg_cron with schema extensions;` — pg_cron is not
relocatable and Supabase installs it into its own fixed schema; the `with schema` clause is likely to
fail the migration outright. Use Supabase's documented form and keep the `cron.schedule` calls as they
are. Task 5 step 8 catches it immediately, but a plan that names the right form saves a cycle.

`plan.md:3532` — the cloud accepts `object_id` as an `isToken`, where the engine requires
`ids::is_id` (`engine/src/uievents.rs:45`). Looser than the emitter; harmless, but the comment at
`plan.md:3484` claims "`uievents::is_token`, exactly" and that is only true of `session`/`view`.

`plan.md:5697-5729` — class (b) is implemented as field overrides only. Spec §6 also names amend-card
decisions, snoozes of proposals and declined events. Neither built nor declared under *What is NOT in
this plan*. Add a line there, or add the three cases.

`plan.md:60-93` — the fidelity ledger has a row for R1–R6 and R8 and none for R7 (ruled "later").
Add a one-line row pointing at `PROVENANCE.md`, so the ledger is complete against §11.

`plan.md:6121` — replacing `the_wizard_has_seven_panels_…` silently drops its
`assert!(!js.to_lowercase().contains("telemetry"))` pin. Nothing in Task 17 puts the word into
`console.js`, so the pin could survive as-is and keep proving that (c) has no UI. Keep it, with a
comment naming D5.

`plan.md:5953-5985` vs `plan.md:3697-3703` — the two scrubbers are called twins and agree on the six
pinned cases, but not in general: Rust replaces a whole space-delimited segment containing `://` with
`<url>`, so `see(https://x/y)` becomes `<url>` in Rust and `see(<url>` in TypeScript. Say so in the
comment, or make the Rust side regex-based on the same four patterns.

`plan.md:3690-3695` and `plan.md:5957` — neither scrubber redacts a Windows path. `log_tail` reads
the engine's stdout, which is full of `C:\Users\<the user's Windows account name>\Knowlu\…`, and the
report is the one payload a human reads. Add a `C:\Users\<name>` → `C:\Users\<user>` rule to both,
and a case to the six.

`plan.md:6224-6231` — `the_wizard_never_offers_a_local_model` slices `console.js` between
`function startWizard(` and the wizard's click listener, and asserts the three inference commands are
absent from that slice. Whether `openSettings` (which legitimately calls them until C4) falls inside
that slice is a fact about line ordering, not about the wizard. Slice on the wizard's own function
names instead.

`plan.md:6621` — the second `attach_in` call is expected to fail, and it does — at
`load_session(PENDING_TARGET)`, not at `write_cloud_yaml_if_absent`. Assert on the error text so the
test proves the refusal it is named for.

`plan.md:6805` — `delete_my_data` calls `app.exit(0)` and then returns an envelope the page's `.then`
will probably never run. Harmless (the page's copy says Knowlu will close) but the return is theatre;
exit after a short delay, or say so.

`plan.md:642` — `readJson`'s cap compares `text.length` (UTF-16 code units) and the error says
"bytes". One or the other.

## Cross-check against the C2 plan's interfaces

C1's *Interfaces with C2* is `plan.md:146-197`; C2's *Interfaces with C1* is
`c2-judge-plan.md:152-191`, and its hand-off H6 (`c2-judge-plan.md:283-354`) edits `app/src/scheduler.rs`,
which the ownership contract gives to C1.

| Item | C1 says (line) | C2 says (line) | Agree? |
|---|---|---|---|
| `config/cloud.yaml` — the four keys and their order | `plan.md:150-157` | `c2:156-163` | **Yes** (C1 writes single-quoted scalars, C2's example is double-quoted; both parse, and C1 forbids a byte compare at `plan.md:159`) |
| Session JWT: target, `UserName` = account id, JSON blob, C1 refreshes | `plan.md:161` | `c2:165` | **Yes** |
| `requireActiveEntitlement(req): Promise<{account_id}>`, throws a `Response`, 401 / 402 | `plan.md:186-192` | `c2:167-173` | **Yes** — signature, semantics and the caller's `try/catch` shape all match `plan.md:1277-1280` |
| `sources` columns | `plan.md:826-833` — `account_id, kind, url_ciphertext, url_iv, added_at` | `c2:182` — `(account_id, kind, url, added_at)` | **No** — C2 expects a column named `url`; there is none |
| Who decrypts the stored URL, and with what | `plan.md:3052-3096` (`_shared/crypto.ts`, `SOURCES_ENC_KEY`) | `c2:175` — "reads it server-side with the service role" | **No** — C2 never mentions decryption, the shared module or the secret |
| Who creates `public.corrections` | `plan.md:3261` (`20260910000400_telemetry.sql`, plain `create table`) | `c2:857` (`20260911000100_judgment_service.sql`, `create table if not exists`) | **No** — both claim it; C1 runs first, so C2's statement silently no-ops |
| `corrections` columns | `plan.md:3261-3272` — `ts`, `received_at`, `ours`/`theirs` nullable, no `judgment_id` | `c2:180-190` — `corrected_at`, `ours`/`theirs` **not null**, `judgment_id` | **No** — and C2's index `corrections (kind, corrected_at desc)` (`c2:869`) fails against C1's table, breaking C2's migration |
| `corrections.kind` vocabulary | `plan.md:5665` — `kind_of(path)` yields `task`, `approval`, `course`, `info`, `issue`, `archive` | `c2:860` — `check (kind in ('task','event','email'))` | **No** — an approval-card correction would be rejected by C2's constraint |
| Nullability of `ours`/`theirs` for a flagged field | `plan.md:3557-3558` — deliberately `null` for `course` | `c2:186-187` — `not null` | **No** — C2's shape would reject exactly the rows C1's content rule produces |
| The `judge` step's argv | `plan.md:165-176`, `plan.md:4683` — `judge --vault <v> --via local-runner` **and nothing else** | `c2:324-327` — adds `--log-dir <dir>` | **No** |
| The type that gates `judge` in `scheduler.rs` | `plan.md:4648` — a new `enum JudgePlan { Cloud, Local, Skip }`, `slot_argv(…, &JudgePlan)` | `c2:295-338` — `JudgeState::Cloud { log_dir }`, `slot_argv(…, &JudgeState)` | **No** — two incompatible rewrites of the same two functions |
| Entitlement's place in the slot decision | `plan.md:4657-4667` — entitlement outranks everything; past grace the step is not run | `c2:344-349` — no entitlement branch; a cloud vault always runs `judge` | **No** |
| `ingest` when `cloud.yaml` exists | `plan.md:4676` — `if has_ics_url(vault)` | `c2:320` — `if has_ics_url(vault) \|\| cloud` | **No** (a later, deliberate change by C2 — but to a function C1 has rewritten differently) |
| Telemetry ownership: (a)+(b) to C1's `POST /telemetry`, C2 never posts | `plan.md:194` | `c2:177` | **Yes** |
| Migration numbering `20260910…` / `20260911…` | `plan.md:31`, `plan.md:720` | `c2:110-113`, `c2:705` | **Yes in principle** — but C1's test asserts it over *every* file in the directory (Important, above) |
| `cloud/supabase/functions/_shared/**` ownership | `plan.md:40` — C1 owns all of `cloud/supabase/**` except `functions/{judge-*,ingest-*,events,gmail-*}` | `c2:114-121` — C2 creates eight `_shared/judge_*.ts` | **No** — ownership overlap the merge will hit |
| `config.toml` and `deno.json` for C2's functions | `plan.md:100-101`, `plan.md:451-467` — C1's files; `verify_jwt = false` listed per function | `c2` — never mentions either | **No (gap)** — C2's functions get no `[functions.*]` entry, so Supabase's gateway rejects with *its* 401 shape instead of C1's, and `@anthropic-ai/sdk` (`c2:114`) has no import-map entry |
| The `deno` CI job | `plan.md:331-343` — job `cloud`, `deno test --allow-read`, `--config cloud/supabase/deno.json`, scoped to `cloud/supabase/` | `c2:361-377` — jobs `cloud-test` + `eval-gate`, `deno test --allow-read=. --allow-net=127.0.0.1 cloud/`, no `--config` | **No** — two hand-offs adding overlapping jobs; C1's flags would fail C2's loopback tests |

**Twelve mismatches.** The two that must be settled before either stream starts writing SQL are the
`corrections` table (who owns it, what its columns are called, and whether `ours`/`theirs` may be
null) and `sources.url` vs `url_ciphertext`/`url_iv` — both are migrations, and a migration that has
been applied to staging is not free to change. The `scheduler.rs` cluster is settled by ownership:
`scheduler.rs` is C1's, C1 Task 11 already delivers what C2's H6 exists to achieve (a vault with
`cloud.yaml` runs `judge`), and C2's H6 should be reduced to the two things C1's version does not
have — `--log-dir` on the cloud arm, and `ingest` running on a cloud vault with no `ics_url` — stated
as a diff against `JudgePlan`, not a replacement of it.

**C2's H6, specifically, as the brief asks:** C1 Task 11 does deliver H6's substance. `judge_plan`
returns `JudgePlan::Cloud` for any vault with a readable `config/cloud.yaml` and a live entitlement
(`plan.md:4657-4667`), `slot_argv` emits the `judge` step for it (`plan.md:4680-4684`), and
`app/tests/scheduler.rs` gains the case C2 names as "C1's file to add"
(`an_entitled_vault_runs_judge_with_no_runtime_and_no_account_on_the_command_line`, `plan.md:4455`).
Two differences remain: C1 refuses `--log-dir`, and C1 adds an entitlement gate C2 does not know
about, so a C2 test that expects `judge` to run on a cloud vault with a stale or absent
`entitlement.json` will see a skip.

## Controller hand-offs — checked

- **H1** (`app/src/lib.rs` + four stub modules) — **complete.** Pre-flight, compile-blocking, correctly
  identified as such; `app/src/lib.rs` is thirteen `pub mod` lines today, so "add four, alphabetically"
  is exact. The four doc comments carry stale task numbers (Minor, above).
- **H2** (why the cache is not a `Settings` field) — **complete, and verified.** `state::Settings` has
  no `#[serde(default)]` on any of its four fields and `Settings::load` falls back to defaults on a
  parse failure (`app/src/state.rs:13-33`), exactly as claimed. No edit needed; correctly recorded.
- **H3** (`main.rs`, the two `generate_handler!` lists and the `use` line) — **complete and exact.** I
  counted the current lists: the shell registers 11, the console 29 (26 `commands::*` + three
  `onboarding::*`), matching the plan. The new lists are spelled out in full, the six-in-both-windows
  decision is argued, and the `use` line is given verbatim. The arithmetic holds if the magic-link
  button survives (Important, above); if it is cut, the numbers become 20 / 40 / 51.
- **H4** (`tray.rs`, two menu items) — **complete.** `build()` really does make seven `MenuItem::with_id`
  calls today; the `Menu::with_items` line and the two `on_menu_event` arms are exact, and
  `knowlu_engine::childproc::NoConsole` exists with that method name. The argument for *Delete my data*
  not being a tray item is sound.
- **H5** (`app/README.md` recount) — **complete**, and correctly made contingent on Task 21 step 2's
  hand count rather than on the plan's arithmetic.
- **H6** (`scripts/wizard-check.py`) — **incomplete by design, and the design is wrong.** The hand-off
  says the controller replaces the script's `check()` with "the exact text Task 17 step 9 prints", and
  Task 17 step 9 (`plan.md:6567`) describes that text in prose rather than producing it. A hand-off
  whose payload is generated at execution time cannot be reviewed now and cannot be applied by a
  controller who was not in the room. Either give the replacement `check()` here, or make it an
  explicit deliverable of the stream's report with a named acceptance ("the script exits 0 against
  the nine-panel wizard").
- **H7** (`ci.yml`, the `deno test` job) — **complete for C1, conflicting with C2.** The job is exact,
  the SHA-pinning rule is named against the real test
  (`engine/tests/workflows.rs::every_action_is_pinned_to_a_full_commit_sha`), and the advice not to add
  it to branch protection in the same commit is right. See the cross-check row.
- **H8** (`CLAUDE.md`, two edits) — **complete.** Both replacement sentences are given verbatim and both
  are the kind of claim `CLAUDE.md` asks to be recounted before quoting.
- **H9** (`main.rs`, the four `lms_link` commands, mid-stream) — **complete**, and correctly flagged as
  the plan's only mid-stream pause with the reason (a window cannot be opened from an unregistered
  command).

## Testing rules — checked

- **0 warnings, dev profile, four `#[ignore]`s untouched** — stated at `plan.md:26`, `plan.md:30`, and
  every task's run step names its expected counts. Task 11 step 5 (`plan.md:4716`) is exemplary: it
  names the two existing scheduler tests that must *not* change and says that if they fail, the
  ordering in `judge_plan` is the bug.
- **No test reaches the network** — honoured. The Rust loopback harness (`plan.md:1882-1928`) binds
  `127.0.0.1:0`, serves exactly N responses and is `join()`ed before each test returns
  (`plan.md:1955`, `:3974`, `:4376`); it reads the head, then exactly `Content-Length` bytes, which is
  the failure mode a naive single `read` has. Deno tests inject `fetch` as a field on `Rest`
  (`plan.md:1085`) and on every `Deps`, so no handler test can reach out. `check_api_base`
  (`plan.md:4050`) permits `http://` only for `127.0.0.1:`, and the offline-telemetry scheduler test
  uses port 9 (`plan.md:5817`). One gap: `deno test` still fetches `@std/assert` from JSR on a cold
  cache — true and stated (`plan.md:516`), but worth `--frozen`/vendoring in CI.
- **`static_assets.rs` pins the wizard's step set and the absence of `http(s)://`** — honoured and
  strengthened. The nine-panel list is pinned by id *and* by the `PANELS` array literal
  (`plan.md:6139`), the absence of `wiz-backup` and both folder pickers is asserted
  (`plan.md:6133-6134`), and three genuinely new pins are added (no LMS credential field, the 18+ and
  terms checkboxes, the report preview). The `no_network_reference_in_the_shipped_page` rule survives:
  `api_base` and `anon_key` are Rust constants (`plan.md:4021-4022`), campus URLs are the third column
  of `CAMPUSES` in Rust (`plan.md:4859`), and the only literals added to the page are the relative
  `terms.html` / `privacy.html` (which is its own problem — Important, above). One pin is dropped
  without comment (`!contains("telemetry")`, Minor) and one is rewritten a task too early (Important).
- **Every Tauri command computes nothing itself; every vault write goes through the engine's `write`
  with `console_ctx()`** — honoured, and the one apparent exception is argued. C1 adds no note-writing
  command: the two new mutators are `attach_account` (writes `config/cloud.yaml`, a config file, through
  `scaffold::write_file` → `pystr::write_text`, exactly as `runners.yaml` is written at birth) and
  `delete_my_data` (removes the vault entirely). Neither is a note edit, so `journal::VIAS` does not
  grow and no note is parsed and re-dumped. `plan.md:24` states this and it holds.
- **Deno modules under `deno test`, thin `index.ts`, one contract test per function, migrations to
  staging only** — honoured throughout. Every `index.ts` is env → `Deps` → `Deno.serve`; every
  `handler.ts` is pure; every function has a `handler_test.ts`; `plan.md:32` and every deploy step
  names `knowlu-staging`, and `plan.md:7063` repeats it under *What is NOT in this plan*.
- **No secret anywhere** — honoured. `plan.md:23` names the six secrets and shows no value; P2's ask is
  phrased so Quinn generates and sets each one himself; the two test "secrets"
  (`whsec_test_only_not_a_real_key`, 32 zero bytes) are labelled as literals invented for the test;
  `DEFAULT_ANON_KEY` is a public value and the comment explains why publishing it is safe.
- **No single-user assumptions** — honoured. Nothing names a person's vault, machine, account or
  credential. `%USERPROFILE%` and `profiles::id_for(dest)` are derived; `CAMPUSES` keeps its existing
  two entries and the plan says adding one is a file plus a line (`plan.md:7065`). The one thing worth
  a second look is the test vault named `"Quinn Hall"` at `plan.md:6593` — it is a temp-directory name
  inside a test, not a configuration value, so it is fine, but `"Fall 2026"` (used elsewhere) would be
  a better habit.

## Security and legal — checked

- **RLS on every table** — yes, and enforced by a test that greps `create table … public.<name>`
  against `alter table … enable row level security` (`plan.md:734-747`). All nine C1 tables are covered.
  The regex requires the `public.` prefix, which C1 uses consistently and C2 does not (cross-check).
- **The service role never in the app** — yes. It is read only from `SUPABASE_SERVICE_ROLE_KEY` inside
  `restFromEnv` (`plan.md:1167-1172`), which lives in `_shared/db.ts` and is called only from
  `index.ts` files. Nothing in `app/src` knows the name.
- **JWT verification in `_shared`** — yes, delegated to GoTrue's `/auth/v1/user` with the caller's own
  token (`plan.md:1149-1156`), injected as `VerifyToken` everywhere so no handler can skip it. The
  `verify_jwt = false` on every function is explained, not laxity: the gateway's rejection has a
  different body, and the 401/402 shape is a contract with C2 and with `account.rs`.
- **The webhook's signature check** — yes, and correct: the raw body is read as text and verified
  *before* it is parsed (`plan.md:2094-2104`), the comparison is over every `v1` in the header, the
  five-minute tolerance is enforced, and the compare is length-independent. The test proves all four
  failure modes. Missing: idempotency and ordering (Important, above).
- **The webhook is the only writer of `entitlements`** — yes, and structurally, not by convention:
  RLS grants `select` only, a test refuses any policy that is not `for select` (`plan.md:749-757`), and
  the staging check `select count(*) from pg_policies where cmd <> 'SELECT'` → 0 is in the exit gate.
  **Neither the trial nor the pause bypasses it:** the trial is created by Checkout and lands via
  `customer.subscription.created`; `billing-jobs` calls Stripe and writes only `billing_reminders`
  (`plan.md:2544-2545`), never `entitlements` — the `paused` flag comes back through the webhook.
- **`DELETE /account`'s cascade and the 90-day tombstone** — yes, and the ordering is argued and pinned
  by an order-recording test (`plan.md:2732-2763`): Stripe first (cancel at period end, not
  immediately — "deleting is not a refund"), then the rows, then the tombstone, then the auth user
  **last**, because deleting it first would invalidate the token authenticating the rest of the request.
  The tombstone is `(email_hash, deleted_at)`, swept daily at 90 days, with no read policy at all.
  `consents` survives with `account_id` nulled — the right resolution of a retention duty against a
  deletion right, and commented as such.
- **The 18+ boolean + timestamp with no birthdate** — yes, twice: refused in Rust before the request
  (`plan.md:4236-4239`) and again in the sign-up trigger so a patched client gets an account it cannot
  use (`plan.md:859-872`), with no birthdate column anywhere and a test that greps five spellings.
- **ToS/privacy version + timestamp logged** — yes. The versions are Rust constants
  (`plan.md:4027-4028`) sent as user metadata, the trigger refuses a sign-up missing either
  (`plan.md:873-875`), three `consents` rows are written in the same transaction, and Task 19 step 1
  ties the constant to the page's own effective date.
- **The cancel link in-app** — yes: a settings row (`plan.md:6312`) → `open_portal` → Stripe's Portal
  with cancellation on and no survey, plus the same link in every billing email (`plan.md:2450`,
  `plan.md:2466`). P2 asks for "no cancellation survey" explicitly.
- **The 3-year consent log** — yes, `consents` with `price_cents` and `ip`, written *before* the
  redirect with the argument that a customer who abandons still saw the terms (`plan.md:1743-1745`).
- **Stripe Tax** — yes: `automatic_tax[enabled]`, `customer_update[address]=auto` and
  `billing_address_collection=required` are all pinned by the form test, and P3 asks for the Kentucky
  registration with the reason and the Alabama question framed for a CPA.
- **The pause as `pause_collection`** — yes, `{"pause_collection[behavior]": "void"}` to pause and
  `{"pause_collection": ""}` to resume, with a resume notice sent in the same pass and a test asserting
  the mail names the amount and how to cancel. Scoped too widely (Important #13).
- **The trial as Stripe's trial with card up front** — yes, and both halves are asserted together.
- **The sign-in window's ephemeral data directory and the discard after capture** — yes, and it is the
  strongest part of the LMS work. The directory is `std::env::temp_dir()/knowlu-lms-session-<pid>-<id>`,
  unique per capture (`plan.md:5076-5078`), passed to `WebviewWindowBuilder::data_directory`, deleted by
  `close_and_wipe`, and the panel closes it on leaving whether or not a link was captured
  (`plan.md:6389-6392`). A source-reading test pins that the path is the temp folder's, that
  `remove_dir_all` is called, and that two captures never share it. The window gets no capability grant
  — which I verified: `app/capabilities/default.json` is scoped `"windows": ["main"]`, so a window
  labelled `lms-signin` can reach no Tauri command at all. Task 13 correctly treats a missing
  `data_directory` API as a **stop**, not a workaround.
- **Nothing asks for a campus SSO credential** — yes. No LMS password field, a static test that every
  `type="password"` on the page belongs to a named allow-list, and the standing rule restated in three
  places including *What is NOT in this plan*.
- **Gmail-derived content out of (c)** — moot and safe: (c) is not built, there is no export path for
  training data, and the Gmail panel calls no endpoint.

## What I did not verify

- **Nothing was executed.** No `cargo`, no `deno`, no `supabase`, no `git` command was run; every
  claim about the existing code comes from reading it. In particular I did not compile any of the
  plan's Rust or TypeScript, so type errors, `deno lint` findings and borrow-checker complaints are
  outside this review. The `ureq 3.4` call chains
  (`Agent::config_builder().timeout_global(…).http_status_as_error(false).build().into()`,
  `res.body_mut().with_config().limit(n).read_to_string()`) I checked only for internal consistency
  across the plan's eight uses, not against the crate's API.
- **The Stripe, Supabase and Google surfaces** are assessed from knowledge, not from their current
  documentation: the `current_period_end` relocation, the `consent_collection[terms_of_service]`
  prerequisite, the pg_cron schema constraint and the invoice-metadata behaviour are all findings you
  should confirm against today's docs before acting on them. I did not fetch anything.
- **Tauri 2 API existence** — `WebviewWindowBuilder::data_directory`, `WebviewWindow::navigate`,
  `url()` and `cookies_for_url` are exactly what Task 13's spike exists to establish; I took the spike
  at face value and did not check the pinned version's surface.
- **The C2 plan** was read only at the ranges the brief named — its *Interfaces with C1*
  (152–195), its hand-offs through H7 (195–408), and targeted greps for its `corrections`/`sources`
  tables, its migration filenames, and for `config.toml`/`deno.json`/`verify_jwt` (which return
  nothing). Mismatches outside those ranges are not covered.
- **The legal note** (`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`) was not opened; I
  checked Task 19's section list against the spec's §9 table and the brief's enumeration, not against
  the note's own rows.
- **`scripts/wizard-check.py`** was not read, so H6's claim about what its `check()` currently walks is
  taken on trust.
- **Prose quality** of the privacy policy and terms cannot be reviewed — they do not exist yet, by
  design. I checked only that Task 19's requirement list covers every binding sentence the brief
  enumerated, which it does.

## Controller rulings on this review and the cross-check (2026-09-09)

One authority per shared interface; the other plan adopts it verbatim.

- **R-X-1 `sources`:** C1's schema (`url_ciphertext`, `url_iv`, `_shared/crypto.ts`, secret `SOURCES_ENC_KEY`). C2 imports C1's `decryptString`/`importAesKey`.
- **R-X-2 `corrections`:** C1 creates it in its telemetry migration with its columns (`ts`, `received_at`, nullable `ours`/`theirs`) and its `kind` = the note kind (`task|approval|course|info|issue|archive`). C2 never creates it; C2 adds, in its own migration, `alter table … add column if not exists judgment_id uuid null`, `add column if not exists judgment_kind text null check (judgment_kind in ('task','event','email'))`, and its index on C1's `(judgment_kind, ts desc)`. No check constraint on `kind` from C2.
- **R-X-3 telemetry (b) `request`:** per R-C2-4 in the C2 review, (b) rows gain an optional `request jsonb` present only under the (c) opt-in, never for `origin = gmail_api`. C1 defines it; C2 reads it for eval cases.
- **R-X-4 the scheduler:** C1's types and functions (`JudgePlan { Cloud, Local, Skip }`, its `slot_argv`) are the only rewrite; entitlement outranks everything. The Cloud arm's argv is `judge --vault <v> --via local-runner --log-dir <profile logs dir>` — `--log-dir` stays in both arms because judgment logs never enter the vault (CLAUDE.md) and only the app knows the profile's log folder. C2 deletes its H6 code and states the dependency. The `ingest` gate stays `has_ics_url(vault)` in C1; C2 Task 8's hand-off adds `|| cloud` against C1's merged shape.
- **R-X-5 `_shared/` ownership:** `cloud/supabase/functions/_shared/judge_*.ts` are C2's; every other `_shared` file is C1's. C1's ownership line states the exception.
- **R-X-6 `config.toml` and `deno.json`:** C1's files. C2 supplies, as a controller hand-off applied at C2's merge, the exact `[functions.<name>] verify_jwt = false` entries for its nine functions and its `deno.json` import-map lines (`@anthropic-ai/sdk` pinned).
- **R-X-7 the Deno CI job:** one `cloud` job, C1's definition (`deno test --config cloud/supabase/deno.json`, scoped to `cloud/supabase/`); C2's hand-off amends that job (paths and `--allow-net=127.0.0.1` only if its loopback tests need it) and adds `eval-gate` as a second job. The controller merges both into `ci.yml`.
- **R-X-8 migration-numbering test:** C1's test asserts `20260910…` only over C1's own files, never over the directory, so C2's `20260911…` files pass.
- Everything else in Critical/Important is the writer's to fix as written.

---

## Re-review of fix round 1 (2026-09-09)

Scope as set by the controller: verdict every Critical and Important finding, verdict each R-X item,
check the two latitude decisions against everything else I reviewed, judge the deferrals, and flag only
breakage the fixes themselves introduced. **The C2 cross-check is not redone** — C2 is mid-fix and gets
its own round. Plan re-read at 7,979 lines, in two passes: the changed regions by diff of the section
map, then the eight R-X interface statements against the task code that has to honour them. Where a fix
names a symbol in the existing codebase I checked it again (`profiles::load`/`save`, `Profile.id`,
`state::app_data_root`, `inference::judgments_dir`).

**Verdict: all three Critical and fourteen of sixteen Important findings are addressed; two Important
remain partly open, and one new Important defect was introduced.** Nothing that was right in round 0
was broken by the fixes.

### Critical — 3 of 3 ADDRESSED

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| C1 | `PUT /account/sources` never called | **ADDRESSED** | `put_source_at` (`plan.md:6027-6046`) and `store_source` (`plan.md:6051-6055`); `finish()` calls it the moment a link validates (`plan.md:6057-6069`); two loopback tests (`plan.md:5751-5791`); Task 18's back-fill out of `config/ingest.yaml` through a `pystr` + `serde_yaml_ng` reader (`plan.md:7495-7516`); the contract restated at `plan.md:166`. Best-effort and non-fatal in both places, which is the right shape. |
| C2 | The upgrade overlay covers today's page offline | **ADDRESSED** | `#upgrade` is a `setpanel`, the same side panel `#settings` is, with a *Not now* button (`plan.md:7666-7674`); `UPGRADE_DISMISSED` / `UPGRADE_UNREACHABLE` gate `maybeUpgrade` (`plan.md:7694-7711`); `checkAccount`'s `.catch` hides it (`plan.md:7760`); four static assertions pin dismissability, both flags and the `setpanel` class (`plan.md:7447-7459`). No boot-time probe, as the controller accepted. |
| C3 | The campus is collected two panels after the panel that needs it | **ADDRESSED** | The radios move into `wiz-lms` (`plan.md:6990`); `readSlotsPanel` loses its campus read and the change handler sets `WIZ.campus` where it is chosen (`plan.md:7037-7043`); two static assertions pin it on the calendar panel and off the slots panel (`plan.md:6887-6889`); the headless walk checks `#wiz-lms #wiz-campus` (`plan.md:477`). |

### Important — 14 of 16 ADDRESSED, 2 partly open

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| I4 | Terms/privacy links break the console window | **PARTLY ADDRESSED** | The wizard is fixed: `class="policy" data-policy=…` (`plan.md:6973`), a `preventDefault()` branch in the wizard's click listener (`plan.md:6206-6211`), `account::open_policy` (`plan.md:4761-4763`), registered in both lists (`plan.md:299`, `:305`), argued in H3 (`plan.md:318`). **The upgrade overlay is not:** its links are plain `<a href="terms.html">` with no `class="policy"` (`plan.md:7671`), and `#upgrade`'s own click listener (`plan.md:7712-7744`) has no `a.policy` branch — so in the one window that has a working console to lose, clicking the terms the user is ticking a box to accept still navigates to a missing asset. The static test greps the whole of `console.js` (`plan.md:6906`), so it passes anyway. **Fix:** add `class="policy" data-policy="terms"` / `"privacy"` to both links at `plan.md:7671` and the same four-line branch to the `#upgrade` listener; tighten the assertion to require `a.policy` inside the `#upgrade` markup. |
| I5 | `subscribed.html` / `signed-in.html` referenced, never created | **ADDRESSED** | Both created with their copy in Task 19 step 3a (`plan.md:7821-7825`), and `signed-in.html` carries the sentence the six-digit-code flow depends on. Listed in *File structure* (`plan.md:142-143`). Task 19's own **Files:** header still names only `privacy.html`, `terms.html`, `site.css`, `console.js` — see the bookkeeping cluster below. |
| I6 | R-C1-3's second half: no custom SMTP for Auth | **ADDRESSED** | `[auth.email.smtp]` with `pass = "env(SMTP_PASSWORD)"` and the reason in the comment (`plan.md:676-687`); P2 now asks for the SMTP host, port, username and password as one provider account shared with the reminder (`plan.md:51`). |
| I7 | Task-number drift in the ledger and preconditions | **PARTLY ADDRESSED** | P4 → Task 20 and P5 → Task 19 (`plan.md:53-54`) are fixed, as are the in-task references (`plan.md:3632`, `:4032`, `:4041`, `:131`). **Three ledger rows are still stale:** D4 says "13 (telemetry…)" (`plan.md:69`, is 15); D7 says "Tasks 16 and 17 add pages to `site/`" (`plan.md:72`, are 19 and 20); D11 says "Task 15 keeps the coursework-login panel … Task 16's privacy policy" (`plan.md:76`, are 17 and 19). |
| I8 | Task 17 changed the privacy-sentence pin before Task 19 wrote it | **ADDRESSED** | New Task 17 step 6a writes the sentence into both files at the point the pin starts requiring it, with the reason (`plan.md:7321-7335`); Task 19 step 1 now says the sentence is already there and not to reword it (`plan.md:7783-7787`). |
| I9 | Three tests that cannot pass against the plan's own source | **ADDRESSED** | (a) the LMS lede is now "never what you type into it" (`plan.md:6989`), so the panel carries no `password`; (b) the `session_dir` doc says "neither the app's own data root nor a profile folder" (`plan.md:5590-5593`), so the assertion at `plan.md:5804` holds; (c) the ACTIONS count splits on `= [` first, with the reason in the comment (`plan.md:6221-6230`). |
| I10 | `current_period_end` read from a field Stripe removed | **ADDRESSED** | Item-level first with a top-level fallback (`plan.md:2955-2961`), a `stripe-version` header pinned on every Stripe call (`plan.md:1897`), and the fixtures moved with a third test for the pre-`basil` shape (`plan.md:2976`). |
| I11 | `invoice.payment_failed` branch was dead | **ADDRESSED** | `accountIdFromEvent` now reads `subscription_details.metadata.account_id` between the two old sources, with the reason (`plan.md:2986-2995`), plus a test both ways. |
| I12 | No webhook idempotency or ordering guard | **ADDRESSED** | `public.webhook_events` (`plan.md:1145-1151`); `seenEvent` / `recordEvent` / `currentUpdatedAt` on `Deps`; a `duplicate` short-circuit and a `stale` guard comparing the event's own `created` against `entitlements.updated_at` (`plan.md:2997-3040`), with two tests. `updated_at` now carries the event time rather than wall clock. |
| I13 | The summer pause applied to academic-year subscribers | **ADDRESSED** | `if (s.plan !== "monthly") { …reminder only…; continue; }` with the revenue argument in the comment (`plan.md:2791-2802`); the annual reminder still runs for every plan. |
| I14 | `delete_my_data` deleted the shared backups root | **ADDRESSED** | `b.join(&profile_id)` with the reason (`plan.md:7630-7636`); the profile row is removed from `profiles.json` (`plan.md:7638-7644`) — I re-checked `profiles::load(&Path) -> Result<Vec<Profile>, String>`, `profiles::save(&Path, &[Profile])`, `Profile.id` and `state::app_data_root() -> Option<PathBuf>`, and the code type-checks against all four; the process exit is deferred on a thread so the envelope reaches the page (`plan.md:7652-7658`). A two-profile test proves the sibling survives (`plan.md:7415-7429`). |
| I15 | The telemetry watermark could skip events | **ADDRESSED** | `events_capped` / `corrections_capped`, and `high` is the **minimum** of the two tails when either stream hit the cap (`plan.md:6427-6444`), with the 600-events worked example in the comment. |
| I16 | `migrations_test.ts` broke on C2's merge | **ADDRESSED** | `MINE = /^20260910\d{6}_…/` filters `migrations()`; a separate `allMigrationNames()` asserts only the filename *shape* over the whole directory (`plan.md:965-1006`). This is R-X-8, carried exactly. |
| I17 | Stripe Checkout needs a ToS URL P2 never asked for | **ADDRESSED** | P2 now asks for it by name, says what breaks without it, and notes that it sequences Task 19 before Task 4's first live Checkout (`plan.md:51`). |
| I18 | Magic-link sign-in could not complete on the device | **ADDRESSED** | See *Decision 1* below. |

### Controller rulings — 8 of 8 carried

| Ruling | Verdict | Evidence |
|---|---|---|
| R-X-1 `sources` | Carried | The schema is quoted as the authority, "there is **no column called `url`**", and C2 is told to import C1's `importAesKey` / `decryptString` with the same `SOURCES_ENC_KEY` (`plan.md:168-179`). |
| R-X-2 `corrections` | Carried | "**`public.corrections` is created by C1**", C1's column list, `ours` / `theirs` "nullable and must stay nullable", `kind` = the note kind with **no** check constraint, and C2's two `add column if not exists` lines plus its index on `(judgment_kind, ts desc)` — "never on `corrected_at`, which does not exist here" (`plan.md:222-231`). The migration matches, with the rule restated in SQL comments (`plan.md:3664-3671`). |
| R-X-3 `request jsonb` | Carried | Defined in the migration with the Gmail exclusion in its comment (`plan.md:3672-3676`); gated on `opt_in_raw: true` in the handler and written null otherwise (`plan.md:3925`, `:3939`, `:3944`); stated in the interface (`plan.md:233-237`) and the ledger (`plan.md:70`). |
| R-X-4 the scheduler | Carried | `JudgePlan { Cloud { log_dir }, Local, Skip }` is the only rewrite (`plan.md:5163`, `:5175`, `:5195`); the Cloud argv is `judge --vault … --via local-runner --log-dir <profile judgments dir>` and the test asserts exactly that vector plus `log_dir.starts_with(&cs.data_dir)` (`plan.md:4975-4983`); entitlement outranks everything and C2 is told to expect the skip (`plan.md:195-197`); the `ingest` gate stays `has_ics_url(vault)` (`plan.md:201-202`). |
| R-X-5 `_shared/` ownership | Carried | The exception is in the binding ownership sentence: "except `functions/{judge-*,ingest-*,events,gmail-*}` **and `functions/_shared/judge_*.ts`** … every other file under `_shared/` is C1's" (`plan.md:40`). |
| R-X-6 `config.toml` / `deno.json` | Carried | Named as C1's, with C2 supplying its nine `verify_jwt = false` entries and its import-map lines as a hand-off at C2's merge, and "C1 does not guess at them" (`plan.md:239-244`). |
| R-X-7 the Deno CI job | Carried | One `cloud` job, C1's definition, amended by C2's hand-off; "neither stream adds a second `cloud` job" (`plan.md:246-249`), consistent with H7. |
| R-X-8 migration numbering | Carried | See I16. |

### The two decisions taken inside the writer's latitude

**Decision 1 — the magic link as a six-digit code.** Sound, and better than what it replaces. GoTrue's
`POST /verify` with `{type: "magiclink", email, token}` is the correct trade (`plan.md:4649-4652`); the
`{{ .Token }}` template is configured where it belongs (`plan.md:688-694`); `site/signed-in.html`
explains why a desktop app cannot follow the link (`plan.md:7825`); the panel row, the two handlers and
the "we emailed you a 6-digit code" copy are all present (`plan.md:6975`, `:7178-7198`). No crate, no
URI scheme, no capability change — the constraint I flagged is genuinely respected.

*Counts, recounted by hand from the two lists at `plan.md:299` and `plan.md:305`:* shell = 11
`onboarding` + 8 `account` + 4 `lms_link` = **23**; console = 26 `commands` + 3 `onboarding` + 12
`account` + 2 `report` = **43**; overlap = 3 + 8 = 11, so 23 + 43 − 11 = **55 distinct**. H3
(`plan.md:296`, `:308`, `:315`), H5 (`plan.md:357-372`) and H8 (`plan.md:580`) all say 23 / 43 / 55, and
H5's per-module split (26 / 11 / 12 / 4 / 2 = 55) is right. **Task 21 step 2 was not updated and still
says "21 and 41 names, 53 distinct" (`plan.md:7902`)** — see *New breakage* below.

**Decision 2 — the upgrade side panel with two page-side flags.** Correct, and it settles Critical #2
without a network probe. `#upgrade` is a `setpanel`, so the console under it stays usable; *Not now*
dismisses for the session; the three static assertions are real assertions rather than restatements
(`plan.md:7447`, `:7454-7456`, `:7459`), and they hold against the markup and the JS the same task
writes. Nothing it touches breaks anything else I reviewed: the overlay's markup carries no "folder", so
the older assertion at `plan.md:7443` still passes, and `up-error` was added to the id list
(`plan.md:7439`) so the loop no longer under-specifies the panel.

### The deferred minors — accept 3, object 0

The plan's section defers **three**, not two (`plan.md:7961-7979`); the relayed "12/14 addressed, 2
deferred" is one off against the plan's own "eleven of its fourteen". Bookkeeping only — I re-checked,
and eleven of the fourteen are fixed in place.

- **The three other §6(b) cases, declared rather than built** — **accept.** The original finding offered
  "add a line there, or add the three cases", and the line is there with a real reason (they read the
  approvals ledger, not the journal's `set` records).
- **The Rust scrubber left non-regex, with the divergence named in the comment** — **accept.** The
  original finding offered exactly this alternative, and adding a `regex` crate to preserve a bracket in
  `see(https://x/y)` is not worth a dependency when both sides redact the URL.
- **No `--frozen` / vendored Deno cache in CI** — **accept.** I raised it as a note, not a defect, and
  recording it in H7 for whoever first runs CI without network is the proportionate answer.

### New breakage introduced by the fixes

- **Important — `plan.md:7902`, Task 21 step 2 still expects "21 and 41 names, 53 distinct".** Decision 1
  moved every other statement of the count to 23 / 43 / 55 (H3, H5, H8) and left behind the one step
  whose whole job is to verify them. The step's own "do not trust this plan's arithmetic" saves it, but a
  verification step that names three wrong numbers as the expectation is the wrong way round. **Fix:**
  23 / 43 / 55 at `plan.md:7902`.
- **Minor — `plan.md:7326` and `plan.md:7332` are not byte-identical, and the step says they must be.**
  Step 6a gives the HTML with `Knowlu&rsquo;s` and the JS with `Knowlu's`, then says "make the two
  strings identical rather than loosening the test" without saying which spelling wins. The existing
  extraction takes the `<p>`'s inner text with the entity intact, so matching *it* would put a literal
  `Knowlu&rsquo;s` on the welcome panel (`EL("wiz-privacy").textContent = PRIVACY`). **Fix:** use a plain
  apostrophe in `site/privacy.html` too, and say so.
- **Minor — `plan.md:6065` returns a `note` nothing displays.** `finish()` carries the
  `PUT /account/sources` failure sentence as `note`, and the doc comment promises "a 402 becomes one
  sentence beside the link" (`plan.md:6023-6025`) — but neither the capture handler (`plan.md:7228-7236`)
  nor the paste handler (`plan.md:7266-7269`) reads it. **Fix:** append `note` to `WIZ.icsNote` in both.
- **Minor — `UPGRADE_UNREACHABLE` cannot fire for the case it was written for.** It is set from
  `.catch(upgradeUnreachable)` on the sign-in path (`plan.md:7728`), but `account::sign_in` never
  rejects: a dead network returns an `{ok: false, error: "…"}` envelope, so the `!r.ok` branch runs and
  the flag stays false. Harmless — *Not now* is what actually carries the promise, and Critical #2 is
  addressed either way — but the auto-stand-down is decorative as written. **Fix:** set the flag when
  `!r.ok` and the error is a transport failure rather than a credential one.
- **Minor (bookkeeping cluster)** — four statements the fixes and Decision 1 made stale: `plan.md:123`
  still says `account.rs` has "ten Tauri commands" (twelve); ledger D3 at `plan.md:68` still says the
  cloud arm runs "with nothing but `--vault` and `--via` on the command line", which R-X-4 changed;
  Task 17's *Consumes* list (`plan.md:6821`) omits `verify_email_code` and `open_policy`; Task 19's
  *Files* header omits `site/subscribed.html` and `site/signed-in.html`, which its own step 3a creates.

### Not re-examined

The C2 cross-check (deferred by the controller to C2's own round); every task region the fixes did not
touch; anything requiring execution — still no `cargo`, `deno`, `supabase` or `git` command was run, and
the Stripe, Supabase, GoTrue and Tauri API surfaces are again assessed from knowledge rather than from
their current documentation. In particular I did not verify that GoTrue's `/verify` accepts
`type: "magiclink"` on the pinned Supabase version — check it against today's docs before Task 10.

---

## Re-review of fix round 2 (2026-09-09)

Scope: the seven items left open after round 1, the calendar addition as a new unit, the writer's
GoTrue caveat, and breakage introduced by this round only. Plan re-read at 8,183 lines. I read the
spec's new §11a row first, then checked the addition against the engine it has to feed —
`engine/src/calfeed.rs` (the `calendars:` reader at 590-648 and its `one_feed_config` fixture at 951)
and `app/src/scheduler.rs` (`needs_first_run` at 188 and the `spawn` first-run block at 561-569).

**Verdict: all seven open items ADDRESSED. The calendar addition honours §11a, the ownership list and
the testing rules, and the shape it pins is the shape `calfeed` actually parses — with one contract gap
(the C2 interface never names the new kind) and one guard that still cannot fire.**

### The seven open items — 7 of 7 ADDRESSED

| # | Item | Verdict | Evidence |
|---|---|---|---|
| 1 | I4 — the upgrade overlay's policy links | **ADDRESSED** | Both links carry `class="policy" data-policy=…` (`plan.md:7863`) and `#upgrade`'s own listener gained the `a.policy` → `preventDefault()` → `open_policy` branch (`plan.md:7908-7911`). |
| 2 | I7 — the three stale ledger rows | **ADDRESSED** | D4 now says "and 15" (`plan.md:69`), D7 "Tasks 19 and 20" (`plan.md:72`), D11 "Task 17 … Task 19's privacy policy" (`plan.md:76`). D3 was corrected to `--vault`, `--via` and `--log-dir` at the same time (`plan.md:68`). |
| 3 | New-Important — Task 21 step 2's counts | **ADDRESSED** | Now "**23** and **43** names, **55** distinct", with the per-module split and the overlap spelled out (`plan.md:8105`). |
| 4 | New-Minor — the `&rsquo;` / `'` byte-identity coin flip | **ADDRESSED** | Both copies now use a plain apostrophe (`plan.md:7481`, `:7487`), and Task 19's quotation matches (`plan.md:7988`). |
| 5 | New-Minor — `finish()`'s `note` displayed nowhere | **ADDRESSED** | Appended to the status line in all three handlers: capture (`plan.md:7374`), school paste (`plan.md:7412`) and personal paste (`plan.md:7423`). |
| 6 | New-Minor — `UPGRADE_UNREACHABLE` could not fire | **ADDRESSED in substance** | The Rust side gained a stable first clause — `pub const UNREACHABLE` and `post_json`'s `format!("{UNREACHABLE} ({e})")` (`plan.md:4610`, `:4617`) — and the JS now tests the envelope rather than only the rejection (`plan.md:7924`). The test as written still cannot match; see *New breakage*. |
| 7 | New-Minor — the bookkeeping cluster | **ADDRESSED** | `account.rs` is "twelve Tauri commands" (`plan.md:125`); Task 17's *Consumes* lists `verify_email_code` and `open_policy` (`plan.md:6937`); Task 19's *Files* creates `subscribed.html` and `signed-in.html` (`plan.md:7982`); ledger D3 carries `--log-dir`. |

### The calendar addition, as a unit

**Against the §11a row — honoured, clause by clause.**

- *"The C1 wizard's first connection step is Connect your calendars … the calendar step precedes
  coursework logins and Gmail."* `wiz-lms` becomes `wiz-calendars` in place, so the panel set stays at
  nine and the order is welcome, account, subscribe, vault, **calendars**, logins, gmail, slots, finish
  (`plan.md:6949`, `:6958`, `:7187`). The panel is one `<h2>` with two `<h3>` halves — school above,
  personal below (`plan.md:7114-7128`). The ledger row states it as panel 5 of nine (`plan.md:82`).
- *"school (the sign-in window above)"* — unchanged, and the campus radios stay on this panel
  (`plan.md:7118`), pinned on it and off the slots panel (`plan.md:7003`, `:7015`).
- *"personal … by its secret iCal address, validated on the device"* — `#wiz-cal-ics` →
  `paste_calendar_link{kind: "calendar_ics"}` → `finish` → the same `lms_link::validate`
  (`plan.md:7421`, `:6170-6193`). The panel's copy tells the student exactly where Google keeps it and
  **to press Reset first if it has ever been shared** (`plan.md:7124`) — that sentence is not in the
  ruling and it should be; a secret iCal address is a capability URL and this is the only place a
  student would be warned.
- *"stored server-side as a `sources` row of kind `calendar_ics`"* — the check constraint is
  `kind in ('lms_ics', 'calendar_ics')` (`plan.md:1128`), `SOURCE_KINDS` exists on both sides
  (`plan.md:3597` TS, `:6168` Rust), `put_source_at` / `store_source` / `paste_calendar_link` all take
  a `kind` (`plan.md:5803`, `:6192`), and one new Deno test covers the kind end to end plus a
  `webcal://` refusal (`plan.md:3547-3579`).
- *"written to the vault's `calendars:` so today's engine counts busy time"* — `VaultPlan.personal_calendar`
  (`plan.md:5475`), `WizardPlan.personal_calendar` (`plan.md:5567`), carried through `create_vault_in`
  (`plan.md:5625`) and emitted by `ingest_yaml` (`plan.md:5535-5541`).
- *"every wizard finish runs the first slot at once … now also true of the pre-C1 app,
  `scheduler::needs_first_run`"* — correctly delivered as **nothing new**. Step 8a explains that `main`
  already does it and that adding a run here would double-fire (`plan.md:7494`), with a seam test both
  ways (`plan.md:7504-7510`) and a matching note in Task 18 step 6 (`plan.md:7971`). I checked `main`:
  `needs_first_run` is `!vault.join("today.md").exists()` (`app/src/scheduler.rs:188`) and the `spawn`
  block fires one slot behind `mode == App && device_ok && needs_first_run`, on its own thread, with
  `run_slot` serialising it (`app/src/scheduler.rs:561-569`). The plan's description is accurate.
- *"C2 adds Google sign-in for the same panel"* — `#wiz-google` is a `disabled` button with a "next
  release" label (`plan.md:7127`), pinned present-and-inert, with a second assertion that `console.js`
  contains neither `"connect_google"` nor `gmail.readonly` (`plan.md:7011-7013`). Honest, and the right
  shape for a placeholder.

**Does `calfeed` really parse the pinned shape? Yes.** The scaffold test pins
`calendars:\n  - name: personal\n    ics_url: 'https://…'\n` (`plan.md:5382`). `load_calendar_events`
takes `config.get("calendars")` as a `Value::Sequence` (`engine/src/calfeed.rs:591-594`), then per feed
reads `mapping.get("name")` and `mapping.get("ics_url")` as strings, `pystr::strip`ping the URL
(`engine/src/calfeed.rs:637-648`) — so the single quotes `yaml_scalar` adds are consumed by
`serde_yaml_ng` and never reach the fetcher. The engine's own fixture is byte-for-byte the same shape
modulo the quotes (`engine/src/calfeed.rs:951-956`), and `name: personal` is what makes the snapshot
line read `- personal · …` (`engine/src/calfeed.rs:977`). The empty case still emits `calendars: []`,
which `calfeed` treats as silence (`plan.md:5376`; `engine/src/calfeed.rs:592`). **The addition is
correct against the engine it feeds.**

**Ownership — clean.** Everything the addition touches is C1's: `20260910000100_accounts.sql`,
`functions/account/handler.ts`, `app/src/{lms_link,scaffold,onboarding}.rs`, `app/static/**`,
`app/tests/**`. `scheduler.rs` is only *referenced*; the new seam test lives in `app/tests/scheduler.rs`.
`scripts/wizard-check.py` moves through H6, which was already a hand-off and whose payload was updated
with the panel (`plan.md:478-491`, `:558`). No new hand-off is needed and none is claimed.

**Testing rules — held.** The new Deno test is pure over injected `deps` with no `fetch`
(`plan.md:3547-3566`); the two Rust loopback tests bind `127.0.0.1:0`, are joined before the test
returns, and now assert the `kind` on the wire (`plan.md:5856-5901`); the page still carries no
`http(s)://` literal (the Google-Calendar instructions are prose, not a link); the Deno count chain
stays consistent (7 → 12 → 21 → 25 → 37 → 47 → 52 → **61** → 69 → 75, Task 7 adding four crypto plus
five route tests).

**The one contract gap** is under *New breakage* below: *Interfaces with C2* was not updated for the
new kind.

### The GoTrue `type: "magiclink"` caveat — accept, with one placement objection

**Accept the resolution.** The note is right on the substance: `"magiclink"` is the type for a
link-issued OTP and `"email"` is the same call on newer GoTrue, one `curl` against staging settles it,
and the fix is one string (`plan.md:4694-4697`). That is exactly the proportionate answer to a fact
neither of us verified from documentation, and it names the failure signature (a 400 saying the type is
unsupported) so the implementer recognises it.

**Object to where it lives.** It is a doc comment *inside the code block the implementer copies*, not a
step. Task 10's steps are checkboxes and a subagent works them in order; nothing in step 3 or step 5
says "run this curl first". A caveat that must be acted on before the code is written should be the
step before it. **Fix:** promote it to a `Step 3a` in Task 10 — `curl.exe -i -X POST
"https://<staging ref>.supabase.co/auth/v1/verify" -H "apikey: <anon>" -H "content-type: application/json"
-d "{\"type\":\"magiclink\",\"email\":\"…\",\"token\":\"000000\"}"` → a 400 about the *token*, not the
*type*, means `magiclink` is right; a 400 about the type means use `"email"` — and record which you saw.

### New breakage from this round

- **Important — `plan.md:7924`, the unreachable guard still cannot fire.** The JS is
  `if (!r.ok && String(r.error || "").indexOf("could not be reached") === 0)`. The Rust error is
  `format!("{UNREACHABLE} ({e})")` where `UNREACHABLE` is *"the account service could not be reached"*
  (`plan.md:4610`, `:4617`) — so the searched substring begins at index 20, not 0, and the condition is
  always false. The round-1 fix moved the problem rather than closing it: the Rust half is now right and
  the JS half tests the wrong thing. **Fix:** match the whole clause —
  `.indexOf("the account service could not be reached") === 0` — or `.indexOf("could not be reached") >= 0`.
  Add it to the static assertion at `plan.md:7636` so the two halves are pinned to one string, the way
  `PRIVACY` is.
- **Important — *Interfaces with C2* item 3 was not updated for the second kind.** It still says "The
  **LMS** calendar URL is stored twice" and quotes the `sources` schema without its `kind` vocabulary
  (`plan.md:166-181`), while the table now carries `check (kind in ('lms_ics', 'calendar_ics'))`
  (`plan.md:1128`). Three consequences the section is the only place to settle: (a) C2's `/ingest-ics`
  now has two kinds to read and is told about one; (b) C2's forthcoming `google_calendar` kind appears
  nowhere in the plan — I grepped, there is no occurrence; (c) adding it means altering a check
  constraint in **C1's** migration and adding a line to **two C1-owned** `SOURCE_KINDS`
  (`cloud/supabase/functions/account/handler.ts:3597` and `app/src/lms_link.rs:6168`), which the binding
  ownership list forbids C2 from doing. The SQL comment anticipates the mechanism — "A third kind is a
  migration and one line in `SOURCE_KINDS`" (`plan.md:1126-1127`) — without saying whose. **Fix:** widen
  item 3 to name both kinds and their vault destinations (`ics_url:` for `lms_ics`, the `calendars:`
  list for `calendar_ics`), and state that C2 adds `google_calendar` as a controller hand-off at C2's
  merge: one `alter table … drop/add constraint` in its own `20260911…` migration plus the one line in
  each `SOURCE_KINDS`. This is the same shape as R-X-6 and needs the same ruling.
- **Minor — an empty personal calendar is refused.** `finish` runs both kinds through the same
  `validate`, which returns `CaptureError::Empty` when `summarise` counts zero `VEVENT`s
  (`plan.md:6174`), and the sentence the student sees is *"that feed is empty — check you copied the
  whole link"*. For a school feed that rule is right and deliberate ("an empty parse is a failure, never
  an empty semester"); for a personal calendar with nothing in the fetched window it is a false negative
  on a correct address. **Fix:** for `calendar_ics`, accept zero events with a different sentence
  ("connected — nothing on it yet"), and keep `Empty` fatal only for `lms_ics`.
- **Minor — the two `SOURCE_KINDS` and the SQL constraint are not pinned to each other.** The plan
  already has the pattern for this: `the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire`
  reads the TypeScript from Rust and counts it (`plan.md:6216-6231`). Nothing does the same for the
  three copies of the kind vocabulary, and a third kind has to land in all three. There is also no test
  for `finish`'s own unknown-kind guard (`plan.md:6171-6173`) — Task 14 step 5 still expects seven tests
  (`plan.md:6197`). **Fix:** one Rust test reading `handler.ts` and the migration and asserting all
  three lists agree.
- **Minor — Task 17's "Step 8a" sits between step 7 and step 8** (`plan.md:7492`, `:7494`, `:7515`).
  Step 6a is correctly placed after step 6; 8a is not. Renumber, or move it after step 8.

### Not re-examined

The C2 cross-check beyond the single `sources.kind` question the controller put in scope; every task
region this round did not touch; anything requiring execution — still no `cargo`, `deno`, `supabase` or
`git` command run. `engine/src/calfeed.rs` and `app/src/scheduler.rs` were read, not run.

---

## Re-review of fix round 3 (2026-09-09)

Scope: the two Importants and three Minors left open after round 2, the GoTrue caveat's new placement,
and breakage from this round only. Plan re-read at 8,339 lines; one pass over the changed regions, then
a second over the three files the new pin reads (`20260910000100_accounts.sql`,
`functions/account/handler.ts`, `app/src/lms_link.rs`) to check the pin against what it will actually
find.

**Verdict: all five open items ADDRESSED and the GoTrue caveat accepted — with one new Important: the
vocabulary test added this round cannot pass, because it reads the wrong check constraint.**

### The five open items — 5 of 5 ADDRESSED

**(1) The unreachable guard — ADDRESSED, and correctly.** The Rust literal is
`pub const UNREACHABLE: &str = "the account service could not be reached";` (`plan.md:4638`) and the
page literal is `var UNREACHABLE = "the account service could not be reached";` (`plan.md:7626`). I
compared them character by character: **identical**, no trailing period, no entity, no capital
difference. `post_json` still emits `format!("{UNREACHABLE} ({e})")` (`plan.md:4646`), so the guard
`String(r.error || "").indexOf(UNREACHABLE) === 0` (`plan.md:8070`) now matches at index 0 and fires —
which is exactly what round 2 said it could not do. `the_unreachable_clause_is_one_string_on_both_sides`
(`plan.md:7172-7186`) extracts the Rust literal by splitting on `pub const UNREACHABLE: &str = "` and
taking up to the next `"`, then requires `var UNREACHABLE = "<clause>";` verbatim in `console.js` —
the extraction terminates correctly and `src/account.rs` resolves from an integration test's crate-root
cwd, the same way `lms_link.rs`'s source-reading test already does. The shape assertion is separate and
explicit: `assert!(js.contains("indexOf(UNREACHABLE) === 0"), "the guard must test the shared clause,
not a fragment of it")` (`plan.md:7783`). Between them, a drift on either side is caught twice.

One clarification on the brief's "both guards": there is **one** `indexOf(UNREACHABLE)` guard
(`plan.md:8070`) plus the static shape assertion. That is right — the wizard has no overlay to stand
down and needs no guard, only a sentence.

**(2) *Interfaces with C2* item 3 — ADDRESSED, and this is now the strongest section in the plan.** The
three-row table names writer, reader and device destination for each kind (`plan.md:169-173`), and every
claim it makes is true of the code:

| Claim | Verified at |
|---|---|
| `google_calendar` is in the check constraint | `plan.md:1150` — `check (kind in ('lms_ics', 'calendar_ics', 'google_calendar'))` |
| …and in the endpoint's `SOURCE_KINDS` | `plan.md:3625` — `const SOURCE_KINDS = ["lms_ics", "calendar_ics", "google_calendar"];` |
| …and in the crate's | `plan.md:6256` — `pub const SOURCE_KINDS: [&str; 3]` |
| `DEVICE_KINDS` narrows the device to two | `plan.md:6261` — `pub const DEVICE_KINDS: [&str; 2] = ["lms_ics", "calendar_ics"];` |
| `validate_for` refuses the third | `plan.md:6270-6275`, with tests at `plan.md:5938-5939` (`google_calendar` and `nonsense` both `NotACalendarLink`) |
| C1 defines no Google code | `plan.md:186` plus the static assertion that `console.js` carries neither `"connect_google"` nor `gmail.readonly` |
| C2's H9 hook is named verbatim | `plan.md:188-190` — `#wiz-google` inside `#wiz-google-row` inside `#wiz-calendars`, with the exact markup C2 amends and the instruction that it changes no other element on the panel |

The design decision underneath is the right one and worth recording: putting all three kinds in the
shared vocabulary now, and narrowing the *device* with a second list, means **C2 never edits a C1-owned
file to add its kind** — which is what the ownership rule requires and what a check constraint cannot
safely tolerate from two streams. The round-2 gap is closed properly rather than papered over.

**(3) The three Minors — all ADDRESSED.**
- *An empty personal calendar is a connection, not a failure.* `validate_for` keeps `CaptureError::Empty`
  fatal for `lms_ics` and accepts zero events for `calendar_ics` (`plan.md:5932-5936`), and the page says
  so in its own words — *"Connected — nothing on it yet."* against *"Found N things already on your
  calendar."* (`plan.md:7557`), with the reason in the comment above it.
- *The vocabulary pinned in three places.* `the_source_kind_vocabulary_is_one_list_in_three_places`
  (`plan.md:5896-5921`) reads the migration and the handler from Rust, checks every kind against both,
  and counts the quotes both ways so a kind in one place and not the other fails. The intent is exactly
  right; the extraction is not — see *New breakage*.
- *Task 17 step 8a's ordering and Task 14's count.* Step 8a now follows step 8 (`plan.md:7634`, `:7636`)
  and ends with its own `cargo test -p knowlu --test scheduler` run (`plan.md:7656`), so the test it adds
  is still executed by a step. Task 14 step 5 expects `9 passed` and enumerates them — five pure-half,
  the vocabulary pin, the `validate_for` rules, and the two loopback (`plan.md:6308`).

**(4) Step 3a's placement — ADDRESSED, accepted.** It is now a real checkbox step in Task 10
(`plan.md:4750-4756`) with the exact `curl` against staging and **both failure signatures spelled out**:
a 400 about the *token* means `magiclink` is right because the request got as far as looking one up; a
400 about the *type* means use `"email"`. It also says to record which was seen above step 1 and to use
it in step 3's `verify_email_code_at`. That is the right shape, and it is what I asked for.

One nit, not a defect: the step's title says "**before** the code that depends on it", and it sits
*after* step 3, which writes `verify_email_code_at`. That ordering is unavoidable — the curl needs the
staging project — and the first run is step 5, so nothing breaks; the title just describes an order the
placement does not have.

### New breakage from this round

- **Important — `plan.md:5900-5905`, the vocabulary test reads the wrong constraint.** It extracts with
  `sql.split("check (kind in (").nth(1)`, which takes the **first** occurrence in
  `20260910000100_accounts.sql`. That is `consents`' — `kind text not null check (kind in ('tos',
  'privacy', 'age_18', 'auto_renew'))` at `plan.md:1130` — not `sources`' at `plan.md:1150`. The test
  then asserts `constraint.contains("lms_ics")` and fails on its first iteration with *"the check
  constraint does not allow lms_ics"*; the quote count would fail too (4 against 3). **Fix:** anchor on
  the table first —
  `sql.split("create table public.sources").nth(1).and_then(|s| s.split("check (kind in (").nth(1)).and_then(|s| s.split(')').next())`
  — and keep both count assertions. This is the fourth source-reading test in the plan to be written
  against a file it did not re-read (after the LMS panel's "password", `app_data_root`, and the ACTIONS
  `]` split); every one of them was caught by reading the target rather than the test, and the pattern is
  worth a line in the stream's report so the implementer checks each extraction against a real file
  before running it.
- **Minor — `plan.md:7449` and `plan.md:7471` are a third copy of the unreachable clause.** Both wizard
  `.catch`es still write `WIZ.error = "the account service could not be reached";` as a literal rather
  than `WIZ.error = UNREACHABLE;`. They only display it, so nothing depends on the match — but step 6a's
  own comment is *"One string, two languages"*, and the new test pins only two of the three copies.
  **Fix:** use the constant in both.
- **Minor — `PUT /account/sources` now accepts `google_calendar` from any signed-in client**
  (`plan.md:3625`, `:3651`). The device is narrowed by `DEVICE_KINDS`, but the endpoint is not: a
  patched client, or C1's own code by mistake, could write a `google_calendar` row that C2's
  `/ingest-calendar` would then try to use as an iCal URL. The table says that kind is written by C2's
  `google-callback`, server-side. **Fix:** in `putSource`, refuse `google_calendar` on the client path —
  one line beside the existing kind check — and let C2's callback write it with the service role, which
  bypasses the handler anyway.
- **Minor — Task 10 now has two steps numbered 3a** (`plan.md:750`, the magic-link mail template in Task
  0, and `plan.md:4750`, this one). They are in different tasks, so nothing collides; worth knowing only
  because a report that says "step 3a" is now ambiguous without its task number.

### Not re-examined

Everything this round did not touch; the C2 cross-check beyond the `sources.kind` question already in
scope; and anything requiring execution — no `cargo`, `deno`, `supabase` or `git` command was run in any
of the three rounds. The GoTrue `/verify` type is still unverified against the pinned version by
construction: settling it is what step 3a is for.

---

## Re-review of fix round 4 (2026-09-09)

Scope: the one Important and three Minors from round 3, the Deno running totals, and breakage from this
round only. Plan re-read at 8,382 lines. For the vocabulary test I did what the plan's own new note
asks an implementer to do — opened each file the pin slices and checked the anchor against the real
text rather than against the test.

**Verdict: all four items ADDRESSED, the running totals reconcile exactly, and I found no new breakage
from this round.**

### The four items — 4 of 4 ADDRESSED

**(1) The vocabulary test's extraction — ADDRESSED, and I verified it against the target text rather
than taking the fix on its face.**

- `create table public.sources` occurs **once** in the migration (`plan.md:1141`); the plan's other two
  occurrences are the test's own anchor string and its comment (`plan.md:5923`, `:5925`), both in a
  different file.
- The block terminates at the `);` on its own line (`plan.md:1155`), so `split("\n);").next()` stops
  before `comment on table public.sources`, whose body carries `''`-escaped quotes
  (`plan.md:1156-1159`) that would otherwise wreck the quote count. That the slice ends *before* the
  comment is what makes the second count assertion safe, and it is not accidental.
- Inside that block, `check (kind in (` appears once (`plan.md:1150`); the long comment above it talks
  about "this constraint" and "a check constraint" without ever spelling the anchor
  (`plan.md:1143-1149`). `consents`' competing constraint (`plan.md:1130`) is now outside the slice
  entirely — which is the whole of the round-3 defect, closed at the root rather than by reordering the
  file.
- `constraint` therefore resolves to `'lms_ics', 'calendar_ics', 'google_calendar'`: six single quotes,
  `/2 = 3`, matching `SOURCE_KINDS.len()`.
- The TypeScript anchor `const SOURCE_KINDS = [` (`plan.md:5937`) matches the declaration at
  `plan.md:3640` and nothing else — the only other mention is `SOURCE_KINDS.includes(kind)` at
  `plan.md:3649`, which carries no `= [`. The slice yields six double quotes, `/2 = 3`.
- The Rust copy is **imported**, not sliced (`plan.md:5917`), and the test says so in a comment
  (`plan.md:5941-5942`). That is the right instinct: a test that reads its own crate's source is
  weaker than one that links against it.

Both count assertions and the per-kind `contains` loop now pass against the files as written.

**(2) Both wizard `.catch`es use the shared constant — ADDRESSED.** `WIZ.error = UNREACHABLE;` at
`plan.md:7481` and `plan.md:7503`; no literal copy of the clause survives outside the two pinned
declarations. I checked the scope question the change raises: `var UNREACHABLE` is declared at
console.js module level in Task 17 step 6a, textually *after* the two handlers that use it, but `var`
hoists to module scope and the handlers only run on a click — long after the module has evaluated. No
temporal-dead-zone hazard, and no reordering needed.

**(3) `putSource` refuses `google_calendar` from a client — ADDRESSED.** The guard is
`if (kind === "google_calendar") throw fail(403, "that calendar is connected by signing in, not by
pasting a link")` (`plan.md:3654`), with four lines of comment saying why the endpoint needs the rule
even though `DEVICE_KINDS` already narrows C1's own device. The placement is right: after the
vocabulary check, so an unknown kind is still a 400, and before the URL checks, because this is an
authorisation rule and not a shape one. The Deno test asserts the 403 *and* that `putSource` was never
reached (`plan.md:3553-3566`). No collision with the existing unknown-kind case, which uses
`gradebook` (`plan.md:3608`).

**(4) Task 0 renumbered — ADDRESSED.** Task 0 now runs Step 1 → Step 12 with no `3a`; the magic-link
template is Step 4 (`plan.md:750`) and the commit at Step 12 stages it (`plan.md:971`). I grepped for
cross-references to Task 0's step numbers and there are none, so nothing dangles. `.gitignore`'s
explicit "`templates/` is **not** ignored" line (`plan.md:772`) survived the renumber.

**The new anchor note is present and accurate** (`plan.md:8373-8382`): it names the habit, cites all
four prior instances, gives the instruction in one sentence, and lists the four extractions that need
it — `the_source_kind_vocabulary_is_one_list_in_three_places`,
`the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire`,
`the_unreachable_clause_is_one_string_on_both_sides` and
`the_wizards_privacy_sentence_is_the_sites_privacy_sentence`. That is exactly the four; I checked for a
fifth and there is none.

### The Deno running totals — confirmed

I counted `Deno.test(` declarations in each task's range and reconciled them against every stated total:

| Task | `Deno.test(` blocks | Prose-added | Running | Plan says |
|---|---|---|---|---|
| 0 | 7 | — | 7 | `ok \| 7` (`plan.md:961`) |
| 1 | 5 | — | 12 | `ok \| 12` (`plan.md:1240`) |
| 2 | 9 | — | 21 | `ok \| 21` |
| 3 | 4 | — | 25 | `ok \| 25` |
| 4 | 12 | — | 37 | `ok \| 37` |
| 5 | 6 | **+4** | 47 | `ok \| 47` |
| 6 | 5 | — | 52 | `ok \| 52` |
| 7 | 10 | — | **62** | `ok \| 62` (`plan.md:3694`) |
| 8 | 8 | — | **70** | `ok \| 70` |
| 9 | 6 | — | **76** | `ok \| 76` |

The only task whose block count differs from its increment is Task 5, and the difference is exactly the
four tests it adds to `stripe-webhook/handler_test.ts` in prose rather than as new blocks: one in step 6
(the pre-`basil` top-level shape), one in step 6a (`invoice.payment_failed` with
`subscription_details.metadata`), and two in step 6b (`duplicate` and `stale`). Its own step 7 says so —
"the six this task added, plus the four the webhook gained in steps 6, 6a and 6b" (`plan.md:3080`).
Task 7's ten are four crypto plus six route tests, which is what its step 7 claims. **Every stated total
reconciles.**

### New breakage from this round

**None found.** I checked each of the three edits for collateral: the 403 does not change the
unknown-kind case or any `deps()` helper; the shared constant is in scope at both new call sites; and
the renumber leaves no dangling reference and does not drop the template from the commit. The
vocabulary test's rewrite is the one change with real logic in it, and I traced it end to end above.

### Out of scope — for the controller to park, not to reopen

- **`Step 3a` is still ambiguous across tasks:** Task 10 has one (`plan.md:4770`) and Task 19 has one
  (`plan.md:8202`). The collision this round fixed was Task 0's; these two predate round 3 and are in
  different tasks, so nothing is wrong — only a report that says "step 3a" without its task number.
- **The global *File structure* section omits `cloud/supabase/templates/magic_link.html`**
  (`plan.md:101-107`), though Task 0's own *Files* list has it (`plan.md:633`). Added in round 2, never
  back-filled into the index.
- **The same section still describes `20260910000100_accounts.sql` as "`accounts`, `entitlements`,
  `consents`, `sources`; RLS on all four"** (`plan.md:107`), though round 1 added `webhook_events` to
  that migration (`plan.md:1183`) — five tables, not four.

### Closing note on the four rounds

The plan arrived with three Critical and sixteen Important findings and leaves with none open. Of the
twenty-three Critical/Important items raised across four rounds, every one was fixed in place rather
than argued away, and the three deferrals were each an option the review had itself offered. The
recurring defect class was source-reading tests written against a remembered file — four instances,
each caught by opening the target — and the plan now carries the habit that prevents a fifth. Nothing in
Critical or Important remains open; the three items above are bookkeeping.

## Controller close-out (2026-09-09)

- Four rounds; the final re-review found no open item. Three out-of-scope one-liners, handled by the controller directly in the plan text (index lines, not requirements): the `20260910000100_accounts.sql` entry now says five tables and names `webhook_events`; `cloud/supabase/templates/magic_link.html` is in the global *File structure*; and the "Step 3a" labels in Tasks 10 and 19 are **parked — Ruling:** no change, the steps live in different tasks and every reference carries its task number.
- **R-X-13** (from the C2 re-review's second round): C1's *Interfaces with C2* item 8 said "its nine functions"; C2 ships eleven (`judge-rules` and `ingest-calendar` joined later). Edited to "eleven" by the controller, since C2 may not touch this file. **R-X-14:** C2's hand-off H9 part (d) — what invokes `set_google_calendar(vault, true)` when the Google consent completes — is a small `google_connected` command polled by the wizard's calendar panel, self-contained in H9, not C1's LMS-link poll; the merge-time controller applies it as written in H9.
- The plan is final and is committed with this review.
