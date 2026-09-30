// GET /google-connect?scope=calendar|gmail|reconnect -> the Google consent URL.
// GET /google-connect?status=1                       -> {connected, scopes, status, email}.
// DELETE /google-connect                             -> revoke and forget.
//
// **One Google connect, two scopes, in a deliberate order** (cloud design §11a). The wizard's
// first connection step is *Connect your calendars*, so the default and first ask is
// `calendar.readonly` **alone** — a *sensitive* scope, which means a lighter verification review
// and **no CASA security assessment**. `gmail.readonly` is *restricted*, is asked for later and
// only if the student takes the Gmail step, and is added **incrementally**
// (`include_granted_scopes=true`) so the second consent widens the same grant instead of replacing
// it — and since the P3 live pass of 2026-09-17 that flag is on EVERY ask, because a calendar
// re-consent sent without it drops Gmail from the new token (the comment at the flag says how).
// Calendar *write* is a third ask, later still, only when the student approves their first
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

/** The Google API scopes a reconnect may re-ask for, in the order it asks. Anything else a row
 * records is never forwarded to Google. */
const RECONNECTABLE = [CALENDAR_SCOPE, GMAIL_SCOPE];

/** `?scope=` -> the scope string this consent asks for — the identity scopes plus ONE Google API
 * scope for `calendar` or `gmail`. Every ask widens whatever is already granted
 * (`include_granted_scopes=true` below), so nothing here says which.
 *
 * `reconnect` (Gmail connect D8) is the one ask that names more than one API scope: it re-asks for
 * every API scope `recorded` on the account's row, in one consent, so the new refresh token can
 * never cover fewer scopes than the row claims (the P3 live defect). It widens nothing the student
 * had not already granted. With nothing recorded it answers `{ nothing: true }` (a 400). */
export function scopeFor(
  name: string | null,
  recorded: readonly string[] = [],
): { scope: string } | { nothing: true } | null {
  const ask = (apiScopes: string[]) => ({ scope: [...IDENTITY_SCOPES, ...apiScopes].join(" ") });
  if (name === null || name === "" || name === "calendar") return ask([CALENDAR_SCOPE]);
  if (name === "gmail") return ask([GMAIL_SCOPE]);
  if (name === "reconnect") {
    const again = RECONNECTABLE.filter((s) => recorded.includes(s));
    return again.length === 0 ? { nothing: true } : ask(again);
  }
  return null;
}

/** The account's `google_accounts` row, read with NO status filter (spec §4.1). */
export interface GoogleGrant {
  scopes: string[];
  status: "active" | "revoked" | "quiet";
  /** `google_accounts.email_hint`: the student's own Google address, shown back to them (D5). */
  email: string | null;
}

/** `?status=1`'s answer (D5). `connected` and `scopes` keep their pre-D5 meaning — a revoked row
 * reads as no scopes, exactly what the wizard saw when that row was filtered out — and `status`
 * adds `"none"` for no row, so the Settings row can tell "never connected" from "reconnect". */
export function statusFrom(row: GoogleGrant | null) {
  if (row === null) return { connected: false, scopes: [] as string[], status: "none", email: null };
  const scopes = row.status === "revoked" ? [] : row.scopes;
  return { connected: scopes.length > 0, scopes, status: row.status, email: row.email ?? null };
}

export interface ConnectDeps {
  clientId: string;
  redirectUri: string;
  /** Persists a single-use nonce bound to this account and returns it. */
  saveState(accountId: string): Promise<string>;
  /** Revokes the refresh token at Google, then deletes the Vault secret and the row — in that order. */
  disconnect(accountId: string): Promise<void>;
  /** The account's `google_accounts` row, whatever its status; `null` when there is none. One read
   * serves `?status=1`'s `status` and `email` and `?scope=reconnect`'s scopes. */
  grant(accountId: string): Promise<GoogleGrant | null>;
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
        return Response.json(statusFrom(await deps.grant(account_id)));
      }
      const name = new URL(req.url).searchParams.get("scope");
      // Only `reconnect` reads the row; the calendar and Gmail asks stay a row-free consent.
      const recorded = name === "reconnect" ? (await deps.grant(account_id))?.scopes ?? [] : [];
      const asked = scopeFor(name, recorded);
      if (asked === null) {
        return Response.json({ error: "scope must be 'calendar', 'gmail' or 'reconnect'" }, { status: 400 });
      }
      if ("nothing" in asked) {
        // Checked before `saveState`: a refused reconnect mints no nonce.
        return Response.json({ error: "nothing to reconnect" }, { status: 400 });
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
      // Incremental on EVERY ask, not only the Gmail one. The Gmail consent must WIDEN the
      // calendar grant rather than replace it — and the calendar re-consent (the one Testing mode
      // forces every seven days, and the one the settings row will offer) must keep Gmail just the
      // same. Google issues the new refresh token for the scopes of THIS request alone unless this
      // flag is set; the P3 live pass (2026-09-17) proved what `false` did here: a calendar-only
      // token under a `google_accounts.scopes` row that still claimed Gmail, `/gmail-read` a 403
      // from Gmail and a 500 to the device, on every slot. On a first consent nothing was granted
      // before, so `true` changes nothing there.
      url.searchParams.set("include_granted_scopes", "true");
      url.searchParams.set("state", await deps.saveState(account_id));
      return Response.json({ url: url.toString() });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`google-connect: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "connect failed" }, { status: 500 });
    }
  };
}
