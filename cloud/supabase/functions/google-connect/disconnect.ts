// The disconnect ordering, isolated (ruling R-C2-E33) so it is unit tested directly rather than
// only through a mock that proves nothing about the real order or the revoke's own response.
//
// Revoking the refresh token at Google revokes EVERY scope on it at once, which is what
// "disconnect" means on this panel: the calendar and the mailbox go together, because they are one
// grant. A future "disconnect Gmail only" would be a re-consent for the calendar alone, not a
// partial revoke — Google has no such thing.
export interface DisconnectDeps {
  /** The refresh token to revoke, or `null` when the account has no grant to disconnect. */
  token: string | null;
  /** POSTs the revoke request to Google and resolves with its response — never rejects on a
   *  non-2xx status, the way `fetch` itself does not. */
  revoke(token: string): Promise<Response>;
  /** Deletes the Vault secret and the row. Reached only once the revoke has succeeded, was already
   *  moot (the token was already dead), or there was never a token to revoke in the first place. */
  forget(): Promise<void>;
}

/**
 * Revoke FIRST, forget second — never the other way round, and never both when the revoke's own
 * answer says it did not happen. A row deleted before a successful revoke is a live grant nobody
 * can find any more; a row deleted after a FAILED revoke destroys the only copy of a token that
 * might still be live, leaving the student with no way to retry.
 *
 * `200` is a normal revoke. Google's own `invalid_token` (`400`) means the token was already dead
 * — a double disconnect, or one that raced an expiry — and there is nothing left to revoke, so this
 * counts as done rather than as a failure. Any other status (a `5xx`, a network hiccup surfaced as
 * a non-OK response) means the revoke did not happen, and `forget` is never called.
 */
export async function disconnectGrant(deps: DisconnectDeps): Promise<void> {
  if (deps.token === null) {
    await deps.forget();
    return;
  }
  const response = await deps.revoke(deps.token);
  if (response.status !== 200 && response.status !== 400) {
    // The status only — Google's revoke error bodies are not something to hold onto here, and this
    // is a log line (`index.ts`'s caller), never the token.
    throw new Error(`google revoke HTTP ${response.status}`);
  }
  await deps.forget();
}
