/**
 * The wire for `mail-resolve` (email plan T4.5): the store and the charges (`store.ts`), the `mail_resolve`
 * pin, then `handler.ts`. `verify_jwt = false` (config.toml): the handler authenticates every request
 * itself, a session plus an active entitlement (§6.3). Its secrets are the service role (`restFromEnv`,
 * `sharedDb`) and the pinned provider's key, read by `modelFor`.
 */
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse } from "../_shared/http.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { judgmentSink } from "../_shared/judge_log.ts";
import { modelRow } from "../_shared/judge_models.ts";
import { modelFor } from "../_shared/judge_provider.ts";
import { type ResolveDeps, resolveHandler } from "./handler.ts";
import { resolveCharges, storeOver } from "./store.ts";

/** One model call's limit: one course batch, as `mail-process` gives one message. */
const CALL_MS = 30_000;

let handler: ((req: Request) => Promise<Response>) | null = null;

function wire(): (req: Request) => Promise<Response> {
  const db = sharedDb();
  let pinned: ReturnType<ResolveDeps["pin"]> | null = null;
  const pin = () => {
    pinned ??= modelRow(db, "mail_resolve").then((row) => ({
      row,
      model: modelFor(row, (name) => Deno.env.get(name), { timeoutMs: CALL_MS }),
    }));
    pinned.catch(() => (pinned = null));
    return pinned;
  };
  return resolveHandler({
    entitle: requireActiveEntitlement,
    store: storeOver(restFromEnv()),
    charges: resolveCharges(db),
    log: judgmentSink(db),
    pin,
    now: () => Date.now(),
  });
}

Deno.serve(async (req) => {
  try {
    handler ??= wire();
    return await handler(req);
  } catch (e) {
    return asResponse(e);
  }
});
