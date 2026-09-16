// The one place a refresh token is POSTed to Google's own revoke endpoint. Google revokes EVERY
// scope on a token at once — there is no partial revoke — so every caller that needs to kill a
// live grant shares this: the settings panel's disconnect (`google-connect/index.ts`, through
// `disconnect.ts`'s `disconnectGrant`), an account deletion (C2 final review F-5,
// `account/index.ts`) and a reconnect under a different Google identity (F-5,
// `google-callback/reconnect.ts`). None of them hold the token any longer than the POST needs, and
// none of them log the token itself — only ever a response STATUS, which is not a credential.
export const GOOGLE_REVOKE_ENDPOINT = "https://oauth2.googleapis.com/revoke";

/**
 * POSTs the revoke request to Google and resolves with its response — never rejects on a non-2xx
 * status, the way `fetch` itself does not. `200` is a normal revoke; Google's own `invalid_token`
 * (`400`) means the token was already dead (a double disconnect, or one that raced an expiry) and
 * counts as done rather than as a failure — every caller here treats the two alike.
 */
export function revokeGoogleToken(fetchImpl: typeof fetch, token: string): Promise<Response> {
  return fetchImpl(GOOGLE_REVOKE_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({ token }),
  });
}
