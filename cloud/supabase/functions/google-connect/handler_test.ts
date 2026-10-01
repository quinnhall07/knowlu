import { assert, assertEquals } from "@std/assert";
import { CALENDAR_SCOPE, connectHandler, GMAIL_SCOPE, GOOGLE_NOT_CONFIGURED, IDENTITY_SCOPES } from "./handler.ts";
import type { GoogleGrant } from "./handler.ts";
import { requireUser } from "../_shared/auth.ts";
import { requireActiveEntitlementWith } from "../_shared/entitlement.ts";
import type { EntitlementStatus } from "../_shared/entitlement.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });

/** P4: the fake `grant` row the old `grantedScopes` fakes implied — `active`, those scopes. */
function activeRow(scopes: string[]): GoogleGrant {
  return { scopes, status: "active", email: null };
}

function deps(overrides: Record<string, unknown> = {}) {
  return {
    authenticate: OK,
    clientId: "client-id-not-a-secret",
    redirectUri: "https://ref.supabase.co/functions/v1/google-callback",
    saveState: () => Promise.resolve("state-nonce"),
    disconnect: () => Promise.resolve(),
    grant: () => Promise.resolve(null),
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
  assertEquals(url.searchParams.get("scope"), `${IDENTITY_SCOPES.join(" ")} ${CALENDAR_SCOPE}`);
  assertEquals(CALENDAR_SCOPE, "https://www.googleapis.com/auth/calendar.readonly");
  // C2 final review C-2: `openid email` ride along so Google returns an `id_token` at all. Both
  // are NON-SENSITIVE and touch neither the sensitive nor the restricted review path.
  assertEquals(IDENTITY_SCOPES, ["openid", "email"]);
  assert(!url.searchParams.get("scope")!.includes("gmail"), "Gmail is a separate, later, optional ask");
  assert(!url.searchParams.get("scope")!.includes("calendar.events"), "read-only; write is a third ask");
  assertEquals(url.searchParams.get("access_type"), "offline");
  assertEquals(url.searchParams.get("prompt"), "consent");
  // P3 live pass, 2026-09-17: `true` on this ask too — see the "EVERY ask is incremental" test.
  assertEquals(url.searchParams.get("include_granted_scopes"), "true");
  assertEquals(url.searchParams.get("state"), "state-nonce");
  assertEquals(url.searchParams.get("response_type"), "code");
});

Deno.test("the gmail ask is incremental — one API scope, and it widens the existing grant", async () => {
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?scope=gmail"))).json();
  const url = new URL(reply.url);
  assertEquals(url.searchParams.get("scope"), `${IDENTITY_SCOPES.join(" ")} ${GMAIL_SCOPE}`);
  assertEquals(GMAIL_SCOPE, "https://www.googleapis.com/auth/gmail.readonly");
  // `include_granted_scopes=true` is what makes this ADD to the grant rather than replace it: a
  // student who connected the calendar in the wizard must not lose it by connecting Gmail later.
  assertEquals(url.searchParams.get("include_granted_scopes"), "true");
});

