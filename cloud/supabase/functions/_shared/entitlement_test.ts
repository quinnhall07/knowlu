import { assert, assertEquals } from "@std/assert";
import { Rest } from "./db.ts";
import {
  depsFrom,
  EntitlementDeps,
  EntitlementRow,
  isActive,
  lookupFrom,
  requireActiveEntitlementWith,
  routeName,
} from "./entitlement.ts";
import { CASES } from "../../migrations/testers_cases.ts";
import { run } from "../../migrations/testers_model.ts";

const row = (status: EntitlementRow["status"]): EntitlementRow => ({
  plan: "monthly",
  status,
  current_period_end: null,
});
const bearer = new Request("http://127.0.0.1:1/", { headers: { authorization: "Bearer good" } });
const verify = (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null);

Deno.test("active and trialing are entitled; nothing else is", () => {
  assertEquals(isActive(row("active")), true);
  assertEquals(isActive(row("trialing")), true);
  assertEquals(isActive(row("past_due")), false);
  assertEquals(isActive(row("canceled")), false);
  assertEquals(isActive(row("none")), false);
  assertEquals(isActive(null), false);
});

Deno.test("an entitled caller gets its account id back", async () => {
  const out = await requireActiveEntitlementWith(bearer, {
    verify,
    lookup: () => Promise.resolve(row("trialing")),
  });
  assertEquals(out, { account_id: "acc-1" });
});

Deno.test("an unentitled caller gets a thrown 402 Response — the contract C2 imports", async () => {
  try {
    await requireActiveEntitlementWith(bearer, { verify, lookup: () => Promise.resolve(row("past_due")) });
    throw new Error("requireActiveEntitlementWith let an unentitled caller through");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 402);
    assertEquals(await e.json(), { error: "this account has no active subscription" });
  }
});

Deno.test("an account with no entitlement row at all is 402, never 500", async () => {
  try {
    await requireActiveEntitlementWith(bearer, { verify, lookup: () => Promise.resolve(null) });
    throw new Error("a missing entitlement row was not refused");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 402);
  }
});

Deno.test("a bad token is 401 and never reaches the lookup", async () => {
  let looked = false;
  const stale = new Request("http://127.0.0.1:1/", { headers: { authorization: "Bearer stale" } });
  try {
    await requireActiveEntitlementWith(stale, {
      verify,
      lookup: () => {
        looked = true;
        return Promise.resolve(row("active"));
      },
    });
    throw new Error("a stale token was accepted");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 401);
  }
  assertEquals(looked, false);
});

// L8 S3 (docs/specs/2026-10-18-friends-testers-design.md §3.2, §4, §9 "Functions").
const DAY = 86_400_000;
const T0 = Date.parse("2026-11-01T12:00:00.000Z");
/** The fake lookup over the case table: testers_model.ts's view answer, in the wire shape. */
const fromCase = (r: ReturnType<typeof run>["after"][number]): EntitlementRow | null =>
  r && {
    status: r.status,
    plan: r.plan,
    current_period_end: r.periodEnd === null ? null : new Date(T0 + r.periodEnd * DAY).toISOString(),
  };

function recordingRest(calls: string[], bodies: string[] = []): Rest {
  const fetch = (input: RequestInfo | URL, init?: RequestInit) => {
    const url = new URL(input instanceof Request ? input.url : String(input));
    calls.push(`${init?.method ?? "GET"} ${url.pathname}${url.search}`);
    if (url.pathname === "/auth/v1/user") return Promise.resolve(Response.json({ id: "acc-1" }));
    if (url.pathname.startsWith("/rest/v1/rpc/")) {
      bodies.push(String(init?.body));
      return Promise.resolve(new Response(null, { status: 204 }));
    }
    return Promise.resolve(Response.json([{ plan: "monthly", status: "active", current_period_end: null }]));
  };
  return { url: "http://127.0.0.1:1", serviceKey: "placeholder", fetch };
}

