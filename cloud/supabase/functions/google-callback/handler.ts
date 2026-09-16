// GET /google-callback?code=…&state=… — the one place a Google refresh token exists in our code.
//
// It goes straight into Supabase Vault and the row keeps only the secret's id. It is never
// returned, never logged, and never sent to the device (D12): the device's whole knowledge of the
// connection is "connected: true".
//
// This is the one C2 function with no bearer token at all — Google redirects a browser here — so
// it is authenticated by the single-use `state` nonce, which `take_google_state` consumes and
// expires in one statement. `config.toml` therefore sets `verify_jwt = false` here for a different
// reason than everywhere else, and hand-off H7 says so.
import { CALENDAR_SCOPE, GMAIL_SCOPE } from "../_shared/google_scopes.ts";

export interface CallbackDeps {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  /** Consumes the nonce and returns the account it was issued for, or null. Single use. */
  takeState(state: string): Promise<string | null>;
  exchange(code: string): Promise<{ refresh_token: string; access_token: string; sub: string; email?: string; scopes: string[] }>;
  /** `scopes` is what Google ACTUALLY granted, from the exchange's own `scope` field — never what
   *  was asked for. `/ingest-calendar` and `gmail-read` both check it before they read anything. */
  storeRefreshToken(
    accountId: string, sub: string, email: string | undefined, refreshToken: string, scopes: string[],
  ): Promise<void>;
}

// R2-8 / R-C2-E56 (regrading Task 10's F-6): this used to render `message` as an HTML page, but
// Supabase's functions relay rewrites that response before it ever reaches the browser — observed
// live on staging 2026-09-16, the wire response for this function arrived as `content-type:
// text/plain` carrying a `content-security-policy: default-src 'none'; sandbox` this code never
// set, with the markup below passed through as literal text in the body. Every consent page
// (success, declined, expired, error) was showing raw HTML source to the student, on every call,
// since Task 10 — no test caught it because the tests call this handler directly, never the relay
// in front of it. So this is not HTML at all: the body is the sentence alone, nothing else;
// `content-type: text/plain; charset=utf-8` matches what a browser actually receives; `nosniff`
// still applies; and the CSP tightens to `default-src 'none'` — there is no inline style left to
// allow. A real page of our own is C4's, once the site is public and a redirect can land there
// instead of on the shared functions domain.
function page(message: string, status = 200): Response {
  return new Response(message, {
    status,
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "x-content-type-options": "nosniff",
      "content-security-policy": "default-src 'none'",
    },
  });
}

// R-C2-E31: every page said "Gmail", but the FIRST ask this pair ever makes is the calendar — a
// student connecting only their calendar must not read a success page that names a mailbox they
// were never asked about. The success message names what `tokens.scopes` actually carries.
function successMessage(scopes: string[]): string {
  const calendar = scopes.includes(CALENDAR_SCOPE);
  const gmail = scopes.includes(GMAIL_SCOPE);
  const what = calendar && gmail
    ? "Google Calendar and Gmail are"
    : gmail
    ? "Gmail is"
    : calendar
    ? "Google Calendar is"
    : "Google is";
  return `${what} connected. You can close this window — Knowlu will read it at your next slot.`;
}

export function callbackHandler(deps: CallbackDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    const url = new URL(req.url);
    const state = url.searchParams.get("state") ?? "";
    const code = url.searchParams.get("code") ?? "";
    if (url.searchParams.get("error") !== null) {
      return page("Google was not connected. You can close this window and try again from Knowlu.");
    }
    const accountId = state === "" ? null : await deps.takeState(state);
    if (accountId === null || code === "") {
      return page("That link has expired. Start again from Knowlu.", 400);
    }
    let tokens: Awaited<ReturnType<CallbackDeps["exchange"]>>;
    try {
      tokens = await deps.exchange(code);
      await deps.storeRefreshToken(accountId, tokens.sub, tokens.email, tokens.refresh_token, tokens.scopes);
    } catch (e) {
      // The class only. A token-exchange error body can echo the code and, on some failures, the
      // client secret's prefix.
      console.error(`google-callback: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return page("Google could not be connected just now. You can close this window and try again from Knowlu.");
    }
    return page(successMessage(tokens.scopes));
  };
}
