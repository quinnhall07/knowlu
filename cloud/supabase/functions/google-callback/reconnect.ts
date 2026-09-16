// C2 final review F-5 — a reconnect under a DIFFERENT Google identity must revoke the stale token.
//
// `store_google_grant`'s own SQL (`20260911000200_google.sql`) upserts keyed on `account_id`
// alone: it overwrites `google_sub` and `secret_id` unconditionally, because the common case is the
// same person re-consenting (a testing-mode 7-day expiry, or the later incremental Gmail step) —
// which is also why it UNIONS `scopes` rather than replacing them. What one SQL statement cannot
// see is whether the NEW `sub` even names the same Google account as the one already stored. If it
// does not, the OLD refresh token is simply abandoned the moment the row is overwritten: never
// revoked, still valid at Google, with nothing in this database pointing at it any more.
//
// This runs BEFORE `store_google_grant`, from `google-callback/index.ts`'s `storeRefreshToken`:
// read the account's current grant, and when it names a different `sub`, revoke ITS token first —
// the same rule F-5's other half applies to a deleted account, applied here to a superseded one. A
// failed revoke never blocks the NEW consent from being stored: the student came here to connect an
// account, and that must not fail because Google's revoke endpoint is briefly down.
export interface ReconnectDeps {
  /** The account's currently stored grant — whichever scopes it carries — or `null` when it has
   *  none yet. */
  currentGrant(accountId: string): Promise<{ sub: string; token: string } | null>;
  /** POSTs the revoke to Google. Injected so a test never reaches the network. */
  revoke(token: string): Promise<Response>;
  /** `store_google_grant`, wired. Reached exactly once, after any stale revoke. */
  store(
    accountId: string, sub: string, email: string | undefined, refreshToken: string, scopes: string[],
  ): Promise<void>;
}

export async function storeGrantRevokingStale(
  accountId: string,
  sub: string,
  email: string | undefined,
  refreshToken: string,
  scopes: string[],
  deps: ReconnectDeps,
): Promise<void> {
  const current = await deps.currentGrant(accountId);
  if (current !== null && current.sub !== sub) {
    try {
      const response = await deps.revoke(current.token);
      if (response.status !== 200 && response.status !== 400) {
        console.error(`google-callback: stale revoke HTTP ${response.status}`);
      }
    } catch (e) {
      console.error(`google-callback: stale revoke ${e instanceof Error ? e.constructor.name : "unknown"}`);
    }
  }
  await deps.store(accountId, sub, email, refreshToken, scopes);
}
