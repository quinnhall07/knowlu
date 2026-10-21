# Honest page and re-consent (lane L4): build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.
> Briefs are extracted by heading (`## T3. …`), never by line number.

**Status: final (r4 review PASS, minors applied).** Round 4's delta review,
`docs/reports/2026-10-19-l4-plan-review-r4.md`, passed with 2 minor findings and 2 nits, all applied in
place; no further round follows. Nothing runs until the controller has Quinn's answers to §5's
questions marked **(Quinn)**, or has deferred them. Earlier reports: `…-r1.md`, `…-r2.md`, `…-r3.md`.
Every finding's answer is in §11.

**Date:** 2026-10-19 (session 5, the friends-MVP push). **Lane:** L4 of
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5: "privacy.html rewritten to today's app
(mail table, grades, syllabus), PRIVACY_VERSION bump, re-consent screen, sign-out clears `pending_setups`
(email hand-back)"; starts after the email merge; owns `app/src/account.rs` (contract list) in this wave;
agents contract-engineer, console-ui, reviewer for the page text; **Quinn reads the page**.

**Authority, newest first.**
- The cloud design's **Amendment 2026-10-19** (signed; uncommitted in the main checkout, lands with
  `HANDOFF.md`'s bump-#1 line): L4's bump **is** privacy bump #1, carrying the sentences the Gmail
  connect (§6, Q8), email forwarding (§7, D18), M2 (§5) and grades (§11) specs assign to it, with no
  lawyer read. The lawyer read stays a before-Pilot item.
