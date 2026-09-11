/** Who is calling. Verification is injected, so every handler test runs with no network. */
import { fail } from "./http.ts";

export interface AuthedUser {
  id: string;
  email: string | null;
}

export type VerifyToken = (token: string) => Promise<AuthedUser | null>;

/** `Authorization: Bearer <token>`, case-insensitive on both the header and the scheme. */
export function parseBearer(req: Request): string | null {
  const h = req.headers.get("authorization");
  if (!h) return null;
  const m = /^bearer\s+(\S+)\s*$/i.exec(h.trim());
  return m ? m[1] : null;
}

/** **Throws a `Response`** (401) rather than returning an error: see `_shared/http.ts`. */
export async function requireUser(req: Request, verify: VerifyToken): Promise<AuthedUser> {
  const token = parseBearer(req);
  if (!token) throw fail(401, "no bearer token");
  const user = await verify(token);
  if (!user) throw fail(401, "the session is not valid");
  return user;
}
