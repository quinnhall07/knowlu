# Email forwarding: the cloud follow-ups after PR #42 (build plan)

> **For agentic workers:** use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: FINAL.**
- **Review r1** FAILED (0 critical, 2 important, 8 minor;
  `docs/reports/2026-10-20-email-cloud-follow-ups-plan-review-r1.md`). All ten findings are closed in
  place (§10).
- **Review r2**, the delta, PASSED with 2 minors
  (`docs/reports/2026-10-20-email-cloud-follow-ups-plan-review-r2.md`). Both are applied in place (§10).
- **Quinn ruled Q1-Q5 on 2026-10-20**, taking every recommended option: Q1 both gates, Q2 (a) hold until
  the object's lifecycle ends, Q3 yes, Q4 yes, Q5 yes with one hour (§4).
- **The controller ruled C1-C3** (§4).
- The spec amendments the rulings imply are written by T7 (§4, "Spec amendments the rulings imply").

**Date:** 2026-10-20. **Branch:** `email-cloud-follow-ups`, cut from `origin/main` (`ba7eb0b`) by the
controller. **Worktree:** `.claude/worktrees/email-cloud-follow-ups`. **Ledger:**
`.superpowers/sdd/2026-10-20-email-cloud-follow-ups/progress.md`. **Executes:** `cloud-engineer` (Opus,
high), every code task. Nothing here touches `engine/`, `app/`, the contract list or a fixture.

