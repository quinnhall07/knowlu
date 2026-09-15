import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { connectHandler } from "./handler.ts";

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

    // Revoke FIRST: a row deleted before the revoke is a live grant nobody can find any more.
    // Revoking the refresh token at Google revokes EVERY scope on it at once, which is what
    // "disconnect" means on this panel: the calendar and the mailbox go together, because they are
    // one grant. A future "disconnect Gmail only" would be a re-consent for the calendar alone,
    // not a partial revoke — Google has no such thing.
    const secret = await db.rpc("read_google_grant_any", { p_account: accountId });
    if (typeof secret === "string" && secret !== "") {
      await fetch(REVOKE, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({ token: secret }),
      });
    }
    await db.rpc("delete_google_grant", { p_account: accountId });
  },
}));