Deno.test("the column contract: lookupFrom reads the view by account_id, selecting only the view's names", async () => {
  const calls: string[] = [];
  const got = await lookupFrom(recordingRest(calls))("acc-1");
  assertEquals(got, { plan: "monthly", status: "active", current_period_end: null });
  assertEquals(calls.length, 1);
  const url = new URL(`http://h${calls[0].slice("GET ".length)}`);
  assertEquals(url.pathname, "/rest/v1/effective_entitlements");
  assertEquals(url.searchParams.get("account_id"), "eq.acc-1");
  const selected = (url.searchParams.get("select") ?? "").split(",");
  assert(selected.length > 0 && selected.every((c) => ["plan", "status", "current_period_end"].includes(c)));
  const dir = new URL("../../migrations/", import.meta.url);
  const files = [...Deno.readDirSync(dir)].map((e) => e.name).filter((n) => /^\d{14}_testers\.sql$/.test(n));
  assertEquals(files.length, 1, "exactly one testers migration");
  const sql = Deno.readTextFileSync(new URL(files[0], dir));
  const at = sql.search(/create (or replace )?view public\.effective_entitlements as\b/);
  assert(at >= 0, "the testers migration defines public.effective_entitlements");
  const list = sql.slice(at, at + sql.slice(at).search(/\n\s*from\s/));
  const outputs = [...list.matchAll(/\bas ([a-z_]+)\s*(?:,|$)/g)].map((m) => m[1]);
  for (const c of [...selected, "account_id"]) assert(outputs.includes(c), `the view outputs ${c}`);
});

Deno.test("the gate over the case table: a comped account passes, an ended or expired grant is 402", async () => {
  const seen: Record<string, boolean> = {};
  for (const c of CASES) {
    for (const [i, answer] of run(c).after.entries()) {
      if (answer === null) continue;
      const deps = { verify, lookup: () => Promise.resolve(fromCase(answer)) };
      const out = await requireActiveEntitlementWith(bearer, deps).then(() => true, (e) => {
        if (!(e instanceof Response) || e.status !== 402) throw e;
        return false;
      });
      assertEquals(out, isActive(fromCase(answer)), `${c.id}[${i}]`);
      if (i === 0) seen[c.id] = out;
    }
  }
  // The none-row cases first (P0, P3), then the ended (P9) and expired (P8) grants.
  assertEquals([seen.P0, seen.P3, seen.P9, seen.P8, seen.P3x, seen.P2], [
    false,
    true,
    false,
    false,
    true,
    true,
  ]);
});

Deno.test("the route name is the first segment after an optional /functions/v1, in the route pattern", () => {
  const table: Array<[string, string | null]> = [
    ["/functions/v1/judge-task", "judge-task"],
    ["/judge-task", "judge-task"],
    ["/functions/v1/account/sources", "account"],
    ["/account/sources", "account"],
    ["/functions/v1/judge-task/", "judge-task"],
    ["/functions/v1/judge-task?x=1&y=/other", "judge-task"],
    ["/judge-task/?x=1", "judge-task"],
    ["/", null],
    ["/functions/v1", null],
    ["/functions/v1/", null],
    ["/functions/v1/Judge_Task", null],
    ["/functions/v1/1judge", null],
    ["/functions/v1/judge%2Dtask", null],
    [`/functions/v1/${"a".repeat(41)}`, null],
    [`/${"a".repeat(40)}`, "a".repeat(40)],
  ];
  for (const [path, want] of table) assertEquals(routeName(`http://127.0.0.1:1${path}`), want, path);
});

const at = (path: string, token = "good") =>
  new Request(`http://127.0.0.1:1${path}`, { headers: { authorization: `Bearer ${token}` } });

/** Deps that record, in order, every lookup, count and defer. */
function counting(status: EntitlementRow["status"], count?: EntitlementDeps["count"]) {
  const log: string[] = [];
  const deferred: Promise<unknown>[] = [];
  const deps: EntitlementDeps = {
    verify,
    lookup: () => (log.push("lookup"), Promise.resolve(row(status))),
    count: count ?? ((a, r) => (log.push(`count ${a} ${r}`), Promise.resolve())),
    defer: (p) => (log.push("defer"), deferred.push(p)),
  };
  return { deps, log, deferred };
}

Deno.test("the count: once, after an entitled lookup, with the route name, its promise handed to defer", async () => {
  const { deps, log, deferred } = counting("active");
  assertEquals(await requireActiveEntitlementWith(at("/functions/v1/judge-task/x"), deps), {
    account_id: "acc-1",
  });
  assertEquals(log, ["lookup", "count acc-1 judge-task", "defer"]);
  assertEquals(deferred.length, 1);
});

