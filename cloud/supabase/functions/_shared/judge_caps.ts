import type { Db } from "./judge_db.ts";
import type { Kind, MailKind } from "./judge_validate.ts";

/** What one `charge_call` answered (L3, spec 2026-10-06 §3 step 3). Only a real `false` is a
 *  refusal; a thrown or rejected rpc, or any answer that is not a boolean, is an `error`. */
export type ChargeOutcome = "charged" | "refused" | "error";

export interface CapStore {
  /** The daily cap's one charge, failing closed: it folds `"refused"` and `"error"` into `false`.
   *  The pipeline calls `chargeOutcome`, which must tell the two apart. */
  charge(account: string, kind: Kind): Promise<boolean>;
  /** The same one `charge_call` as `charge`, with the same args, answering which of the three it
   *  was. The first-day event allowance opens only on a `"refused"`, so a transient error or a broken
   *  signature can never open or spend it while the cap had room. Never counts toward `tripped`. */
  chargeOutcome(account: string, kind: Kind): Promise<ChargeOutcome>;
  /** One judgment charged to the account's one-time allowance (`charge_judge_allowance`), after
   *  `chargeOutcome` answered `"refused"`. A database it cannot reach REFUSES (`false`): here a
   *  refusal and an error both mean `capped`. */
  chargeAllowance(account: string, kind: Kind): Promise<boolean>;
  /** Gives back the judgment `chargeAllowance` counted, after a call the model never answered
   *  (`refund_judge_allowance`). Never throws, and counts one failure toward `tripped` as `refund`
   *  does: the breaker counts every `model failed`, whichever store charged it. */
  refundAllowance(account: string, kind: Kind): Promise<void>;
  /** Gives back the call `charge` counted, after a call the model never answered (F1, live proof
   *  2026-10-02). Never throws. Also counts one more consecutive failure toward `tripped`. */
  refund(account: string, kind: Kind): Promise<void>;
  /** True once `BREAKER_FAILURES` calls in a row failed in this store's life (one invocation) with
   *  no reply between: the pipeline then stops sending and answers `model failed` uncharged. */
  tripped(): boolean;
  withinBudget(account: string): Promise<boolean>;
  recordTokens(account: string, kind: Kind, inTokens: number, outTokens: number): Promise<void>;
}

/**
 * §5.2's rate guards — **and the price they imply, worked out here so the next reader can check it.**
 *
 *   Haiku 4.5 is $1.00 / $5.00 per MTok (claude-api skill, 2026-09-09).
 *   This plan's own bounds: system ~280 tokens; user up to ~700 (body 1200 chars + weights 600 +
 *   preferences 600 + title 200, at ~4 chars/token); `max_tokens` 256 for task and event, 640 for
 *   email, of which a real reply uses ~80.
 *   So one call is about (980 / 1e6 * $1.00) + (80 / 1e6 * $5.00) = **$0.0014**, and email's
 *   longer cap does not change that because the cap is not what is billed.
 *
 * A plausible heavy day is ~10 enrichments, ~20 event verdicts, ~30 emails = 60 calls = $0.084,
 * i.e. **~$2.50 a month** against a $9.99 subscription. The caps below are ~4x that heavy day.
 *
 * (The earlier draft's 200/300/500 permitted 1,000 calls a day, about $45 a month per account —
 * 4.5x the price of the product. Caps and price are chosen together from here on.)
 *
 * CORRECTION (comment only, provider swap Task 2 fix 1, 2026-09-16): the paragraph above priced
 * these caps on Haiku 4.5; on the new OpenRouter pins (Granite 4.2 8B for task/event, Qwen3.5-35B-
 * A3B for email — see `MONTHLY_CEILING_USD` below) a fully-capped account (60 + 80 + 120 calls a
 * day, every day) costs about **$1.3 a month**, UNDER the $2 `MONTHLY_CEILING_USD`. The daily caps
 * therefore no longer bound spend below the monthly ceiling on their own the way "~4x that heavy
 * day" once did — `DAILY_CAP` now exists to bound REQUEST VOLUME per kind (protecting the upstream
 * and the database from a hot loop), while `MONTHLY_CEILING_USD` is what actually guards spend, and
 * it is sized to survive a mispriced or re-pinned row, or the day a heavier model is pinned above
 * these — not to survive a capped account running flat out on these prices.
 *
 * RE-PIN (comment only, task-3, 2026-10-23): the task kind moved to Qwen3.5-35B-A3B at $0.14/$1.00
 * (`20261023000100_task_effort_anchors.sql`); event stays on Granite 4.2 8B. At ~980 tokens in and
 * ~80 out a task call is ~$0.00022 (was ~$0.00011), so a fully-capped account is about **$1.45 a
 * month** (was ~$1.3), plus the event allowance's ~$0.22 about $1.66, still under the $2
 * `MONTHLY_CEILING_USD`; the heavy persona below rises from ~$0.87 to ~$0.94, the lighter one from
 * ~$0.29 to ~$0.32. No cap or ceiling moves.
 *
 * An account's first two UTC judging days charge against twice this cap; the SQL does it
 * (`charge_call`, migration `20260923000100`, R-C1c-7), and `p_cap` below stays this value.
 */