Deno.test("EVERY ask is incremental — a calendar re-consent must keep Gmail on the new token", async () => {
  // Found by the P3 live pass on staging, 2026-09-17, not by any review: with
  // `include_granted_scopes=false` on the calendar ask, the re-consent Testing mode forces every
  // seven days (and the one the settings row will offer) made Google issue a refresh token for
  // `openid email calendar.readonly` ALONE, while `google_accounts.scopes` — a union, by
  // `store_google_grant`'s design — still claimed Gmail. `read_google_grant` then handed that
  // calendar-only token to `/gmail-read`, Gmail answered 403, the function answered 500, and the
  // device printed `gmail: skipped (…)` on every slot with nothing a student could act on. `true`
  // on every ask makes the new token cover whatever was already granted — which is exactly what
  // the row says it covers. On a FIRST consent nothing was granted before, so `true` costs nothing.
  const handler = connectHandler(OK, deps());
  for (const path of ["/google-connect", "/google-connect?scope=calendar", "/google-connect?scope=gmail"]) {
    const url = new URL((await (await handler(new Request(`http://127.0.0.1${path}`))).json()).url);
    assertEquals(url.searchParams.get("include_granted_scopes"), "true", path);
  }
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
    grant: () => Promise.resolve(activeRow([CALENDAR_SCOPE])),
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
  const handler = connectHandler(OK, deps({ grant: () => Promise.resolve(activeRow([GMAIL_SCOPE])) }));
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

Deno.test("a rejected deps.disconnect is a 502 that claims nothing was disconnected", async () => {
  // R-C2-E34: `disconnectGrant` forgets the row only on a successful (or already-moot) revoke, so
  // a rejected `deps.disconnect` means the grant is STILL live at Google and STILL stored.
  // Answering `{disconnected: true}` here would be a lie the very next `?status=1` poll exposes —
  // so this is a 502, and the body must never carry a `disconnected` field a caller could read as
  // truthy by accident.
  const handler = connectHandler(OK, deps({ disconnect: () => Promise.reject(new Error("google 503")) }));
  const response = await handler(new Request("http://127.0.0.1/google-connect", { method: "DELETE" }));
  assertEquals(response.status, 502);
  const body = await response.json();
  assertEquals(body, { error: "Google could not be reached to disconnect; try again" });
  assertEquals("disconnected" in body, false, "nothing here may claim the grant is gone");
});

Deno.test("a missing client id is a 503 that names the configuration, not the account", async () => {
  const handler = connectHandler(OK, deps({ clientId: "" }));
  const response = await handler(new Request("http://127.0.0.1/google-connect"));
  assertEquals(response.status, 503);
  assertEquals((await response.json()).error, GOOGLE_NOT_CONFIGURED);
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
  assertEquals(first.searchParams.get("scope"), `${IDENTITY_SCOPES.join(" ")} ${CALENDAR_SCOPE}`);
  assertEquals(second.searchParams.get("scope"), `${IDENTITY_SCOPES.join(" ")} ${GMAIL_SCOPE}`);
  for (const url of [first, second]) {
    // C2 final review C-2: exactly ONE Google API scope per consent — the identity scopes
    // (`openid email`, non-sensitive) do not count, because the thing §11a's ordering exists to
    // avoid is dragging the calendar's *sensitive* review behind Gmail's *restricted* one.
    const api = url.searchParams.get("scope")!.split(" ").filter((s) => s.includes("googleapis.com/auth/"));
    assertEquals(api.length, 1, "exactly one Google API scope per consent");
  }
  // Both incremental (P3 live pass, 2026-09-17): the ORDER is what this test pins, not the flag.
  assertEquals(first.searchParams.get("include_granted_scopes"), "true");
  assertEquals(second.searchParams.get("include_granted_scopes"), "true");
});

// --- Gmail connect T2 (spec §4.1, D5, D8; §8.3 items 3 and 4) ---------------------------------

/** One `?status=1` round trip against a fake `grant` row. */
async function statusOf(row: GoogleGrant | null) {
  const handler = connectHandler(OK, deps({ grant: () => Promise.resolve(row) }));
  return await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
}

Deno.test("status: no row answers none", async () => {
  assertEquals(await statusOf(null), { connected: false, scopes: [], status: "none", email: null });
});

Deno.test("status: an active row answers its scopes and email_hint", async () => {
  const reply = await statusOf({ scopes: [CALENDAR_SCOPE, GMAIL_SCOPE], status: "active", email: "hint@example.test" });
  assertEquals(reply, {
    connected: true,
    scopes: [CALENDAR_SCOPE, GMAIL_SCOPE],
    status: "active",
    email: "hint@example.test",
  });
});

Deno.test("status: a revoked row answers not connected with no scopes", async () => {
  // `scopes: []` for a revoked row is what the wizard's `google_connected` saw before D5, when the
  // row was filtered out; `status` is what lets the Settings row say "Reconnect" instead of "none".
  const reply = await statusOf({ scopes: [CALENDAR_SCOPE, GMAIL_SCOPE], status: "revoked", email: "hint@example.test" });
  assertEquals(reply.connected, false);
  assertEquals(reply.scopes, []);
  assertEquals(reply.status, "revoked");
});

Deno.test("status: a quiet row answers connected with its scopes", async () => {
  const reply = await statusOf({ scopes: [GMAIL_SCOPE], status: "quiet", email: null });
  assertEquals(reply.connected, true);
  assertEquals(reply.scopes, [GMAIL_SCOPE]);
  assertEquals(reply.status, "quiet");
  assertEquals(reply.email, null);
});

/** One `?scope=reconnect` ask against a fake `grant` row; counts the nonces it minted. */
async function reconnectWith(row: GoogleGrant | null) {
  let minted = 0;
  const handler = connectHandler(OK, deps({
    grant: () => Promise.resolve(row),
    saveState: () => { minted += 1; return Promise.resolve("state-nonce"); },
  }));
  const response = await handler(new Request("http://127.0.0.1/google-connect?scope=reconnect"));
  return { response, minted: () => minted };
}

Deno.test("scopeFor reconnect asks for every recorded scope in one consent", async () => {
  // D8: a Testing-mode token that lapsed after seven days is re-asked for EVERYTHING the row
  // records, in one consent, so the new token can never cover fewer scopes than the row claims
  // (P3's live defect). A revoked row is the normal case here: it is what "Reconnect" is for.
  const { response } = await reconnectWith({ scopes: [GMAIL_SCOPE, CALENDAR_SCOPE], status: "revoked", email: null });
  assertEquals(response.status, 200);
  const url = new URL((await response.json()).url);
  const scope = url.searchParams.get("scope")!.split(" ");
  for (const s of [...IDENTITY_SCOPES, CALENDAR_SCOPE, GMAIL_SCOPE]) assert(scope.includes(s), s);
  assertEquals(scope.length, 4);
  assertEquals(url.searchParams.get("include_granted_scopes"), "true");
  assertEquals(url.searchParams.get("prompt"), "consent");
  assertEquals(url.searchParams.get("state"), "state-nonce");
});

Deno.test("scopeFor reconnect with a calendar-only row asks for calendar only", async () => {
  const { response } = await reconnectWith({ scopes: [CALENDAR_SCOPE], status: "active", email: null });
  const url = new URL((await response.json()).url);
  assertEquals(url.searchParams.get("scope"), `${IDENTITY_SCOPES.join(" ")} ${CALENDAR_SCOPE}`);
});

Deno.test("reconnect with no row, or a row with empty scopes, answers 400 nothing to reconnect", async () => {
  for (const row of [null, { scopes: [], status: "revoked" as const, email: null }]) {
    const { response, minted } = await reconnectWith(row);
    assertEquals(response.status, 400, JSON.stringify(row));
    assertEquals(await response.json(), { error: "nothing to reconnect" });
    assertEquals(minted(), 0, "a refused reconnect mints no nonce");
  }
});

// --- Whole-branch review, 2026-09-30: a lapsed account can still see and disconnect ------------
// Only the consent URL is a paid service. D14's purge and the signed privacy sentence
// ("Disconnecting (in Settings, at any time) …") must hold for a canceled or past_due account too,
// as account deletion does (`account/handler.ts` gates it on `requireUser` alone).

/** The real gates over a fake session and a fake `entitlements` row: `requireActiveEntitlementWith`
 * is what production's `requireActiveEntitlement` runs; `requireUser` is authentication alone. */
const verify = (token: string) => Promise.resolve(token === "session" ? { id: "acct-lapsed", email: null } : null);
function lapsed(status: EntitlementStatus) {
  const lookup = () => Promise.resolve({ plan: "monthly", status, current_period_end: null });
  return (req: Request) => requireActiveEntitlementWith(req, { verify, lookup });
}
const signedIn = async (req: Request) => ({ account_id: (await requireUser(req, verify)).id });
const withSession = (method = "GET") => ({ method, headers: { authorization: "Bearer session" } });

Deno.test("a canceled or past_due account's DELETE calls disconnect and answers 200", async () => {
  for (const status of ["canceled", "past_due"] as const) {
    const disconnected: string[] = [];
    const handler = connectHandler(lapsed(status), deps({
      authenticate: signedIn,
      disconnect: (id: string) => { disconnected.push(id); return Promise.resolve(); },
    }));
    const response = await handler(new Request("http://127.0.0.1/google-connect", withSession("DELETE")));
    assertEquals(response.status, 200, status);
    assertEquals(await response.json(), { disconnected: true });
    assertEquals(disconnected, ["acct-lapsed"], status);
  }
});

Deno.test("a lapsed account's ?status=1 answers its row, so Settings can offer Disconnect", async () => {
  const handler = connectHandler(lapsed("canceled"), deps({
    authenticate: signedIn,
    grant: () => Promise.resolve(activeRow([CALENDAR_SCOPE, GMAIL_SCOPE])),
  }));
  const response = await handler(new Request("http://127.0.0.1/google-connect?status=1", withSession()));
  assertEquals(response.status, 200);
  assertEquals((await response.json()).status, "active");
});

Deno.test("a lapsed account's consent asks stay 402 and mint no nonce", async () => {
  let minted = 0;
  const handler = connectHandler(lapsed("canceled"), deps({
    authenticate: signedIn,
    grant: () => Promise.resolve(activeRow([CALENDAR_SCOPE])),
    saveState: () => { minted += 1; return Promise.resolve("n"); },
  }));
  for (const query of ["", "?scope=calendar", "?scope=gmail", "?scope=reconnect"]) {
    const response = await handler(new Request(`http://127.0.0.1/google-connect${query}`, withSession()));
    assertEquals(response.status, 402, query);
  }
  assertEquals(minted, 0);
});

Deno.test("status and DELETE still need a valid session: 401, and nothing is purged", async () => {
  // `OK` as the entitlement gate: these two routes must pass `authenticate`, never skip it.
  let called = false;
  const handler = connectHandler(OK, deps({
    authenticate: signedIn,
    disconnect: () => { called = true; return Promise.resolve(); },
  }));
  for (const init of [{ method: "DELETE" }, { method: "DELETE", headers: { authorization: "Bearer stale" } }]) {
    assertEquals((await handler(new Request("http://127.0.0.1/google-connect", init))).status, 401);
  }
  assertEquals((await handler(new Request("http://127.0.0.1/google-connect?status=1"))).status, 401);
  assertEquals(called, false);
});
