/**
 * **The gate C2 imports.** `requireActiveEntitlement(req)` resolves to the caller's account id, or
 * **throws a `Response`**: 401 with no valid bearer token, **402** with no active or trialing
 * subscription. C2's handlers wrap it as
 *
 * ```ts
 * try { const { account_id } = await requireActiveEntitlement(req); … }
 * catch (e) { if (e instanceof Response) return e; throw e; }
 * ```
 *
 * The `…With` twin takes its dependencies explicitly and is what every test drives; the one-argument
 * form builds them from the platform's environment, once, and is what production calls. Splitting
 * them is what lets this file be tested with no environment, no database and no network.
 */
import { AuthedUser, requireUser, VerifyToken } from "./auth.ts";
import { authGetUser, Rest, restFromEnv, restSelect } from "./db.ts";
import { fail } from "./http.ts";

export type EntitlementStatus = "active" | "trialing" | "past_due" | "canceled" | "none";

/** The two statuses that mean "the cloud may work for this account". */
export const ACTIVE_STATUSES: readonly EntitlementStatus[] = ["active", "trialing"];

export interface EntitlementRow {
  plan: string | null;
  status: EntitlementStatus;
  current_period_end: string | null;
}

export type LookupEntitlement = (accountId: string) => Promise<EntitlementRow | null>;

export interface EntitlementDeps {
  verify: VerifyToken;
  lookup: LookupEntitlement;
}

export function isActive(row: EntitlementRow | null): boolean {
  return row !== null && ACTIVE_STATUSES.includes(row.status);
}

export async function requireActiveEntitlementWith(
  req: Request,
  deps: EntitlementDeps,
): Promise<{ account_id: string }> {
  const user: AuthedUser = await requireUser(req, deps.verify);
  const row = await deps.lookup(user.id);
  if (!isActive(row)) throw fail(402, "this account has no active subscription");
  return { account_id: user.id };
}

/** The production lookup: one PostgREST read with the service role. */
export function lookupFrom(rest: Rest): LookupEntitlement {
  return async (accountId: string) => {
    const rows = await restSelect<EntitlementRow>(
      rest,
      "entitlements",
      `account_id=eq.${encodeURIComponent(accountId)}&select=plan,status,current_period_end&limit=1`,
    );
    return rows[0] ?? null;
  };
}

export function depsFrom(rest: Rest): EntitlementDeps {
  return { verify: (token) => authGetUser(rest, token), lookup: lookupFrom(rest) };
}

let cached: EntitlementDeps | null = null;

export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }> {
  cached ??= depsFrom(restFromEnv());
  return await requireActiveEntitlementWith(req, cached);
}
