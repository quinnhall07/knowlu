import { assertEquals } from "@std/assert";
import { handle } from "./handler.ts";
import { CASES } from "../../migrations/testers_cases.ts";
import { run } from "../../migrations/testers_model.ts";

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

// L8 S3 (docs/specs/2026-10-18-friends-testers-design.md §3.2, §3.4, §9): the lookup is the view, so
// the fake lookup is testers_model.ts's answer for a case's first account, its days counted from NOW.
const DAY = 86_400_000;
function caseLookup(id: string) {
  const c = CASES.find((x) => x.id === id);
  if (!c) throw new Error(`no case ${id}`);
  const r = run(c).after[0];
  return () =>
    Promise.resolve(
      r && {
        status: r.status,
        plan: r.plan,
        current_period_end: r.periodEnd === null
          ? null
          : new Date(NOW.getTime() + r.periodEnd * DAY).toISOString(),
      },
    );
}

Deno.test("a comped account (P3: the sign-up none row, an open grant) answers active, tester, the grant's end", async () => {
  const res = await handle(get("Bearer good"), { verify, lookup: caseLookup("P3"), now: () => NOW });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    status: "active",
    current_period_end: "2026-10-10T12:00:00.000Z",
    plan: "tester",
    checked_at: "2026-09-10T12:00:00.000Z",
  });
});

Deno.test("an unpaid account (P0), an ended grant (P9) and an expired one (P8) answer none at 200", async () => {
  for (const id of ["P0", "P9", "P8"]) {
    const res = await handle(get("Bearer good"), { verify, lookup: caseLookup(id), now: () => NOW });
    assertEquals(res.status, 200, id);
    assertEquals(await res.json(), {
      status: "none",
      current_period_end: null,
      plan: null,
      checked_at: "2026-09-10T12:00:00.000Z",
    }, id);
  }
});
