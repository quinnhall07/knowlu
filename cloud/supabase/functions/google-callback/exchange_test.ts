import { assert, assertEquals, assertRejects } from "@std/assert";
import {
  claimsFromIdToken,
  ExchangeError,
  exchangeCode,
  TOKEN_ENDPOINT,
  USERINFO_ENDPOINT,
} from "./exchange.ts";

const DEPS = {
  clientId: "test-client",
  clientSecret: "test-secret",
  redirectUri: "https://example.test/functions/v1/google-callback",
};

/** A base64url JWT payload with these claims. No signature is ever checked, so the header and
 * signature segments are placeholders. */
function idToken(claims: Record<string, unknown>): string {
  const payload = btoa(JSON.stringify(claims)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  return `eyJhbGciOiJSUzI1NiJ9.${payload}.signature`;
}

/** A `fetch` that answers from a table of URL prefix -> response, recording what it was asked. */
function fakeFetch(
  table: Array<[string, { status?: number; body?: unknown }]>,
  seen: string[] = [],
): typeof fetch {
  return ((input: string | URL | Request) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    seen.push(url);
    const hit = table.find(([prefix]) => url.startsWith(prefix));
    if (hit === undefined) return Promise.resolve(new Response("", { status: 404 }));
    const [, answer] = hit;
    return Promise.resolve(
      new Response(JSON.stringify(answer.body ?? {}), {
        status: answer.status ?? 200,
        headers: { "content-type": "application/json" },
      }),
    );
  }) as unknown as typeof fetch;
}

Deno.test("an exchange that carries an id_token reads sub and email from it, and asks userinfo nothing", async () => {
  const seen: string[] = [];
  const grant = await exchangeCode("the-code", {
    ...DEPS,
    fetchImpl: fakeFetch([[TOKEN_ENDPOINT, {
      body: {
        refresh_token: "r1",
        access_token: "a1",
        id_token: idToken({ sub: "google-sub-1", email: "student@example.test" }),
        scope: "openid email https://www.googleapis.com/auth/calendar.readonly",
      },
    }]], seen),
  });
  assertEquals(grant.sub, "google-sub-1");
  assertEquals(grant.email, "student@example.test");
  assertEquals(grant.refresh_token, "r1");
  // Verbatim, `openid`/`email` included: the row records Google's answer, not our request.
  assertEquals(grant.scopes, ["openid", "email", "https://www.googleapis.com/auth/calendar.readonly"]);
  assertEquals(seen, [TOKEN_ENDPOINT], "no userinfo round trip when the id_token answered");
});

Deno.test("an exchange with NO id_token falls back to userinfo instead of throwing", async () => {
  // The shape every C2 consent actually produced before this fix: one API scope asked for, so
  // Google sent no `id_token` at all, and `body.id_token.split(".")` was a TypeError.
  const seen: string[] = [];
  const grant = await exchangeCode("the-code", {
    ...DEPS,
    fetchImpl: fakeFetch([
      [TOKEN_ENDPOINT, {
        body: {
          refresh_token: "r2",
          access_token: "a2",
          scope: "https://www.googleapis.com/auth/calendar.readonly",
        },
      }],
      [USERINFO_ENDPOINT, { body: { sub: "google-sub-2", email: "other@example.test" } }],
    ], seen),
  });
  assertEquals(grant.sub, "google-sub-2");
  assertEquals(grant.email, "other@example.test");
  assertEquals(grant.scopes, ["https://www.googleapis.com/auth/calendar.readonly"]);
  assertEquals(seen, [TOKEN_ENDPOINT, USERINFO_ENDPOINT]);
});

Deno.test("an exchange with neither an id_token nor a usable userinfo is a named error, never a TypeError", async () => {
  const error = await assertRejects(
    () =>
      exchangeCode("the-code", {
        ...DEPS,
        fetchImpl: fakeFetch([
          [TOKEN_ENDPOINT, { body: { refresh_token: "r3", access_token: "a3", scope: "" } }],
          [USERINFO_ENDPOINT, { status: 403, body: { error: "forbidden" } }],
        ]),
      }),
    ExchangeError,
  );
  assert(!(error instanceof TypeError), "a missing id_token must never be a TypeError");
  assertEquals(error.name, "ExchangeError");
  assert(error.message.includes("no id_token and no userinfo"), error.message);
  // The message is our own words only — never the code, never a response body.
  assert(!error.message.includes("the-code"), error.message);
  assert(!error.message.includes("forbidden"), error.message);
});

Deno.test("an exchange with no refresh_token, and a non-200 exchange, are both named errors", async () => {
  const noRefresh = await assertRejects(
    () =>
      exchangeCode("the-code", {
        ...DEPS,
        fetchImpl: fakeFetch([[TOKEN_ENDPOINT, { body: { access_token: "a4" } }]]),
      }),
    ExchangeError,
  );
  assert(noRefresh.message.includes("no refresh_token"), noRefresh.message);

  const refused = await assertRejects(
    () =>
      exchangeCode("the-code", {
        ...DEPS,
        fetchImpl: fakeFetch([[TOKEN_ENDPOINT, { status: 400, body: { error: "invalid_grant" } }]]),
      }),
    ExchangeError,
  );
  assertEquals(refused.message, "token exchange 400");
});

Deno.test("an unreadable id_token falls through to userinfo rather than failing the consent", async () => {
  assertEquals(claimsFromIdToken(undefined), null);
  assertEquals(claimsFromIdToken("not-a-jwt"), null);
  assertEquals(claimsFromIdToken("a.!!!.c"), null);
  // A well-formed JWT with no `sub` names nobody, which is the same situation as no token at all.
  assertEquals(claimsFromIdToken(idToken({ email: "who@example.test" })), null);
  assertEquals(claimsFromIdToken(idToken({ sub: "s" })), { sub: "s", email: undefined });

  const grant = await exchangeCode("the-code", {
    ...DEPS,
    fetchImpl: fakeFetch([
      [TOKEN_ENDPOINT, { body: { refresh_token: "r5", access_token: "a5", id_token: "garbage", scope: "" } }],
      [USERINFO_ENDPOINT, { body: { sub: "google-sub-5" } }],
    ]),
  });
  assertEquals(grant.sub, "google-sub-5");
  assertEquals(grant.email, undefined);
});
