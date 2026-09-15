// cloud/supabase/functions/judge-rules/index.ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { rulesHandler } from "./handler.ts";

Deno.serve(rulesHandler(requireActiveEntitlement, {
  async pending(accountId) {
    return await sharedDb().select(
      `rules?account_id=eq.${accountId}&active=is.false&decided_at=is.null&select=id,kind,feature,value,verdict,proposed_at&order=proposed_at`,
    ) as Array<{ id: number; kind: string; feature: string; value: string; verdict: Record<string, unknown>; proposed_at: string }>;
  },
  async decide(accountId, id, decision) {
    const before = await sharedDb().select(
      `rules?account_id=eq.${accountId}&id=eq.${id}&decided_at=is.null&select=id`,
    ) as Array<{ id: number }>;
    if (before.length === 0) return false;
    await sharedDb().update(
      `rules?account_id=eq.${accountId}&id=eq.${id}&decided_at=is.null`,
      { active: decision === "approved", decided_at: new Date().toISOString() },
    );
    return true;
  },
}));