- Quinn's rulings in `.superpowers/sdd/2026-10-18-session-5/progress.md`: 2026-10-18 ruling 5 (honest
  page now; the screen built now so the lawyer-read version re-prompts everyone before the Pilot);
  row 39 item (5) (the registrar sentence rides L4's page, "at schools Knowlu supports"); row 105
  (C3 Q3: R12b adds D18's upload row after L4 merges, `PRIVACY_VERSION` unmoved); row 121 (L5 Q39,
  Q42: L4's page carries the syllabus spec's §6.1, and no student-facing syllabus task merges before
  it); rows 133, 135 and 143 (the lapsed-mailbox fix, `last_lapsed_at`, judged not a new data class).
- The friends-testers spec (`docs/specs/2026-10-18-friends-testers-design.md`, signed 2026-10-19,
  ledger row 135): §8.5 and task S8, Q7's two sentences on L4's page **before** L4's bump.
- The page draft, reviewed PASS with two minors, closed in round 4's polish (ledger rows 132, 134):
  `docs/notes/2026-10-19-l4-privacy-page-draft.md` ("the draft"). Its §3 is the page text; its
  Appendix A maps every claim tag `⟨cN⟩` to the code that makes it true. **This plan does not repeat the
  page text.** Task briefs cite the draft by section.
- `docs/plans/2026-10-18-p3-registrar-close-out-plan.md` §8 Q5 and O8 (the class-schedule sentence).

**Goal.** A student who installs the friends release reads a privacy page that says what this build
does: forwarded mail with its retention table, Gmail by Google's permission, the kept Blackboard and
mail sign-ins, grades in the account, the registrar's class schedule at supported schools, the syllabus
lines as "not in this version", and L8's daily counts. The page and `PRIVACY_VERSION` move together.
An account that accepted an older version sees one panel at launch, reads the page, and accepts;
until then the slot's cloud steps are named skips. Signing out leaves no unfinished mail setup behind
for the next account on the same profile.

**Architecture, in five pieces.**
1. **The page** (`site/privacy.html`, `site/site.css`): the draft's §3, less its *[after C3]* blocks.
2. **The bump**, one commit with the page: `PRIVACY_VERSION`, the page's Effective and version lines,
   `PRIVACY_BUMP_1` in `app/tests/grades.rs`, and the screen's "what changed" key.
3. **The cloud** (`account` function): `GET /account/consent` answers the recorded versions, and
   `POST /account/consent` records a **later** privacy version on an account that already consented.
   Today the second call is a silent no-op, so no existing account can ever accept a new version.
4. **The app**: two commands (`privacy_status`, `accept_privacy`), a per-profile cache file, a gate
   the scheduler reads, and the panel in `app/static`.
5. **Sign-out** (`account::sign_out_with`): tries the unfinished setups' deletes with the session it is
   about to forget, then clears `pending_setups`, whatever the deletes answered.

**Code read for this plan:** the main checkout (`main` before the email merge, `e920454` after L2's
merge) and, read-only, the email-forwarding worktree (`.claude\worktrees\email-forwarding`), which
merges first. Every `file:line` below is the email branch's ("EF") unless marked "main", so the agent
re-confirms each one on the lane branch before editing. Items are named by function, test or heading.

## 1. VISION and the two overriding rules

- **VISION commitment 2** ("the privacy page says so"): the page names the account copy, grades
  included, with "encrypted at rest, not end-to-end" kept (draft §1, §3.3).
- **VISION "Grades"** ("named on the privacy page, and never enter telemetry"): named; telemetry's entry
  is unchanged (draft §3.3, §3.8).
- **VISION "Knowlu reads"** (amended A8): the mailbox actions on the page are A8's list and no more
  (draft §1, §3.8). Microsoft 365's backfill is described as held, because it is (Q8 of the draft).
- **VISION "Telemetry"**: nothing from forwarded mail or the Gmail API, never grades (draft §3.8).
- **VISION "Data minimization"**: the screen before any model is described as a reduction (draft §1).
- **VISION standing rule "Failures are visible"**: a slot that cannot use the cloud before Accept names
  each step it leaves out (`sync (skipped: privacy update not accepted)` and the rest), at exit 0,
  never a run-and-fail. The panel says why.
- **VISION "Nothing the student sees depends on the network"** (commitment 2) and spec §5.1/D4: an
  unreachable service never raises the panel and never hides today's page; `rank` is never gated.
- **VISION Build order, the pilot** ("No one else signs up before the privacy page names … a lawyer has
  read it, the re-consent a second account needs is in the app"): L4 delivers the page's naming and the
  re-consent; the lawyer read stays before the Pilot (Amendment 2026-10-19). Whether friends count as
  "anyone else" is L8 §8.2's question, not this lane's (§9).
- **Rule 1 (no single-user assumptions).** The page names no person, school, tenant or vault ("at
  schools Knowlu supports" and "at every school Knowlu has a Blackboard listing for" are the only school
  references). The cache file and the gate are per profile and per account id. No test name, fixture or
  comment names a person, school, address or account. The live check uses a scratch profile.
- **Rule 2 (frozen references).** No task opens `engine/tests/fixtures/**` or a `surface-today-*.json`
  for writing. `rank`, `surface` and the read model are untouched. T16 checks
  `git diff --exit-code main -- engine/tests/fixtures`.
- **The contract list.** `app/src/account.rs` is on it and is L4's alone in this wave (handoff §5):
  every edit to it is `contract-engineer` at xhigh, read by `contract-reviewer`. `app/tests/account.rs`
  carries real Credential Manager tests under `CREDMAN_LOCK`, so contract-engineer writes its own failing
  tests there (test-writer would need its sign-off anyway). No other contract-list file is touched.
- **Money, releases.** The lane touches neither. The bump unblocks `scripts/ci/privacy-guard.ps1` for
  the next release, which stays L10's, on Quinn's go per tag.

## 2. Global constraints

- **Branch and worktree.** Branch `l4/honest-page`, worktree
  `C:\Users\danie\GitHub\knowlu\.claude\worktrees\l4-honest-page`, created by the controller from `main`
  **after the email-forwarding merge** (L2 has already merged, so `app/tests/grades.rs` is settled).
  Absolute paths only; never `cd`.
- **Ledger.** `.superpowers/sdd/2026-10-18-l4-honest-page/progress.md`, dated rows by printf.
- **Agents never commit, push or branch.** Each task ends with a file list and a commit message; the
  controller commits with `git add <paths>` (never `-A`) and `git commit -F <file>`, and merges the lane
  PR with a merge commit.
- **Rust gates.** `cargo build -p knowlu-engine` first (the zero-byte sidecar trap,
  `docs/reference/toolchain.md`), then `cargo build --workspace` and `cargo test --workspace`, dev profile,
  in the foreground, under the build-slot guard:
  `bash C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test --workspace --manifest-path C:/Users/danie/GitHub/knowlu/.claude/worktrees/l4-honest-page/Cargo.toml`
  (four builds machine-wide since ledger row 136; new builds wait while memory is low). A task that
  touches only `app` may run `-p knowlu --test <file>` for its red/green loop, but its hand-off gate is
  the workspace run. **0 warnings is part of green**; the one accepted line is the app's `.rsrc merge
  failure: multiple non-default manifests`. Also `scripts/ci/eol-check.ps1`.
- **Cloud gates.** The three commands `ci.yml`'s `cloud` job runs, with the worktree's absolute path
  prefixed (L3's plan §2 states them in full): `deno check --config cloud/supabase/deno.json
  cloud/supabase/functions/**/*.ts cloud/eval/*.ts`; `deno lint --config cloud/supabase/deno.json
  cloud/supabase/ cloud/eval/`; and `ci.yml`'s `deno test` line with its flags exactly as written there.
- **Page and console checks.** `scripts/wizard-check.py` and the settings check, run by the controller
  (they need the `.wv/` venv). Fixture screenshots by headless Edge (memory: console fixture screenshots)
  for the panel and the page, never synthetic OS input.
- **Tests never leave the machine.** Every new app test that needs a server binds `127.0.0.1:0` and
  serves itself (`app/tests/account.rs`'s existing pattern). Every Credential Manager test holds the
  file's `CREDMAN_LOCK`, uses a generated test id and cleans up with a `Drop` guard.
- **TDD.** Every behaviour is a failing test first, seen red for the stated reason. No existing assertion
  is weakened. A test this plan does not name as rewritten is never edited to pass: stop and report.
- **Held diffs (review r1, I3).** Two diffs cannot be committed before T13: T9's page pins (red until
  the page lands) and T10's page (it must land with the constant). A third waits for T8: T7's panel
  pins (red until the panel lands, and T8 waits behind S5 while T5 and T6 run). Each lives as a patch
  in the ledger folder, `t7-panel-pins.patch`, `t9-page-pins.patch` and `t10-page.patch`, never as a
  dirty worktree and never as a stash. The controller applies them (`git apply --check`, then
  `git apply`) only for the red run of T7 and T9, for T8 (T7's), and for T10, T11, T12's edits and T13
  (T9's and T10's); saves the patch again after every change; and reverses them (`git apply -R`)
  before any other task's gate and before every rebase, so the worktree is clean whenever another task
  runs. After a rebase it re-applies with `--check` first; a patch that no longer applies is a ledger
  row and goes back to its task's agent. So every committed tree is green, and T5, T6 and T8's gates
  are the whole workspace with no exclusions.
- **LF everywhere**; `*.ps1` stay CRLF (none is edited here). `engine/tests/fixtures/**` untouched.
- **`commands.rs` computes nothing.** The two new commands live in `account.rs`, beside the module they
  serve (CLAUDE.md), and `commands.rs` is not edited.
- **`app/src/main.rs` is L6's** until L6 merges (handoff §5; L6 plan §5.1 item 4: lanes that add a
  command do it after L6 merges, and L6 takes no foreign lines). L4's two handler names wait for that
  merge (T5). Recount both handler lists by script after the edit; never quote a count from this plan.
- **No secret, address, account id, school or person's name** in code, tests, comments, test names,
  the page or the ledger. The live check (T15) records counts and outcomes only.
- **The page text is the draft's.** The page task copies draft §3's wording, settles each *[bracketed
  instruction]* as §5 rules it, and invents no sentence. A sentence the draft does not have (L8's two,
  the panel's copy) comes from §5's rulings, in Quinn's words.

## 3. What exists today (EF, the email branch that merges first)

**The version and the page.**
- `pub const PRIVACY_VERSION: &str = "2026-09-24";` (`app/src/account.rs:25`), beside `TOS_VERSION`
  (`:24`), under the doc "Bump BOTH the constant and the page's date in the same commit" (`:21-23`).
- `app/tests/static_assets.rs::the_privacy_version_constant_is_the_published_pages_date` (`:1666`)
  pins the page's Effective and version lines to the constant.
- `app/tests/grades.rs`: `const PRIVACY_BUMP_1: Option<&str> = None;` (`:183`), read by the
  date-and-bump test (`:256`), which refuses any `policy_read` date while `PRIVACY_VERSION` is older
  than bump #1, and requires bump #1 later than `2026-09-24` (`:204-207`). L2 merged without touching it.
- `scripts/ci/privacy-guard.ps1` (D18): a release from a tree carrying any mail marker fails while
  `PRIVACY_VERSION` is on or before `2026-09-24`. `engine/tests/workflows.rs::the_privacy_guard_blocks_mail_code_at_or_before_the_pre_bump_version`
  asserts the verdict on this tree follows the rule (`:363-368`), so it stays green through the bump and
  flips the guard to "pass".
- The site is deployed **only by a release**: `release.yml`'s `publish` job runs
  `wrangler pages deploy site` (main `release.yml:278-282`). No workflow deploys `site/` alone. So the
  new page reaches `knowlu.com` at L10's tag, and `open_policy("privacy")` opens
  `https://knowlu.com/privacy.html` (`account.rs:620-631`), the live page.
- The wizard and the upgrade overlay both say "I accept the terms and the privacy policy" beside the
  18+ box (`app/static/index.html:122`, `:259`), so every sign-in is an acceptance of the build's
  `PRIVACY_VERSION`.

**How consent is recorded.**
- App: `record_consent_at(api_base, token, tos, privacy)` posts `{tos_version, privacy_version,
  age_attested: true}` to `POST /account/consent` (`account.rs:1135-1138`). It is called after both
  sign-in paths (`verify_email_code` `:566`, `google_sign_in` `:1209`) and retried before Checkout
  (`checkout_url_at` `:1168`), each with `TOS_VERSION` and `PRIVACY_VERSION`.
- Cloud: `recordConsent` (`cloud/supabase/functions/account/handler.ts:152-184`) requires
  `age_attested === true` and both versions, bounds them at 64 characters, then **returns a silent 200
  if the account has any `tos` consent row** (`hasConsent`, `:173`; `index.ts:171-178`). Otherwise it
  PATCHes `accounts` (`tos_version`, `tos_accepted_at`, `privacy_version`, `age_attested_at`) under the
  filter `age_attested_at=is.null` and inserts three `consents` rows (`tos`, `privacy`, `age_18`)
  (`index.ts:179-212`).
- **The gap.** An account that consented at version A can never record version B: every later call is
  the silent 200. No route returns the recorded version; only `GET /account/export` carries the whole
  `accounts` row.

**Sign-out and `pending_setups`.**
- `sign_out(app, profile_id)` (`account.rs:644-657`) wraps `sign_out_with` (`:668-696`), which reads
  the session's token, deletes the credential, then (for a profile id) closes the mail window, takes
  its hold and calls `mail_session::forget_all(dir)` (every `mail-session-*` and `mail-setup` profile),
  then revokes the token at GoTrue, best effort. It returns `{ok, had_session, mail_profiles_left}`.
- `MailState.pending_setups: Vec<PendingSetup>` lives in the profile's `mail.json`
  (`app/src/mail_session.rs:140-162`): each in-flight setup's `address_id`, `placed`, `backfilled`,
  `shown`. At start-up `mail::setup::clean_unfinished(data_dir, &client)` (`app/src/mail/setup.rs:392-426`)
  deletes each never-placed `pending` address on the service, forgets its profile, and clears the record;
  a list or delete that fails keeps the record for the next start.
- **The hand-back** (email plan, the review-of-`a7f1ca0` row): "sign-out's `forget_all` (`account.rs`)
  does not clear `pending_setups` after trying their deletes, so a different account signed in on the
  same profile could inherit them; its start-up clean then lists that account's addresses, does not find
  the old ids, and only removes their profiles. A contract-engineer step."
- `delete_local_data` removes the whole data folder (`account.rs:1274-1284`), so deletion needs nothing.

**Two stale comments L6 hands to L4** (L6 plan §9): `account.rs:767` cites `onboarding::offer_marker`
as a precedent (L6 deletes it), and `:818` says a vault with no cloud config "falls through to the local
runtime/model gate" (L6 makes that `judge (skipped: no account)`).

## 4. The behaviour this lane adds

### 4.1 The page

The draft's §3, every block marked **[now]**, **[#19]** and **[L5]** (the last as Q2 of the draft rules),
and none marked *[after C3]* (R12b's, after L4 merges). Plus, as §5 Q-L8 rules, L8's two sentences
(friends-testers spec Q7) in its own words. The summary sentence, "That is the whole policy in one
line…" and every sentence the existing pins hold stay word for word (draft §0, last paragraph). No
`<script>`. A table style in `site/site.css` only if none exists. The "Last changed" line and the
narrow change-rule sentence follow draft Q3 (ii) unless Quinn rules (i).

### 4.2 The bump (one commit, the lane's last code commit)

`V` is the date the bump commit is made (§5 Q-V). The commit moves, together and nowhere else:
the page's Effective line, its version line and (Q3) its "Last changed" line to `V`;
`PRIVACY_VERSION = "V"`; `PRIVACY_BUMP_1 = Some("V")`; and the panel's "what changed" key to `V`
(§4.6). The page's content change lands in the same commit (`account.rs:21-23`'s rule), so on the lane
branch the page task's diff, and T9's pins that read it, are held as patches (§2, "Held diffs") until
the bump task adds the four tokens and commits all of it at once (§7, T9-T13).

### 4.3 The cloud: reading and recording a later version

`cloud/supabase/functions/account/`:
- **`GET /account/consent`** (new, `requireUser`, no entitlement needed): `200 {"tos_version": <text or
  null>, "privacy_version": <text or null>}`, read from the caller's `accounts` row. A caller with no
  `accounts` row: 404 `no such account` (as `POST` answers today). Nothing else is returned.
- **`POST /account/consent`** keeps its validation exactly (age attestation, both versions present, the
  64-character bound). Then:
  - no `tos` consent row: today's first-consent path, unchanged byte for byte;
  - a `tos` row exists and the body's `privacy_version` sorts **after** the recorded
    `accounts.privacy_version` (ISO dates of equal length compare as strings; a recorded null counts as
    older): PATCH `accounts.privacy_version` under a filter that only moves it forward
    (`privacy_version=lt.<v>` or `is.null`), insert **one** `consents` row (`kind: privacy`, the
    address's hash, the version), and nothing else: no `tos` or `age_18` row, no `tos_version`,
    `tos_accepted_at` or `age_attested_at` restamp;
  - a `tos` row exists and the version is equal or older: the silent 200 of today.
  - A version string that is not `YYYY-MM-DD` on the second path: 400 (the forward-only filter compares
    strings, so a non-date would sort anywhere).
- **The purge list** (`index.ts`'s `purge`, "what the privacy policy's deletion paragraph is written
  from"): gains `usage_daily` and `judgments`, one line each, as L8 §8.6 recommends (T1 first confirms
  each table's foreign key; if either does not cascade from `accounts`, that is a deletion bug the line
  fixes, recorded as a finding). The export is **not** edited here: L8's S4 adds `usage_daily`,
  `route_daily` and `comp_grants` (§6, `account/index.ts`).

The concurrency note on the existing insert (`index.ts:196-203`) applies unchanged: two racing Accepts
can write two identical `privacy` rows, which is harmless; the forward-only filter keeps the column right.

### 4.4 The app: status, accept, cache, gate (`app/src/account.rs`)

- **`consent_status_at(api_base, token) -> Result<Option<String>, String>`**: `GET /account/consent`,
  the recorded privacy version (`None` for null). `check_api_base` first; a non-2xx is the provider's
  error; the reply body is bounded like `post_authed`'s.
- **The cache**, `privacy_cache_path(data_dir) = data_dir.join("privacy.json")`, beside
  `entitlement.json` and written the same atomic way: `{account_id, recorded, checked_at}`. Written only
  from a reply the service gave. Never synced, never in the vault, never in telemetry. None of the three
  keys is a new data class (review r1, M4): `account_id` is already in `config/cloud.yaml` and
  `entitlement.json`, `recorded` is a version date the page publishes, and `checked_at` is the time of
  the read.
- **The gate**, a pure function of the cache, the vault's `account_id` and `PRIVACY_VERSION`:
  `Accepted` (recorded ≥ current), `Older` (the service said older, or null), `Unknown` (no cache, or
  a cache for a different account id). Recommended rule (§5 Q-GATE): only `Older` holds anything back.
- **`#[tauri::command(async)] privacy_status(cs)`**: for a vault with an account, refreshes the session
  as `account_status`'s siblings do (the existing `valid_access_token_at` under `SESSION_REFRESH_LOCK`),
  calls `consent_status_at`, writes the cache, and answers `{ok, current, recorded, needs_accept}`,
  where `needs_accept` is `true` only when the service answered older. Unreachable or refused:
  `{ok: false, error, needs_accept: null}` and the cache is left as it was. No account: `{ok: true,
  needs_accept: false}` (the upgrade overlay owns that case).
- **`#[tauri::command(async)] accept_privacy(cs)`**: `record_consent_at(base, token, TOS_VERSION,
  PRIVACY_VERSION)`, the same body every sign-in and the Checkout retry already send (§5 Q-ACCEPT), then
  re-reads with `consent_status_at` and writes the cache from **that** reply, never from the request. It
  answers `{ok, recorded}`; `ok` is true only when the re-read shows `recorded >= PRIVACY_VERSION`.
- **`refresh_privacy(vault, data_dir) -> Result<Gate, String>`**, the scheduler's one call: the same
  read and cache write as `privacy_status`, without a Tauri state. It reads the session before any
  request, at the target the vault's `config/cloud.yaml` names (`session_credential_target`, as
  `refresh_entitlement` reads it, EF `account.rs:865-868`): with no session there it answers
  `Err("no session")` and sends nothing, and any `Err` leaves the cache as it was (review r1, I1; T3
  tests both).
- After a successful sign-in (`verify_email_code`, `google_sign_in`) nothing new is written: there is no
  profile yet, and the slot's first refresh fills the cache (§4.5).

### 4.5 The scheduler: a named skip until Accept (`app/src/scheduler.rs`)

In `run_slot_with`, right after `let est = entitlement_state(cs)` (EF `scheduler.rs:955`) and before
the grades step. **The block runs only when `est` is `Entitled`** (review r1, M3): `NotEntitled`,
`NoAccount` and `Unreadable` leave the slot exactly as it is today, with no privacy row. Why the
entitlement's name wins: a step the entitlement already stops would not run whatever the student
accepts, so naming privacy would point at the wrong fix; past the grace the engine refuses `sync`,
`coursework` and `ingest` itself (CLAUDE.md), so nothing new runs either way; and the panel still shows
at launch, so the student still learns of the update.

For an `Entitled` vault:
- **The check.** When the cache's gate is not `Accepted`, call `account::refresh_privacy` once
  (`steps.start("privacy")`). **Success adds no row** (`steps.idle()`): it is routine while the gate is
  `Unknown`, and the rows below show what it found. **A failure** is the summary row `privacy (check
  failed: <reason>)` at exit 0; the runner log gets the bare `privacy (check failed)`, sanitised exactly
  as the entitlement refusal is (EF `scheduler.rs:1024-1025`). A failure leaves the cache, and so the
  gate, as it was. An `Accepted` cache makes no call and adds no row.
- **Then, only when the gate is `Older`**, a step that would have run is not run and is named
  `<step> (skipped: privacy update not accepted)`; **a step that would not have run anyway keeps
  today's name** (review r1, M2):
  - grades: `grades_step`'s own earlier skips (not a Blackboard school, not available yet, not
    connected, no window on this run, sign-in window open) keep their names; only a capture that would
    have run becomes the privacy skip, and the capture seam is never called. **The mechanism** (review
    r2, M-3): `grades_step` names a capture's failure only through `CaptureError::as_str` (EF
    `scheduler.rs:455-459`), and a `SignedOut` also calls `grades::set_signed_out(data_dir, true)`
    (`:456-457`). So when the seam has a capture, `run_slot_with` hands `grades_step` a stand-in
    capture instead: a closure that records "would have captured" in a local flag and returns
    `Err(CaptureError::Busy)`, never `SignedOut`, with no other side effect. If the flag is set,
    `run_slot_with` replaces the row `grades_step` returned with `grades (skipped: privacy update not
    accepted)`; if not, `grades_step` stopped earlier and its own row stands. A seam with no capture
    passes `None` as today, so `no window on this run` keeps its name;
  - mail: no mailbox connected adds no row, as today (EF `scheduler.rs:972-974`); `mail_step`'s own
    skips before the pass (could not read your mailboxes, mail.json unreadable, saved sign-in off, not
    due, mail window open) keep their names; only a pass that would have run becomes `mail (skipped:
    privacy update not accepted)`, and the pass seam is never called. One way that needs no edit to
    `mail_step` is to call it with a `MailSeam` whose `pass` answers `Err("privacy update not
    accepted")`; the mailbox list it reads is the existing read-only `GET /addresses`;
  - judge: a `JudgePlan::Run` from `judge_plan_for` becomes `Skip("judge (skipped: privacy update not
    accepted)")`; a `Skip` it already gave keeps its name;
  - the engine steps: the argv `slot_argv` returns is filtered in `run_slot_with`: its `sync`,
    `coursework` and `ingest` entries are dropped (by `args[0]`), and each dropped entry becomes a row
    `<step> (skipped: privacy update not accepted)` at exit 0, pushed after judge's row and before the
    runner-log write, in `slot_argv`'s order. An entry `slot_argv` did not emit (`ingest` when not
    included) gets no row.
  `grades_step`, `mail_step`, `slot_argv` and `judge_plan_for` are **not** edited (L6 rewrites the last
  two; §6). If the rule above cannot be met without editing one of them, T6 stops and reports.
- `rank` and the backup run as always; telemetry runs as always (§5 Q-SKIP).
- The runner-log filter gains the `sync (skipped:`, `coursework (skipped:` and `privacy (check failed`
  prefixes, so every privacy skip except mail's (whose rows stay out of the vault, as today) reaches
  `state/runner-log.md` as `ok`.
`Accepted` and `Unknown` add no skip and leave the argv byte for byte as today; `Unknown` adds only the
check's row if the check fails.

### 4.6 The panel (`app/static`)

- **When it shows.** `bootConsole` → `checkAccount` → for a signed-in vault (`!s.needs_account`), one
  `privacy_status` call per launch. `needs_accept === true` raises `#reconsent`. `false`, `null`, a
  failed call or an unreachable reply leave it down for the session (D4; the scheduler's cached `Older`,
  if any, still holds the cloud steps). The upgrade overlay and the panel never show together: the panel
  needs an account, the overlay needs none.
- **What it is.** A `.setpanel` beside `#upgrade` (today's page stays usable underneath, as
  `#upgrade`'s comment explains), with: a heading; three or four lines of what changed, from a
  `PRIVACY_CHANGES` object in `console.js` keyed by the version it describes; a policy link
  (`<a href="privacy.html" class="policy" data-policy="privacy">`) handled through `open_policy`;
  **Accept**, which calls `accept_privacy` and, on `ok`, hides the panel and calls `poll()`; a
  **Settings** button (`id="rc-settings"`) that calls the existing `openSettings()` (EF
  `console.js:2482`); and an error line for a refusal. **No *Not now*, no Close, no Escape** (§5 Q4 of
  the draft): the way out is Accept, or Settings → Delete my data, which the panel names and its own
  button opens. Why a button inside the panel (review r1, I4): `#reconsent` is a fixed `.setpanel` at
  `z-index: 20` (EF `console.css:471`) with no dismiss, and the gear (EF `console.js:88`) is the only
  other way into Settings; nothing guarantees the panel never covers the gear at every window size, so
  the panel carries its own way out.
- **Settings, while it is owed.** Settings still opens (Delete my data must stay reachable, draft Q4),
  and the panel stands down while Settings is open and returns when it closes, as `maybeUpgrade` does
  for `#upgrade` (R-C1-57: two `.setpanel`s never stack). Inside Settings, every control that starts a
  use this version newly names is disabled with one line, "Accept the privacy update to use this": the
  Google row's Connect, Email's set-up, Resume and Reconnect, and Grades' Connect and Refresh (T8 lists
  the exact ids it found). Disconnect, Turn off, Forget and Delete my data stay live: stopping a use
  never waits for consent. Quick capture, the deck and every local edit work as usual.
- **Copy.** Quinn words it (§5 Q-COPY). The draft lines this plan proposes, for Quinn to rewrite:
  "Knowlu's privacy policy changed on V." / "It now covers the email you forward to Knowlu and what is
  kept from it, the Blackboard and mail sign-ins Knowlu keeps on this computer, your grades in your
  account, and daily counts of your AI calls." / "Until you accept, Knowlu keeps today's page up to date
  from this computer but does not sync, import or read anything new." / "To leave instead, use Settings
  → Delete my data." (the button below it reads "Settings").
- **One key per bump.** `PRIVACY_CHANGES` carries the version its lines describe; a static pin (T7)
  fails unless that version equals `account.rs`'s `PRIVACY_VERSION`, so the Pilot's lawyer-read bump
  cannot ship the friends' lines by accident.

### 4.7 Sign-out clears `pending_setups` (`account::sign_out_with`)

Two facts fix the order (review r1, I2): `clean_unfinished` deletes `mail-session-*` profiles through
`forget` (EF `mail/setup.rs:419`), so it must run under the mail window's hold; and the clean's client
reads the session from Credential Manager, once, at the clean's first request (through
`valid_access_token_within`, below; review r3, m-4), so it must run before the credential is deleted.
Today the hold is taken
after the delete (EF `account.rs:679-683`). For `Some(profile_id)` with a profile folder, the new
order is:
1. close the mail window and take its hold (`close_mail_window(&dir)`, the existing `FnOnce`, called
   once, here, as the first step). **It may give no hold** (review r2, M-6): `sign_out`'s closure
   answers `None` when the open console is another profile's (EF `account.rs:650-652`), and
   `take_mail_window` answers `None` when `MAIL_STOP_BOUND` passes with the window still held
   (`:1518-1519`). **Steps 2-5 still run** without a hold, as today's `forget_all` already does
   (`:683-685`): sign-out never waits past the bound. A mail profile WebView2 still holds fails its
   delete and is reported in `mail_profiles_left`, as today;
2. run the clean, through a new seam parameter of `sign_out_with` (a closure taking the profile
   folder, as `close_mail_window` is one, so tests drive it with no network);
3. **clear every `pending_setups` record** through `MailState::update`, whatever the clean answered
   (placed, shown and failed-delete records included: they name addresses of the account being
   signed out);
4. `mail_session::forget_all(&dir)`, as today;
5. drop the hold;
6. read the session's access token (moved here from EF `:677`, so a refresh the clean made is the
   token revoked), then delete the credential;
7. revoke the token at GoTrue, best effort, as today.
`None` and a non-profile id skip steps 1-5 and keep today's order (read, delete, revoke).

**The clean's client (review r2, I-1; Quinn's decision of 2026-10-19).** `sign_out` builds it with a
new `pub fn sign_out_mail_client(api_base, anon, target, per_call) -> Result<MailClient, MailError>`,
called as `sign_out_mail_client(&api_base(), &anon_key(), &session_target(id), SIGN_OUT_MAIL_CALL)`,
and passes `clean_unfinished(dir, &client)` as the seam. `SIGN_OUT_MAIL_CALL` is a new constant of at
most 5 seconds. The builder takes its base as a parameter so a test can point the clean at its own
listener while `api_base()` points elsewhere (review r2, I-2). It is `MailClient::with_token_fn(api_base,
f).with_timeout(per_call)`; both are `pub` (EF `mail_client.rs:179`, `:193`), and `mail_client.rs` is
not edited. The token function `f` reads the session **once per clean**, lazily at its first request,
through a second new function, `valid_access_token_within(..., within = per_call)`, and gives every
later request of that clean the same answer, an `Err` included. **The memo is a
`std::sync::OnceLock<Result<String, String>>`** captured by `f` (review r3, m-3): `TokenFn` is
`Fn + Send + Sync` (EF `mail_client.rs:151`), so a `Cell` or `RefCell` will not compile, and with no
memo each request would read again, making the clean 10 × (1 + n) s under lock contention.
`sign_out_mail_client`'s doc comment gives the reason it keeps a token at all, against
`mail_client.rs`'s module doc ("read on every call … never kept", `:7-9`): the token is kept for one
clean only, a few seconds at sign-out, so the bound counts one read; it is dropped with the client.
- **Why not `for_session`** (round 1's choice): its token function is `valid_access_token_at`
  (`mail_client.rs:186-190`), called on every request. That function waits for `SESSION_REFRESH_LOCK`
  without limit (`account.rs:497`) and refreshes over the account agent's 30 s (`:40`, `:159-161`,
  `:500`), so the clean had no bound.
- **Why not a token read at sign-out's start:** round 1's reason stands. A console left open past the
  hour holds an expired token, and every clean would fail with a 401. The helper refreshes it, and the
  credential still exists at step 2.
- **Why `api_base()`:** not the profile's vault and `client_for`, because sign-out already revokes
  against `api_base()` and needs no vault path.
- A client that cannot be built skips the clean, never the sign-out.

**`valid_access_token_within(auth_base, anon, target, now_unix, within) -> Result<String, String>`**,
beside `valid_access_token_at`, contract-engineer's:
- it takes `SESSION_REFRESH_LOCK` by `try_lock`, retried every 25 ms or less until `within` has passed
  since the call began. `Err(TryLockError::Poisoned(p))` **means the lock was acquired**: the guard
  is taken with `p.into_inner()`, as the two existing takers do (`:497`, `:519`); only
  `TryLockError::WouldBlock` means another caller holds it (review r3, m-1). Read the other way, every
  sign-out after a panic under the lock would time out. Not taken in time is an `Err` with a fixed
  sentence, and no request is made;
- under the lock it applies `valid_access_token_at`'s rule: `load_session`, and a token with more than
  120 s left is answered with no request;
- otherwise it sends one `POST {auth}/token?grant_type=refresh_token` with the body `refresh_at` sends,
  over an agent whose `timeout_global` is what is left of `within`. If nothing is left, it answers
  `Err` and sends nothing. A fresh session is saved exactly as `valid_access_token_at` saves it
  (`:500-503`);
- one call therefore takes at most `within`, plus local Credential Manager reads and one write. Its
  errors use the file's existing forms, and no token or reply body reaches an error string.

**It leaves `valid_access_token_at`'s contract unchanged for its other callers**, which was the
condition on Quinn's decision. Checked on EF:
- `valid_access_token_at`, `ensure_session_for_at`, `refresh_at`, `post_json` and `agent()` keep their
  behaviour and their bytes on the wire. A private `post_json` variant that takes the timeout may be
  factored out, with `post_json` passing `TIMEOUT`. The two existing takers keep their blocking `lock()`.
- The new function shares only the lock and the session entry, and applies the same 120 s floor and
  saves the same entry the same way. **Each new hold is at most 5 s plus local Credential Manager
  work, once per sign-out** (review r3, m-1). std's `Mutex` on Windows is an unfair SRWLock, so the
  helper's `try_lock` can win ahead of a caller already blocked in `.lock()`; that waiter may then wait
  up to about 5 s more, once per sign-out, and never a refresh's 30 s more. No behaviour or bytes
  change for any other caller.
- A capped refresh that times out after GoTrue has rotated the refresh token leaves the old token
  stored. `valid_access_token_at` already has that hazard at its 30 s timeout. Here the window is
  short: the clean is step 2, steps 3-5 are local, and step 6 reads and then deletes the credential.
  Another thread may still reuse the old refresh token inside that window; GoTrue then either answers
  within its reuse interval or revokes the session family. Either way the student is signing out, so
  neither outcome costs anything (review r3, m-1).
- `sign_out_mail_client` is its only caller, and T4 pins that. So the documented-bound alternative is
  not needed.

**The bound.** `clean_unfinished` returns at once when there is no record (EF `mail/setup.rs:401-404`).
Otherwise it makes one list and at most one delete per record that the service lists as `pending` and
that was never placed (`:405-417`). A failed list ends it (`:405-407`). For a clean that makes `n`
deletes (at most three, below), sign-out's worst case is:
- `MAIL_STOP_BOUND`, 60 s, for the hold (step 1);
- plus `SIGN_OUT_MAIL_CALL` for the clean's one session read (lock wait and refresh together);
- plus `SIGN_OUT_MAIL_CALL` × (1 + n) for the list and the deletes;
- plus the account agent's `TIMEOUT`, 30 s, for the revoke (step 7, unchanged).

That is **60 + 5 × (2 + n) + 30 seconds** of network waits: 100 s with one delete. **One local term
adds to it** (review r3, m-2): deleting a mail profile WebView2 still holds goes through
`mail_session`'s `wipe`, 12 tries with 11 waits of 250 ms (EF `mail_session.rs:31-32`, `:301-311`),
so up to about 2.75 s per held profile, in the clean's `forget` and again in `forget_all`. The rest is
local file and Credential Manager work with no wait. `SIGN_OUT_MAIL_CALL`'s doc line states the sum
and the `wipe` term. `None` and a non-profile id keep today's single 30 s wait, the revoke.

**Deletes are at most three, whatever the record count** (review r4, nit). A delete goes only to a
row the service lists as `pending` (EF `mail/setup.rs:415`); a `pending` row is a live address, and D3
caps live addresses at three. So the clean makes at most 1 + 3 requests, and the network worst case
is at most 60 + 5 × (2 + 3) + 30 = 115 s, however many records `mail.json` holds. The session read
once per clean has at least 120 s left, or an hour after a refresh, and those four requests take at
most 20 s after it, so no delete meets an expired token. Rules:
- a clean that fails (no network, a refused list or delete, an unreadable `mail.json`) never stops the
  sign-out and never keeps a record; the reply gains `"setups_cleared": <bool>` (true when `mail.json`
  holds no record afterwards);
- `None` (the pending session) and a non-profile-id touch no `mail.json` (as today);
- every other `mail.json` field is untouched (`turned_off`, `open_backfills` and the rest), and the
  write goes through `MailState::update`, the one write path;
- the known cost, recorded as accepted in the ledger: an address whose delete failed stays `pending` on
  the service under the signed-out account (it holds that mailbox's hint and one of D3's three live
  addresses for that account) until that account's next setup or a Settings Turn off.

### 4.8 The two comments from L6

`account.rs:767`'s precedent becomes `onboarding::mail_offer_marker` (it exists once the email branch
merges, so the sentence is true before and after L6). `:818` is reworded to "the named `judge (skipped:
no account)`" **only if L6 has merged** when T4 runs; otherwise it stays, and the ledger hands it to the
next contract-list change, as L6's plan §9 says.

## 5. Open questions

Each has a recommendation, and the plan is written to it, so a "yes" changes nothing. **(Quinn)** marks
Quinn's; the rest are the controller's at dispatch. Quinn hears them one at a time, recommendation first.

**From the draft (its §5), carried here unchanged; the page task follows each recommendation unless
Quinn rules otherwise:** Q1 the summary sentence stays word for word **(Quinn)**; Q2 the syllabus lines
carry "(Knowlu does not read syllabi in this version; the app will say when it does.)", and removing the
parenthesis later is material, (a) and (b) both **(Quinn)**; Q3 (ii), a "Last changed" line apart from
the version, with the narrow change-rule sentence, consistent with the standing C3 Q3 ruling **(Quinn)**;
Q4 the panel has no dismiss, Settings keeps Delete my data and disables what would start a new use
(§4.6), and (b) the server-side gap for forwarded mail is accepted
for founder accounts, with the tripwire **(Quinn)**; Q5 testers accept `V` at sign-up and never see the
panel **(Quinn, confirm)**; Q6 disclose "who it was from" **(Quinn)**; Q7 "moves to your Trash"
**(Quinn)**; Q8 Microsoft 365 described as held **(Quinn)**; Q9 "up to about four months" **(Quinn)**.
The draft's Q10 is R12b's (C3's), not this lane's.

- **Q-L8. L8's two sentences, and their order against L8's merge.** **The order is ruled (Quinn,
  2026-10-19, option (a) of review r1's C1):** L8's S5 moves ahead of L4's T8 in the `console.js`
  queue (Q-QUEUE), S5 waits for no L4 task, and so L8 can merge before T13. **The bump commit (T13)
  waits for L8's merge**, so both sentences are true on the day the page lands. Round 1 found the old
  order deadlocked: T13 waited for L8, L8 could not merge without S5, and the friends-testers spec held
  S5 until L4 had merged. The spec's §8.1 and S5 row now carry the new order, under a dated note citing
  this ruling.
  - Sentence 1 says the daily counts "are in your export and are deleted with your account": the
    export half becomes true only with L8's S4 (`usage_daily`, `route_daily` in `GET /account/export`),
    which merges with L8.
  - **Still open (Quinn), L8's Q5, Q7 and Q8**, unless the controller finds a ruling row: the
    friends-testers spec is signed (ledger row 135), but no row rules them one by one, and its S8 runs
    "as soon as Quinn rules Q5, Q7 and Q8, and in any case before L4's bump". *Recommend:* as the spec
    recommends (both sentences verbatim; the audit dies with the account, so the page's "the one record
    a deletion does not take with it" stays), and the page carries both.
  - **Still open (Quinn), the fallback.** If L8 has not merged when T13 is otherwise ready (it can be
    held by S7a's wait for L3, M7, or by Quinn's go at S6, which is money), the controller asks Quinn
    once: wait, or ship L4 with sentence 2 only (true while no grant exists) and take L8 §8.5's own
    fallback (no prod grant and `ROUTE_COUNTS` off until a later bump carries sentence 1). *Recommend:*
    wait; the page goes live only at L10's release, so a few days' wait costs no student anything. If
    the fallback is taken, L8 rebases over L4's merge instead, which S5's new place allows.
  - Cost of the ruling: L4's merge rides L8's, and every lane behind L4 in the queue with it, unless
    the controller lets them overtake L4 (Q-QUEUE).
  - Sentence 1's "cloud requests" half is disclosed ahead of collection on prod, by L8's design (the
    `ROUTE_COUNTS` secret waits for this page); that is the safe direction and needs no third sentence.
- **Q-GATE (Quinn). What holds the cloud steps back.** *Recommend:* only a definite answer from the
  service that the recorded version is older (`Older`), cached so it holds offline too. A cache that
  says nothing yet, a check that fails, or a cache for another account (`Unknown`) leaves the slot as it
  is today. Why: every new account records the build's version at sign-up, so for a real account
  "unknown" nearly always means "accepted, not yet read"; failing closed on it would turn any outage of
  the `account` function (or a prod deploy that lags the release, §6) into a slot with no sync, no
  import and no judgment for every student. Cost: an account whose first read fails keeps running on its
  old consent until a read succeeds (the next slot or launch). Alternative: fail closed on `Unknown`
  too (stronger against the page's "carrying on is not how we take consent"; brittle).
- **Q-SKIP (Quinn). Which steps the skip covers.** *Recommend:* the draft's list, `sync`, `coursework`,
  `ingest`, `grades`, `judge` and `mail`; `rank`, the backup and telemetry run (telemetry's use is the
  terms' and is unchanged by this version). Alternative (narrower, truer to "new uses only"): skip only
  `grades`, `judge` and `mail`, the steps whose uses this version newly names. Cost of the
  recommendation: an un-accepted founder account stops syncing between desktops until Accept. Two
  naming rules ride this answer (review r1, M2 and M3; §4.5): a step that would not have run anyway
  keeps today's name, so a profile with no mailbox shows no mail row; and an account the entitlement
  stops shows only the entitlement's names, never privacy's.
- **Q-COPY (Quinn).** The panel's four lines (§4.6) are a proposal; Quinn words them at the page-read
  checkpoint, with the page. console-ui writes the draft lines until then, and T13 replaces them with
  Quinn's words if they differ (one text edit in the bump commit, pinned by T7's static test only for
  its version key, never its wording).
- **Q-LIVE (Quinn). When the page is live, and what reads it.** The page reaches `knowlu.com` only at
  L10's release (§3). Consequences: (1) Gmail spec §6's ordering (the page live no later than prod's
  `GOOGLE_CLIENT_ID`) means L9 sets that secret **after** L10's publish job, or the controller runs a
  site-only Pages deploy first; (2) L8's S7 (prod `ROUTE_COUNTS=on`, the first grants) waits for the
  release as well; (3) on a dev build of the merged `main`, the panel's link opens the **old** live page.
  *Recommend:* keep the page in the release (no new deploy path, no new secret use), order L9's Google
  secret after L10's publish, and have Quinn accept on staging scratch accounts only until then (T15
  uses a scratch account). Alternative: a one-off `wrangler pages deploy site` by the controller on
  Quinn's go, which puts V live before any build can record it.
- **Q-AGENT (Quinn).** `.claude/agents/reviewer.md:26` and `cloud-engineer.md:26` say "`PRIVACY_VERSION`
  moves only in a release PR". L4 moves it in a lane PR (Amendment 2026-10-19; handoff ruling 7 lets a
  lane PR merge on PASS). The reviewer will flag it. *Recommend:* the reviewer reads the Amendment as
  the ruling for this bump, and the guard's real release gate is `privacy-guard.ps1` plus L10's tag;
  Quinn decides whether to reword the two agent lines ("moves only in the PR that carries its page,
  and never ships before a release deploys that page"). No agent edits `.claude/`. Two facts the
  reviewer will need (review r1, M1): the Amendment is uncommitted in the main checkout today, so T0
  stops until it is committed on `main`; and its text never mentions a lane PR, so Quinn's answer here
  is the ruling for that, recorded as a ledger row that T16's check 6 cites.
- **Q-V (controller). The version's value.** *Recommend:* `V` is the date T13's commit is made, and is
  never re-stamped once committed: the page says Effective `V`, and a merge a day or two later changes
  nothing a reader can act on. If T13's commit is rebased onto a later day before the merge, it keeps
  its `V`. `PRIVACY_BUMP_1`'s test needs only `V > 2026-09-24`.
- **Q-ACCEPT (controller). Accept re-sends the sign-in's attestation.** *Recommend:* `accept_privacy`
  calls the existing `record_consent_at` with `age_attested: true`, as `checkout_url_at` already retries
  it before Checkout: every signed-in device ticked the 18+ box and the policy box at its sign-in
  (`index.html:122`, `:259`), and the route writes no `age_18` row on the second path (§4.3). This also
  makes Accept the retry for an account whose sign-up consent never landed (a comped friend never
  reaches Checkout's retry). Alternative: a privacy-only sub-route (`POST /account/consent/privacy`)
  that carries no attestation; cleaner on paper, one more route and test set.
- **Q-QUEUE (controller, within Quinn's ruling). The `console.js` queue.** **L6 T4 → L8 S5 → L4 T8
  → L7 → #19 → L1 R11 → L5**, with `index.html` and `console.css` in the same queue (L8 §8.1). The
  place of S5 is Quinn's (2026-10-19, Q-L8); the rest was already agreed: the handoff's L4 → L7 →
  L1 R11 → L5, #19 after L7 (close-out plan), L6 first (L6 Q7). L6's T4 only deletes, so the lanes
  behind rebase over it trivially. S5 starts after L6's merge and waits for no L4 task. What the slots
  mean:
  - **T8 starts once L6 has merged and S5 is committed on L8's branch** (its diff fixed). console-ui
    reads S5's diff read-only (`git show`) and keeps clear of its lines (`renderAccountRow`, the
    wizard's subscribe step, `afterUpgradeSignIn`). L4's branch takes S5 when it rebases onto `main`
    after L8's merge, before T13 (T13's "Before starting"); a conflict there goes back to console-ui,
    which keeps S5's lines and re-fits the panel's. *Why not wait for L8's merge before T8:* T13 waits
    for that merge anyway, so making T8 wait too only idles L4's panel behind L8's S7a and Quinn's money
    go, and moves T8, T11 and T12 onto the critical path after it. If S5 is not committed when T8 is
    otherwise ready, the controller may start T8 anyway, recorded in the ledger; the rebase before T13
    is the same.
  - L4's slot is T8 alone: T13's later `console.js` edit touches only the panel's own constant and
    copy, so it does not hold the queue.
  - **The lanes behind L4 wait for L4's merge, not T8's commit** (they branch from `main`). L4's merge
    now waits for L8's (Q-L8), so if T13 is held there, the controller may let L7, then #19 and L1 R11,
    take their slots ahead of L4, each merging on its own gates; L4 rebases over each before T13. L5's
    student-facing tasks still wait for L4's merge (session ledger row 121, Q42).
- **Q-PURGE (controller).** *Recommend:* L4 adds `usage_daily` and `judgments` to the purge list in T2
  (L8 §8.6), and L8's S4 keeps the export lines. Whichever of T2 and S4 lands second rebases the other's
  lines in `account/index.ts`.
- **Q-SCHED (controller). If the scheduler edit collides.** `scheduler.rs` is edited by L6 (the judge
  region), by C3's R9c and by L7 perhaps. *Recommend:* T6 starts after L6 merges and rebases over
  whichever of C3 and L7 landed; its edit is confined to `run_slot_with` (one block after the
  entitlement block, the argv filter, the runner-log prefixes). If T6 cannot land before T13 is ready,
  the controller may merge L4 without T6 **only on Quinn's word**: the panel alone then stands for the
  skip, and only founder accounts can be on an older version (draft Q5), so no student runs on old
  consent meanwhile.

## 6. Conflict map

**Order.** L0 (the email branch) merges first; L4's branch is cut from that `main` (L2 is already
merged). Inside the wave: L4's cloud and `account.rs` work (T2-T4) needs nobody; T5 (`main.rs`) and T6
(`scheduler.rs`) wait for L6's merge; T8 waits for its `console.js` slot, behind L6's merge and L8's
S5 commit (Q-QUEUE); T13 waits for L8's merge, which carries S5 and S4 (Q-L8, Quinn's ruling of
2026-10-19), and for Quinn's page read. No wait runs the other way: L8 waits for no L4 task (its S5
waits only for L6, and its S8 hands L4 two sentences before T13), so the order has no cycle. After L4
merges: C3's R12b may edit the page (row 105), L5's
student-facing tasks may merge (Q42), #19's release condition can be met (row 39 item 8), and L8's S7
and L9's Google secret wait for L10's publish (Q-LIVE). **No migration**: `accounts.privacy_version`
and `consents.kind = 'privacy'` exist, so the shared migration-stamp rule (M1-M8) does not reach L4.

| File | L4 task | Other lanes | Rule |
|---|---|---|---|
| `app/src/account.rs` | T3, T4, T13 | none in this wave (handoff §5: L4 only); L6 hands it two comments | contract-engineer only; contract-reviewer reads every diff |
| `app/tests/account.rs` | T3, T4 | C3 changes `app/tests/support/slot_rig.rs`, which this file includes by `#[path]`, add-only, and never this file (C3 plan §5) | T3/T4 add tests and edit none; if L4 must touch the rig, whichever of L4 and C3 lands second rebases |
| `app/src/main.rs` | T5 (two names in the console's `generate_handler!`) | **L6** (single owner until it merges), #19, L1, L5, L7 | T5 runs after L6 merges; recount both lists by script after any rebase |
| `app/tests/handler_lists.rs` | T5 (one test) | L6 T1, #19 | add a test; keep every other |
| `app/src/scheduler.rs`, `app/tests/scheduler.rs` | T6 | **L6** T3a (judge region, `slot_argv`'s doc and `Local` arm), C3 R9c3/R9c5 (M365 tests), L7 (maybe) | T6 after L6 merges; edits only `run_slot_with`'s new block, the argv filter and the runner-log prefixes; `slot_argv`, `judge_plan_for` and every existing test unchanged |
| `app/static/console.js`, `index.html`, `console.css` | T8, T13 (the key and copy) | L6 T4, L8 S5, L7, #19, L1 R11, L5 | Q-QUEUE: L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5 (S5's place is Quinn's ruling); L4 rebases over S5 before T13 |
| `app/tests/static_assets.rs` | T7 (held as a patch, committed in T8's commit) | L6, L8 S5t (it reaches `main` with L8's merge, so L4's branch meets it at the rebase before T13), L7, #19, L5 | add tests; a pin two lanes need carries both. **A conflict at that rebase goes to `test-writer`** (T7's author, review r2, M-4). It keeps both sides whole, S5t's tests as merged and T7's pins after them, and edits neither. T16 checks both sets are intact |
| `site/privacy.html`, `site/site.css` | T10, T13 | C3 R12b (after L4, row 105); #19 takes `main`'s page (close-out plan); L5's go-live removes the syllabus parenthesis later (draft Q2) | L4 is the only writer while it runs |
| `engine/tests/site.rs` | T9 (held as a patch), T13 (committed) | C3 R12b deletes the Microsoft 365 "does not yet" pin with its sentence | add tests |
| `app/tests/grades.rs` | T13 (`PRIVACY_BUMP_1`, one line) | L2 (merged, `e920454`) | one line |
| `cloud/supabase/functions/account/{handler.ts,index.ts,handler_test.ts}` | T2 | **L8 S4** (`index.ts`'s export and purge, `handler_test.ts`) | Q-PURGE; whichever lands second rebases; neither edits the other's lines |
| `HANDOFF.md`, `docs/reference/app.md`, a new `docs/notes/` file | T14 | the Docs lane (HANDOFF refresh), L6 T6 | T14 hands its lines to the Docs lane if it is rewriting HANDOFF at the time |
| `docs/notes/2026-10-19-l4-privacy-page-draft.md` | T1 (Appendix A re-read) | none | L4's own note |

**Not touched:** `app/src/mail_session.rs` and `app/src/mail/**` (C3's while it runs; T4 calls
`mail::setup::clean_unfinished` and `MailState::update` and edits neither; C3 plan §5's note that R9d
"rebases on L4" therefore finds nothing to rebase over), `app/src/mail_client.rs` (T4 builds a client
with its `pub` `with_token_fn` and `with_timeout` and edits neither), `app/src/commands.rs`, every contract-list file
but `account.rs`, `engine/src/**`, `engine/tests/fixtures/**`, `cloud/supabase/migrations/**`,
`.github/**`, `scripts/**`.

**Deploys.** The `account` function goes to staging from the lane branch for T15 (the controller's
push) and to prod in L9's parity pass, **before** L10's release (a released app whose `privacy_status`
meets a prod function without the route reads `Unknown`, which under Q-GATE holds nothing back, but
the panel could then never be answered). The controller adds that line to L9's checklist.

## 7. Tasks at a glance

| # | Task | Agent | Why that agent | After |
|---|---|---|---|---|
| T0 | Branch, worktree, ledger, baseline; §5's answers as rows | main session | branching, commits and baselines are the controller's | email merge |
| T1 | Re-read draft Appendix A on the merged tree; record moved lines and any claim no longer true; the extra facts T1 lists | `docs-keeper` | read-and-record, docs only | T0 |
| T2 | Cloud: `GET /account/consent`, the later-version path, the purge lines; deno tests first | `cloud-engineer` | `cloud/` is its lane; a consent-log write | T0 (parallel with T1) |
| T3 | `account.rs`: status, accept, cache, gate; failing tests first in `app/tests/account.rs` | `contract-engineer` (xhigh) | contract list; the consent log is a contract | T2's route shape agreed (T2 may still be in review) |
| T4 | `account.rs`: sign-out clears `pending_setups`; the two comments; failing tests first | `contract-engineer` (xhigh) | contract list, Credential Manager tests | T3 (same file, same agent, second commit) |
| T4R | Review of T3 and T4 | `contract-reviewer` | contract-list diff | T4 |
| T5 | The two handler names and their pin | main session (two lines) or `implementer` | one-screen edit in a single-owner file | L6 merged, T4R PASS |
| T6 | Scheduler: the named skip; failing scheduler tests first | `implementer` (high) | specified, test-checked, off the contract list | L6 merged, T4R PASS |
| T7 | Failing static pins for the panel (held as a patch, committed in T8's commit); at T13's rebase, keeps L8 S5t's tests and its own whole (§6) | `test-writer` | the failing test first; the pins are its own | T4R (the command names are fixed) |
| T8 | The panel | `console-ui` | `app/static` is its lane | T7; its queue slot: L6 merged and L8's S5 committed (Q-QUEUE) |
| T9 | Failing page pins (held as a patch until T13) | `test-writer` | the failing test first | T1, Q-L8 and the draft's Qs ruled |
| T10 | The page text (held as a patch until T13) | `console-ui` | `site/` is its lane | T9 |
| T11 | Page-text review against Appendix A | `reviewer` | Quinn's L4 row: "reviewer for the page text" | T10 |
| T12 | **Quinn reads the page** (and the panel's copy) | Quinn | the L4 row; the page is Quinn's to read | T11 PASS |
| T13 | The bump commit: four tokens, Quinn's copy, with T9's pins and T10's page | `contract-engineer` (xhigh) | `account.rs`; the constant and the page move together | T12; L8 merged (Q-L8, ruled) and the branch rebased onto it |
| T14 | Docs: HANDOFF's bump-#1 lines, `docs/reference/app.md`, the lawyer-read delta note | `docs-keeper` | docs and HANDOFF are its lane | T13 |
| T15 | Live check on staging (dev build, scratch profile) | main session | staging deploys and live reads are the controller's | T13 |
| T16 | Whole-lane review | `reviewer` and `contract-reviewer` | everything a cheaper agent changed, and every contract-list diff | T14, T15 |
| T17 | Gate, PR, CI, merge; notify the waiting lanes; worktree removal | main session | pushes and merges are the controller's | T16 PASS |

No task needs `implementer` on the contract list, and no task edits `engine/src`. Every commit leaves
the workspace building and every test green: red tests are seen red and recorded in the ledger, but are
committed only inside the commit that turns them green (T7's in T8's, T9's in T13's), so no other
task's gate ever runs on a red tree (§2, "Held diffs").

---

## T0. Branch, ledger, baseline (main session)

- [ ] **Stop unless the cloud design's Amendment 2026-10-19 is committed on `main`** (review r1, M1;
      it is uncommitted in the main checkout as this plan is revised). Record its commit sha in the
      ledger: it is the authority T13's bump and T16's check 6 cite.
- [ ] After the email merge lands on `main`, create branch `l4/honest-page` and its worktree (§2).
- [ ] Open the ledger with this plan's path, the branch point's sha, and §5's answers as dated rows
      (Quinn's one at a time, recommendation first; the controller's as rulings).
- [ ] Baseline under the slot guard: engine build, workspace build, workspace test (expect
      `ignored: 4`, or 3 if L6 has merged), and the three cloud commands. Record passed / failed /
      ignored and the warning tally.
- [ ] Confirm on the branch, by grep, that §3's facts hold: `PRIVACY_VERSION` at `2026-09-24`,
      `PRIVACY_BUMP_1` `None`, `hasConsent`'s silent 200, `sign_out_with`'s order, `pending_setups` and
      `clean_unfinished` as described, and `mail::setup::clean_unfinished` reachable from `account.rs`
      (`pub` all the way). For T4 (§4.7): `clean_unfinished` still calls `forget` on a gone record and
      still ends at a failed list; `MailClient::with_token_fn`, `with_timeout` and `MailError` are
      `pub` and reachable, and `with_token_fn` builds without a request; `SESSION_REFRESH_LOCK` is
      still a `std::sync::Mutex<()>` (so `try_lock` exists); `valid_access_token_at`'s floor is still
      120 s; `MAIL_STOP_BOUND` is still 60 s and the account agent's `TIMEOUT` 30 s (§4.7's sum uses
      both). For T6 (§4.5): `MailSeam` and `GradesSeam` are still structs of borrowed seams, so
      `run_slot_with` can build a substitute without editing `mail_step` or `grades_step`, and
      `CaptureError::Busy` still exists and has no side effect in `grades_step`.
      A fact that moved is a ledger row and joins the task that owns the file.

## T1. Re-read the evidence on the merged tree (docs-keeper)

**Files:** `docs/notes/2026-10-19-l4-privacy-page-draft.md` (Appendix A and §6 only). Read-only
everywhere else.

**Brief.**
- Re-read every Appendix A row tagged `EF` on the lane branch and diff from `5f71623`; update each
  line number in place, and mark any claim the merged code no longer makes **untrue**, quoting the code.
  Re-read the `main` rows too where the email merge touched their files.
- Re-read **c42a** against the lapsed-mailbox fix (migration `20261019000100_mail_last_lapsed_at`, the
  `mail-pull` comeback to `quiet`/`pending`, `lapsed_since` cleared only by the device's "covered"
  report, ledger rows 133, 135, 143): record whether the draft's sentence ("…up to 30 days back after a
  lapse") is still true either way, and that `last_lapsed_at` is a timestamp on the mailbox row, which
  the page's "the facts that show forwarding works" covers (no new data class, as ruled).
- List every server-side scheduled job that reads an account's mail (draft Q4 (b)'s tripwire names
  them all): cron jobs in the migrations and the functions they call.
- Confirm whether `usage_daily` and `judgments` each have a foreign key to `accounts` with `on delete
  cascade` (for T2's purge lines), with the migration and line.
- Read draft §6's open items and record which are still open (the S3 lifecycle rule stays the
  controller's at L9).

**Acceptance.** Appendix A's lines match the branch; each changed claim is one ledger row with the
code quoted; no file outside the draft note is edited. Commit message: "docs(l4): the page draft's
evidence re-read on the merged tree".

## T2. Cloud: read and record a later privacy version (cloud-engineer)

**Files:** `cloud/supabase/functions/account/handler.ts`, `account/index.ts`, `account/handler_test.ts`.

**Behaviour.** §4.3 exactly. In `handler.ts`: the header comment lists the sixth route; `Deps` gains
`getConsentVersions(accountId)` and `recordLaterPrivacy({account_id, email, privacy_version, at})`;
`recordConsent` splits after `hasConsent` into the later-version path and the silent 200; a new
`consentStatus` answers `GET /account/consent`; `handle`'s `/consent` case takes `GET` and `POST`, and
answers 405 naming both for anything else. In `index.ts`: the two deps against PostgREST (the PATCH's
forward-only filter is in the query string, not only in the handler, so two racing Accepts cannot move
the column backwards); the purge list gains `usage_daily` and `judgments` (Q-PURGE), each with the
existing comment's reason. No other route, dep or table changes.

**Tests (deno, red first, in `handler_test.ts`'s existing fake-deps style):**
- `GET /account/consent` answers the recorded pair; a null pair as nulls; no bearer is 401; no account
  row is 404; `POST`/`GET` only (405 otherwise).
- A later version on an account with a `tos` row calls `recordLaterPrivacy` once with that version and
  never `recordAccountConsent`; an equal and an older version call neither and answer 200; a non-date
  later string is 400 and writes nothing.
- The first-consent path is unchanged: the existing tests (`:403-436`) pass with their assertions
  untouched, and a new one shows a fresh account still gets `recordAccountConsent`, not the later path.
- A static pin over `index.ts`: the later-version PATCH carries the forward-only filter, inserts exactly
  one `consents` row of kind `privacy`, and names no `tos_version`, `tos_accepted_at` or
  `age_attested_at`; the purge list names `usage_daily` and `judgments`.

**Acceptance.** The three cloud commands green; deno lint clean. Commit message: "cloud(l4): the account
function reads the recorded consent and records a later privacy version". **The controller deploys
`account` to staging at T15, not here.**

## T3. The app's status, accept, cache and gate (contract-engineer, xhigh)

**Files:** `app/src/account.rs`, `app/tests/account.rs`. Nothing else.

**Behaviour.** §4.4 exactly: `consent_status_at`, `privacy_cache_path`, the cache's load and atomic save
(the `entitlement.json` pattern, its per-call `.tmp` name included), the `Gate` enum and its pure
function, `refresh_privacy`, and the two `#[tauri::command(async)]` commands `privacy_status` and
`accept_privacy`. Rules the brief adds:
- The gate compares `recorded` and `PRIVACY_VERSION` as `YYYY-MM-DD` strings; a recorded value that is
  not that shape reads as `Older` (the service only ever stores what the route accepted, so a bad value
  is a service bug the student should not be blocked from answering).
- A cache whose `account_id` is not the vault's reads `Unknown` and is overwritten by the next read.
- No token, address or reply body ever reaches a log line or an error string (CLAUDE.md, this file's
  module doc); errors use the file's existing `provider_error` and `UNREACHABLE` forms.
- `PRIVACY_VERSION` does not move here (T13).

**Tests (red first; real servers on `127.0.0.1:0`; any Credential Manager use under `CREDMAN_LOCK`
with a generated id and a `Drop` guard):**
- `consent_status_at` sends `GET /account/consent` with the bearer and reads the version and a null;
  a non-2xx is an error carrying the provider's message; an `http://` base other than loopback is
  refused before any request.
- The gate: equal and later are `Accepted`; earlier and null are `Older`; no file, an unreadable file
  and another account's file are `Unknown`; a non-date recorded value is `Older`.
- The cache round-trips and is written atomically (no `.tmp` left behind); the file holds the three
  keys and nothing else.
- `accept_privacy`'s pure core (split out as `accept_privacy_at`, the `checkout_url_at` pattern) posts
  the sign-in body with `TOS_VERSION` and `PRIVACY_VERSION`, then reads back, and answers `ok` only when
  the read-back version is at least `PRIVACY_VERSION`; a server that accepts the POST but still answers
  the old version yields `ok: false` and a cache that says `Older`.
- `refresh_privacy` over a scratch vault whose `config/cloud.yaml` names a generated
  `session_credential_target` that is never written answers `Err("no session")`, sends nothing to a
  `127.0.0.1:0` listener that counts every request, and leaves an existing cache byte for byte; with
  a generated session written at that target and a listener that answers, it writes the cache from
  the reply (T6 relies on the first case).
- A source pin, beside the existing `record_consent_at(` owner pin (`:1348`): `accept_privacy_at` is a
  caller of `record_consent_at`, and no new function writes a `consents` row any other way.

**Acceptance.** Workspace green, 0 new warnings; `git diff --stat` shows the two files. Hand-off with
the command names exactly as T5 must register them. Commit message: "account(l4): read the recorded
privacy version, cache it per profile, and accept a newer one".

## T4. Sign-out clears `pending_setups`; the two comments (contract-engineer, xhigh)

**Files:** `app/src/account.rs`, `app/tests/account.rs`.

**Behaviour.** §4.7 and §4.8 exactly, in §4.7's seven-step order: hold, clean, clear, `forget_all`,
drop the hold, read the token and delete the credential, revoke. `sign_out_with` gains one seam
parameter for the clean (a closure taking the profile folder). `sign_out` wires it to
`mail::setup::clean_unfinished(dir, &client)` with `client = sign_out_mail_client(&api_base(),
&anon_key(), &session_target(id), SIGN_OUT_MAIL_CALL)`; a client that cannot be built skips the clean,
never the sign-out. Three new `pub` items, as §4.7 specifies them:
- `sign_out_mail_client(api_base, anon, target, per_call)`: `with_token_fn` over a token function
  that calls the helper once per clean, memoised in a `std::sync::OnceLock`, then
  `.with_timeout(per_call)`; its doc comment says why it keeps a token for one clean (§4.7);
- `valid_access_token_within(auth_base, anon, target, now_unix, within)`: the lock taken by `try_lock`
  until `within` has passed, and the refresh over what is left of `within`;
- `SIGN_OUT_MAIL_CALL`, at most 5 seconds. Its doc line states §4.7's worst case term by term:
  `MAIL_STOP_BOUND` + `SIGN_OUT_MAIL_CALL` × (2 + deletes, at most three) + the revoke's 30 s, plus the local `wipe`
  term (about 2.75 s per held mail profile), every other part local.

`valid_access_token_at`, `ensure_session_for_at`, `refresh_at`, `post_json` and `agent()` keep their
behaviour; only the pure factoring §4.7 allows may touch them. The records are cleared through
`MailState::update` after the clean and before the hold is dropped. The reply gains `setups_cleared`.
The doc comments above `sign_out` and `sign_out_with` describe the new order and say why: the clean
deletes mail profiles, so it runs under the hold; its client reads the session, so it runs before the
delete; and the records are forgotten because another account may sign in here next.

**Tests (red first; the Credential Manager ones under the file's `CREDMAN_LOCK`, generated ids, `Drop`
guards).** **Every new test that calls `sign_out_with` sets `ApiBase::set(&closed_loopback_base())`
first** (review r2, I-2). Two reasons: step 7 revokes against `api_base()`, which is then refused at
once; and the file's tripwire `no_test_in_this_file_can_reach_the_compiled_in_project` (EF `:954-965`)
requires that seam in every test that reaches `api_base()`. A test that needs a listener passes the
listener's base to `sign_out_mail_client` directly and never points `KNOWLU_API_BASE` at it, so the
revoke never waits out the listener.
- `sign_out_runs_hold_clean_clear_forget_then_deletes_the_credential`: a generated session credential
  and a scratch profile folder whose `mail.json` holds three records (one never placed, one placed, one
  shown), a `turned_off` id and a `mail-session-<id>` profile. The two seams log their calls in order;
  the clean seam asserts the credential still exists and the hold is held; `close_mail_window` runs
  once, first. Afterwards: `pending_setups` empty, `turned_off` unchanged, the mail profiles gone, the
  credential gone, `setups_cleared` true.
- `a_failed_clean_still_clears_the_records_and_signs_out`: the clean seam fails; the credential is gone,
  the records are gone, the reply is `ok`.
- `the_pending_session_sign_out_touches_no_mail_json`: `None` and a non-profile id write nothing and
  call neither seam.
- `the_sign_out_clean_is_bounded_against_a_silent_service`: `ApiBase` at `closed_loopback_base()`; a
  `127.0.0.1:0` listener that accepts and never answers; a generated session with an hour left, so no
  refresh; one never-placed record. The clean seam is `clean_unfinished(dir,
  &sign_out_mail_client("http://127.0.0.1:<port>/functions/v1", "anon", &target, 200 ms))`, `<port>`
  being the silent listener's (review r3, m-3); the same listener answers `/auth/v1`. The sign-out returns
  in under 3 s; uncapped, it would wait out a 30 s default. The records are cleared, the credential is
  gone, and the reply is `ok`.
- `a_sign_out_clean_whose_refresh_hangs_is_bounded_too` (review r2, I-1): the same rig, with a session
  under two minutes from expiry, so the clean's one session read refreshes against the silent
  listener's `/auth/v1`. That refresh gives up at 200 ms, the list fails as `no-session`, and the
  sign-out returns in under 3 s with the records cleared and the credential gone.
- `valid_access_token_within`'s own tests, against loopback listeners that count every request:
  - a token with more than 120 s left is answered with no request;
  - against a silent listener, an expiring session is an `Err` within `within` + 1 s, and the stored
    session is unchanged byte for byte;
  - against an answering listener, the fresh session is saved and returned, as
    `valid_access_token_at`'s own refresh test expects.
- `valid_access_token_within_gives_up_on_a_held_lock` (review r3, I-1), `#[cfg(windows)]` like the
  race test at EF `tests/account.rs:658`, under `CREDMAN_LOCK`, two generated targets with `Drop`
  guards:
  - thread A runs `valid_access_token_at` on an expiring session at target A against a silent
    listener; the listener's first accepted connection shows that A holds `SESSION_REFRESH_LOCK`. The
    test waits for that connection against a 10 s deadline, as `loopback` does (EF
    `tests/account.rs:19-28`), and panics past it (review r4, m-2);
  - the accepted stream is moved to the test thread and kept as a local inside the `thread::scope`
    closure until the helper returns, so a failed assert drops it while unwinding and A never hangs
    the run (review r4, m-2);
  - the helper is then called with `within = 200 ms` on target B (a session with an hour left) against
    a second listener that counts every request. Only that call is timed (review r4, m-2). It must
    answer `Err` with zero requests and target B's stored session unchanged byte for byte, and its
    elapsed time must be **at least 200 ms and under 1.2 s** (review r4, m-1): a helper with no
    deadline would wait out A's 30 s and then answer `Ok`, and a helper that tries once and gives up
    at once would answer under 200 ms, so both fail;
  - finally the test drops A's accepted stream, so A's refresh fails at once, and joins A.
- Source pins:
  - `SIGN_OUT_MAIL_CALL` is 5 seconds or less, and its doc line names `MAIL_STOP_BOUND`, the revoke
    and `wipe`;
  - `valid_access_token_within`'s body calls `try_lock` and never `.lock(`;
  - `sign_out_mail_client`'s body holds a `OnceLock` and calls `valid_access_token_within(` only
    inside its `get_or_init` (review r3, m-3);
  - its only caller in `src/` is `sign_out_mail_client`;
  - `sign_out`'s body calls `sign_out_mail_client(` with `SIGN_OUT_MAIL_CALL`.
- The existing `sign_out_deletes_this_profiles_mail_session_and_mail_setup_profiles_and_no_others`
  (`:2068`) keeps every assertion; it only passes the new seam (a no-op). The source tripwire at
  `:2168` keeps its assertions and is extended to require the clean's call in `sign_out`'s body and
  the clean before `credentials::delete` in `sign_out_with`'s.
- No test leaves the machine: the only real client in a test points at the loopback listener.

**Acceptance.** Workspace green, 0 new warnings. `git grep -n "offer_marker" app/src/account.rs` hits
only `mail_offer_marker`; `:818`'s comment is reworded only if L6 has merged (§4.8), else a ledger row
hands it on. Commit message: "account(l4): sign-out tries the unfinished mail setups, then forgets
them".

## T4R. Contract review of T3 and T4 (contract-reviewer)

Read `git diff <T0 sha>..HEAD -- app/src/account.rs app/tests/account.rs` against §4.4, §4.7, §4.8 and
CLAUDE.md's contract rules: no secret in a log, error or test name; the consent body unchanged; the
session refresh under `SESSION_REFRESH_LOCK`; §4.7's seven-step sign-out order (hold first; clean and
clear under it; the token read and the credential deleted after the hold is dropped; revoke last); and
**the clean's bound** (review r2, I-1). For the bound, the reviewer checks four things:
- every network wait between the hold and the revoke is capped by `SIGN_OUT_MAIL_CALL`: the helper's
  lock wait and refresh together, and each mail request. The lock-wait half is proved by
  `valid_access_token_within_gives_up_on_a_held_lock` and the refresh half by
  `a_sign_out_clean_whose_refresh_hangs_is_bounded_too` (review r3, I-1); `Poisoned` is taken as
  acquired; the token is memoised in a `OnceLock`, so the clean reads it once;
- §4.7's worst case, 60 + 5 × (2 + n) + 30 s plus the `wipe` term, matches the code and
  `SIGN_OUT_MAIL_CALL`'s doc line;
- `valid_access_token_at`, `ensure_session_for_at`, `refresh_at`, `post_json` and `agent()` behave as
  before for their other callers: their diff is empty, or a pure factoring with `TIMEOUT` passed;
- `valid_access_token_within` has one caller.

The reviewer also checks that every Credential Manager test is serialised and cleaned up, and that
every test calling `sign_out_with` sets `ApiBase` and reaches no network but loopback. Report to
`docs/reports/2026-10-19-l4-account-review.md`. A FAIL goes back to T3/T4's agent and is re-reviewed;
T5 and T6 wait for PASS.

## T5. The two handler names (main session, or implementer)

**Files:** `app/src/main.rs` (the console's `generate_handler!`), `app/tests/handler_lists.rs`.

After L6 merges and the branch is rebased onto it: add `account::privacy_status` and
`account::accept_privacy` to the **console** list only (the wizard has no vault and never asks), and a
test `the_console_list_names_the_two_privacy_commands_and_the_wizard_list_neither` in the file's style
(`list_with`), red before the two names. Recount both lists by script and record the counts.
**Acceptance:** workspace green. Commit message: "app(l4): register privacy_status and accept_privacy".

## T6. The scheduler's named skip (implementer, high)

**Files:** `app/src/scheduler.rs`, `app/tests/scheduler.rs` (new tests only).

**Before starting:** rebase onto `main` if L6, C3 or L7 merged a `scheduler.rs` change since T0, and
re-read `run_slot_with` on the rebased branch.

**Behaviour.** §4.5 exactly, through `account::refresh_privacy` and the gate function from T3 (no new
logic about versions in `scheduler.rs`). One block right after `let est`, entered only when `est` is
`Entitled`; `est` and every entitlement path are unchanged. `grades_step`, `mail_step`, `slot_argv` and
`judge_plan_for` are not edited: a step that would have run becomes the privacy skip through a
substitute seam or by replacing a `JudgePlan::Run`, built in `run_slot_with`, and the three engine
steps are filtered out of `slot_argv`'s result by `args[0]`. The step names are exactly `privacy (check
failed: <reason>)` (summary) and `privacy (check failed)` (runner log), `sync (skipped: privacy update
not accepted)`, `coursework (skipped: …)`, `ingest (skipped: …)` (only when `slot_argv` emitted it),
`grades (skipped: …)`, `judge (skipped: …)` and `mail (skipped: …)`, each at exit 0. A successful check
adds no row.

**The rig (review r1, I1; review r2, M-1).** Each test holds `ENGINE_ENV_LOCK` with an `EnvSeam`
pointing `LOCALAPPDATA` at a fresh temp folder. Its scratch vault's `config/cloud.yaml` names two
things: an `api_base` of a `127.0.0.1:0` listener the test serves, and a generated
`session_credential_target` (`knowlu/<generated id>/session`) that nothing ever writes.
- **The slot reads Credential Manager at that target**, so this paragraph no longer says "no test
  touches Credential Manager". The readers are the session step (`ensure_session_for_at`, EF
  `scheduler.rs:884-905`), `refresh_privacy` (which reads `cloud.yaml`'s target as
  `refresh_entitlement` does, `account.rs:865-868`), `refresh_entitlement` in the M3 test, and
  telemetry if it has anything to send.
- Each of these is a read of a never-written target. That is the exemption `tests/scheduler.rs`
  already records (`m4_quit_flush_reports_a_skip_as_a_skip_never_synced`, EF `:630-634`), so the tests
  take no `CREDMAN_LOCK`, and none writes or deletes a credential.
- Each read fails before any request, so the listener counts **every** request, whatever its method
  and path, and every test asserts it got none.
- `refresh_privacy` therefore fails with `no session` (T3 pins this), and the test's cache stays as
  written. The success path of the refresh is T3's to test.

The gate is driven by writing the privacy cache file. The entitlement cache is written `Entitled`
(`NotEntitled` in the M3 test). The grades and mail seams are test closures that panic if called,
unless the test says otherwise; the mailbox list is a closure returning the rows the test names.

**Tests (red first):**
- `an_older_privacy_version_skips_every_cloud_step_by_name_and_still_ranks`: an `Entitled` vault, an
  `Older` cache, a grades row that would capture and one due mailbox. Rows, in order (review r2,
  M-2): the session step's `session (refresh failed: …)` (EF `scheduler.rs:901`, matched by that
  prefix: its reason is the Credential Manager read's), then **exactly one privacy row**, `privacy
  (check failed: no session)`, then the grades skip, the mail skip, the judge skip, then the `sync`,
  `coursework` and (if emitted) `ingest` skips. `rank` runs; every code is 0; `engine_ok` holds.
  Neither real seam was called. The grades row is privacy's, not `another grades capture is running`,
  and no grades signed-out mark was written (the stand-in of §4.5). The argv handed to the child
  loop names no `sync`, `coursework` or `ingest`.
- `an_older_version_names_only_steps_that_would_have_run` (M2): an `Older` cache, no mailbox connected
  and a grades row that is not connected: no mail row at all, and the grades row is today's `grades
  (skipped: not connected)`, not privacy's.
- `an_accepted_privacy_version_leaves_the_slot_as_it_was`: the rows are today's for the same vault,
  the session row first, with no privacy row of any kind; no request; and the argv equals
  `slot_argv`'s unfiltered output.
- `an_unknown_privacy_version_names_only_the_failed_check`: no cache. The session row comes first, then
  exactly one privacy row, `privacy (check failed: no session)`, and no `(skipped: privacy` row. The
  argv is unfiltered, and no cache file is created.
- `an_unentitled_account_keeps_the_entitlements_names` (M3): a `NotEntitled` entitlement cache and an
  `Older` privacy cache: the rows equal today's `NotEntitled` rows (`grades (skipped: no entitlement)`,
  `judge (skipped: no entitlement)` and the rest), no privacy row, the argv unfiltered.
- `the_privacy_skips_reach_the_runner_log_except_mails`: the vault's `state/runner-log.md` gains the
  engine-step, grades and judge skips and the bare `privacy (check failed)` as `ok` lines, beside
  today's bare `session (refresh failed)`, and no mail line and no reason text.
- `run_slot_end_to_end` stays `#[ignore]` and untouched; no existing test is edited.

**Acceptance.** Workspace green, 0 new warnings; `git diff main -- app/src/scheduler.rs` touches only
`run_slot_with` and its doc. Commit message: "scheduler(l4): until the privacy update is accepted, the
slot's cloud steps are named skips".

## T7. Failing static pins for the panel (test-writer)

**Files:** `app/tests/static_assets.rs` (new tests only).

**Tests, each red today for the stated reason:**
- `the_reconsent_panel_has_accept_settings_a_policy_link_and_no_way_to_dismiss_it`: `index.html` has
  `id="reconsent"` as a `.setpanel`, a button `id="rc-accept"`, a button `id="rc-settings"`, an
  `a.policy` with `data-policy="privacy"` inside it, and no element inside it whose id or text says
  *Not now*, *Close* or *Later*; `console.js` binds `#rc-settings` to `openSettings()` (review r1, I4)
  (red: no panel).
- `the_console_asks_privacy_status_once_at_launch_and_accept_privacy_on_accept`: `console.js` invokes
  `privacy_status` from the account check's path and `accept_privacy` from `#rc-accept`'s handler, and
  routes the panel's policy link through `open_policy` (red: neither invoke exists).
- `settings_starts_nothing_new_while_the_update_is_owed`: the settings render reads the owed flag and
  disables the start controls §4.6 names (by id, as T8's hand-off lists them; T7 writes the ids it
  finds in `index.html` today and T8 may only add), carries the line "Accept the privacy update to use
  this", and leaves the Delete my data, Disconnect, Turn off and Forget controls untouched; the panel
  stands down while `#settings` is open (red).
- `an_unreachable_privacy_check_never_raises_the_panel`: the failed-call branch hides the panel and
  sets no owed flag (red).
- `the_panels_changes_describe_this_privacy_version`: the version key of `PRIVACY_CHANGES` in
  `console.js` equals `account.rs`'s `PRIVACY_VERSION` (read as `the_privacy_version_constant_is_the_published_pages_date`
  reads it). Red today (no `PRIVACY_CHANGES`); T8 makes it green with today's key; from then on it
  fails any commit that moves the constant without the key, which is what makes T13 move both.

**Acceptance.** The five fail for those reasons and no other; every existing test is green. The
controller records the red run in the ledger, saves the diff as `t7-panel-pins.patch` and reverses it
(§2, "Held diffs"); nothing is committed here. The pins are committed in T8's commit, with the panel
that turns them green, so the branch never carries a red test; the ledger's red run is the TDD record.

## T8. The panel (console-ui)

**Files:** `app/static/index.html`, `app/static/console.js`, `app/static/console.css` (only if the
panel needs a rule `.setpanel` does not give it).

**Before starting:** its `console.js` slot (Q-QUEUE): L6 has merged and the branch is rebased onto it
(worktree clean, §2), and L8's S5 is committed on L8's branch, or the controller has recorded starting
without it. Read S5's diff with `git show` (read-only) and keep clear of its lines: `renderAccountRow`,
the wizard's subscribe step and `afterUpgradeSignIn`. The controller applies `t7-panel-pins.patch`.
When L4 rebases onto `main` after L8's merge (before T13), a conflict in these files comes back to
console-ui, which keeps S5's lines as they are and re-fits the panel's around them.

**Behaviour.** §4.6 exactly. The panel sits beside `#upgrade` and follows its shape: one owed flag,
one busy flag (two presses on Accept are one command), `open_policy` for the link, `UNREACHABLE` read
from the reply as `#upgrade` reads it, and the Settings button calling `openSettings()`. `PRIVACY_CHANGES` holds the draft copy under the key
`"2026-09-24"` (today's constant; T13 moves it with the constant). In Settings, the owed flag disables
the start controls and shows the one line (§4.6); the hand-off lists every id disabled and every
stopping control left live. No other view, id or handler changes; no copy outside the panel and that
one line.

**Tests.** T7's five pins turn green.
**Acceptance.** `scripts/wizard-check.py` and the settings check ok (controller); a fixture screenshot of
the panel over today's page, by headless Edge. Hand-off with the diff's line counts. The controller
commits T7's pins (from the applied patch) and T8's files as one commit: "console(l4): the re-consent
panel and its pins". The patch is then retired in the ledger.

## T9. Failing page pins (test-writer)

**Files:** `engine/tests/site.rs` (new tests only).

**Tests.** The draft's §4.3 "L4.1" list, as ruled at T0: the banned strings, the present strings, the
Q3 (ii) or (i) sentence, and the "Last changed" pin (a real `YYYY-MM-DD` no earlier than the version
line's date). Added by this plan: if Q-L8 is ruled as recommended, sentence 1's "We keep daily counts
of your AI calls and cloud requests" and sentence 2's "If Knowlu gives you free tester access"; the
registrar's "at schools Knowlu supports, your class schedule" (already in the draft's list); and
"Settings → Delete my data" stays pinned. Every existing pin in `site.rs` and `static_assets.rs` is
untouched.

**Acceptance.** Each new test fails on today's page for its own reason; every other test stays green.
The controller records the red run (each test's name and failure line) in the ledger, saves the diff as
`t9-page-pins.patch` and reverses it (§2, "Held diffs"). **Nothing is committed here:** the pins are
committed in T13's one commit, with the page they read, so the branch never carries a red test.

## T10. The page text (console-ui)

**Files:** `site/privacy.html`, `site/site.css` (only for a table style, if none exists).

**Behaviour.** §4.1: the draft's §3 applied to the page, section by section, with each *[bracketed
instruction]* settled as T0's ledger rows rule it, every *[after C3]* block left out, and L8's two
sentences placed as Q-L8 rules (sentence 1 in "What we collect" beside the account copy's other
counters; sentence 2 in "How long we keep it"; T11 may move either). The Effective and version lines
**keep `2026-09-24`**, and the "Last changed" line (Q3 (ii)) reads `2026-09-24` too: T13 moves all
three. No `<script>`, no external asset, no new page.

**Before starting:** the controller applies `t9-page-pins.patch` to a clean worktree.

**Tests.** With T9's patch applied, T9's pins turn green except any that read the version date;
`site.rs`'s and `static_assets.rs`'s existing pins stay green.

**Acceptance.** Nothing is committed. The controller saves the page's diff as `t10-page.patch` (kept
apart from T9's) after each change, so a crash loses nothing, and reverses both patches before any
other task's gate or any rebase (§2, "Held diffs"). A headless-Edge render of the page to a PNG in the
session scratchpad for T11 and Quinn. Hand-off: the section list with, for each, the draft section it
came from.

## T11. Page-text review (reviewer)

Read T10's diff and render against the draft's §3 and the re-read Appendix A (T1), with draft §1's
VISION checks: every sentence traceable to a tag, every tag's evidence true on the branch, no sentence
the draft lacks except Q-L8's two, no school, person or tenant named, the A8 list and no more, telemetry
and grades kept apart, "set aside" not "scrubbed". Also: the page reads as plain English to a student.
Report to `docs/reports/2026-10-19-l4-page-text-review.md`. A FAIL goes back to T10; re-reviewed.

## T12. Quinn reads the page (checkpoint)

The controller gives Quinn, in one message: the rendered page (PNG and the file path), the list of what
changed since 2026-09-24 (draft §0), the panel's four lines (Q-COPY), and any of §5's (Quinn) questions
still open. Quinn's edits are ledger rows; T10 applies them (on the re-applied patches, saved again
after) and T11 re-reads only the changed sentences. **Nothing is committed with the page until Quinn
says the page reads right.**

## T13. The bump commit (contract-engineer, xhigh)

**Files:** `app/src/account.rs` (one line), `app/tests/grades.rs` (one line), `site/privacy.html` (the
three date lines), `app/static/console.js` (the `PRIVACY_CHANGES` key and, if Q-COPY changed them,
the four lines). The commit also carries the held `t9-page-pins.patch` (`engine/tests/site.rs`) and
`t10-page.patch` (`site/privacy.html`, `site/site.css` if T10 touched it), which this task does not
edit.

**Before starting:** T12 done. **L8 has merged** (Q-L8, ruled 2026-10-19: L8's S5 sits ahead of T8 in
the queue and waits for no L4 task, so this wait ends when L8 does), carrying S4's export lines and
S5's console lines. With the worktree clean (§2), the controller rebases the branch onto `main`.
Conflicts go to their owners:
- one in `app/static` goes to console-ui (T8's rule: S5's lines are kept);
- one in `app/tests/static_assets.rs` goes to test-writer (§6: S5t's tests and T7's pins are both
  kept whole; review r2, M-4);
- one in `account/index.ts` follows Q-PURGE.

**Then the rebased tree is checked before any patch goes back on** (review r2, M-5). With the
patches reversed, the controller runs, under the slot guard:
- the workspace gate: engine build, then workspace build and test, 0 other warnings, and the eol check;
- `scripts/wizard-check.py` and the settings check;
- a headless-Edge fixture screenshot of the panel over today's page, compared with T8's.

A red result goes to the agent that owns the file it names, and nothing below starts until all of them
are green. The controller then re-applies T9's and T10's patches with `--check` first and re-runs
`site.rs`. If L8 has not merged when everything else is ready, the
controller asks Quinn Q-L8's fallback once and records the answer; nothing here proceeds on an
assumption.

**Behaviour.** §4.2: `V` is today's date (Q-V). `PRIVACY_VERSION = "V"`; `PRIVACY_BUMP_1 = Some("V")`;
the page's Effective, version and "Last changed" lines read `V`; the panel's key reads `"V"`. Nothing
else changes in any of the four files.

**Tests.** No new test. Green after: `the_privacy_version_constant_is_the_published_pages_date`, the
date-and-bump test, T7's version pin, every T9 pin, and
`the_privacy_guard_blocks_mail_code_at_or_before_the_pre_bump_version` (which now asserts the guard
passes on this tree). The controller also runs `scripts/ci/privacy-guard.ps1 -Root <worktree>` and
records its "past 2026-09-24" line.

**Acceptance.** Workspace green, 0 new warnings, eol check. The controller commits T9's pins, T10's
page and this task's four files **as one commit**: "privacy(l4): privacy bump #1, the page says what the
app does today (version V)". `git show --stat` lists exactly `app/src/account.rs`,
`app/tests/grades.rs`, `app/static/console.js`, `site/privacy.html` and `engine/tests/site.rs`: five
files, six with `site/site.css` if T10 touched it. (Round 1's draft said "five" without
`engine/tests/site.rs`, which counted `privacy.html` twice.) The patches are then retired in the
ledger.

## T14. Docs (docs-keeper)

**Files:** `HANDOFF.md`, `docs/reference/app.md`, a new `docs/notes/2026-10-19-privacy-bump-1-lawyer-delta.md`.

**Brief.** From the merged branch's code, not this plan's words:
- `HANDOFF.md`: the bump-#1 lines (§3's "What privacy bump #1 owes from M1" and every "before privacy
  bump #1" release gate it names) marked done with `V`, the commit and the PR; the Pilot row's
  re-consent screen marked built (its lawyer-read bump still owed); §1's C3′ bullet ("no re-ask screen
  is needed") left as history with a dated pointer. If the Docs lane is rewriting `HANDOFF.md`, hand
  it these lines instead.
- `docs/reference/app.md`: the two commands; `privacy.json` in app data, with its three keys named
  (`account_id`, `recorded`, `checked_at`), that it is written only from a service reply, never syncs
  and enters neither the vault nor telemetry, and that none of the three is a new data class (§4.4,
  review r1 M4); the scheduler's new skip names, `privacy (check failed)`, and the two naming rules
  (only a step that would have run; the entitlement's names win); sign-out's new order and
  `setups_cleared`.
- The lawyer-delta note: every sentence new or changed since the 2026-09-24 page, grouped by the spec
  that required it (Gmail §6, D18, M2 §5, grades §11, #19 §8 Q5, L5 §6.1, L8 Q7), plus C3 Q7's
  automated-access item, for the before-Pilot lawyer read. No page text is invented there.

**Acceptance.** `git grep -n "PRIVACY_VERSION" docs/reference/app.md` names `V`. Commit message:
"docs(l4): bump #1 recorded; the re-consent commands, cache and skips in the reference".

## T15. Live check on staging (main session)

Scratch profile, staging, a dev build of the lane branch; never a live vault; DOM only, no OS input;
counts and outcomes recorded, never an address or token.
- [ ] Deploy `account` to staging from the lane branch (`supabase functions deploy account --use-api`,
      `--workdir cloud`); the hook asks; record "Deployed Functions.".
- [ ] A staging account that recorded `2026-09-24` (a scratch account signed up on a build of `main`
      before the bump, or an existing scratch proof account): `GET /account/consent` by OTP session
      answers `2026-09-24`.
- [ ] Launch the lane build on a scratch profile for that account: the panel shows; Settings opens
      with the start controls disabled and Delete my data live, from the panel's own Settings button;
      *Run now* records the privacy skip for each step that would have run (§4.5: on a scratch profile
      with no mailbox and no grades sign-in, `sync`, `coursework` and `judge` at least, no mail row,
      grades under its own name), all at code 0, and `rank` ran; the tray is not amber.
- [ ] The policy link opens the live page (the old one, per Q-LIVE (3); recorded, not a failure).
- [ ] Accept: the panel closes; `GET /account/consent` answers `V`; `privacy.json` holds `V`; the next
      *Run now* runs `sync`, `coursework`, `judge` and the rest as before.
- [ ] Sign-up path: a fresh scratch account signed up on the lane build records `V` at once and never
      sees the panel (draft Q5).
- [ ] Sign-out: only if a never-placed record can be produced on a scratch profile without signing in to
      a real mailbox (T0 checks where `runner.rs` writes the first record against the sign-in step):
      sign out. **Expected, whatever the clean answered** (review r1, M5): the reply is `ok` with
      `setups_cleared: true`, and `mail.json` holds no `pending_setups` (a failed list or delete keeps
      nothing locally). On the service: if the clean's list and delete succeeded, the staging address
      list no longer shows the address; if either failed, the address stays `pending` under the
      signed-out account (§4.7's accepted cost), the ledger records which, and clean-up below deletes
      it. One mint at most; the staging mint cap (session ledger row 48) is the controller's to watch.
      Otherwise the check is recorded as not run, and T4's tests stand for it.
- [ ] Policy-change notice (the page's "we will email the address on your account" is a published
      commitment; T12-fix2): before the release that deploys V, the controller sends one notice to every
      account whose latest consent `privacy_version` is below V, through the account-mail provider, and
      records the count in the ledger. Nothing in `cloud/supabase/functions` sends it; do not soften the
      page's sentence.
- [ ] Clean-up per memory "live-proof profile cleanup": the scratch profiles, vaults, credentials and
      autostart entries; any staging address left by the check deleted.

## T16. Whole-lane review (reviewer, contract-reviewer)

`reviewer` reads `git diff main...l4/honest-page` against this plan; `contract-reviewer` re-reads
`account.rs`'s whole diff (T3, T4, T13). Reports to `docs/reports/2026-10-19-l4-honest-page-review.md`.
Checks beyond the usual:
1. Each §4 behaviour landed as written; nothing outside §6's file list changed; T13's commit has
   exactly its five (or six) named files and is the only commit that moves a version token; no commit
   on the branch leaves a test red (T7's pins inside T8's commit, T9's inside T13's).
2. No existing test was weakened or edited beyond what §7's tasks name; T4's two edited tests keep
   every assertion.
3. Rule 2: `git diff --exit-code main -- engine/tests/fixtures` is empty; no `surface-today-*.json` moved.
4. No contract-list file but `account.rs` changed.
5. No secret, address, account id, school or person in code, tests, the page, the ledger or a report.
6. `PRIVACY_VERSION` moved in a lane PR by Amendment 2026-10-19, committed on `main` at the sha T0
   recorded, and by Quinn's Q-AGENT answer, cited by its ledger row (the Amendment itself names no lane
   PR; review r1, M1); and the privacy guard now passes on this tree.
8. The ledger holds the T15 policy-change notice row (count sent) before T17 merges; the page's Export
   bullet names what `exportAll` leaves out (decisions, Google connection, Gmail proposals) until a
   cloud-engineer task adds those tables, without token columns, to `exportAll` with a handler test.
7. The warning tally is 0 other; the ignored count is the baseline's.
A PASS lets T17 go ahead; findings are answered in §11 and re-reviewed.

## T17. Gate, PR, merge (main session)

- [ ] Final gate under the slot guard (engine build, workspace build and test, eol, the three cloud
      commands, `wizard-check.py`, the settings check); numbers in the ledger.
- [ ] Push, open the PR ("L4: the honest page, privacy bump #1 and the re-consent panel"), watch CI
      (`gh run watch <id> --exit-status`).
- [ ] Merge with a merge commit on green CI, T16's two PASSes, and T12's record that Quinn read the page
      (handoff ruling 7; the page read is the L4 row's own gate). Never over a red check.
- [ ] Ledger rows; tell L7 (its `console.js` slot is open, unless it already overtook L4 under
      Q-QUEUE), C3 (R12b may start), L5 (Q42's gate is open; L5's go-live owes a bump under
      draft Q2 (a)), #19 (row 39 item 8's page condition is met), L8 (S7 waits for L10's publish) and
      L9 (the `account` function before L10; prod `GOOGLE_CLIENT_ID` after L10's publish, Q-LIVE); and
      L10 (the privacy guard now passes). Remove the worktree.

## 8. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T0; findings answered in §11; the status line moves to
   REVIEWED with the passing report's path. Round 1: `docs/reports/2026-10-19-l4-plan-review-r1.md`
   (FAIL); round 2: `…-r2.md` (FAIL); round 3: `…-r3.md` (FAIL); round 4: `…-r4.md` (PASS, minors
   applied); each answered in §11. **Passed.**
2. **Quinn, before T9**, one question at a time with its recommendation: L8's Q5, Q7 and Q8 (Q-L8; the
   order itself is ruled), the draft's Q1-Q9, Q-GATE, Q-SKIP (with its two naming rules), Q-LIVE,
   Q-AGENT (before T0's ledger opens if possible: T16 cites it). Q-COPY waits for T12. Q-L8's fallback
   is asked only at T13, and only if L8 has not merged by then.
3. **Controller at dispatch:** Q-V, Q-ACCEPT, Q-QUEUE, Q-PURGE, Q-SCHED; T0's confirmations.
4. **After T4:** `contract-reviewer` (T4R) before anything builds on `account.rs`.
5. **T11, then T12: Quinn reads the page** (and the panel's copy) before anything is committed with it.
6. **T15** (live), then **T16**, then T17's merge.

## 9. Out of scope, and what this lane hands on

- **D18's upload row and the Microsoft 365 mailbox-actions sentence**: C3's R12b, after L4 merges, with
  `PRIVACY_VERSION` unmoved (row 105); R12b also deletes T9's Microsoft 365 "does not yet" pin with its
  sentence.
- **Removing the syllabus parenthesis**: L5's go-live, a new version and a re-prompt under draft Q2 (a);
  L5's plan carries the bump and the `account.rs` edit (contract-engineer). The panel and the gate built
  here are what it reuses.
- **The lawyer-read bump** before the Pilot: a later lane; this lane's panel and `PRIVACY_CHANGES` key
  are built for it. The lawyer packet is the delta note T14 writes, plus the terms.
- **A later `TOS_VERSION`** (the terms carrying the Amendment 2026-10-16 prices before the first outside
  sign-up): the route records only a later **privacy** version. A terms re-consent is a later lane's,
  with the same shape (Q-ACCEPT's body already carries `TOS_VERSION`).
- **Server-side gating of forwarded mail** before Accept: not built, under draft Q4 (b)'s ruling, with
  its tripwire; T1's list of scheduled mail jobs is what the ruling names.
- **The export's new tables** (`usage_daily`, `route_daily`, `comp_grants`): L8's S4.
- **Whether friends count as "anyone else" under VISION's pilot sentence**: L8 §8.2's amendment line,
  Quinn's to sign.
- **The page going live**: L10's release (Q-LIVE); a site-only deploy only on Quinn's word.
- **The app half of the lapsed comeback** (the "covered" report): the email lane's follow-up; the page
  sentence holds either way (T1 confirms).
- **The S3 lifecycle rule** behind "about two days" (draft §6): the controller reads it at L9, before the
  page ships.
- **Rewording `.claude/agents/{reviewer,cloud-engineer}.md`** (Q-AGENT): Quinn's; no agent edits `.claude/`.
- **`account.rs:818`'s comment** if L6 has not merged by T4: the next contract-list change (§4.8).

## 10. Fidelity ledger

| Source | Requirement | Task | Proof |
|---|---|---|---|
| Handoff L4 row; ruling 5 of 2026-10-18 | `site/privacy.html` says what the app does today: the mail table, grades, syllabus | T10, T13 | T9's pins; T11's review against Appendix A; T12 |
| Amendment 2026-10-19 | L4's bump is privacy bump #1, carrying the Gmail §6, D18, M2 §5 and grades §11 sentences, no lawyer read | T10, T13 | draft §7's ledger rows, each pinned by T9; T14's delta note lists them for the later read |
| Handoff L4 row | `PRIVACY_VERSION` bump | T13 | `the_privacy_version_constant_is_the_published_pages_date`; the guard's verdict on this tree |
| `account.rs:21-23` | the constant and the page's date move in one commit | T13 | T16 check 1 (`git show --stat`, five or six named files) |
| Grades spec §11; HANDOFF §3 M1 | `PRIVACY_BUMP_1` set to the bump's version | T13 | the date-and-bump test with `Some(V)` |
| D18 (email spec §7) and `privacy-guard.ps1` | no release carries mail code before bump #1; after it, the guard passes | T13 | `workflows.rs`'s guard test; the script's own line at T13 |
| Session ledger row 39 item (5); p3 close-out O8, §8 Q5 | the registrar sentence, "at schools Knowlu supports" | T10 | T9's pin "at schools Knowlu supports, your class schedule" |
| Session ledger row 121 (L5 Q39, Q42) | L5's §6.1 on L4's page, with the "not in this version" parenthesis | T10 | T9's pins; draft Q2 |
| Friends-testers spec §8.5, S8, Q7 | L8's two sentences before L4's bump | T10, T13 (after L8's merge) | T9's two pins; Q-L8 |
| Quinn's ruling 2026-10-19 on review r1's C1 (option (a)); friends-testers spec §8.1 and S5 row as revised | L8's S5 ahead of L4's T8 in the queue; L8 merges before T13, so both sentences are true when the page lands | T8, T13 | Q-QUEUE; T8's and T13's "Before starting"; T13's commit sits on a `main` that carries L8's merge |
| Session ledger rows 133, 135, 143 | `last_lapsed_at` is not a new data class; c42a still true | T1 | T1's ledger row with the code quoted |
| Handoff L4 row; R-PS-4 | a re-consent screen | T3, T5, T7, T8 | T7's pins; T15's live check |
| Ruling 5 of 2026-10-18 | the screen built now so the lawyer-read version re-prompts everyone | T3, T8 | the generic gate; `PRIVACY_CHANGES` keyed by version (T7's pin) |
| `privacy.html`'s "carrying on is not how we take consent" | new uses do not run on the old consent before Accept, on the device | T6 | T6's six tests; Q-SKIP (with its naming rules), Q-GATE |
| Cloud: no existing account can record a later version (§3) | `POST /account/consent` records a later privacy version; `GET` reads it | T2 | T2's deno tests and static pin |
| VISION, spec §5.1, D4 | a dead network never raises the panel or hides today's page; `rank` never gated | T6, T8 | T7's unreachable pin; T6's "still ranks" test |
| VISION "Failures are visible" | every left-out step is named at exit 0 | T6 | T6's tests (names, codes, runner log) |
| Handoff L4 row; email plan's a7f1ca0 review row | sign-out clears `pending_setups` after trying their deletes | T4 | T4's order, failed-clean and pending-session tests; T15's sign-out check |
| L6 plan §9 | `account.rs`'s two stale comments | T4 | T4's grep; the ledger row for `:818` if L6 has not merged |
| L8 §8.6 | the purge list names `usage_daily` and `judgments` | T2 | T2's static pin; T1's foreign-key finding |
| Handoff §5 serialisation, as amended by Quinn's queue ruling | `account.rs` L4 only; `main.rs` after L6; the `console.js` queue L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5 | T3-T5, T8, T13 | §6; Q-QUEUE |
| Review r1, I3 | no committed tree carries a red test; rebases run on a clean worktree | T7-T10, T13 | §2 "Held diffs"; T16 check 1 |
| Email plan's a7f1ca0 row; review r1, I2 | the clean runs under the mail window's hold and before the credential delete, bounded | T4 | T4's order, failed-clean and bound tests; T4R |
| Review r2, I-1; Quinn's decision of 2026-10-19 | every network wait in sign-out's clean is capped by `SIGN_OUT_MAIL_CALL` (a new `valid_access_token_within`), the worst case 60 + 5 × (2 + n) + 30 s plus the local `wipe` term is stated, and `valid_access_token_at`'s other callers are unchanged | T4 | the two bound tests, the helper's three tests, `valid_access_token_within_gives_up_on_a_held_lock` (review r3, I-1) and the source pins (`OnceLock` memo included); T4R's four bound checks |
| Review r2, I-2 | the bound tests never point `KNOWLU_API_BASE` at the silent listener, and every `sign_out_with` test sets `ApiBase` | T4 | the file's tripwire (`no_test_in_this_file_can_reach_the_compiled_in_project`); T4R |
| Review r2, M-1 to M-3 | T6's rig names its Credential Manager reads and counts every request; its rows include the session row; grades' privacy skip has a stated mechanism with no side effect | T6 | T6's tests (session row first, one privacy row, no grades signed-out mark) |
| Review r2, M-4 and M-5 | a `static_assets.rs` conflict at T13's rebase has an owner, and the rebased tree is green before the patches go back on | T7, T13 | §6's row; T13's "Before starting"; T16 check 1 |
| Draft Q4; review r1, I4 | Settings → Delete my data reachable from the panel itself while consent is owed | T7, T8 | T7's first pin (`#rc-settings` bound to `openSettings()`); T15's "Settings opens" step |
| Handoff L4 row | a reviewer checks the page text, and Quinn reads the page | T11, T12 | T11's report; T12's ledger row |
| CLAUDE.md contract list | `account.rs` by contract-engineer, read by contract-reviewer | T3, T4, T13 | T4R; T16 |
| CLAUDE.md rule 1 | no single-user assumption; the cache keyed by account id | all | T16 check 5; T3's other-account cache test |
| CLAUDE.md rule 2 | no frozen reference regenerated | all | T16 check 3 |
| CLAUDE.md "tests never leave the machine", Credential Manager lock | loopback servers only; serialised credential tests | T3, T4, T6 | T4R; T16 |
| CLAUDE.md "commands.rs computes nothing" | the two commands live in `account.rs` | T3, T5 | T16 check 1 (`commands.rs` untouched) |
| CLAUDE.md "0 warnings", TDD | each task red first, then green, 0 other warnings | every task | each acceptance line; T17's gate |

## 11. Fidelity notes

### Round 1 (2026-10-19): `docs/reports/2026-10-19-l4-plan-review-r1.md`, FAIL (1 critical, 4 important, 5 minor)

The report's "code facts check out" list needed no answer. Its gate-design and privacy paragraph
still holds after this revision: the skips are still app-side named rows at exit 0, `Unknown` still
changes nothing, and the one new cloud write is still a `consents` row of an existing kind.

| # | Sev. | Finding | Answer |
|---|---|---|---|
| C1 | critical | The L8 ordering deadlocked. T13 waited for L8's merge, L8 needed S5, and the friends-testers spec held S5 until L4 had merged. | **Quinn ruled option (a), 2026-10-19:** L8's S5 moves ahead of L4's T8 in the queue and waits for no L4 task, so L8 can merge before T13. Q-L8 records the ruling and keeps two open parts for Quinn: L8's Q5/Q7/Q8, and the fallback, asked only at T13. Q-QUEUE gives the new queue, L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5, and what each slot means. T8 starts once S5 is committed on L8's branch, keeps clear of S5's lines, and L4 takes S5 at the rebase before T13. Why not wait for L8's merge before T8: T13 waits for it anyway. While T13 waits, L7, #19 and L1 R11 may overtake L4. §6's order paragraph states that no wait runs from L8 to L4, so there is no cycle; §6's table rows, §7's T8 and T13 rows, T8's and T13's "Before starting", §8 checkpoint 2 and a new §10 row follow. The friends-testers spec's §8.1 and S5 row now carry the new order, under a dated note citing the ruling. |
| I1 | important | T6's tests would reach the network and expect the wrong rows; a successful refresh would overwrite the test's cache. | §4.4: `refresh_privacy` reads the session first, and with none it answers `no session`, sends nothing and leaves the cache (a new T3 test pins this). §4.5: a successful check **adds no row**; a failed one is `privacy (check failed: <reason>)` in the summary and the bare form in the runner log. T6's new "rig" paragraph uses a loopback `api_base` that asserts it got no request, and a generated profile id whose session target is never written, so no test touches Credential Manager. The success path is T3's. T6's tests now name every row, including the failed-check row; the Unknown test asserts no cache is created. |
| I2 | important | §4.7's sign-out order could not be built: the clean deletes mail profiles (needs the hold), and its client reads the credential (needs it to still exist). | §4.7 gives the report's seven steps, with one improvement: the token for the revoke is read at step 6, not at EF `:677`, so a refresh the clean made is the token revoked. The client is `MailClient::for_session(api_base(), anon_key(), target)` with a ≤5 s per-call timeout, not `with_token_fn` over `:677`'s token, which may be past its hour and would fail every clean with a 401. The bound is `SIGN_OUT_MAIL_CALL` × (1 + records), on top of the existing `MAIL_STOP_BOUND`. T4's behaviour and tests follow, with an order test and a bound test against a silent loopback service; T4R checks both. T0 confirms the APIs. |
| I3 | important | T9's committed red tests broke "workspace green", and T10's uncommitted diff blocked rebases. | Taken as the report's first option and widened: §2's "Held diffs" keeps **T7's pins too** as a patch, because T8 now waits behind S5 while T5 and T6 run. Patches live in the ledger folder, never as a stash. The controller applies them only for the tasks that need them and reverses them before any other task's gate and any rebase, then re-applies with `--check` after. T7's pins commit inside T8's commit, and T9's inside T13's, so no committed tree is red. T7, T9, T10, T12, T13, §7's footnote, T16 check 1 and a §10 row follow. The edit also fixes T13's file count, which said "five" while counting `privacy.html` twice: it is now five named files with `engine/tests/site.rs`, six with `site.css`. |
| I4 | important | The way into Delete my data while consent is owed was not pinned; the panel could cover the gear. | §4.6: the panel carries a **Settings** button, `#rc-settings`, that calls `openSettings()`. T7's first pin requires the button and its binding; T8 builds it; the copy notes it; T15 opens Settings from it; and a §10 row records it. |
| M1 | minor | Amendment 2026-10-19 is uncommitted and names no lane PR. | T0's first step stops unless the Amendment is committed on `main`, and records its sha. Q-AGENT states both facts and makes Quinn's answer a ledger row. T16 check 6 cites that sha and that row. |
| M2 | minor | `mail (skipped: privacy…)` would show with no mailbox connected, where today there is no row. | §4.5's rule: **a step that would not have run anyway keeps today's name**. So no mailbox still means no mail row; `mail_step`'s and `grades_step`'s own earlier skips keep their names; and only a pass or capture that would have run becomes the privacy skip, with the seam never called and neither function edited. This applies the report's rule to grades and judge too, so every privacy row means "this would have run". T6's `an_older_version_names_only_steps_that_would_have_run` tests it; Q-SKIP carries the rule to Quinn; T15's expected rows follow. |
| M3 | minor | No precedence was given between skips: Older and NotEntitled together. | §4.5: the privacy block runs only for an `Entitled` account, so the entitlement's names win. A step the entitlement stops would not run whatever the student accepts, and the engine refuses its own steps past the grace. The panel still tells the student at launch. T6's `an_unentitled_account_keeps_the_entitlements_names` tests it; Q-SKIP carries it to Quinn. |
| M4 | minor | `privacy.json` also holds `account_id` and `checked_at`. | §4.4 says none of the three keys is a new data class, and why. T14's `app.md` entry names all three, with where they never go. |
| M5 | minor | T15's sign-out line should state its expected outcome. | T15 now expects, whatever the clean answered, `ok` with `setups_cleared: true` and no `pending_setups` locally. On the service, the address is either gone or still `pending` (§4.7's accepted cost); the ledger records which, and clean-up deletes it. |

**Also changed in this revision, beyond the findings.**
- The status line, and checkpoint 1's record of the round, now name this report's path; later rounds go
  beside it as `-r2`, `-r3`.
- T17 now tells L7 that its slot is open.
- §10's T4 and T6 rows count the new tests.

**Lines other plans need (not edited here; the controller or each plan's owner applies them).**
- **The L7 plan** (`docs/plans/2026-10-19-l7-named-issues-plan.md`):
  - Q9 (:429-432) names the handoff's old queue and puts T5 "after L4 (and after L6 …)". It should
    give the new queue and add "after L8's S5", since L4 now merges only after L8, and say that L7 may
    take its slot ahead of L4 while L4's T13 waits for L8 (this plan's Q-QUEUE).
  - The `console.js` row (:448) should read L6 → L8 S5 → L4 → **L7** → #19 B5 → L1 R11 → L5.
  - T5's "Before starting" (:721) should read "Rebase onto whatever L6, L8 and L4 merged".
- **The L5 plan** (`docs/plans/2026-10-19-l5-syllabus-class-times-plan.md`):
  - the queue bullet (:165-166) should read L6 → L8 S5 → L4 → L7 → #19 (B5) → L1 R11 → L5 → the
    redesign;
  - the `app/static/*` row (:304) should name L8 S5 among the lanes ahead;
  - the ledger row (:846) should give the same queue and cite Quinn's ruling of 2026-10-19.
  - Its T11a start condition (:747) and PR-2 line (:229) need no change.
- **Also stale, for whoever owns them:**
  - the L6 plan's queue lines (:385, :444; :444's "or right after L4" no longer fits);
  - the handoff's §5 queue line;
  - the syllabus spec (:1324), the p3 close-out plan (:687) and the page draft (:382). Those three
    are history; each wants at most a dated pointer to this ruling.

### Round 2 (2026-10-19): `docs/reports/2026-10-19-l4-plan-review-r2.md`, FAIL (0 critical, 2 important, 6 minor)

The report closed C1, I3, I4 and M1-M5 of round 1. Round 1's I2 answer above names
`MailClient::for_session`; that choice is superseded by I-1's answer below and is kept as history.
The report's note on the L7 and L5 queue edits is outside this revision (the controller's).

| # | Sev. | Finding | Answer |
|---|---|---|---|
| I-1 | important | The clean's time bound left out `for_session`'s token step: `valid_access_token_at` waits for `SESSION_REFRESH_LOCK` without limit and refreshes over the account agent's 30 s. | **Quinn's decision, 2026-10-19: the capped token function.** §4.7 replaces `for_session` with a new `sign_out_mail_client(api_base, anon, target, per_call)` over `MailClient::with_token_fn`. Its token function calls a new `valid_access_token_within(..., within)` once per clean. That function takes the lock by `try_lock` until `within` has passed, and refreshes over whatever is left of `within`. §4.7 also checks Quinn's condition on EF: `valid_access_token_at`, `ensure_session_for_at`, `refresh_at`, `post_json` and `agent()` keep their behaviour. The new function holds the lock at most 5 s, under the 30 s that callers already wait. The rotation hazard of a timed-out refresh already exists at 30 s and ends at step 6's delete. So the documented-bound alternative was not needed. **The worst case** is stated in §4.7: 60 s (the hold) + 5 s (the one session read) + 5 s × (1 + n) (the list and deletes) + 30 s (the revoke) = 60 + 5 × (2 + n) + 30 s. `SIGN_OUT_MAIL_CALL`'s doc line states it (T4). T4 adds a refresh-hang bound test, the helper's three tests and four source pins. T4R checks the bound four ways, and T0 confirms the APIs and constants it relies on. A new §10 row. |
| I-2 | important | T4's bound test would make step 7 revoke against a silent `KNOWLU_API_BASE` (30 s), and the file's tripwire requires `ApiBase` in every test reaching `api_base()`. | As the report says: the builder takes `(api_base, anon, target, timeout)` (§4.7; T4 behaviour). Every new test that calls `sign_out_with` sets `ApiBase::set(&closed_loopback_base())`, so the revoke is refused at once and the tripwire at EF `tests/account.rs:954-965` holds. The order, failed-clean and pending-session tests are included. The bound tests pass the silent listener's base to the builder directly. The existing test at `:2068` already sets the seam (`:2070`). §10 row. |
| M-1 | minor | T6's rig does read Credential Manager. | T6's rig now says so: the session step, `refresh_privacy`, `refresh_entitlement` (M3 test) and telemetry read `cloud.yaml`'s `session_credential_target`. Those are reads of a generated, never-written target, the exemption EF `tests/scheduler.rs:630-634` records, so no `CREDMAN_LOCK` is taken. The listener counts every request. §4.4 and T3's no-session test now name the vault's `session_credential_target`, not the profile's. |
| M-2 | minor | T6's expected rows left out the session row. | The Older, Accepted and Unknown tests name `session (refresh failed: …)` first (EF `scheduler.rs:901`, matched by prefix) and say "exactly one privacy row". The runner-log test expects today's bare `session (refresh failed)` beside the privacy lines. |
| M-3 | minor | The grades skip had no stated mechanism. | §4.5's grades bullet: when the seam has a capture, `run_slot_with` passes a stand-in. The stand-in records "would have captured" and returns `Err(CaptureError::Busy)`, never `SignedOut`, with no side effect. `run_slot_with` then replaces the row; with no capture, `None` passes as today. T6's Older test asserts the row is privacy's and no signed-out mark was written; T0 confirms `Busy`. |
| M-4 | minor | No owner for a conflict in `app/tests/static_assets.rs` with L8's S5t. | "Ahead of T7" is dropped. §6's row says S5t reaches L4 at the rebase before T13, and a conflict there goes to `test-writer`. It keeps S5t's tests and T7's pins whole and edits neither. §7's T7 row and T13's "Before starting" say the same, and a §10 row records it. |
| M-5 | minor | The rebased tree was not checked before T13. | T13's "Before starting": with the patches reversed, the controller runs the workspace gate, both console checks and the panel screenshot on the rebased tree. A red result goes to the file's owner. Only then are the patches re-applied with `--check`. A §10 row records it. |
| M-6 | minor | Step 1 may give no hold. | §4.7 step 1: `None` comes for another profile's console (EF `account.rs:650-652`) or a timeout (`:1518-1519`). **Steps 2-5 still run**, as today's `forget_all` does without a hold (`:683-685`). A profile still held is reported in `mail_profiles_left`. |

**Also changed in this revision.** The status line and checkpoint 1 name the round-2 report. §6's
"Not touched" adds `app/src/mail_client.rs`, which T4 uses through its `pub` API and does not edit.

### Round 3 (2026-10-19): `docs/reports/2026-10-19-l4-plan-review-r3.md`, FAIL (0 critical, 1 important, 4 minor)

The report closed every round-2 finding. Every answer below is a text edit; none reopens a design choice.

| # | Sev. | Finding | Answer |
|---|---|---|---|
| I-1 | important | The helper's lock-wait cap had no test that could fail: only a source pin, which a deadline-free `try_lock` loop passes. | T4 adds `valid_access_token_within_gives_up_on_a_held_lock` as the report gives it. Thread A holds the lock through a refresh against a silent listener. The helper, with `within = 200 ms` on a second target, must answer `Err` within 1.2 s with zero requests and that session unchanged. A's stream is then dropped and A joined. T4R's first bound check names it beside the refresh-hang test, and the I-1 row of §10 lists it. |
| m-1 | minor | §4.7's wording on `Poisoned`, waiters and token reuse. | `Poisoned(p)` means acquired, via `into_inner()`; only `WouldBlock` means held. "No other caller waits longer than today" becomes: each new hold is at most 5 s plus local Credential Manager work, once per sign-out, and a waiter may wait that much more, never a refresh's 30 s more (std's unfair SRWLock). "No later caller reuses the old token" becomes the reuse-interval reasoning: another thread may reuse it between steps 2 and 6, and GoTrue answers or revokes the family, which costs a student who is signing out nothing. |
| m-2 | minor | The bound left out `wipe`, and the 401 arithmetic was off by one. | §4.7 adds the local term: up to about 2.75 s per held mail profile (12 tries, 11 waits of 250 ms, EF `mail_session.rs:31-32`, `:301-311`), in the clean's `forget` and in `forget_all`. `SIGN_OUT_MAIL_CALL`'s doc line, its source pin, T4R and §10 carry it. The 401 case is now n ≥ 24, and D3's caps (three live addresses, six mints per 30 days) mean it cannot occur. |
| m-3 | minor | The once-per-clean memo was unpinned; the doc lacked its reason; the test base was not spelled out. | §4.7: the memo is a `std::sync::OnceLock<Result<String, String>>` (`TokenFn` is `Fn + Send + Sync`, EF `mail_client.rs:151`); without it the clean is 10 × (1 + n) s under contention (corrected in round 4). `sign_out_mail_client`'s doc comment says why it keeps a token for one clean, against `mail_client.rs`'s "never kept" module doc (`:7-9`). T4 adds a source pin (`OnceLock`, the helper called only in `get_or_init`) and spells the bound test's base `http://127.0.0.1:<port>/functions/v1`. |
| m-4 | minor | §4.7's opening said the client reads the session on every request. | It now says the clean's client reads it once, at the clean's first request, through `valid_access_token_within`. The order argument is unchanged. |

**Also changed.** The status line now points round 4 at `…-r4.md`, and checkpoint 1 lists all three reports.

### Round 4 (2026-10-19): `docs/reports/2026-10-19-l4-plan-review-r4.md`, PASS (0 critical or important, 2 minor, 2 nits)

All four applied in place; no further round follows.

| # | Sev. | Finding | Answer |
|---|---|---|---|
| m-1 | minor | The held-lock test would pass a helper that tries once and gives up at once. | The test also asserts the helper's elapsed time is at least `within` (200 ms), beside the under-1.2 s bound. |
| m-2 | minor | The held-lock test's rig needed four details. | Added: A's connection is awaited against a 10 s deadline, as `loopback` does (EF `tests/account.rs:19-28`); the accepted stream moves to the test thread and lives as a local in the `thread::scope` closure until the helper returns, so a failed assert drops it while unwinding; only the helper's call is timed; the test is `#[cfg(windows)]`, like the race test at `:658`. |
| nit | nit | With no memo the clean costs 10 × (1 + n) s, not 5 + 10 × (1 + n) s. | Corrected in §4.7 and in round 3's m-3 row. |
| nit | nit | The 401 case is unreachable for a simpler reason: deletes go only to rows the service lists as `pending` (EF `mail/setup.rs:415`), which are live addresses, at most three. | §4.7 says so: at most 1 + 3 requests whatever the record count, a network worst case of at most 115 s, and no delete can meet an expired token. `n` is now the number of deletes, in §4.7 and `SIGN_OUT_MAIL_CALL`'s doc line in T4. |

**Also changed.** The status line reads "final (r4 review PASS, minors applied)", and checkpoint 1 records the pass.

### Whole-lane review T16 (2026-10-21): round 1 FAIL (3 important, 4 minor), contract PASS (4 minor); round 2 PASS, contract PASS

Rulings are the controller's under Quinn's standing go of 2026-10-20 (Quinn-delegated); the lane ledger holds the rows.

| # | Sev. | Finding | Answer |
|---|---|---|---|
| I1 | important | The panel said Knowlu "does not sync, import or read anything new", but `rank` (never gated) still refreshes the student's Google Calendar through the cloud. | The copy now reads "Until you accept, Knowlu keeps today's page and your calendar up to date, but does not sync, import or read anything else new." (cf892e7). T8R's "code true to the copy" stands for every path that can be gated. §4's quotation above keeps the round-4 wording as history. |
| I2 | important | `app/src/mail/live.rs`, `app/src/state.rs`, `app/tests/mail.rs` and `app/tests/mail_window.rs` changed outside §6's list; `on_start`'s resume is L1's territory. | Accepted. The session ledger tells L1 that the launch resume now checks (and, after 5b63855, refreshes) the gate before `open_backfills`, with the order pin in `app/tests/mail.rs`, and that the C3 plan's R9d no longer finds nothing to rebase over. |
| I3 | important | `app/tests/static_assets.rs`'s import check was narrowed in 0cd05bf with no task naming it. | Ruled in, then restored to full strength: strings and comments are stripped, and `import` is refused anywhere (cf892e7). |
| m4 | minor | The page text was committed before its version (be92709, c51c381, 62310ff) and changed after the bump (114e38c). | Recorded. Nothing was released and the date did not move; the release deploys `site/` from `main` at the merge, never from an earlier commit. |
| m5 | minor | The first launch after an upgrade has no `privacy.json`, so the mail resume read `Unknown` and resumed. | The resume asks the service when the cache is not `Accepted` (5b63855). |
| m6 | minor | Grades, Google-connection and Gmail §8.5 sentences had no pin. | Pinned in `engine/tests/site.rs` (14528d1). |
| m7 | minor | The account route took any future date as a version. | Refused after tomorrow (UTC) (7ce8a5d). A version is therefore the day its page is published, never a later effective date. |
| C-M1 | minor | A failed `privacy.json` write discarded the service's answer; the save fallback deleted the old file first. | The gate comes from the reply and the save error is named separately; the save never deletes (9cebf5b). |
| C-M2, C-M3 | minor | Sign-out's credential delete waits behind the mail hold and clean; `setups_cleared` can be true before a late `note_pending`. | Accepted costs, as §4.7 already accepts. |
| r2 minors | minor | Round 2: a failed save is visible only in a debug log and `cache_error`, which the console does not read; the regex-quote case in the import check's stripper; one test cannot see a third request; this plan still quotes the old panel line. | Recorded. A named scheduler row for a failed save is a follow-up, not this lane's. |
