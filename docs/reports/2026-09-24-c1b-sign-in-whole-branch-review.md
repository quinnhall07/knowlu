# C1b (sign-in) — the whole-branch review, its fix round and the re-review

The final review of branch `c1b-sign-in`, the one fix dispatch it led to, and the scoped re-review, in that order. Preserved verbatim from the worktree workspace (`.superpowers/sdd/2026-09-17-c1b-sign-in-plan/`, git-ignored) when the stream was closed on 2026-09-25, before the worktree and its branch were deleted.

---

<!-- the review: final-review.md -->

# C1b whole-branch final review — `0544d82..bce4b80` (21 files, 12 commits)

**Merge after one fix round: seven should-fix items, none of which changes a design decision — one dishonest
sentence in the browser on the refusal path, one lost defence-in-depth on the Google attestation, two published
wordings that outlived the facts they described, one config value that makes a `config push` dangerous, one
collection list that is now narrower than the page's own Google paragraph, and one staging proof that tests the
wrong half of the claim it is there for.**

Reviewed as a security review first, against the spec (482 lines, amended), the plan's Global Constraints and
Exit gate, the ledger's rulings R-C1b-2…5 and R-C1b-exec-1…5, and the seven task reviews. Closed findings are
not re-litigated. The Task 7 reviewer's blocking `site/privacy.html` fragment and its fix commit landing after
`bce4b80` are treated as known and in flight (before-merge list, item 1).

---

## Findings

**F1 — the browser is told "You are signed in" when the student pressed Cancel.** `app/src/account.rs:419`
serves `CALLBACK_PAGE` for **any** request whose target is `/callback`, *before* `code_from_request_line` at
`:420` decides whether there is a code. A consent screen the student declined redirects to
`/callback?error=access_denied&error_description=…`; the app correctly shows the refusal, and the browser tab
the student is looking at says *You are signed in to Knowlu. You can close this window.* The refusal test
(`app/tests/account.rs:1039-1041`) asserts only `HTTP/1.1 200 OK`, never the body, which is why a per-task view
passed it. **Severity: should-fix.** Fix: parse first, then serve `CALLBACK_PAGE` on `Ok` and a second constant
(*Knowlu did not get a sign-in. Go back to the Knowlu window and try again.*) on `Err`, and assert the body in
that test.

**F2 — `google_sign_in` asserts the 18+ attestation with no Rust-side gate; the email path still has one.**
`account.rs:485-488` refuses `send_magic_link` unless `age_attested` is true, and `record_consent_at`
(`:990`) hard-codes `"age_attested": true` for both callers. `google_sign_in` (`:1039`) takes no argument, so
on the Google path the *only* thing that ever consults the student's attestation is a checkbox in
`console.js:1788`. C1's static test used to say "said on the panel, and enforced again in Rust"; that second
enforcement is now gone for the primary path, and the consent log is legal evidence. Nothing is broken today —
the page gate is pinned by `static_assets.rs` and the server tooth is checkout's 403 — but the belt is one
edit from being the only strap. **Severity: should-fix.** Fix: `google_sign_in(age_attested: bool)`, the page
passing `ageAttested: EL("wiz-18").checked` / `EL("up-18").checked`, and one line in the `wizard-check.py`
fake. Spec §2 step 1's "no arguments" is about the URL, the port and the tokens, not the attestation — worth
putting to Quinn in one line if that reading is disputed.

**F3 — the privacy page's collection list is now narrower than its own Google paragraph.**
`site/privacy.html:32` (*Your account*) lists the address, the fact of Google, and the Google account id.
`:74` says Google sign-in tells us *"your email address, your name, and a link to your profile picture"* —
and GoTrue does store both in `auth.users.raw_user_meta_data`. The page opens that section with *"If a category
is not on this list, we are not collecting it"* (`:29`). The two paragraphs must agree. **Severity:
should-fix.** Fix: add the name and the profile-picture link to the *Your account* entry — the same entry the
in-flight fragment fix is already editing, so it costs nothing extra.

