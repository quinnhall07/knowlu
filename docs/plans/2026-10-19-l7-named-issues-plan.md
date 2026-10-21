# Named issues (lane L7): build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: final (r3b delta check PASS, minors applied;
`docs/reports/2026-10-19-l7-named-issues-plan-review-r3b.md`).** Round 1 FAILED (0 critical, 4 important,
6 minor; `docs/reports/2026-10-19-l7-named-issues-plan-review.md`). The round-2 re-review PASSED with
4 minor findings (`docs/reports/2026-10-19-l7-named-issues-plan-review-r2.md`). r3 records Quinn's
rulings on Q1-Q3 (2026-10-19) and reworks the Gmail rule for Q3: 7 days, one list call (§11, Round 3).
The r3 delta review FAILED with 0 critical, 2 important and 5 minor findings
(`docs/reports/2026-10-19-l7-named-issues-plan-review-r3.md`); every finding is applied in place
(§11, Round 3 delta). The r3b delta check PASSED with 2 minors, applied in place (T2b test 5b; T0's
marker step). Nothing runs until the controller has answers to §4's remaining open questions (Q4,
Q6-Q10), or has deferred them.

**Date:** 2026-10-19 (session 5, the friends-MVP push). **Lane:** L7 of
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5: "OpenRouter 402 (out of credit) and
Gmail 'source went quiet' surface as named issues". It starts after the email merge, touches the engine's
issue surface and `app/static`, and runs with `implementer` and `console-ui`. This plan adds
`cloud-engineer` (T2a, T2b) and `test-writer` (T1), for the reasons §6 gives.

**Authority, newest first.**
- Handoff §1.2 (2026-10-18): moves "the OpenRouter-402 / Gmail-quiet issues" into the MVP.
- `HANDOFF.md` §3 (Pilot list, now MVP):
  - "OpenRouter's 402 (payment required) as a named issue instead of quiet model failures";
  - "source went quiet" (`google_accounts.status = 'quiet'` after 14 days): "modelled server-side,
    nothing shows it yet; the Issues panel is its home". The 14 days are superseded by Quinn's Q3
    ruling (next bullet); T6 updates this line.
- **Quinn's rulings of 2026-10-19 on Q1-Q3** (§4): the Issues view only; the copy as written, with no
  2-day expiry; and "quiet" means no message of any kind for **7 days**, not 14. Q3 amends cloud design
  §5.3, through an inline marker on its Campus-accounts bullet, in the spec's own lowercase form:
  *(amended 2026-10-19, Quinn (L7 Q3, `docs/plans/2026-10-19-l7-named-issues-plan.md` §4): quiet = no
  message of any kind for 7 days, first firing on day 7)*. The controller commits that marker on its
  own, docs-only, before T0 (T0; round 3 delta, important 2).