export const DAILY_CAP: Record<Kind, number> = { task: 60, event: 80, email: 120 };

/**
 * The mail kinds' daily caps (email spec §5.1, plan P6): their own record, keyed by `MailKind`, so
 * `DAILY_CAP` and every `Record<Kind, …>` stay as they were. `mail_claim` starts at email's 120 and
 * charges only `live` and `recover` items; a `backfill` item charges the account's allowance instead
 * (D23), never this cap. `mail_resolve` is one call per course batch of ambiguous claims (D12), so 20
 * a day is many backfills' worth. `mail_digest` is 0 until the optional digest (plan T16) is built and
 * sets its own number: a cap of 0 refuses every call, so an unbuilt kind can never spend.
 */
export const MAIL_DAILY_CAP: Record<MailKind, number> = { mail_claim: 120, mail_resolve: 20, mail_digest: 0 };

/**
 * The enforced monthly ceiling per account, in dollars. The provider swap (Quinn's ruling of
 * 2026-09-16 on R8, option 1) re-pinned all three kinds off Haiku 4.5 ($1.00/$5.00 per MTok) onto
 * OpenRouter: Granite 4.2 8B for task/event at $0.10/$0.15, Qwen3.5-35B-A3B for email at
 * $0.14/$1.00 — an order of magnitude cheaper per call. Two personas, two different numbers, so
 * state both rather than pick one:
 *   - The HEAVY STUDENT of `docs/notes/2026-09-16-inference-provider-and-model-scoping.md` §2 —
 *     150 calls a day (20 task, 40 event, 90 email), about 950 tokens in and 80 to 130 out per
 *     call — costs about **$0.87 a month** on the new pins.
 *   - This file's own LIGHTER estimate just below (~10 enrichments, ~20 event verdicts, ~30
 *     emails a day, all at ~980 in / ~80 out) costs about **$0.29 a month** on the new pins.
 * Either way it is a fraction of the ~$2.50 (light) to ~$6.44 (heavy, per the scoping note's own
 * corrected table) Haiku's pricing implied. The ceiling is therefore no longer sized as a multiple
 * of plausible heavy use — at $2 it sits comfortably above even the heavy persona's real month —
 * and its job is to be a flat runaway guard cheap enough that even a looping bug cannot approach
 * the $9.99 subscription before it trips, never a budget line the product plans around. Past it
 * the judgment is refused with outcome `capped` and one row lands in `budget_alerts`, which is
 * what makes the overspend visible without querying a view nobody queries.
 */
export const MONTHLY_CEILING_USD = 2.0;

/**
 * F1 (live proof 2026-10-02): how many model calls in a row may fail inside one invocation before
 * the rest of it stops sending. Only `gmail-read` judges many items in one invocation; for it, a
 * dead key or an upstream outage ends the round's model calls after three, not after every email
 * on the list. Each `judge-*` request is its own invocation and its own store, so across requests
 * the bound on a failing loop is `refund_call`'s daily refund allowance instead.
 */
export const BREAKER_FAILURES = 3;

