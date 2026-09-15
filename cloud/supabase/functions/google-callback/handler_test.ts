import { assert, assertEquals } from "@std/assert";
import { callbackHandler } from "./handler.ts";

const TOKENS = {
  refresh_token: "REFRESH-TRIPWIRE-9f2c",
  access_token: "ACCESS-TRIPWIRE-9f2c",
  sub: "1029384756",
  email: "student@example.invalid",
  // R-C2-E23 (F14 + F26): what Google actually granted on this exchange — the calendar scope,
  // asked for alone and first (§11a). A student who declines part of the consent grants a
  // different set, which is exactly why this travels with the exchange rather than being assumed.
  scopes: ["https://www.googleapis.com/auth/calendar.readonly"],
};

function deps(overrides: Record<string, unknown> = {}) {
  return {
    clientId: "client-id-not-a-secret",
    clientSecret: "client-secret-not-a-secret",
    redirectUri: "https://ref.supabase.co/functions/v1/google-callback",
    takeState: (s: string) => Promise.resolve(s === "good" ? "acct-1" : null),
    exchange: () => Promise.resolve(TOKENS),
    storeRefreshToken: () => Promise.resolve(),
    ...overrides,
  };
}

function get(query: string): Request {
  return new Request(`http://127.0.0.1/google-callback?${query}`);
}

Deno.test("a good code stores the refresh token and says the window may be closed", async () => {
  let stored: unknown = null;
  const handler = callbackHandler(deps({
    storeRefreshToken: (accountId: string, sub: string, email: string | undefined, token: string) => {
      stored = { accountId, sub, email, token };
      return Promise.resolve();
    },
  }));
  const response = await handler(get("state=good&code=abc"));
  assertEquals(response.status, 200);
  const page = await response.text();
  assert(page.includes("close this window"));
  // The token reaches the store and NOTHING else — not the page, not a header.
  assertEquals((stored as { token: string }).token, TOKENS.refresh_token);
  assertEquals(page.includes("TRIPWIRE-9f2c"), false);
});

Deno.test("the state nonce is required and single use", async () => {
  const handler = callbackHandler(deps());
  assertEquals((await handler(get("code=abc"))).status, 400);
  assertEquals((await handler(get("state=replayed&code=abc"))).status, 400);
  assertEquals((await handler(get("state=good"))).status, 400, "a state with no code is not a connect");
});

Deno.test("a Google error is a page, not a stack trace, and names no token", async () => {
  const handler = callbackHandler(deps({ exchange: () => Promise.reject(new Error("invalid_grant REFRESH-TRIPWIRE-9f2c")) }));
  const response = await handler(get("state=good&code=abc"));
  const page = await response.text();
  assert(page.includes("try again"));
  assertEquals(page.includes("TRIPWIRE-9f2c"), false);
});

Deno.test("a user who declined at Google gets a page and no exchange is attempted", async () => {
  let exchanged = false;
  const handler = callbackHandler(deps({
    exchange: () => {
      exchanged = true;
      return Promise.resolve(TOKENS);
    },
  }));
  const response = await handler(get("state=good&error=access_denied"));
  assertEquals(response.status, 200);
  assert((await response.text()).includes("not connected"));
  assertEquals(exchanged, false);
});

Deno.test("the granted scopes reach storeRefreshToken, not just the token", async () => {
  // R-C2-E23 (F14 + F26): every reader checks `google_accounts.scopes` (`/ingest-calendar`,
  // `gmail-read`), so a callback that dropped this on the floor would leave every grant looking
  // scope-less no matter what the student actually approved.
  let sawScopes: string[] | null = null;
  const handler = callbackHandler(deps({
    storeRefreshToken: (_accountId: string, _sub: string, _email: string | undefined, _token: string, scopes: string[]) => {
      sawScopes = scopes;
      return Promise.resolve();
    },
  }));
  const response = await handler(get("state=good&code=abc"));
  assertEquals(response.status, 200);
  assertEquals(sawScopes, TOKENS.scopes);
});