**F4 — `cloud/supabase/config.toml:92` still promises a six-digit code.** R-C1b-exec-5 found both projects at
`otp_length = 8` and retired the count from the page copy, the two messages and `account.rs`'s comments. The
comment above `[auth.email.template.magic_link]` — in a file this stream owns and edited in this very branch —
was missed. **Severity: should-fix.** Fix: "a one-time code as well as a link".

**F5 — `site/signed-in.html:13` says the same wrong thing, published.** *"the same email has a six-digit code
in it"*. Outside the stream's ownership list, so it is a hand-off, not a silent edit. **Severity: should-fix.**
Fix: drop the count, as R-C1b-exec-5 did everywhere else.

**F6 — `config.toml:15` declares `db.major_version = 15` while both projects run 17.** R-C1b-exec-4 recorded
this and assigned the correction to Task 7, which has not run steps 3-5. As the branch stands, the declaration
of record disagrees with both projects, and any `supabase config push` from this file carries a third,
unintended update. **Severity: should-fix.** Fix: `major_version = 17` before merge, with the ruling cited.

**F7 — the staging proof tests the wrong half of the consent invariant.** On record: both routes answer 401
*without* a bearer. The load-bearing claim is that an attacker holding only the **public anon key** — which
ships in every build (`account.rs:19`) — cannot fabricate a consent row. That rests on `authGetUser`
(`_shared/db.ts:104`) answering 401 when the anon JWT is presented *as* the bearer, which is reasoning, not
evidence, and `config.toml` sets `verify_jwt = false` on the function so the gateway checks nothing.
**Severity: should-fix.** Fix: add to the staging list — `POST /account/consent` and `POST /billing-checkout`
with `Authorization: Bearer <anon_key>` must both be 401.

**F8 — `[auth.rate_limit]` declares only `email_sent`.** `config.toml:57-58`. With the password gone the
emailed code is the whole of the credential, so `token_verifications` is now the brute-force bound on it and is
left at a platform default in a table the file otherwise presents as the declaration of record. The arithmetic
is comfortable (8 digits, one hour), so this is completeness, not a hole. **Severity: nit.**

**F9 — `ageAttested: true` is a literal, not the checkbox.** `console.js:1296` and `:1807`. The gate two lines
above makes them equal today; the value no longer flows from the control the student ticked, which is the same
root as F2. **Severity: nit.**

**F10 — the wizard's email door has no busy guard.** The overlay got `UP_BUSY` (`console.js:1246-1251`) and the
wizard's Google button gets `WIZ.busy`; `#wiz-magic` (`:1801`) and `#wiz-code-go` (`:1815`) get neither, so two
quick presses spend two of the twenty emails an hour that `config.toml:58` calls "the whole of the cap".
**Severity: nit.**

**F11 — `app/tests/account.rs:144`, `a_six_digit_code_from_the_email_becomes_a_session_on_this_machine`.**
The count R-C1b-exec-5 removed everywhere else survives in a test name. **Severity: nit.**

**F12 — the 403 string drifted from the documents that quote it.** `billing-checkout/handler.ts:100` answers
*"the 18+ attestation is missing — sign in again"*; spec §5.1 and Exit gate item 7 quote *"the 18+ attestation
is missing"*. The code's sentence is the better one (it names the move); the quotes should follow it.
**Severity: nit.**

## Cross-task assessment

Every Produces is consumed in the shape it was published. `google_sign_in` returns `ok_account`'s
`{ok, error, account_id, email}` and the page reads exactly those four on both surfaces;
`send_magic_link(email: String, age_attested: bool)` is called as `{email, ageAttested}` from the wizard and
the overlay, with the camelCase trap pinned by a test that now watches the right command; the consent body
`{tos_version, privacy_version, age_attested}` is what `recordConsent` destructures; `billing-checkout`'s 403
reaches a `textContent` write on both panels (`console.js:1319`, `:1838`), never `innerHTML`, so a
provider sentence cannot become markup. The two `generate_handler!` lists recount by script to **29 + 42, 61
distinct**, which is what H3 wrote into `CLAUDE.md`. Ordering held across the tasks that shared files: T3 added
before T4 removed, H1 landed between them, and the `WIZ.busy` migration in T6 is pinned by a whole-file count
(`static_assets.rs`: exactly one writer of `wiz-next.disabled`) rather than by two slices that missed
`credentialsStranded`. The only interface drift found across the whole branch is F12's sentence.

