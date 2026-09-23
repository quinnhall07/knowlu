import { authGetUser, restFromEnv, restSelect, restUpsert } from "../_shared/db.ts";
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
      // F7 decision 6: one select per batch. The ids are already UUID-checked by the handler, so
      // nothing but hex and dashes reaches the `in.(...)` list; a failed select throws, and the batch
      // is a 5xx that saved nothing.
      ownedJudgments: async (accountId, ids) => {
        const rows = await restSelect<{ id: string }>(
          rest,
          "judgments",
          `select=id&account_id=eq.${encodeURIComponent(accountId)}&id=in.(${ids.join(",")})`,
        );
        return new Set(rows.map((r) => r.id.toLowerCase()));
      },
    });
  } catch (e) {
    return asResponse(e);
  }
});
