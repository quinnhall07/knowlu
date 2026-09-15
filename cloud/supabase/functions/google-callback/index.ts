import { sharedDb } from "../_shared/judge_deps.ts";
import { exchangeCode } from "./exchange.ts";
import { callbackHandler } from "./handler.ts";

// The wiring only. Every line of parsing lives in `./exchange.ts`, where `exchange_test.ts` drives
// it with a fake `fetch` — nothing in this file can be reached by a test, and the review found the
// one bug that hides in exactly that gap (C2 final review C-2).
const clientId = () => Deno.env.get("GOOGLE_CLIENT_ID") ?? "";
const clientSecret = () => Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "";
const redirectUri = () => `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`;

Deno.serve(callbackHandler({
  clientId: clientId(),
  clientSecret: clientSecret(),
  redirectUri: redirectUri(),
  async takeState(state) {
    const got = await sharedDb().rpc("take_google_state", { p_nonce: state });
    return typeof got === "string" && got !== "" ? got : null;
  },
  exchange(code) {
    return exchangeCode(code, {
      clientId: clientId(),
      clientSecret: clientSecret(),
      redirectUri: redirectUri(),
    });
  },
  async storeRefreshToken(accountId, sub, email, refreshToken, scopes) {
    const id = await sharedDb().rpc("store_google_grant", {
      p_account: accountId, p_sub: sub, p_email: email ?? null, p_token: refreshToken, p_scopes: scopes,
    });
    if (typeof id !== "string") throw new Error("the grant was not stored");
  },
}));
