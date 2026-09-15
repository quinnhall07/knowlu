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

function page(message: string, status = 200): Response {
  return new Response(
    `<!doctype html><meta charset="utf-8"><title>Knowlu</title>` +
      `<body style="font:16px system-ui;padding:3rem"><p>${message}</p></body>`,
    { status, headers: { "content-type": "text/html; charset=utf-8" } },
  );
}

// R-C2-E31: every page said "Gmail", but the FIRST ask this pair ever makes is the calendar — a
// student connecting only their calendar must not read a success page that names a mailbox they
// were never asked about. The success message names what `tokens.scopes` actually carries.
const CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.readonly";
const GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly";

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
  return `${what} connected. You can <b>close this window</b> — Knowlu will read it at your next slot.`;
}

export function callbackHandler(deps: CallbackDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    const url = new URL(req.url);
    const state = url.searchParams.get("state") ?? "";
    const code = url.searchParams.get("code") ?? "";
    if (url.searchParams.get("error") !== null) {
      return page("Google was <b>not connected</b>. You can close this window and try again from Knowlu.");
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
