/**
 * The wire for `mail-pull` (email plan T4.4): the store over PostgREST (`store.ts`), then `handler.ts`.
 * `verify_jwt = false` (config.toml): the handler authenticates every request itself, a session plus an
 * active entitlement (§6.3). No secret of its own: the service role through `restFromEnv`.
 */
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse } from "../_shared/http.ts";
import { pullHandler } from "./handler.ts";
import { storeOver } from "./store.ts";

let handler: ((req: Request) => Promise<Response>) | null = null;

Deno.serve(async (req) => {
  try {
    handler ??= pullHandler({
      entitle: requireActiveEntitlement,
      store: storeOver(restFromEnv()),
      now: () => Date.now(),
    });
    return await handler(req);
  } catch (e) {
    return asResponse(e);
  }
});
