# Gmail connect in the app (MVP): implementation plan

**Date:** 2026-09-29. **Base:** the `gmail-connect` worktree (`.claude/worktrees/gmail-connect`), at
main `4cf3661` or later. The spec's line numbers are at `97dc27b`; every brief below names a function,
test or heading, never a line.
**Governs:** `docs/specs/2026-09-29-gmail-connect-design.md`, signed by Quinn on 2026-09-29 with every
recommendation accepted: Q1 (a′), Q2 (a), Q3 (a), Q4 (a), Q5 (a), Q6 (a), Q7 (c), Q8 (a) held by
(a1), Q9 (a) with (ii). Above it: the cloud design's Amendment 2026-09-29 (ruling 3, who started a
change decides how it lands; ruling 10, which puts "Gmail connect in the app" in the MVP).
**Stage:** MVP, HANDOFF §3's MVP lane 2. **Status:** draft, revised after the plan review (four
findings, all accepted; §9's last table). Nothing here is built yet.

## 1. Goal, and what must not move

After onboarding, a student opens Settings, finds a Google row and presses **Connect Gmail**. From the
next slot on, what their mail says they have to do arrives in the decisions deck as proposals. A lapsed
connection is named, in the row and in the run log, and offers **Reconnect**. **Disconnect Google**
revokes Knowlu's access at Google in one step and deletes what the server kept only to serve the
connection. The wizard's Gmail panel gets the same button, with Skip still the default.

VISION check: this answers "what's next?" more completely (obligations that exist only in mail reach
the list) and more honestly (a lapsed source fails visibly). Adding the source is one action. Every
Gmail item waits for the student's approval (commitment 3 and ruling 3). The AI sends nothing.

These must not move:

- **Vault contracts.** No note field, frontmatter key, `journal::VIAS` entry or run-record shape
  changes. The `gmail:` summary line keeps its words; only its counts change. The actor stays
  `agent:knowlu.gmail`, so judge-once holds.
- **Frozen references.** None of the eight Python references and none of the three
  `surface-today-*.json` references is touched. `oracle.rs` and `surface_oracle.rs` pass unchanged.
  No fixture vault reaches the Gmail pull.
- **The contract list, beyond `app/src/account.rs`.** `write`, `journal`, `approvals`, `sync` and
  every other listed engine file stay as they are.
- **`site/privacy.html` and `PRIVACY_VERSION`.** Both are byte-identical at merge (D13). The page's
  sentences go to privacy bump #1 through T14.
- **The slot and its exit codes.** `sync → coursework → ingest → judge → rank`; `judge` exits 0 on
  every failure shape.
- **The two overriding rules.** The console commands read the session target the vault names. Tests
  use generated targets and `127.0.0.1:0` loopback servers. Nothing names a person, address or
  project.

## 2. Decisions this plan makes, with the reason for each

The spec decides the product. These decide how the work is cut and fix four places where the spec,
read against the code, needs a ruling.

- **P1. The spec's T1, T2, T3 and T4 are each split into one-agent tasks.** Cloud becomes T1
  (`gmail-read`), T2 (`google-connect`) and T3 (the migration). App becomes T4a and T4b (the pure
  cores) and T5 (the console commands). Engine becomes T6 (D7), T7a and T7b (D4) and T8 (the
  transport stop). Page becomes T9a, T9b and T9c (the Settings row) and T10 (the wizard's button);
  P10 gives the reason for the a/b/c splits. Reason: each piece then fits one agent's
  context and one review, and the engine piece, which the spec calls the largest, is split along its
  two decisions.
- **P2. §7's stop rule applies per task, after the new tests are seen red.** Each engine task runs in
  four steps: (1) write the new tests and see each fail on the unchanged source, for the stated
  reason; (2) make only the source change; (3) run `cargo test -p knowlu-engine --lib enrich` and
  `--test cloud_contract`, where the new tests now pass and only the rows of the spec's §7 table for
  that decision may fail (any other failure stops the task and goes back to the controller,
  unedited); (4) re-point the tabled tests. T6 does this for D7, T7a and T7b for D4, T8 for the one
  row §3 of this plan adds. Reason: spec §8's heading and CLAUDE.md's TDD rule require every new
  test to be seen failing before its code, and the spec's stop rule, applied per task, still
  catches every unlisted failure and attributes it to one decision.
- **P3. Four lanes, each on its own branch off `gmail-connect`.** `gmail-connect-cloud` (T1–T3),
  `gmail-connect-app` (T4a–T5), `gmail-connect-engine` (T6–T8) and `gmail-connect-page` (T9a–T10). T11
  merges the four into `gmail-connect`. Reason: the lanes' files are disjoint, so four agents can work
  at once without sharing a working tree; the one shared file, `app/src/main.rs`, is T11's alone.
- **P4. `google-connect/handler_test.ts` changes its fakes, not its assertions.** The spec renames
  `ConnectDeps.grantedScopes` to `grant(accountId)` (§4.1) and also says `handler_test.ts` passes
  unchanged (§8.3 item 5). Both cannot hold: the test builds its fakes with `grantedScopes`. T2
  re-points each fake to `grant`, returning the row its old scopes imply (an `active` row with those
  scopes, or `null` for `[]`), and edits no assertion. The reviewer checks that the diff to that file
  touches fakes only. Reason: the rename is what lets one row read serve `status`, `email` and
  `reconnect`.
- **P5. The migration's stamp.** T3 names its file `20260929000200_gmail_disconnect_purge.sql`, which
  sorts after main's newest migration and after M1's `20260929000100` (on `m1-grades`). If main holds
  a stamp at or after it when T11 merges, the integrator re-stamps the file to the next free one after
  every migration on main and on `m1-grades`, and T14 quotes the final name. Reason: production
  parity's step (1) pushes in filename order.
- **P6. The handler-list test gets its own file.** §8.1 item 6's source-text test over
  `app/src/main.rs` goes in a new `app/tests/handler_lists.rs`, written by T11 before it edits
  `main.rs`. Reason: no existing test reads `main.rs`, and putting it in `app/tests/account.rs` would
  put a non-credential test under that file's contract-list treatment.
- **P7. The copy is final in §4 below.** The spec left the row's copy "for the plan to finalise".
  §4 fixes every sentence T9a–T9c and T10 put on the page, so console-ui makes no copy decision.
- **P10. No task is much over about 80 lines of edit.** The first draft's T4, T7 and T9 are each split along seams
  they already had (T4a/T4b, T7a/T7b, T9a/T9b/T9c). Each split keeps its lane and agent and runs in
  sequence. Reason: a single agent carrying the whole Settings row is the likeliest to stall or leave the
  copy half-applied, and a smaller diff is a sharper review.
- **P11. Records from real accounts are redacted.** The repository is public. T12's smoke, T13's
  proof record and T15's report give states, counts and yes/no observations only: never the Google
  address the row shows, a message id, a subject, a sender or a card's text. Reason: review check 7
  (no student content or personal identifiers in reports), and the founder's own mail is student
  data here.
- **P8. The release-order line is already in HANDOFF.** §3's Pilot section carries it ("Release
  gate: no release is cut from a main carrying the Gmail settings row until privacy bump #1 merges"),
  added in `4cf3661`. T14 checks it is there and does not add a second one.
- **P9. The account-deletion path inherits D14's purge.** `functions/account/google_delete.ts` also
  calls `delete_google_grant` when an account is deleted. After T3, that call deletes the account's
  `gmail_queue` and `gmail_seen` rows too, a moment before the account row's cascade would. Nothing
  visible changes. The T1–T3 reviewer confirms that `google_delete.ts`'s own tests still pass
  unchanged.

## 3. Open question for Quinn (gates T8 only)

**PQ1. Does the transport stop still write tier 1's answers?** Q1 (a′) says that when the probe fails
in transport, "no enrichment batch, events pass, Gmail pull, rule pull or label report runs, and the
run prints exactly one line". Read against the code, "no enrichment batch" has a cost the spec did not
name. Today, offline, `enrich_with` still writes each pending item's tier-1 answer (the rule-based
course and effort seeds, which need no network). The test
`a_service_that_cannot_be_reached_still_writes_tier_one_and_exits_zero` exists for exactly that case.
The spec's §7 lists that test as unchanged, but it was written before (a′), and the spec asked the
plan to find every test that scripts a transport-failed probe. This is the only one: a grep of
`engine/` and `app/` for a dropped listener behind a `judge` run finds no other.

