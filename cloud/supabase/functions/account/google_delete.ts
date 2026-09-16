// C2 final review F-5 — a deleted account must not leave a live Google grant behind.
//
// `accounts` cascades `google_accounts` at the database level (the foreign key says so), so the
// ROW always goes when the account does. What a cascade cannot reach is the grant Google itself is
// still holding: the Vault-held refresh token and the mailbox/calendar access it stands for. Left
// alone, that refresh token stays valid at Google forever — the one place C2 kept the account's
// consent alive after every trace of the account on our side is gone.
//
// This is C1's file (`account/index.ts`) and C1's deletion flow (`purge`), but the grant lifecycle
// is C2's (hand-off H9 says the boundary): a Google grant is created, read and destroyed only by
// C2's own RPCs, and this hook is what makes account deletion one more place that destroys it
// correctly — never by reaching into `google_accounts` directly.
//
// **The account's right to delete wins.** A revoke that fails (Google is down, the token is
// already dead in a way `disconnectGrant`'s own 200/400 check does not recognise, a network
// timeout) must never turn a deletion into a support ticket — the row and the Vault secret are
// removed either way, and only the CLASS of what went wrong is logged, never the token.
import { revokeGoogleToken } from "../_shared/google_revoke.ts";

export interface GoogleDeleteDeps {
  /** The account's refresh token, whatever scopes it carries, or `null` when there is no grant. */
  grant(accountId: string): Promise<string | null>;
  /** POSTs the revoke to Google. Injected so a test never reaches the network. */
  revoke(token: string): Promise<Response>;
  /** Deletes the Vault secret and the `google_accounts` row. Called even when there was no token
   *  to revoke, so a row left behind by some earlier partial failure is still cleaned up. */
  forget(accountId: string): Promise<void>;
}

/**
 * Revoke, then forget — the same order `disconnectGrant` uses, for the same reason (a row deleted
 * before a successful revoke is a live grant nobody can find any more) — except that here NOTHING
 * this function does may ever stop the caller's own deletion from proceeding: a failure reading
 * the grant, revoking it, or even forgetting it is caught and logged by class only, and `forget`
 * is attempted regardless of whether the steps before it succeeded. This function never throws.
 */
export async function revokeGoogleGrantOnDelete(accountId: string, deps: GoogleDeleteDeps): Promise<void> {
  try {
    const token = await deps.grant(accountId);
    if (token !== null) {
      try {
        const response = await deps.revoke(token);
        if (response.status !== 200 && response.status !== 400) {
          console.error(`account delete: google revoke HTTP ${response.status}`);
        }
      } catch (e) {
        console.error(`account delete: google revoke ${e instanceof Error ? e.constructor.name : "unknown"}`);
      }
    }
  } catch (e) {
    console.error(`account delete: google grant lookup ${e instanceof Error ? e.constructor.name : "unknown"}`);
  }
  try {
    await deps.forget(accountId);
  } catch (e) {
    console.error(`account delete: google forget ${e instanceof Error ? e.constructor.name : "unknown"}`);
  }
}

/** Production wiring: `fetch` and the two RPCs, nothing else. */
export function liveGoogleDeleteDeps(
  db: { rpc(fn: string, args: Record<string, unknown>): Promise<unknown> },
): GoogleDeleteDeps {
  return {
    async grant(accountId) {
      const token = await db.rpc("read_google_grant_any", { p_account: accountId });
      return typeof token === "string" && token !== "" ? token : null;
    },
    revoke: (token) => revokeGoogleToken(fetch, token),
    async forget(accountId) {
      await db.rpc("delete_google_grant", { p_account: accountId });
    },
  };
}
