# Gmail connect in the app (MVP): design

**Status: Draft — for Quinn's signature.** Written 2026-09-29 on branch `mvp-specs` at main `97dc27b`;
revised the same day after the spec review (D4's gate wording, D7's test ledger, D13 → Q8, D14 and Q7–Q9
added; the one finding not taken as stated is at the end of §11), and again after the re-check (Q9's
`judgments` sub-question and the corrected Disconnect copy, D4's real offline cost and Q1 (a′), and the
purge migration in production parity's step (1)).
**Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (§5.3 D12, §9, §11a) and its *Amendment
2026-09-29* (ruling 3: who started a change decides how it lands; ruling 10: the stages, which put
"Gmail connect in the app" in the MVP). Where §5.3's body and ruling 3 disagree, ruling 3 wins (D7).
**Starts from:** W1's research, `.superpowers/sdd/2026-09-29-ultracode/w1/research-gmail.md`. Every
file:line claim below was re-read on `97dc27b`; §0.2 lists where the research's numbers had drifted and
the two places where its facts were wrong.

## 0. Where this comes from

C2 built Gmail server-side and proved it live on staging on 2026-09-17 (P3: consent, 55 emails judged,
disconnect; HANDOFF §1). The device only has a Google button for the **calendar**. The wizard's Gmail
panel is text that tells the student to skip it, and the console has no Google row at all. HANDOFF §4's
C4 row listed "the Gmail step's incremental-consent button and copy; the settings panel's
Connect/Disconnect Google row". Ruling 10 moved that row into the MVP.

### 0.1 What the code does today (verified at `97dc27b`)

- **App commands.** `account::google_connect_url(scope)` (`app/src/account.rs:1330`) and
  `account::google_connected()` (`:1357`) read the session from `PENDING_TARGET`
  (`knowlu/pending/session`, `:32`), which exists only between the wizard's sign-in and Finish.
  `open_external` (`:1445`) opens `https://accounts.google.com/` and nothing else (`:1431`). The GET
  helper `get_json` (`:1404`) maps a failed status through `google_error_for_status` (`:1393`: 401,
  503, else generic). Nothing in the app sends `DELETE /google-connect`.
- **Registration.** All three are in the **wizard** window's `generate_handler!` list
  (`app/src/main.rs:107`). The **console** list (`:177`, 47 commands) has none of them. The console
  pattern for an account call is `open_portal` (`account.rs:1196`): `cloud_config(&cs.vault)`, then the
  vault's own `session_credential_target`, then `open_in_browser` from Rust.
- **The engine gate.** `enrich::run_lines_with` computes `let google = google_calendar_linked(vault)`
  (`engine/src/enrich.rs:373`). That is true only when `config/ingest.yaml` carries the calendar marker
  `ics_url: 'cloud:google'` (`:532`). The marker is written only by the wizard's calendar step
  (`app/src/scaffold.rs:481-482`). `pull_gmail` runs only `if google` (`:455-458`). `google` is also
  one of the conditions for the early return that skips the probe (`:386-393`). **A student who skipped
  the calendar step and connects Gmail later gets a live grant and no mail.**
- **The quiet answers.** `/gmail-read` answers `quiet` with a reason. `no_gmail_scope` prints nothing
  on the device (`enrich.rs:609-611`; `cloudmodel.rs:183-187`). `revoked` prints
  `gmail: skipped (gmail is not connected; re-connect from settings)` (`cloudmodel.rs:199`,
  `enrich.rs:612-614`).
- **How the service picks the reason.** `gmail-read/index.ts:98-101` selects the account's
  `google_accounts` row with `status=neq.revoked`. If a row is found, the answer is `missing: "scope"`
  (silent). If none is found, the answer is `missing: "grant"`, and `handler.ts:237-243` marks the row
  revoked and answers `revoked`. **An account that never connected Google therefore reads as revoked.**
  So does an account whose row `DELETE` removed.
- **The task tier.** `pull_gmail` writes a `tier: task` message as a note directly
  (`enrich.rs:669`, `write_gmail_note`). The three middle tiers become `kind: task` approval cards
  (`:675`, `write_gmail_card`, `:925-953`). Approving one of those cards produces the same note a task
  tier would (`an_approved_gmail_card_materialises_the_same_note_a_task_tier_would`, `:2171`).
- **The cloud endpoints.** `google-connect/handler.ts`:
  - `GET ?scope=calendar|gmail` returns a consent URL. It asks for `openid email` plus one API scope
    (`:37-47`), with `access_type=offline`, `prompt=consent` (`:107`) and
    `include_granted_scopes=true` on every ask (`:116`).
  - `GET ?status=1` answers `{connected, scopes}` (`:91-94`) from a row filtered to
    `status=neq.revoked` (`index.ts:15-20`).
  - `DELETE` revokes at Google first and deletes the row second. A revoke that fails answers 502, and
    the row stays (`:69-84`, `disconnect.ts:30-42`).
  - A revoke removes **every** scope. Google has no partial revoke (`disconnect.ts:4-7`).
  - Every call passes `requireActiveEntitlement` (`index.ts:7`), so a lapsed subscription answers
    402.
- **The table.** `google_accounts` (`migrations/20260911000200_google.sql:7-26`) already stores
  `email_hint`, `scopes` (a union across consents) and `status` (`active | revoked | quiet`), so the
  row needs no schema change. D14's one migration changes only a function.
- **After a disconnect.** The vault keeps its `cloud:google` marker. `/ingest-calendar` answers 409
  "the Google calendar is not connected" (`ingest-calendar/handler.ts:395`). `calfeed` prints
  `google: fetch failed (…); using snapshot` on every rank and keeps showing the last snapshot
  (`engine/src/calfeed.rs:1143`).
- **What a disconnect leaves on the server.** `delete_google_grant`
  (`migrations/20260911000200_google.sql:135-150`) deletes the `google_accounts` row and its Vault
  secret, and nothing else. `gmail_seen` (`:31-36`) and `gmail_queue` (`:48-57`) cascade only from
  `accounts`, so a Disconnect keeps both:
  - `gmail_seen` keeps the account's message ids until the nightly 30-day prune
    (`20260911000900_final_review_fixes.sql:72`).
  - `gmail_queue` keeps every **undelivered** row, each carrying a model-written title and reason
    about one email (`:39-47`). The nightly sweep deletes only delivered rows (`20260911000900:48`),
    so an undelivered row has no end date short of account deletion.
  - The device does not receive them while disconnected. Both quiet answers carry
    `items: await undelivered()` (`gmail-read/handler.ts:233-243`), but `pull_gmail_queue` returns
    `Err(Quiet)` before it reads `items` (`cloudmodel.rs:667-675`). The first non-quiet pull after a
    later reconnect delivers them, however stale.
- **The early return that D4 touches.** The cloud arm skips the probe, and every pass after it, when
  five predicates all say "nothing to do": `pending` is empty, no event source is enabled,
  `google_calendar_linked` is false, `rule_decisions_waiting` is false and `labels_waiting` is false
  (`enrich.rs:386-393`). Three review rulings widened it (R-C2-E15, R-C2-E38, R-C2-E46; the comment
  at `:374-385`). `rule_decisions_waiting` (`:1130`, private) has no other caller. `labels_waiting`
  (`:1233`) is `pub` and `engine/tests/cloud_contract.rs` calls it.
- **The tests that pin today's Gmail behaviour.** Nine `enrich.rs` tests feed a `tier: task` item or
  call `write_gmail_note`. Six more tests (five in `enrich.rs`, one in `cloud_contract.rs`) script the
  cloud arm's request sequence or assert that a bare vault makes no request. §7 lists every one and
  says what happens to it.
- **The page.** The wizard's Gmail panel is copy only (`app/static/index.html:176-179`). The Settings
  panel is `#settings` (`:191-202`). The wizard's calendar button and its poll use a `WIZ.googleSeq`
  cancel token (`app/static/console.js:2731-2759`). `app/tests/static_assets.rs:534` asserts that
  `console.js` never contains `gmail.readonly`.
- **The privacy page** (`site/privacy.html`, version 2026-09-24):
  - `:77` says "(Gmail is not connected in this version of Knowlu; the app will say when it is.)" and
    that disconnecting Gmail in Settings "revokes our access at Google and deletes the stored
    connection".
  - `:80` says Gmail items are proposed, "where you approve or decline each one".
  - `:87` states the Testing-mode limits.
  - `:120` says a new category collected is announced and asked about.
  - The "What we collect" list (`:30-49`) has no entry for the stored Google connection itself.
  - `:56` already says an email's **sender** travels to the model for the length of one call. Nothing
    says a sender is stored.
  - `app/tests/static_assets.rs:1605` (`the_privacy_version_constant_is_the_published_pages_date`)
    already pins the page's date to `PRIVACY_VERSION`.
  - Nothing at `97dc27b` encodes privacy bump #1's version. Ruling 12's date-and-bump test is M1's to
    write and does not exist yet.

### 0.2 Corrections to the research

- **Line drift.** The research's line numbers are stale for these items:

  | Item | Research said | Actual line |
  |---|---|---|
  | The `pull_gmail` gate in `enrich.rs` | :432-441 | :455-458 |
  | `QuietReason` in `cloudmodel.rs` | :156-170 | :183-202 |
  | `pull_gmail_queue` in `cloudmodel.rs` | :529 | :642 |
  | `PANELS` in `console.js` | :1621 | :1982 |
  | The wizard's Google button in `console.js` | :2370-2396 | :2731-2759 |
  | The wizard's Gmail panel in `index.html` | :155-158 | :176-179 |
  | `#up-google` in `index.html` | :194 | :215 |

  Its `account.rs` and `main.rs` numbers hold.
- **Its option (b) would not be silent.** The research proposed pulling on entitlement alone and
  relying on `no_gmail_scope` staying quiet. But an account with no Google row answers `revoked`
  (§0.1), not `no_gmail_scope`. Without a cloud change, every student who never connected Google would
  see a "re-connect from settings" line on every slot. D4 fixes this in the cloud.
- **It did not mention the task tier.** The research did not note that the `task` tier bypasses the
  deck. Both ruling 3 and the privacy page's own sentence at `:80` say that it should not (D7).

## 1. The goal, for the student

A student opens Knowlu and asks **what's next?** Some of the answer is sitting in their inbox. A
professor moves a deadline by email, a lab partner proposes a meeting, a club posts a sign-up with a
date. VISION lists Gmail among the sources Knowlu reads. It also says that adding a source is one action
and that a source that breaks later fails visibly.

This spec gives the student that one action after onboarding. They open **Settings**, find a **Google**
row and press **Connect Gmail**. Google's consent page opens in their browser, and the row confirms when
the connection lands. From the next slot on:

- Obligations found in their mail arrive in the decisions deck as proposals, each approved or declined
  by the student.
- When the connection lapses (weekly, while Google's review is pending), the row says so and offers
  **Reconnect**. The run log names it too.
- **Disconnect Google** revokes Knowlu's access at Google in one step, and the row explains what that
  also removes.

This makes "what's next?" **more complete**: obligations that only exist in mail reach the list. It
also makes it **more honest**: a lapsed connection is named instead of silently yielding nothing. It
does not add typing, and it does not let the AI send anything (VISION commitment 3).

## 2. Stage

**MVP**, under ruling 10 of the Amendment 2026-09-29. The MVP includes "Gmail connect in the app" and is
proven on a founder-owned scratch profile and test account on **staging**, with no second person. The
stage decides three things:

- **Google stays in Testing mode throughout the MVP.** Staging's Google client already lists Quinn as
  the one test user (P3, 2026-09-17), and refresh tokens expire after 7 days. The MVP is satisfied by a
  weekly Reconnect from the row. §5 covers what this means for the Pilot and Launch.
- **Verification and CASA for `gmail.readonly` are Launch items** (ruling 10's Launch list: "`gmail.readonly`
  verification and CASA (or Quinn accepts the 100-user cap)"). This stream neither starts nor depends
  on them. It does build the in-app flow that the verification's demo video will show.
- **The Pilot owns "source went quiet" and the R-PS-4 re-consent screen** (ruling 10's Pilot list).
  This spec leaves room for both and builds neither (§10).

**The MVP's exit row for this item:** the live proof in §9 passes on a dev build pointed at staging.

## 3. Decisions

Decisions marked *(Qn)* follow the recommendation of that open question in §11. Quinn's answer there
replaces them.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| **D1** | A **Google** row in the console's Settings panel, placed after *Account*. It shows the connection's state and offers **Connect Gmail**, **Reconnect** and **Disconnect Google**. It is hidden for a vault with no account (`needs_account`). | Settings is where a student who skipped the wizard step looks. One row, because Google holds one grant. | None structural; copy only. |
| **D2** | Three new **console** commands in `account.rs`: `google_status`, `google_connect(scope)` and `google_disconnect`. Each reads `cloud_config(&cs.vault)` and the vault's `session_credential_target`, never `PENDING_TARGET`. `google_connect` fetches the consent URL, checks it with `external_url_allowed` and opens it from Rust, as `open_portal` does. | The pending session does not exist after onboarding, so the wizard's commands would answer "sign in again". Opening from Rust keeps the URL out of the webview, so `open_external` stays out of the console list. | A command registered in the wrong list fails at run time (no `ConsoleState`). A test pins the lists (§8.1). |
| **D3** | The wizard's three commands keep their names, signatures and behaviour. They are refactored onto the same pure cores (`google_status_at`, `google_connect_url_at`, `google_disconnect_at`), which take `(api_base, anon_key, token)`. | One HTTP path, tested once against a loopback server; the wizard's tests keep passing unchanged. | None; a pure refactor under existing tests. |
| **D4** *(Q1)* | **The engine pulls Gmail on every slot where the cloud judge passes its probe**, whether or not the calendar marker exists. The service decides: an account with **no** `google_accounts` row answers the silent `no_gmail_scope`, and only a row whose status is `revoked` answers `revoked`. The cloud arm's early return goes (§4.3), and with it `google_calendar_linked` and `rule_decisions_waiting`. This supersedes R-C2-E15, R-C2-E38 and R-C2-E46 for the cloud arm. Under Q1 (a′), the recommendation, a probe that fails in transport ends the arm with one named line (§4.3). | The vault should not become a second record of a cloud grant, and a grant can change without the device knowing. The cost is the probe plus one `/gmail-read` call on every cloud slot, including slots that had nothing else to do. | Without the cloud half, every student with no Google connection gets a false "re-connect" line on every slot, so the cloud change deploys first (§12). A network that black-holes rather than refuses is worse than one stall (the risk R-C2-E15 named). A transport error is not fatal (`CloudError::fatal` matches only 401/402/403, `cloudmodel.rs:235-237`), so `probe()` returns `None` (`:712-720`) and the arm goes on. A bare vault then makes three calls in a row: the probe, `/gmail-read` and `pull_rules`' `GET /judge-rules`. Each can run to `CALL_TIMEOUT` (120 s, `:71`), so a slot that cost 0 s today can cost about 360 s, bounded only by the arm's `opts.budget`. Every offline slot for every student also gains up to two lines, `gmail: skipped (no network (…))` and `rules: skipped (no network (…))`, where a bare vault printed neither. Q1 (a′) offers a transport stop that caps this at one call and one line. |
| **D5** *(Q4)* | `GET /google-connect?status=1` also returns `status` (`none`, `active`, `quiet` or `revoked`) and `email` (from `email_hint`). `connected` and `scopes` keep today's meaning, so the wizard is unaffected. | Without `status`, the row cannot tell "never connected" from "expired, reconnect". `email` tells a student with two Google accounts which one Knowlu reads. | An additive JSON change. An older app ignores both keys. |
| **D6** *(Q2)* | **Disconnect Google** revokes the whole grant, Calendar included, after a two-step confirm whose copy says so. What Knowlu already wrote stays in the vault. The `cloud:google` marker stays too, so the calendar shows its last snapshot with the named `fetch failed … using snapshot` line until those events age out. | Google has no partial revoke (`disconnect.ts:4-7`), and the endpoint already does exactly this. | A student who wanted to keep Calendar has to reconnect it. The copy tells them before they confirm. |
| **D7** *(Q5)* | **Every Gmail-derived item arrives as a proposal.** The `task` tier files the same `kind: task` approval card the middle tiers file (`write_gmail_card`). Approving it creates the same note as before. | Ruling 3: "Knowlu noticed (an email …) → it proposes and waits". The privacy page already promises this at `:80`. The 15-a-day cap then covers mail uniformly. | More cards in the deck. The cap and the first-day doubling (C1c) already bound them. |
| **D8** | **Reconnect** asks for the same scopes the connection had. `google-connect` accepts `?scope=reconnect`, which asks for every API scope on the account's row (`calendar.readonly` and/or `gmail.readonly`) in one consent. If the row has none, it answers 400. | Whether a Testing-mode token that expired after 7 days keeps its other scope under `include_granted_scopes=true` is not proven. P3's defect showed that a token covering fewer scopes than the row claims fails quietly on every slot. Asking for both in one consent avoids that case. | One more branch in `scopeFor`, with a Deno test. |
| **D9** | **No new Google scope.** The flow uses `gmail.readonly` only. It does not use `gmail.metadata` (also restricted, so no easier to verify) or `gmail.labels` (cannot read message bodies). The page never names a scope string, so `static_assets.rs:534` stays true. | Cloud design §9's table. | None. |
| **D10** *(Q3)* | **The row is visible to every signed-in student and always carries the Testing-mode sentence**, the same sentence the wizard's calendar row uses. When the poll times out, the copy names the likeliest reason: the Google account is not on the tester list. The copy changes, in a release, when verification lands. | The app has no way to know whether a Google address is on Google's tester list. A server flag would just duplicate a list that Google already holds. | A student who is not a tester reaches Google's "not verified" wall. The row has told them in advance. |
| **D11** *(Q6)* | The wizard's Gmail panel gets a **Connect Gmail** button. It uses the existing wizard commands with `scope: "gmail"`, the same poll pattern (`WIZ.gmailSeq`) and the same Testing sentence. Skip remains the default path. | Per D4, the button needs nothing but the existing commands and the page. A pilot student can then connect Gmail during onboarding. | More wizard surface for `wizard-check.py` to walk. |
| **D12** | **No vault schema, read-model or telemetry change.** No new note field, no `surface` key, and no new `uievents::ACTIONS` entry. The row reads the connection's state from the service, not from the vault. | The grant belongs to the account, not the vault. The less that changes in the vault, the less the frozen references and the surface oracle are at risk. | None. |
| **D13** *(Q8)* | **This stream does not edit `site/privacy.html` or move `PRIVACY_VERSION`.** §6 drafts the sentences here. They are folded into privacy bump #1, whose PR moves the text, the Effective date and the constant together after its lawyer read (ruling 12's pattern; ruling 10 puts bump #1 in the Pilot). **No release that carries the row or the wizard's button ships ahead of bump #1** (§6 says how this is held). | The MVP proof runs on a dev build against staging, so `:77` only becomes false in the first release that carries the row. Under ruling 10 that release is the Pilot's v0.1.1, which already carries bump #1. | A release cut from main between this merge and bump #1 would ship a page that says Gmail is not connected. §6's guard is what prevents it. |
| **D14** *(Q9)* | **Disconnect also deletes what the server holds about the student's mail.** A new migration redefines `delete_google_grant` so that, in the same transaction, it deletes the account's `gmail_queue` rows (delivered or not) and its `gmail_seen` rows. The applied migration is not edited. | §0.1: today a Disconnect keeps model-written summaries of undelivered mail with no end date, and a reconnect weeks later delivers them stale. The purge covers what exists only to serve the connection: the token, the undelivered summaries and the read ledger. It does **not** cover the `judgments` rows: each judged email leaves one, with `origin = 'gmail_api'`, its message id as `item_id` and the verdict's `fields` (`20260911000100_judgment_service.sql:67-83`; `gmail-read/handler.ts:281-296`), deleted only with the account, as `privacy.html:48` already says. Q9 (a)'s sub-question asks whether Disconnect should delete them too. | A reconnect within 30 days re-judges messages still in the read window. That spends model calls under the daily cap, but it creates no duplicates in the vault, because the device's `state/ingest-seen.md` acks a uid it already holds (`enrich.rs:640-643`). |
| **D15** *(Q7)* | **In the MVP a Gmail card shows its Gmail attribution (`created_by: gmail`) and the model's reason, and no sender.** Storing and showing the sender's display name is a Pilot item, decided in Q7, whose disclosure joins bump #1. | VISION commitment 5 says "showing who it came from". This spec does not settle whether that means the sender; Q7 puts that reading to Quinn. The MVP's only user is the founder. | If Quinn reads commitment 5 as "the sender", the MVP ships cards that fall short of it until the Pilot item lands. |

## 4. What changes, by layer

### 4.1 Cloud (`cloud/supabase/functions/`, one migration for D14)

- **`gmail-read`.** The row lookup in `index.ts:98-101` moves into a pure function in `handler.ts`,
  `lookupFromRow(row | null)`, so a Deno test can drive it directly. It selects the row without the
  status filter and decides:
  - no row: `missing: "scope"`, so the answer is `no_gmail_scope` (silent);
  - a row with `status = 'revoked'`: `missing: "grant"`, so the answer is `revoked` and `markRevoked`
    runs (idempotent);
  - any other row without the Gmail scope: `missing: "scope"`.

  The token path (a Gmail-scoped grant, then `accessTokenFromRefresh`) is unchanged. So is the 503
  for a deployment with no Google client.

  An older engine is fine: it only pulls for marker vaults, and they already saw `revoked` for a
  missing row.
- **`google-connect`.**
  - `ConnectDeps.grantedScopes` becomes `grant(accountId)`, which returns
    `{scopes, status, email} | null` from one row read without the status filter.
  - `?status=1` answers `{connected, scopes, status, email}`:
    - `status` is `"none"` when there is no row.
    - `connected` stays `scopes.length > 0 && status !== "revoked"`.
    - `scopes` is `[]` for a revoked row, which is what the wizard's `google_connected` sees today.
    - `email` is `email_hint` or `null`.
  - `scopeFor` gains `"reconnect"` (D8). It reads the row's `scopes` and asks for the identity scopes
    plus every API scope recorded there, in one consent. With no row, or an empty `scopes`, it answers
    400 `nothing to reconnect`.
  - `DELETE`'s handler is unchanged. It still revokes at Google first and calls `delete_google_grant`
    second, so a failed revoke (502) purges nothing.
- **The migration (D14).** A new, dated file under `cloud/supabase/migrations/` runs
  `create or replace function delete_google_grant(p_account uuid)` with the same signature,
  `security definer` and pinned `search_path`. It re-issues the `revoke`/`grant` of
  `20260911000300_google_privileges.sql:74-75`, because `create or replace` keeps the grants but the
  file should say so. The body deletes, for `p_account`:
  - the `gmail_queue` rows;
  - the `gmail_seen` rows;
  - the `google_accounts` row and its Vault secret, as today;
  - **only if Quinn answers Q9 (a)(i):** the account's `judgments` rows with `origin = 'gmail_api'`
    (`rule_evidence` rows that cite them go by cascade).

  `20260911000200_google.sql` itself is never edited.
- **Deploy.** The controller deploys both functions and the migration to staging from the merged
  branch (`db push --include-all`, as C3's migrations needed) before any device build with D4 runs a
  slot there. For production, the functions join production parity's function list (HANDOFF §4's
  production-parity row, step (2)), which already names C2's eleven, **and** the migration joins
  step (1)'s ordered migration list, after J's `20260922120200` and in filename order. It must reach
  production before the first release that carries the row; otherwise a production Disconnect keeps
  `gmail_queue` and `gmail_seen` rows while bump #1's page says they are deleted. T7 makes both edits.

### 4.2 App commands (`app/src/account.rs`, contract list)

Pure cores, each tested against a `127.0.0.1:0` loopback server (§8.1):

- `google_status_at(api_base, anon, token) -> Result<GoogleStatus, String>` parses the reply into
  `{state, calendar, gmail, email}`. `state` is one of `none`, `active`, `quiet`, `revoked`.
  - A reply without `status` (an older server) reads as `active` when `connected`, and `none`
    otherwise.
  - `calendar` and `gmail` test the full scope URLs, as `google_connected` does at `:1377-1378`.
- `google_connect_url_at(api_base, anon, token, scope) -> Result<String, String>`. `scope` is
  normalised to `calendar`, `gmail` or `reconnect`. The returned URL must pass `external_url_allowed`,
  or the result is `Err("the service returned a url Knowlu will not open")`.
- `google_disconnect_at(api_base, anon, token) -> Result<(), String>` sends a bearer `DELETE` through
  a new `send_json(method, url, token)`, which `get_json` becomes a thin wrapper over.
  `check_api_base` applies to both.

`google_error_for_status` gains two rows:

- **402:** "your subscription is not active, so Google cannot be connected"
- **502:** "Google could not be reached to disconnect; try again"

It keeps 401, 503 and the generic form.

Tauri commands:

| Command | Window list | Session | Returns |
|---|---|---|---|
| `google_status(cs)` | console | `cfg.session_credential_target` | `{ok, state, calendar, gmail, email, error}` |
| `google_connect(cs, scope)` | console | same | `{ok, error}`; the browser opens from Rust |
| `google_disconnect(cs)` | console | same | `{ok, error}` |
| `google_connect_url(scope)`, `google_connected()`, `open_external(url)` | wizard (unchanged) | `PENDING_TARGET` | unchanged; now call the cores |

The console list at `main.rs:177` goes from 47 to 50 commands; recount at merge. The wizard list at
`:107` stays at 29. `main.rs` is a shared file with a single owner, so the three names are a controller
hand-off (§12). No token, URL or Google email is logged. The email reaches the page because it is the
student's own address, shown back to them.

### 4.3 Engine (`engine/src/enrich.rs`, not on the contract list)

- **The gate (D4).** `run_lines_with` no longer reads the calendar marker. With a Gmail pull on every
  cloud slot, "nothing to do" can no longer be decided on the device, so **the cloud arm's early
  return at `:386-393` is removed whole**. Deleting only its `!google` conjunct would do the
  opposite: the return would fire more often, and a bare vault would never pull.
  - The probe therefore runs on every cloud slot, and `pull_gmail` runs unconditionally once past
    it (`:455-458` loses its `if`).
  - `google_calendar_linked` (`:532`) and `rule_decisions_waiting` (`:1130`) lose their only
    callers and go, since a dead private function is a warning. `labels_waiting` stays: it is `pub`
    and `cloud_contract.rs` calls it. The doc comments that name the removed predicates (`:1108`,
    `:986-987`) are corrected.
  - The comment at `:361-385` is rewritten to say the probe now runs every cloud slot, why, and
    that R-C2-E15, R-C2-E38 and R-C2-E46 are superseded for this arm.
  - The local arm (`cloud` is `None`, `:350-359`) is untouched.
  - **Under Q1 (a′) only (recommended):** a probe that failed in transport ends the cloud arm. No
    enrichment batch, events pass, Gmail pull, rule pull or label report runs, and the run prints
    exactly one line, `judge: skipped (no network (…))`, at exit 0. `probe()` (`cloudmodel.rs:712-720`,
    off the contract list) gains a way to report the transport case without changing what a fatal
    status sets on `model.fatal()`. A 5xx or 429 on the probe is not transport and keeps today's
    path. Test: §8.2 item 9.
- **The task tier (D7).** In `pull_gmail`, `"task"` routes to `write_gmail_card`, as
  `"borderline" | "event" | "opportunity"` do (`:675`), and counts in `cards`. `write_gmail_note`
  (`:871`) then has no caller outside one test and **is deleted**, not kept as `#[cfg(test)]`.
  Approving the card is the only path that makes the note. `gmail_note_text` stays, since the card's
  fenced `task` block is built from it (`:959`). The test that compared the two paths
  (`:2171`) is rewritten to compare the approved card's note with a literal (§7). A test-only writer
  would only prove agreement with a path that no longer ships.
- **The run lines.** The summary line reads `gmail: 0 task(s), N proposed, …`. The `task(s)` count
  stays in the line (it will read 0) so that the run-record text keeps its shape; see §7. The `notes`
  counter is never incremented any more, so it becomes immutable (an `unused_mut` warning otherwise).
  A lapsed grant prints `gmail: skipped (gmail is not connected; re-connect from settings)`, which the
  Settings row now makes true.
- **Unchanged:** the slot (`sync → coursework → ingest → judge → rank`), exit 0 on every failure shape,
  the actor `agent:knowlu.gmail`, the seen ledger, the ack, the budget, `rank`, `surface` and `write`.

### 4.4 The Settings row and the wizard panel (`app/static/`)

**The row** (`#set-google`, after `#set-account`) renders one of these states from `google_status`:

| State | What the row says (copy for the plan to finalise) | Buttons |
|---|---|---|
| `none` | "Gmail is not connected. Knowlu can read your inbox for things you have to do and propose each one for you to approve." | Connect Gmail |
| `active`, calendar only | "Google Calendar is connected (as {email}). Gmail is not." | Connect Gmail, Disconnect Google |
| `active`, Gmail (with or without calendar) | "Gmail is connected (as {email}), read-only. Knowlu proposes what it finds; nothing is added without you." | Disconnect Google |
| `quiet` | Same as `active`. The Pilot's "source went quiet" issue owns this state (§10). | same |
| `revoked` | "Google stopped answering for Knowlu. While Google reviews Knowlu, connections expire after seven days." | Reconnect, Disconnect Google |
| error | the command's `error` sentence | Try again |

Under every state it shows the **Testing sentence** (D10): "While Google reviews Knowlu, this works
only for invited testers, and the connection needs renewing about once a week." It also shows one line
of disclosure: "Google also tells Knowlu which Google account you connected, so this row can show it."

**The connect flow.** Connect or Reconnect calls `google_connect({scope})` and then polls
`google_status` every 3 s, up to 20 times. It stops when `gmail` is true (Connect) or `state` is
`active` (Reconnect). A `SET.googleSeq` token is bumped on Settings close and on every new click, so a
stale poll never repaints the row, as `WIZ.googleSeq` does at `console.js:2731-2759`. The row is
disabled while polling. On timeout: "Google did not finish connecting. If Google said Knowlu is not
verified, this Google account is not on the tester list yet."

**Disconnect** is two steps, like *Delete my data*. The first button reveals
"This disconnects Google Calendar too — Google keeps them as one permission. Knowlu stops reading both
and deletes the proposals from your mail it had not delivered yet. What it already added stays in your vault, and
your calendar stops updating." (D14; under Q9's option (b) the middle sentence is replaced by (b)'s
copy) and a
**Yes, disconnect** button. On `ok: false`, the row stays as it was and shows the error.

**The wizard's Gmail panel** (D11, `#wiz-gmail`):

- The text "Not yet … Skip it for now" is replaced by one sentence saying what Gmail connect does, the
  Testing sentence, and a **Connect Gmail** button (`#wiz-gmail-connect`).
- Its state lives on `WIZ` (`gmail`, `gmailNote`, `gmailPolling`, `gmailSeq`) and is painted by
  `renderWizard()`, following the A-5 rule that `static_assets.rs:539-595` pins for the calendar
  button.
- Next never waits on it. Nothing is written to the plan, because under D4 the vault needs no marker
  for Gmail.

### 4.5 Data model

- **Vault:** no change. A Gmail item is a `type: approval, kind: task` card under `approvals/`, written
  through `write` by `agent:knowlu.gmail` with `created_by: gmail`, `source_uid: gmail:<id>` and, when
  present, `judgment_id` / `judgment_kind: email`. This is today's middle-tier shape (`enrich.rs:931-953`).
  `state/ingest-seen.md` keeps its `gmail:<id>` lines. `config/ingest.yaml` is not touched.
- **Cloud:** no new table or column. `google_accounts.status` and `email_hint` already exist
  (`20260911000200_google.sql:11, 25`). The additions are the JSON keys `status` and `email` in
  `?status=1`, and D14's migration, which changes only what `delete_google_grant` deletes.
- **Credential Manager:** no new target. The console commands read the vault's own
  `session_credential_target` (normally `knowlu/<profile_id>/session`), and `wincred`'s rule stands:
  read whatever the vault names.
- **Device-side app data:** none. The row keeps no cache of the connection's state; it asks each time
  Settings opens.

## 5. Google verification and CASA: what the MVP uses before Launch

**The two scopes are different classes** (cloud design §5.3 and §9; `google-connect/handler.ts:4-13`):

- `calendar.readonly` is *sensitive*. It needs verification but no security assessment.
- `gmail.readonly` is *restricted*. It needs restricted-scope verification **and** a CASA assessment.
  No exemption applies, because Gmail data passes through our servers. CASA is renewed every 12 months
  from the Letter of Assessment.
- `openid` and `email` ride along on every consent and are non-sensitive.

Knowlu uses **two Google projects**:

- The **sign-in** project is In production. It asks for `openid email profile` only and has no cap
  (HANDOFF §4, the C1b row).
- The **Calendar/Gmail** project is in Testing on staging. The row's consent goes here. Production
  needs its own client and redirect URI for this project, which is item (3) of the production-parity
  list.

**Testing mode**, until verification lands, applies to both API scopes:

- at most 100 test users, each named by address in Google Cloud Console;
- a warning screen on every consent;
- refresh tokens that expire after 7 days.

A Google account not on the list is refused at Google's own consent page. The device sees no
connection, and the poll times out with D10's sentence.

| Stage | What Gmail runs on | Who can connect | Who owns it |
|---|---|---|---|
| **MVP** | Staging's Calendar/Gmail client, in Testing; a dev build pointed at staging (`KNOWLU_API_BASE`) | Quinn, the one test user (P3, 2026-09-17) | this stream builds the row; the weekly Reconnect is the whole of the MVP's answer to the 7-day limit |
| **Pilot** | Production's Calendar/Gmail client (production parity (3)), still in Testing | each pilot student whose Google address Quinn adds as a test user (Q3) | Quinn adds the addresses; `calendar.readonly` verification is submitted (ruling 10) |
| **Launch** | The same client after restricted-scope verification and CASA, or still in Testing if Quinn accepts the 100-user cap (ruling 10) | anyone | Quinn; the row's copy loses the Testing sentence in the release after verification |

**What the Launch submission needs.** The legal landscape note (`:795`) estimates about six weeks of
review plus CASA lab fees "in the low thousands". The submission needs:

- the verified domain `knowlu.com`;
- the homepage and the privacy policy at that domain (live since v0.1.0);
- a scope justification;
- a demo video of the consent flow and the feature.

This stream supplies the last item's subject: the Settings row, the consent and the proposals in the
deck. Google's Limited Use policy wants the feature to be prominent in the UI, and D1 plus the deck's
`created_by: gmail` attribution cover that.

CASA's Tier 2 is either an authorised self-scan (where Google assigns one) or a lab assessment. Nothing
in this spec changes the server-side attack surface that CASA assesses. It adds no endpoint and no
token path, and it adds no stored field unless Q7 is answered (b), which adds the sender's display
name to the queue payload. D14 only deletes more.

**Before Launch, nothing else is used.** There is no email forwarding inbox (cloud design §13's
fallback, not built), no `gmail.metadata` and no local reading.

## 6. Privacy impact

What the student's data does is unchanged. Gmail text is still read server-side, judged, and
discarded (§5.3). Rows are still flagged `origin = gmail_api` and excluded from the training export.
The token still never reaches the device.

**Where these sentences go (D13, Q8).** This section is the draft. None of it is written into
`site/privacy.html` by this stream. Following ruling 12's pattern, the draft joins privacy bump #1:
one PR in the Pilot moves the page's text, its Effective date and `PRIVACY_VERSION` together
(`account.rs:21-23`) after bump #1's lawyer read. T7 adds these sentences to bump #1's list in HANDOFF.

Four things change for the page and the consent log:

1. **`privacy.html:77` becomes false in the first release that carries the row.** Once the row
   ships, "(Gmail is not connected in this version of Knowlu; the app will say when it is.)" is no
   longer true, so the parenthesis goes. The same paragraph says disconnecting Gmail "revokes our
   access at Google and deletes the stored connection". Under D6 that must also say Calendar goes
   with it, and under D14 it can say what else goes. Proposed text, under Q9 (a)(ii): "Disconnecting
   (in Settings, at any time) revokes our access at Google and deletes the stored connection, our list
   of which emails we have already read, and any proposals from your mail that had not reached your
   computer yet. The record of what Knowlu decided about each email, including its message id, stays
   until you delete your account. Google holds Calendar and Gmail as one permission, so disconnecting
   one disconnects both." Under Q9 (a)(i) the second sentence goes and the first ends "…had not
   reached your computer yet, and the record of what we decided about each email", and `:48` and `:85`
   change to say that record ends at a disconnect.
2. **"What we collect" gains an entry for the connection itself.** The page says, "If a category is
   not on this list, we are not collecting it". It lists Gmail content but not the stored connection.
   Proposed entry: *Your Google connection, if you make one*, covering:
   - the permission Google gave us, stored encrypted and never sent to your computer;
   - the Google account's id and email address, so the app can show which account is connected;
   - which of Calendar and Gmail you allowed;
   - when you connected;
   - whether the connection still works.

   It is deleted when you disconnect or delete your account.
3. **`privacy.html:80` becomes true.** It says every Gmail item is proposed for approval, which the
   `task` tier breaks today. D7 fixes the code rather than the page. This part lands with this
   stream: it makes an existing sentence true and needs no page edit.
4. **"How long we keep it" gains a Gmail bullet (D14).** Proposed, under Q9 (a)(ii): "**Gmail** —
   our list of the emails we have read, for 30 days, so the same email is not read twice; a proposal
   waiting to reach your computer, until it does, and then for 7 days. Disconnecting Gmail, or
   deleting your account, deletes both at once. The record of what we decided about each email,
   including its message id, is kept until you delete your account." Under Q9 (a)(i) the last
   sentence becomes "…is kept until you disconnect Gmail or delete your account." The 30 and 7 days
   are the nightly jobs in `20260911000900_final_review_fixes.sql:48, 72`; the bump-#1 PR re-reads
   them before it quotes them.
   Under Q9's option (b) the fourth sentence instead reads: "Disconnecting stops new reading;
   proposals that had not reached your computer are kept, and delivered if you reconnect, until you
   delete your account."
5. **If Q7 is answered (b):** "What survives" (`:48`) and `:85` add "and who the message was from",
   and the connection entry gains no line (the sender belongs to the message, not the connection).

**Holding the release order (D13).** Nothing in CI knows bump #1's version (§0.1), so this stream
adds no test that ties the row to the page. The guard is one line that T7 adds to HANDOFF's Pilot
gate, beside bump #1: *no release is tagged from a main that carries the Gmail row until bump #1 has
merged*. Q8 offers the mechanical alternative and its cost.

**No re-consent screen is needed for this stream**, because it moves no version. When bump #1 moves
the version, the R-PS-4 screen, which is already a Pilot gate (ruling 10), asks for it. The earlier
draft's claim about M1's date-and-bump test is withdrawn: that test does not exist yet, and nothing
encodes bump #1's version to check the claim against. Under D13 this stream moves no constant, so
the question does not arise.

**Is the connection entry a "new category" under `:120`** ("we will email … and ask, before the new
use begins")? No production account has ever held a Google connection, because production carries no
C2 function yet (HANDOFF §1). So the entry is disclosed before the first use it describes, as long as
one ordering holds.

- **The ordering.** Bump #1's page reaches `knowlu.com` no later than production's `GOOGLE_CLIENT_ID`
  is set. Until that secret is set, `google-connect` answers 503 in production and nobody can connect.
- **Why it matters.** The production-parity list sets the secret at step (3) and publishes the page at
  step (4). The released 0.1.0 already has the wizard's calendar button, so a student on 0.1.0 could
  connect Calendar as soon as the secret is set.
- **The fix.** §12's T7 asks the docs-keeper to put bump #1's page deploy ahead of the secret in
  HANDOFF §4's checklist.

## 7. Contract-list impact

| File | On the list? | What changes | Who |
|---|---|---|---|
| `app/src/account.rs` | **yes** | three cores, three console commands, `send_json`, two error rows; `PRIVACY_VERSION` is **not** touched (D13) | contract-engineer |
| `app/tests/account.rs` | yes (its entitlement tests; the file is treated as the list's) | the loopback and session-target tests of §8.1 | contract-engineer |
| `app/src/main.rs` | no; a shared file with a single owner | three names in the console `generate_handler!` list | controller hand-off |
| `engine/src/enrich.rs` | no | the gate (D4), the `task` routing (D7), and the test changes tabled below | implementer |
| `engine/tests/cloud_contract.rs` | no | one test re-scripted for D4 (below) | implementer |
| `engine/src/{approvals,sync,write,journal}.rs` | yes | **nothing**: the card goes through the existing `write_gmail_card` → `write`, and the cap through `approvals::defer_over_budget` at `rank`, both unchanged | — |
| `cloud/supabase/functions/{gmail-read,google-connect}/`, one new file in `cloud/supabase/migrations/`, `migrations_test.ts` | no | §4.1, D14 | cloud-engineer |
| `app/static/{index.html,console.js}`, `app/tests/static_assets.rs`, `scripts/{settings,wizard}-check.py` | no | §4.4 | console-ui |
| `site/privacy.html`, `engine/tests/site.rs` | no, but tied to `PRIVACY_VERSION` by the `:21-23` rule | **nothing in this stream.** §6's draft is folded into privacy bump #1's PR (D13) | bump #1's owner, in the Pilot |
| `engine/tests/fixtures/**`, the eight frozen references, `surface-today-*.json` | yes | **nothing**. No fixture vault carries a Gmail item or a `cloud.yaml` that reaches the Gmail pull; `oracle.rs` and `surface_oracle.rs` must pass untouched | — |

**Vault contracts.** Nothing is renamed:

- `journal::VIAS` is unchanged.
- The run-record shape is unchanged. The `gmail:` summary line keeps its words; only its counts
  change.
- Note frontmatter and the card shape are unchanged.
- The actor is `agent:knowlu.gmail`, so judge-once holds.

**Fifteen existing tests change on purpose.** All are ordinary tests (none `#[ignore]`d). Each
changes because D4 or D7 changes the behaviour it pins, never to make a failure pass. Lines are at
`97dc27b`, and every one was read, not inferred. Each row is one entry in the plan's fidelity ledger:

- *Replaced*: the old test goes and a new one asserts the new rule.
- *Rewritten*: same name and purpose, new mechanism.
- *Re-pointed*: same scenario, with the assertion moved from the note to the card (or to the new
  request sequence) and nothing else loosened.
- *Re-scripted*: only the loopback's reply list gains a `/gmail-read` reply, and no assertion
  changes.

**D7: tests that feed a `tier: task` item or call `write_gmail_note`** (all in `engine/src/enrich.rs`):

| Test (line) | Change | What it asserts after |
|---|---|---|
| `an_approved_gmail_card_materialises_the_same_note_a_task_tier_would` (:2171) | **rewritten** (`…_materialises_todays_gmail_note_bytes`) | A card is approved and materialised, and the note equals, apart from its `id:` line, the literal that `an_item_without_one_writes_todays_bytes` pins at `:2366-2372`. The literal is copied from the old test verbatim, never regenerated from new output. It is the byte contract existing vaults hold. |
| `a_clear_task_email_becomes_a_note_with_created_by_gmail` (:2282) | **replaced** by `a_clear_task_email_becomes_a_proposal_card` | §8.2 item 5. The summary is `gmail: 0 task(s), 1 proposed, 0 dropped as information`, and the ack flush still names `gmail:m1`. |
| `a_pulled_task_note_carries_the_email_judgment_id` (:2316) | **re-pointed** (`a_pulled_task_card_carries_the_email_judgment_id`) | The same two keys, read from `approvals/task-ph-106-problem-set-4.md`. |
| `an_item_without_one_writes_todays_bytes` (:2347) | **re-pointed** | The task-tier card's text has no `judgment_id` or `judgment_kind` anywhere. Its byte literal moves to the `:2171` rewrite above, so it is still pinned. |
| `a_failed_round_stops_the_pull_and_says_so` (:2499) | **re-pointed** | The first round's **card** exists (not a note), and `gmail:m1` is seen-recorded. The skipped line and the two-request count are unchanged. |
| `nothing_is_acknowledged_that_was_not_written` (:2546) | **re-pointed** | Its fault (making `approvals/` a file) would now fail both items. So the first item becomes `information`, a success that writes nothing under `approvals/`, and the rest stays: `m1` is acked and seen, `m2` is named `not written` and is absent from the ack, and the summary ends `, 1 not written`. |
| `a_malformed_due_is_skipped_not_written_and_not_acknowledged` (:2607) | **re-pointed** | Its `tasks/` assertion would be vacuous after D7, so it becomes "no `approvals/task-ph-106-problem-set-4.md`". The seen, summary and request-count assertions are unchanged. |
| `a_duplicate_queue_row_in_the_same_batch_produces_only_one_note` (:2633) | **re-pointed** (`…_produces_only_one_card`) | One `approvals/task-ph-106-problem-set-4.md`, no `-2`, and `gmail: 0 task(s), 1 proposed, 0 dropped as information` (was `1 task(s), 0 proposed` at `:2652`). Two requests. |
| `a_duplicate_row_whose_first_occurrence_failed_is_never_acknowledged` (:2663) | **re-pointed** | "No note" becomes "no card", and the rest is unchanged. |

Unchanged, because they use other tiers or call `write_gmail_card` directly: `:2218` (which gains
task-tier items per §8.2 item 6), `:2382`, `:2415`, `:2455`, `:2702`, `:2725`, `:2745`, `:2766`,
and the completion and unknown-tier tests from `:3271` to `:3498`.

**D4: tests that script the cloud arm's requests or pin the early return:**

| Test (file, line) | Change | What it asserts after |
|---|---|---|
| `enrich.rs::an_empty_queue_makes_no_request_to_the_service` (:1966) | **replaced** by §8.2 item 2 | It pinned R-C2-E15, which D4 supersedes. The replacement asserts the probe, one `/gmail-read` and the rule pull, in that order, for an empty queue, and that a `no_gmail_scope` reply adds no line. |
| `enrich.rs::the_events_pass_runs_after_an_empty_enrichment_batch_and_the_verdict_reaches_the_ledger` (:2088) | **re-pointed** | Five requests: the probe, `POST /events`, `POST /judge-event`, **`POST /gmail-read`** (answered `no_gmail_scope`), then `GET /judge-rules`. Every other assertion is unchanged. |
| `enrich.rs::the_gmail_pull_runs_only_when_the_vault_has_linked_a_google_calendar` (:2787) | **replaced** by §8.2 items 1–2 | It pinned the marker gate. |
| `enrich.rs::a_capped_reply_stops_the_batch_after_one_request_and_the_log_says_capped` (:2847) | **re-scripted** | A `no_gmail_scope` reply is inserted before `pull_rules`' reply. The cap assertions are unchanged. |
| `enrich.rs::an_answered_rule_card_alone_makes_the_probe_fire_and_a_bare_vault_makes_none` (:3171) | **re-pointed** (`an_answered_rule_card_is_sent_and_a_bare_vault_still_asks_for_mail`) | The waiting half is re-scripted with a `/gmail-read` reply and its assertion is unchanged. The bare half pinned the early return D4 removes. It now asserts the probe, `/gmail-read` and the rule pull, and that the only line is `judge: nothing to enrich`. |
| `cloud_contract.rs::the_probe_fires_when_only_a_label_is_waiting` (:1198) | **re-pointed** | Four requests, with `/gmail-read` (answered `no_gmail_scope`) third, before `pull_rules` would have read, and the telemetry POST at index 3, not 2. `labels: sent 1` and the body assertions are unchanged. |

Checked and unchanged: `:1938` (the probe fails first, so nothing follows) and `:2890` (with the
budget spent, `pull_gmail` makes no request and prints nothing, per `:594-601`).

**The stop rule.** This list was built by reading. The implementer's first step, before writing any
code, is to make only the D4 and D7 source changes and run
`cargo test -p knowlu-engine --lib enrich` and `--test cloud_contract`. **Any failing test not named
in these two tables stops the task and goes back to the controller.** It is never edited to pass.
The reviewer checks every re-pointed row against this table: same scenario, no assertion dropped,
and a new count or path that follows from D4 or D7.

**The two overriding rules.** *No single-user assumptions:* the commands take the session target the
vault names, the tests use generated targets and loopback servers, and nothing names a person, an
address or a project. The Pilot's tester list is Google's configuration, not ours; Q3 names that
operator step plainly. *No frozen reference regenerated:* none is touched (table above).

## 8. Test plan (TDD: each test is written and seen failing before its code)

### 8.1 App (`app/tests/account.rs` and unit tests in `account.rs`; contract-engineer)

The loopback helper at `app/tests/account.rs:12` serves each case on `127.0.0.1:0`. The server records
the request, so each test can assert the method, the path, the bearer and the `apikey`.

1. `google_status_at` parses `{connected, scopes, status, email}` into each of the four states and
   maps both scope URLs. It also reads a reply with no `status` and no `email` (an older server) as
   `active` or `none`, with `email: None`.
2. `google_connect_url_at` asks `GET /google-connect?scope=gmail` (and `reconnect`, and falls back to
   `calendar` for anything else) with the bearer. If the service returns a URL that fails
   `external_url_allowed` (`https://evil.example/…`), the result is an error and nothing is opened.
3. `google_disconnect_at` sends `DELETE /google-connect`. The replies map as follows:
   - 200 `{disconnected:true}` → `Ok`
   - 502 → the "try again" sentence
   - 401 → "sign in again"
   - 402 → the subscription sentence
   - 503 → the not-configured sentence
4. `google_error_for_status` covers 402 and 502 (extends `google_error_for_status_tests`, `:1456`).
5. **The console commands use the vault's session target, never the pending one.** The test takes the
   file's `CREDMAN_LOCK` and uses a generated test profile id with a `Drop` guard. It builds a scratch
   vault whose `config/cloud.yaml` points `api_base` at the loopback and names that generated target.
   `KNOWLU_API_BASE` is set to the same loopback through the file's existing env guard (`:728`),
   because `cloud_config` refuses a vault `api_base` that differs from the compiled-in one (R-C1-59 I1,
   `account.rs:683-688`). The test stores a token under the generated target only. It never writes
   `PENDING_TARGET`, which is a fixed, real target that a developer's own wizard may be holding. It then
   calls the command's non-Tauri inner function (`google_status_in(&vault)`). The bearer the server
   saw must be the profile's.
6. **The handler lists.** A source-text test over `app/src/main.rs` asserts that:
   - the console list names `account::google_status`, `account::google_connect` and
     `account::google_disconnect`;
   - the wizard list names none of them;
   - the console list still does not name `account::open_external`, which keeps D2's promise that the
     URL never passes through the console's webview.
7. The existing wizard tests pass unchanged (D3). `no_test_in_this_file_can_reach_the_compiled_in_project`
   (`:963`) covers the new tests too.

### 8.2 Engine (`engine/src/enrich.rs` tests, `gmail_loopback` at `:2241`; implementer)

1. **D4, no marker:** a vault with `config/cloud.yaml`, a session and **no** `calendars:` entry.
   `run_lines_with` makes a `POST /gmail-read` request after the probe. This replaces the marker test
   (§7).
2. **D4, nothing else pending:** the same vault with no pending enrichment, no event feed, no rule
   decision and no label waiting still probes, pulls and runs the rule pull, in that order. This pins
   the removal of the early return and replaces `:1966` (§7).
3. `no_gmail_scope` still prints no line and makes no second request (the existing test at `:2702`
   stays as it is).
4. A `revoked` reply prints exactly
   `gmail: skipped (gmail is not connected; re-connect from settings)`, and the run exits 0.
5. **D7:** a `tier: task` item produces one `approvals/` card with `kind: task`, `created_by: gmail`,
   `source_uid: gmail:<id>` and the journal actor `agent:knowlu.gmail`. No `tasks/` note is created,
   and the uid is acked and seen-recorded. Approving the card materialises the note with today's
   bytes (the `:2171` rewrite, §7).
6. **D7 and the cap:** `enrich.rs::an_over_budget_gmail_batch_is_snoozed_not_dropped` (`:2218`) gains
   `task`-tier items and still snoozes past 15, never deletes. `cli.rs::gmail_and_event_digest_share_one_joint_daily_budget`
   (`:2027`) stays as it is, because `cli.rs` is a shared file with a single owner.
7. The whole workspace passes: `oracle.rs`, `surface_oracle.rs`, `dependency_boundary.rs`,
   `no_console.rs`, 0 warnings. A dead `google_calendar_linked`, `rule_decisions_waiting` or
   `write_gmail_note`, or a `mut` that is never mutated, would be a warning, which is why §4.3
   removes them.
8. Every row of §7's two tables is done as tabled, and the stop rule held.
9. **Q1 (a′) only, the transport stop:** a bare vault whose `api_base` points at a loopback listener
   that is bound and then dropped (so the connection is refused locally; no egress) prints exactly
   `judge: skipped (no network (…))`, makes no `/gmail-read` or `/judge-rules` request, and exits 0.
   A probe answered 503 still runs the arm as today.

### 8.3 Cloud (Deno; cloud-engineer)

1. `lookupFromRow`:
   - no row → `missing: "scope"`, and `markRevoked` is never called;
   - a `revoked` row → `missing: "grant"`;
   - an `active` row without the Gmail scope → `missing: "scope"`;
   - an `active` row with the Gmail scope → the token path;
   - no client configured → 503, as today.
2. `readHandler`, end to end with fakes: an account with no row answers
   `{quiet: true, reason: "no_gmail_scope"}`, and no mark is made.
3. `connectHandler` with `?status=1`:

   | Row | `connected` | `scopes` | `status` | `email` |
   |---|---|---|---|---|
   | none | `false` | `[]` | `"none"` | `null` |
   | `active` | `true` | the row's | `"active"` | `email_hint` |
   | `revoked` | `false` | `[]` | `"revoked"` | — |
   | `quiet` | `true` | the row's | `"quiet"` | — |
4. `scopeFor("reconnect")` with a row carrying both scopes asks for `openid email`, `calendar.readonly`
   and `gmail.readonly` in one URL, with `include_granted_scopes=true` and `prompt=consent`. With no
   row, the answer is 400.
5. The existing account-scoping scan (every `google_accounts` select carries `account_id=eq.`) still
   passes, as do `handler_test.ts` and `disconnect_test.ts` unchanged.
6. **D14, in `migrations_test.ts`:** the **last** migration that defines `delete_google_grant` (found
   the way `gmail_queue_tier_check`'s test finds the last constraint, `:804`) has a body that
   deletes from `gmail_queue` and from `gmail_seen`, each filtered on `account_id = p_account`, as
   well as the `google_accounts` row and the Vault secret. It keeps `security definer`, and the
   service-role-only grant is still in force. `20260911000200_google.sql` is byte-identical to `97dc27b`.
   Under Q9 (a)(ii) the body does **not** delete from `judgments`; under (a)(i) it deletes from
   `judgments` filtered on both `account_id = p_account` and `origin = 'gmail_api'`, and on nothing
   looser.
7. **D14, live on staging (§9 step 6):** after Disconnect, the account has no `gmail_queue` and no
   `gmail_seen` rows, and its `gmail_api` `judgments` count is unchanged under (a)(ii) or zero under
   (a)(i). The controller runs a count-only query through the service role, never a query
   that reads a row's contents.

### 8.4 Page (`app/tests/static_assets.rs`, `scripts/settings-check.py`, `scripts/wizard-check.py`; console-ui)

1. `#set-google` exists in `#settings`, after `#set-account`, with the ids `set-google-state`,
   `set-google-connect`, `set-google-reconnect`, `set-google-disconnect-1`, `set-google-disconnect-2`
   and `set-google-note`.
2. `console.js` invokes `"google_status"`, `"google_connect"` and `"google_disconnect"`. Outside the
   wizard's handlers it never invokes `"open_external"`, and it still never contains `gmail.readonly`
   (`:534` stays).
3. The row's poll carries a `SET.googleSeq` token that is bumped when Settings closes. A source test
   pins this the same way `the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it` does.
4. The Disconnect confirm's text names Calendar.
5. The wizard's `#wiz-gmail` has `#wiz-gmail-connect`, its state on `WIZ` painted by `renderWizard()`,
   and `wizGo` bumping `WIZ.gmailSeq` on leaving the panel.
6. `settings-check.py` walks the six row states with a stubbed `invoke`, and `wizard-check.py` walks
   the nine panels with the Gmail button present. Both end `ok`.
7. No `http://` or `https://` appears in any static file (the existing test at `:10`).

### 8.5 Privacy page (in privacy bump #1's PR, not this stream; D13)

These tests are handed to bump #1 with §6's draft. They are listed here so that the hand-off carries
them:

1. The page no longer contains "Gmail is not connected in this version".
2. The disconnect sentence names Calendar and what D14 deletes, and, under Q9 (a)(ii), says that the
   record of each decision, message id included, stays until the account is deleted. No sentence on
   the page says a disconnect deletes message ids that the answered Q9 option keeps.
3. "What we collect" has the Google-connection entry, and "How long we keep it" has the Gmail bullet.
4. The page's date equals `account::PRIVACY_VERSION`. This is already pinned by
   `app/tests/static_assets.rs:1605`, which stays green.

This stream adds one page-free check instead. `engine/tests/site.rs` keeps passing unchanged, and
`site/privacy.html` is byte-identical to `97dc27b` at merge (the reviewer confirms this from the diff).

## 9. Exit gate: the live proof (the controller, on staging)

The controller runs this with a dev build of the merged branch, on a **scratch profile**, against
staging (`KNOWLU_API_BASE`). The session comes from the OTP route, as the standing practice has it.
Quinn is at the browser for Google's consent, since only their address is a tester. Subagents never
hold the token.

1. Onboard a scratch vault with **no** calendar step and **no** Gmail step. Run a slot. The judge step
   prints no `gmail:` line (D4, cloud half).
2. Settings → Google reads *not connected*. Press Connect Gmail and consent in the browser. Within the
   poll, the row reads *Gmail is connected (as …)*.
3. Run a slot. The run shows `gmail: 0 task(s), N proposed, …`. The deck shows the cards, attributed to
   Gmail, and no `tasks/` note was created by `agent:knowlu.gmail` (D7).
4. Approve one card; the note appears.
5. Revoke Knowlu at Google's own third-party-access page (standing in for the 7-day expiry), then run a
   slot:
   - the run prints the `re-connect from settings` line;
   - the row reads *Google stopped answering* and offers Reconnect;
   - after Reconnect, the next slot pulls again, and both scopes are back if both were granted (D8).
6. Disconnect Google through the two-step confirm. The row reads *not connected*, Google's
   third-party-access page no longer lists Knowlu, and the next slot prints no `gmail:` line. Under
   D14, the controller's count-only query shows no `gmail_queue` or `gmail_seen` rows for the account,
   and the `gmail_api` `judgments` count the answered Q9 option calls for (§8.3 item 7).
7. Clean up per the standing rule: remove the scratch profile, its vault, its credentials and its
   autostart entry.

Pass means every step as written. Quinn's word that the item is done closes the MVP row.

## 10. Out of scope, named

- **"Source went quiet"** (`status = 'quiet'` after 14 days). This is the Pilot's issue surface (ruling
  10). The row treats `quiet` as connected until then.
- **The R-PS-4 re-consent screen and privacy bump #1** belong to the Pilot. §6's draft and §8.5's
  tests join bump #1 (D13).
- **Connecting or disconnecting Calendar alone from Settings** (H9 phase (b), `set_google_calendar`).
  Connecting Calendar needs a write to `config/ingest.yaml`, which the app has no journaled path for.
  The two-desktop spec's D17 settles who owns that file first. Until then, calendar connect stays in
  the wizard and the secret-address fallback covers the gap.
- **Removing the `cloud:google` marker on Disconnect**, for the same reason. D6's copy names the effect.
- **A Gmail-only disconnect** (Q2 option b).
- **User-excludable labels** (`google_accounts.excluded_labels`, which exists but has no UI).
- **The sender on a Gmail card, in the MVP** (D15). Whether VISION's "showing who it came from"
  means the sender is Q7, and the recommended answer lands it in the Pilot, not here.
- **Telemetry for connect and disconnect**: no new `uievents` action (D12).
- **Two desktops pulling one queue.** Before the two-desktop lease (Launch), two awake desktops can
  both pull. The service's `gmail:<id>` dedup and the seen ledger bound the result. This spec changes
  nothing about it; D4 only widens which vaults pull.

## 11. Open questions for Quinn

Each question changes what gets built. They are ordered by how much of the build they move. The
research's four questions are Q1–Q4 here (its Q3 merged with the Testing copy). Q5 and Q6 are new, from
the verification in §0.

**Q1. When should the engine pull Gmail?** Today it pulls only when the vault has the calendar marker,
so a student who skipped the calendar step gets no mail after connecting Gmail.

- (a) Pull on every cloud slot, and have the service answer the silent `no_gmail_scope` for an account
  with no Google connection. Costs one `/gmail-read` call per slot, plus the probe and the rule pull on
  slots that had nothing else to do, plus a small cloud change that must deploy first. **Offline it
  costs more.** A transport error is not fatal, so after a failed probe the arm still calls
  `/gmail-read` and `GET /judge-rules`. On a network that black-holes, each of the three can run to
  `CALL_TIMEOUT` (120 s): about 360 s on a slot that cost 0 s today, bounded only by the arm's budget.
  Every offline slot for every student also prints `gmail: skipped (no network (…))` and
  `rules: skipped (no network (…))`, where a bare vault printed neither.
- (a′) (a), plus a **transport stop**: when the probe fails in transport (`CloudError::Transport`),
  the cloud arm runs no further pass and prints one line, `judge: skipped (no network (…))`. This caps
  an offline slot at one `CALL_TIMEOUT` and one line. Costs: a small `enrich.rs` branch and one §8.2
  test; `probe()` must report a transport failure, which it now folds into `None` (`cloudmodel.rs:719`);
  and a vault with pending items loses its whole slot to a one-off blip on the probe, where today each
  item is tried. The plan must also find every existing test that scripts a transport-failed probe and
  table it in §7 before T3 starts; none is tabled yet.
- (b) Keep the marker and have the app write one when Gmail consent lands. The vault then records a
  cloud grant that can go stale, and the app has no journaled path for writing `config/ingest.yaml`
  (§10). A calendar marker on a Gmail-only grant would also make every `rank` fail its calendar fetch.
- (c) Have the engine ask `?status=1` first and pull only when Gmail is granted. This avoids the cloud
  change, but the call count is the same. Because `?status=1` hides revoked rows, a lapsed grant would
  go silent after the first slot, which breaks "fails visibly".

**RECOMMENDATION: (a′).** D4 as written, plus the transport stop. The stall and the two lines fall on
every student who is offline, the stop is small, and the slot lost to a probe blip comes back on the
next slot. If Quinn prefers not to widen T3, (a) alone is acceptable: the stall stays inside the arm's
budget and the scheduler's child cap, and D4 names it.

**Q2. What does Disconnect mean?** Google can revoke only the whole grant.

- (a) One **Disconnect Google** button that removes Calendar and Gmail together, with a confirm that
  says so. The calendar marker stays, and the calendar shows its last snapshot with a named line until
  those events age out.
- (b) **Disconnect Gmail** implemented as a full revoke followed by an automatic calendar-only consent.
  That is two browser trips, the student can abandon it halfway (leaving no Calendar either), and the
  code roughly doubles.
- (c) (a), plus removing the `cloud:google` marker. This needs the config-write path that §10 defers to
  the two-desktop work.

**RECOMMENDATION: (a).** It is D6 as written.

**Q3. Who sees the row before Google's review is done, and what happens at the Pilot?** In Testing
mode, only addresses on Google's tester list (100 at most) can connect. The app cannot see that list.

- (a) Every signed-in student sees the row, with the Testing sentence always shown. For the Pilot, you
  add a pilot student's Google address to the tester list when they ask. This is an operator step
  outside the product, and it ends at verification. (Rule 1 is about our code and config; this is
  Google's own configuration. It is still a per-student step, so it is named here instead of assumed
  away.)
- (b) The row appears only for accounts flagged as testers on the server. That needs a new table and
  flag, and the per-student step still exists (you set both).
- (c) Gmail stays founder-only until Launch: release builds hide the row. Pilot students get no Gmail.

**RECOMMENDATION: (a).** It is D10 as written. It keeps Gmail in the Pilot, which is where the
product's first honest test of email judgment happens.

**Q4. Should the row show which Google account is connected, and whether the connection lapsed?** Both
need `?status=1` to return the `status` and `email_hint` it already stores.

- (a) Both.
- (b) Status only. The row can still say "reconnect", but not which account.
- (c) Neither. The row cannot tell "never connected" from "expired", so a lapsed grant reads as
  "not connected" with no reason.

**RECOMMENDATION: (a).** It is D5 as written. The address is the student's own, shown back to them,
and §6 discloses that it is stored.

**Q5. Should an email that is clearly a task become a task at once, or a proposal?**

- (a) A proposal, like every other Gmail item. This follows ruling 3 ("Knowlu noticed … it proposes
  and waits") and what the privacy page already says at `:80`.
- (b) Keep today's direct note for the `task` tier. That matches quinn-ops and cloud design §5.3, but
  contradicts both ruling 3 and the page.

**RECOMMENDATION: (a).** It is D7 as written. The deck's 15-a-day cap and the snooze-not-delete rule
already carry the extra cards.

**Q6. Does the wizard's Gmail panel get the button now?**

- (a) Yes. It uses the existing wizard commands with `scope: "gmail"`, the same Testing sentence, and
  Skip as the default.
- (b) No. The panel keeps its "Skip it for now" text, and Gmail is connected only from Settings.

**RECOMMENDATION: (a).** It is D11 as written. The panel's current copy says it will connect "in one
click" once possible, and with D4 nothing else is needed for that.

Q7–Q9 come from the spec review.

**Q7. Does "showing who it came from" mean the email's sender?** VISION commitment 5 (`VISION.md:45`)
says a change Knowlu noticed "proposes and waits, showing who it came from and a short summary of the
context". For an email, the plain reading is the sender. No ruling of Quinn's reads it either way; the
earlier draft of this spec read it as the card's Gmail attribution without asking. D7 makes every Gmail
item a card, so the answer shows on every one.

- (a) Gmail attribution only (`created_by: gmail`, plus the model's `why`). Nothing new is built.
  Costs: a reading of a signed commitment that Quinn may not share, and a card that asks the student
  to judge an email without saying whose it is.
- (b) Store and show the sender's display name. Costs:
  - **Cloud:** `gmail-read` adds a `from` field (the display name only, never the address) to the
    queue payload. That is a `judge_email` output change and a Deno test.
  - **Engine:** `write_gmail_card` adds a card field such as `from:`. It must come **after**
    `source_uid:`, so that `a_pulled_card_carries_it`'s line-order assertion (`:2442-2445`) holds, and
    never inside the fenced `task` block, so that the approved note's bytes (the `:2171` literal)
    are unchanged. Existing cards are not rewritten. A new key on new cards renames nothing, so the
    frontmatter contract holds.
  - **Read model and UI:** `surface` gains a key for the deck to show. That breaks D12's "no
    `surface` key", needs a `docs/surface/anatomy.md` line, and changes no `surface-today-*.json`,
    because no fixture carries a Gmail card.
  - **Privacy:** new stored data (in `gmail_queue` until delivered plus 7 days, in the vault, and in
    the synced vault copy). §6 item 5's line joins bump #1. The page already says the sender goes to
    the model (`:56`), so no new processor or use is added.
  - **Size:** about one more S task across T1, T3 and T4.
- (c) (a) in the MVP, and (b) as a Pilot item that lands with bump #1, before any pilot student
  connects Gmail.

**RECOMMENDATION: (c).** It is D15 as written. The MVP's only user is the founder, who needs no
disclosure to themselves. The Pilot is the first time a card reaches someone else, and bump #1 is
already the PR where the disclosure has to go. If Quinn reads commitment 5 as the sender and wants it
from day one, choose (b) and add it to T1, T3 and T4 now. The bump-#1 ordering in D13 then covers
its disclosure too.

**Q8. When do §6's sentences reach the page, and does `PRIVACY_VERSION` move in this stream?**

- (a) **Draft here, fold into privacy bump #1.** The text, the date and the constant move together
  in bump #1's PR after its lawyer read (ruling 12's pattern). The row may merge to main now, because
  `:77` describes released versions and `site/` deploys only at release. The release that carries
  the row must not ship ahead of bump #1. That can be held in two ways:
  - (a1) **operationally**: a line in HANDOFF's Pilot gate, "no release from a main carrying the
    Gmail row until bump #1 merges". Costs nothing to build. Its weakness is that a hotfix release
    cut from main before the Pilot would break it, and only a person would notice.
  - (a2) **mechanically**, as ruling 12 did for grades. The page-facing half (T4's row and wizard
    button, and T6's three console registrations) stays on a proof branch that is never merged ahead
    of bump #1. A test in `static_assets.rs` fails whenever `console.js` offers Connect Gmail while
    the page still says "Gmail is not connected in this version". The MVP proof runs from that
    branch, as the grades proof does. Costs: the row lives off main until the Pilot, and
    `console.js` merge conflicts accumulate against every other MVP stream in the meantime.
- (b) **D13 as the earlier draft had it.** Edit the page in this stream, and move the constant if
  2026-09-24 is already published. Costs: a page change with no lawyer read, outside any release
  PR; possibly a version and a consent record that exist only between this merge and bump #1; and a
  departure from how ruling 10 lists bump #1 and how ruling 12 handles its sentences.

**RECOMMENDATION: (a), held by (a1).** It is D13 as written. It matches rulings 10 and 12, and it
costs no extra version. (a1) is enough because the one release ruling 10 plans before any
non-founder account, v0.1.1, already carries bump #1, and a Pilot-gate line is the kind of check
Quinn already runs at that gate. Choose (a2) if a pre-Pilot hotfix release is likely.

**Q9. What does Disconnect delete on the server?** Today: the connection row and its token, and
nothing else (§0.1).

- (a) **Also delete the account's `gmail_queue` and `gmail_seen` rows**, in a new migration that
  redefines `delete_google_grant`, with a `migrations_test.ts` check (§8.3 item 6). Costs: one small
  migration and a staging `db push`. A reconnect within 30 days re-judges any message still in the
  read window. That spends model calls under the daily cap, but it adds nothing twice to the vault,
  because the device's seen ledger catches it. **A sub-question within (a): the judgment record.** Each
  judged email also leaves a `judgments` row (`origin = 'gmail_api'`, its message id as `item_id`,
  and the verdict's `fields`), deleted today only with the account; `privacy.html:48` and `:85`
  already say it survives.
  - (i) D14 also runs `delete from judgments where account_id = p_account and origin = 'gmail_api'`.
    Costs: `rule_evidence` cascades on a judgment delete (`20260911000100_judgment_service.sql:140`),
    so the evidence behind the account's learned email rules goes with it (the rules themselves
    stay); `knownCourses` (`gmail-read/index.ts:154-171`) loses the course names it read from those
    rows; and the page's `:48` and `:85` must change in bump #1.
  - (ii) Keep the `judgments` rows. §6 items 1 and 4 say plainly that the record of what Knowlu
    decided about each email, message id included, stays until the account is deleted, consistent
    with `:48`. Costs: no code; "Disconnect" does not mean the server forgets every message id.
- (a′) Delete only the `gmail_queue` rows, the model-written summaries, and let `gmail_seen`'s
  30-day prune take the message ids. Costs: the page must say that message ids outlive a disconnect
  by up to 30 days. It saves re-judging on a quick reconnect.
- (b) **Change nothing; say it.** The page and the confirm copy say that undelivered proposals stay
  on our servers, and arrive if the student reconnects, until the account is deleted, and that
  message ids are kept for 30 days. Costs: no code, but it leaves summaries of the student's mail
  with no end date after the student asked Knowlu to stop, and a reconnect weeks later delivers
  stale proposals.

**RECOMMENDATION: (a), with (ii).** It is D14 as written. "Disconnect" should delete what exists only
to serve the connection: the token, the undelivered summaries and the read ledger. The judgment record
is the account's history. It holds no message text, feeds rule learning, is already disclosed at
`:48`, and goes when the account goes. The re-judging cost falls only on a student who disconnects and
reconnects within 30 days, and the daily cap bounds it. Choose (i) if Disconnect should leave no
message id behind; the price is the account's email-rule evidence.

### Review findings not taken as stated

- **"After Disconnect, the next slot still delivers already-judged Gmail proposals to the vault"**
  (the finding behind D14): **rejected in part.** The server does return `items` on the quiet path
  (`gmail-read/handler.ts:234, 242`), but `pull_gmail_queue` returns `Err(Quiet)` before reading them
  (`engine/src/cloudmodel.rs:667-675`), so a disconnected account's slot writes nothing. The rest of
  the finding stands: `delete_google_grant` keeps `gmail_queue` and `gmail_seen`, undelivered rows have
  no sweep, and a later reconnect delivers them stale. §0.1 records the corrected behaviour, and D14 and
  Q9 answer it.
- **The re-check's three findings** (D14's `judgments` rows, D4's offline cost, the purge missing from
  production parity's migration list): each was verified against the code and taken, none rejected.
  One note: the finding cites `HANDOFF.md:369` on main; this worktree's HANDOFF predates that list, so
  §4.1 and T7 name it by heading (§4's production-parity row, step (1)).

## 12. Task sketch

The plan (`docs/plans/`, with its fidelity ledger) turns this into tasks. The sizes and agents below
follow CLAUDE.md's roster; the files are disjoint except where named. T1–T4 can run in parallel once
T0 is done; T5, which exists only under Q8 (b), follows T2 (same file); T6 and T7 follow all of them.
Under Q8 (a2), T4's work and T6's three registrations land on the proof branch, not main.

| # | Task | Agent, and why | Files (exclusive) |
|---|---|---|---|
| **T0** | Record Quinn's answers to Q1–Q9. Under Q8 (b) only, read the date on the live `knowlu.com/privacy`. | main session; decisions and a live read are the controller's | — |
| **T1** | Cloud: `lookupFromRow`, the extended `?status=1`, `scope=reconnect`, D14's migration, and §8.3's tests 1–6. Deno check, lint and test green. | **cloud-engineer** (Opus, high): `cloud/` is theirs. A wrong lookup either spams every student or hides a revoked grant, and a wrong purge deletes another account's rows | `cloud/supabase/functions/{gmail-read,google-connect}/**`, one new `cloud/supabase/migrations/<date>_gmail_disconnect_purge.sql`, `cloud/supabase/migrations/migrations_test.ts` |
| **T2** | App: the three cores, `send_json`, the two error rows, the three console commands and their inner functions, and §8.1 tests 1–5 and 7. | **contract-engineer** (Opus, xhigh): `account.rs` is on the contract list and handles the session token | `app/src/account.rs`, `app/tests/account.rs` |
| **T3** | Engine: the D4 gate (early return removed) and the D7 routing; `google_calendar_linked`, `rule_decisions_waiting` and `write_gmail_note` removed; the rewritten comment at `enrich.rs:361-385`; §8.2's tests; and all fifteen rows of §7's two tables, run under §7's stop rule. | **implementer** (Sonnet, high): fully specified, off the contract list, checked by the tests. Every test change is enumerated, so none is a judgment call. If the stop rule fires, the task goes back to the controller and does not move up an effort level on its own | `engine/src/enrich.rs`, `engine/tests/cloud_contract.rs` (one test); under Q1 (a′), also `engine/src/cloudmodel.rs` (`probe()` only) and §8.2 item 9 |
| **T4** | Page: the Settings row, the wizard's Gmail button, their copy from §4.4, §8.4's tests and the two walk scripts. | **console-ui** (Sonnet, medium): `app/static` is theirs | `app/static/{index.html,console.js,console.css}`, `app/tests/static_assets.rs`, `scripts/{settings,wizard}-check.py` |
| **T5** | *Under Q8 (a): no task.* §6's draft and §8.5's tests go to bump #1 through T7. *Under Q8 (b) only:* §6's edits, `PRIVACY_VERSION` if the live date requires it, and §8.5's tests, all in one commit. | **contract-engineer** (Opus, xhigh), under (b) only: the constant is on the list, and the page and the constant move together | under (b): `site/privacy.html`, `engine/tests/site.rs`, the one line of `app/src/account.rs` |
| **T6** | Integration: the console `generate_handler!` hand-off (+3, recounted), §8.1 test 6, the full workspace gate (0 warnings), the staging deploy of T1 **before** the first slot of a D4 build, and §9's live proof. | **integrator** (Opus, high) for the merge; the **main session** for the deploy and the proof, since staging pushes, the OTP session and Quinn's consent click are the controller's | `app/src/main.rs` |
| **T7** | Docs: HANDOFF (the MVP row; the production-parity order fix of §6; in §4's production-parity row, T1's `<date>_gmail_disconnect_purge.sql` added to step (1)'s ordered migration list after J's `20260922120200` and in filename order, with a note that it reaches production before the first release that carries the row, and the two functions in step (2) (§4.1 Deploy); §6's draft sentences and §8.5's tests added to bump #1's list; under Q8 (a1), the release-order line in the Pilot gate; under Q7 (c), the sender as a Pilot item), `docs/reference/app.md` (three commands, the row), `docs/reference/engine-commands.md` (the `judge` step's Gmail pull on every cloud slot), and the Gmail and `delete_google_grant` lines in `cloud/supabase/README.md`. | **docs-keeper** (Sonnet, medium) | those docs |

**Review checkpoints.**

- The **reviewer** reads each of T1, T3 and T4 before it merges into the branch.
- The **contract-reviewer** (Opus, xhigh) reads T2, and T5 if Q8 (b) makes it exist.
- T3's reviewer walks §7's two tables row by row against the diff.
- The whole-branch review lands in `docs/reports/`, together with §9's proof. Quinn's word merges it.

**Size.** One stream of about M:

- T1: ~120 lines of TypeScript with tests, plus a ~30-line migration and its test.
- T2: ~200 lines of Rust with tests.
- T3: ~80 lines of source changed (mostly deletions), 15 existing tests changed per §7, and ~100
  lines of new tests. That is roughly 300 lines of test diff, the largest task in the stream.
- T4: ~150 lines of page code with tests.
- T5: none under Q8 (a); ~30 lines under (b).

Nothing here is pre-written beyond the shapes above. The plan specifies each test's assertions, and the
agents write the code.