- **(i) Recommended: stop the network, keep tier 1.** On a transport-failed probe the arm runs the
  enrichment with the service marked missing, the path `run_lines` already takes for a missing
  session, and prepends one `judge: skipped (no network (…))` line. It then skips the events pass,
  the Gmail pull, the rule pull and the label report. Cost: a bare vault prints two lines, the skip
  line and `judge: nothing to enrich`, so §8.2 item 9's "exactly one line" is read as "exactly one
  skipped line, and no `gmail:`, `rules:` or `labels:` line". The existing test keeps its name and
  assertions and gains one assertion (below). Because this departs from signed wording (spec §4.3's
  Q1 (a′) bullet, "the run prints exactly one line"; §8.2 item 9, "prints exactly"; and the Q1
  option text, "runs no further pass and prints one line"), choosing (i) also means a signed spec
  edit, made by T0 in the same docs commit that records the answer here, so the spec and the plan
  never disagree about which output governs.
- **(ii) Literal: run nothing.** The arm prints exactly the one line and returns. Cost: every offline
  slot stops writing tier-1 answers for new tasks until the next online slot. That existing test's
  name becomes false, and it is re-pointed to assert the skip line and that the note is unwritten.

**Answer (Quinn, 2026-09-30): (i), stop the network and keep tier 1.** The spec's three passages are
amended in the same commit, each citing this answer.

Either way the network cost is the same: one call, capped at `CALL_TIMEOUT`. T0 asks this; T8 does not
start until it is answered. T6, T7a and T7b do not depend on it.

## 4. The copy, final (P7)

Every sentence below is exact. `{email}` is the `email` the status command returns; when it is null,
the parenthesis `(as {email})` is left out, never shown empty.

**The Settings row, `#set-google`, labelled "Google":**

| State | `#set-google-state` reads | Buttons shown |
|---|---|---|
| `none` | Gmail is not connected. Knowlu can read your inbox for things you have to do and propose each one for you to approve. | Connect Gmail |
| `active` or `quiet`, calendar only | Google Calendar is connected (as {email}). Gmail is not. | Connect Gmail, Disconnect Google |
| `active` or `quiet`, Gmail granted | Gmail is connected (as {email}), read-only. Knowlu proposes what it finds; nothing is added without you. | Disconnect Google |
| `revoked` | Google stopped answering for Knowlu. While Google reviews Knowlu, connections expire after seven days. | Reconnect, Disconnect Google |
| error | the command's `error` sentence, verbatim | Try again |
| loading | Checking Google… | none |

Under every state, `#set-google-note` carries two sentences:

- "While Google reviews Knowlu, this works only for invited testers, and the connection needs
  renewing about once a week."
- "Google also tells Knowlu which Google account you connected, so this row can show it."

**While polling** (after Connect or Reconnect): the state reads "Waiting for Google… finish in your
browser." and every button is disabled.

**On poll timeout:** "Google did not finish connecting. If Google said Knowlu is not verified, this
Google account is not on the tester list yet." The row then repaints from one fresh status call.

**Disconnect, step 1** (`#set-google-disconnect-1`, "Disconnect Google") reveals this text and step 2:

> This disconnects Google Calendar too — Google keeps them as one permission. Knowlu stops reading
> both and deletes the proposals from your mail it had not delivered yet. What it already added stays
> in your vault, and your calendar stops updating.

**Disconnect, step 2** (`#set-google-disconnect-2`): "Yes, disconnect". On `ok: false` the row keeps
its previous state and shows the error under it. Closing Settings hides step 2 again.

**The wizard's Gmail panel** (`#wiz-gmail`): the "Not yet … Skip it for now" text becomes:

- "Knowlu can read your Gmail for things you have to do and propose each one for you to approve. You
  can skip this and connect later in Settings."
- the Testing sentence above, verbatim;
- the button `#wiz-gmail-connect`, "Connect Gmail";
- after a connection lands, `WIZ.gmailNote` reads "Gmail is connected."; on timeout it reads the
  Settings timeout sentence, verbatim.

The page never names a scope string (D9), so `console.js` still never contains `gmail.readonly`.

## 5. Tasks

Every task writes its named tests first and sees each fail for the stated reason, then writes the
code. Cargo tasks end with `cargo build --workspace` and `cargo test --workspace` from the root, dev
profile, `-j 2`, in the foreground: green, 0 warnings (the one accepted `.rsrc` line aside), the four
`#[ignore]`s untouched. Deno tasks end with `deno check`, `deno lint` and `deno test` over
`cloud/supabase` green. **CL** marks a contract-list file. **Starts on Opus** and **Needs Quinn** are
stated on every task. Each agent appends one line to the stream's ledger,
`.superpowers/sdd/2026-09-29-ultracode/gmail-connect/ledger.md`, which the controller creates at T0.

| Task | Agent | Starts on Opus | Needs Quinn | Lane |
|---|---|---|---|---|
| T0 pre-flight | main session | yes | PQ1 | — |
| T1 `gmail-read` lookup | cloud-engineer | yes (high) | no | cloud |
| T2 `google-connect` status and reconnect | cloud-engineer | yes (high) | no | cloud |
| T3 the purge migration | cloud-engineer | yes (high) | no | cloud |
| T4a `send_json` and the error rows (CL) | contract-engineer | yes (xhigh) | no | app |
| T4b the three cores and the wizard refactor (CL) | contract-engineer | yes (xhigh) | no | app |
| T5 console commands (CL) | contract-engineer | yes (xhigh) | no | app |
| T6 engine D7 | implementer | no (Sonnet high) | no | engine |
| T7a engine D4: the gate and the new tests | implementer | no (Sonnet high) | no | engine |
| T7b engine D4: the tabled tests re-pointed | implementer | no (Sonnet high) | no | engine |
| T8 transport stop | implementer, `model: opus` | yes (high) | PQ1 first | engine |
| T9a Settings row: markup and status render | console-ui | no (Sonnet medium) | no | page |
| T9b Settings row: connect, reconnect, the poll | console-ui | no (Sonnet medium) | no | page |
| T9c Settings row: disconnect and the walk | console-ui | no (Sonnet medium) | no | page |
| T10 wizard button | console-ui | no (Sonnet medium) | no | page |
| T11 integration | integrator | yes (high) | no | branch |
| T12 staging deploy | main session | yes | the go | — |
| T13 live proof | main session | yes | at the browser | — |
| T14 docs | docs-keeper | no (Sonnet medium) | no | branch |
| T15 whole-branch review | reviewer, contract-reviewer | yes (high; xhigh) | merge and push | — |

### T0. Pre-flight

