# First-day event allowance: design (signed by Quinn, 2026-10-19)

**Status:** Signed by Quinn, 2026-10-19 (§11 as ruled; earlier: DRAFT for Quinn, no code until §11 was ruled).
**Date:** 2026-10-06. **Asked by:** Quinn's ruling of 2026-10-06, after the T18c live proof.
**Governs:** how a new student's first event backlog is paid for in the judgment service.
**Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (judgment is a cloud service with rate
guards); `VISION.md` (:155-159 "Minimize the AI surface", :192 success criterion 5, "a new student
reaches 'it already knows what's next' in their first session"); the events spec
(`docs/specs/2026-09-29-events-design.md`); the email spec's backfill allowance (D23 as amended by
PQ6 (a), `docs/specs/2026-09-30-email-forwarding-design.md`), which this copies in shape.
**Code read:** the worktree `.claude/worktrees/email-forwarding` (it carries the mail allowance and
main's F1 refund). Every `file:line` below is from that tree.
**Contract list:** nothing here touches it. `judge_caps.ts`, `judge_pipeline.ts`, the migrations
and `engine/src/events.rs` are all off the list; no fixture or frozen reference moves (§8, Q8).

Quinn's ruling (2026-10-06): a new student's first-day event backlog gets a **one-time first-day
judgment allowance**, like the email backfill allowance, instead of a judging horizon or a cap raise.

## 1. The evidence, read carefully

T18c (staging, a new profile at a real university, the older proof account) ran its first slot.
`judge` logged `events: 80 judged, 862 left for the next slot` and hit the daily event cap.

What produced that line:

- `DAILY_CAP = { task: 60, event: 80, email: 120 }` (`cloud/supabase/functions/_shared/judge_caps.ts:45`)
  is passed as `p_cap` to `charge_call` (`judge_caps.ts:98`), which the pipeline calls before every
  model call, after the monthly ceiling (`_shared/judge_pipeline.ts:185-208`).
- `charge_call` doubles the cap for an account's first two UTC judging days
  (`migrations/20260923000100_first_day_cap.sql:32-33`), where the first day is the account's earliest
  `usage_daily` row of **any** kind (`:24`). The proof account had judged before, so no doubling.
- The engine draws its events from `prefilter_events`'s survivors (`engine/src/events.rs:453-454`;
  the rules at `engine/src/eventfilter.rs:81-105`: the 90-day `roster_window_days`, standing exhibits,
  `never` interests, staff-only audiences), less what the ledger already holds and what a settled
  series answers (`events.rs:462-469`), soonest first (`engine/src/eventfeed.rs:133-134`, and `:846`
  for the merged list). It takes at most `judge_per_run_cap` = 150 a run (`events.rs:26`, `:470-471`)
  inside the slot's shared 15-minute budget (`events.rs:478`).
- A `capped` reply sets the cloud model's fatal flag, so every later item in the run answers
  `ModelError::Capped` without a request (`engine/src/cloudmodel.rs:425-427`, `:442-443`, `:541`).
- An unjudged event never shows: `relevant_events` keeps only a ledger verdict of `obligation` or
  `opportunity` (`engine/src/eventroster.rs:38`, `:59-79`).

Three things the line hides, which shape this design:

1. **The real backlog was about 1,012, not 942.** The batch was 150; after the 80th call every item
   returned `Capped` and was *processed*, so `judge_roster` printed 70 `not judged` lines and its
   `left` (`events.rs:528`) counted only the 862 the run cap excluded. One university's 90-day,
   pre-filtered backlog is therefore about 1,000 events, about 11 a day.
2. **A genuinely new account would not have stopped at 80.** Its doubled cap is 160, so its first
   slot would have judged the full 150. The proof stopped at 80 because the doubling keys on the
   account's first judging day, and the proof account is old. A new profile (or a second vault) on
   an existing account gets no first-day headroom at all today.
3. **The steady state is not the problem.** Once the backlog is gone, about 11 new events a day enter
   the far edge of the 90-day window, far inside 80. Soonest-first also means the judged frontier
   moves forward faster than the calendar does. The allowance buys a *complete* 90-day roster in days
   rather than about two weeks, and covers the account that the doubling misses.

