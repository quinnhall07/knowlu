/**
 * `GET /entitlement` — spec §5.1. The device calls this at launch and every six hours and caches the
 * answer with a 72-hour grace, so **an account with no subscription is a 200 saying `none`**, never a
 * 404: the app has to be able to tell "you are not subscribed" from "the network is not there", and
 * a status code is the only thing it can tell them apart by.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { EntitlementRow, LookupEntitlement } from "../_shared/entitlement.ts";
import { json, methodNotAllowed } from "../_shared/http.ts";

export interface Deps {
  verify: VerifyToken;
  lookup: LookupEntitlement;
  now: () => Date;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "GET") return methodNotAllowed(["GET"]);
  const user = await requireUser(req, deps.verify);
  const row: EntitlementRow | null = await deps.lookup(user.id);
  return json(200, {
    status: row?.status ?? "none",
    current_period_end: row?.current_period_end ?? null,
    plan: row?.plan ?? null,
    checked_at: deps.now().toISOString(),
  });
}
