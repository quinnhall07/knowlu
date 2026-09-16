import { assertEquals } from "@std/assert";
import { storeGrantRevokingStale } from "./reconnect.ts";

function deps(overrides: Partial<Parameters<typeof storeGrantRevokingStale>[5]> = {}) {
  return {
    currentGrant: () => Promise.resolve(null),
    revoke: () => Promise.resolve(new Response("", { status: 200 })),
    store: () => Promise.resolve(),
    ...overrides,
  };
}

Deno.test("no prior grant: revoke is never called, and the new grant is still stored", async () => {
  let revoked = false;
  let stored: unknown[] = [];
  await storeGrantRevokingStale(
    "acct-1",
    "sub-new",
    "new@example.edu",
    "refresh-new",
    ["calendar.readonly"],
    deps({
      revoke: () => {
        revoked = true;
        return Promise.resolve(new Response("", { status: 200 }));
      },
      store: (id, sub, email, token, scopes) => {
        stored = [id, sub, email, token, scopes];
        return Promise.resolve();
      },
    }),
  );
  assertEquals(revoked, false);
  assertEquals(stored, ["acct-1", "sub-new", "new@example.edu", "refresh-new", ["calendar.readonly"]]);
});

Deno.test("the SAME Google account reconnecting never revokes its own token", async () => {
  let revoked = false;
  await storeGrantRevokingStale(
    "acct-1",
    "sub-same",
    undefined,
    "refresh-fresh",
    ["gmail.readonly"],
    deps({
      currentGrant: () => Promise.resolve({ sub: "sub-same", token: "refresh-old" }),
      revoke: () => {
        revoked = true;
        return Promise.resolve(new Response("", { status: 200 }));
      },
    }),
  );
  assertEquals(revoked, false, "the same sub reconnecting is an incremental consent, not a switch");
});

Deno.test("a DIFFERENT Google account revokes the stale token before the new one is stored", async () => {
  const calls: string[] = [];
  let revokedToken = "";
  await storeGrantRevokingStale(
    "acct-1",
    "sub-b",
    "b@example.edu",
    "refresh-b",
    ["calendar.readonly"],
    deps({
      currentGrant: () => Promise.resolve({ sub: "sub-a", token: "refresh-a" }),
      revoke: (token) => {
        revokedToken = token;
        calls.push("revoke");
        return Promise.resolve(new Response("", { status: 200 }));
      },
      store: () => {
        calls.push("store");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(revokedToken, "refresh-a", "the OLD token is revoked, never the new one");
  assertEquals(calls, ["revoke", "store"], "revoke happens before the new grant is stored");
});

Deno.test("Google's own invalid_token (400) for the stale grant still lets the new one store", async () => {
  let stored = false;
  await storeGrantRevokingStale(
    "acct-1",
    "sub-b",
    undefined,
    "refresh-b",
    ["calendar.readonly"],
    deps({
      currentGrant: () => Promise.resolve({ sub: "sub-a", token: "refresh-a" }),
      revoke: () => Promise.resolve(new Response("", { status: 400 })),
      store: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(stored, true);
});

// F-5's central assertion for this half: a student here to connect an account must not be told no
// because revoking the STALE token failed.

Deno.test("a failed stale revoke (5xx) does not block storing the new grant", async () => {
  let stored = false;
  await storeGrantRevokingStale(
    "acct-1",
    "sub-b",
    undefined,
    "refresh-b",
    ["calendar.readonly"],
    deps({
      currentGrant: () => Promise.resolve({ sub: "sub-a", token: "refresh-a" }),
      revoke: () => Promise.resolve(new Response("", { status: 503 })),
      store: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(stored, true);
});

Deno.test("the stale revoke throwing (a network failure) does not block storing the new grant", async () => {
  let stored = false;
  await storeGrantRevokingStale(
    "acct-1",
    "sub-b",
    undefined,
    "refresh-b",
    ["calendar.readonly"],
    deps({
      currentGrant: () => Promise.resolve({ sub: "sub-a", token: "refresh-a" }),
      revoke: () => Promise.reject(new Error("network unreachable")),
      store: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(stored, true);
});

// R2-2: pre-fix, `currentGrant`'s own read ran outside every try/catch — a transient
// `google_accounts` read failure rejected `storeGrantRevokingStale` after the `state` nonce and the
// authorization code were both spent, so the student had to restart the whole flow. The lookup is
// now wrapped exactly like the revoke call above it: on failure, skip the stale-revoke step (there
// is nothing to compare `sub` against) and store the new grant exactly as when there is no prior
// grant at all.

Deno.test("a currentGrant read that throws does not block storing the new grant, and never revokes", async () => {
  let revoked = false;
  let stored: unknown[] = [];
  await storeGrantRevokingStale(
    "acct-1",
    "sub-b",
    "b@example.edu",
    "refresh-b",
    ["calendar.readonly"],
    deps({
      currentGrant: () => Promise.reject(new Error("postgrest 503 (unknown) on google_accounts")),
      revoke: () => {
        revoked = true;
        return Promise.resolve(new Response("", { status: 200 }));
      },
      store: (id, sub, email, token, scopes) => {
        stored = [id, sub, email, token, scopes];
        return Promise.resolve();
      },
    }),
  );
  assertEquals(revoked, false, "there is nothing to compare sub against when the read itself failed");
  assertEquals(stored, ["acct-1", "sub-b", "b@example.edu", "refresh-b", ["calendar.readonly"]]);
});

Deno.test("a currentGrant rejection with a non-Error value still resolves and stores", async () => {
  let stored = false;
  await storeGrantRevokingStale(
    "acct-1",
    "sub-b",
    undefined,
    "refresh-b",
    ["calendar.readonly"],
    deps({
      // Deliberately a non-Error rejection — proving the catch's `e instanceof Error` guard also
      // survives whatever a PostgREST client or a future refactor might one day reject with.
      currentGrant: () => Promise.reject("boom"),
      store: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(stored, true);
});