## 2. What the allowance is

One allowance per account, for event judgments, used once.

- **Size:** 2,000 event judgments (Q1). About twice the observed backlog; a larger school's feed is
  still covered, and past it the daily cap continues soonest-first.
- **Opens:** at the account's first event judgment the daily cap refuses (Q2). For a new student that
  is the first or second slot of day one. An alternative opens it at the account's first events
  judge run whether or not it is needed; both are one-time, and Q2 sets out the difference.
- **Lasts:** 14 days from opening, as the mail allowance does (Q3). Unspent judgments lapse.
- **Never renews** (Quinn: one-time). The mail allowance renews every 120 days
  (`20261002000200_mail.sql:410-417`); this one does not. A new term needs nothing, because the
  roster window moves a day at a time and a day's new events fit inside the cap (§1 point 3).
- **Events only** (Q4). Tasks and email keep the first-two-days doubling and, for mail backfill, D23.
- **Keyed on the account id**, never on a profile, vault, device or mailbox, so a second vault, a
  reinstall or a changed school does not reopen it (Q10). Nothing names a person or a school.

State: one new table, `judge_allowances`, one row per `(account_id, kind)`, with `kind` checked to
`'event'` for now: `opened_at`, `until`, `left` (non-negative), `refunds` (non-negative). RLS on, no
grants to `anon` or `authenticated`, `on delete cascade` from `accounts` so the account purge
removes it. The row is created by the first charge and **never deleted while the account lives**,
which is what makes the allowance one-time: the opening test is "no row", not "row expired".

## 3. How `judge` spends it

**The server decides; the engine is unchanged in what it asks.** The device cannot be trusted to say
"this is my backlog", and it does not need to: the allowance is spent only on what the day's cap
refuses.

In `judge_pipeline.ts`, the order for one `/judge-event` request becomes:

1. `withinBudget` (unchanged; an account past the monthly ceiling never touches or opens the
   allowance).
2. `charge_call` at `DAILY_CAP.event` (unchanged; doubled on the first two judging days as today).
3. **New, kind `event` only:** when step 2 refused, `charge_judge_allowance(account, 'event')`. One
   statement: insert the row (opening it, `left = size - 1`) or, on conflict, decrement `left` only
   while `until > now()` and `left > 0`; answer whether it charged. Concurrent requests serialise on
   the row, as `charge_allowance` does (`20261002000200_mail.sql:390-424`).
4. Both refused: `capped`, exactly as today. The pipeline remembers which of steps 2 and 3 charged.

The reverse order (allowance first, as mail backfill does) is rejected: an event request carries no
"backlog" marker, so allowance-first would spend the allowance on the steady state of every account
in its first 14 days. Overflow-only spends it on the backlog and on nothing else.

**Soonest first, bounded per run.** Nothing in the engine changes how it orders or bounds the pass:
soonest-first from the feeds, at most `judge_per_run_cap` (150) a run, inside the slot's shared
15-minute budget. So a slot is no longer than today's worst case; the allowance changes only whether
the cap stops the run partway. With the console's default two slots a day (`12:00` and `18:00`,
`app/static/console.js:2849`) plus the first slot, about 1,000 events drain in three to four days.
The first slot's 150 cover the soonest ~13 days at the T18c school, about `propose_horizon_days`
(14, `events.rs:27`), so the cards and the five *Coming up* events are complete after slot one.

Raising the run cap while the allowance is open is rejected (Q7): the calls are sequential, and a
1,000-call slot would run well past the 15-minute budget and `scheduler::CHILD_TIMEOUT`.

## 4. The first-two-days doubling

Kept, unchanged, for every kind (Q5). It still gives tasks (120) and email (240) their day-one
headroom, and `charge_call` and `refund_call` keep their SQL. For events the two stack: a new
account's first two days get 160 a day from the cap, then the allowance. The allowance is what
covers the account the doubling misses (§1 point 2).

Replacing the doubling for events only would mean a `p_kind = 'event'` branch in both `charge_call`
and `refund_call` for no gain a student could see; dropping it everywhere is outside the ruling.