Hygiene is clean: 12/12 commits carry both trailers, all 21 files are `i/lf w/lf`, `git diff --name-only
main...c1b-sign-in` is exactly those 21 — nothing outside the stream's ownership but H1, H2 and H3 — no
frozen fixture and no `engine/**` file is touched, the four `#[ignore]` attributes are byte-identical, and a
scan of every added line finds no client id, secret, promotion code, real address, port or person. `CLAUDE.md`'s
two overriding rules are untouched. The branch does not carry the plan's stale trailer line, so main's
R-C1b-exec-2 fix will not be reverted by the merge.

## Security assessment

**What the public anon key buys an attacker.** `/otp` with `create_user: true`, at most 20 an hour
project-wide, each producing an `accounts` row with four null consent columns and an `entitlements` row at
`none` — and, since migration `20260917000100` reads nothing out of `raw_user_meta_data` and leaves no `raise`,
**zero** consent rows. That is the right trade: the ARL log now contains only attestations made behind a
session whose address was proved, and the cost is empty rows and a shared 20/hour budget the config comment
argues for honestly. Consent cannot be fabricated: `POST /account/consent` goes through `requireUser` →
`authGetUser`, the anon JWT carries no `sub`, and the route is in the bearer loop — modulo F7, which asks for
that to be proved rather than reasoned. A code cannot be replayed (GoTrue one-time, one hour, and the mail
says so). The consent invariant holds end to end: the route is the only writer of `tos`/`privacy`/`age_18`,
`hasConsent` filters on `kind=eq.tos` so checkout's `auto_renew` row cannot mask it, the `accounts` PATCH is
filtered `age_attested_at=is.null` so two racing sign-ins cannot restamp, the PATCH precedes the inserts so a
half-failure retries correctly, and `billing-checkout` re-reads the column server-side on every checkout — no
cache, no second device and no `open_checkout` retry can route around it.

**The loopback.** Bound `127.0.0.1:0` (pinned by a source test against the wildcard), one listener per
sign-in, taken by value and dropped when the call returns, deadline checked at the top of every iteration, a
fixed 4 KiB request-line buffer, non-`/callback` peers answered 404 without ending the wait. A local process
that only guesses the port can end a sign-in with a refusal sentence — rendered with `textContent` — and
nothing more; a forged code fails the exchange because the verifier is per-attempt and never leaves the
process. Same-user code execution is out of scope and the doc comment says so rather than overclaiming. The
verifier is 256 bits of `getrandom` via `ids::new_id`, the challenge is real S256, and nothing — verifier,
code, access or refresh token — crosses the IPC, enters a log line, or appears in an error: the three
`eprintln!`s carry a route's own sentence and no credential, and `Session`'s hand-written `Debug` redacts both
tokens.

**The upgrade overlay.** It records consent on both of its doors, and it should: the route is idempotent, a
pre-C1b account's `hasConsent` is true so nothing is restamped and the four columns are not re-PATCHed, and a
new account made from the overlay needs the row like any other. The I7 case — a Google sign-in on a
pre-C1b address that was never confirmed — creates a second `auth.users` row and a second `accounts` row under
the same address, and the app says nothing about it. In practice that population cannot have subscribed (an
unconfirmed C1 password account never held a session), so the honest remedy the plan names — one emailed-code
sign-in first, after which Google links — costs the student nothing but needs to reach them. Confirm it on
staging and put the sentence in front of Quinn before the pilot; it is exit gate item 4, still open.

## Before merge

1. The Task 7 fix commit after `bce4b80` (the `site/privacy.html` *Your account* fragment, plus the stronger
   `static_assets.rs` pin), re-read once it lands; F3 folds into the same entry.
2. Quinn's two dashboard toggles on **both** projects (R-C1b-exec-4): Confirm email off, emails sent 20/hour.
3. The staging proofs that wait on them: `/otp` sends Knowlu's template with a code (spec §5.2), the
   never-verified address leaves three nulls and zero consent rows, identity linking on a confirmed and on an
   unconfirmed pre-C1b address (exit gate 4, 8, 9) — plus F7's anon-key-as-bearer check.
