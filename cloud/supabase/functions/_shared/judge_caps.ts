import type { Db } from "./judge_db.ts";
import type { Kind } from "./judge_validate.ts";

export interface CapStore {
  charge(account: string, kind: Kind): Promise<boolean>;
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
 * An account's first two UTC judging days charge against twice this cap; the SQL does it
 * (`charge_call`, migration `20260923000100`, R-C1c-7), and `p_cap` below stays this value.
 */
export const DAILY_CAP: Record<Kind, number> = { task: 60, event: 80, email: 120 };

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

export function capStore(db: Db): CapStore {
  // One budget check per account per invocation is enough: an edge function handles one request,
  // and `gmail-read` (Task 11) is the only caller that judges many items in one — it memoises.
  const budget = new Map<string, boolean>();
  return {
    async charge(account, kind) {
      // One statement (`charge_call`), so two concurrent calls cannot both read `calls` below the
      // cap and both write. A cap store that cannot be reached REFUSES: over-charging a student's
      // account is recoverable, an uncapped loop against a metered API is not.
      try {
        return await db.rpc("charge_call", { p_account: account, p_kind: kind, p_cap: DAILY_CAP[kind] }) ===
          true;
      } catch {
        return false;
      }
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
