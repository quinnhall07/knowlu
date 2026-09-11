/**
 * Postgres and Auth over `fetch`, against Supabase's own REST and Auth APIs. **No SDK**: the whole
 * of what this codebase needs is six calls, and depending on `supabase-js` would put a registry
 * fetch in front of `deno check` and a moving surface under every handler.
 *
 * `fetch` is a field on `Rest`, not the global, so every test injects its own and no test in this
 * codebase can reach the network.
 */
import { AuthedUser } from "./auth.ts";
import { fail } from "./http.ts";

export interface Rest {
  /** `https://<ref>.supabase.co` — no trailing slash. */
  url: string;
  /** The service-role key. Bypasses RLS: this is the only writer any table has. */
  serviceKey: string;
  fetch: typeof fetch;
}

function headers(rest: Rest, extra: Record<string, string> = {}): Record<string, string> {
  return {
    apikey: rest.serviceKey,
    authorization: `Bearer ${rest.serviceKey}`,
    "content-type": "application/json",
    ...extra,
  };
}

async function ok(res: Response, what: string): Promise<void> {
  if (res.ok) return;
  // R-C1-59 (M8): the upstream body can echo an offending value (a caller's IP-ish header on a
  // `consents` constraint failure, an object id on a `telemetry_events` one) — it never reaches the
  // CALLER, which is the promise this makes and keeps (`fail(502, …)` below carries no body of its
  // own). It is kept in the function LOG on purpose: nothing here is another user's data, and a 502
  // with no detail anywhere is not one this on-call can debug.
  console.error(`${what}: ${res.status} ${await res.text()}`);
  throw fail(502, `${what} failed`);
}

/** `query` is a PostgREST query string without the leading `?`, e.g. `account_id=eq.<id>&select=*`. */
export async function restSelect<T>(rest: Rest, table: string, query: string): Promise<T[]> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, { headers: headers(rest) });
  await ok(res, `select ${table}`);
  return await res.json() as T[];
}

/**
 * `restSelect`, but past PostgREST's own row cap (`config.toml`'s `max_rows`, 1000 in this project):
 * one project-wide setting a single `select` cannot see past, silently. Pages by `limit`/`offset`
 * until a page comes back shorter than `pageSize`, which is also how the last page is recognised
 * with no separate count call.
 *
 * **The caller's `query` must carry its own `order=`.** `offset` paging is only stable over a
 * deterministically ordered result; without one, PostgREST is free to hand back the same row twice
 * across two pages, or skip one, as the underlying scan plan shifts between requests.
 */
export async function restSelectAll<T>(
  rest: Rest,
  table: string,
  query: string,
  pageSize = 1000,
): Promise<T[]> {
  const out: T[] = [];
  for (let offset = 0;; offset += pageSize) {
    const page = await restSelect<T>(rest, table, `${query}&limit=${pageSize}&offset=${offset}`);
    out.push(...page);
    if (page.length < pageSize) return out;
  }
}

export async function restUpsert(
  rest: Rest,
  table: string,
  rows: unknown[],
  onConflict?: string,
): Promise<void> {
  const q = onConflict ? `?on_conflict=${onConflict}` : "";
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}${q}`, {
    method: "POST",
    headers: headers(rest, { prefer: "resolution=merge-duplicates,return=minimal" }),
    body: JSON.stringify(rows),
  });
  await ok(res, `upsert ${table}`);
}

export async function restPatch(rest: Rest, table: string, query: string, patch: unknown): Promise<void> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, {
    method: "PATCH",
    headers: headers(rest, { prefer: "return=minimal" }),
    body: JSON.stringify(patch),
  });
  await ok(res, `patch ${table}`);
}

export async function restDelete(rest: Rest, table: string, query: string): Promise<void> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, {
    method: "DELETE",
    headers: headers(rest, { prefer: "return=minimal" }),
  });
  await ok(res, `delete ${table}`);
}

/** GoTrue's `/auth/v1/user` with the caller's own token: this is what verifies a session. */
export async function authGetUser(rest: Rest, token: string): Promise<AuthedUser | null> {
  const res = await rest.fetch(`${rest.url}/auth/v1/user`, {
    headers: { apikey: rest.serviceKey, authorization: `Bearer ${token}` },
  });
  if (!res.ok) return null;
  const body = await res.json() as { id?: string; email?: string };
  return body.id ? { id: body.id, email: body.email ?? null } : null;
}

export async function authDeleteUser(rest: Rest, id: string): Promise<void> {
  const res = await rest.fetch(`${rest.url}/auth/v1/admin/users/${id}`, {
    method: "DELETE",
    headers: headers(rest),
  });
  await ok(res, "delete auth user");
}

/** The one place an `index.ts` builds a `Rest` from the platform's own variables. */
export function restFromEnv(): Rest {
  const url = Deno.env.get("SUPABASE_URL");
  const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY");
  if (!url || !serviceKey) throw fail(500, "the function is not configured");
  return { url: url.replace(/\/+$/, ""), serviceKey, fetch: globalThis.fetch };
}
