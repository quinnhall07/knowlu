import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { connectHandler } from "./handler.ts";
import { disconnectGrant } from "./disconnect.ts";

const REVOKE = "https://oauth2.googleapis.com/revoke";

Deno.serve(connectHandler(requireActiveEntitlement, {
  clientId: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
  redirectUri: `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`,
  async saveState(accountId) {
    const nonce = crypto.randomUUID();
    await sharedDb().insert("google_state", { nonce, account_id: accountId }, false);
    return nonce;
  },
  async grantedScopes(accountId) {
    const rows = await sharedDb().select(
      `google_accounts?account_id=eq.${accountId}&status=neq.revoked&select=scopes`,
    ) as Array<{ scopes: string[] }>;
    return rows[0]?.scopes ?? [];
  },
  async disconnect(accountId) {
    const db = sharedDb();
    const secret = await db.rpc("read_google_grant_any", { p_account: accountId });
    await disconnectGrant({
      token: typeof secret === "string" && secret !== "" ? secret : null,
      revoke: (token) =>
        fetch(REVOKE, {
          method: "POST",
          headers: { "Content-Type": "application/x-www-form-urlencoded" },
          body: new URLSearchParams({ token }),
        }),
      forget: () => db.rpc("delete_google_grant", { p_account: accountId }).then(() => undefined),
    });
  },
}));
