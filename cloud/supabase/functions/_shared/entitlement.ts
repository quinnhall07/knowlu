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
  /**
   * Counts one gated call into `route_daily` (friends-testers spec §4). Optional, so every existing
   * `{ verify, lookup }` still compiles; production supplies it only while `ROUTE_COUNTS` reads `on`.
   */
  count?: (accountId: string, route: string) => Promise<void>;
  /** Keeps a count's promise alive past the response (`EdgeRuntime.waitUntil`); never awaited. */
  defer?: (p: Promise<unknown>) => void;
}

export type EnvGet = (name: string) => string | undefined;

/** `count_route`'s own pattern (the `route_daily.route` check); a name outside it is never sent. */
export const ROUTE_PATTERN = /^[a-z][a-z0-9-]{0,39}$/;

/**
 * The function a request reached: the first path segment after an optional `/functions/v1`, the same
 * tolerance as `subPath` (a direct invoke or `supabase functions serve` has no prefix). `null` for no
 * segment or one outside `ROUTE_PATTERN`, so the gate skips the count rather than send a value the RPC
 * would refuse, a refusal that is swallowed and would leave `route_daily` silently empty.
 */
export function routeName(url: string): string | null {
  const segments = new URL(url).pathname.split("/").filter((s) => s !== "");
  const at = segments[0] === "functions" && segments[1] === "v1" ? 2 : 0;
  const name = segments[at];
  return name !== undefined && ROUTE_PATTERN.test(name) ? name : null;
}

/** Starts the count, never awaited: every error swallowed, the promise handed to `defer`. */
function countCall(deps: EntitlementDeps, accountId: string, url: string): void {
  const route = deps.count ? routeName(url) : null;
  if (!deps.count || route === null) return;
  let p: Promise<unknown>;
  try {
    p = deps.count(accountId, route).catch(() => undefined);
  } catch {
    p = Promise.resolve();
  }
  try {
    deps.defer?.(p);
  } catch {
    // A runtime that cannot keep the isolate alive only loses this count.
  }
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
  countCall(deps, user.id, req.url);
  return { account_id: user.id };
}

/**
 * The production lookup: one PostgREST read with the service role, of `effective_entitlements`
 * (friends-testers spec §3.2): the Stripe row unchanged, or `active`/`tester`/the grant's end for an
 * account with an open, unexpired comp grant and no paid row. Its four column names are pinned against
 * the testers migration in `entitlement_test.ts`.
 */
export function lookupFrom(rest: Rest): LookupEntitlement {
  return async (accountId: string) => {
    const rows = await restSelect<EntitlementRow>(
      rest,
      "effective_entitlements",
      `account_id=eq.${encodeURIComponent(accountId)}&select=plan,status,current_period_end&limit=1`,
    );
    return rows[0] ?? null;
  };
}

/** One `count_route` call by the service role. A non-2xx throws, and the gate swallows it. */
export function countFrom(rest: Rest): (accountId: string, route: string) => Promise<void> {
  return async (accountId, route) => {
    const res = await rest.fetch(`${rest.url}/rest/v1/rpc/count_route`, {
      method: "POST",
      headers: {
        apikey: rest.serviceKey,
        authorization: `Bearer ${rest.serviceKey}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({ p_account: accountId, p_route: route }),
    });
    await res.body?.cancel();
    if (!res.ok) throw new Error(`count_route: ${res.status}`);
  };
}

type Runtime = { EdgeRuntime?: { waitUntil?: (p: Promise<unknown>) => void } };

/**
 * The production deps. `count` only while the `ROUTE_COUNTS` secret reads exactly `on` (§4: off until
 * the privacy page says so); read once per isolate, since `requireActiveEntitlement` caches these, so
 * turning it on or off takes a redeploy. `defer` is `EdgeRuntime.waitUntil` where that global exists.
 */
export function depsFrom(
  rest: Rest,
  env: EnvGet = (name) => Deno.env.get(name),
  runtime: Runtime = globalThis as Runtime,
): EntitlementDeps {
  const deps: EntitlementDeps = { verify: (token) => authGetUser(rest, token), lookup: lookupFrom(rest) };
  if (env("ROUTE_COUNTS") !== "on") return deps;
  const edge = runtime.EdgeRuntime;
  return {
    ...deps,
    count: countFrom(rest),
    defer: typeof edge?.waitUntil === "function" ? (p) => edge.waitUntil!(p) : () => {},
  };
}

let cached: EntitlementDeps | null = null;

export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }> {
  cached ??= depsFrom(restFromEnv());
  return await requireActiveEntitlementWith(req, cached);
}
