import { assert, assertEquals, assertRejects } from "@std/assert";
import { accessTokenFromRefresh } from "./google_token.ts";

const TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";

function deps(fetchImpl: typeof fetch) {
  return { clientId: "client-id-not-a-secret", clientSecret: "client-secret-not-a-secret", fetch: fetchImpl };
}

Deno.test("the exchange is a POST to the token endpoint with the four form fields", async () => {
  let seen: { url: string; method: string; contentType: string | null; body: string } | null = null;
  const fake: typeof fetch = (input, init) => {
    seen = {
      url: String(input),
      method: String(init?.method),
      contentType: (init?.headers as Record<string, string> | undefined)?.["Content-Type"] ?? null,
      body: String(init?.body),
    };
    return Promise.resolve(new Response(JSON.stringify({ access_token: "ACCESS-1" }), { status: 200 }));
  };
  const token = await accessTokenFromRefresh("REFRESH-1", deps(fake));
  assertEquals(token, "ACCESS-1");
  assert(seen !== null);
  const s = seen as { url: string; method: string; contentType: string | null; body: string };
  assertEquals(s.url, TOKEN_ENDPOINT);
  assertEquals(s.method, "POST");
  assertEquals(s.contentType, "application/x-www-form-urlencoded");
  const form = new URLSearchParams(s.body);
  assertEquals(form.get("grant_type"), "refresh_token");
  assertEquals(form.get("refresh_token"), "REFRESH-1");
  assertEquals(form.get("client_id"), "client-id-not-a-secret");
  assertEquals(form.get("client_secret"), "client-secret-not-a-secret");
});

Deno.test("invalid_grant (revoked, or the 7-day testing-mode expiry) is null, not a throw", async () => {
  const fake: typeof fetch = () =>
    Promise.resolve(new Response(JSON.stringify({ error: "invalid_grant" }), { status: 400 }));
  const token = await accessTokenFromRefresh("REFRESH-2", deps(fake));
  assertEquals(token, null);
});

Deno.test("any other failure throws, and the message names the status but never the token or the body", async () => {
  const TRIPWIRE = "TRIPWIRE-do-not-leak-9f2c";
  const fake: typeof fetch = () =>
    Promise.resolve(new Response(`{"error":"server_error","body":"${TRIPWIRE}"}`, { status: 500 }));
  const err = await assertRejects(() => accessTokenFromRefresh(`REFRESH-${TRIPWIRE}`, deps(fake)));
  const message = err instanceof Error ? err.message : String(err);
  assert(message.includes("500"), message);
  assertEquals(message.includes(TRIPWIRE), false, message);
  assertEquals(message.includes("REFRESH-"), false, message);
});
