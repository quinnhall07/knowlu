import { sharedDb } from "../_shared/judge_deps.ts";
import { callbackHandler } from "./handler.ts";

const TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";

Deno.serve(callbackHandler({
  clientId: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
  clientSecret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
  redirectUri: `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`,
  async takeState(state) {
    const got = await sharedDb().rpc("take_google_state", { p_nonce: state });
    return typeof got === "string" && got !== "" ? got : null;
  },
  async exchange(code) {
    const response = await fetch(TOKEN_ENDPOINT, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        code,
        client_id: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
        client_secret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
        redirect_uri: `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`,
        grant_type: "authorization_code",
      }),
    });
    if (!response.ok) throw new Error(`token exchange ${response.status}`);
    const body = await response.json() as {
      refresh_token?: string;
      access_token: string;
      id_token: string;
      scope?: string;
    };
    if (body.refresh_token === undefined) throw new Error("no refresh_token in the exchange");
    // `sub` and `email` out of the id_token's payload. Not verified cryptographically here: the
    // token came from a TLS connection to Google's own endpoint in response to our own code, which
    // is the same trust the access token itself rests on.
    const claims = JSON.parse(atob(body.id_token.split(".")[1].replace(/-/g, "+").replace(/_/g, "/"))) as {
      sub: string;
      email?: string;
    };
    // `scope` is space-separated and is what was ACTUALLY granted — a student can untick one on
    // the consent screen, and on an incremental ask Google returns only the new one.
    return {
      refresh_token: body.refresh_token,
      access_token: body.access_token,
      sub: claims.sub,
      email: claims.email,
      scopes: (body.scope ?? "").split(" ").filter((s) => s !== ""),
    };
  },
  async storeRefreshToken(accountId, sub, email, refreshToken, scopes) {
    const id = await sharedDb().rpc("store_google_grant", {
      p_account: accountId, p_sub: sub, p_email: email ?? null, p_token: refreshToken, p_scopes: scopes,
    });
    if (typeof id !== "string") throw new Error("the grant was not stored");
  },
}));
