import { assertEquals } from "@std/assert";
import { revokeGoogleGrantOnDelete } from "./google_delete.ts";

function deps(overrides: Partial<Parameters<typeof revokeGoogleGrantOnDelete>[1]> = {}) {
  return {
    grant: () => Promise.resolve(null),
    revoke: () => Promise.resolve(new Response("", { status: 200 })),
    forget: () => Promise.resolve(),
    ...overrides,
  };
}

Deno.test("no grant at all: revoke is never called, and forget still runs", async () => {
  let revoked = false;
  let forgotten = false;
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      revoke: () => {
        revoked = true;
        return Promise.resolve(new Response("", { status: 200 }));
      },
      forget: () => {
        forgotten = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(revoked, false);
  assertEquals(forgotten, true);
});

Deno.test("a normal revoke (200) is followed by forget", async () => {
  const calls: string[] = [];
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      grant: () => Promise.resolve("refresh-token-not-a-secret"),
      revoke: (token) => {
        assertEquals(token, "refresh-token-not-a-secret");
        calls.push("revoke");
        return Promise.resolve(new Response("", { status: 200 }));
      },
      forget: () => {
        calls.push("forget");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(calls, ["revoke", "forget"]);
});

Deno.test("Google's own invalid_token (400) counts as done, and forget still runs", async () => {
  let forgotten = false;
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      grant: () => Promise.resolve("refresh-token-not-a-secret"),
      revoke: () => Promise.resolve(new Response("", { status: 400 })),
      forget: () => {
        forgotten = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(forgotten, true);
});

// F-5's central assertion: the account's right to delete wins. A revoke that fails must never
// stop the deletion — `forget` (the Vault secret and the row) runs regardless.

Deno.test("a failed revoke (5xx) does not block forget", async () => {
  let forgotten = false;
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      grant: () => Promise.resolve("refresh-token-not-a-secret"),
      revoke: () => Promise.resolve(new Response("", { status: 503 })),
      forget: () => {
        forgotten = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(forgotten, true);
});

Deno.test("revoke itself throwing (a network failure) does not block forget", async () => {
  let forgotten = false;
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      grant: () => Promise.resolve("refresh-token-not-a-secret"),
      revoke: () => Promise.reject(new Error("network unreachable")),
      forget: () => {
        forgotten = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(forgotten, true);
});

Deno.test("even a failure reading the grant itself does not block forget", async () => {
  let forgotten = false;
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      grant: () => Promise.reject(new Error("postgrest 500")),
      forget: () => {
        forgotten = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(forgotten, true);
});

Deno.test("the function itself never throws, even when forget also fails", async () => {
  await revokeGoogleGrantOnDelete(
    "acct-1",
    deps({
      grant: () => Promise.resolve("refresh-token-not-a-secret"),
      revoke: () => Promise.reject(new Error("network unreachable")),
      forget: () => Promise.reject(new Error("postgrest 500")),
    }),
  );
  // Reaching this line at all is the assertion: nothing above propagated.
});