**Agent:** main session. **Why:** decisions, worktrees and build slots are the controller's.
**Starts on Opus:** yes. **Needs Quinn:** yes, PQ1 only (one question, with §3's context).
**Files:** none under `engine/`, `app/`, `cloud/`; this plan's §3; under PQ1 (i), the spec's three
passages named below; creates the ledger and the four lane branches.

**Steps:**
- Confirm the worktree's base is main `4cf3661` or later and that `cargo test --workspace` is green
  there, so every later red is the stream's.
- Create the four lane branches of P3.
- Ask Quinn PQ1. Record the answer in this plan's §3 in a docs commit.
- **If Quinn picks (i)**, the same docs commit also edits the spec
  (`docs/specs/2026-09-29-gmail-connect-design.md`): §4.3's "Under Q1 (a′) only" bullet, §8.2
  item 9, and the Q1 (a′) option text each come to read "exactly one skipped line,
  `judge: skipped (no network (…))`; tier-1 answers are still written; no `gmail:`, `rules:` or
  `labels:` line", and each edit cites Quinn's answer and its date. The spec's signature line is
  not re-dated; the edit is marked as an amendment by Quinn's PQ1 ruling. Under (ii) the spec is
  not touched.
- Hand each lane its briefs by heading (this section's task headings).

**Done when:** the base is green, the lanes exist, and PQ1 is answered (with the spec edit under
(i)) or T8 is marked waiting.

### T1. Cloud: `gmail-read` answers `no_gmail_scope` for an account with no row

**Agent:** `cloud-engineer`. **Why:** `cloud/` is theirs, and a wrong lookup either puts a false
"re-connect" line on every slot of every student or hides a revoked grant.
**Starts on Opus:** yes (high). **Needs Quinn:** no.
**Files:** `cloud/supabase/functions/gmail-read/{handler.ts,index.ts,handler_test.ts}`.

**Tests first** (in `handler_test.ts`):
- `lookupFromRow: no row reads as a missing scope and marks nothing`: the result is
  `missing: "scope"`, and a spy `markRevoked` is never called.
- `lookupFromRow: a revoked row reads as a missing grant`.
- `lookupFromRow: an active row without the Gmail scope reads as a missing scope`.
- `lookupFromRow: an active row with the Gmail scope takes the token path`.
- `readHandler: no client configured still answers 503` (today's behaviour, pinned through the new
  function).
- `readHandler: an account with no google_accounts row answers quiet no_gmail_scope`: the reply is
  `{quiet: true, reason: "no_gmail_scope"}` (plus today's `items`), and no mark is made.

**Behaviour:** spec §4.1's `gmail-read` bullet. `lookupFromRow(row | null)` is pure and lives in
`handler.ts`. `index.ts`'s row select drops its `status=neq.revoked` filter and keeps its
`account_id=eq.` filter. `markRevoked` stays idempotent. The token path and the 503 are unchanged.

**Done when:** the six tests pass; the account-scoping scan (every `google_accounts` select carries
`account_id=eq.`) and every other `gmail-read` test pass unchanged.

### T2. Cloud: `google-connect` returns `status` and `email`, and accepts `scope=reconnect`

**Agent:** `cloud-engineer`. **Why:** as T1; a wrong `reconnect` asks Google for scopes the student
never granted, or fewer than the row claims (P3's live defect).
**Starts on Opus:** yes (high). **Needs Quinn:** no.
**Files:** `cloud/supabase/functions/google-connect/{handler.ts,index.ts,handler_test.ts}`.
`disconnect.ts` and `disconnect_test.ts` are read only.

**Tests first** (in `handler_test.ts`):
- `status: no row answers none`: `{connected: false, scopes: [], status: "none", email: null}`.
- `status: an active row answers its scopes and email_hint`.
- `status: a revoked row answers not connected with no scopes`: `connected: false`, `scopes: []`,
  `status: "revoked"`.
- `status: a quiet row answers connected with its scopes`: `connected: true`, `status: "quiet"`.
- `scopeFor reconnect asks for every recorded scope in one consent`: a row with both API scopes gives
  one URL whose `scope` holds `openid`, `email`, `calendar.readonly` and `gmail.readonly`, with
  `include_granted_scopes=true` and `prompt=consent`.
- `scopeFor reconnect with a calendar-only row asks for calendar only`.
- `reconnect with no row, or a row with empty scopes, answers 400 nothing to reconnect`.

**Behaviour:** spec §4.1's `google-connect` bullet and D5, D8. `ConnectDeps.grantedScopes` becomes
`grant(accountId)`, one row read with no status filter. `DELETE` is unchanged. P4 governs the existing
fakes. The account-scoping scan still passes.

**Done when:** the seven tests pass; `handler_test.ts`'s existing assertions and `disconnect_test.ts`
pass unchanged, and the diff to `handler_test.ts` outside the new tests touches fakes only (P4).

### T3. Cloud: Disconnect purges the queue and the read ledger (D14)

**Agent:** `cloud-engineer`. **Why:** a wrong purge deletes another account's rows, and a wrong grant
opens a `security definer` function to the client.
**Starts on Opus:** yes (high). **Needs Quinn:** no (the staging push is T12's).
**Files:** new `cloud/supabase/migrations/20260929000200_gmail_disconnect_purge.sql` (P5);
`cloud/supabase/migrations/migrations_test.ts`. `20260911000200_google.sql` is read only.

**Tests first** (in `migrations_test.ts`, finding the last definition the way the
`gmail_queue_tier_check` test finds the last constraint):
- `the last delete_google_grant purges the account's gmail_queue and gmail_seen rows`: its body
  deletes from `gmail_queue` and from `gmail_seen`, each filtered on `account_id = p_account` and on
  nothing looser, and still deletes the `google_accounts` row and its Vault secret.
- `the last delete_google_grant keeps judgments` (Q9 (a)(ii)): the body contains no `delete from
  judgments` and no `judgments` reference at all.
- `the last delete_google_grant is still security definer, service-role only`: `security definer`,
  a pinned `search_path`, and the same file re-issues `revoke … from public, anon, authenticated` and
  `grant … to service_role`.
- `the original google migration is unchanged`: `20260911000200_google.sql`'s text equals a
  SHA-256 recorded in the test from the base's bytes. This one is a guard, green from the start;
  the reviewer checks that the hash was taken from the base commit, not from a working copy.

**Behaviour:** spec §4.1's migration bullet. `create or replace` with the same signature. The
file's header comment names D14 and Q9 (a)(ii).

**Done when:** the four tests pass; the file's existing `security definer` scan and every other
migration test pass unchanged.

### T4a. App: `send_json` and the two error rows

**Agent:** `contract-engineer`. **Why:** `app/src/account.rs` is on the contract list and handles the
session token; a token in a log or an unchecked URL opened from Rust are the failures that matter.
**Starts on Opus:** yes (xhigh). **Needs Quinn:** no.
**Files:** `app/src/account.rs` (CL) only.

**Tests first** (in `account.rs`'s own `google_error_for_status_tests`, each seen red first): 402
gives "your subscription is not active, so Google cannot be connected"; 502 gives "Google could not
be reached to disconnect; try again"; 401, 503 and the generic form are unchanged.

**Behaviour:** the two error rows (spec §4.2). `get_json` becomes a thin wrapper over a new
`send_json(method, url, token)`; `check_api_base` applies to both. This half adds no caller of
`send_json` beyond `get_json`; the refactor's guard is that every existing test in
`app/tests/account.rs` passes unchanged. `PRIVACY_VERSION` is not touched.

**Done when:** the extended error tests pass, and the full `app/tests/account.rs` run is green with
its lock and unchanged.

### T4b. App: the three pure cores, and the wizard on them

**Agent:** `contract-engineer`. **Why:** as T4a.
**Starts on Opus:** yes (xhigh). **Needs Quinn:** no.
**Files:** `app/src/account.rs` (CL); `app/tests/account.rs` (treated as CL). Runs after T4a on
the same lane.

**Tests first** (each seen red first):
- In `app/tests/account.rs`, each against the file's `127.0.0.1:0` loopback helper, which records
  the method, path, bearer and `apikey`:
  - `google_status_at_reads_each_state_and_both_scopes`: the four `status` values map to the four
    states; `calendar` and `gmail` test the full scope URLs; `email` is carried.
  - `google_status_at_reads_an_older_servers_reply`: a reply with no `status` and no `email` reads
    as `active` when `connected`, `none` otherwise, with `email: None`.
  - `google_connect_url_at_asks_for_the_named_scope_with_the_bearer`: `gmail`, `reconnect` and
    `calendar` pass through; anything else asks for `calendar`; the request is
    `GET /google-connect?scope=…` with the bearer and `apikey`.
  - `google_connect_url_at_refuses_a_url_knowlu_will_not_open`: a reply naming
    `https://evil.example/…` is `Err("the service returned a url Knowlu will not open")`.
  - `google_disconnect_at_sends_delete_and_maps_each_status`: `DELETE /google-connect`; 200
    `{disconnected:true}` is `Ok`; 502, 401, 402 and 503 each give T4a's sentences.

**Behaviour:** spec §4.2's pure cores and D3. The cores (`google_status_at`,
`google_connect_url_at`, `google_disconnect_at`) call T4a's `send_json`. The wizard's three
commands (`google_connect_url`, `google_connected`, `open_external`) keep their names, signatures
and behaviour and call the cores. `PRIVACY_VERSION` is not touched. No token, URL or Google email
reaches a log line or an error string.

**Done when:** the five loopback tests pass; every existing wizard test in `app/tests/account.rs`
passes unchanged, and `no_test_in_this_file_can_reach_the_compiled_in_project` covers the new ones.

### T5. App: the three console commands

**Agent:** `contract-engineer`. **Why:** as T4a; this is where the session target is chosen, and the
pending target must never be read after onboarding.
**Starts on Opus:** yes (xhigh). **Needs Quinn:** no.
**Files:** `app/src/account.rs` (CL); `app/tests/account.rs`. Runs after T4b on the same lane.
`app/src/main.rs` is **not** touched here; the three registrations are T11's.

**Tests first** (in `app/tests/account.rs`):
- `the_console_google_commands_use_the_vaults_session_not_the_pending_one`: holds the file's
  `CREDMAN_LOCK`; a generated test profile id and a `Drop` guard; a scratch vault whose
  `config/cloud.yaml` points `api_base` at the loopback and names the generated target;
  `KNOWLU_API_BASE` set to the same loopback through the file's existing env guard; a token stored
  under the generated target only, and `PENDING_TARGET` never written. Calling
  `google_status_in(&vault)` makes the server see the profile's bearer.
- `google_connect_in_returns_nothing_to_open_when_the_url_is_refused`: the same setup with a
  loopback that answers an `https://evil.example/…` URL; the inner function returns T4b's sentence as
  an error and no URL, so the Tauri wrapper has nothing to open.
- `google_disconnect_in_reports_a_502_and_changes_nothing_locally`: `ok: false`, the "try again"
  sentence, and no credential or vault file changes.

**Behaviour:** spec §4.2's command table and D2. Each command has a non-Tauri inner function taking
the vault path (`google_status_in`, `google_connect_in`, `google_disconnect_in`); the Tauri wrapper
reads `ConsoleState`. Each reads `cloud_config(&cs.vault)` and the vault's
`session_credential_target`, never `PENDING_TARGET`. `google_connect` checks the URL with
`external_url_allowed`: the inner function returns the checked URL, and only the Tauri wrapper opens
it from Rust, as `open_portal` does. Replies: `{ok, state, calendar, gmail, email, error}` and `{ok, error}`.

**Done when:** the named tests pass, the full `app/tests/account.rs` run is green with its lock, and
the contract-reviewer has read T4a, T4b and T5's diff (§6, checkpoint A).

### T6. Engine: every Gmail item is a proposal (D7)

**Agent:** `implementer`. **Why:** off the contract list, fully specified, and every test change is
enumerated, so none is a judgment call. If the stop rule fires, the task goes back to the controller;
it does not move up an effort level on its own.
**Starts on Opus:** no (Sonnet, high). **Needs Quinn:** no.
**Files:** `engine/src/enrich.rs` only.

**The four steps (P2), in this order:**
1. **New tests, seen red.** Write `a_clear_task_email_becomes_a_proposal_card` (below) and add the
   `task`-tier items and their assertions to `an_over_budget_gmail_batch_is_snoozed_not_dropped`.
   Run `cargo test -p knowlu-engine --lib enrich` on the unchanged source and see each fail because
   a `task`-tier item still becomes a note, not a card. A new test that passes here is a hand-back.
   Write the rewrite `an_approved_gmail_card_materialises_todays_gmail_note_bytes` in this step
   too; it pins bytes that must not move, so if it is green on the unchanged source it is recorded
   in the ledger as a guard (as T3's hash test is), not treated as a failure to see red.
2. **Source change only**, as below. No test is edited in this step.
3. **The stop rule.** Run `cargo test -p knowlu-engine --lib enrich` and
   `cargo test -p knowlu-engine --test cloud_contract`. Step 1's tests now pass. Any failing test
   that is not a row of the spec's §7 D7 table stops the task and goes back to the controller,
   unedited.
4. **Re-point the tabled tests**, one per row, as listed below, then run the workspace gate.

**Source change:** in `pull_gmail`, the `"task"` tier routes to `write_gmail_card` exactly as
`"borderline" | "event" | "opportunity"` do, and counts in `cards`. `write_gmail_note` loses its last
caller and is deleted, not kept under `#[cfg(test)]`. `gmail_note_text` stays (the card's fenced
`task` block is built from it). The `notes` counter is never incremented, so it becomes immutable; the
summary line still prints its `task(s)` count, which reads 0.

**Tests, one per row of the spec's §7 D7 table, each done exactly as tabled** (the rewritten,
replaced and extended rows in step 1; the re-pointed rows, and the old tests the replacements
retire, in step 4):
- *Rewritten:* `an_approved_gmail_card_materialises_the_same_note_a_task_tier_would` becomes
  `an_approved_gmail_card_materialises_todays_gmail_note_bytes`. The approved card's note equals,
  apart from its `id:` line, the literal that `an_item_without_one_writes_todays_bytes` holds today.
  **The literal is copied from the old test verbatim, never regenerated from new output.** If the
  approved card does not produce it, stop and hand back: the engine is wrong, not the literal.
- *Replaced:* `a_clear_task_email_becomes_a_note_with_created_by_gmail` by
  `a_clear_task_email_becomes_a_proposal_card` (spec §8.2 item 5): one `approvals/` card with
  `kind: task`, `created_by: gmail`, `source_uid: gmail:<id>`, the journal actor
  `agent:knowlu.gmail`; no `tasks/` note; the uid acked and seen-recorded; the summary
  `gmail: 0 task(s), 1 proposed, 0 dropped as information`.
- *Re-pointed:* `a_pulled_task_note_carries_the_email_judgment_id` →
  `a_pulled_task_card_carries_the_email_judgment_id`; `an_item_without_one_writes_todays_bytes`;
  `a_failed_round_stops_the_pull_and_says_so`; `nothing_is_acknowledged_that_was_not_written`;
  `a_malformed_due_is_skipped_not_written_and_not_acknowledged`;
  `a_duplicate_queue_row_in_the_same_batch_produces_only_one_note` → `…_produces_only_one_card`;
  `a_duplicate_row_whose_first_occurrence_failed_is_never_acknowledged`. Each is changed only as the
  spec's table says; no other assertion is loosened.
- *Extended:* `an_over_budget_gmail_batch_is_snoozed_not_dropped` gains `task`-tier items and still
  snoozes past 15, never deleting (§8.2 item 6). `cli.rs`'s
  `gmail_and_event_digest_share_one_joint_daily_budget` is not touched.

**Done when:** the D7 rows are done as tabled; the unchanged tests the spec names (the other-tier and
completion tests) pass untouched; the workspace is green with 0 warnings.

### T7a. Engine: pull Gmail on every cloud slot (D4), the gate and the new tests

**Agent:** `implementer`. **Why:** as T6.
**Starts on Opus:** no (Sonnet, high). **Needs Quinn:** no.
**Files:** `engine/src/enrich.rs`. Runs after T6 on the same lane.

**The first three steps (P2), in this order; step 4 is T7b:**
1. **New tests, seen red.** Write the three tests under "Tests first" below. Run
   `cargo test -p knowlu-engine --lib enrich` on the unchanged source and see each fail for its
   reason: the first two because a vault with no calendar marker, or with nothing waiting, makes no
   `/gmail-read` request; the third because its vault, like the second's, has no calendar marker
   and nothing waiting, so today it never reaches the pull and prints no reconnect line. A new test that passes here is a hand-back.
2. **Source change only**, as below. No existing test is edited in this step.
3. **The stop rule.** Run `cargo test -p knowlu-engine --lib enrich` and
   `cargo test -p knowlu-engine --test cloud_contract`. Step 1's tests now pass. Any failing test
   that is not a row of the spec's §7 D4 table stops the task and goes back to the controller,
   unedited. Record in the ledger which tabled rows failed; they are T7b's list. The workspace
   gate is T7b's, since the tabled rows are red until then.

**Source change** (spec §4.3, "The gate"):
- `run_lines_with` no longer reads the calendar marker. **The cloud arm's early return is removed
  whole**, not just its `!google` conjunct (deleting the conjunct alone would make it fire more often).
- The probe runs on every cloud slot; `pull_gmail` runs unconditionally once past it.
- `google_calendar_linked` and `rule_decisions_waiting` lose their only callers and are deleted.
  `labels_waiting` stays (it is `pub`, and `cloud_contract.rs` calls it).
- The comment above the probe is rewritten: the probe now runs every cloud slot, why (the grant lives
  in the account, not the vault), and that R-C2-E15, R-C2-E38 and R-C2-E46 are superseded for this
  arm. The doc comments that name the removed predicates are corrected.
- The local arm (no cloud client) is untouched.

**Tests first (new, spec §8.2 items 1, 2 and 4):**
- `the_gmail_pull_runs_without_a_calendar_marker`: a vault with `config/cloud.yaml`, a session and
  no `calendars:` entry makes a `POST /gmail-read` after the probe. Replaces the marker test.
- `a_bare_vault_probes_pulls_mail_and_pulls_rules_in_that_order`: no pending item, no event feed, no
  rule decision, no label; the requests are the probe, `/gmail-read` (answered `no_gmail_scope`) and
  `GET /judge-rules`, in that order, and the `no_gmail_scope` reply adds no line. Replaces
  `an_empty_queue_makes_no_request_to_the_service`.
- `a_revoked_grant_through_the_whole_arm_prints_the_reconnect_line_and_exits_zero`: through
  `run_lines_with`, exactly `gmail: skipped (gmail is not connected; re-connect from settings)` and
  exit 0.

**Done when:** the three new tests were seen red, then pass; the stop rule held; the failing tabled
rows are in the ledger. `cargo build --workspace` is warning-free (a dead `google_calendar_linked`
or `rule_decisions_waiting` would be a warning).

### T7b. Engine: the D4 table's tests re-pointed

**Agent:** `implementer`. **Why:** as T6.
**Starts on Opus:** no (Sonnet, high). **Needs Quinn:** no.
**Files:** `engine/src/enrich.rs` (tests only); `engine/tests/cloud_contract.rs` (one test). Runs
after T7a on the same lane. No source line outside `#[cfg(test)]` changes here.

**Step 4 (P2).** Re-point each row below, and only these. A row that T7a's ledger line did not list
as failing is still done as tabled if the spec's table changes it; any other test that fails is a
hand-back.

**Existing tests, one per row of the spec's §7 D4 table, each done exactly as tabled:**
- *Replaced:* `an_empty_queue_makes_no_request_to_the_service` and
  `the_gmail_pull_runs_only_when_the_vault_has_linked_a_google_calendar` (by the two tests above).
- *Re-pointed:* `the_events_pass_runs_after_an_empty_enrichment_batch_and_the_verdict_reaches_the_ledger`
  (five requests, `/gmail-read` fourth); `an_answered_rule_card_alone_makes_the_probe_fire_and_a_bare_vault_makes_none`
  → `an_answered_rule_card_is_sent_and_a_bare_vault_still_asks_for_mail`;
  `cloud_contract.rs::the_probe_fires_when_only_a_label_is_waiting` (four requests, `/gmail-read`
  third, the telemetry POST at index 3).
- *Re-scripted:* `a_capped_reply_stops_the_batch_after_one_request_and_the_log_says_capped` gains a
  `no_gmail_scope` reply before the rule pull's; no assertion changes.
- Checked and unchanged: `a_service_that_cannot_be_reached_still_writes_tier_one_and_exits_zero`
  (T8 owns its (a′) change) and `a_probe_that_eats_the_budget_leaves_the_batch_and_the_rules_pull_for_the_next_slot`.
  `a_quiet_no_gmail_scope_reply_prints_no_line_and_makes_no_second_request` stays as it is (§8.2
  item 3).

**Done when:** the D4 rows are done as tabled; the workspace is green with 0 warnings.

### T8. Engine: the transport stop (Q1 (a′))

**Agent:** `implementer`, dispatched with `model: opus`. **Why:** it changes what an offline slot
writes and touches `probe()`, whose "a fatal status sets `model.fatal()`" behaviour other paths rely
on. Off the contract list, but a silent error here changes vault writes on every offline slot, so it
starts one level up.
**Starts on Opus:** yes (high). **Needs Quinn:** yes, PQ1 must be answered first (T0).
**Files:** `engine/src/cloudmodel.rs` (`probe()` and its doc comment only); `engine/src/enrich.rs`.
Runs after T7b on the same lane. P2's four steps apply: the three new tests are seen red on T7b's
tip before `probe()` or the arm changes (except `a_probe_answered_503_still_runs_the_arm` and the
fatal half of `a_fatal_probe_still_sets_model_fatal`, which pin today's behaviour and are recorded
as guards); the found row is changed last.

**Tests first:**
- `a_transport_failed_probe_ends_the_arm_with_one_named_line` (spec §8.2 item 9): a bare vault whose
  `api_base` points at a `127.0.0.1:0` listener that is bound and then dropped. The run exits 0; the
  first line starts `judge: skipped (no network (`; no line starts `gmail:`, `rules:` or `labels:`;
  under PQ1 (ii) the skip line is the only line, under (i) the only other line is
  `judge: nothing to enrich`. The line never contains the session token.
- `a_probe_answered_503_still_runs_the_arm`: a loopback answering the probe 503, then scripted replies
  for `/gmail-read` and `/judge-rules`; all three requests are made, as today.
- `a_fatal_probe_still_sets_model_fatal` (in `cloudmodel.rs`): 401, 402 and 403 on the probe still
  set `model.fatal()` to their label; a transport failure does not.
- The found row (§3): `a_service_that_cannot_be_reached_still_writes_tier_one_and_exits_zero`. Under
  PQ1 (i) it keeps its name and assertions and gains one: the pending note carries tier 1's fields.
  Under (ii) it is re-pointed to `…_writes_nothing_and_exits_zero`, asserting the skip line and an
  unwritten note.

**Behaviour:** spec §4.3's Q1 (a′) bullet, as PQ1 answers it. `probe()` gains a way to report the
transport case separately (its existing `Option` answer and its effect on `model.fatal()` stay as they
are for every other caller). A 5xx or 429 on the probe is not transport and keeps today's path.

**Done when:** the four tests pass; T7a's and T7b's tests still pass; the workspace is green with 0
warnings.

### T9a. Page: the Settings row's markup and status render

**Agent:** `console-ui`. **Why:** `app/static` is theirs; the copy is final in §4, so no product
decision is left in the task.
**Starts on Opus:** no (Sonnet, medium). **Needs Quinn:** no.
**Files:** `app/static/{index.html,console.js,console.css}`; `app/tests/static_assets.rs`.

**Tests first** (in `static_assets.rs`, spec §8.4 items 1 and 7; each seen red first):
- `the_google_row_sits_after_account_with_its_ids`: `#set-google` is inside `#settings`, after
  `#set-account`, and carries `set-google-state`, `set-google-connect`, `set-google-reconnect`,
  `set-google-disconnect-1`, `set-google-disconnect-2` and `set-google-note`.
- The existing no-`http://`-or-`https://` and `gmail.readonly` tests pass unchanged.

**Behaviour:** spec §4.4's row and D1, D10, with §4's copy for the `none`, calendar-only, Gmail,
`revoked`, error and loading states and the two `#set-google-note` sentences. `SET` is a
settings-scoped state object this task introduces. The row is hidden when `account_status` says
`needs_account`. It asks `google_status` each time Settings opens, keeps no cache (D12), and shows
the buttons §4's table names for each state. The buttons are present but not yet wired (T9b, T9c).
No new `uievents` action (D12).

**Done when:** the ids test passes, the existing tests are unchanged and green, and every state
sentence in §4's table appears in `console.js` verbatim.

### T9b. Page: Connect, Reconnect and the `SET.googleSeq` poll

**Agent:** `console-ui`. **Why:** as T9a.
**Starts on Opus:** no (Sonnet, medium). **Needs Quinn:** no.
**Files:** `app/static/console.js`; `app/tests/static_assets.rs`. Runs after T9a on the same lane.

**Tests first** (spec §8.4 items 2 and 3; each seen red first):
- `the_console_invokes_the_three_google_commands_and_never_open_external_outside_the_wizard`:
  `console.js` invokes `"google_status"`, `"google_connect"` and `"google_disconnect"`; outside the
  wizard's handlers it never invokes `"open_external"`. (Its `google_disconnect` half is red until
  T9c; T9b records that in the ledger rather than stubbing the call.)
- `the_google_row_poll_is_cancelled_when_settings_closes`: the row's poll carries a
  `SET.googleSeq` token, bumped on Settings close and on every new click, pinned the way
  `the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it` pins the wizard's.

**Behaviour:** Connect calls `google_connect({scope: "gmail"})`, Reconnect
`google_connect({scope: "reconnect"})`; each then polls `google_status` every 3 s, up to 20 times,
stopping when `gmail` is true (Connect) or `state` is `active` (Reconnect). While polling, §4's
polling sentence shows and every button is disabled; on timeout, §4's timeout sentence shows and the
row repaints from one fresh status call.

**Done when:** the poll test passes, and the commands test passes apart from its disconnect half.

### T9c. Page: the two-step Disconnect, and the walk

**Agent:** `console-ui`. **Why:** as T9a.
**Starts on Opus:** no (Sonnet, medium). **Needs Quinn:** no.
**Files:** `app/static/{console.js,console.css}`; `app/tests/static_assets.rs`;
`scripts/settings-check.py`. Runs after T9b on the same lane.

**Tests first** (spec §8.4 item 4; seen red first):
- `the_disconnect_confirm_names_calendar`: the step-1 text contains "Google Calendar too".

**Behaviour:** spec D6 with §4's copy. `#set-google-disconnect-1` reveals the confirm text and
`#set-google-disconnect-2`; step 2 invokes `google_disconnect`; on `ok: false` the row keeps its
previous state and shows the error under it; closing Settings hides step 2 again.

**Walk:** `scripts/settings-check.py` walks the six states of §4's table with a stubbed `invoke`
(`none`, calendar only, Gmail, `revoked`, error, and a poll timeout) and the two-step Disconnect,
and ends `ok`.

**Done when:** T9a–T9c's tests all pass (the commands test whole), the walk ends `ok`, and
`static_assets.rs` is green.

### T10. Page: the wizard's Connect Gmail button (D11)

**Agent:** `console-ui`. **Why:** as T9a.
**Starts on Opus:** no (Sonnet, medium). **Needs Quinn:** no.
**Files:** `app/static/{index.html,console.js}`; `app/tests/static_assets.rs`;
`scripts/wizard-check.py`. Runs after T9c on the same lane.

**Tests first** (spec §8.4 item 5):
- `the_wizard_gmail_flow_keeps_its_state_on_wiz_and_renders_it`: `#wiz-gmail` has
  `#wiz-gmail-connect`; its state lives on `WIZ` (`gmail`, `gmailNote`, `gmailPolling`, `gmailSeq`) and
  is painted by `renderWizard()`, following the A-5 rule the calendar button's test pins; `wizGo`
  bumps `WIZ.gmailSeq` on leaving the panel.
- `the_wizard_gmail_button_uses_the_wizard_commands`: the handler invokes `"google_connect_url"` with
  `scope: "gmail"` and polls `"google_connected"`; it invokes no console-only command.
- `next_never_waits_on_gmail`: the Gmail panel's Next is enabled while `WIZ.gmailPolling` is true,
  and nothing about Gmail is written into the plan the wizard sends at Finish.

**Behaviour:** spec §4.4's wizard bullet, with §4's copy. Skip remains the default path.

**Walk:** `scripts/wizard-check.py` walks the nine panels with the Gmail button present and ends `ok`.

**Done when:** the named tests pass, both walks end `ok`, and `static_assets.rs` is green.

### T11. Integration: the lanes merged, the handlers registered

**Agent:** `integrator`. **Why:** the merge train and the one shared file (`app/src/main.rs`) are
theirs; a command in the wrong list fails only at run time.
**Starts on Opus:** yes (high). **Needs Quinn:** no.
**Files:** `app/src/main.rs`; new `app/tests/handler_lists.rs` (P6); merge commits into
`gmail-connect`.

**Steps:**
- Merge `gmail-connect-cloud`, `-app`, `-engine` and `-page` into `gmail-connect`, each only after its
  checkpoint in §6 has passed. Re-stamp T3's migration if P5 requires it.
- **Test first**, `the_console_list_names_the_three_google_commands_and_the_wizard_list_none`: over
  `main.rs`'s source text, the console `generate_handler!` list names `account::google_status`,
  `account::google_connect` and `account::google_disconnect`; the wizard list names none of them; the
  console list still does not name `account::open_external`. It fails until the next step.
- Add the three names to the console list. Recount both lists and record the counts in the ledger
  (the spec expects the console list to go from 47 to 50 and the wizard list to stay at 29; the
  recount is what is quoted).
- Run the full gate: `cargo build --workspace`, `cargo test --workspace` (0 warnings, the four
  `#[ignore]`s untouched, `oracle.rs`, `surface_oracle.rs`, `dependency_boundary.rs` and
  `no_console.rs` green), and the Deno suite over `cloud/supabase`.
- Confirm from the branch diff that `site/privacy.html`, `engine/tests/site.rs`, `PRIVACY_VERSION`
  and `engine/tests/fixtures/**` are byte-identical to the base.

**Done when:** the four lanes are merged, the new test passes, and the full gate is green.

### T12. Staging deploy of the cloud half

**Agent:** main session. **Why:** staging pushes are the controller's, and a deploy is asked at the
time.
**Starts on Opus:** yes. **Needs Quinn:** yes, the go for the deploy.
**Files:** none; the staging project only.

**Steps:**
- Ask Quinn for the go, in one line, with T11's gate as the evidence.
- From the merged `gmail-connect` branch: `db push --include-all` (T3's migration), then deploy
  `gmail-read` and `google-connect`.
- A count-free smoke: `GET /google-connect?status=1` on the founder's staging session answers the four
  keys. Nothing that reads a row's contents.
- **Redaction (P11):** the smoke's record names the four keys present and the `status` value only;
  the `email` value is never written down, in the ledger, a commit or a report.

**Order:** this must land **before** any device build carrying T7a runs a slot against staging.
Otherwise every account with no Google row gets the false "re-connect" line (spec §0.2).

**Done when:** the migration and both functions are live on staging and the smoke passes.

### T13. The live proof (spec §9)

**Agent:** main session. **Why:** the OTP session, the proof harness and Quinn's consent click are the
controller's; subagents never hold the token.
**Starts on Opus:** yes. **Needs Quinn:** yes, at the browser for Google's consent (only Quinn's
address is a tester), and for revoking Knowlu at Google's third-party-access page in step 5.
**Files:** none in the repository; the proof's record goes into T15's report. **Redaction (P11):**
the record gives states, counts and yes/no observations only; it never includes the address the
row shows, a message id, a subject, a sender or a card's text, and no screenshot of the row or the
deck goes into the repository.

**Steps:** spec §9's steps 1–7, as written, on a dev build of `gmail-connect` pointed at staging
(`KNOWLU_API_BASE`), on a scratch profile, with the session from the OTP route. Step 3 also checks
that no `tasks/` note has a journal record by `agent:knowlu.gmail`. Step 6's count-only query runs
through the service role and reads no row's contents: after Disconnect the account has no
`gmail_queue` and no `gmail_seen` rows, and its `gmail_api` `judgments` count is unchanged (Q9
(a)(ii)). Step 7 removes the scratch profile, vault, credentials and autostart entry.

**Done when:** every step passes as written, and the redacted record (each step; the state seen,
the counts, yes or no) is handed to T15. Quinn's word that the item is done closes the MVP row.

### T14. Docs

**Agent:** `docs-keeper`. **Why:** docs only; HANDOFF, `docs/reference/` and the cloud README are
theirs.
**Starts on Opus:** no (Sonnet, medium). **Needs Quinn:** no.
**Files:** `HANDOFF.md`, `docs/reference/app.md`, `docs/reference/engine-commands.md`,
`cloud/supabase/README.md`. Runs after T11, so it quotes the final migration name and handler counts.

**Edits:**
- HANDOFF §3, MVP lane 2: the lane's state (built, proven or waiting, as it stands).
- HANDOFF §4, the production-parity row:
  - step (1): T3's migration added to the ordered list after J's `20260922120200`, in filename order,
    with the note that it reaches production before the first release that carries the row;
  - step (2): `gmail-read` and `google-connect` named as redeployed after that migration;
  - the order fix of spec §6: bump #1's page deploy (today's step (4)) comes **before** the Gmail
    project's `GOOGLE_CLIENT_ID` is set (today's step (3)), because the released 0.1.0 can connect
    Calendar the moment that secret exists.
- HANDOFF §4, the bump #1 row: spec §6's draft sentences, items 1, 2 and 4 in their Q9 (a)(ii) form,
  and spec §8.5's four tests, added to what the bump-#1 PR carries.
- HANDOFF §3, Pilot: the sender on a Gmail card (Q7 (c)) as a Pilot item that lands with bump #1;
  the release-gate line already there is checked, not duplicated (P8).
- `docs/reference/app.md`: the three console commands, their replies and their window list; the
  Settings row; the wizard's Gmail button.
- `docs/reference/engine-commands.md`: `judge` pulls Gmail on every cloud slot past the probe;
  every Gmail item is a proposal; the transport stop's line (as PQ1 answered it).
- `cloud/supabase/README.md`: `?status=1`'s four keys, `scope=reconnect`, and what
  `delete_google_grant` now deletes.

**Done when:** each edit is made, no edit touches `site/` or anything outside the four files, and the
reviewer has read the diff in T15.

### T15. Whole-branch review

**Agent:** `reviewer` for the branch; `contract-reviewer` for the contract-list diff
(`app/src/account.rs`, `app/tests/account.rs`). **Why:** the stream's last gate before Quinn's word;
the contract-list half is read at xhigh.
**Starts on Opus:** yes (reviewer high; contract-reviewer xhigh). **Needs Quinn:** yes, the word to
merge and the go for the code push.
**Files:** the report `docs/reports/2026-09-29-gmail-connect-review.md`, and nothing else.

**What it checks:**
- Every row of §9's fidelity ledger against the diff and the named tests.
- The spec's §7 tables row by row against T6's, T7a's and T7b's diffs: same scenario, no assertion
  dropped, a new count or path that follows from D4 or D7; and T8's found row against PQ1's answer.
- P2's order held: the ledger shows each new engine test seen red before its source change (or
  recorded as a guard), and T7b changed no line outside `#[cfg(test)]`.
- Under PQ1 (i), the spec's three passages carry T0's signed edit, and T8's output matches them.
- P4: `google-connect/handler_test.ts`'s diff outside the new tests touches fakes only.
- P9: `functions/account`'s tests pass unchanged.
- Byte-identity at the branch tip: `site/privacy.html`, `engine/tests/site.rs`, `PRIVACY_VERSION`,
  `engine/tests/fixtures/**` and `20260911000200_google.sql`.
- No token, URL or Google email in a log line or error string (T4a, T4b, T5); no scope string on the page;
  no new `uievents` action, `surface` key or note field (D12).
- T13's record is complete, and the scratch profile is gone.
- **Redaction (P11), before the report lands:** the report, the ledger lines it quotes and T12's
  smoke record hold states, counts and yes/no observations only; no Google address, message id,
  subject, sender or card text appears in any of them. The repository is public.
- CI is green on the branch.

**Done when:** the report lands with a verdict and T13's record. Any code fix after it re-runs the
touched task's tests and the affected proof steps, and the report says which. Quinn's word merges it;
the push is a code push and is asked.

## 6. Order, parallelism and checkpoints

- **Order.** T0 first. Then four lanes at once: cloud (T1, T2, T3), app (T4a → T4b → T5), engine
  (T6 → T7a → T7b → T8) and page (T9a → T9b → T9c → T10). T1, T2 and T3 touch disjoint files and
  may run in parallel; within the other lanes the splits run in sequence.
  T8 waits on PQ1. Then T11, T12, T13, T14 and T15, in that order.
- **Build slots.** One cargo at a time, `-j 2`, tests in the foreground. The app, engine and page
  lanes all run cargo, so the controller grants their slots in turn; the cloud lane runs Deno and
  needs no slot.
- **Checkpoint A (app).** After T5: `contract-reviewer` reads T4a, T4b and T5's diff before the
  lane merges.
- **Checkpoint B (cloud).** After T3: `reviewer` reads T1–T3, with the account-scoping and the
  purge's `account_id = p_account` filter as its first two questions.
- **Checkpoint C (engine).** After T6 and again after T8: `reviewer` walks the spec's §7 tables row by
  row against the diff, and T8's row against PQ1.
- **Checkpoint D (page).** After T10: `reviewer` reads T9a–T10 and the two walk outputs, and checks
  §4's copy is applied whole.
- **Checkpoint E (branch).** T15, with T13's proof. Quinn's word merges the branch.
- **Two failed attempts** on any task means one level up (Sonnet to Opus; Opus high to xhigh), never
  straight to `max`. T6, T7a and T7b return to the controller on the stop rule rather than
  escalating.

## 7. Quinn's items in this stream

- **T0:** PQ1 (§3), one question with its context. Under (i), the answer also signs T0's edit to
  three spec passages, so the spec and the plan say the same thing.
- **T12:** the go for the staging deploy.
- **T13:** at the browser for Google's consent, and for the revoke at Google's third-party-access
  page.
- **T15:** the word to merge, and the go for the code push.
- **After the stream:** the MVP row closes on Quinn's word. The purge migration and the two functions
  reach production through production parity, before the first release that carries the row.

## 8. Risks

- **The cloud half deploys late.** A D4 build running a slot against staging before T12 shows every
  account without a Google row a false "re-connect" line. T12 is ordered before any such slot, and
  production parity carries the same order (T14).
- **An unlisted test fails in the engine lane.** The spec's tables were built by reading; the stop rule
  (P2) turns any surprise into a hand-back, never an edit to pass.
- **The approved-card literal drifts.** The `:2171` rewrite's literal is the byte contract existing
  vaults hold. It is copied, never regenerated; a mismatch is an engine bug and a hand-back.
- **A release is cut before bump #1.** The page would still say Gmail is not connected. HANDOFF's
  release gate (P8) is the only guard; Q8 (a1) accepted that it is operational, not mechanical.
- **The Testing-mode wall.** A student not on Google's tester list meets Google's "not verified"
  page. The row's Testing sentence says so in advance, and the timeout sentence names the likeliest
  cause (D10).
- **Offline cost, if PQ1 lands on (ii).** Offline slots stop writing tier-1 answers until the next
  online slot. The next online slot writes them; nothing is lost, only delayed.

## 9. Fidelity ledger

Each row is one spec requirement, the task that builds it, and what proves it.

| Spec requirement | Task | What proves it |
|---|---|---|
| D1: a Google row in Settings after Account, hidden with `needs_account` | T9a, T9c | `the_google_row_sits_after_account_with_its_ids`; the `settings-check.py` walk |
| D2: three console commands on the vault's session, never `PENDING_TARGET`; the URL opened from Rust | T5, T11 | `the_console_google_commands_use_the_vaults_session_not_the_pending_one`; `google_connect_in_returns_nothing_to_open_when_the_url_is_refused`; `the_console_list_names_the_three_google_commands_and_the_wizard_list_none` |
| D3: the wizard's commands unchanged, on shared cores | T4a, T4b | the existing wizard tests in `app/tests/account.rs`, unchanged |
| D4: pull on every cloud slot; no row answers `no_gmail_scope`; the early return removed | T1, T7a, T7b, T12 | the four `lookupFromRow` tests; `readHandler: an account with no google_accounts row answers quiet no_gmail_scope`; `the_gmail_pull_runs_without_a_calendar_marker`; `a_bare_vault_probes_pulls_mail_and_pulls_rules_in_that_order`; §9 step 1 |
| D4, Q1 (a′): the transport stop | T8 | `a_transport_failed_probe_ends_the_arm_with_one_named_line`; `a_probe_answered_503_still_runs_the_arm`; `a_fatal_probe_still_sets_model_fatal` |
| D5: `?status=1` adds `status` and `email`; older replies still read | T2, T4b | the four `status:` Deno tests; `google_status_at_reads_each_state_and_both_scopes`; `google_status_at_reads_an_older_servers_reply` |
| D6: Disconnect revokes the whole grant after a two-step confirm naming Calendar | T9c, T13 | `the_disconnect_confirm_names_calendar`; §9 step 6 |
| D7: every Gmail item is a proposal | T6 | `a_clear_task_email_becomes_a_proposal_card`; `an_approved_gmail_card_materialises_todays_gmail_note_bytes`; §9 steps 3–4 |
| D8: Reconnect asks for the row's scopes in one consent | T2, T9b, T13 | `scopeFor reconnect asks for every recorded scope in one consent`; `scopeFor reconnect with a calendar-only row asks for calendar only`; `reconnect with no row, or a row with empty scopes, answers 400 nothing to reconnect`; §9 step 5 |
| D9: no new scope; the page names none | T9a, T10 | the existing `gmail.readonly` test in `static_assets.rs`, unchanged |
| D10: the row for every signed-in student, with the Testing sentence and the timeout sentence | T9a, T9b, T9c | §4's copy, checked by the `settings-check.py` walk (all six states, the timeout among them) |
| D11: the wizard's Connect Gmail button, Skip the default | T10 | `the_wizard_gmail_flow_keeps_its_state_on_wiz_and_renders_it`; `the_wizard_gmail_button_uses_the_wizard_commands`; `next_never_waits_on_gmail`; the `wizard-check.py` walk |
| D12: no vault, read-model or telemetry change | T15 | the review's D12 check; `oracle.rs` and `surface_oracle.rs` unchanged (T11) |
| D13: no page edit, no `PRIVACY_VERSION` move; release held behind bump #1 | T11, T14, T15 | the byte-identity checks in T11 and T15; HANDOFF's release gate (P8); the bump-#1 row's new sentences and tests (T14) |
| D14, Q9 (a)(ii): Disconnect purges `gmail_queue` and `gmail_seen`, keeps `judgments` | T3, T12, T13 | the four migration tests; §9 step 6's count-only query |
| D15, Q7 (c): no sender in the MVP; the sender is a Pilot item | T14 | HANDOFF's Pilot entry; the review's D12 check (no new card field) |
| §4.1 deploy: staging before any D4 slot; production parity steps (1) and (2) | T12, T14 | T12's record; HANDOFF's production-parity row |
| §6: bump #1's page before the Gmail client's secret | T14 | HANDOFF's production-parity row, reordered |
| §6, §8.5: the draft sentences and four tests handed to bump #1 | T14 | HANDOFF's bump-#1 row |
| §8.1 items 1–4 | T4a, T4b | the extended `google_error_for_status_tests` (T4a) and the five loopback tests (T4b) |
| §8.1 item 5 | T5 | `the_console_google_commands_use_the_vaults_session_not_the_pending_one` |
| §8.1 item 6 | T11 | `the_console_list_names_the_three_google_commands_and_the_wizard_list_none` |
| §8.1 item 7 | T4a, T4b, T5 | the wizard tests and `no_test_in_this_file_can_reach_the_compiled_in_project`, unchanged |
| §8.2 items 1, 2, 4 | T7a | the three new T7a tests, each seen red first |
| §8.2 item 3 | T7b | `a_quiet_no_gmail_scope_reply_prints_no_line_and_makes_no_second_request`, unchanged |
| §8.2 items 5, 6 | T6 | `a_clear_task_email_becomes_a_proposal_card`; `an_over_budget_gmail_batch_is_snoozed_not_dropped`, extended |
| §8.2 items 7, 8 | T11, T15 | the full gate; the review's row-by-row walk |
| §8.2 item 9 | T8 | `a_transport_failed_probe_ends_the_arm_with_one_named_line` |
| §8.3 items 1–2 | T1 | the six `gmail-read` tests |
| §8.3 items 3–5 | T2 | the seven `google-connect` tests; the account-scoping scan; `disconnect_test.ts` unchanged |
| §8.3 item 6 | T3 | the four migration tests |
| §8.3 item 7 | T13 | §9 step 6's count-only query |
| §8.4 items 1–4, 7 | T9a, T9b, T9c | the ids test (T9a); the commands and poll tests (T9b, the commands test whole after T9c); the Calendar-confirm test (T9c); the no-URL test, unchanged |
| §8.4 item 5 | T10 | T10's three tests |
| §8.4 item 6 | T9c, T10 | both walks end `ok` |
| §8 heading: each test seen failing before its code | every code task | the ledger's red-then-green line per new test, or its guard note (P2; T15 checks) |
| §9 steps 1–7 | T13 | T13's redacted record, in T15's report (P11) |
| Review check 7: no personal identifiers in reports | T12, T13, T15 | P11's redaction lines; T15's redaction check |
| Rule 1: no single-user assumptions | T5, T15 | the generated-target test; the review |
| Rule 2: no frozen reference regenerated | T11, T15 | `engine/tests/fixtures/**` byte-identical; `oracle.rs` and `surface_oracle.rs` green, unchanged |

**The spec's §7 test rows, one entry each.** The reviewer (checkpoint C and T15) marks each done as
tabled.

| Existing test (spec §7) | Change | Task | Done as tabled when |
|---|---|---|---|
| `an_approved_gmail_card_materialises_the_same_note_a_task_tier_would` | rewritten | T6 | its successor compares the approved card's note with the verbatim-copied literal |
| `a_clear_task_email_becomes_a_note_with_created_by_gmail` | replaced | T6 | `a_clear_task_email_becomes_a_proposal_card` passes, with the summary and ack assertions |
| `a_pulled_task_note_carries_the_email_judgment_id` | re-pointed | T6 | the same two keys, read from the card |
| `an_item_without_one_writes_todays_bytes` | re-pointed | T6 | the card has no `judgment_id` or `judgment_kind`; the byte literal lives on in the rewrite |
| `a_failed_round_stops_the_pull_and_says_so` | re-pointed | T6 | the first round's card exists; the skipped line and the two-request count are unchanged |
| `nothing_is_acknowledged_that_was_not_written` | re-pointed | T6 | the first item is `information`; `m1` acked, `m2` named `not written`, the summary ends `, 1 not written` |
| `a_malformed_due_is_skipped_not_written_and_not_acknowledged` | re-pointed | T6 | "no card" replaces the `tasks/` check; the other assertions are unchanged |
| `a_duplicate_queue_row_in_the_same_batch_produces_only_one_note` | re-pointed | T6 | one card, no `-2`, the new summary, two requests |
| `a_duplicate_row_whose_first_occurrence_failed_is_never_acknowledged` | re-pointed | T6 | "no note" becomes "no card"; the rest is unchanged |
| `an_empty_queue_makes_no_request_to_the_service` | replaced | T7a (new), T7b (old removed) | `a_bare_vault_probes_pulls_mail_and_pulls_rules_in_that_order` passes |
| `the_events_pass_runs_after_an_empty_enrichment_batch_and_the_verdict_reaches_the_ledger` | re-pointed | T7b | five requests, `/gmail-read` fourth; every other assertion unchanged |
| `the_gmail_pull_runs_only_when_the_vault_has_linked_a_google_calendar` | replaced | T7a (new), T7b (old removed) | `the_gmail_pull_runs_without_a_calendar_marker` passes |
| `a_capped_reply_stops_the_batch_after_one_request_and_the_log_says_capped` | re-scripted | T7b | one `no_gmail_scope` reply added; no assertion changed |
| `an_answered_rule_card_alone_makes_the_probe_fire_and_a_bare_vault_makes_none` | re-pointed | T7b | the waiting half unchanged but re-scripted; the bare half asserts the three requests and the one line |
| `cloud_contract.rs::the_probe_fires_when_only_a_label_is_waiting` | re-pointed | T7b | four requests, `/gmail-read` third, telemetry at index 3; `labels: sent 1` unchanged |
| `a_service_that_cannot_be_reached_still_writes_tier_one_and_exits_zero` (found by this plan, §3) | as PQ1 answers | T8 | (i): name and assertions kept, the tier-1 assertion added; (ii): re-pointed to `…_writes_nothing_and_exits_zero` |
| `google-connect/handler_test.ts` fakes (P4) | fakes re-pointed | T2 | the file's diff outside the new tests touches fakes only |

**The plan review's findings (2026-09-29), each checked against the spec and the code before
acting.** None was rejected.

| Finding | Checked | Resolution |
|---|---|---|
| T6 and T7 made the source change before writing their new tests, so the new tests were never seen red (spec §8 heading; CLAUDE.md TDD) | confirmed: both tasks opened with "Step 1, the stop rule", and T7's "Tests first" came after it | accepted: P2 now runs four steps (new tests red, source only, stop rule, re-point); T6, T7a and T8 follow it; a pinning test that is green before the change is recorded as a guard |
| T4, T7 and T9 were far over about 80 lines of edit | confirmed | accepted: P10 splits them into T4a/T4b, T7a/T7b and T9a/T9b/T9c, same lane and agent, in sequence |
| PQ1 (i) departs from signed wording, and the plan alone recorded the answer | confirmed: spec §4.3's Q1 (a′) bullet, §8.2 item 9 and the Q1 (a′) option text all say one line | accepted: under (i), T0 edits those three passages in the same docs commit, citing Quinn's answer and date; T15 checks it |
| T12, T13 and T15 could carry the founder's address or mail-derived text into a public report | confirmed: T12's smoke returns `email`, the row shows it, and §9 steps 3–4 show cards from real mail | accepted: P11, with redaction lines in T12, T13 and T15 and a ledger row for review check 7 |
