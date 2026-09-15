import { assert, assertEquals, assertRejects } from "@std/assert";
import { disconnectGrant } from "./disconnect.ts";

function deps(overrides: Record<string, unknown> = {}) {
  const forgotten: string[] = [];
  const base = {
    token: "REFRESH-1" as string | null,
    revoke: () => Promise.resolve(new Response(null, { status: 200 })),
    forget: () => {
      forgotten.push("forgotten");
      return Promise.resolve();
    },
  };
  return { deps: { ...base, ...overrides }, forgotten };
}

Deno.test("revoke resolves before forget is called", async () => {
  const order: string[] = [];
  const { deps: d } = deps({
    revoke: (token: string) => {
      order.push(`revoke:${token}`);
      return Promise.resolve(new Response(null, { status: 200 }));
    },
    forget: () => {
      order.push("forget");
      return Promise.resolve();
    },
  });
  await disconnectGrant(d);
  assertEquals(order, ["revoke:REFRESH-1", "forget"]);
});

Deno.test("a 200 revoke is forgotten", async () => {
  const { deps: d, forgotten } = deps({ revoke: () => Promise.resolve(new Response(null, { status: 200 })) });
  await disconnectGrant(d);
  assertEquals(forgotten, ["forgotten"]);
});

Deno.test("a 400 (Google's invalid_token — already dead) is still forgotten", async () => {
  const { deps: d, forgotten } = deps({
    revoke: () => Promise.resolve(new Response(JSON.stringify({ error: "invalid_token" }), { status: 400 })),
  });
  await disconnectGrant(d);
  assertEquals(forgotten, ["forgotten"]);
});

Deno.test("any other status throws and forget is never called — the grant stays for a retry", async () => {
  const { deps: d, forgotten } = deps({ revoke: () => Promise.resolve(new Response("boom", { status: 503 })) });
  const err = await assertRejects(() => disconnectGrant(d));
  const message = err instanceof Error ? err.message : String(err);
  assert(message.includes("503"), message);
  assertEquals(forgotten, [], "a failed revoke must not destroy the only copy of the token");
});

Deno.test("no token means no revoke call — forget only", async () => {
  let revoked = false;
  const { deps: d, forgotten } = deps({
    token: null,
    revoke: () => {
      revoked = true;
      return Promise.resolve(new Response(null, { status: 200 }));
    },
  });
  await disconnectGrant(d);
  assertEquals(revoked, false);
  assertEquals(forgotten, ["forgotten"]);
});
