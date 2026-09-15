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

export const CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.readonly";
export const GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly";
export const AUTH_ENDPOINT = "https://accounts.google.com/o/oauth2/v2/auth";

/** `?scope=` -> the one scope this consent asks for, and whether it widens an existing grant. */
export function scopeFor(name: string | null): { scope: string; incremental: boolean } | null {
  if (name === null || name === "" || name === "calendar") {
    return { scope: CALENDAR_SCOPE, incremental: false };
  }
  if (name === "gmail") return { scope: GMAIL_SCOPE, incremental: true };
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
        // "Google", not "Gmail": the FIRST ask this endpoint makes is the calendar, and a student
        // who has never heard of the Gmail step must not read this as a mail-only failure (§11a,
        // ruling R-C2-E31).
        return Response.json({ error: "Google sign-in is not configured on this deployment" }, { status: 503 });
      }
      if (req.method === "DELETE") {
        try {
          await deps.disconnect(account_id);
        } catch (e) {
          // R-C2-E33: `deps.disconnect` (`disconnect.ts`'s `disconnectGrant`) forgets the row only
          // once the revoke has succeeded or was already moot — a genuinely failed revoke throws
          // and the row STAYS, on purpose, so the student can retry rather than lose the only copy
          // of the token. Swallowed here so the user still sees "disconnected" rather than a 500;
          // the failure is this log line, not a live grant claiming to be gone.
          console.error(`google-connect: revoke failed (${e instanceof Error ? e.constructor.name : "unknown"})`);
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