4. The live proof with Quinn: Continue with Google end to end on a scratch profile, and the test-mode
   100-percent promotion code through Checkout with the card still collected (exit gate 1, 6).
5. H4 — `HANDOFF.md` §3, C1b's row to merged with the branch and the merge commit.
6. The PR, and prod parity for the migration and the two functions.

---

<!-- the fix round: final-fix-report.md -->

# C1b final review — one fix round — report

Commit: `c9bef129ad32e08b96c56362a6f7f715628bd9d5` on `c1b-sign-in`, HEAD before this round `b5ecb3e`.

## Per finding

- **F1** `app/src/account.rs::serve_one_callback`: now calls `code_from_request_line` first, then
  serves `CALLBACK_PAGE` on `Ok` and the new `CALLBACK_FAILED_PAGE` ("Knowlu did not get a sign-in.
  Go back to the Knowlu window and try again.") on `Err`. `app/tests/account.rs`'s
  `a_refusal_in_the_query_is_the_providers_sentence_and_the_browser_still_gets_a_page` now asserts
  the body is `CALLBACK_FAILED_PAGE` and never "You are signed in"; the success test already
  asserted `CALLBACK_PAGE`.
- **F2** `account::google_sign_in(age_attested: bool)`: refuses with `send_magic_link`'s exact
  sentence ("Knowlu is for people 18 or older.") as the first statement, before `api_base()`, the
  listener bind or the browser open. `console.js`'s wizard Google button now sends
  `ageAttested: EL("wiz-18").checked`; the overlay (which does carry an `up-18` checkbox — not the
  "existing account with none" case) sends `ageAttested: EL("up-18").checked`. Two new
  `app/tests/account.rs` tests: `google_sign_in_gates_the_attestation_before_any_listener_is_bound`
  (source-text order: the `!age_attested` check precedes `TcpListener::bind`) and
  `google_sign_in_refuses_with_no_attestation_and_opens_nothing` (a real call with `false` against a
  closed loopback base, returning the sentence with no network touched). The generic
  `no_multi_word_command_argument_is_sent_in_the_wrong_case` pin in `static_assets.rs` picks up the
  new argument automatically (it scans every `#[tauri::command]` signature), so no separate edit was
  needed there. `scripts/wizard-check.py` was not touched, per instruction.
- **F3** `site/privacy.html`'s "Your account" `<dd>` now lists the Google account id, the name, and
  the profile-picture link, matching the page's own Google paragraph. Extended
  `the_privacy_version_constant_is_the_published_pages_date` in `static_assets.rs` to assert the
  entry carries all three.
- **F4** `cloud/supabase/config.toml`: the comment above `[auth.email.template.magic_link]` now
  reads "a one-time code as well as a link".
- **F6** `cloud/supabase/config.toml`: `major_version = 17`, with a comment citing R-C1b-exec-4. No
  Deno test pinned the old value.
- **F9** Folded into F2: both `send_magic_link` calls (`up-magic`, `wiz-magic`) now send
  `ageAttested: EL("up-18").checked` / `EL("wiz-18").checked` instead of the literal `true`.
- **F10** `console.js`: `#wiz-magic` and `#wiz-code-go` now check `if (WIZ.busy) { return; }`, latch
  `WIZ.busy = true` before their `invoke`, and release it (`WIZ.busy = false`) on every `.then`/
  `.catch` outcome; `renderWizard` paints `EL("wiz-magic").disabled` and `EL("wiz-code-go").disabled`
  from `WIZ.busy`. New `static_assets.rs` test
  `the_wizards_email_doors_get_the_same_busy_guard_the_google_button_has` pins both the render lines
  and the guard/latch/release shape inside the click listener.
- **F11** `app/tests/account.rs`: renamed
  `a_six_digit_code_from_the_email_becomes_a_session_on_this_machine` to
  `a_code_from_the_email_becomes_a_session_on_this_machine`.
- **F12** `docs/specs/2026-09-17-c1b-sign-in-design.md` §5.1 (line 255) and
  `docs/plans/2026-09-17-c1b-sign-in-plan.md`'s Exit gate item 7 (line 1873) now quote
  `"the 18+ attestation is missing — sign in again"`, matching `billing-checkout/handler.ts:100`.
  Two other pre-existing quotes without the suffix (plan line 187, the migration SQL comment) are
  out of the two named locations and were left as the instruction scoped.
- **F5, F7** confirmed already fixed on the branch (`site/signed-in.html` says "a one-time code";
  F7 is a staging-proof item, not code) — not touched.
- **F8** left as the recorded follow-up — not touched.

## Commands and results

- `cargo test -p knowlu --test account`: **44 passed, 0 failed**.
- `cargo test -p knowlu --test static_assets`: **46 passed, 0 failed**.
- `cargo test --workspace`: every crate's suite green (engine unit/integration suites, `knowlu`
  lib+bin+tests, `knowlu_engine` bin tests) — **932 passed / 3 ignored** in the engine's largest
  suite plus every other suite passing, **0 failed** anywhere; 4 `#[ignore]` total across the
  workspace, matching CLAUDE.md. Only warning: `linker stderr: ... .rsrc merge failure: multiple
  non-default manifests` (the one accepted line), on both the `bin` and `bin test` targets — no
  other warning.
