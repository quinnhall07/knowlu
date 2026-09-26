import { asResponse } from "../_shared/http.ts";
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { readNotes, readRecords } from "../_shared/sync_db.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      requireEntitled: requireActiveEntitlement,
      readRecords: (accountId, after, limit, now) => readRecords(rest, accountId, after, limit, now),
      readNotes: (accountId, after, limit, now) => readNotes(rest, accountId, after, limit, now),
      now: () => new Date(),
    });
  } catch (e) {
    return asResponse(e);
  }
});
