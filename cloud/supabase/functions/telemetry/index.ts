import { authGetUser, restFromEnv, restUpsert } from "../_shared/db.ts";
import { asResponse } from "../_shared/http.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      // A resent batch lands on the rows it landed on the first time: the unique constraints are
      // the dedup, and `merge-duplicates` is what makes a retry free.
      saveEvents: (rows) =>
        restUpsert(rest, "telemetry_events", rows, "account_id,session,ts,action,object_id"),
      saveCorrections: (rows) => restUpsert(rest, "corrections", rows, "account_id,ts,item_id,field"),
    });
  } catch (e) {
    return asResponse(e);
  }
});