- Deno, `<W>` = `C:\Users\danie\GitHub\knowlu\.claude\worktrees\c1b-sign-in`:
  - `deno test --allow-read --allow-write=<W>/cloud/eval --allow-net=127.0.0.1
    --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS
    --config <W>/cloud/supabase/deno.json <W>/cloud/supabase/ <W>/cloud/eval/`: **412 passed, 0
    failed** — matches the stated baseline.
  - `deno lint --config <W>/cloud/supabase/deno.json <W>/cloud/supabase/ <W>/cloud/eval/`: **Checked
    116 files**, clean.
  - `deno fmt --check`: not run — no `.ts` file was touched this round.
- `scripts/wizard-check.py`: not run, per instruction (its fake needs the controller's H2b first).

## Deviations

- None from the findings' prescribed fixes. One judgment call: F2's fix text says "if the overlay
  has no attestation checkbox, pass `true`... — the overlay is an existing account"; the overlay
  (`#upgrade`) does carry an `up-18` checkbox (`the_upgrade_overlay_offers_the_same_two_doors_and_no_password`
  already required it for `up-magic`'s consent), so it is wired to that checkbox rather than a
  literal `true`.
- Added two Rust tests for F2 rather than one, since the finding's own examples ("the source-text pin
  or a call with `false`") read as alternatives — both are cheap and prove different halves of the
  claim (order in source; behavior with no network touched), so both were kept.

## Concerns

- None found during this round. The gate is green end to end (Rust workspace, Deno tests, Deno
  lint), the tree is clean after the commit, and every touched file is LF with no BOM (verified on
  the staged git blobs, not the CRLF-checkout working tree — `core.autocrlf=true` locally, but
  `.gitattributes`' `eol=lf` normalizes on `git add`).
- The before-merge list (final-review.md, items 2-6) is unchanged by this round and still needs the
  controller/Quinn: dashboard toggles, staging proofs (including F7's anon-key-as-bearer check, which
  this round did not touch since F7 was marked already done), the live Google/Checkout proof, H4's
  HANDOFF row, and the PR.

---

<!-- the re-review: final-rereview.md -->

# C1b fix round 2 — re-review of `c9bef12` + `350e8fb`

**Closed — merge.**

Checked each finding against `final-review.md`'s text and ruling R-C1b-exec-6, against the diff
`review-b5ecb3e..350e8fb.diff`, and against the current worktree source (not just the diff).

- **F1 resolved.** `app/src/account.rs:426-433` (`serve_one_callback`): `code_from_request_line`
  runs before either page is served; `Ok` serves `CALLBACK_PAGE` (:429), `Err` serves
  `CALLBACK_FAILED_PAGE` (:432). `app/tests/account.rs:1021` (success test) asserts
  `page.contains(CALLBACK_PAGE)`; `:1044-1046` (refusal test) asserts
  `page.contains(CALLBACK_FAILED_PAGE)` and `!page.contains("You are signed in to Knowlu.")`.
- **F2 resolved.** `account.rs:1058-1065`: `if !age_attested { return … }` (:1059) precedes
  `TcpListener::bind` (:1065), same sentence `send_magic_link` uses. `console.js:1801` sends
  `ageAttested: EL("wiz-18").checked`; `:1281` (overlay `#up-google`) sends
  `EL("up-18").checked` — the report's documented judgment call, since `#upgrade` carries its own
  `up-18` box rather than being the "no checkbox" case. `static_assets.rs`'s generic
  `no_multi_word_command_argument_is_sent_in_the_wrong_case` (:986-1029) scans every
  `#[tauri::command]` arg in `src/*.rs`, so `age_attested` on the new signature is watched with no
  dedicated edit. `scripts/wizard-check.py:568-571`, inside check 2 ("Panel 2 is the account",
  header at :113), asserts `ga.get("ageAttested") is True`.
- **F3 resolved.** `site/privacy.html:32`, "Your account" now reads (in full): "The email address
  you sign up with; and, if you choose Continue with Google, the fact that this account signs in
  with Google, the Google account id that identifies it, your name, and a link to your profile
  picture — Knowlu never sees your Google password and never asks for one. There is no password on
  a Knowlu account at all: you sign in with Google, or with a code we email you. We also record
  that you attested you are 18 or older, as a yes and a timestamp, and which version of these two
  policies you accepted, and when. Without it there is nobody for a subscription or a judgment to
  belong to." Whole sentences, agrees with the Google paragraph; pinned by the extended
  `the_privacy_version_constant_is_the_published_pages_date` test.
- **F4 resolved.** `cloud/supabase/config.toml:94`: "a one-time code as well as a link."
- **F6 resolved.** `config.toml:17`: `major_version = 17`, comment above cites `(R-C1b-exec-4)`.
- **F9 resolved (folded into F2).** `console.js:1298`, `:1817`: both `send_magic_link` calls send
  `EL("up-18").checked` / `EL("wiz-18").checked`, not a literal `true`.
- **F10 resolved.** `console.js` `#wiz-magic` and `#wiz-code-go` handlers both guard
  `if (WIZ.busy) { return; }`, latch `WIZ.busy = true` before their `invoke`, and release it on
  every `.then`/`.catch`; `renderWizard` paints `EL("wiz-magic").disabled` /
  `EL("wiz-code-go").disabled` from `WIZ.busy`. Pinned by
  `the_wizards_email_doors_get_the_same_busy_guard_the_google_button_has` in `static_assets.rs`.
- **F11 resolved.** `app/tests/account.rs:266`: renamed to
  `a_code_from_the_email_becomes_a_session_on_this_machine`.
- **F12 resolved.** `docs/specs/2026-09-17-c1b-sign-in-design.md:255` and
  `docs/plans/2026-09-17-c1b-sign-in-plan.md:1873` (Exit gate item 7) now quote `"the 18+
  attestation is missing — sign in again"`, matching `billing-checkout/handler.ts:100`. Diff shows
  exactly one line changed per file — no drift beyond the quoted string. Two other pre-existing
  un-suffixed quotes (plan ~line 187's reproduced SQL comment, the migration SQL itself) sit outside
  the instruction's two named locations and were correctly left alone.

**Nothing newly broken.** Exactly the nine files the diff header names
(`app/src/account.rs`, `app/static/console.js`, `app/tests/account.rs`,
`app/tests/static_assets.rs`, `cloud/supabase/config.toml`,
`docs/plans/2026-09-17-c1b-sign-in-plan.md`, `docs/specs/2026-09-17-c1b-sign-in-design.md`,
`scripts/wizard-check.py`, `site/privacy.html`) across the two commits; both `c9bef12` and
`350e8fb` carry `Co-Authored-By` and `Claude-Session` trailers; `git ls-files --eol` reports
`i/lf w/lf` for all nine; a scan of the diff finds no real address, port, client id or person —
only the pre-existing `hello@knowlu.com` sender in unchanged context. F5, F7, F8 untouched this
round, as scoped.

No new findings.
