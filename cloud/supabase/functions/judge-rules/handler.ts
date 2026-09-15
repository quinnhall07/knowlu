// GET  /judge-rules  -> the promoted-rule proposals this account has not decided.
// POST /judge-rules  -> { id, decision } — approve (activate) or reject (settle) one.
//
// This is also the service's cheapest authenticated call, so `CloudModel::probe()` uses it to
// learn once per run whether an account can be judged at all: it charges no cap, spends no model
// tokens, and answers 401/402 exactly as every other endpoint does.
import type { Entitle } from "../_shared/judge_handler.ts";

export interface RuleDeps {
  pending(accountId: string): Promise<Array<{
    id: number; kind: string; feature: string; value: string;
    verdict: Record<string, unknown>; proposed_at: string;
  }>>;
  decide(accountId: string, id: number, decision: "approved" | "rejected"): Promise<boolean>;
}

export function rulesHandler(entitle: Entitle, deps: RuleDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      const { account_id } = await entitle(req);
      if (req.method === "GET") {
        return Response.json({ proposals: await deps.pending(account_id) });
      }
      if (req.method !== "POST") return Response.json({ error: "GET or POST" }, { status: 405 });
      const body = await req.json().catch(() => ({})) as { id?: unknown; decision?: unknown };
      const id = typeof body.id === "number" ? body.id : NaN;
      const decision = body.decision;
      if (!Number.isInteger(id) || (decision !== "approved" && decision !== "rejected")) {
        return Response.json({ error: "expected { id: integer, decision: approved|rejected }" }, { status: 400 });
      }
      // The account id is part of the update's filter, not merely checked: a decision on somebody
      // else's proposal must change nothing and say so.
      const changed = await deps.decide(account_id, id, decision);
      if (!changed) return Response.json({ error: "no such undecided proposal" }, { status: 404 });
      return Response.json({ decided: decision });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`judge-rules: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "rules failed" }, { status: 500 });
    }
  };
}
