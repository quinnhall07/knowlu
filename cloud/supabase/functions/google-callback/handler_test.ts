import { assert, assertEquals } from "@std/assert";
import { callbackHandler } from "./handler.ts";
import { CALENDAR_SCOPE, GMAIL_SCOPE } from "../_shared/google_scopes.ts";

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
  // R-C2-E31: TOKENS.scopes is the calendar scope alone (§11a's first, common ask) — the page
  // must name the calendar, not the mailbox nobody was asked about.
  assert(page.includes("Google Calendar is connected"), page);
  // The token reaches the store and NOTHING else — not the page, not a header.
  assertEquals((stored as { token: string }).token, TOKENS.refresh_token);
  assertEquals(page.includes("TRIPWIRE-9f2c"), false);
});

Deno.test("the success page names exactly what tokens.scopes granted", async () => {
  // R-C2-E31: calendar-only, Gmail-only and both, over the same handler — one test, one fixture
  // matrix, matching the loop-over-fixtures idiom this suite already uses elsewhere in the repo.
  const cases: Array<[string[], string]> = [
    [[CALENDAR_SCOPE], "Google Calendar is connected"],
    [[GMAIL_SCOPE], "Gmail is connected"],
    [[CALENDAR_SCOPE, GMAIL_SCOPE], "Google Calendar and Gmail are"],
  ];
  for (const [scopes, want] of cases) {
    const handler = callbackHandler(deps({ exchange: () => Promise.resolve({ ...TOKENS, scopes }) }));
    const response = await handler(get("state=good&code=abc"));
    const page = await response.text();
    assert(page.includes(want), `scopes ${scopes.join(",")}: expected "${want}" in ${page}`);
  }
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
  // R-C2-E31: "Google", not "Gmail" — the exchange can fail on a calendar-only connect too.
  assert(page.includes("Google could not be connected"), page);
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
  const page = await response.text();
  // R-C2-E31: "Google", not "Gmail" — the FIRST ask is the calendar, and a student declining that
  // consent must not read a page that names a mailbox they were never asked about.
  assert(page.includes("Google was"), page);
  assert(page.includes("not connected"), page);
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

// R2-8 / R-C2-E56: Supabase's functions relay rewrites an HTML response as `text/plain` on the
// shared *.supabase.co domain — observed live on staging 2026-09-16, `GET
// /functions/v1/google-callback?state=bogus&code=x` answered with `content-type: text/plain` and a
// `content-security-policy: default-src 'none'; sandbox` this code never set, with the markup
// arriving as literal text in the body. Every consent page (success, declined, expired, error) was
// showing raw HTML source to the student. `page()` now answers the plain sentence itself — no
// markup at all — with headers that match what actually reaches the browser.
Deno.test("every page is a plain sentence, no markup, with nosniff and a CSP of default-src 'none'", async () => {
  const cases: Array<[Response, string]> = [
    [
      await callbackHandler(deps())(get("state=good&code=abc")),
      "Google Calendar is connected. You can close this window — Knowlu will read it at your next slot.",
    ],
    [
      await callbackHandler(deps())(get("code=abc")),
      "That link has expired. Start again from Knowlu.",
    ],
    [
      await callbackHandler(deps({ exchange: () => Promise.reject(new Error("boom")) }))(get("state=good&code=abc")),
      "Google could not be connected just now. You can close this window and try again from Knowlu.",
    ],
    [
      await callbackHandler(deps())(get("state=good&error=access_denied")),
      "Google was not connected. You can close this window and try again from Knowlu.",
    ],
  ];
  for (const [response, expected] of cases) {
    assertEquals(response.headers.get("content-type"), "text/plain; charset=utf-8");
    assertEquals(response.headers.get("x-content-type-options"), "nosniff");
    assertEquals(response.headers.get("content-security-policy"), "default-src 'none'");
    const body = await response.text();
    assertEquals(body.includes("<"), false, `the body must carry no markup at all: ${body}`);
    assertEquals(body, expected);
  }
});
