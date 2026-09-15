import { assert, assertEquals } from "@std/assert";
import { CALENDAR_SCOPE, connectHandler, GMAIL_SCOPE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });

function deps(overrides: Record<string, unknown> = {}) {
  return {
    clientId: "client-id-not-a-secret",
    redirectUri: "https://ref.supabase.co/functions/v1/google-callback",
    saveState: () => Promise.resolve("state-nonce"),
    disconnect: () => Promise.resolve(),
    grantedScopes: () => Promise.resolve([]),
    ...overrides,
  };
}

Deno.test("the DEFAULT ask is calendar.readonly alone, offline, with consent", async () => {
  // §11a: the wizard's first connection step is "Connect your calendars", so the first Google ask
  // is the *sensitive* calendar scope — lighter verification, no CASA. Gmail comes later or not
  // at all.
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect"))).json();
  const url = new URL(reply.url);
  assertEquals(url.origin + url.pathname, "https://accounts.google.com/o/oauth2/v2/auth");
  assertEquals(url.searchParams.get("scope"), CALENDAR_SCOPE);
  assertEquals(CALENDAR_SCOPE, "https://www.googleapis.com/auth/calendar.readonly");
  assert(!url.searchParams.get("scope")!.includes("gmail"), "Gmail is a separate, later, optional ask");
  assert(!url.searchParams.get("scope")!.includes("calendar.events"), "read-only; write is a third ask");
  assertEquals(url.searchParams.get("access_type"), "offline");
  assertEquals(url.searchParams.get("prompt"), "consent");
  assertEquals(url.searchParams.get("include_granted_scopes"), "false");
  assertEquals(url.searchParams.get("state"), "state-nonce");
  assertEquals(url.searchParams.get("response_type"), "code");
});

Deno.test("the gmail ask is incremental — one scope, and it widens the existing grant", async () => {
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?scope=gmail"))).json();
  const url = new URL(reply.url);
  assertEquals(url.searchParams.get("scope"), GMAIL_SCOPE);
  assertEquals(GMAIL_SCOPE, "https://www.googleapis.com/auth/gmail.readonly");
  // `include_granted_scopes=true` is what makes this ADD to the grant rather than replace it: a
  // student who connected the calendar in the wizard must not lose it by connecting Gmail later.
  assertEquals(url.searchParams.get("include_granted_scopes"), "true");
});

Deno.test("an unknown scope name is a 400, not a silent calendar grant", async () => {
  const handler = connectHandler(OK, deps());
  const response = await handler(new Request("http://127.0.0.1/google-connect?scope=drive"));
  assertEquals(response.status, 400);
});

Deno.test("?status=1 reports what was granted, and mints no nonce", async () => {
  // The wizard's poll (hand-off H9): the consent window closes itself, so this is the only thing
  // that tells the device the round trip finished. It must NOT start a second consent — a poll
  // that minted a nonce every three seconds would fill `google_state` and leak a fresh URL.
  let minted = 0;
  const handler = connectHandler(OK, deps({
    grantedScopes: () => Promise.resolve([CALENDAR_SCOPE]),
    saveState: () => { minted += 1; return Promise.resolve("n"); },
  }));
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
  assertEquals(reply.connected, true);
  assertEquals(reply.scopes, [CALENDAR_SCOPE]);
  assertEquals(reply.url, undefined, "a status check is not a consent");
  assertEquals(minted, 0);
});

Deno.test("?status=1 on an account that never connected is connected:false and no scopes", async () => {
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
  assertEquals(reply.connected, false);
  assertEquals(reply.scopes, []);
});

Deno.test("?status=1 distinguishes a gmail-only grant from a calendar one", async () => {
  // The wizard keys on the CALENDAR scope specifically, because a student can untick one on the
  // consent screen and a bare boolean would let the panel say the calendar is on when it is not.
  const handler = connectHandler(OK, deps({ grantedScopes: () => Promise.resolve([GMAIL_SCOPE]) }));
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
  assertEquals(reply.connected, true);
  assertEquals(reply.scopes.includes(CALENDAR_SCOPE), false);
});

Deno.test("the handler answers disconnected: true once deps.disconnect resolves", async () => {
  // R-C2-E33: this only proves the shape at this layer — that a resolved `deps.disconnect` is
  // awaited before the reply. The real revoke-then-forget ORDERING, and what a failed revoke does
  // to the row, is `disconnect.ts`'s own contract, proven in `disconnect_test.ts` — a mock's own
  // literal here proved nothing about either.
  let called = false;
  const handler = connectHandler(OK, deps({
    disconnect: () => {
      called = true;
      return Promise.resolve();
    },
  }));
  const response = await handler(new Request("http://127.0.0.1/google-connect", { method: "DELETE" }));
  assertEquals(response.status, 200);
  assertEquals(await response.json(), { disconnected: true });
  assert(called, "deps.disconnect must be awaited before the reply");
});

Deno.test("a rejected deps.disconnect still answers disconnected: true at this layer", async () => {
  // The user asked to be disconnected; a failed revoke must not surface as a 500 that leaves a UI
  // stuck retrying forever. Whether the grant itself survives a failed revoke is `disconnect.ts`'s
  // job (`disconnect_test.ts`), not this handler's — this only proves the API boundary swallows
  // the rejection and logs it (`google-connect: revoke failed (…)`), which is a distinct promise
  // from what actually happens to the row.
  const handler = connectHandler(OK, deps({ disconnect: () => Promise.reject(new Error("google 503")) }));
  const response = await handler(new Request("http://127.0.0.1/google-connect", { method: "DELETE" }));
  assertEquals(response.status, 200);
  assertEquals((await response.json()).disconnected, true);
});

Deno.test("a missing client id is a 503 that names the configuration, not the account", async () => {
  const handler = connectHandler(OK, deps({ clientId: "" }));
  const response = await handler(new Request("http://127.0.0.1/google-connect"));
  assertEquals(response.status, 503);
  assertEquals((await response.json()).error, "Google sign-in is not configured on this deployment");
});

Deno.test("the entitlement check runs before a nonce is minted", async () => {
  let minted = false;
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const handler = connectHandler(refuse, deps({
    saveState: () => {
      minted = true;
      return Promise.resolve("n");
    },
  }));
  assertEquals((await handler(new Request("http://127.0.0.1/google-connect"))).status, 402);
  assertEquals(minted, false);
});

Deno.test("calendar_is_asked_for_before_gmail_and_never_together", async () => {
  // §11a in one assertion: the wizard's first connection step is the calendars, the calendar
  // scope is *sensitive* (lighter review, no CASA), and Gmail is restricted and optional. A
  // single consent asking for both would drag the calendar behind Gmail's verification and CASA
  // — which is the whole cost this ordering exists to avoid.
  const handler = connectHandler(OK, deps());
  const first = new URL((await (await handler(new Request("http://127.0.0.1/google-connect"))).json()).url);
  const second = new URL((await (await handler(new Request("http://127.0.0.1/google-connect?scope=gmail"))).json()).url);
  assertEquals(first.searchParams.get("scope"), CALENDAR_SCOPE);
  assertEquals(second.searchParams.get("scope"), GMAIL_SCOPE);
  for (const url of [first, second]) {
    assertEquals(url.searchParams.get("scope")!.split(" ").length, 1, "one scope per consent");
  }
  assertEquals(first.searchParams.get("include_granted_scopes"), "false");
  assertEquals(second.searchParams.get("include_granted_scopes"), "true");
});
