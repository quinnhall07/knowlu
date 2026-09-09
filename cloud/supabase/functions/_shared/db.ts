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
  // The upstream body can name a column, a constraint or a row; it never reaches the caller.
  console.error(`${what}: ${res.status} ${await res.text()}`);
  throw fail(502, `${what} failed`);
}

/** `query` is a PostgREST query string without the leading `?`, e.g. `account_id=eq.<id>&select=*`. */
export async function restSelect<T>(rest: Rest, table: string, query: string): Promise<T[]> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, { headers: headers(rest) });
  await ok(res, `select ${table}`);
  return await res.json() as T[];
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
