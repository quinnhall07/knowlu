import { assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const NOW = new Date("2026-09-10T12:00:00.000Z");
const get = (auth?: string) =>
  new Request("http://127.0.0.1:1/entitlement", { headers: auth ? { authorization: auth } : {} });
const verify = (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null);

Deno.test("an entitled account gets the four keys the device caches, and no others", async () => {
  const res = await handle(get("Bearer good"), {
    verify,
    lookup: () =>
      Promise.resolve({
        plan: "monthly",
        status: "active" as const,
        current_period_end: "2026-10-10T00:00:00+00:00",
      }),
    now: () => NOW,
  });
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(body, {
    status: "active",
    current_period_end: "2026-10-10T00:00:00+00:00",
    plan: "monthly",
    checked_at: "2026-09-10T12:00:00.000Z",
  });
  assertEquals(Object.keys(body).sort(), ["checked_at", "current_period_end", "plan", "status"]);
});

Deno.test("an account with no row is `none`, at 200 — never a 404 the device would read as offline", async () => {
  const res = await handle(get("Bearer good"), {
    verify,
    lookup: () => Promise.resolve(null),
    now: () => NOW,
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    status: "none",
    current_period_end: null,
    plan: null,
    checked_at: "2026-09-10T12:00:00.000Z",
  });
});

Deno.test("no bearer token is 401", async () => {
  const res = await handle(get(), { verify, lookup: () => Promise.resolve(null), now: () => NOW }).catch(
    (e) => e as Response,
  );
  assertEquals(res.status, 401);
});

Deno.test("anything but GET is 405 and never touches the lookup", async () => {
  let looked = false;
  const res = await handle(new Request("http://127.0.0.1:1/entitlement", { method: "POST" }), {
    verify,
    lookup: () => {
      looked = true;
      return Promise.resolve(null);
    },
    now: () => NOW,
  });
  assertEquals(res.status, 405);
  assertEquals(looked, false);
});
