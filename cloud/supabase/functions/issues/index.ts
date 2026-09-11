import { authGetUser, restFromEnv } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      save: async (row) => {
        const res = await rest.fetch(`${rest.url}/rest/v1/issues`, {
          method: "POST",
          headers: {
            apikey: rest.serviceKey,
            authorization: `Bearer ${rest.serviceKey}`,
            "content-type": "application/json",
            prefer: "return=representation",
          },
          body: JSON.stringify([row]),
        });
        if (!res.ok) {
          console.error(`insert issue: ${res.status}`);
          throw fail(502, "the report could not be stored");
        }
        const rows = await res.json() as { id: string }[];
        return rows[0].id;
      },
    });
  } catch (e) {
    return asResponse(e);
  }
});
