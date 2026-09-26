import { asResponse } from "../_shared/http.ts";
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { bytesUsed, ceiling, saveNotes, saveRecords } from "../_shared/sync_db.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      requireEntitled: requireActiveEntitlement,
      bytesUsed: (accountId) => bytesUsed(rest, accountId),
      ceiling: () => ceiling(rest),
      saveRecords: (rows) => saveRecords(rest, rows),
      saveNotes: (rows) => saveNotes(rest, rows),
    });
  } catch (e) {
    // C1's shape: a thrown `Response` — `fail`'s, or `requireActiveEntitlement`'s — goes back
    // verbatim, and anything else becomes a bare 500 with no body of ours in it.
    return asResponse(e);
  }
});
