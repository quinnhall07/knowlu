import { sharedDb } from "../_shared/judge_deps.ts";
import { revokeGoogleToken } from "../_shared/google_revoke.ts";
import { exchangeCode } from "./exchange.ts";
import { callbackHandler } from "./handler.ts";
import { storeGrantRevokingStale } from "./reconnect.ts";

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
  // F-5: revokes a DIFFERENT Google account's stale token before this one overwrites its row —
  // `store_google_grant`'s own upsert has no way to see that the `sub` changed, only that a row
  // exists (`reconnect.ts` has the full reasoning).
  storeRefreshToken(accountId, sub, email, refreshToken, scopes) {
    return storeGrantRevokingStale(accountId, sub, email, refreshToken, scopes, {
      async currentGrant(id) {
        const rows = await sharedDb().select(
          `google_accounts?account_id=eq.${id}&select=google_sub`,
        ) as Array<{ google_sub: string }>;
        if (rows.length === 0) return null;
        const token = await sharedDb().rpc("read_google_grant_any", { p_account: id });
        return typeof token === "string" && token !== "" ? { sub: rows[0].google_sub, token } : null;
      },
      revoke: (token) => revokeGoogleToken(fetch, token),
      async store(id, s, e, t, sc) {
        const gid = await sharedDb().rpc("store_google_grant", {
          p_account: id, p_sub: s, p_email: e ?? null, p_token: t, p_scopes: sc,
        });
        if (typeof gid !== "string") throw new Error("the grant was not stored");
      },
    });
  },
}));
