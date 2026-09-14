// PostgREST over `fetch`, with the service role. Not `supabase-js`: C1 established that this
// codebase has no runtime dependency but the inference SDK, and one HTTP client for one schema is
// smaller than a library that brings its own.
//
// C1 already has `_shared/db.ts` (`Rest`, `restSelect`, `restUpsert`, …): free functions taking a
// `Rest` struct, no interface. This file exists beside it, not instead of it, because the pipeline
// (`judge_pipeline.ts`) is injected with a `Db` — an interface, not a set of free functions — so a
// test can substitute an in-memory fake, and because `judge_db_test.ts`'s account-scoping guard
// scans this directory's source for the `.select(` call SHAPE this interface produces.
//
// **The service role bypasses RLS**, which is exactly why every C2 table has RLS on and no policy
// (Task 1) and why every call site here scopes by `account_id` in the query string. That scoping
// is the whole access control, so `judge_db_test.ts` scans this directory's `.ts` files for a
// `select`/`insert` on an account-scoped table that omits it — one missing `.eq` would be a
// cross-account read with no database backstop.
export interface Db {
  /** A PostgREST path, e.g. `models?kind=eq.task&select=*`. Returns the rows. */
  select(path: string): Promise<unknown[]>;
  /** Inserts one row and returns it (`Prefer: return=representation`), or null when asked not to. */
  insert(
    table: string,
    row: Record<string, unknown>,
    returning?: boolean,
  ): Promise<Record<string, unknown> | null>;
  update(path: string, patch: Record<string, unknown>): Promise<void>;
  /** `POST /rpc/<fn>`. */
  rpc(fn: string, args: Record<string, unknown>): Promise<unknown>;
}

function env(name: string): string {
  const value = Deno.env.get(name);
  if (value === undefined || value === "") throw new Error(`the function is missing ${name}`);
  return value;
}

/// Built lazily, at first use — never at module scope. A throw at module scope is a boot failure
/// with an opaque message; a throw here is caught by the handler and becomes a named 500.
export function serviceDb(): Db {
  const base = `${env("SUPABASE_URL")}/rest/v1`;
  const key = env("SUPABASE_SERVICE_ROLE_KEY");
  const headers = { apikey: key, Authorization: `Bearer ${key}`, "Content-Type": "application/json" };

  async function call(path: string, init: RequestInit): Promise<Response> {
    const response = await fetch(`${base}/${path}`, {
      ...init,
      headers: { ...headers, ...(init.headers ?? {}) },
    });
    if (!response.ok) {
      // The status and the PostgREST error CODE, never the body: a constraint violation's message
      // quotes the offending row, and this string reaches a log line (§5.6).
      const code = (await response.json().catch(() => ({}))).code ?? "unknown";
      throw new Error(`postgrest ${response.status} (${code}) on ${path.split("?")[0]}`);
    }
    return response;
  }

  return {
    async select(path) {
      return await (await call(path, { method: "GET" })).json();
    },
    async insert(table, row, returning = true) {
      const response = await call(table, {
        method: "POST",
        body: JSON.stringify(row),
        headers: { Prefer: returning ? "return=representation" : "return=minimal" },
      });
      if (!returning) return null;
      const rows = await response.json();
      return Array.isArray(rows) && rows.length > 0 ? rows[0] : null;
    },
    async update(path, patch) {
      await call(path, {
        method: "PATCH",
        body: JSON.stringify(patch),
        headers: { Prefer: "return=minimal" },
      });
    },
    async rpc(fn, args) {
      return await (await call(`rpc/${fn}`, { method: "POST", body: JSON.stringify(args) })).json();
    },
  };
}