- The cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`), Amendment 2026-09-29, ruling 10,
  as amended 2026-09-30: "the OpenRouter 402 and Gmail's 'source went quiet' shown as issues (email
  forwarding's is the MVP's)". §5.3 and the Campus-accounts bullet define the Gmail case: "shows
  *source went quiet* if a connected account stops yielding for 14 days", amended to 7 days by Q3.
- The Gmail connect spec (`docs/specs/2026-09-29-gmail-connect-design.md`):
  - §4.4, the `quiet` row: "Same as `active`. The Pilot's 'source went quiet' issue owns this state";
  - §10 lists it as out of scope there.
- The email forwarding spec (`docs/specs/2026-09-30-email-forwarding-design.md`):
  - D14: "Each non-active state is a named line in the run and in Settings, and an issue in the Issues
    panel".
  - §4.10 *Shown*: one `heads-up` info item per non-active mailbox, `close_key: mail-source:<address_id>`.

There is no separate spec. The rulings above name what is shown; this plan names how. §4 asked Quinn
the open design choices (where the issues live, the copy, and what "quiet" means), and Quinn ruled on
all three on 2026-10-19.

**Goal.** Two situations stop being silent:
1. **The judgment service is out of credit.** OpenRouter answers HTTP 402 when Knowlu's own balance is
   spent. Today the student sees a column of per-item `model failed` lines in the run, the same words an
   outage produces.
2. **A connected Gmail has gone quiet.** Knowlu has found no mail in a connected Gmail for 7 days.
   Today nothing computes this state.

After this lane, each situation is:
- one named line in the run;
- one open `heads-up` item in the vault, kept while the situation lasts and closed when it clears;
- a row in the console's Issues view.

Gmail's Settings row also says so. Email forwarding's own mailbox heads-ups (D14) join the Issues view
in the same change.

**Architecture: four pieces, no migration, no new Tauri command.**
1. **Cloud (T2a, then T2b).**
   - T2a: a provider 402 becomes a typed error. `judge_pipeline.ts` answers it with
     `unavailable: "out of credit"` beside the existing `cause: "model failed"`, so an older engine
     behaves exactly as it does today.
   - T2b: `/gmail-read` computes "went quiet" from the 7-day list it already makes (the raw list is
     empty and the grant is at least 7 days old) and answers `went_quiet: true|false`. No new Gmail
     call. It also moves `google_accounts.status` between `active` and `quiet`.
2. **Engine (T3a, T3b, and T3c if L7 lands after L3).**
   - T3a: `judge` reads `unavailable` as a global stop, like the cap. One small new module,
     `engine/src/source_issues.rs`, owns the two keys and the keep-or-clear rule.
   - T3b: `judge` prints one summary line and keeps or clears one heads-up. `pull_gmail` does the same
     for `went_quiet`.
   - T3c: the events pass ends at the first out-of-credit reply, as L3's T3 ends it at the cap.
3. **Read model (T4).** `surface::issues_panel` gains `sources`: the open heads-ups that name a source
   in trouble. `good_to_know` leaves them out. `sources` is omitted from the JSON when it is empty, so
   the three `surface-today-*.json` references stay byte-identical.
4. **Page (T5).** The Issues view renders `sources`. The Settings Google row gets its `quiet` sentence.

**Code read for this plan:** the email-forwarding worktree
(`C:\Users\danie\GitHub\knowlu\.claude\worktrees\email-forwarding`), which is what `main` becomes at the
merge. Also read: the L6, L3, L5 and #19 plans for overlaps. `file:line` values are that tree's
("EF"). Re-confirm each one on the lane branch, because L6 may have merged first (§5).

## 1. VISION and the two overriding rules

- **"Failures are visible; silence is never ambiguous"** and **"a source that breaks later fails
  visibly rather than going quiet"** (VISION, Standing rules; Where it gets its information). This
  lane exists for these two sentences.
  - An out-of-credit service is not the student's fault and not an outage. Reading it as `model failed`
    is the ambiguity this lane removes.
  - A Gmail grant that has stopped finding mail looks exactly like a light week of tasks. A rule on
    *any* message over 7 days tells them apart (Q3): a student can go a week without a task in their
    mail, but not without mail.
- **Commitment 5's exception text:** "a notice that changes nothing in the plan (a room, office hours, a
  mailbox gone quiet) is shown, not proposed". So both issues are info items (shown), never approval
  cards (proposed). They do not count toward the 15-a-day cap.
- **Commitment 1, "`rank` never calls a model":** unchanged. The new behaviour lives in `judge` (a
  reply it already receives) and in `surface` (a read of `info/`). `rank` reads nothing new.
- **"The engine is deterministic":** holds.
  - Whether a heads-up opens or closes is a function of the service's reply and the vault.
  - The read model lists `info/` in path order, as it already does.
  - No clock is added, apart from the server's 7-day comparison on `connected_at`, which is
    server-side and arrives as data.
- **"Every vault write goes through `write`":** holds. The heads-ups use the same path as the mail
  heads-ups (`mail/heads.rs`):
  - opening uses `info::open_info`, then `write::create`, journal first;
  - closing uses `info::close_info`, then `write::write` and `write::delete`.
  - No note is parsed and re-dumped.
- **`judge` and `sync` always exit 0.** An out-of-credit reply is a named outcome, like a spent cap
  (`judge::CAPPED_LABEL`). A heads-up that fails to write is one more line, never a non-zero exit.
  `sync` is untouched.
- **Fetch on device, think in the cloud.** The 7-day rule is evaluated server-side, where the Gmail
  read happens. The device only shows the answer.
- **Rule 1 (no single-user assumptions).** Holds.
  - No address, account, school or person is named in code, tests or copy.
  - The heads-up keys are fixed words, not per-user values. The Gmail one carries no address: an
    account has at most one `google_accounts` row.
- **Rule 2 (never regenerate a frozen reference).** Holds.
  - No file under `engine/tests/fixtures/**` is opened for writing.
  - The three Rust-generated `surface-today-*.json` references stay byte-identical, because `sources`
    is `skip_serializing_if = "Vec::is_empty"` and no reference vault holds a source heads-up. T0
    checks this by grep. T7 checks `git diff --exit-code main -- engine/tests/fixtures` and that
    `engine/tests/surface_oracle.rs` passes untouched.
- **Contract list: no file on it is edited.** Checked file by file against CLAUDE.md's list:
  - The engine files this lane edits are off the list: `judge.rs`, `cloudmodel.rs`, `enrich.rs`,
    `surface.rs`, `lib.rs` and the new `source_issues.rs`.
  - `info.rs` is only called, never edited. `write.rs`, `journal.rs`, `yamlemit.rs`, `ids.rs`,
    `approvals.rs` and `sync.rs` are only reached through existing calls.
  - No `app/src` file is edited, so `credentials.rs`, `account.rs` and `updates.rs` are untouched.
  - The oracle, sync and entitlement tests are untouched. `surface_oracle.rs` runs, and is not edited.
  - `engine/tests/fixtures/**` is untouched.
  - `engine/tests/cloud_contract.rs`, whose status L3 (Q-B) and L6 (Q9) both ask about, is **not**
    edited. L7's integration tests go in a new file (T1), so this lane adds nothing to that question.
- **Vault contracts.**
  - `info::KINDS` is unchanged: the heads-ups use the existing kind `heads-up`.
  - The info note's frontmatter shape is `open_info`'s, unchanged.
  - The two new `close_key` values are new data in an existing field.
  - No journal record shape, run-record shape, `journal::VIAS` entry or ledger changes.
- **Agent actors.** The out-of-credit heads-up is written under the actor `judge` already writes notes
  as (`agent:knowlu.enrich`, CLAUDE.md). The Gmail one is written under `enrich::GMAIL_ACTOR`
  (`agent:knowlu.gmail`). Both start `agent:`, so `provenance::is_agent` reads them as agents.
- **Privacy.** Nothing new leaves the device. Nothing new is stored server-side beyond what the schema
  already holds:
  - the only new server write is `google_accounts.status` moving between `active` and `quiet`, an
    already allowed value (`20260911000200_google.sql:25`);
  - `google_accounts.last_read_at` stays unwritten (round 1, finding 2: the 7-day rule reads Gmail's
    own list, not a stamp);
  - no new Gmail call is made: the rule reads the 7-day list `/gmail-read` already makes (§3.3).
  - The heads-up notes carry fixed sentences, never mail text, an address or a model's words. They sync
    like every `info/` note.
  - Telemetry is unchanged. L4 (the privacy page) needs no new line.
- **Money, releases, rules.** No billing or entitlement change. No release, and no CLAUDE.md or VISION
  edit.
  - The staging deploy of the changed functions is the controller's (T8).
  - The prod deploy rides L9, with Quinn's go.
  - **CI's `eval-gate` runs on this PR.** T2a edits `judge_pipeline.ts`, which is in the gate's
    touched-check (`ci.yml:242`). So the PR runs `cloud/eval/run_eval.ts` with the staging secrets:
    - it fails loud if staging cannot answer its `eval_cases` count (`ci.yml:190-192`), by design;
    - with `eval_cases` empty it scores nothing and spends nothing;
    - with rows in `eval_cases` it makes real model calls, bounded by `MAX_CASES` at 1,800 calls,
      "well under a dollar" (`ci.yml:205-208`).
    - T8 counts `eval_cases` on staging before the push. A non-zero count is Q10, put to Quinn before
      the push. L3's PR, which also edits `judge_pipeline.ts`, carries the same note (L3 plan, T2b's
      CI note).

## 2. Global constraints

- **Branch and worktree.** Branch `l7-named-issues`, worktree
  `C:\Users\danie\GitHub\knowlu\.claude\worktrees\l7-named-issues`. The controller cuts both from `main`
  **after L6 merges** (Q8, the default since round 1), which is after the email-forwarding merge
  (handoff §5). Use absolute paths and never `cd`.
- **Agents never commit, push or branch.** Each task ends with a file list and a commit message. The
  controller commits with `git add <paths>` (never `-A`) and `git commit -F <file>`.
- **Cargo gates.** Run against the worktree, dev profile, in the foreground, and only under the
  build-slot guard (two cargo runs machine-wide):
  `bash C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test --workspace --manifest-path C:/Users/danie/GitHub/knowlu/.claude/worktrees/l7-named-issues/Cargo.toml`
  (and the same with `cargo build`). Build the engine first (`cargo build -p knowlu-engine`, the
  zero-byte sidecar trap in `docs/reference/toolchain.md`). **0 warnings is part of green.** The one
  accepted line is the app's `.rsrc merge failure: multiple non-default manifests`. Also run
  `scripts/ci/eol-check.ps1`.
- **Deno gates** (T2a, T2b, T7, T8), the CI lines with the worktree's absolute paths (`W` below stands
  for `C:/Users/danie/GitHub/knowlu/.claude/worktrees/l7-named-issues`):
  - `deno check --config W/cloud/supabase/deno.json W/cloud/supabase/functions/**/*.ts W/cloud/eval/*.ts`
  - `deno lint --config W/cloud/supabase/deno.json W/cloud/supabase/ W/cloud/eval/`
  - `deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config W/cloud/supabase/deno.json W/cloud/supabase/`
    (CI's `cloud` line without `cloud/eval`.)
  - CI's full test line, which T2a, T2b and T8 each run once because the eval imports the adapter. It
    adds `--allow-write=W/cloud/eval` (the eval's loader tests build temp fixtures under `cloud/eval/`,
    `ci.yml:162-167`) and the second path `W/cloud/eval/`:
    `deno test --allow-read --allow-write=W/cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config W/cloud/supabase/deno.json W/cloud/supabase/ W/cloud/eval/`
  - If the agent's shell does not expand the `**` glob, it passes the changed files and their
    importers by name, and the controller runs the CI line at T8.
- **Tests never leave the machine.**
  - Engine tests reach the "service" through a listener on `127.0.0.1:0`: `enrich.rs`'s in-module
    `multi_reply_loopback`, or a copy of its shape in the new test file.
  - Deno tests inject `fetchImpl` and fake stores, as `judge_openrouter_test.ts` and
    `gmail-read/handler_test.ts` already do.
  - No test resolves a name or reaches staging.
- **No Credential Manager.** No test touches the real store. Every engine test builds its
  `CloudClient` straight from a config and a token (`CloudClient::new`), as the existing cloud-arm tests
  do.
- **LF everywhere.** `engine/tests/fixtures/**` is untouched.
- **No secret, account id, email, school or person's name** appears in code, tests, comments, copy or
  test names. Test vaults use invented course names. Test Gmail addresses use `.example.invalid`, as
  the existing tests do.
- **TDD.** Every behaviour is a failing test first, run and seen red for the stated reason:
  - T1 for the engine and the page, and T3c's own first step for the events pass;
  - T2a's and T2b's first steps for the cloud.
  - A red test must fail on behaviour, not on compilation. T1's tests use only items that exist when
    T1 runs: string literals for the keys, and JSON for the read model (§6, T1).
- **`commands.rs` computes nothing, and no Tauri command is added.** The Issues view reads `state`,
  which every command already returns. The Settings row reads `google_status`, which exists. So L7
  never waits on L6's single ownership of `main.rs` and `commands.rs` (L6 plan §5.1 item 3).
- **The old-engine and old-server pairs keep working.** Deploy order is free.
  - The new server's keys (`unavailable`, `went_quiet`) are ignored by today's engine.
  - Today's server sends neither key, and the new engine then behaves exactly as today.
  - T2a/T2b and T3a/T3b each pin their half.

## 3. Behaviour

### 3.1 What happens today (EF)

- **A provider 402.**
  1. `OpenRouterModel.complete` throws a plain `Error("the model service answered HTTP 402")`
     (`_shared/judge_openrouter.ts:86-89`).
  2. `judge_pipeline.ts` maps every plain error without `max_tokens` in its message to
     `cause: "model failed"`, refunds the charge and logs the row (`:229-262`).
  3. The reply is HTTP 200: `{verdict: null, tier: 3, outcome: "low confidence", cause: "model failed"}`.
  4. On the device, `CloudModel::verdict_of` turns that into
     `ModelError::Failed("the judgment service answered model failed")` (`cloudmodel.rs:420-435`), one
     item at a time. `judge_event` keeps it retryable (`:545-552`).
  - **The trap:** HTTP 402 *to the device* already means "no entitlement" (`CloudError::label`,
    `cloudmodel.rs:223`; the sync line's `SYNC_SAYS`). The provider's 402 must never reach the device as
    an HTTP 402. Today it does not, and T2a pins that it never will.
- **Gmail.** `google_accounts.last_read_at` and `status = 'quiet'` exist, and nothing writes either
  (grep: the migration is the only hit). `/gmail-read` uses the key `quiet: true` with a `reason` for
  *not connected* (`no_gmail_scope`, `revoked`; `gmail-read/handler.ts:321-334`). The device reads
  every unknown reason as "re-connect from settings" (`cloudmodel.rs:677-685`). **That key and its
  reasons must not be reused for "went quiet".**
- **Forwarded mailboxes.** These already have heads-ups (`engine/src/mail/heads.rs`, keys
  `mail-source:<address_id>` and `mail-reenabled:<address_id>`). They show in *Good to know*
  (`surface::good_to_know`, `surface.rs:1598`), not in the Issues panel D14 names.

### 3.2 Cloud: out of credit (T2a)

- A provider HTTP 402 throws a **typed** error, `ProviderOutOfCredit`. It is defined beside
  `ModelRefused` in `judge_anthropic.ts`, which `judge_pipeline.ts` already imports from, so there is
  no import cycle.
  - It extends `Error`, and its message names the status only, never the body.
  - The pipeline tells it apart with `instanceof`, not by matching a message.
- `judge_pipeline.ts`'s catch:
  - The row is logged with `cause: "model failed"`. That is true (the model never answered), and it
    keeps `judgments.cause`'s check constraint (`20260911000100_judgment_service.sql:74`) as it is, so
    no migration.
  - The charge is refunded, as for every `model failed`.
  - The reply adds one key, `unavailable: "out of credit"`. `JudgeReply` gains
    `unavailable?: "out of credit"`, a closed set of one word for now.
  - No other reply key changes.
- `judge_handler.ts` still answers HTTP 200 for it. The handler test pins that a provider 402 never
  becomes a device-facing 402.
- `/gmail-read` (Q6): a reply carrying `unavailable` stops the round, as `capped` already does
  (`handler.ts:407-411`). The item is deferred, not marked seen. Nothing new reaches the device on this
  path.
- `mail-process` and `cloud/eval/run_eval.ts` change no behaviour:
  - `mail-process` relies on the existing breaker (three failures).
  - The eval treats the new class as a failed attempt, as it treats any thrown error.
  - T2a confirms both by reading the code, and records the answer.

### 3.3 Cloud: Gmail went quiet (T2b)

- **Definition** (Q3; Quinn ruled 2026-10-19, amending cloud design §5.3): a connected Gmail has
  **gone quiet** when Gmail's own list, with Knowlu's excluded labels kept, holds no message from the
  last **7 days**, and the grant itself is at least 7 days old. "Yield" means *any* message, not *a
  task*: a student can go a week without a task in their mail, but not without mail. Dedup plays no
  part: an already-seen message still counts.
- **The list already exists.** `WINDOW` is `newer_than:7d` (EF `handler.ts:37`, pinned by
  `handler_test.ts:276`), and `query()` puts it first, ahead of the `-label:` terms (`handler.ts:240`).
  So the rule reads the one list call the handler already makes. There is no second list call, no new
  query and no new Gmail cost.
  - Not this rule: the `14 * 86_400_000` in `gmail-read/index.ts`'s `seen` (EF `:124-131`) is the
    `gmail_seen` dedup window's lookback, twice `WINDOW`. It stays as it is (round 3 delta, minor 7).
- **Why Gmail's list and not a stamp** (round 1, finding 2). A "last read that found mail" stamp would
  lag the window by its own length. Reading the 7-day list itself makes the rule fire on the first read
  more than 7 days after the last message, and, for a grant that has had no mail since connect, on the
  first read on or after day 7 after connect. That is the amended §5.3: "no message of any kind for 7
  days, first firing on day 7".
- **The grant's age.** `connected_at`. `store_google_grant` already resets `connected_at` and
  `status = 'active'` on every connect and reconnect, the incremental Gmail-scope grant included
  (live definition `20260911000300_google_privileges.sql:58-66`, `connected_at = now()` at `:62`; it
  replaces the first one in `20260911000200_google.sql`), so a fresh connect never reads as quiet.
  `last_read_at` is not read or written.
- **On each `/gmail-read` that reaches the 7-day list** (a grant with the Gmail scope, not the
  forwarding path), the check reads the **raw** result of `deps.api.list`, before `.filter(unseen)`
  and the `READ_CAP` slice (EF `handler.ts:351-353`; round 2, minor 1). The handler keeps the raw
  array under its own name; the filter-then-slice that feeds the judging loop is unchanged. An
  already-seen id is still mail (the definition above):
  - The raw list holds at least one id, seen or not: move a `quiet` row to `active`, answer
    `went_quiet: false`.
  - The raw list is empty and `connected_at` is 7 days old or more (`deps.clock()` minus
    `connected_at`, compared in milliseconds against `7 * 86_400_000`, with no rounding to days, so
    exactly 7 days counts and 1 ms less does not): move `active` to `quiet`, answer
    `went_quiet: true`.
  - **The answer is the rule, never the write's effect** (round 3 delta, important 1). A row that is
    already `quiet` still answers `went_quiet: true` on every read the rule holds, though its
    conditional `active`→`quiet` write changes nothing. An answer taken from "did the write change a
    row" would flip to `false` on the next slot, and the engine's heads-up would close and reopen on
    alternate slots.
  - The raw list is empty and the grant is younger: no status write, answer `went_quiet: false`.
  - The list call throws: nothing new happens. Today's outer catch answers HTTP 500 `read failed` and
    logs its one class-and-status line (EF `handler.ts:423-431`). The check runs only after the list
    returns, so a failed list writes no status and claims neither answer. No new logging is added.
  - **When the write runs** (round 3 delta, minor 5; the controller's ruling): the status write goes
    straight after `deps.api.list` returns, **before** the judging loop. So a throw later in the loop
    cannot leave a row whose list held mail marked `quiet`. The write is not caught on its own: a
    failed write reaches the outer catch and is HTTP 500 `read failed`, as a failed `markSeen` or
    `enqueue` already is.
  - Every status write is conditional (`status=eq.active` or `status=eq.quiet`), so it never touches a
    `revoked` row.
- **Testing mode (an observation, not a change; §11 Round 3 and its delta).** Until `gmail.readonly`
  is verified, Google's Testing mode gives refresh tokens that die 7 days after consent (cloud design
  §5.3), and a dead token takes the `revoked` path (`handler.ts:328-334`). `connected_at` is stamped
  after the code exchange (`20260911000300_google_privileges.sql:62`), so the token dies before
  `connected_at` + 7 days, and **the rule cannot fire in Testing mode**. If timing ever let one read
  through, the worst case is one slot of "gone quiet": the next read takes the `revoked` path, and the
  engine's `Quiet(Revoked)` arm clears the heads-up (§3.4). The 14-day rule could not fire either. The
  rule is built and tested in full now, and becomes live at verification (Launch).
- **On the forwarding path** (D22, `handler.ts:307-311`), forwarding owns that mailbox's silence (the
  canary, D14). The handler moves a `quiet` row back to `active` and answers `went_quiet: false`.
- **On the `no_gmail_scope`, `revoked` and not-configured paths:** no `went_quiet` key, and no status
  change beyond today's `markRevoked`.
- **The `quiet: true` key keeps its one meaning (not connected).** A test pins that a went-quiet reply
  has `quiet: false` and no `reason`.
- **Deps.** `ReadDeps` gains what the handler needs: the row's `connected_at` (a new dep, or a field on
  the row `lookupToken` already reads) and the two conditional moves. No stamp writer. `gmail-read/index.ts`
  wires them through `sharedDb()`, as `markRevoked` is wired (`index.ts:121`). The names are
  cloud-engineer's choice.
- **Settings.** `google-connect`'s status already answers the row's `status`
  (`google-connect/handler.ts:70`), so the Settings row sees `quiet` with no change there.

### 3.4 Engine (T3a: `source_issues.rs`, `judge.rs`, `cloudmodel.rs`; T3b: `enrich.rs`; T3c: `events.rs`)

- **`engine/src/source_issues.rs` (new, small, no I/O of its own beyond `info`):**
  - Two key constants: `judge-service:out-of-credit` and `gmail-source:quiet`.
  - The prefix list a source issue is recognised by: `mail-source:`, `judge-service:`,
    `gmail-source:`. `mail-reenabled:` is deliberately absent: it is an announcement, not trouble.
  - `is_source_issue(kind, close_key)`: true for kind `heads-up` with a listed prefix.
  - **raise**: opens one `heads-up` with the given key, title and sentence, but only when no open
    `info/` item carries that key. It never duplicates and never re-opens a twin.
  - **clear**: closes every open item with the key (`info::close_info`). Clearing a key with no open
    item writes nothing.
  - Both go through the caller's `WriteContext` and `Journal`. A failure is returned, never swallowed.
- **`judge.rs`:**
  - `ModelError` gains `OutOfCredit`, and a closed-set label `OUT_OF_CREDIT_LABEL` sits beside
    `CAPPED_LABEL`.
  - `judge_task` maps `OutOfCredit` to `Outcome::ServiceUnavailable(seed)`. So tier 1 is still written,
    the flag stays set, and the judgment log records `service unavailable`. No new `Outcome` or
    `LowCause` variant.
  - Nothing L6 deletes or L3 reads is renamed (§5).
- **`cloudmodel.rs`:**
  - `verdict_of` checks `unavailable == "out of credit"` **first**, before `outcome` and `cause`. It
    sets `fatal` to `OUT_OF_CREDIT_LABEL` and returns `ModelError::OutOfCredit`.
  - `call` short-circuits on that label with `OutOfCredit`, as it does for the cap.
  - **Out of credit is global:** every later `/judge-*` call in the run stops, task, event and email
    alike, however L3's T3b scopes the cap to a path (L3 plan §9.00 item 3: "402 stays global").
  - `judge_event` passes `OutOfCredit` through as an `Err`. It is never rescued as `unsure`, so the uid
    is asked again next slot.
  - `CloudModel` also records whether any tier-3 reply in the run showed the model answering (a
    verdict, or a cause in `REPEATABLE_CAUSES`), exposed as one read-only accessor. Tier 2 and tier 0
    replies do not count: they say nothing about the provider's balance.
  - `GmailPull` gains `went_quiet: Option<bool>`, read from the reply. An absent or non-boolean value
    is `None`.
- **`enrich.rs`, the cloud arm** (`run_lines_with`, after the passes, where `model.fatal()` is read
  today, EF `:473-477`):
  - `fatal` is the out-of-credit label:
    - the existing summary line prints, worded from the label (§3.7);
    - **raise** `judge-service:out-of-credit`.
  - Otherwise, when the run heard the model answer: **clear** it.
  - A run that made no tier-3 call leaves it alone (Q2's note).
  - The `ProbeOutcome::Transport` and `ProbeOutcome::Fatal` arms change nothing: no model was reached.
- **`enrich.rs`, `pull_gmail`.** The quiet state is handled **before** the round-0 early return for an
  empty pull (EF `:602-605`), because a quiet mailbox is exactly the pull that comes back empty.
  - `Some(true)`: one line (§3.7), then **raise** `gmail-source:quiet` under `GMAIL_ACTOR`.
  - `Some(false)`: **clear** it.
  - `None`: nothing.
  - **Every `CloudError::Quiet(_)`** clears it (round 1, finding 6). Each reason means Knowlu cannot
    read that Gmail at all, and that state already has its own words, so "Gmail has gone quiet" must not
    stand beside them:
    - `Quiet(NoGmailScope)` (disconnected, or calendar only): **clear**, and still print nothing;
    - `Quiet(Revoked)` and every other reason: **clear**, and today's line ("re-connect from settings",
      or the not-configured label) prints unchanged.
  - Transport, status and body errors (`CloudError::Transport`, `CloudError::Status`,
    `CloudError::Body`) leave it alone: they say nothing about the mailbox.
- **`events.rs`, the events pass (T3c, coordinated with L3, which owns `events.rs` edits).** Today any
  `Err` from `judge_event` prints one `not judged` line per event (`events.rs:500-503`). After T3a,
  `call`'s short-circuit means no request is made, but the lines would still print, one per batch item.
  L3's T3 (L3 plan, "T3. The capped pass ends with one line") ends that loop at the first `Capped`
  with one summary line. Out of credit gets the same arm:
  - `Err(ModelError::OutOfCredit)` ends the loop exactly as L3's `Capped` arm does: the item is not
    counted as processed, no per-item line, and one summary line ending `" ({OUT_OF_CREDIT_LABEL})"`
    where L3's ends `" ({CAPPED_LABEL})"`.
  - **Whichever lane lands second adds this arm.** It needs both L7's `OutOfCredit` variant and L3's
    loop-ending arm, and only the second lane has both.
    - L3 on `main` first: L7's T3c adds the arm and its test.
    - L7 first: L7 does not touch `events.rs`. The controller hands L3's controller the arm and test
      6b's text (§6, T3c), for L3 to add when it rebases onto L7. The hand-off is written in **both**
      ledgers (round 2, minor 2b): a row in L7's, and a row in L3's own
      `.superpowers/sdd/2026-10-18-l3-first-day-allowance/progress.md`. If L3's ledger is not open
      yet, the row goes into `HANDOFF.md`'s L3 lane row instead. The row says that the arm L3 adds gets
      a `reviewer` delta check against §3.4 and test 6b before L3's push. Until then an out-of-credit
      run prints one `not judged` line per event in the batch, the same residual L3's T3 names for the
      cap. This plan does not edit L3's plan.
    - L3 merges after L7's last rebase but before L7's merge: the arm would fall between the two.
      T8 re-checks T0(f) just before the merge to catch it (T8; round 2, minor 2a).
- **Both arms:** a raise or clear that fails adds one line (`judge: a heads-up could not be written`
  or `gmail: a heads-up could not be written`). It never changes the exit code, and never stops the
  pass.
- **Unchanged:** `rank`, `write`, `sync`, the slot, exit 0, the run-record shape, every existing line's
  text, and every existing test's assertions.

### 3.5 Read model (T4, `engine/src/surface.rs`)

- `IssuesPanel` gains `sources: Vec<SourceIssueRow>`, serialised with
  `skip_serializing_if = "Vec::is_empty"`.
  - Each row: `id`, `title`, `text` (the note body's first non-blank line), `since` (`opened_at`),
    `close_key` and `path`.
  - Rows are the open `info/` items for which `source_issues::is_source_issue` holds, in `list_info`'s
    path order.
- `open_count` counts flagged rows **and** sources (one "open" number for the view and its nav badge).
  `empty_text` is set only when both lists are empty.
- `good_to_know` leaves source issues out (Q1). Its nav count follows. `mail-reenabled:` heads-ups and
  every other kind stay in it.
- No other region, key or empty string changes.
- A vault with no source heads-up serialises byte-identically to today. The three references prove
  that for their vaults. T1 adds a direct pin that the key is absent.

### 3.6 Page (T5, `app/static/`)

- **The Issues view** (`renderIssuesView`, EF `console.js:614-627`) renders `issues_panel.sources`
  first, then the flagged rows.
  - A source row shows the title, the `text`, and "since YYYY-MM-DD".
  - It has no Resolve box, no flag and no Close: it clears itself.
  - It carries `data-kind="info"`, so the dwell observer's kind rule holds
    (`the_good_to_know_and_issues_views_are_real_and_every_observed_row_names_its_kind`).
  - The header count is `open_count`.
- **The Settings Google row** (`googleSentence`, EF `:1883-1891`): for `state === "quiet"` with Gmail
  granted, the quiet sentence (§3.7) replaces the connected sentence. The buttons follow the `active`
  state's (Disconnect stays, and no Reconnect).
- No new command, no `index.html` change unless console-ui needs a container, and no client-side
  computation: every word the page shows for a source issue comes from the note via the read model,
  apart from the "since" date's label and the Settings sentence.

### 3.7 Copy (Quinn ruled 2026-10-19, Q2: as written; Q3: a week, not two)

| Where | Out of credit | Gmail went quiet |
|---|---|---|
| Run line | `judge: the service answered out of credit on Knowlu's side, so the rest of the batch was not sent` (the existing summary format with `OUT_OF_CREDIT_LABEL` = `out of credit on Knowlu's side`) | `gmail: no mail found in the connected Gmail for 7 days` |
| Heads-up title | Knowlu's AI checks are paused | Gmail has gone quiet |
| Heads-up sentence | This is on Knowlu's side, not your account. Your list still updates; new items get their estimates once it is back. | Knowlu has found no mail in your connected Gmail for a week. Open Settings to check which Google account is connected. |
| Settings row | (none) | Gmail is connected (as {email}), but Knowlu has found no mail in it for a week. If this is the wrong account, disconnect it and connect the right one. |

The out-of-credit words never say "subscription", "payment" or "entitlement". The student's own 402
keeps its existing words ("can't confirm your subscription"). The two must never read alike.

## 4. Open questions

Quinn's are marked **(Quinn)**; the rest are the controller's at dispatch. Each has a recommendation,
and the plan is written to it, so a "yes" changes nothing. Ask Quinn one at a time, in this order.

- **Q1 (Quinn). Where the named issues live.** Recommended: **the Issues view, moved out of *Good to
  know*.**
  - What moves: every source heads-up, email forwarding's `mail-source:` ones included. That is what
    D14 already says ("an issue in the Issues panel"), and what `HANDOFF.md` says for Gmail ("the Issues
    panel is its home").
  - `mail-reenabled:` announcements stay in *Good to know*.
  - The alternative is to show them in both. That is a duplicate row, and a *Close* button on an item
    that re-opens next slot.
  - A second alternative is *Good to know* only, which is today's mail behaviour. It hides a broken
    source among packages and room changes.
  - **Quinn ruled 2026-10-19: the Issues view only, as recommended.** No plan change.
- **Q2 (Quinn). The copy** (§3.7): the run lines, the two heads-ups and the Settings sentence.
  Recommended: as written.
  - A behaviour note that rides with this answer: the out-of-credit item clears on the first later run
    in which the model answers. A run with nothing to judge leaves it as it was, because it has no
    evidence either way. So after credit is restored, the item can stay up until the next slot that has
    something to judge.
  - The alternative is a 2-day `expires`, which hides a still-broken service after two quiet days. Not
    recommended.
  - **Quinn ruled 2026-10-19: the copy as written, with no 2-day expiry.** No plan change, apart from
    Q3's words in the Gmail column of §3.7 ("7 days" in the run line, "a week" in place of "two weeks"
    in the heads-up and Settings sentences).
- **Q3 (Quinn). What "quiet" means for Gmail.** Recommended (r2): **no message of any kind for 14
  days**, matching the cloud design's "stops yielding for 14 days" and the migration's comment.
  - The alternative, "no task or event found for 14 days", fires on any light fortnight. It would teach
    students to ignore the issue.
  - **Quinn ruled 2026-10-19: no message of any kind, as recommended, but over 7 days, not 14.** The
    rule now reads the 7-day list `/gmail-read` already makes, with no second call, and first fires on
    day 7 after connect (§3.3). The ruling amends cloud design §5.3 (an inline dated amendment on its
    Campus-accounts bullet). The migration's "14 days" comment is left alone, because that file is
    hash-pinned (§11 Round 3, item 5; question C1 to the controller).
- **Q4 (Quinn). Live proof.** Recommended: **no live 402 proof.**
  - Provoking a real 402 means swapping staging's `OPENROUTER_API_KEY` for a zero-limit key, which is a
    secret change.
  - The Deno tests pin the adapter, pipeline and handler, and the engine tests pin the device with a
    loopback server.
  - Instead, the controller renders the Issues view with a staged surface JSON (`?fixture=`, memory
    `console-fixture-screenshots`) and shows Quinn.
  - For Gmail, an optional staging check of the `went_quiet: false` half only: the controller
    backdates `connected_at` on Quinn's own staging test account, runs a slot, and expects
    `went_quiet: false` with the row still `active`. The `true` half is not proved live (round 3
    delta, minor 6): in Testing mode the rule cannot fire (§3.3), and T2b's tests 4 and 4b pin it.
    Recommended: skip it. It shows no row; the fixture screenshot above is the look at the row.
- **Q5. Withdrawn in round 1** (finding 2). It asked whether `last_read_at` could carry "the last read
  whose list found mail". The quiet rule now reads Gmail's list directly (§3.3), so nothing writes or
  reads `last_read_at`, and no column question remains. The number is kept so later references stay
  stable.
- **Q6 (controller). `/gmail-read` stops its round on `unavailable`.** Recommended: **yes** (§3.2). It
  is three lines beside the `capped` stop. Without it, the round spends its budget finding the same
  answer item by item until the breaker trips. Nothing new reaches the device on this path. The judge
  step names the state.
- **Q7 (controller). The Anthropic provider.** Anthropic signals a spent balance as a 400 ("credit
  balance is too low"), not a 402. Every pinned row is on OpenRouter today (the provider swap,
  `ci.yml`'s eval comment). Recommended: **out of scope**, named in §9.
- **Q8 (controller). Branch point.** L7 shares `enrich.rs`, `judge.rs` and `cloudmodel.rs` with L6 (§5).
  `judge.rs` is in L6's own file list, and L3 already treats it as L6's (L3 plan, T3: "`judge.rs:190`,
  read only; L6 owns that file").
  - **Default (round 1, finding 9): cut L7 after L6 merges.** No engine task starts before then.
  - The one exception the controller may take if L6 slips: T2a and T2b touch only `cloud/`, which L6
    leaves untouched (L6 plan §5, "`cloud/**` ... untouched"). They may start on a branch cut after the
    email merge and rebase onto L6's merge before T1 starts. After that rebase, and before T1, the
    controller re-runs T0's baseline and its re-greps (a)-(d) on the rebased branch (round 2, minor 4),
    because the numbers and locations T0 first recorded predate L6. The controller records the choice,
    and the second baseline, in the ledger.
- **Q9 (controller). The `console.js` queue.** Quinn ruled on 2026-10-19 that L8's S5 moves ahead of
  L4's T8. The queue is now **L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5** (L4 plan, Q-QUEUE).
  Recommended: **T5 takes L7's slot after L6, after L8's S5, and after L4.**
  - L4 now merges only after L8: its T13 waits for L8's merge. While T13 waits there, the controller
    may let L7 take its slot ahead of L4 and merge on its own gates (L4 plan, Q-QUEUE). L4 then
    rebases over L7 before T13.
  - The lanes behind L4 branch from `main`, so T5 waits for merges, not commits. It rebases onto
    whatever L6, L8 and L4 merged.
  - T5 adds about 20 lines in two functions and deletes none.
- **Q10 (Quinn, only if T8's count is non-zero). The eval gate's spend on this PR.** T2a edits
  `judge_pipeline.ts`, so CI's `eval-gate` runs on the PR with staging's secrets (§1, Money).
  - If staging's `eval_cases` is empty, the gate scores nothing and spends nothing, and Q10 is never
    asked.
  - If it holds rows, the gate makes real model calls, bounded at 1,800 calls and "well under a dollar"
    (`ci.yml:205-208`). Recommended: **push anyway**; that is the gate working as designed. The
    alternative is to hold the PR until the cases are reviewed, which delays L7 for no product gain.
  - L3's PR edits the same file and names the same spend inside its Q-G merge ask (L3 plan, T2b's
    "CI note for the controller"). One answer can cover both if they push close together.

## 5. Conflict map

Every file L7 edits, and who else does. "Second rebases" means whichever of the two lanes merges later
rebases onto the other and re-runs its gates.

| File | L7 task | Other lanes | Rule |
|---|---|---|---|
| `engine/src/surface.rs` | T4 | **L5** T17/T16 (after L7, handoff §5 and L5 plan §2) | L7 first; L5 waits for L7's merge |
| `engine/src/enrich.rs` | T1 (tests only), T3b | **L6** T2 (`Options`, `run`, `run_lines_with`'s `None` arm), **L5** T5 (a syllabus pass before events) | L7 edits only the closing `fatal` block of the cloud arm and `pull_gmail`'s head; second rebases. T1's tests use the branch's `Options` fields |
| `engine/src/judge.rs` | T3a | **L6** T5b (deletes `Missing::{Runtime,Model}`, two `Outcome` variants, `GRAMMAR`, `prompt_for`), **L3** (reads `CAPPED_LABEL`), **L5** T4g (`Heuristics::load`) | L7 adds one `ModelError` variant, one label and one `judge_task` arm, and renames nothing. L6 merges first (Q8's default), so the `judge_task` match already has two fewer arms |
| `engine/src/events.rs` | T3c, only if L3's T3 is on `main` first | **L3** T3 (owns `events.rs` edits; the loop ends at the first `Capped`) | Whichever lane lands second adds the `OutOfCredit` arm beside L3's `Capped` arm (§3.4). L7 first: no L7 edit, and the arm and test 6b are handed to L3's controller |
| `engine/src/cloudmodel.rs` | T1 (tests only), T3a | **L3** T3b (`call`, `verdict_of`, `fatal()`: the cap per path), **L6** T5b (one doc), **L5** T5 (`/judge-syllabus`) | L7's out-of-credit stays global whatever T3b does to the cap. Second rebases. A reviewer checks the merged `call` keeps both rules |
| `engine/src/lib.rs` | T3a (`pub mod source_issues;`) | L6 (`pub mod runtime;` out), #19 (`pub mod registrar;`) | keep every other line |
| `engine/src/source_issues.rs` (new) | T3a | none | — |
| `engine/tests/named_issues.rs` (new) | T1 | none | — |
| `app/static/console.js` (and `style.css` if a class is needed) | T5 | the one console-ui queue (Quinn's 2026-10-19 ruling): L6 → L8 S5 → L4 → **L7** → #19 B5 → L1 R11 → L5 | Q9 |
| `app/tests/static_assets.rs` | T1 | L4, L6, #19, L5 add assertions; L8 S5 has no plan yet, so T5 re-checks it on rebase | keep every other test; append only |
| `cloud/supabase/functions/_shared/judge_anthropic.ts`, `judge_openrouter.ts` (+ tests) | T2a | none found (L3 edits `judge_caps.ts`) | — |
| `cloud/supabase/functions/_shared/judge_pipeline.ts`, `judge_pipeline_test.ts` | T2a | **L3** T2b (owns; the cap path) | L7 edits the `catch` block and `JudgeReply` only. Second rebases. L3's `Caps` fake extension is kept. Both PRs run CI's `eval-gate` (§1, Money; Q10) |
| `cloud/supabase/functions/gmail-read/handler.ts`, `handler_test.ts` | T2a (the `unavailable` stop), then T2b (`went_quiet`) | none found (L1 C3 is the `mail` route) | T2a and T2b run in sequence, never at once |
| `cloud/supabase/functions/gmail-read/index.ts` | T2b | none found | — |
| `cloud/supabase/functions/judge-*/` handler tests (if T2a adds the 200-not-402 pin there) | T2a | none found | — |
| `docs/surface/anatomy.md`, `docs/reference/engine-commands.md`, `cloud/supabase/README.md`, `HANDOFF.md` | T6 | the Docs lane (HANDOFF refresh), L5, L6 | T6 lands after the Docs lane's refresh, or hands it the lines |

**Untouched:** `app/src/**` (so `main.rs` and `commands.rs`; no Tauri command), every file on the
contract list, `engine/src/info.rs`, `engine/src/mail.rs`, `engine/src/mail/heads.rs`,
`engine/src/ranking.rs`, `cloud/supabase/migrations/**` (no migration), `engine/tests/cloud_contract.rs`
and `engine/tests/fixtures/**`. `engine/src/events.rs` is untouched unless T3c runs (the row above).

## 6. Tasks at a glance

| # | Task | Agent | Why that agent | After |
|---|---|---|---|---|
| T0 | Branch, worktree, ledger, baseline, re-grep | main session | branching, commits and baselines are the controller's | L6 merged (Q8's default) |
| T1 | The red tests: engine (out of credit, Gmail quiet, read model) and page | `test-writer` | the failing tests first; it may append to `#[cfg(test)]` modules of `enrich.rs` and `cloudmodel.rs`, and to `app/tests/static_assets.rs`, and it creates `engine/tests/named_issues.rs`. No non-test line | T0 |
| T2a | Cloud: the typed 402, `unavailable`, the gmail-read stop on `unavailable`, with Deno tests first | `cloud-engineer` | `cloud/` is its lane. A wrong reply could tell students their subscription failed, and a provider 402 must never reach the device as a 402 | T0 (parallel with T1: disjoint files) |
| T2b | Cloud: `went_quiet` from the raw 7-day list, and the status moves, with Deno tests first | `cloud-engineer` | a wrong status write could mark a working grant `revoked`, or call a live mailbox quiet | T2a (both edit `gmail-read/handler.ts` and its test) |
| T3a | Engine: `source_issues.rs` (+ `lib.rs`), `ModelError::OutOfCredit` and its label, `verdict_of`/`call`/`judge_event`, the heard flag, `GmailPull.went_quiet` | `implementer` | fully specified by §3.4 and checked by T1's test 6, its own in-module tests and the compiler, off the contract list; about 80 non-test lines | T1 |
| T3b | Engine: the two `enrich.rs` sites (the cloud arm's `fatal` block, `pull_gmail`'s head) | `implementer` | same; about 40 non-test lines, checked by T1's 1-3, 7 and 8 | T3a |
| T3c | Engine: the events pass's `OutOfCredit` arm, **only if L3's T3 is on `main`** when T3b ends or at T8's last rebase | `implementer` | one arm beside L3's `Capped` arm, test first; small and fully specified (§3.4) | T3a, and L3's T3 merged |
| T4 | Read model: `issues_panel.sources`, `good_to_know` without them, counts | `implementer` | same as T3a; the reference pins are the guard | T3b |
| T5 | Page: the Issues view's source rows; the Settings quiet sentence | `console-ui` | `app/static` is its lane | T4, and its slot in the queue (Q9) |
| T6 | Docs: anatomy, engine-commands, cloud README, HANDOFF row | `docs-keeper` | docs and HANDOFF are its lane | T5 |
| T7 | Whole-branch review (cargo and Deno diffs) | `reviewer` | everything a cheaper agent changed passes review before a push | T6 |
| T8 | Gate, staging deploy of the changed functions, PR, CI, merge on green and PASS | main session | deploys, pushes and merges are the controller's | T7 PASS |

No task needs `contract-engineer` or `contract-reviewer`: nothing on the contract list changes (§1).
T2a and T2b are Opus `high` (cloud-engineer). T3a, T3b, T3c and T4 are Sonnet `high` (implementer).
An implementer that fails twice goes one level up, never straight to `max` (CLAUDE.md). Round 1
(finding 4) split T2 and T3 so each implementer task stays within about 80 non-test lines.

---

## T0. Branch, ledger, baseline (main session)

- [ ] **Commit the §5.3 marker first** (round 3 delta, important 2; the controller's ruling). The
      controller commits the one-line inline marker on cloud design §5.3's Campus-accounts bullet in
      its own docs-only commit on `main`, staged non-interactively (r3b, minor 2): write a patch file
      (in the scratchpad) holding only the `@@ -162` hunk of `git diff --
      docs/specs/2026-09-09-knowlu-cloud-design.md`, stage it with `git apply --cached <patch>`, then
      confirm that `git diff --cached --stat` shows 1 file and 1 line changed before committing. The
      uncommitted price amendments in the same file stay unstaged. Record the sha in the ledger. No L7
      task edits the spec.
- [ ] Answer Q8, then create branch `l7-named-issues` and its worktree from the agreed `main` (§2).
- [ ] Open the ledger `.superpowers/sdd/2026-10-19-l7-named-issues/progress.md` (dated as this plan
      is). Record this plan's path, the branch point's sha, and §4's answers as dated rows.
- [ ] Baseline under the slot guard: `cargo build -p knowlu-engine`, `cargo build --workspace`,
      `cargo test --workspace`; and §2's Deno lines, CI's full test line included. Record passed, failed and ignored, and
      the warning tally. Later tasks compare against these numbers.
- [ ] Re-confirm on the branch, each as a ledger row:
  - (a) Every "EF" location in §3 by name: `verdict_of`, `call`, `judge_event`'s arms,
    `pull_gmail_queue`, `pull_gmail`'s round-0 return, the cloud arm's `fatal` block, `issues_panel`,
    `good_to_know`, `renderIssuesView`, `googleSentence`, the pipeline's `catch`, and gmail-read's
    forwarding return and list call.
  - (b) `git grep -n "heads-up" engine/tests/fixtures` shows only `pyyaml-safe-dump-reference.json`.
    So no reference vault holds an `info/` heads-up, and `sources` cannot appear in a reference.
  - (c) `git grep -n "last_read_at\|'quiet'" cloud/supabase/functions` shows no writer of either. After
    T2b, `last_read_at` still has none (§3.3).
  - (d) `git grep -n "unavailable\|went_quiet" engine/src cloud/supabase/functions` shows that neither
    key name is already in use on the wire. A hit is a stop-and-report: the name changes before T1.
  - (e) Which edge functions import `judge_pipeline.ts`, `judge_openrouter.ts` or `judge_anthropic.ts`,
    directly or through `_shared/`. That list is T8's deploy list.
  - (f) Whether L3's T3 (the events pass's `Capped` arm) is on `main`. That decides whether T3c runs
    (§3.4, §5). Re-checked at T3b's end, at T8's last rebase, and just before T8's merge.

## T1. The red tests (test-writer)

**Files:**
- `engine/tests/named_issues.rs` (new);
- appended `#[cfg(test)]` tests in `engine/src/enrich.rs` and `engine/src/cloudmodel.rs`;
- `app/tests/static_assets.rs` (appended).

No non-test line in any file.

**Brief.** Every test's doc comment says what it guards and cites this plan's section. No real course,
school, address or name. Every server is a loopback listener (`enrich.rs`'s `multi_reply_loopback`
in-module, or a copy of its shape in the new file, as `cloud_contract.rs` copies `Loopback`).

**Red-not-uncompilable rule.** Use only items that exist on the branch at T1:
- the keys as string literals (`"judge-service:out-of-credit"`, `"gmail-source:quiet"`);
- the read model through `serde_json::to_value` and JSON paths, never a `sources` field;
- vault state through `info::list_info` and the `archive/` folder.

Tests that need the out-of-credit label compare against the §3.7 run line's literal text.

**Engine tests (in-module where the harness is private, else the new file):**
1. `an_out_of_credit_reply_stops_the_batch_and_names_it_once` (enrich.rs). Three flagged tasks. The
   loopback answers the probe as clear, then the first `/judge-task` with
   `{verdict: null, tier: 3, outcome: "low confidence", cause: "model failed", unavailable: "out of credit"}`,
   and then whatever the cloud arm's later passes ask (copy the replies an existing cloud-arm test
   uses for an account with no Gmail, no mailbox and no rules). Assert, **in this order** (round 1,
   finding 7):
   - **first**, the loopback's request list, by path: exactly one `/judge-task` request, followed by the
     later passes' paths. `multi_reply_loopback` answers by arrival, not by path (`enrich.rs`'s doc on
     it), so a mismatched reply could otherwise turn a later assertion red for the wrong reason. The
     assertion's message prints the whole path list. At T1 it is red here, on the path list (a second
     `/judge-task` where the next pass's path should be), and that is the recorded red reason;
   - the §3.7 run line appears once;
   - the exit code is 0;
   - all three tasks keep `needs_enrichment: true`, and tier 1's fields are written;
   - exactly one open `info/` item has `close_key: judge-service:out-of-credit`, `kind: heads-up` and
     an `opened_by` that starts `agent:`.
2. `a_second_out_of_credit_run_does_not_duplicate_the_heads_up` (enrich.rs). Run 1 twice. There is
   still exactly one open item with that key.
3. `a_tier_three_answer_clears_the_out_of_credit_heads_up` (enrich.rs). The vault holds the open item
   from 1. The service answers a tier-3 verdict. The item moves to `archive/` with `status: closed`. No
   out-of-credit line is printed.
4. `a_tier_two_answer_does_not_clear_it` (enrich.rs). The same set-up (the open item planted through
   `info::open_info`), but the reply is `tier: 2` with a verdict. The item stays open. **Green at T1**
   (round 1, finding 3): nothing clears a heads-up today. It is the regression pin that T3b's clear
   fires only on a tier-3 answer.
5. `an_old_servers_model_failed_reply_is_unchanged` (enrich.rs). The same reply as 1 without
   `unavailable`. Today's per-item behaviour, no heads-up, and no out-of-credit line. This is a
   regression pin and is green at T1.
6. `out_of_credit_stops_the_event_pass_and_keeps_the_uid_retryable` (**cloudmodel.rs**, in-module, so
   T3a alone turns it green). Drive one `CloudModel`: a task call whose reply carries
   `unavailable: "out of credit"`, then one `judge_event` call. Give the loopback a second reply (an
   event verdict), so a regression is answered rather than refused. Assert:
   - the loopback received exactly one request (the task's), checked first;
   - `judge_event` returned an `Err`, never an `unsure` verdict, so the uid stays pending.
   Red at T1 on the request count (two). Whether the events pass then prints per-event lines is T3c's
   test 6b, not this one (§3.4).
7. `a_quiet_gmail_is_named_and_kept_as_one_heads_up` (enrich.rs, `pull_gmail`). The loopback answers
   `/gmail-read` with `{items: [], read: 0, quiet: false, more: false, went_quiet: true}`. Assert:
   - the §3.7 Gmail line appears;
   - exactly one open `gmail-source:quiet` heads-up exists, `opened_by` `agent:knowlu.gmail`;
   - a second call adds none.
8. Four `pull_gmail` cases (enrich.rs), each starting from an open `gmail-source:quiet` item planted
   through `info::open_info` (round 1, finding 6):
   - `went_quiet_false_clears_it`;
   - `no_gmail_scope_clears_it_and_prints_nothing`;
   - `a_revoked_reply_clears_it_and_keeps_todays_line` (the reply `quiet: true`, `reason: "revoked"`;
     today's "re-connect from settings" line is printed unchanged);
   - `a_transport_error_leaves_it_open` (the loopback answers HTTP 500). **Green at T1**: a pin that
     T3b clears only on a `Quiet` reason or `went_quiet: false`.
9. `a_reply_without_went_quiet_changes_nothing` (enrich.rs). An open item stays open, and the lines
   equal today's. Green at T1.
10. `the_read_model_lists_source_issues_in_the_issues_panel` (`named_issues.rs`). Plant through
    `info::open_info`: a `heads-up` keyed `mail-source:a1`, one keyed `mail-reenabled:a1`, one keyed
    `gmail-source:quiet`, and a `notice`. Build the state the way `surface_oracle.rs` does. Assert
    through JSON:
    - `issues_panel.sources` holds the `mail-source:` and `gmail-source:` items in path order, each with
      `title`, `text` (the body's sentence) and `since`;
    - `good_to_know` holds the other two;
    - `issues_panel.open_count` is 2, and `empty_text` is null;
    - `nav_counts.issues.count` is 2, and `nav_counts.good_to_know.count` is 2.
11. `an_issues_panel_with_no_sources_serialises_without_the_key` (`named_issues.rs`). An empty vault's
    `issues_panel` JSON has no `sources` key, and is otherwise today's. Green at T1. It is the direct
    pin behind the references.

**Page tests (`app/tests/static_assets.rs`):**
12. `the_issues_view_renders_source_issues_from_the_payload`. `console.js` reads
    `issues_panel.sources`, and its row markup carries `data-kind="info"`.
13. `the_settings_google_row_names_a_quiet_gmail`. `googleSentence` has a branch on
    `state === "quiet"`, and the §3.7 Settings sentence (or Quinn's Q2 wording) is present.

**Acceptance.**
- Under the slot guard, `cargo test --workspace` shows these red, each for **its stated reason**:
  - 1: the request path list (more than one `/judge-task`);
  - 2 and 3: no heads-up to keep or close (per-item failures, no heads-up);
  - 6: two requests where one is expected (the event is still sent);
  - 7 and 8's first three cases: no line and no item, or the item still open;
  - 10: `sources` is absent, and the items are in `good_to_know`;
  - 12 and 13: the strings are absent.
- Green at T1, as regression pins: 4 (T3b's tier-3-only clear), 5, 8's transport case, 9 and 11.
- No compile error. Every pre-existing test is still green, with 0 new warnings.
- Hand-off: the file list, one line per test (red reason or green), and a commit message:
  "test(l7): out of credit and a quiet Gmail as named issues, red".

**Commands** (each under the slot guard, `M` = `--manifest-path C:/Users/danie/GitHub/knowlu/.claude/worktrees/l7-named-issues/Cargo.toml`):
- `cargo test -p knowlu-engine --lib M -- enrich:: cloudmodel::`
- `cargo test -p knowlu-engine --test named_issues M`
- `cargo test -p knowlu --test static_assets M`
- then `cargo test --workspace M`, for the full tally.

## T2a and T2b. Cloud (cloud-engineer)

Split in round 1 (finding 4). T2a and T2b both edit `gmail-read/handler.ts` and its test, so they run
in sequence, never at once. Each is its own commit.

### T2a. Out of credit on the server

**Files:**
- `cloud/supabase/functions/_shared/judge_anthropic.ts` (the error class only);
- `_shared/judge_openrouter.ts` and `_shared/judge_openrouter_test.ts`;
- `_shared/judge_pipeline.ts` and `_shared/judge_pipeline_test.ts`;
- `gmail-read/handler.ts` and `gmail-read/handler_test.ts`, for the `unavailable` stop only (Q6);
- one judge handler test file, if the 200-not-402 pin needs a handler-level test. The handler shared
  by the three `/judge-*` functions is `_shared/judge_handler.ts`: pin it there, or in whichever test
  already exercises it.

No migration. No prompt, schema, model pin or `cloud/eval` change.

**Brief.** §3.2, exactly. Tests first:
1. `judge_openrouter_test.ts`:
   - an HTTP 402 from the fake fetch throws `ProviderOutOfCredit`, whose message names 402 and no body
     text;
   - every other non-2xx still throws a plain `Error`, unchanged.
2. `judge_pipeline_test.ts`:
   - a model that throws `ProviderOutOfCredit` gives a reply with `verdict: null`, `tier: 3`,
     `outcome: "low confidence"`, `cause: "model failed"` and `unavailable: "out of credit"`;
   - the logged row's `cause` is `model failed`;
   - the charge is refunded;
   - a plain `Error` gives no `unavailable` key (byte-for-byte today's reply).
3. The handler: a pipeline answering `unavailable` is HTTP **200**, never 402. Name the test for the
   trap (§3.1), for example `a_provider_402_never_reaches_the_device_as_a_402`.
4. `gmail-read/handler_test.ts` (Q6): an item whose reply carries `unavailable` stops the round. Later
   ids are not judged, and the item is neither enqueued nor marked seen.
5. Read `mail-process` and `cloud/eval/run_eval.ts`. Record in the hand-off that neither needs a change
   (§3.2), or stop and report.

**Acceptance.**
- §2's Deno lines are green, CI's full test line included, and `deno lint` and `deno check` are clean.
- Every pre-existing Deno test's assertions are unchanged.
- `git diff` touches only the files above.
- Hand-off: the file list, the red-then-green record per test, T0(e)'s deploy list re-checked, and a
  commit message: "cloud(l7): a provider 402 is a named 'out of credit', never a device-facing 402".

### T2b. A quiet Gmail on the server

**Files:** `gmail-read/handler.ts`, `gmail-read/index.ts` and `gmail-read/handler_test.ts`. No
migration, and no other function.
- **Not** `cloud/supabase/migrations/20260911000200_google.sql`, although its `status` comment still
  says "yielding nothing for 14 days" (EF `:24`). The file is hash-pinned by
  `migrations_test.ts`'s "the original google migration is unchanged" (EF `:980-987`, "Applied
  migrations are never edited"), so editing even a comment turns that test red, and changing the pin
  edits an existing assertion. T6's README line records that the comment is superseded by the
  amended §5.3. Question C1 (§11 Round 3) asks the controller whether that is enough.

**Before starting.** T2a is committed. Re-run `handler_test.ts` and confirm it is green.

**Brief.** §3.3, exactly. Tests first, in `handler_test.ts`, with a fake clock, a fake `GmailApi` that
records each `list` call's query, and fake deps that record every status write and its condition.
"N days old" below only describes a fixture: `connected_at` set N × 24 hours before the fake clock.
The handler itself compares milliseconds and never rounds to days (§3.3; test 3b pins it).
Renumbered in r3 (§11 Round 3). Tests 1-6 (with 3b, 4b and 5b) each assert that **exactly one** list
call is made, and that its query is today's `query()` output (`WINDOW` first), so none can pass with
a second call. Tests 7 and 8 assert that **no** list call is made:
1. `a_message_five_days_old_is_mail`: the 7-day list holds one unseen id (a message 5 days old),
   `connected_at` is 30 days old, and the row is `quiet`. `quiet` moves to `active`, and the reply has
   `went_quiet: false`. The id is judged as today.
2. `a_seven_day_list_of_seen_ids_is_not_quiet` (round 2, minor 1): the 7-day list returns only ids this
   account has already seen, `connected_at` is 30 days old, and the row is `quiet`. The reply has
   `went_quiet: false`, `read: 0`, and `quiet` moves to `active`. Red if the check reads the list after
   `.filter(unseen)`.
3. `a_grant_six_days_old_is_not_quiet`: the list is empty and `connected_at` is 6 days old. The reply
   has `went_quiet: false`, and no status is written.
3b. `a_grant_one_millisecond_short_of_seven_days_is_not_quiet` (round 3 delta, minor 3): the list is
   empty and `connected_at` is 7 × 24 hours − 1 ms before the fake clock. `went_quiet: false`, and no
   status is written. Red under `Math.round` or `Math.ceil` on days.
4. `a_grant_exactly_seven_days_old_goes_quiet`: the list is empty, the row is `active`, and
   `connected_at` is exactly 7 × 24 hours before the fake clock (and again at 30 days). `active` moves
   to `quiet`, and the reply has `went_quiet: true`, `quiet: false` and no `reason`.
4b. `a_row_already_quiet_still_answers_went_quiet_true` (round 3 delta, important 1): the row is
   already `quiet`, the list is empty and `connected_at` is 30 days old. The reply has
   `went_quiet: true`, and no status write changes the row (the fake's conditional `active`→`quiet`
   write matches nothing, or none is made). Red if the answer is taken from the write's effect, which
   would make the engine's heads-up close and reopen on alternate slots.
5. `a_failed_list_claims_nothing`: the list call throws a `GmailApiError`. The reply is today's
   (HTTP 500, `read failed`), no status is written, and there is no `went_quiet` key. A pin on today's
   behaviour: no new catch and no new log line.
5b. `a_failed_status_write_is_a_500_before_any_item_is_judged` (round 3 delta, minor 5): the list
   holds one unseen id, the row is `quiet`, and the fake's `quiet`→`active` write throws. The reply is
   HTTP 500, `read failed`. The fakes record zero `deps.api.message` calls (the loop's first act, EF
   `handler.ts:364`) and no `enqueue` or `markSeen` call, so the write runs before the loop and is not
   caught on its own. (`deps.pipeline()` is built before the list, EF `:340`, so it is not the
   signal; r3b, minor 1.)
6. Every status write carries its condition (`status=eq.active` or `status=eq.quiet`), so a `revoked`
   row is never written by any case above.
7. The forwarding path: `quiet` moves to `active`, `went_quiet: false`, and no list call at all.
8. The `no_gmail_scope` and `revoked` replies carry no `went_quiet` key and write no status beyond
   today's `markRevoked`, as today.

The r2 tests 2 (a 10-day-old message on the 14-day list), 5 (the 14-day query) and 6 (the second
call's failure and its log line) are dropped: the second call they tested no longer exists.

**Acceptance.**
- §2's Deno lines are green, CI's full test line included, and `deno lint` and `deno check` are clean.
- Every pre-existing Deno test's assertions are unchanged.
- `git diff` touches only the three files above.
- `git grep -n "quiet: true" cloud/supabase/functions/gmail-read/handler.ts` shows only the two
  existing not-connected replies.
- `git grep -n "last_read_at" cloud/supabase/functions` has no hit: nothing writes or reads it.
- `git grep -n "newer_than" cloud/supabase/functions/gmail-read/handler.ts` shows only `WINDOW`'s own
  line: no second query exists.
- Hand-off: the file list, the red-then-green record per test, and a commit message: "cloud(l7):
  gmail-read says when a connected Gmail has found no mail for 7 days".

## T3a, T3b and T3c. Engine (implementer)

Split in round 1 (finding 4): T3a is the module, the error and the client (about 80 non-test lines);
T3b is the two `enrich.rs` sites (about 40); T3c is one `events.rs` arm and runs only if L3's T3 is
already on `main` (finding 5). Each is its own commit. **Not** `surface.rs`, which is T4.

### T3a. The module, the error and the client

**Files:**
- `engine/src/source_issues.rs` (new) and `engine/src/lib.rs` (one `pub mod` line);
- `engine/src/judge.rs`;
- `engine/src/cloudmodel.rs`.

**Before starting.** If L3 merged after T1 (L6 is already in, Q8), rebase first and re-run T1's tests
to confirm the same reds. A changed red reason is a stop-and-report.

**Behaviour.** §3.4's `source_issues.rs`, `judge.rs` and `cloudmodel.rs` bullets, exactly. In
particular:
- **raise and clear.** They are the only code that opens or closes the two keys. T3b's `enrich.rs`
  calls them; it never calls `info::` directly for these keys. Keep `source_issues.rs` small: about 40
  lines of non-test code is the expected size.
- **The order in `verdict_of`.** `unavailable` is checked before `outcome == "capped"`. A reply
  carrying both is out of credit.
- **`OUT_OF_CREDIT_LABEL`.** Its text makes the existing summary line read as §3.7's run line (or Q2's
  answer). The line itself is not reformatted.
- **The heard flag.** It is set only by a tier-3 reply that shows the model answered.
- **In-module tests** for `source_issues.rs`:
  - raise twice gives one item;
  - clear with none open writes nothing (the journal is unchanged);
  - `is_source_issue` is true for the three prefixes, false for `mail-reenabled:` and for a kind
    other than `heads-up`.
- **In-module tests** for `cloudmodel.rs`:
  - `verdict_of` with `unavailable` and with `unavailable` plus `capped`;
  - the short-circuit returns `OutOfCredit` without a request;
  - `went_quiet` is parsed as true, false, absent, and a non-boolean value (`None`).

**Acceptance.**
- T1's 6 and the in-module tests above are green.
- T1's 1-3, 7 and 8's first three cases are still red, for the heads-up reason only (1's request path
  list is now right). 10 and 12-13 are still red (T4, T5). The green pins (4, 5, 8's transport case, 9,
  11) stay green. Every other test is green, with 0 new warnings.
- `git diff` touches only the four files above.
- No existing assertion is edited.
- Hand-off: the file list and a commit message: "engine(l7): out of credit is a global stop in the
  cloud client; source_issues keeps one heads-up per key".

**Commands:** `cargo build -p knowlu-engine M`; `cargo test -p knowlu-engine M`; then
`cargo test --workspace M` (slot guard).

### T3b. The two `enrich.rs` sites

**Files:** `engine/src/enrich.rs` only.

**Before starting.** T3a is committed. If L3 or L5 merged since, rebase and re-run T1's tests first.

**Behaviour.** §3.4's two `enrich.rs` bullets and the "Both arms" bullet, exactly:
- the cloud arm's closing `fatal` block raises or clears `judge-service:out-of-credit` through
  `source_issues`, reading T3a's heard flag;
- `pull_gmail`'s quiet handling runs once per call, on the first round's reply, before the round-0
  empty return. It clears on every `CloudError::Quiet(_)` and leaves the item alone on transport,
  status and body errors (finding 6).
- No in-module test is needed beyond T1's: they drive both sites end to end.

**Acceptance.**
- T1's 1-9 are green. 10 and 12-13 are still red (T4, T5). Every other test is green, with 0 new
  warnings.
- `git diff` touches `enrich.rs` only.
- `git grep -n "info::open_info\|info::close_info" engine/src/enrich.rs` adds no hit over the branch
  point.
- No existing assertion is edited.
- Hand-off: the file list and a commit message: "engine(l7): judge names out of credit once and keeps
  one heads-up; a quiet Gmail keeps one heads-up".

**Commands:** as T3a.

### T3c. The events pass ends at out of credit (only if L3's T3 is on `main`)

**Run it** when T0(f), the check at T3b's end, or T8's last rebase finds L3's T3 on `main` (L7 lands
second). **Skip it** otherwise. The controller then hands L3's controller the arm and test 6b below,
so L3 adds them when it rebases onto L7 (§3.4). The hand-off row goes into L7's ledger **and** L3's
(`.superpowers/sdd/2026-10-18-l3-first-day-allowance/progress.md`, or `HANDOFF.md`'s L3 lane row if
that ledger is not open), and says that L3's arm gets a `reviewer` delta check before L3's push
(round 2, minor 2b). Either way the outcome is recorded in L7's ledger.

**Files:** `engine/src/events.rs` only (`judge_roster`'s item loop and its test module).

**Behaviour.** §3.4's `events.rs` bullet: `Err(ModelError::OutOfCredit)` ends the loop exactly as L3's
`Capped` arm does, and the summary ends `" ({OUT_OF_CREDIT_LABEL})"`. L3's `Capped` arm, its summary
and its tests are unchanged. If the two arms share code, they share it by matching both variants in one
arm that picks the label; neither label's text changes.

**Test 6b, first, seen red:** `an_out_of_credit_reply_ends_the_event_pass_with_one_line`, beside L3's
`a_capped_reply_ends_the_pass_with_one_line_and_an_exact_left`, using L3's `scratch_vault_with_n_events`
helper. `n = 5`; the fake model judges 2 events, then answers `ModelError::OutOfCredit` from call 3 on.
Assert: the model was called exactly 3 times; no line contains `not judged`; exactly one line starts
`events: `, and it equals `events: 2 judged, 3 left for the next slot (` followed by
`OUT_OF_CREDIT_LABEL` and `)`. Red before the arm: three `not judged` lines.

**Acceptance.** 6b is green; L3's events tests are unchanged and green; the workspace is green with 0
new warnings; `git diff` touches `events.rs` only. Commit message: "engine(l7): the events pass ends
at out of credit with one line, beside the cap".

## T4. Read model (implementer)

**Files:** `engine/src/surface.rs` only (struct, `issues_panel`, `good_to_know`, the `nav_counts`
call, and in-module tests).

**Behaviour.** §3.5, exactly.
- `SourceIssueRow.text` is read from the note's body: the first non-blank line after the frontmatter.
  Read it through the same split `issues::list_issues` uses, or `models::split_frontmatter`. It is
  never computed from the key.
- `good_to_know` filters with `source_issues::is_source_issue`. It does not keep a second prefix list.
- `open_count` and the nav badge count both lists. `empty_text` needs both lists empty.

**Tests.**
- T1's 10 turns green, and 11 stays green.
- `engine/tests/surface_oracle.rs` passes with **no edit and no regeneration**.
- An in-module test pins that a heads-up whose body is empty gives `text: ""`, not a panic.
- An in-module test pins that a `heads-up` whose key has no listed prefix stays in `good_to_know`.

**Acceptance.**
- The workspace is green apart from T1's 12-13, with 0 new warnings.
- `git diff --exit-code -- engine/tests/fixtures` is empty.
- `git diff` touches `surface.rs` only.
- Hand-off with a commit message: "surface(l7): the Issues panel lists sources in trouble; Good to
  know leaves them out".

**Commands:** `cargo test -p knowlu-engine --test surface_oracle M`; `cargo test -p knowlu-engine
--test named_issues M`; `cargo test --workspace M`.

## T5. Page (console-ui)

**Files:** `app/static/console.js`, and `app/static/style.css` only if a class is needed. Nothing else
under `app/`.

**Before starting.** Wait for the queue slot (Q9: L6 → L8 S5 → L4 → L7, or ahead of L4 if the
controller lets L7 go first while L4's T13 waits for L8). Rebase onto whatever L6, L8 and L4 merged,
and re-run `static_assets`.

**Behaviour.** §3.6, with §3.7's Settings sentence or Q2's answer.
- Source rows come before flagged rows. Each has title, text and "since YYYY-MM-DD", and no
  Resolve/flag/Close.
- `googleSentence` gains the `quiet` branch. `renderGoogleRow`'s buttons treat `quiet` as `live`, as
  they do today.
- No string the engine supplies is re-worded on the page. No new `invoke`, and no new event action
  (`uievents::ACTIONS` is unchanged).

**Check.**
- Render a staged surface JSON with two source issues and one flagged issue through `?fixture=` in
  headless Edge (memory `console-fixture-screenshots`). Hand the screenshot path to the controller for
  §8 checkpoint 4.
- Run `scripts/wizard-check.py` as a regression walk.

**Acceptance.**
- T1's 12 and 13 are green, and the whole `static_assets` file is green.
- `cargo test --workspace M` is green, with 0 new warnings.
- Hand-off with a commit message: "console(l7): the Issues view shows sources in trouble; Settings
  names a quiet Gmail".

## T6. Docs (docs-keeper)

**Files:**
- `docs/surface/anatomy.md`:
  - the Issues row of the regions table;
  - a new paragraph in the Issues section on `sources`;
  - §4.4's empty-state line (needs both lists empty).
- `docs/reference/engine-commands.md` (`judge`):
  - the out-of-credit summary line and the heads-up;
  - `pull_gmail`'s quiet line and the heads-up.
- `cloud/supabase/README.md`:
  - `/gmail-read`'s `went_quiet` key, the 7-day rule read off the existing list, and the status
    moves;
  - that `last_read_at` stays unwritten (§3.3), so no reader mistakes it for the quiet clock;
  - that the "14 days" in `20260911000200_google.sql`'s `status` comment is superseded by the amended
    cloud design §5.3 (7 days, Quinn's Q3 of 2026-10-19), and that the migration is not edited because
    it is hash-pinned;
  - the judge reply's `unavailable` key.
- `HANDOFF.md`: the L7 bullet in §3 and the lane row, as done, with the PR link at T8. The §3 bullet's
  "`google_accounts.status = 'quiet'` after 14 days" becomes "after 7 days (Quinn's Q3, 2026-10-19)".

**Also.** One ledger row telling L5's owner that `surface.rs` is free after T8. (Round 1 dropped the
note to L4: with no `last_read_at` writer, L7 stores nothing new that L4 could name.)

**Acceptance.**
- Docs only.
- Every claim cites the code by function name, not by line.
- Hand-off with a commit message: "docs(l7): named issues in the read model, judge, gmail-read".

## T7. Whole-branch review (reviewer)

Review `git diff main...l7-named-issues` against this plan, the Rust and the TypeScript. Write the
report to `docs/reports/2026-10-19-l7-named-issues-review.md`. Checks, beyond the usual:
1. **The 402 trap.** No path turns a provider 402 into a device-facing HTTP 402. The engine's
   `CloudError::Status { code: 402 }` mapping, and the page's "no entitlement" words, are untouched.
2. **The `quiet` key.** It keeps its one meaning: no went-quiet reply sets `quiet: true` or a `reason`.
   The engine's `QuietReason` handling is unchanged.
3. **No status write can touch `revoked`.** Every write is conditional.
4. **Old pairs.** An old engine with the new server, and the new engine with an old server, behave as
   today (T1's 5 and 9, T2a's plain-error and T2b's absent-key pins).
5. **Out of credit is global.** It stops every `/judge-*` path, whatever L3's cap scoping did. It is
   never rescued as an event `unsure`. If T3c ran, the events pass ends with one line. If it did not,
   L7's ledger and L3's (or `HANDOFF.md`'s L3 lane row) both hold the hand-off to L3.
5a. **The 7-day rule fires on day 7.** `went_quiet: true` needs the **raw** 7-day list (before
   `.filter(unseen)` and the `READ_CAP` slice) empty and `connected_at` at least 7 days old. There is
   exactly one list call, `WINDOW` is still `newer_than:7d`, and a failed list claims neither answer
   and writes no status. The age compares milliseconds, never rounded days. A row already `quiet`
   still answers `went_quiet: true`. The status write runs before the judging loop, and a failed write
   is a 500. Nothing writes `last_read_at`, and the hash-pinned google migration is unedited (T2b's
   tests 1-5, with 3b, 4b and 5b). **The §5.3 marker is on `main`:** `git log main --oneline --
   docs/specs/2026-09-09-knowlu-cloud-design.md` shows T0's docs-only marker commit, and the spec's
   Campus-accounts bullet on `main` carries the marker (round 3 delta, important 2).
6. **Writes go through `write`.** Every heads-up write uses `source_issues` raise or clear, through
   `info::open_info` or `info::close_info`, with an `agent:` actor. Nothing writes a file directly.
7. **`judge` always exits 0.** No new line can change the exit code.
8. **Rule 2.** `git diff --exit-code main -- engine/tests/fixtures` is empty, and `surface_oracle.rs`
   is unedited and green.
9. **The contract list.** Nothing on CLAUDE.md's list changed (file by file). No migration, and no
   prompt, schema or pin.
10. **No single-user assumption.** No address, school or name in code, tests or copy. The copy matches
    Q2's answer.
11. **The warning tally** is 0 other. The Deno check and lint are clean.

A PASS lets T8 go ahead. Findings are answered in §11 and re-reviewed.

## T8. Gate, deploy to staging, PR, merge (main session)

- [ ] Rebase onto `main` one last time and re-check T0(f). If L3's T3 landed since T3b and T3c has not
      run, run T3c now; its commit then gets a `reviewer` delta check before the push (§3.4, finding 5).
- [ ] Final gate on the branch under the slot guard: engine build, workspace build and test, eol
      check, §2's Deno lines with CI's full test line. Record the numbers in the ledger.
- [ ] **Staging deploy** of T0(e)'s list (the `/judge-*` functions, `gmail-read`, and any other
      importer of the changed `_shared` files). The hook asks for each `supabase functions deploy`.
      Record each in the ledger.
  - Staging only. **Prod rides L9, with Quinn's go.**
  - Deploy order against devices is free (§2).
- [ ] Post-deploy smoke on staging (controller only; a staging session by OTP, memory
      `staging-session-otp`).
  - One slot on a scratch profile with flagged tasks. Expect answered judgments and no out-of-credit
    line: the regression half.
  - Q4 decides whether anything more is run.
  - Then remove the scratch profile (memory `live-proof-profile-cleanup`).
- [ ] **Count staging's `eval_cases` before the push** (round 1, finding 1), read-only, as the
      controller: `select count(*) from eval_cases` on staging. Record the count and the time in the
      ledger. The count shows only that staging answered at that moment (round 2, minor 4). It does
      not promise that the gate's own staging call will answer, and that call still fails loud by
      design if staging is down (§1, Money).
  - 0: push. The eval gate runs, scores nothing and spends nothing (§1, Money).
  - Non-zero: stop and ask Quinn Q10 before the push. If L3 pushes close by, ask once for both.
- [ ] Push the branch and open the PR ("L7: out of credit and a quiet Gmail as named issues"). Watch CI
      with `gh run watch <id> --exit-status`.
- [ ] **Re-check T0(f) just before the merge** (round 2, minor 2a). The branch's last rebase may
      predate L3's merge. If L3's T3 landed in that window and T3c has not run, do one of two things:
      - rebase, run T3c and its `reviewer` delta check, and let CI run again before the merge; or
      - merge as is and file the arm as a **named follow-up**: a row in L7's ledger, and in
        `HANDOFF.md`'s lane rows, naming the arm, test 6b (§6, T3c) and the `reviewer` delta check.
        L7's controller owns it, because L3 has already merged and will not rebase onto L7.
      Record the choice in the ledger.
- [ ] Merge on green CI and T7 PASS (handoff §1 item 7), with a merge commit. Never merge over a red
      check without Quinn's word.
- [ ] Ledger rows for the merge. Tell L5's controller that `surface.rs` is free, and the console-ui
      queue that L7's slot is done. Remove the worktree.

## 7. Fidelity ledger

| Source | Requirement | Task | Proof |
|---|---|---|---|
| Handoff §5 L7; HANDOFF §3 | OpenRouter's 402 is a named issue, "instead of quiet model failures" | T2a, T3a, T3b, T3c, T4, T5 | T2a's tests 1-3; T1's 1, 2 and 6; T3c's 6b (or the hand-off to L3); T1's 10 and 12 |
| Quinn's ruling Q3 (2026-10-19), amending cloud design §5.3 | "source went quiet" = connected, no message of any kind for 7 days, first firing on day 7 | T0 (the marker commit), T2b | T2b's tests 1 (a 5-day-old message is mail), 2 (seen mail still counts), 3, 3b and 4 (6 days, 7 days − 1 ms, exactly 7 days on `connected_at`), 4b (a row already `quiet` still answers `true`), 5 (a failed list claims nothing) and 5b (the write runs before the loop; a failed write is a 500); one list call in tests 1-6; the §5.3 marker committed on `main` at T0 (T7 check 5a) |
| HANDOFF §3 ("the Issues panel is its home"); email spec D14 | the issues live in the Issues panel; forwarded mailboxes' too | T4, T5 | T1's 10; Q1 |
| Gmail connect spec §4.4 (`quiet` row) | Settings shows the quiet state | T5 | T1's 13 |
| VISION "Failures are visible" / "fails visibly rather than going quiet" | a named line in the run and a visible row, every slot it holds; a quiet notice never stands beside a not-connected line | T3b, T5 | T1's 1 and 7 (the lines); 8's revoked case; 10 and 12 (the rows) |
| VISION commitment 5 (amended, A10) | "a mailbox gone quiet is shown, not proposed" | T3a, T3b | heads-ups are `info/` items; no approval card is filed (T7 check 6) |
| CLAUDE.md "`judge` always exits 0" | out of credit and every heads-up failure are named outcomes at exit 0 | T3b | T1's 1 (exit 0); §3.4's failure line |
| CLAUDE.md "`rank` never calls a model" | nothing on `rank`'s path changes | all | T7 reads the diff; `rank` and `surface` reach no model |
| CLAUDE.md "every note write goes through `write`" | heads-ups through `info::open_info` and `close_info` | T3a, T3b | T7 check 6 |
| CLAUDE.md "deterministic" | the open or closed state is a function of the reply and the vault; rows in path order | T3a, T3b, T4 | T1's 2 and 10 (no duplicates, the order asserted) |
| CLAUDE.md agent actors | heads-ups written under `agent:` actors | T3b | T1's 1 and 7 (`opened_by` starts `agent:`) |
| CLAUDE.md rule 1 | no single-user assumption | all | T7 check 10 |
| CLAUDE.md rule 2; console spec §4.6 | no reference regenerated; `surface-today-*.json` byte-identical | T4 | T1's 11; `surface_oracle.rs` green and unedited; T7 check 8 |
| CLAUDE.md contract list | no file on it is edited | all | §1; T7 check 9 |
| CLAUDE.md "tests never leave the machine" | loopback servers and injected fetches only | T1, T2a, T2b, T3c | T7 reads the test files |
| CLAUDE.md "0 warnings"; TDD | the gates, and red first | every task | T1's red record; T2a's and T2b's red-then-green records; T3c's 6b red first; T8's gate line |
| §3.1 trap | a provider 402 never reaches the device as an HTTP 402 | T2a | T2a's test 3; T7 check 1 |
| §3.1 trap | `quiet: true` keeps its not-connected meaning | T2b | T2b's test 4 and the `quiet: true` grep; T7 check 2 |
| L3 plan §9.00 item 3 | "402 stays global"; the capped state stays per path | T3a, T3c | T1's 6; T3c's 6b; T7 check 5 |
| L3 owns `events.rs` edits (L3 plan, T3) | the `OutOfCredit` arm is added by whichever lane lands second | T3c or L3 | T0(f) and T8's two re-checks (last rebase, just before the merge); the row naming who added it, in L7's ledger and L3's (or HANDOFF's lane row); a `reviewer` delta check on whichever lane adds the arm |
| Handoff §5, serialisation; Quinn's 2026-10-19 queue ruling | `surface.rs` L7 before L5; the `console.js` queue L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5; Tauri commands only after L6 | T4, T5, T8 | §5; Q9; no `app/src` diff |
| L6 owns `judge.rs` this week (L6 plan §5; L3 plan T3) | L7 cuts after L6 merges | T0 | Q8; the ledger's branch-point sha |
| CI's `eval-gate` (`ci.yml:205-208`, `:242`) | no unannounced model spend from this PR | T8 | the `eval_cases` count row; Q10 if non-zero |
| Compatibility | old and new engines and servers interoperate | T2a, T2b, T3a, T3b | T1's 5 and 9; T2a's plain-error and T2b's absent-key pins; T7 check 4 |

## 8. Checkpoints

1. **Plan review** (`reviewer`, plan mode), before T0. Round 1 failed (0/4/6). The round-2 re-review
   passed with 4 minors, closed in §11. r3 (Quinn's rulings, §11 Round 3) had a short delta review,
   which failed (0/2/5); its findings are applied (§11, Round 3 delta). A delta check of those lines
   passes before T0.
2. **Quinn**, one question at a time, each with its recommendation:
   - Q1 (where the issues live): **ruled 2026-10-19**, the Issues view only;
   - Q2 (the copy, and the clearing rule): **ruled 2026-10-19**, as written, no 2-day expiry;
   - Q3 (what quiet means): **ruled 2026-10-19**, no message of any kind for 7 days;
   - Q4 (live proof): open;
   - Q10, at T8 and only if staging's `eval_cases` count is non-zero (the eval gate's spend).
3. **Controller at dispatch:** Q6-Q9 (Q5 is withdrawn; Q8's default is to cut after L6), and T0's
   re-confirmations (the "unavailable" and "went_quiet" name check first, and (f), L3's T3).
4. **After T1:** the red run, one reason per test, as a ledger row. **After T5:** the fixture
   screenshot shown to Quinn with the PR link, as a look, not a gate.
5. **T7**, before the push. Then T8's staging deploy, merge on green CI and PASS, and the prod deploy
   handed to L9.

## 9. Out of scope, and follow-ups this lane hands on

- **The Anthropic provider's "credit balance is too low"** (Q7). It is a 400 with a body to recognise.
  No row is pinned there today. When one is, the same typed error and the same `unavailable` word
  apply, and that is a one-function change in `judge_anthropic.ts`.
- **An operator alert for Quinn** when the balance runs out (a Supabase log alert, or OpenRouter's own
  low-balance email). Students see the issue. Whether Quinn also gets paged is an ops choice, not this
  lane's.
- **`mail-process` stopping its batch on `unavailable`.** The breaker already bounds it at three
  failed calls (§3.2). It is a follow-up only if the logs show waste.
- **Naming out of credit on the Gmail and forwarded-mail paths on the device.** Those judgments run
  server-side, and their items are deferred and come back by themselves. The judge step's heads-up
  names the state for the whole account whenever a slot has anything to judge.
- **A "quiet" rule for Google Calendar.** The calendar feed has its own failure lines. Ruling 10 named
  Gmail only.
- **`google_accounts.last_read_at`.** It stays unwritten (§3.3). Whether to drop or use it is a later
  schema question, a migration, and not worth one for an MVP.
- **Promoting the source issues onto Today** (a line under the topline, or the tray). The Issues
  view's nav badge is the MVP's signal. Quinn can ask for more after seeing it (§8 checkpoint 4).

## 10. Proposed rule edits

None needed. No VISION sentence changes. CLAUDE.md's "no runtime, model, account, session,
entitlement or network is a named outcome" already covers an out-of-credit service ("no model").

**One optional proposal for Quinn, at the merge checkpoint.** L6's §10 item 2 proposes shortening the
same sentence to "no account, session, entitlement or network". If Quinn adopts that, the sentence no
longer names a service with no credit. The proposed wording is then:

> no account, session, entitlement, network or judgment-service credit is a named outcome

No agent applies it. It is put to Quinn together with L6's item 2.

## 11. Fidelity notes

Each round adds a dated subsection that answers its findings in the reviewer's order.

### Round 1 (2026-10-19): FAIL, 0 critical, 4 important, 6 minor

Report: `docs/reports/2026-10-19-l7-named-issues-plan-review.md`. Every recommended fix is adopted;
the one place this plan goes a different way is stated with its reason (finding 2's `maxResults`). Two
outside facts arrived with the round and are folded in: Quinn's 2026-10-19 queue ruling (L8 S5 ahead of
L4) and L3's ownership of `events.rs` edits.

**Important.**

1. **The eval gate does run on this PR.** Adopted. §1 "Money" now says T2a's edit to
   `judge_pipeline.ts` trips `ci.yml:242`, that the gate fails loud if staging cannot answer its count,
   and that it spends (bounded at 1,800 calls) only if `eval_cases` holds rows. T8 gains a step that
   counts staging's `eval_cases` before the push. A non-zero count stops the push for the new **Q10
   (Quinn)**, which recommends pushing anyway and notes L3's identical exposure. Fidelity ledger: a new
   `eval-gate` row.
2. **"Quiet" would first fire on day 21.** Adopted: the second list call. §3.3 is rewritten. When the
   7-day list is empty, `/gmail-read` lists again with `newer_than:14d` and the excluded labels kept,
   and answers `went_quiet: true` only if that is empty too and `connected_at` is at least 14 days old.
   - Dropped: the `last_read_at` writer (§1 Privacy, §3.3 Deps, T6, §9), Q5 (withdrawn, number kept),
     and T6's note to L4.
   - Added: T2b's test 2, `a_message_ten_days_old_is_not_quiet`, the review's required case; and test
     6, so a failed second call claims neither answer and writes no status.
   - **Not adopted as written: `maxResults` 1.** `GmailApi.list` hard-codes 100 per page in
     `index.ts`, so `maxResults=1` means a new parameter on the interface and its wiring. With an
     `unseen` that accepts every id, `pagedList` stops after one full page because `READ_CAP` (60) is
     below 100, so the existing call already costs one request and returns ids only. cloud-engineer may
     still add the parameter if `GmailApi`'s shape test stays green (§3.3, last bullet).
3. **T1's test 4 is green at T1.** Adopted. Test 4 is marked green at T1 as the regression pin for
   T3b's tier-3-only clear, and T1's acceptance lists it with the other green pins.
4. **T3 is too big.** Adopted, both splits.
   - T3a: `source_issues.rs`, `lib.rs`, `judge.rs` and `cloudmodel.rs`, about 80 non-test lines,
     turning T1's 6 and its own in-module tests green. Test 6 now lives in `cloudmodel.rs` only, so T3a
     alone can turn it green.
   - T3b: the two `enrich.rs` sites, about 40 lines, turning T1's 1-3, 7 and 8 green. Test 4 was already
     green; 9 is a pin.
   - T2a: the typed 402, `unavailable` and the gmail-read stop. T2b: `went_quiet`, the 14-day call and
     the status moves. They run in sequence because both edit `gmail-read/handler.ts`.
   - The header's Architecture list, the §5 conflict map, the §6 table, the task bodies and the §7
     ledger all use the new names.

**Minor.**

5. **`events.rs` prints one line per event on out of credit.** Adopted, coordinated with L3, which owns
   `events.rs` edits. §3.4 gains an `events.rs` bullet, and §5 gains a row. New task **T3c**: an
   `OutOfCredit` arm beside L3's `Capped` arm (L3 plan, "T3. The capped pass ends with one line"), with
   test 6b (no `not judged` lines, one summary ending in `OUT_OF_CREDIT_LABEL`).
   - Whichever lane lands second adds the arm, since only it has both the variant and the loop-ending
     arm.
   - If L3 is on `main` first, L7 runs T3c. T0(f), the end of T3b and T8's last rebase each check.
   - If L7 lands first, it leaves `events.rs` alone, and the controller hands L3's controller the arm
     and test 6b through a ledger row. L3's plan is not edited. The interim per-event lines are named
     as a residual, like L3's own T3-alone residual for the cap.
   - Test 6's assertion is narrowed to what L7 owns: no `/judge-event` request, and no `unsure` rescue.
6. **A quiet notice beside "re-connect from settings".** Adopted. §3.4 clears the heads-up on every
   `CloudError::Quiet(_)`, and leaves it alone only on `Transport`, `Status` and `Body` errors. T1's
   test 8 gains `a_revoked_reply_clears_it_and_keeps_todays_line` and a green pin,
   `a_transport_error_leaves_it_open`.
7. **Test 1's red could come from replies out of order.** Adopted, the first option. Test 1 asserts
   the loopback's request path list first, and its message prints the list. The recorded red reason at
   T1 is that path list. `multi_reply_loopback` stays as it is.
8. **The Deno line.** Adopted. §2 spells out CI's full test line, with `--allow-write=W/cloud/eval`
   and the second path (`ci.yml:162-167`), and says T2a, T2b and T8 each run it.
9. **Q8's branch point.** Adopted. "Cut after L6 merges" is the default in Q8, §2 and §6's T0 row
   ("After"). The reason recorded is that `judge.rs` is L6's, and L3 treats it so. The one exception is
   that T2a and T2b may start early, since L6 leaves `cloud/**` untouched. They rebase before T1.
10. **The ledger path.** Adopted: `.superpowers/sdd/2026-10-19-l7-named-issues/progress.md`, dated as
    the plan is.

**Outside facts folded in.**
- **Quinn's queue ruling (2026-10-19):** Q9, §5's `console.js` row and T5's "Before starting" now read
  L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5. L8 has no plan yet, so T5 re-checks `static_assets.rs` on
  rebase.
- **L3 owns `events.rs` edits:** see finding 5.

**Re-review scope.** §1 (Privacy, Money), §2 (branch, Deno), §3.3, §3.4 (the `Quiet(_)` and `events.rs`
bullets), §4 (Q5, Q8, Q9, Q10), §5, §6, T0, T1 (tests 1, 4, 6, 8 and acceptance), T2a, T2b, T3a, T3b,
T3c, T6, T7 (checks 4, 5, 5a), T8 (the rebase, gate and `eval_cases` steps), §7 and §8.

### Round 2 (2026-10-19): PASS, 4 minor

The short re-review of round 1's edits passed. Its four minors are closed below, in the reviewer's
order. No further round is needed. The status line and §8 checkpoint 1 now read REVIEWED.

1. **Decide on the raw 7-day list.** Adopted. §3.3 now reads the raw result of `deps.api.list`, before
   `.filter(unseen)` and the `READ_CAP` slice. A 7-day list that holds only already-seen ids is mail,
   so it never triggers the second call. New T2b test 10,
   `a_seven_day_list_of_seen_ids_is_not_quiet`, asserts that there is one list call, that
   `went_quiet` is false, and that a `quiet` row moves to `active`. T7 check 5a and the §7 quiet row
   cite it.
2. **The T3c hand-over could fall through.**
   - (a) **L3 merges after L7's last rebase.** Adopted. T8 gains a step that re-checks T0(f) just
     before the merge. If L3's T3 landed in that window, the controller either runs T3c with its
     `reviewer` delta check (CI runs again), or merges and files the arm as a named follow-up in L7's
     ledger and `HANDOFF.md`'s lane rows, owned by L7's controller. T0(f), §3.4 and §7 name the extra
     re-check.
   - (b) **L7 lands first.** Adopted. §3.4 and T3c now write the hand-off row into L3's own ledger
     (`.superpowers/sdd/2026-10-18-l3-first-day-allowance/progress.md`) as well as L7's, or into
     `HANDOFF.md`'s L3 lane row if that ledger is not open. The row says that the arm L3 adds gets a
     `reviewer` delta check before L3's push. L3's plan is still not edited. T7 check 5 and §7 follow.
3. **The second call's failure was silent.** Adopted. §3.3 logs it the way the outer catch does
   (`handler.ts:425-430`): one line, `gmail-read: GmailApiError <status>`, with the class and status
   only, never the message. T2b test 6 throws a `GmailApiError` whose message holds a sentinel. It
   captures `console.error` and pins that one line, and pins that the sentinel appears in no log line
   and nowhere in the reply.
4. **Q8's exception and T8's overclaim.**
   - Q8: if T2a and T2b start before L6 merges, the controller re-runs T0's baseline and re-greps
     (a)-(d) after the rebase onto L6, before T1, and records the second baseline.
   - T8: the `eval_cases` step now says the count shows only that staging answered at that moment,
     not that the gate's own call will answer.

**Outside fact folded in: the `console.js` queue (Quinn's 2026-10-19 ruling; L4 plan, Q-QUEUE).**
L8's S5 is ahead of L4's T8: **L6 → L8 S5 → L4 → L7 → #19 → L1 R11 → L5**.
- Q9 gives the new queue and puts T5 after L6, after L8's S5, and after L4. While L4's T13 waits for
  L8, the controller may let L7 take its slot ahead of L4 (L4 plan, Q-QUEUE).
- §5's `console.js` row already read `L6 → L8 S5 → L4 → L7 → #19 B5 → L1 R11 → L5` and is unchanged.
- T5's "Before starting" says to rebase onto whatever L6, L8 and L4 merged, and names the
  overtaking case.

### Round 3 (2026-10-19): Quinn's rulings on Q1-Q3

Quinn ruled on Q1-Q3 on 2026-10-19. Each ruling is recorded under its question in §4.
- **Q1:** the Issues view only, as recommended. No plan change.
- **Q2:** the copy as written, with no 2-day expiry. No plan change, apart from Q3's words.
- **Q3:** "quiet" means no message of any kind, as recommended, but over **7 days, not 14**. That is the
  one change of substance in r3.

**Changes for Q3.**
1. **§3.3, the rule.** `went_quiet: true` needs the **raw** 7-day list (before `.filter(unseen)` and
   the `READ_CAP` slice) to be empty and `connected_at` to be at least 7 days old. Checked in the code:
   `WINDOW` is `newer_than:7d` (EF `handler.ts:37`, pinned at `handler_test.ts:276`). The second,
   14-day list call is removed, along with everything that existed only for it: its query test (r2
   test 5), its own catch and log line (r2 test 6 and round 2, minor 3), its paging-cost analysis and
   the `maxResults` paragraph.
   - **Kept:** a raw list of only already-seen ids is mail; any list failure claims nothing (now the
     existing outer catch, with no new log line); `quiet` moves back to `active` on mail; `revoked` is
     untouched, since every write is conditional; and a grant without the Gmail scope is untouched,
     since the `no_gmail_scope` path returns before the list.
   - **Round 1, finding 2 (day 21) stays closed.** It came from a stamp that the 7-day list kept
     refreshing, so the rule lagged the window by its own length. The rule now reads the window
     itself, with no stamp, so it fires on the first read more than 7 days after the last message, and
     first on day 7 after connect. Nothing writes or reads `last_read_at`.
   - **Round 2, minor 1 (the raw list) stays closed.** The decision is still taken on the raw result of
     `deps.api.list`, now for the whole rule rather than as the trigger for a second call. T2b's test 2
     (`a_seven_day_list_of_seen_ids_is_not_quiet`) is kept, and goes red if the check reads the
     filtered list.
   - **Round 2, minor 3 (the second call's silent failure)** no longer applies: no second call exists.
     A failed list is today's HTTP 500 and today's log line, which test 5 pins.
   - Also updated for the rule: the header's Goal and Architecture (T2b), §1 (the VISION, clock,
     fetch-on-device and privacy bullets; the privacy line on the extra call is replaced by "no new
     Gmail call"), Q4's staging note, Q5's wording, §6's T2b row, T6's README and HANDOFF lines, and
     §8.
2. **T2b's tests, renumbered and rewritten:** 1, a 5-day-old message is mail; 2, only seen ids;
   3, `connected_at` at 6 days; 4, exactly 7 days (and 30); 5, a failed list claims nothing; 6, the
   conditional writes; 7, the forwarding path; 8, the not-connected replies. Every case asserts exactly
   one list call. The r2 tests 2 (the 10-day message), 5 (the 14-day query) and 6 (the second call's
   failure) are dropped. T2b's acceptance gains a `newer_than` grep. T7 check 5a and §7's quiet row
   cite the new numbers.
3. **§3.7, the copy.** The run line is `gmail: no mail found in the connected Gmail for 7 days`. The
   heads-up and Settings sentences say "for a week" in place of "for two weeks". Every other word is
   unchanged.
4. **§7's quiet row.** The source is now Quinn's ruling Q3 (2026-10-19), amending cloud design §5.3.
5. **The spec amendment.** `docs/specs/2026-09-09-knowlu-cloud-design.md` §5.3's Campus-accounts bullet
   gains an inline marker in the spec's existing style: *(amended 2026-10-19, Quinn (L7 Q3,
   `docs/plans/2026-10-19-l7-named-issues-plan.md` §4): quiet = no message of any kind for 7 days, first
   firing on day 7)* (the plan path added in the round 3 delta, minor 7). No other text in the spec is changed, and
   the uncommitted Amendment 2026-10-19 (privacy bump #1) at its end is untouched.
   - **The migration comment is not edited.** `20260911000200_google.sql:24` still says "14 days", but
     `migrations_test.ts` pins the file's SHA-256 ("the original google migration is unchanged",
     EF `:980-987`; "Applied migrations are never edited"). T2b's file list says so, and T6's README
     line records that the comment is superseded.
6. **Other "14"s.** Every mention of this rule now says 7 days or names the ruling: §1, the authority
   list, the Goal, Q4, Q5, T2b's commit message, T6 and §8. The HANDOFF quote in the authority list is
   kept as a quote, marked superseded, and T6 updates `HANDOFF.md` §3. D14 (the email spec decision)
   and the round-1 and round-2 entries above are history and are left as written.
7. **An observation, not a change: Testing mode** (reworded in the round 3 delta, minor 6). Until
   `gmail.readonly` is verified, refresh tokens die 7 days after consent, and a dead token takes the
   `revoked` path. `connected_at` is stamped after the code exchange, so the token dies before
   `connected_at` + 7 days and the rule cannot fire in Testing mode. At worst one slot of "gone quiet"
   could slip through, and the next read's `revoked` path clears it (§3.3). Under the 14-day rule it
   could not fire either. Question C2 below.

**Questions for the controller.**
- **C1. The stale migration comment.** Leave `google.sql:24`'s "14 days" in place, with T6's README
  line as the record (recommended), or add a new migration whose only content is a
  `comment on column google_accounts.status` carrying the 7 days? The second breaks the plan's "no
  migration" for a comment. Re-pinning the hash is not offered: it edits an existing assertion.
  **Controller ruled 2026-10-19: the README line; no migration.**
- **C2. Testing mode.** Is a Gmail quiet issue that cannot fire before verification acceptable for
  the MVP? Recommended: yes (the round 3 delta agrees). The forwarded-mailbox heads-ups (D14) are the
  MVP's live case, and the Gmail half is built and tested for the day grants outlive 7 days.
  **Controller ruled 2026-10-19: yes.**

**Review.** A short delta review is recommended before T0: §3.3, T2b (tests and Files), T7 check 5a,
§7's quiet row, §3.7's Gmail column and the §5.3 inline amendment. The change removes code paths
rather than adding them, but it moves a signed rule and renumbers tests that T7 and §7 cite.

### Round 3 delta (2026-10-19): FAIL, 0 critical, 2 important, 5 minor

Report: `docs/reports/2026-10-19-l7-named-issues-plan-review-r3.md`. The reviewer found the rule itself
correct against the handler. Every finding is applied in place, as the report specifies, with the
controller's rulings on important 2 and minors 5-7. A delta check of the lines below is pending.

| # | Finding | Applied where | Controller ruling |
|---|---|---|---|
| I1 | Nothing pins that a row already `quiet` keeps answering `went_quiet: true`; an answer taken from the write's effect would flicker the heads-up | §3.3 ("The answer is the rule, never the write's effect"); T2b test 4b, `a_row_already_quiet_still_answers_went_quiet_true`; §7's quiet row; T7 check 5a | — |
| I2 | No task commits the §5.3 marker | T0's first step: the controller commits the marker alone, docs-only, before T0, keeping the uncommitted price amendments out; T7 check 5a verifies it is on `main`; §7's quiet row names T0; the authority list says so | The controller commits it in its own docs-only commit, separate from the price amendments, before T0; T7 5a verifies |
| M3 | "Whole 24-hour days" against a millisecond comparison; 6 days does not catch rounding | T2b's preamble (whole days describe only fixtures; the handler never rounds); §3.3 names `7 * 86_400_000` with no rounding; T2b test 3b at 7 × 24 h − 1 ms; T7 check 5a | — |
| M4 | The preamble said every case makes exactly one list call; tests 7 and 8 make none | T2b's preamble: tests 1-6 (with 3b, 4b, 5b) assert one call, tests 7 and 8 assert none | — |
| M5 | The status write's order and failure were unspecified | §3.3 ("When the write runs"): straight after `deps.api.list` returns, before the loop; not caught on its own, so a failed write is HTTP 500, as `markSeen` and `enqueue` are; T2b test 5b; T7 check 5a | As applied: before the loop, and a failed write is a 500 |
| M6 | The Testing-mode claim overstated the edge | §3.3's Testing-mode bullet and §11 Round 3 item 7 reworded: `connected_at` is stamped after the code exchange, so the rule cannot fire in Testing mode, and at worst one slot slips before `revoked` clears it; C2 reworded; Q4's `went_quiet: true` half dropped (the staging check is the `false` half only, and still recommended skipped) | Reword both passages, and drop Q4's `true` half |
| M7 | Citations and wording | §3.3 cites `store_google_grant`'s live definition, `20260911000300_google_privileges.sql:58-66` (`:62`); the authority list quotes the marker in the spec's lowercase form with the plan path; §3.3 notes that `index.ts`'s 14 days (`seen`, EF `:124-131`) is the `gmail_seen` dedup window, not this rule; the spec marker at cloud design `:165` now names `docs/plans/2026-10-19-l7-named-issues-plan.md` §4, and nothing else in the spec changed | Also update the spec marker to name the plan path; touch nothing else in that file |

**Not changed.** The reviewer's "checked, nothing to report" items stand: no stale references, the
migration exclusion and C1's README note, and tests 2, 4 and 5 each go red against the plausible
wrong implementation. Round 3 item 5's "No other text in the spec is changed" still holds after M7:
the only spec edit is the marker itself, and it now carries the plan path.

### Whole-branch review T7 (2026-10-21): PASS (0 critical or important, minors below)

T5 and T7 ran on top of `l4/honest-page` (PR #55) by a controller ruling of 2026-10-21 (Quinn-delegated): L4's console was final, so L7 took its queue slot early.

| # | Sev. | Finding | Answer |
|---|---|---|---|
| 1 | minor | HANDOFF said T5 was still queued. | Updated at T8: the page is built. |
| 2 | minor | `mail::heads::close_unreported` (92216cf) is outside §3.4. | Controller ruling (a), 2026-10-20, in the lane ledger: a mailbox the service no longer reports closes its `mail-source:` heads-up; an offline, failed or malformed answer closes nothing, pinned by three `mail_pass` tests. Follow-up handed on: `cloudmodel::mailbox_of` drops an entry with an empty `address_id`, which would close that mailbox's heads-up in the same pass; the pass should skip when any reported entry fails to parse. |
| 3 | minor | Q2's rule keeps "Knowlu's AI checks are paused" up until a run makes a tier-3 call, so a student with nothing to judge sees it after the service is back. | Accepted as Q2 rules it. Follow-up handed on: clear the heads-up on a healthy probe answer, or have the server report that it is reachable without a model call. |
| 4 | minor | The order test matched the first mention of "sources", a comment. | It now asserts `innerHTML = srcs + p.rows.map` and `(p.sources || []).map`. |
| 5 | minor | Shipped and test files cite rulings by a person's name. | The shipped `console.js` comment now cites the plan's Q2 and Q3; test and function comments follow the repo's existing practice. |
