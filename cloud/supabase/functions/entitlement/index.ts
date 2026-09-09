import { authGetUser, restFromEnv } from "../_shared/db.ts";
import { lookupFrom } from "../_shared/entitlement.ts";
import { asResponse } from "../_shared/http.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      lookup: lookupFrom(rest),
      now: () => new Date(),
    });
  } catch (e) {
    return asResponse(e);
  }
});
