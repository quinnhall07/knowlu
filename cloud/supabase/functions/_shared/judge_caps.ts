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
 * i.e. **~$2.50 a month** against a $9.99 subscription. The caps below are ~4x that heavy day, so
 * a runaway loop costs at most ~$0.36 a day and ~$11 a month — still above the subscription, which
 * is why `MONTHLY_CEILING_USD` exists and is enforced rather than merely reported.
 *
 * (The earlier draft's 200/300/500 permitted 1,000 calls a day, about $45 a month per account —
 * 4.5x the price of the product. Caps and price are chosen together from here on.)
 */
export const DAILY_CAP: Record<Kind, number> = { task: 60, event: 80, email: 120 };

/**
 * The enforced monthly ceiling per account, in dollars. ~3x plausible heavy use, and about a
 * quarter of the subscription — a number the business can absorb for every account at once. Past
 * it the judgment is refused with outcome `capped` and one row lands in `budget_alerts`, which is
 * what makes the overspend visible without querying a view nobody queries.
 */
export const MONTHLY_CEILING_USD = 7.5;

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