## 5. Refunds

F1's rule carries over: a call the model never answered (`model failed`) is given back to whatever
charged it; a refusal, a truncation or a failed validation reached the model and stays charged.

- Charged by the cap: `refund_call`, as today (`judge_pipeline.ts:243`).
- Charged by the allowance: **new** `refund_judge_allowance(account, 'event')`, one statement: add one
  to `left` and one to `refunds` only while `until > now()`, `left < size` and `refunds < size`. It
  never opens, extends or resizes the allowance.

Why the lifetime `refunds` bound: each `/judge-event` request is its own invocation, so F1's breaker
(`BREAKER_FAILURES`, `judge_caps.ts:85`) never trips across a run's 150 requests. Without a bound, a
device on a dead provider could charge, fail and refund the allowance indefinitely. With it, the
allowance reaches the model at most `2 x size` times in its life, and a long outage costs the
student nothing until the bound is spent.

A charge that `charge_call` refused stays counted in that day's `usage_daily.calls` (`charge_call`
increments before it compares, `20260923000100_first_day_cap.sql:26-30`). That is today's behaviour
and is kept: `calls` then shows the day's real request volume, and `monthly_spend` prices tokens,
not calls, so nothing is billed twice.

## 6. Cost, the ceiling and abuse bounds

**Per call.** Event judgments are pinned to Granite 4.2 8B at $0.10 in / $0.15 out per MTok
(`migrations/20260916000100_provider_swap.sql:36-37`). At ~980 tokens in and ~80 out
(`judge_caps.ts:20-23`) one call is about $0.000098 + $0.000012 = **$0.00011**.

| Case | Calls | Cost, once |
|---|---|---|
| The T18c school's backlog | ~1,000 | ~$0.11 |
| The whole allowance (Q1) | 2,000 | ~$0.22 |
| The whole allowance and every refund bound spent on a dead provider | 4,000 sent | ~$0.22 (failed calls record no tokens) |

**Against the $0.25 per student per month rule** (the model-choice rule at the typical persona,
`docs/notes/2026-09-16-inference-provider-and-model-scoping.md:22`): the allowance is one-time, so
over a nine-month year it is about $0.01 to $0.025 a month. In month one alone the worst case is close
to a whole typical month's target. That is the price of a complete first week, and it is paid once.

**Against the $2 ceiling** (`MONTHLY_CEILING_USD`, `judge_caps.ts:76`): a fully capped account costs
about $1.3 a month (`judge_caps.ts:32-40`), so with the whole allowance it is about $1.52, still under
$2. **The ceiling is not raised while the allowance is open** (Q6), unlike the mail allowance's $1
(`20261002000200_mail.sql:458-487`). The ceiling stays the runaway guard and is checked first, so an
account already over it never opens the allowance. The mail allowance's raise stays keyed to
`mail_accounts` and does not change.

**Abuse bounds.**
- One per account id, ever: the row is never deleted while the account lives.
- Opening needs a request that passes `requireActiveEntitlement` (`judge-event/index.ts:10`;
  `active` or `trialing`, `_shared/entitlement.ts:22`). A new account needs a new trial sign-up, and
  farming trials buys at most $0.22 each, less than the mail allowance's $1.
- Request volume: `2 x size` model-reaching requests for the allowance's life, on top of the daily
  cap's existing bound; refused requests are still counted in `usage_daily.calls`.
- Dollars: the unchanged $2 ceiling, checked before every charge.

## 7. The entitlement gate

Nothing new. `judge` does not run past the 72-hour grace (`engine/src/entitle.rs`; the refusal is a
named line at exit 0), and `/judge-event` answers 402 without an active or trialing subscription
before the pipeline runs. A lapsed account therefore cannot open or spend the allowance, and the
14 days keep running while it is lapsed. That is deliberate: the allowance is for the first days,
not a balance to come back to.

## 8. What the student sees while it drains

**Recommendation: nothing new** (Q8).

- After the first slot, the soonest ~150 events are judged: the five *Coming up* events
  (`docs/surface/anatomy.md:374-378`) and the event cards inside the 14-day propose horizon are
  complete. Later slots fill the far weeks of the 90-day roster, which the student does not see until
  those days come near.