**Authority, newest first.** The cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`) wins over
everything below it. Then the email spec (`docs/specs/2026-09-30-email-forwarding-design.md`, signed
2026-09-30, amendments through 2026-10-19), the email plan (`docs/plans/2026-09-30-email-forwarding-plan.md`),
`HANDOFF.md` §2-§3 (the email lane), and the migration rules M1-M8 (`docs/plans/2026-10-18-first-day-allowance-plan.md`
§2.1, mirrored in the L8 spec §8.4). The L7 ledger (`.superpowers/sdd/2026-10-19-l7-named-issues/progress.md`,
T2a row) is where the out-of-credit follow-up came from.

**Where the code was read.** Every `file:line` below is from the worktree
`.claude/worktrees/l7-named-issues`, which is `origin/main` (`ba7eb0b`) plus L7's two cloud commits
(`81e4b72` T2a, then T2b). Those two commits touch `gmail-read/` and `_shared/judge_*`. They do not touch
`mail-process/`, `mail/`, `mail-inbound/`, `mail-pull/` or `migrations/`, so citations there are
`origin/main`'s lines. **`gmail-read/handler.ts` citations are L7's lines.** `origin/main`'s lines differ by
L7's hunks. The main checkout's working tree is stale and was not used for code.

## 1. What the code does today (verified)

### 1.1 (b) `mail-process` spends model calls for an unentitled account: confirmed

- Intake already gates on entitlement. `mail-inbound/handler.ts:463-477` asks `store.entitled` once per
  account per delivery. An unentitled account's mailbox is marked `lapsed` (`markLapsed`,
  `mail-inbound/store.ts:96-114`) and the message is deleted unread. `entitled` is
  `isActive(lookup(account))`, from `_shared/entitlement.ts` (`mail-inbound/store.ts:95`).
- `mail-process` never reads entitlement. Nothing in `mail-process/` names it; a grep for `entitle` finds
  no hit. `processItem` (`mail-process/handler.ts:256-308`) leases the item and fetches the object. It
  screens the item, then hands what is left to the model path (`:305-307`).
- The model path charges and calls with no entitlement check. `mail-process/model.ts:186` writes the
  attempt. `:192-206` charge the allowance or the day's cap, and then `enforce_budget`. `:221-229` call
  the model. None of `charge_allowance`, `charge_call` or `enforce_budget` reads `entitlements`: only
  three migrations name that table (`20260910000100`, `20260910000200`, `20260917000100`: C1's and C1b's).
- So any item still in `mail_work` when the account lapses is judged and billed to us. That covers an
  item that arrived while entitled and waits (capped or over budget: `model.ts:137-144` sends it to the
  next UTC day), and one that arrived in the minute before the lapse.
- An item's lifetime is bounded by its S3 object. The nightly sweep expires work that arrived at or
  before the start of yesterday (UTC) (`mail-process/sweep.ts:45-49, 63-74`). Expiry goes through
  `expire_mail_work`, which deletes the `mail_seen` row so a later recover is accepted (D8, spec line 177).
  Spec §4.6 step 3 (lines 331-336) promises "a lapse loses nothing for good while the saved session
  works". The device reconciles the last three days, or back to `lapsed_since` after a lapse (§4.11
  step 3, lines 644-646).

### 1.2 (c) Rotate sends a Gmail mailbox back to `pending`, which turns the Gmail read back on: confirmed

- `mail/handler.ts:372`: Rotate writes `status: row.provider === "google" ? "pending" : null`, through
  `mail/store.ts:171-183` (one guarded PATCH on `token_hmac`).
- `gmail-read/handler.ts:203` (L7 lines): D22's allow-list is `["confirmed", "active", "quiet"]`.
  `forwardingCounts` (`:210-215`) matches the OAuth address to a mailbox whose status is on that list.
  When a mailbox matches, the D22 branch (`:322-337`) reads nothing new.
- So, from Rotate until the new address is confirmed, a rotated Gmail mailbox reads `pending` and
  `gmail-read` reads that Gmail by OAuth. Meanwhile the old forward still delivers through the retired
  token, which binds for up to seven days (D3, spec line 172; `mail-inbound/handler.ts:262-266`). Every
  message in that window can be filed twice, as `gmail:<id>` and as `mail:<hash>`: the two paths share no
  key (spec §4.11 step 3's 2026-10-19 note).
- **The status has to stay `pending`.** The device's Gmail confirm step polls the row until it reads
  `confirmed` or `active` (`app/src/mail/runner.rs:796-818`). A Rotate that kept `active` would pass that
  wait at once. The forwarding script would then try to select a new address Gmail has not verified. So
  the fix belongs on the read side, and keeping the status is rejected.
- Intake moves `pending` to `confirmed` on Gmail's confirmation, and to `active` when a canary comes back
  through the current token (`mail-inbound/store.ts:186-193, 210-213`). That canary also ends the grace
  (`mail-inbound/handler.ts:399`, `store.ts:204-209`).

### 1.3 (d) An SNS redelivery double-counts `received`: confirmed

- `batchSend` (`mail-inbound/handler.ts:437-459`) adds `originals.length` to the batch's `received` on
  every delivery (`:456`). It counts duplicates too, by design ("the device's count of what it sent",
  `:435-436`).
- `batchReceived` (`mail-inbound/store.ts:214-229`) reads `received` and then writes `received + n`.
  Nothing marks that a delivery was already counted.
- **Two paths double the count:**
  1. **A failure after the count.** The final `s3.delete(n.key)` (`handler.ts:482`) fails, or a later
     recipient's `arrive` throws. The handler then answers 500 (`:506-512`), SNS redelivers, and the
     batch is counted again.
  2. **Two deliveries at once.** A large backfill send (spec D8: about 100 originals, about 25 MB)
     outlasts SNS's delivery timeout, so SNS redelivers while the first run is still working. Both runs
     read the object and hand off. `mail_hand_off` keeps one work row per message, but both runs call
     `batchReceived`.
- **The harm.** `mail-pull` closes a batch when `received` reaches the device's `sent`
  (`mail-pull/handler.ts:10-13`). A double count closes it early. Intake binds a batch send only to an
  *open* batch's nonce (`mail-inbound/store.ts:115-122`, `closed_at=is.null`). So the batch's later sends
  arrive unbound, are deleted, and are counted `unbound`.

### 1.4 The SHA-256 pin for `20261020000100_mail_max_tokens_repin.sql`: missing

- `migrations/migrations_test.ts:1003-1029` pins three applied migrations by SHA-256: the original
  google migration, the mail migration and `20261005000100`. Each pin has three lines: read the bytes,
  digest them, compare the hex.
- The re-pin file is applied on staging (L3 ledger, T0 row: staging is applied through `20261020000100`).
  Today its only guard is the statement pin at `:1479-1504`. That pin strips comments, so the file's
  bytes can change without failing it.

### 1.5 (L7 T2a) No breaker for an out-of-credit provider: confirmed, and narrower than the ledger says

- L7's T2a (`81e4b72`, on the L7 branch, **not on `main`**) makes OpenRouter's 402 throw
  `ProviderOutOfCredit` (`_shared/judge_anthropic.ts:79`, `_shared/judge_openrouter.ts:96`).
- In `mail-process`, `failureCause` (`model.ts:146-150`) reads that error as `model failed`. The charge
  is given back (`:234-238`). The attempt was already written (`:186`), and the item waits for the next
  UTC day (`:137-144`); `MAX_ATTEMPTS` is 3 (`:125`). The 402 itself costs nothing.
- **The narrower reading.** The sweep expires an item once its arrival is at or before the start of
  yesterday, UTC (§1.1). Take an item that arrives on day 0. Attempt 1 runs on day 0, attempt 2 at
  day 1 00:00, and attempt 3 at day 2 00:00, which is the same instant the sweep's bound passes. So the
  third attempt and the sweep race, and whichever runs first decides. Spent attempts are not the main
  cost. The main cost is that mail waits for the next UTC day even when credit is topped up within the
  hour, and an outage longer than about a day expires items. Expired items come back only through
  recover.
- **The fix reaches `mail_claim`.** Only OpenRouter throws `ProviderOutOfCredit` (L7's
  `judge_openrouter.ts:96`). `mail_claim`'s pin is OpenRouter (`20261002000200_mail.sql:343-353`:
  provider `openrouter`, `qwen/qwen3.5-35b-a3b`, route DeepInfra, no fallbacks, zero data retention).
  The two migrations since then (`20261005000100`, `20261020000100`) move only `max_tokens`, so the pin
  is OpenRouter on `origin/main` and T6 applies. If a later re-pin moved `mail_claim` to direct
  Anthropic, T6 would have no effect there: `AnthropicModel` never throws `ProviderOutOfCredit`.

## 2. What this lane does, and why (each subject to §4)

| Item | Decision | Reason |
|---|---|---|
| (b) gate | **Both: keep intake's gate as it is, and add a gate in `mail-process`.** The new gate runs right after the lease and before the object is read. An unentitled account's item is touched by nothing but a hold and the sweep (T3). | Money: no model call and no charge for an account that is not `active` or `trialing`. A gate only before the charge would still parse, screen and queue template claims for an unpaid account. Gating before the read keeps that account's mail unread, as intake does. Using the same `isActive` as intake means L8's comp grants reach `mail-process` with no further change. |
| (b) queued mail | **Hold it until its object's lifecycle ends** (Q2). The item is not read, charged or attempted. It is re-checked hourly (`LAPSED_HOLD_MS`) and processed as usual if the account renews before the sweep. Otherwise the sweep expires it through D8's path, which deletes the seen row so recover can bring the message back. The mailbox is also **marked lapsed from its oldest held item's arrival**, once per run (Q3), so the device's reconcile reaches back over it. | The hold keeps nothing longer than the page already says ("up to about two days", spec §7). It loses nothing for a student who renews within that time, even with the saved sign-in switched off. Without the mark, a lapse longer than three days (the reconcile window) can lose a held item for good: intake's own mark stamps the first arrival *during* the lapse, which is later than the held item's arrival. |
| (c) | **Record the status before the Rotate (`rotated_from`), and widen D22's reading** (Q4). A Gmail row that reads `pending` inside an unexpired Rotate grace counts as forwarding, if its `rotated_from` was on the allow-list. Rotate still writes `pending`, because the device's confirm wait needs it (§1.2). The grace ending by canary clears `rotated_from` too, for tidiness. A stale value is harmless: D22 reads it only while `retired_until` is in the future. | The old forward still delivers during the grace, so forwarding owns the mailbox. A mailbox that was `pending`, `blocked`, `off` or `lapsed` before the Rotate did not count then and does not count now. When the grace runs out with the new address unconfirmed, the OAuth read comes back on its own, because forwarding has stopped. |
| (d) | **Count each SES receipt once per batch.** The receipt's id is the SHA-256 of the notification's object key. One SQL statement adds `n` and records the id, only while the id is absent. This replaces the read-then-write loop (T5). | An SNS redelivery carries the same object key, and a different send carries a different one. A single guarded `update` is atomic under a row lock: a concurrent second delivery re-reads the row and finds the id there. Hashing keeps object keys out of the table, as the store's privacy notes require (`mail-inbound/handler.ts` header). |
| Pin | **Add the SHA-256 pin** in the existing three-line style (T1). | The file is applied on staging (M6: frozen), and the statement pin does not cover its bytes. |
| Out of credit | **Fix it in this lane, after L7 merges** (Q5, C3). On a `ProviderOutOfCredit` answer, the charge is given back as today, but **no attempt is spent** and the item waits one hour, not until the next UTC day. For that hour, the same instance holds every item it is handed, with no charge, no call and no attempt. | Mail resumes within about an hour of a top-up rather than the next UTC day. Spending attempts on our outage is wrong in principle. The breaker stops one 402 call per item. It needs L7's `ProviderOutOfCredit`; matching an error's text is what L7 ruled out. |

**VISION and the two overriding rules.**
- *No single-user assumptions:* every change is per account and per mailbox, and nothing names a person,
  vault, machine or credential.
- *Frozen references:* no fixture is touched, and no applied migration is edited. The lane's SQL goes in
  one new file (M6).
- *VISION:* "Failures are visible; silence is never ambiguous". A held or out-of-credit item has a run
  count (`lapsed`, `out of credit`), and a lapsed mailbox is the named `lapsed` state in Settings.
  "$9.99 … no free tier" is what (b) enforces. "Data minimization" holds: nothing is read for an unpaid
  account, and the hold keeps only what the page already discloses.
- *Not affected:* `rank` never calls a model, the device's wire shapes do not change, and no app or
  engine change is needed.

## 3. Global constraints (every task)

- **Agent.** `cloud-engineer`, Opus `high` (CLAUDE.md: a silent error here moves money or leaks student
  mail). Nothing is on the contract list. The lane imports `_shared/entitlement.ts` but never edits it or
  its test (they are L8's, and "the entitlement tests" are on the contract list).
- **Tests first, in Deno, with injected fakes.** No network: no `fetch` leaves the process, and a store
  test drives PostgREST through an injected `rest.fetch`, as `mail-inbound/store_test.ts` does. Every
  task records its red run (the failing names) and then its green run in the ledger.
- **Gates for every task, from the repo root** (L7's ledger, T2b row: from another directory, 11 eval
  tests fail on a relative path):
  - `deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts cloud/eval/*.ts`
  - `deno lint --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
  - CI's full line, `ci.yml:167`:
    `deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
    The baseline at `ba7eb0b` is 1096 passed, 0 failed (L7 T0). Each task records its passed and failed
    counts. Zero lint problems and zero check errors are part of green.
- **Migration (T2).**
  - One new file, `<stamp>_mail_follow_ups.sql`, stamped under M2 and M3. The stamp is the next free one
    after the greatest of: every migration on `main`, every stamp recorded by a lane ahead, every version
    staging has applied. L8's `20261020000300` counts in the second set **only if L8's S2a fixes it
    today** (C1). If L8 does not, the stamp follows whatever is greatest in the three sets on the day T0
    runs.
  - If L8's stamp stands and this one is fixed on 2026-10-20, it is `20261020000400`. If it is fixed on a
    later day D, it is `D000100`, provided that sorts after the greatest of those sets.
  - T0 records the stamp in the ledger before T2 creates the file. M5's re-check runs before the first
    staging push.
  - Tests find the file by its suffix, `^\d{14}_mail_follow_ups\.sql$`, and never by its stamp (M4).
  - Applied files are never edited: `20261002000200_mail.sql`, `20261005000100`, `20261019000100` and
    `20261020000100` stay byte-for-byte.
  - **The file never contains the word `models`, in any case** (comments included). `ci.yml:246-247`
    runs the paid eval gate for any changed migration that matches `grep -qiE '\bmodels\b'`. No task
    touches `_shared/judge_{prompts,models,validate,pipeline}.ts` (`:242`).
- **Privacy.** Every new log line is a fixed phrase. No address, object key, receipt hash, token or
  status value is logged, put in an error, or answered.
- **Staging and prod.** The controller makes every staging push and deploy (T9). Prod is L9's, on Quinn's
  word. Order on staging: the migration first, then the functions. The new functions write a new column
  and call new RPCs. The old functions keep working on the new schema: both columns are nullable or
  defaulted, and nothing old calls the new RPCs.

## 4. Questions and rulings

### Quinn's rulings (2026-10-20; recorded, no longer open)

| Q | Ruling |
|---|---|
| Q1 (money) | **Both gates:** intake's stays, and `mail-process` gains one before the object is read (T3). |
| Q2 (money and privacy) | **(a) Hold until the object's lifecycle ends** (T3). |
| Q3 | **Yes:** mark the mailbox lapsed from the oldest held item's arrival (T2 item 4, T3). |
| Q4 | **Yes:** a rotated Gmail keeps the OAuth read off during its grace, when its prior status counted (T2 item 1, T4). |
| Q5 (money-adjacent) | **Yes, with one hour:** no attempt spent, a one-hour hold, and a per-instance breaker (T6; C3 applies). |

Every ruling is the recommended option, and the hold values are `LAPSED_HOLD_MS = OUT_OF_CREDIT_HOLD_MS =`
one hour. The questions below are kept as Quinn read them.

### The questions as asked (Q1, Q2 and Q5 touch money)

- **Q1 (money). Where does the entitlement gate go?**
  - *Recommendation: both.* Keep intake's gate, which already deletes an unpaid account's new mail
    unread, and add one in `mail-process` before the object is read.
  - Process alone would let intake store mail for an unpaid account. Receipt alone is today's gap.
  - **Accepted residual:** the process gate reads entitlement once per account per run, and a run lasts
    up to 50 seconds. An account that lapses in the middle of a run can have that run's remaining items
    charged and judged: at most one run's worth.
  - Signing this adds a sentence to spec §4.7: "An item whose account is not `active` or `trialing` is
    held, never charged or judged".
- **Q2 (money and privacy). What happens to mail already queued when an account lapses?**
  - (a) **Held until its object's lifecycle ends** (about two days at most, the bound the page
    states). It is processed if the account renews in time; otherwise it expires through D8, and recover
    can bring it back.
  - (b) **Dropped at once.** It is expired now, the seen row is deleted, and nothing is held.
  - (c) **Kept past the lifecycle.** This is not offered: it would mean changing the bucket's one-day
    rule and the privacy page's "about two days".
  - *Recommendation: (a).* It costs nothing, since no call is made while the account is unpaid. It keeps
    nothing beyond the stated bound, and it loses nothing for a student who renews within a day or two,
    even with the saved sign-in switched off. (b) deletes sooner, but turns a short card retry into lost
    mail for every student whose saved sign-in is off.
- **Q3. Should a held item mark its mailbox `lapsed` back to the item's arrival?**
  - *Recommendation: yes.* Once per mailbox per run, from the oldest held item's arrival, `lapsed_since`
    would move earlier when that item arrived before the current stamp. `last_lapsed_at` would move too,
    which voids any `covered` report already in flight.
  - Without the mark, a lapse longer than the three-day reconcile window loses a held item for good. That
    breaks §4.6 step 3's "a lapse loses nothing for good while the saved session works".
  - It amends §4.6 step 3, and goes with Q2 (a) or (b) alike.
  - **Accepted residuals** (§9):
    - a held *backfill* or *recover* item is marked from its batch send's arrival, not the original's
      date;
    - a renewal in the middle of a run (up to 50 seconds) can be marked lapsed from the run's stale
      answer. That mailbox then comes back `quiet` or `pending` at its next pull, as after any lapse;
    - *(added after review r2)* an item the sweep expires before any run has held it is never marked.
      The sweep and the item's last due attempt can fall at the same instant (§1.5), and if the sweep
      wins, the item goes without a mark. A renewal later than the three-day reconcile window then
      cannot reach it. The mark itself runs from a `finally`, so a run that fails after a hold still
      marks (T3).
- **Q4. Should a rotated Gmail mailbox keep the Gmail read off during its grace?**
  - *Recommendation: yes, as §2 states.* This amends D22's PQ11 allow-list: a `pending` row inside an
    unexpired Rotate grace counts as forwarding when the status it was rotated from counted.
  - Today every message in the grace can be filed twice.
  - The alternative, keeping the status on Rotate, breaks the device's confirm wait (§1.2) and is not
    offered.
- **Q5 (money-adjacent). On an out-of-credit answer: no attempt spent, a one-hour hold, and a breaker for
  that instance?**
  - *Recommendation: yes, in this lane once L7's cloud half is on `main`.* It amends §4.7 step 5 ("at
    most three attempts" is no longer charged for our outage).
  - It spends nothing. A 402 costs nothing, and the breaker cuts the number of 402 calls.
  - It reaches email extraction because `mail_claim` is pinned to OpenRouter, the only provider that
    signals out of credit (§1.5).
  - The one-hour value is shared with Q2's hold (`LAPSED_HOLD_MS`, `OUT_OF_CREDIT_HOLD_MS`, both 1 hour).
    Quinn may set a different value.

### The controller's rulings (2026-10-20; recorded, no longer open)

- **C1. Where this lane sits in M1's order: ruled.** Any lane ahead in the migration order that has
  neither merged nor pushed is recorded as slipped behind this one (M7, "Clearing"), in the session
  ledger and this lane's ledger, before T9's push. The lanes behind re-check under M5. **L8's
  `20261020000300` is a floor only if L8's S2a fixes it today.** If it does not, T0 takes the greatest of
  M3's three sets as they stand.
- **C2. Should T1 also pin `20261019000100_mail_last_lapsed_at.sql`? Ruled yes.** It is applied on
  staging, and only its statements are pinned (`migrations_test.ts:1861-1885`).
- **C3. What if L7 has not merged when T1-T5 pass review? Ruled:** open the PR without T6. T6 then runs
  as its own small PR after L7 merges, on `email-cloud-follow-ups-credit` cut from `main`. T6's ledger row
  and the `HANDOFF.md` lane row name it as owed.

### Spec amendments the rulings imply (T7 writes them)

Each is a dated italic marker, `*(amended 2026-10-20, Quinn: …)*`, in
`docs/specs/2026-09-30-email-forwarding-design.md` at the place it governs, in the spec's own style. The
marker carries only the ruled behaviour as this table states it. **T7 (`docs-keeper`) writes them all.**
Each marker lands in the commit of the task whose code it describes, or later in the same PR, so the
spec never runs ahead of `main`. If T6 splits off (C3), its marker goes with T6's PR.

| Ruling | Spec place | What the marker says |
|---|---|---|
| Q1 | §4.7, a new first sentence before step 1 | `mail-process` reads the account's entitlement before it reads the object. An item of an account neither `active` nor `trialing` is held, never parsed, charged or judged; intake's gate (§4.6 step 3) stays. |
| Q2, Q3 | §4.6 step 3, after the 2026-10-19 amendment | A held item waits until its object's lifecycle ends (one-hour re-checks). It is processed if the account renews in time, and otherwise expires through D8. Each run marks the item's mailbox `lapsed`, moving `lapsed_since` back to the oldest held arrival (never later), with `last_lapsed_at` set; `off` and `revoked` are untouched. The residuals: Q3's three, and §9's rotated-Gmail one. |
| Q4 | D22, after PQ11's amendment | The allow-list also counts a `pending` Gmail mailbox inside an unexpired Rotate grace whose status before the Rotate (`rotated_from`) was on the list. |
| Q5 | §4.7 step 5 | An out-of-credit answer from the provider gives the charge back, spends no attempt, and waits one hour. For that hour the instance makes no further call. "At most three attempts" counts only other failures. |
| (d), no ruling needed | §4.6 step 5, the batch-send bullet | A fix to the spec's intent, recorded rather than amended: each SES receipt counts toward `received` once. |

## 5. Conflict map (files other lanes also touch)

| File | Other lane | Rule |
|---|---|---|
| `cloud/supabase/migrations/migrations_test.ts` | L3 (parse count 40 to 42; the lapsed-at order pin), L8 (identifier guard) | M4: recount the shared pins on the rebased tree. The parse count is the sum on that tree, never either side's literal. The last lane to land owns the final numbers. The lapsed-at order pin (`:1866-1871`) gains this lane's file, found by suffix. |
| `gmail-read/handler.ts`, `gmail-read/handler_test.ts` | L7 (T2a, T2b; not merged), L3 (T2a edits `handler_test.ts`) | Whichever lane merges second rebases. T4 edits only `forwardingCounts`, the `Forwarding` type and their tests, so conflicts stay small. Re-read L7's lines before editing. |
| `gmail-read/index.ts` | L7 (T2b, `connectedAt`) | As above; T4 changes one `select` list. |
| `mail-process/model.ts`, `worker_test.ts` | L7 (T2a: "mail-process needs no change") | No overlap today. T6 imports L7's `ProviderOutOfCredit` and runs only once it is on `main`. |
| `_shared/entitlement.ts` | L8 (comp grants) | Imported, never edited. |
| `account/mail_test.ts` | none known | T5 adds one name to the secret-column list. |

Within this lane: T3 and T6 both edit `mail-process/{handler,model,index}.ts` and `worker_test.ts`, so
they run in order (T3, then T6). T2 and T1 both edit `migrations/migrations_test.ts`, so T1 runs first.
T4 and T5 both edit `mail-inbound/store.ts` and `store_test.ts`, so they run in order (T4, then T5).
Everything else is disjoint.

## 6. Tasks at a glance

| # | Task | Agent | Why that agent | After |
|---|---|---|---|---|
| T0 | Branch, ledger, stamp, baseline | main session | Branch cutting, stamps and ledgers are the controller's (M3) | — (Q1-Q5 ruled 2026-10-20) |
| T1 | SHA-256 pin(s) | `cloud-engineer` | A test in the migrations suite. Could be `mechanical`, but it shares the file with T2 and needs the bytes' provenance checked | T0 |
| T2 | The migration and its textual tests | `cloud-engineer` | SQL on money and mail tables | T1, Q1-Q4 |
| T3 | (b) the entitlement gate and the hold | `cloud-engineer` | Money path (`cloud/`) | T2 |
| T4 | (c) Rotate and D22 | `cloud-engineer` | Three functions, one wire rule | T2 |
| T5 | (d) one count per receipt | `cloud-engineer` | Intake's write path | T4 (`mail-inbound/store.ts`) |
| T6 | Out of credit: no attempt spent, the breaker | `cloud-engineer` | Money path; needs L7 on `main` | T3, L7 merged, Q5 |
| T7 | Docs: amendment markers, HANDOFF row | `docs-keeper` | `HANDOFF.md` and the spec's dated markers, only text Quinn signed | T3-T6 |
| T8 | Whole-branch review | `reviewer` | Every change goes through it before a push (this plan gets its own plan-mode review before T0) | T7 |
| T9 | Gate, M5 re-check, staging, PR, merge | main session | Staging pushes and merges are the controller's | T8 PASS |

## T0. Branch, ledger, stamp, baseline (main session)

- [ ] Cut `email-cloud-follow-ups` from `origin/main` into `.claude/worktrees/email-cloud-follow-ups`.
      Open the ledger with this plan's path and the base sha.
- [ ] Copy Quinn's rulings on Q1-Q5 (2026-10-20, all as recommended) and the controller's rulings
      C1-C3 into the ledger, one dated row each (§4). Every task in this plan is in scope.
- [ ] **Fix the stamp (M3).** Take the greatest of three sets:
  - every file in `cloud/supabase/migrations/` on `origin/main`;
  - every stamp recorded in the L3, L8, L5 and L1 ledgers. L8's `20261020000300` counts only if L8's
    S2a recorded it today. Re-read L8's ledger at this step; do not trust this plan's value.
  - every version staging has applied (`supabase migration list`, read-only).

  Write the next free stamp after that greatest value (M2) in the ledger, with the three greatest
  values seen, and pass it to every lane behind.
- [ ] Baseline: §3's three Deno lines from the worktree root. Expect 1096 passed and 0 failed at
      `ba7eb0b`; record what is seen.

**Done:** the branch exists; the ledger has the answers, the stamp and the baseline.

## T1. The SHA-256 pin(s) (cloud-engineer)

**Files:** `cloud/supabase/migrations/migrations_test.ts` only.

- [ ] Find the commit that added `20261020000100_mail_max_tokens_repin.sql`:
      `git log --format=%H origin/main -- cloud/supabase/migrations/20261020000100_mail_max_tokens_repin.sql`.
      **More than one commit: stop and report** (the controller decides which bytes staging holds).
- [ ] Red first: add `the mail max_tokens re-pin migration is unchanged` beside `:1021-1029`, in the
      same three lines (read the bytes, digest them, compare the hex). Assert sixty-four zeros, and record
      the failure and the hex it prints.
- [ ] Check that hex independently: `git cat-file blob <sha>:<path>`, piped to `sha256sum` in Git Bash.
      The two must be equal; if they differ, stop (line endings: migrations are LF). Then pin the hex. The
      comment names the file, the commit and "its only commit. Applied on staging; never edited; a change
      is a new file", as `:1022-1024` do.
- [ ] C2 (ruled yes): the same steps for `20261019000100_mail_last_lapsed_at.sql`, in a test named `the
      mail last_lapsed_at migration is unchanged`.

**Done:** both pins are green with the hex `git` gave, and §3's gates are green (the test count is the
baseline plus two).

## T2. The migration (cloud-engineer)

**Files:** a new file, `cloud/supabase/migrations/<T0's stamp>_mail_follow_ups.sql`, and
`cloud/supabase/migrations/migrations_test.ts`.

**What the file holds**, in this order. It is forward-only, and it creates no table, so it needs no new
row-level security. Comments must not use the word `models`, in any case (§3).

1. *(Q4)* `mail_addresses.rotated_from text`, nullable, with one named check,
   `mail_addresses_rotated_from_status`: null, or one of the nine statuses of
   `20261002000200_mail.sql:57`.
   - **No check ties it to the grace** (review r1, M7: dropped).
   - Such a check would make the deploy order load-bearing. Between the migration and `mail-inbound`'s
     redeploy, today's `endRotateGrace` (`mail-inbound/store.ts:204-209`) would fail on any row a new
     Rotate had already marked. Intake would answer 500. SNS's retry would read as a canary replay
     (`mail-inbound/handler.ts:392-395`), so the grace would not end early.
   - D22's read already ignores a stale `rotated_from`: it counts the row only while `retired_until`
     is after now (T4). So the check would buy nothing.
2. `mail_batches.counted_receipts text[] not null default '{}'`: the receipt ids already counted.
3. `count_batch_receipt(p_batch uuid, p_address uuid, p_receipt text, p_n integer) returns boolean`:
   one guarded update. It answers true only when it counted.
   ```sql
   update public.mail_batches
      set received = received + p_n, counted_receipts = counted_receipts || p_receipt
    where id = p_batch and address_id = p_address
      and p_n >= 0 and p_receipt ~ '^[0-9a-f]{64}$'
      and not (p_receipt = any (counted_receipts));
   ```
4. *(Q3)* `mail_mark_lapsed_from(p_address uuid, p_since timestamptz) returns boolean`: one guarded
   update. It answers true when it wrote.
   ```sql
   update public.mail_addresses
      set status = 'lapsed', lapsed_since = least(lapsed_since, p_since), last_lapsed_at = now()
    where id = p_address and status not in ('revoked', 'off') and p_since <= now()
      and (status <> 'lapsed' or lapsed_since is null or lapsed_since > p_since);
   ```
   - `least` skips a null, so an unstamped row takes `p_since`, and an earlier stamp is kept.
   - A row already `lapsed` is rewritten only when the stamp moves earlier. Its `last_lapsed_at` then
     moves too, which voids a `covered` already in flight (spec §4.11 step 3).
   - `off` and `revoked` are never touched, as intake's mark leaves them (`mail-inbound/store.ts:101-106`).
   - The `mail_addresses_last_lapse_stamped` check holds.

Both functions take the pattern every function in the mail migration uses (review r1, I1; for example
`charge_allowance`, `20261002000200_mail.sql:400-426`, and `count_mail_arrival`, `:539-558`):
- `language plpgsql`, **`security invoker`**, `set search_path = public, extensions`;
- the update ends `returning true into <var>`, then `return coalesce(<var>, false)`;
- `revoke execute … from public, anon, authenticated` and `grant execute … to service_role`.

They are **not** `security definer`. The only caller is `service_role`, which needs no privilege
escalation. A definer function would be the first in the mail migrations, and would bypass RLS for no
gain.

**Tests, written first** (`migrations_test.ts`; textual, like the suite; nothing applies SQL):
1. `the_follow_ups_migration_is_found_by_suffix_and_stamped_after_every_other_file`.
2. `it_adds_rotated_from_with_its_status_check_and_counted_receipts_defaulting_empty`: the statements,
   exact after the comments come off, as `:1874-1884` does.
3. `count_batch_receipt_is_one_guarded_update_with_no_read_before_it`: one write, no `select`, and a
   `where` that refuses a receipt already counted.
4. `mail_mark_lapsed_from_never_touches_off_or_revoked_and_only_moves_the_stamp_earlier`.
5. `both_functions_are_invoker_with_public_extensions_revoked_from_clients_and_granted_to_service_role`.
   It asserts all four, and that neither body says `security definer`.
6. `the_follow_ups_migration_never_names_the_table_the_eval_gate_watches`. The test is `/\bmodels\b/i`
   over the **raw** text, comments included. It is case-insensitive because CI greps with
   `grep -qiE` (`ci.yml:247`).

Then recount the shared pins on this tree (M4):
- the parse count at `:333` gains two;
- the lapsed-at order pin (`:1866-1871`) lists this file, found by suffix.

Every applied file's pin stays green.

**Done:** red, then green, both recorded; §3's gates green; the stamp in the file name is T0's. Items 1
and 4 are in, since Quinn ruled Q3 and Q4 yes.

## T3. (b) The entitlement gate and the hold (cloud-engineer)

**Files:**
- `cloud/supabase/functions/mail-process/handler.ts`
- `cloud/supabase/functions/mail-process/index.ts`
- `cloud/supabase/functions/mail-process/handler_test.ts`
- `cloud/supabase/functions/mail-process/worker_test.ts`, for its `FakeStore` (`:187`), which implements
  `ProcessStore` too

**Behaviour.**
- `ProcessStore` gains two methods (review r1, M2: no new `hold`; the existing `store.wait`,
  `mail-process/index.ts:215-218`, already writes `next_attempt_at` and `attempts`):
  - `entitled(accountId)`: in `index.ts`, `isActive(await lookupFrom(rest)(accountId))`, exactly as
    `mail-inbound/store.ts:95` wires it;
  - `markLapsed(addressId, sinceIso)`: the `mail_mark_lapsed_from` RPC.
- **The read.** The run's per-account cache (`handler.ts:341-356`) also reads `entitled`, once per
  account per run. A failed read evicts the entry, as the cache already does.
- **The gate**, in `processItem`, right after the lease (`:262`) and before anything else, the `live`
  `unauthenticated` short-circuit included. When the account is not entitled:
  - the item is held: `store.wait(item.id, now + LAPSED_HOLD_MS, item.attempts)`. `LAPSED_HOLD_MS` is
    exported, at one hour. `attempts` is written back unchanged;
  - the item's mailbox and `arrivedAt` are noted for the run's mark (below);
  - the outcome is a new `lapsed`, counted in the run's answer;
  - the log is one fixed phrase: `account not entitled, held`;
  - **nothing else happens:** no S3 get, no parse, no rules read, no charge, no model call, no queue, and
    no change to `mail_seen`.
- **The mark (Q3), once per account per run** (review r1, M1).
  - `run` makes one `markLapsed` call for each mailbox of an unentitled account that had a held item,
    from the **smallest** held `arrivedAt` of that mailbox in this run.
  - **The marks run in a `finally` around the loop** (review r2, minor 1). A later round's `due` that
    throws (`handler.ts:359`), or any other failure that makes `handle` answer 500, still leaves every
    hold this run wrote marked. A mark's own failure inside the `finally` is caught and logged, so it
    never hides the run's error.
  - So an account costs at most one call per held mailbox per run, never one per item. A held item comes
    due again only hourly, so the repeat rate is about one call per held mailbox per hour, and the RPC
    writes nothing unless the stamp moves.
  - A failed mark logs a fixed phrase and counts `failed`. The holds stand, and the next run marks again.
- **Accepted residuals** (review r1, M1; named in Q1, Q3 and §9):
  - **A lapse in the middle of a run is charged for one run.** The entitlement read is cached for the
    run (up to `RUN_BUDGET_MS`, 50 s). An account that lapses mid-run can have that run's later items
    charged and judged.
  - **A renewal in the middle of a run can be marked lapsed.** A stale `false` from the cache can mark a
    just-renewed mailbox. The next entitled `mail-pull` or `gmail-read` moves it to `quiet` or `pending`,
    as after any lapse (spec §4.6 step 3), and the canary takes it from there.
- **A read that fails throws.** The run counts it `failed` and the lease runs out (`:369-373`): nothing is
  charged and no attempt is spent. An unreachable entitlement is no licence to call a metered model, as
  `model.ts:51-52` says of the caps.
- **The sweep is unchanged.** A held item whose object's lifecycle has run is expired by
  `sweep.ts:63-74`, which deletes its seen row and counts it `expired` (D8).
- Quinn ruled Q2 (a) on 2026-10-20: hold, never expire early. The expire-at-once branch is not built.

**Tests, written first** (names as the suite writes them; fakes only):
1. `an_unentitled_accounts_item_makes_no_model_call_and_no_charge`. In `worker_test.ts`: the `FakeDb`
   sees no `charge_allowance`, `charge_call`, `enforce_budget` or `record_tokens`, and the `SpyModel` sees
   no call.
2. `an_unentitled_accounts_item_is_held_an_hour_unread_with_its_attempts_unchanged`: S3 gets are 0, and
   the fake's `wait` records `(id, now + 1h, attempts as it arrived)`. The row still exists.
3. `a_run_marks_each_held_mailbox_once_from_its_oldest_held_arrival`: three held items over two mailboxes
   of one account give exactly two `markLapsed` calls, each with that mailbox's smallest `arrivedAt`.
3b. `a_failed_mark_leaves_the_holds_and_counts_failed`.
3c. `a_run_that_fails_after_a_hold_still_marks`. Round 1 holds an item; round 2's `due` throws, so
    `handle` answers 500. The fake still records the `markLapsed` call for the held item's mailbox. It is
    red on a `run` with no `finally`.
4. `a_renewed_account_is_processed_at_the_next_run_after_the_hold`: the same item, now entitled, runs the
   model path once, and its attempts move as today.
5. `a_held_item_is_expired_by_the_sweep_and_loses_its_seen_row` (the existing sweep fake).
6. `entitlement_is_read_once_per_account_per_run`: two items of one account give one read.
7. `an_entitlement_read_failure_charges_nothing_spends_no_attempt_and_leaves_the_item`.
8. `an_unentitled_live_unaligned_item_is_held_not_counted`: the gate comes first.
9. `index_wires_entitled_through_the_shared_isActive`: a source-text pin of `index.ts`, as L7's T2b test 6
   pins `gmail-read/index.ts`.

**Done:** 1-9 (with 3b and 3c) red, then green; the earlier `mail-process` tests unchanged and green; §3's gates
green.

## T4. (c) Rotate and D22 (cloud-engineer)

**Files:**
- `mail/handler.ts`, `mail/store.ts`, `mail/handler_test.ts`, `mail/store_test.ts`;
- `mail-inbound/store.ts`, `mail-inbound/store_test.ts`;
- `gmail-read/handler.ts`, `gmail-read/index.ts`, `gmail-read/handler_test.ts`.

All are under `cloud/supabase/functions/`. Re-read the L7 lines of the `gmail-read` files on the rebased
tree first (§5).

**Behaviour.**
- **Rotate** (`mail/handler.ts:363-380`): `RotateNext` gains `rotatedFrom`, the row's status as `find`
  read it.
  - `store.rotate` writes `rotated_from` in the same guarded PATCH (`mail/store.ts:171-183`, still
    filtered on `token_hmac`).
  - Gmail's status still becomes `pending`; other providers' status is still left alone.
  - The answer is unchanged, and so are `GET /addresses` and the export (`account/mail.ts:105-124`):
    neither carries the grace fields today.
- **The grace ending by canary** (`mail-inbound/store.ts:204-209`): `endRotateGrace` also sets
  `rotated_from` to null, in the same PATCH. This is for tidiness, not correctness: T2 has no grace
  check (review r1, M7), and D22 ignores `rotated_from` once `retired_until` is null or past.
- **The deploy order is benign either way.** If `mail` is deployed before `mail-inbound`, the only
  effect is that a grace ended by canary in that window leaves a stale `rotated_from`, which D22 ignores.
  T9 still deploys `mail-inbound` first, as the tidier order.
- **D22** (`gmail-read/handler.ts:198-215`):
  - `Forwarding.mailboxes` rows gain `retiredUntil` and `rotatedFrom`, read by `index.ts:199-207`'s
    `select` (`mailbox_hint,status,retired_until,rotated_from`).
  - `forwardingCounts(gmail, mailboxes, nowMs)` counts a row whose hint matches when its status is on
    `FORWARDING_COUNTS`, **or** when its status is `pending`, its `retiredUntil` is after `nowMs`, and its
    `rotatedFrom` is on `FORWARDING_COUNTS`.
  - Nothing else changes: case-folding, the whole-address match, the non-quiet answer and
    `via_forward` stay as they are.
  - The handler passes its own clock.

**Tests, written first:**
1. `gmail-read`: `a_rotated_active_gmail_mailbox_keeps_the_oauth_read_off_inside_its_grace`. The answer
   is exactly D22's (`read: 0`, `quiet: false`, `via_forward: true`, plus the keys L7 added, as the
   existing forwarding test asserts them), and there is no token exchange and no Gmail call.
2. `gmail-read`: `a_rotated_gmail_mailbox_whose_grace_ran_out_unconfirmed_is_read_by_oauth`.
3. `gmail-read`: `a_gmail_mailbox_rotated_from_pending_blocked_off_or_lapsed_is_read_by_oauth`.
4. `forwardingCounts` unit cases: `confirmed`, `active` and `quiet` count as before; `pending` without a
   grace does not; `notjane@` does not match `jane@`.
5. `mail` handler: `rotating_a_gmail_mailbox_still_reads_pending_for_the_devices_confirm_wait`, with the
   PATCH's `rotated_from` equal to the prior status.
6. `mail` store (injected `rest.fetch`): `rotate_writes_rotated_from_in_the_same_guarded_patch`. One
   PATCH, filtered on the account, the id and `token_hmac`.
7. `mail-inbound` store: `ending_the_grace_by_canary_clears_rotated_from_with_the_retired_pair`.

**Done:** 1-7 red, then green; D22's existing tests unchanged and green; §3's gates green.

## T5. (d) One count per receipt (cloud-engineer)

**Files:**
- `mail-inbound/handler.ts`, `mail-inbound/store.ts`, `mail-inbound/handler_test.ts`,
  `mail-inbound/store_test.ts`;
- `account/mail_test.ts` (one name added to its list of secret columns, `:167`).

**Behaviour.**
- `InboundStore.batchReceived(batchId, addressId, n, receipt)` answers whether it counted. `receipt` is
  the lower-case hex SHA-256 of the notification's object key (`n.key`), computed once per delivery in
  `handle` and passed down.
- `store.batchReceived` calls `count_batch_receipt` and nothing else. The read-then-write loop
  (`store.ts:214-229`) and its `contended` error go.
- **A false answer is not an error.** It means the batch is gone (pruned or disconnected, as today's
  early return) or this receipt was already counted. Both log one fixed phrase.
- The count still includes duplicates of messages already seen (`handler.ts:435-436`): what changes is
  that one receipt counts once. Hand-off, dedup and the 200/500 rules are unchanged.
- The receipt hash is never logged and never answered. The object key never reaches a request body.

**Tests, written first.** The handler fake models the RPC: it keeps a set of receipts per batch.
**The fake S3 is faithful:** a successful delete of `n.key` removes the object, so a later get answers 404.
Each test says, in a comment, why the object is still there when the redelivery reads it. **Test 1's red
run is recorded** with the count today's code gives (twice the originals): it must fail on today's code
(review r1, I2).
1. `an_sns_redelivery_with_the_object_still_present_counts_received_once`.
   - The first delivery's final `s3.delete(n.key)` fails, so it answers 500 and the object stays.
   - The redelivery reads the same object and answers 200.
   - `received` equals the originals once, and each original is queued once.
   - A redelivery after a full success (object gone, 404) is not this test: today's code already counts
     nothing twice there (`mail-inbound/handler.ts:479, 509-511`; see §9).
2. `two_concurrent_deliveries_of_one_send_count_once`: `Promise.all` over two `handle` calls, both
   started before either deletes the object. Red on today's code too.
3. `a_later_recipients_failure_after_the_count_then_the_retry_counts_once`.
   - One notification names two of the account's mailboxes. The first mailbox's batch is counted, then
     the second's `arrive` throws, giving a 500 with the object left.
   - The retry gives 200, and the first batch is counted once.
4. `two_sends_of_one_batch_each_count`: different object keys.
5. Store (injected `rest.fetch`): `batch_received_is_one_rpc_carrying_a_64_hex_receipt_and_never_the_object_key`.
6. `account/mail_test.ts`: the secret-column list names `counted_receipts`, so no export list can carry
   it.

**Done:** 1-6 red, then green (test 1's red count recorded); the earlier intake tests green; §3's gates
green.

## T6. Out of credit: no attempt spent, and the breaker (cloud-engineer; only once L7 is on `main`)

**Files:** `mail-process/model.ts` and `mail-process/worker_test.ts`.

**Preconditions.**
- `ProviderOutOfCredit` is exported from `_shared/judge_anthropic.ts` on `main`, which means L7's PR has
  merged. Rebase first. If L7 has not merged, see C3.
- `mail_claim`'s latest pin on `main` still says `provider = 'openrouter'` (today:
  `20261002000200_mail.sql:343-353`; later files move only `max_tokens`). Re-check with the suite's pin
  parser (`currentPin`, `migrations_test.ts`). If a later re-pin moved it to `anthropic`, stop and report:
  `AnthropicModel` never throws `ProviderOutOfCredit`, so T6 would change nothing (review r1, M6).

**Behaviour** (`model.ts:153-277`):
- **The breaker comes first.** `modelPath` keeps `creditOutUntil` (ms) in its closure; one wired instance
  lives across warm invocations, so the state is time-bounded and never a flag. Before the attempt is
  written (`:181-186`): while `now < creditOutUntil`, the item is held to `creditOutUntil` with
  `attempts` unchanged (`store.wait(id, until, item.attempts)`). There is no charge, no call and no
  judgment row, and the log is one fixed phrase.
- **On a `ProviderOutOfCredit` from `model.complete`:**
  - the judgment row is written as today (`outcome: "low confidence"`, `cause: "model failed"`; the
    `Cause` set is not widened);
  - the charge is given back as today (`:234-237`);
  - the item's wait is rewritten to `now + OUT_OF_CREDIT_HOLD_MS` (exported, one hour) with the
    attempts it **arrived with**;
  - `creditOutUntil` is set to that time;
  - the item is never expired for this reason.
- **Every other failure is unchanged:** a plain `model failed` still spends its attempt, waits for the
  next UTC day, and expires on the third. The sweep still bounds every hold.

**Tests, written first:**
1. `an_out_of_credit_answer_spends_no_attempt_refunds_and_waits_an_hour`.
2. `after_an_out_of_credit_answer_the_next_items_make_no_charge_and_no_call_for_an_hour`: three items,
   one model call, one refund, two holds.
3. `after_the_hour_the_next_item_calls_again`.
4. `an_item_out_of_credit_three_times_is_never_expired_by_attempts`.
5. `a_plain_model_failure_still_spends_its_attempt_and_waits_for_the_next_utc_day`: today's path,
   pinned.

**Done:** 1-5 red, then green; the earlier `worker_test.ts` tests green; §3's gates green.

## T7. Docs (docs-keeper)

**Files:** `docs/specs/2026-09-30-email-forwarding-design.md` (dated amendment markers only),
`HANDOFF.md`, and `cloud/supabase/README.md` only if it describes `mail-process`'s attempts or Rotate.
Grep first, and edit nothing when the grep finds nothing.

- [ ] The five spec markers in §4's table "Spec amendments the rulings imply", each
      `*(amended 2026-10-20, Quinn: …)*`, carrying that table's text and nothing more:
  - Q1: §4.7, a new first sentence;
  - Q2 and Q3: §4.6 step 3;
  - Q4: D22;
  - Q5: §4.7 step 5;
  - (d): §4.6 step 5's batch-send bullet, recorded rather than amended.

  Each marker lands with, or after, the commit whose code it describes. Q5's marker goes with T6's PR
  if C3 split it off.
- [ ] `HANDOFF.md`:
  - a lane row for this lane;
  - the migration's stamp, added to the merge-stamp order of §2 and of L9's row (prod re-runs steps
    (1)-(2) for it);
  - the functions L9 must redeploy: `mail-inbound`, `gmail-read`, `mail-process`, `mail`;
  - T6's owed row, if C3 applied.

**Done:** every marker matches §4's table and a ledger row of Quinn's 2026-10-20 ruling; no other spec
text changes.

## T8. Whole-branch review (reviewer)

- [ ] `reviewer` (Opus high) reviews the branch diff against this plan and the spec, and files the report
      at `docs/reports/2026-10-2x-email-cloud-follow-ups-review.md`. It checks in particular:
  - no path in `mail-process` reaches a charge or `model.complete` for an unentitled account;
  - the migration's raw text has no `models`, and no applied file changed (the SHA pins);
  - `rotated_from` is cleared wherever the retired pair is;
  - no receipt hash, object key or status is logged;
  - T6's breaker cannot hold an item past the sweep or spend an attempt.
- [ ] Findings are fixed by the task's agent, then re-reviewed (a delta check) until PASS.

**Done:** PASS on file.

## T9. Gate, re-check, staging, PR, merge (main session)

- [ ] Rebase onto `main`. Recount T2's shared pins if `main` moved (M4).
- [ ] Run M5's re-check of the stamp against M3's three sets. Apply C1: record any lane ahead as slipped
      before the push, in the session ledger and this lane's ledger.
- [ ] Final gate from the root: §3's three Deno lines. Confirm `git diff --stat origin/main -- engine app`
      is empty (no Rust change; CI still runs the workspace).
- [ ] **Staging, in this order** (the hook asks for each command; record each in the ledger):
  1. `supabase db push` of the one migration;
  2. deploy `mail-inbound` (the tidier order, not a required one: T4 says why);
  3. deploy `gmail-read`;
  4. deploy `mail-process`;
  5. deploy `mail`.
  **Prod is L9's, on Quinn's word.**
- [ ] Smoke on staging (controller only, read-mostly):
  - `supabase migration list` shows the stamp;
  - one cron-shaped `mail-process` call answers `ok` with a `lapsed` count;
  - one rotate of a scratch Gmail row is checked only if a scratch profile already exists for it. If one
    is built for this, it is removed after (memory `live-proof-profile-cleanup`).
- [ ] Push and open the PR. CI must pass in full. The eval gate's `touched` step must read `run=no`;
      `yes` is a stop and a question for Quinn.
- [ ] Merge on green CI and T8 PASS, with a merge commit. Never merge over a red check without Quinn's
      word. Ledger rows; remove the worktree.

## 7. Fidelity ledger

| Source (the ask) | Requirement | Verified at | Task | Proof |
|---|---|---|---|---|
| (b) | Verify that `mail-process` spends model calls for an unentitled account | `mail-process/handler.ts:256-308`, `model.ts:186-229`; intake's gate at `mail-inbound/handler.ts:463-477` | §1.1 | confirmed |
| (b) | Decide where the gate goes | | Q1, §2; T3 | T3 tests 1, 2, 8 |
| (b) | Decide what happens to queued mail of an account that lapses | `sweep.ts:45-74`; spec §4.6 step 3, D8 | Q2, Q3, §2; T2 item 4, T3 | T3 tests 2-5; T2 `mail_mark_lapsed_from_…` |
| (c) | Verify that Rotate sets `pending`, which turns the Gmail read back on | `mail/handler.ts:372`; `gmail-read/handler.ts:203, 210-215` | §1.2 | confirmed |
| (c) | Fix it without breaking the device's confirm wait | `app/src/mail/runner.rs:796-818` | Q4; T2 item 1, T4 | T4 tests 1-7 |
| (d) | Verify that an SNS redelivery double-counts `received` | `mail-inbound/handler.ts:456, 482, 506-512`; `store.ts:214-229` | §1.3 | confirmed (two paths) |
| (d) | Count once | | T2 items 2-3, T5 | T5 tests 1-5; T2 `count_batch_receipt_…` |
| Pin | A SHA-256 pin for `20261020000100_mail_max_tokens_repin.sql`, in the style of `migrations_test.ts` | `migrations/migrations_test.ts:1003-1029` | T1 | the pin, with the hex matching `git cat-file` |
| L7 T2a | Fix the missing breaker for an out-of-credit provider, or name it out of scope | `model.ts:125, 137-150, 186, 230-239`; L7's `ProviderOutOfCredit` | §1.5, Q5, C3; T6 | T6 tests 1-5 |
| Out of scope | Name the five engine-side minors in a follow-ups section only | `approvals.rs:1801-1806`, `cli.rs:867-868` (read: these lines match) | §9 | listed |
| Constraint | `cloud-engineer` executes; tests first, in Deno, with fakes; no network | | §3; every task | red and green ledger rows |
| Constraint | A migration takes the next free stamp after `20261020000300` under M2, and applied files are never edited | L3 ledger T0, L8 ledger | §3, T0, T2, T9 | T0's ledger row; the SHA pins |
| Constraint | Prod deploys are L9's (Quinn's) | | §3, T9 | — |
| Constraint | Open questions for Quinn, each with a recommendation; money ones are Quinn's | | §4 | — |
| Constraint | Branch `email-cloud-follow-ups`, cut by the controller | | T0 | — |
| Constraint | Gates: `deno check`, lint, CI's full `deno test` line | `ci.yml:150, 153, 167` | §3 | ledger counts |
| **Plan's own additions** | The eval-gate word rule for the migration (`ci.yml:242-248`) | | §3, T2 test, T9 | `run=no` |
| | Deploy order: migration first, then `mail-inbound` before `mail` (tidier, not required: r1 M7 dropped the grace check) | | T4, T9 | ledger |
| Review r1 | I1, I2 and M1-M8 closed in place | `docs/reports/2026-10-20-email-cloud-follow-ups-plan-review-r1.md` | §10 | §10's table |
| | Second pin, for `20261019000100` (C2, ruled yes) | | T1 | the pin |

## 8. Checkpoints (Quinn and review)

1. **Before T0:** a `reviewer` plan-mode review of this document, filed at
   `docs/reports/2026-10-20-email-cloud-follow-ups-plan-review-r1.md` (FAIL, closed in §10), and then a
   delta re-review (r2, PASS; minors closed in §10). Quinn ruled Q1-Q5 on 2026-10-20; the controller's
   C1-C3 are ruled (§4). Done.
2. **After T2:** the controller reads the migration against §2 before any function task starts. This is
   the last point where the SQL's shape can change without M5's rename.
3. **After T5** (or T6): T8's review. Quinn is told the staging results before L9 takes prod.

## 9. Out of scope, and follow-ups this lane hands on

**Engine-side minors, named only (not this lane's):**
- A refusal from the shared candidate list repeats once per group.
- The 200-candidate cut can turn a change into a new item.
- A note whose status is `done` ranks as active.
- Attachment answers are short by one.
- The `commitment-ask` pending undercount (`engine/src/approvals.rs:1801-1806`, `engine/src/cli.rs:867-868`).

**Found while verifying, and left for a later lane:**
- **Intake's other per-delivery counters repeat on a redelivery too.** These are `unbound`, the
  since-live `unbound_own` and `unbound_foreign`, and `unauthenticated` (`mail-inbound/handler.ts:361-376`).
  `UNBOUND_STREAK` is 2 (`mail/handler.ts:47`), so one foreign unbound arrival plus a redelivery reads as
  `leaked` and suggests Rotate. The fix is the same per-receipt guard, on a new counting function (the
  mail migration is applied, so `count_mail_arrival` stays). It is display-only, with no money and no
  mail lost.
- **A redelivery after a full success finds the inbound object gone.** The `s3.get` gives a 404, the
  handler answers 500 (`mail-inbound/handler.ts:479, 509-511`), and SNS retries until its policy gives
  up. Answering 200 for a missing inbound object would end the noise; the mail is unaffected.
- **Rotate reads the status before its guarded write.** `rotated_from` is the status `find` read
  (`mail/handler.ts:364`). If a canary moves the row in the milliseconds between, the earlier status is
  recorded. This residual is accepted.
- **A held backfill or recover item is marked from the batch send's arrival, not the original's date.**
  If such an item expires during a lapse, it comes back only when its date falls inside the next
  reconcile window, as D8's expiry already behaves.
- **A redelivery after the batch has closed is counted `unbound`** (review r1, M8). The path:
  1. a batch send is counted, and the delivery's final delete fails (500);
  2. before SNS's retry, `mail-pull` closes the batch because `received` reached `sent`;
  3. the redelivery's batch nonce now binds to no open batch (`mail-inbound/store.ts:120`,
     `closed_at=is.null`);
  4. so it is counted `unbound` (`mail-inbound/handler.ts:361-366`).

  It feeds the since-live count that `UNBOUND_STREAK` reads. T5 makes `received` right but does not
  change this. The same later per-receipt guard on the counters (first bullet) covers it. No mail is
  lost: every original was handed off by the first delivery.
- **Mid-run entitlement changes** (T3's two accepted residuals, Q1 and Q3). A lapse in the middle of a run
  is charged for at most that run (up to 50 s). A renewal in the middle of a run can be marked lapsed from
  the run's stale answer, and then comes back `quiet` or `pending` at the next pull.
- **A held item the sweep expires before any run holds it is never marked** (review r2, minor 1; Q3).
  The case:
  - an item capped on day 0 and day 1 is due at day 2 00:00, which is also the sweep's bound (§1.5);
  - the account lapsed in the hour before;
  - the sweep wins the race (or, before the `finally`, a run held the item and failed).

  Then no mark is written, and a renewal after the three-day reconcile window cannot reach the item.
  The `finally` closes the failed-run half (T3 test 3c). The race half remains: a later lane could have
  the sweep check the account's entitlement and mark before it expires.
- **A rotated Gmail can keep its OAuth read off past the grace** (review r2, minor 2). The path exists
  before this lane; Q3's mark makes it reachable with no arrival during the lapse. In order:
  1. Rotate from `active` leaves `last_canary_ok_at` set (`mail/store.ts:171-183`).
  2. A lapse mark turns the row `lapsed`.
  3. On renewal, the unlapse writes `quiet` (`_shared/mail_unlapse.ts:19`), and `quiet` is on D22's
     allow-list.
  4. Intake's `confirmed` is guarded on `status=eq.pending` (`mail-inbound/store.ts:211`), so the new
     address's confirmation never lands.
  5. Once `retired_until` passes, the old forward's mail is deleted as unbound, while the OAuth read
     stays off.

  For a later lane: the unlapse writes `pending` while a Rotate grace is open (or the confirmation also
  accepts `quiet` inside a grace). Not verified: whether `mail-canary` later moves such a row to
  `blocked`.
- `deno fmt --check` flags three files (L7 ledger, T2a row). CI does not run `fmt`.

## 10. Review r1 (2026-10-20): FAIL, 0 critical, 2 important, 8 minor, all closed in place

Report: `docs/reports/2026-10-20-email-cloud-follow-ups-plan-review-r1.md`.

| # | Finding | Applied where |
|---|---|---|
| I1 | The cited pattern is `security invoker` with `public, extensions`, not definer with `pg_temp` | T2's function pattern; test 5 asserts invoker, `public, extensions`, revoke and grant, and no `security definer` |
| I2 | T5 test 1 could pass on today's code (object gone after a full success, so 404 and 500) | T5: a faithful fake S3; test 1 is a redelivery with the object still present; red count recorded; test 3 is now a later recipient's failure |
| M1 | Run-cached entitlement residuals; a mark per held item per hour | T3: one mark per held mailbox per run, from the smallest `arrivedAt`, at the run's end; both residuals named in T3, Q1, Q3 and §9 |
| M2 | `hold` repeats `store.wait` | T3 uses `store.wait(id, until, item.attempts)`; `hold` dropped |
| M3 | CI greps case-insensitively | §3 and T2 test 6: `/\bmodels\b/i` over the raw text |
| M4 | The canary residual was wrong (`mail-canary/handler.ts:16, 207-211` checks entitlement first) | dropped from §9 |
| M5 | §1.5 overstated the sweep's lead | §1.5: attempt 3 and the sweep come due at the same instant, a race |
| M6 | Which route `mail_claim` uses | §1.5 and T6's preconditions: OpenRouter (`20261002000200_mail.sql:343-353`; later files move only `max_tokens`), so T6 applies; it stops if a re-pin moves the kind to Anthropic |
| M7 | The grace check makes deploy order load-bearing | **Dropped the check** (T2 item 1, T4): D22's `retiredUntil > now` already ignores a stale value. The check would turn an out-of-order deploy into a 500 on a canary, and buys nothing. The deploy order is now only the tidier one (T9) |
| M8 | A redelivery after the batch closes is counted `unbound` | §9, new bullet |

The controller's rulings C1-C3 (2026-10-20) are recorded in §4, including the condition on L8's stamp.

## 11. Review r2 (2026-10-20): PASS, 0 critical, 0 important, 2 minor, both applied; Quinn's rulings

Report: `docs/reports/2026-10-20-email-cloud-follow-ups-plan-review-r2.md`.

| # | Finding | Applied where |
|---|---|---|
| r2-1 | A missed end-of-run mark can lose mail when that run was the item's last hold before the sweep | T3: the marks run in a `finally` around the loop; test 3c `a_run_that_fails_after_a_hold_still_marks`; the sweep-before-first-hold residual named in Q3 and §9 |
| r2-2 | A pre-existing path keeps a rotated Gmail's OAuth read off past the grace | §9, a bullet for a later lane |

**Quinn's rulings of 2026-10-20** (recorded in §4 and the header): Q1 both gates; Q2 (a) hold until the
object's lifecycle ends; Q3 yes; Q4 yes; Q5 yes, with one hour. Every one is the recommended option. The
spec amendments they imply are in §4's table, and T7 writes them. Status: FINAL.
