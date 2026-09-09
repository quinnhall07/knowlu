import { assertEquals } from "@std/assert";
import { EntitlementRow, isActive, requireActiveEntitlementWith } from "./entitlement.ts";

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