- Unjudged events stay invisible (`eventroster.rs:59-79`), as today. Nothing is shown as known that
  is not.
- The run log names the backlog on each slot: `events: N judged, M left for the next slot`.

The alternative is a line under *Coming up*, "M campus events not read yet; more after the next
run", from a count `rank` can compute deterministically (the same pending set `judge_roster` uses).
It needs a new read-model field, so the three `surface-today-*.json` references regenerate under the
console spec §4.6 rule. It is cheap but adds a surface for a state that lasts three to four days and
that the near-term view never shows; deferring it is the recommendation.

**One honesty fix is recommended either way** (Q9). Today a `capped` reply leaves every later item in
the run's batch printing `events <uid>: not judged (the judgment service: the daily judgment cap)`
(70 lines at T18c), and `left` undercounts by those items (`events.rs:498-503`, `:528`). After the
fix, the first `ModelError::Capped` ends the loop, one summary line names the cap, and `left` counts
every pending item not judged: `events: 80 judged, 932 left for the next slot (the daily judgment
cap)`. With the allowance, a new student should rarely see it; it still matters once the allowance
is spent or lapsed.

## 9. Tests (TDD: each written failing first)

Cloud, SQL (`migrations/migrations_test.ts`, against the local database):

1. **Opens on first overflow.** No row; `charge_judge_allowance` creates it with `left = 1,999`,
   `until = opened_at + 14 days`, and answers true.
2. **One-time.** A row whose `until` has passed, or whose `left` is 0, answers false and is not
   reopened, including when `opened_at` is more than 120 days old (the contrast with mail's
   `charge_allowance`).
3. **Never below zero under concurrency.** Two concurrent charges against `left = 1` charge exactly
   once; two concurrent first charges open exactly one row.
4. **Kind is checked.** A `kind` other than `'event'` is refused by the table's check.
5. **Refund bounds.** `refund_judge_allowance` gives back only while open, never past `size` in
   `left`, never past `size` in `refunds`; it never opens a missing row or moves `until`.
6. **Privileges and purge.** `anon` and `authenticated` cannot execute either function or read the
   table; deleting the `accounts` row deletes the allowance row.
7. **`charge_call` and `refund_call` unchanged.** Their existing first-two-days tests still pass with
   no edit.

Cloud, functions (`_shared/judge_caps_test.ts`, the pipeline tests):

8. **Overflow order.** For `event`: cap refuses, allowance charges, the model is called. For `task`
   and `email`: cap refuses, `capped`, the allowance RPC is never called.
9. **Ceiling first.** `withinBudget` false: `capped`, neither charge RPC is called, no row is opened.
10. **Refund routing.** `model failed` on an allowance-charged call calls `refund_judge_allowance`
    and not `refund_call`; on a cap-charged call the reverse. A refusal, a truncation or a failed
    validation calls neither.
11. **Size pinned.** The TS mirror of the size (if one exists) equals the SQL's, as
    `allowance_usd_is_the_dollars_charge_allowance_opens_with` does for mail
    (`mail-process/worker_test.ts:959`).

Engine (`engine/src/events.rs` tests, off the contract list):

12. **A capped reply ends the pass.** A fake `EventModel` answering `ModelError::Capped` from item
    k+1: exactly k ledger verdicts, no per-item `not judged` line for the capped items, one summary
    whose `left` equals pending minus k, exit unaffected (`judge` exits 0).
13. **Order kept.** The k judged are the k soonest pending events (an existing ordering test may
    already pin this; if so, cite it rather than add one).