Deno.test("the count: never after a 401 or a 402, and never without a route name", async () => {
  for (
    const [req, status, want] of [
      [at("/judge-task", "stale"), "active", 401],
      [at("/judge-task"), "none", 402],
      [at("/functions/v1/"), "active", 200],
    ] as const
  ) {
    const { deps, log } = counting(status);
    const got = await requireActiveEntitlementWith(req, deps).then(() => 200, (e: Response) => e.status);
    assertEquals(got, want);
    assertEquals(log.filter((l) => l !== "lookup"), [], `${req.url} ${status}`);
  }
});

Deno.test("the count: a rejecting or throwing count neither throws nor refuses; defer gets a settled promise", async () => {
  const counts: EntitlementDeps["count"][] = [() => Promise.reject(new Error("db down")), () => {
    throw new Error("sync");
  }];
  for (const count of counts) {
    const { deps, deferred } = counting("trialing", count);
    assertEquals(await requireActiveEntitlementWith(at("/judge-task"), deps), { account_id: "acc-1" });
    assertEquals(deferred.length, 1);
    assertEquals(await deferred[0], undefined);
  }
});

Deno.test("the count: one that never resolves does not delay the gate", async () => {
  const { deps } = counting("active", () => new Promise<void>(() => {}));
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise((r) => timer = setTimeout(() => r("late"), 2000));
  const got = await Promise.race([requireActiveEntitlementWith(at("/judge-task"), deps), late]);
  clearTimeout(timer);
  assertEquals(got, { account_id: "acc-1" });
});

Deno.test("ROUTE_COUNTS: depsFrom supplies a count only when the secret reads exactly `on`", () => {
  for (const value of [undefined, "", "off", "ON", "On", "true", "1", " on"]) {
    const deps = depsFrom(recordingRest([]), (n) => n === "ROUTE_COUNTS" ? value : undefined, {});
    assertEquals(deps.count, undefined, `ROUTE_COUNTS=${value}`);
  }
  const asked: string[] = [];
  const on = depsFrom(recordingRest([]), (n) => (asked.push(n), n === "ROUTE_COUNTS" ? "on" : undefined), {});
  assertEquals(typeof on.count, "function");
  assertEquals(asked, ["ROUTE_COUNTS"]);
});

Deno.test("ROUTE_COUNTS off: a gated call through depsFrom's deps makes no count_route request", async () => {
  const calls: string[] = [];
  const deferred: Promise<unknown>[] = [];
  const runtime = { EdgeRuntime: { waitUntil: (p: Promise<unknown>) => deferred.push(p) } };
  const deps = depsFrom(recordingRest(calls), () => undefined, runtime);
  assertEquals(await requireActiveEntitlementWith(at("/functions/v1/judge-task"), deps), {
    account_id: "acc-1",
  });
  assertEquals(deferred, []);
  assertEquals(calls.filter((c) => c.includes("/rpc/")), []);
  assertEquals(calls.map((c) => c.split("?")[0]), [
    "GET /auth/v1/user",
    "GET /rest/v1/effective_entitlements",
  ]);
});

Deno.test("ROUTE_COUNTS on: one count_route call by the service role, kept alive by EdgeRuntime.waitUntil", async () => {
  const calls: string[] = [];
  const bodies: string[] = [];
  const deferred: Promise<unknown>[] = [];
  const runtime = { EdgeRuntime: { waitUntil: (p: Promise<unknown>) => deferred.push(p) } };
  const deps = depsFrom(recordingRest(calls, bodies), () => "on", runtime);
  assertEquals(await requireActiveEntitlementWith(at("/functions/v1/judge-task/x"), deps), {
    account_id: "acc-1",
  });
  assertEquals(deferred.length, 1);
  await deferred[0];
  assertEquals(calls.filter((c) => c.includes("/rpc/")), ["POST /rest/v1/rpc/count_route"]);
  assertEquals(bodies.map((b) => JSON.parse(b)), [{ p_account: "acc-1", p_route: "judge-task" }]);
  // With no EdgeRuntime the count still starts; nothing keeps the isolate alive for it.
  const bare = depsFrom(recordingRest([]), () => "on", {});
  assertEquals(typeof bare.count, "function");
  bare.defer?.(Promise.resolve());
});
