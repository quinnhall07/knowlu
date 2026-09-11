/**
 * Responses and request reading, shared by every function. **No I/O of its own**: a handler that
 * needs the network or the database takes it as an injected dependency, which is what lets
 * `deno test` run this whole codebase with `--allow-read` and nothing else.
 *
 * Failures are thrown as `Response` objects rather than as errors with codes. That is the contract
 * `_shared/entitlement.ts` promises C2 (`throws a Response`, 401 or 402), and it means a handler's
 * happy path reads straight down with no error plumbing in it.
 */

export const JSON_HEADERS = { "content-type": "application/json; charset=utf-8" } as const;

export function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { ...JSON_HEADERS } });
}

/** Every error body in this codebase has exactly one shape: `{"error": "<one sentence>"}`. */
export function fail(status: number, message: string): Response {
  return json(status, { error: message });
}

export function methodNotAllowed(allowed: string[]): Response {
  const list = allowed.join(", ");
  return new Response(JSON.stringify({ error: `method not allowed; use ${list}` }), {
    status: 405,
    headers: { ...JSON_HEADERS, allow: list },
  });
}

/**
 * The path inside one function, with the platform's `/functions/v1/<name>` prefix removed, so a
 * single function can carry the several routes the spec names (`/account`, `/account/export`,
 * `/account/sources`). A request that never had the prefix — a direct invoke in a test, or a
 * `supabase functions serve` call — is returned unchanged, so the routing is the same in both.
 */
export function subPath(url: string, fnName: string): string {
  const path = new URL(url).pathname;
  const marker = `/${fnName}`;
  const at = path.indexOf(marker);
  const rest = at < 0 ? path : path.slice(at + marker.length);
  return rest === "" ? "/" : rest;
}

/**
 * The body, parsed, with a size cap. **Throws a `Response`** — 413 over the cap, 400 for anything
 * that is not JSON — so a caller never has to tell a parse failure from a bug.
 */
export async function readJson<T>(req: Request, limit = 1 << 20): Promise<T> {
  const text = await req.text();
  // `String.length` is UTF-16 code units, not bytes — say what is measured rather than
  // pretending to a precision this does not have. The cap is a sanity bound, not an accounting.
  if (text.length > limit) throw fail(413, `body over ${limit} characters`);
  try {
    return JSON.parse(text) as T;
  } catch {
    throw fail(400, "body is not JSON");
  }
}

/**
 * The one `catch` every `index.ts` uses. A thrown `Response` is the handler's own answer and goes
 * back verbatim; **anything else becomes a bare 500**, because a stray error's message can carry a
 * connection string, a key or a row of somebody's data and an edge function's body is public.
 */
export function asResponse(e: unknown): Response {
  if (e instanceof Response) return e;
  console.error("unhandled:", e instanceof Error ? e.message : String(e));
  return fail(500, "internal error");
}
