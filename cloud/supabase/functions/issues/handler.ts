import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { scrub, scrubJson } from "../_shared/scrub.ts";

const MAX_BODY = 8192;

export interface Deps {
  verify: VerifyToken;
  save: (row: unknown) => Promise<string>;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const b = await readJson<{
    body?: string;
    payload?: Record<string, unknown>;
    app_version?: string;
    engine_build?: string;
    os_build?: string;
    profile_id?: string;
  }>(req, 1 << 18);
  const text = String(b.body ?? "").trim();
  if (!text) throw fail(400, "an issue report needs a sentence about what went wrong");
  if (text.length > MAX_BODY) throw fail(400, `the report is longer than ${MAX_BODY} characters`);

  const id = await deps.save({
    account_id: user.id,
    body: scrub(text),
    payload: scrubJson(b.payload ?? {}),
    app_version: b.app_version ?? null,
    engine_build: b.engine_build ?? null,
    os_build: b.os_build ?? null,
    profile_id: b.profile_id ?? null,
  });
  return json(200, { id });
}