Live (the main session, staging, on Quinn's word to deploy): a new scratch profile at the T18c
school on a **new** account, and one on an **old** account: the first slot judges 150 with no
`capped` line; the allowance's `left` falls slot by slot; the backlog reaches `0 left` within the
expected days; the scratch profiles are removed afterwards.

## 10. Tasks

The plan (`docs/plans/`, with its fidelity ledger) will expand these. Files are disjoint between
T1/T2 and T3, so the cloud lane and the engine lane can run in parallel.

| # | Task | Files | Agent | Why that agent |
|---|---|---|---|---|
| T1 | The migration: `judge_allowances`, `charge_judge_allowance`, `refund_judge_allowance`; tests 1-7 | `cloud/supabase/migrations/2026100?000100_event_allowance.sql`, `migrations/migrations_test.ts` | `cloud-engineer` | SQL that charges money-bearing calls and runs under row locks; `cloud/` is its lane |
| T2 | The pipeline: overflow order for `event`, refund routing; tests 8-11 | `functions/_shared/judge_caps.ts`, `judge_pipeline.ts`, their tests | `cloud-engineer` | Same money path as T1; after T1 (it calls T1's RPC names; fakes until then) |
| T3 | The engine's capped pass ends with one line and an exact `left` (Q9); tests 12-13 | `engine/src/events.rs` | `implementer` | Fully specified, off the contract list, checked by tests |
| T4 | Review of T1-T3 before any push | none (report under `docs/reports/`) | `reviewer` | Cheaper-agent and cloud changes go through review; money path |
| T5 | Deploy the migration and functions to staging; the live proof (§9) | none | main session | Staging pushes and tokens are the controller's; deploy on Quinn's word |
| T6 | Record the table, the two RPCs and the proof (the cost comment in `judge_caps.ts` is T2's) | `HANDOFF.md`, `cloud/supabase/README.md` | `docs-keeper` | Docs only, after T5 |

If Q8 goes the other way (a "not read yet" line), it adds two tasks: the `rank`/surface field with the
three `surface-today-*.json` references regenerated in one commit that says why (`implementer`, then
`reviewer`; the oracle test is contract-list, so the regeneration commit goes to
`contract-reviewer`), and the console line (`console-ui`).

## 11. Decisions for Quinn

Each with a recommendation. Q1-Q3 and Q5-Q6 set the migration; the rest can be ruled later.

**Q1. Size.** 2,000 event judgments (recommended): about twice the one backlog measured, ~$0.22 at
most, and the same number as the mail allowance. Alternatives: 1,000 (just the measured school) or
1,500.

**Q2. When it opens.** At the account's first event judgment the day's cap refuses (recommended):
spent only on overflow, it serves a feed added after week two and the older account the doubling
misses, and it costs the same once. Alternative: at the account's first events judge run, as the
ruling's framing reads; then a feed enabled more than 14 days later gets nothing. Either way existing
staging accounts (the proof account included) get theirs at the next qualifying run.

**Q3. Window and renewal.** 14 days from opening, never renewed (recommended; one-time per the ruling).
At two slots a day, 2,000 drains in about seven days, so 14 leaves room for days the app is closed.

**Q4. Events only.** Recommended. Tasks and email keep the doubling (and D23 for mail backfill). The
table is keyed by `kind`, so tasks can join by a later ruling without reshaping it.

**Q5. The first-two-days doubling.** Keep it for every kind, with the allowance spent only after the
(doubled) event cap refuses (recommended). No change to `charge_call` or `refund_call`.

**Q6. The $2 ceiling.** Not raised while the allowance is open (recommended): the worst case is
~$0.22 and a fully capped account plus the allowance stays under $2. The ceiling is checked first.

**Q7. Per-run bound.** `judge_per_run_cap` stays 150 and the 15-minute budget is unchanged
(recommended). A slot stays as short as today; the backlog drains over three to four days.

**Q8. What the student sees.** Nothing new (recommended): the first slot completes the 14-day
horizon and *Coming up*. Alternative: a "M campus events not read yet" line, which moves the
read-model references.

**Q9. The capped-pass fix (T3).** Include it (recommended): one line instead of up to 150, and a
`left` that counts what was not judged. Small, independent, off the contract list.

**Q10. One per account, across vaults.** A second vault, a reinstall or a changed school on the same
account does not reopen it (recommended). The cost bound is per account, and a school change is rare
enough for the daily cap to absorb.

## 12. Out of scope

- A judging horizon (judging only the next N days of the roster) and a raised daily cap: set aside by
  the ruling.
- Concurrent event judgment within a slot, which would let one slot judge the whole backlog.
- Task and email backlogs (Q4), and any change to D23.