/**
 * The first-day event allowance (L3, spec 2026-10-06 §2, ruled 2026-10-06): once per account, this
 * many event judgments, spent only on the event calls `charge_call` refuses, for this many days from
 * the first such refusal, never renewed. **Mirrors of the migration (the one `*_event_allowance.sql`),
 * never sent to it:** `charge_judge_allowance` and `refund_judge_allowance` take only the account and
 * the kind, and the SQL alone decides the size and the window. A test pins these to its text.
 *
 * What it costs (spec §6), worked out here as the CORRECTION paragraph above works its numbers:
 * event judgments are pinned to Granite 4.2 8B at $0.10 in / $0.15 out per MTok, so at ~980 tokens in
 * and ~80 out one call is (980 / 1e6 * $0.10) + (80 / 1e6 * $0.15) = $0.000098 + $0.000012 =
 * **~$0.00011**, and the whole allowance is 2,000 * $0.00011 = **~$0.22, once per account**. Its
 * refund bound lets at most twice its size reach the model, but a refunded call is one the model never
 * answered and records no tokens, so the bill stays ~$0.22. A fully capped account (~$1.3 a month,
 * above) plus the whole allowance is ~$1.52, under `MONTHLY_CEILING_USD`, which is **not raised** while
 * the allowance is open and is checked before either charge.
 */
export const EVENT_ALLOWANCE_CALLS = 2000;
export const EVENT_ALLOWANCE_DAYS = 14;

export function capStore(db: Db): CapStore {
  // One budget check per account per invocation is enough: an edge function handles one request,
  // and `gmail-read` (Task 11) is the only caller that judges many items in one — it memoises.
  const budget = new Map<string, boolean>();
  let failures = 0;
  // One statement (`charge_call`), so two concurrent calls cannot both read `calls` below the cap
  // and both write. Only a real `false` is a refusal; anything else that is not `true` is an error.
  const chargeOutcome = async (account: string, kind: Kind): Promise<ChargeOutcome> => {
    let answer: unknown;
    try {
      answer = await db.rpc("charge_call", { p_account: account, p_kind: kind, p_cap: DAILY_CAP[kind] });
    } catch {
      return "error";
    }
    if (answer === true) return "charged";
    if (answer === false) return "refused";
    return "error";
  };
  return {
    async charge(account, kind) {
      // A cap store that cannot be reached REFUSES: over-charging a student's account is
      // recoverable, an uncapped loop against a metered API is not.
      return (await chargeOutcome(account, kind)) === "charged";
    },
    chargeOutcome,
    async chargeAllowance(account, kind) {
      // One statement on the allowance's key (`charge_judge_allowance`). Unreachable means refused.
      try {
        return await db.rpc("charge_judge_allowance", { p_account: account, p_kind: kind }) === true;
      } catch {
        return false;
      }
    },
    async refundAllowance(account, kind) {
      failures += 1;
      // Never throws, as `refund`: a refund that cannot land leaves the judgment charged to the
      // allowance. `refund_judge_allowance` bounds itself (open, within its size, and within its
      // size in lifetime refunds), so this needs no bound of its own.
      try {
        await db.rpc("refund_judge_allowance", { p_account: account, p_kind: kind });
      } catch {
        // Counted nowhere on purpose, as `refund` above.
      }
    },
    async refund(account, kind) {
      failures += 1;
      // Never throws: a refund that cannot land leaves the call charged, which is exactly the
      // pre-F1 behaviour — recoverable, and never an uncapped loop. `refund_call` bounds itself
      // (migration `20261002000100`), so this needs no bound of its own.
      try {
        await db.rpc("refund_call", { p_account: account, p_kind: kind, p_cap: DAILY_CAP[kind] });
      } catch {
        // Counted nowhere on purpose, as `recordTokens` below.
      }
    },
    tripped() {
      return failures >= BREAKER_FAILURES;
    },
    async withinBudget(account) {
      const cached = budget.get(account);
      if (cached !== undefined) return cached;
      let ok = false;
      try {
        ok = await db.rpc("enforce_budget", { p_account: account, p_ceiling: MONTHLY_CEILING_USD }) === true;
      } catch {
        ok = false;
      }
      budget.set(account, ok);
      return ok;
    },
    async recordTokens(account, kind, inTokens, outTokens) {
      // A reply arrived, so the provider is answering: the breaker's streak starts again.
      failures = 0;
      // Never throws: the tokens are the bill's record, and a failure to write them must not lose
      // a judgment that already happened.
      try {
        await db.rpc("record_tokens", {
          p_account: account,
          p_kind: kind,
          p_in: inTokens,
          p_out: outTokens,
        });
      } catch {
        // Counted nowhere on purpose — the next call's `enforce_budget` reads the same table and
        // will still see every call that did record.
      }
    },
  };
}
