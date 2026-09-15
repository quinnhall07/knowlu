// GET /google-connect?scope=calendar|gmail -> the Google consent URL.
// DELETE /google-connect                    -> revoke and forget.
//
// **One Google connect, two scopes, in a deliberate order** (cloud design §11a). The wizard's
// first connection step is *Connect your calendars*, so the default and first ask is
// `calendar.readonly` **alone** — a *sensitive* scope, which means a lighter verification review
// and **no CASA security assessment**. `gmail.readonly` is *restricted*, is asked for later and
// only if the student takes the Gmail step, and is added **incrementally**
// (`include_granted_scopes=true`) so the second consent widens the same grant instead of replacing
// it. Calendar *write* is a third ask, later still, only when the student approves their first
// calendar-event card (VISION: rare writes behind explicit approval).
//
// While the Google project is in Testing (§9), and **for both scopes equally**: at most 100 test
// users, a tester warning screen, and refresh tokens that expire after 7 days. The wizard says
// exactly that on both panels — the wizard is C1's; this is the endpoint behind it, and its copy
// is hand-off H9.
import type { Entitle } from "../_shared/judge_handler.ts";
// R-C2-E35: one home for the two scope literals — re-exported here so `handler_test.ts` and every
// existing caller of `./handler.ts` keep importing `CALENDAR_SCOPE` / `GMAIL_SCOPE` from this file.
export { CALENDAR_SCOPE, GMAIL_SCOPE, GOOGLE_NOT_CONFIGURED } from "../_shared/google_scopes.ts";
import { CALENDAR_SCOPE, GMAIL_SCOPE, GOOGLE_NOT_CONFIGURED } from "../_shared/google_scopes.ts";

export const AUTH_ENDPOINT = "https://accounts.google.com/o/oauth2/v2/auth";

/**
 * The identity scopes every consent asks for ALONGSIDE its one API scope (C2 final review C-2).
 *
 * Google returns an `id_token` — the only cheap way to learn WHICH Google account just consented —
 * **only** when `openid` was asked for. C2 asked for one API scope and nothing else, so no
 * `id_token` ever came back and `google-callback` dereferenced a field that was never there.
 * `email` rides along because `google_accounts.email` is what the settings row shows the student
 * ("connected as …"), and because it costs no extra review: `openid` and `email` are both
 * NON-SENSITIVE scopes — they do not touch §11a's sensitive/restricted staging at all.
 */
export const IDENTITY_SCOPES = ["openid", "email"];

/** `?scope=` -> the scope string this consent asks for — the identity scopes plus exactly ONE
 * Google API scope — and whether it widens an existing grant. */
export function scopeFor(name: string | null): { scope: string; incremental: boolean } | null {
  const ask = (apiScope: string, incremental: boolean) => ({
    scope: [...IDENTITY_SCOPES, apiScope].join(" "),
    incremental,
  });
  if (name === null || name === "" || name === "calendar") return ask(CALENDAR_SCOPE, false);
  if (name === "gmail") return ask(GMAIL_SCOPE, true);
  return null;
}

export interface ConnectDeps {
  clientId: string;
  redirectUri: string;
  /** Persists a single-use nonce bound to this account and returns it. */
  saveState(accountId: string): Promise<string>;
  /** Revokes the refresh token at Google, then deletes the Vault secret and the row — in that order. */
  disconnect(accountId: string): Promise<void>;
  /** What Google actually granted, from `google_accounts.scopes`. `[]` when there is no grant. */
  grantedScopes(accountId: string): Promise<string[]>;
}

export function connectHandler(entitle: Entitle, deps: ConnectDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      const { account_id } = await entitle(req);
      if (deps.clientId === "") {
        // C2 final review S-4: the ONE copy, in `_shared/google_scopes.ts`. The device matches it
        // exactly to tell an unconfigured deployment from an ordinary 503.
        return Response.json({ error: GOOGLE_NOT_CONFIGURED }, { status: 503 });
      }
      if (req.method === "DELETE") {
        try {
          await deps.disconnect(account_id);
        } catch (e) {
          // R-C2-E33 / R-C2-E34: `deps.disconnect` (`disconnect.ts`'s `disconnectGrant`) forgets
          // the row only once the revoke has succeeded or was already moot — a genuinely failed
          // revoke throws and the row STAYS, on purpose, so the student can retry rather than lose
          // the only copy of the token. Answering `{disconnected: true}` here would be a lie the
          // very next `?status=1` poll exposes (`connected: true`, the grant never having moved),
          // so this is a 502 instead: the grant is still live at Google and still stored, and the
          // failure is this log line's class only, never the body.
          console.error(`google-connect: revoke failed (${e instanceof Error ? e.constructor.name : "unknown"})`);
          return Response.json({ error: "Google could not be reached to disconnect; try again" }, { status: 502 });
        }
        return Response.json({ disconnected: true });
      }
      if (req.method !== "GET") return Response.json({ error: "GET or DELETE" }, { status: 405 });
      // `?status=1` — the only question the WIZARD can ask, because the consent window closes
      // itself and nothing else tells the device the round trip finished (hand-off H9's poll).
      // Deliberately reports the scopes rather than a bare boolean: a student can untick one on
      // the consent screen, and "connected" without "which" would be a wizard that says the
      // calendar is on when only Gmail is.
      if (new URL(req.url).searchParams.get("status") !== null) {
        const scopes = await deps.grantedScopes(account_id);
        return Response.json({ connected: scopes.length > 0, scopes });
      }
      const asked = scopeFor(new URL(req.url).searchParams.get("scope"));
      if (asked === null) {
        return Response.json({ error: "scope must be 'calendar' or 'gmail'" }, { status: 400 });
      }
      const url = new URL(AUTH_ENDPOINT);
      url.searchParams.set("client_id", deps.clientId);
      url.searchParams.set("redirect_uri", deps.redirectUri);
      url.searchParams.set("response_type", "code");
      url.searchParams.set("scope", asked.scope);
      url.searchParams.set("access_type", "offline");
      // `consent` every time: without it Google returns no refresh token on a re-connect, and a
      // re-connect is the normal case while the project is in Testing and tokens die weekly.
      url.searchParams.set("prompt", "consent");
      // Incremental only for the second ask: the Gmail consent must WIDEN the calendar grant, not
      // replace it — a student who connected the calendar in the wizard must not lose it by
      // connecting Gmail a week later.
      url.searchParams.set("include_granted_scopes", asked.incremental ? "true" : "false");
      url.searchParams.set("state", await deps.saveState(account_id));
      return Response.json({ url: url.toString() });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`google-connect: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "connect failed" }, { status: 500 });
    }
  };
}
