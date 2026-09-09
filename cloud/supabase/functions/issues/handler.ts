import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { scrub, scrubJson } from "../_shared/scrub.ts";

const MAX_BODY = 8192;
// Fix round 1, item 5: a version string, a git sha, a Windows build number, a profile id — none of
// them need more than this to say what they are. Untyped, uncapped free text in these columns is
// exactly the kind of stray fact the scrub function exists to keep out of the store; better to
// refuse a malformed one than to store it unexamined.
const META_SHAPE = /^[A-Za-z0-9_.:+-]{1,64}$/;

export interface Deps {
  verify: VerifyToken;
  save: (row: unknown) => Promise<string>;
}

/** `undefined`/`null` stays absent; anything else must be a short token-shaped string. */
function metaField(v: unknown, name: string): string | null {
  if (v === undefined || v === null) return null;
  if (typeof v === "string" && META_SHAPE.test(v)) return v;
  throw fail(400, `${name} does not look like a version or build id`);
}

/** Fix round 1, item 6: `payload` must be a plain object when present — not an array, not a scalar. */
function checkPayload(v: unknown): Record<string, unknown> {
  if (v === undefined || v === null) return {};
  if (typeof v !== "object" || Array.isArray(v)) throw fail(400, "payload must be an object");
  return v as Record<string, unknown>;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const b = await readJson<{
    body?: string;
    payload?: unknown;
    app_version?: unknown;
    engine_build?: unknown;
    os_build?: unknown;
    profile_id?: unknown;
  }>(req, 1 << 18);
  const text = String(b.body ?? "").trim();
  if (!text) throw fail(400, "an issue report needs a sentence about what went wrong");
  if (text.length > MAX_BODY) throw fail(400, `the report is longer than ${MAX_BODY} characters`);

  const id = await deps.save({
    account_id: user.id,
    body: scrub(text),
    payload: scrubJson(checkPayload(b.payload)),
    app_version: metaField(b.app_version, "app_version"),
    engine_build: metaField(b.engine_build, "engine_build"),
    os_build: metaField(b.os_build, "os_build"),
    profile_id: metaField(b.profile_id, "profile_id"),
  });
  return json(200, { id });
}
