// Layer ii of docs/specs/2026-10-18-friends-testers-design.md §9 (task S2m): the TS model of §3.2's
// precedence and §5's admin.testers state order, run over every P row of testers_cases.ts. It proves
// the case table self-consistent; it does not prove the SQL (layer iii does, on staging).
import { assert, assertEquals } from "@std/assert";
import {
  CASES,
  EXTEND_NOT_FUTURE,
  EXTEND_TOO_FAR,
  type Grant,
  GRANT_NOT_FUTURE,
  GRANT_TOO_FAR,
  GUARD_CASES,
  type TesterCase,
} from "./testers_cases.ts";
import { grantState, isPaid, run, testersRow, viewRow } from "./testers_model.ts";

const P = CASES.filter((c) => c.id.startsWith("P"));
const NONE = { status: "none", plan: null, periodEnd: null } as const;

Deno.test("S2m: every P row's answers, before and after its calls, are the model's", () => {
  assert(["P0", "P0x", "P3"].every((id) => P.some((c) => c.id === id)), "the loop meets P0, P0x and P3");
  for (const c of P) {
    const out = run(c);
    assertEquals(out.before, c.expectBefore ?? c.expect, `${c.id} before its calls`);
    assertEquals(out.after, c.expect, `${c.id} at the end`);
  }
});

Deno.test("S2m: every call is refused exactly where the table says, naming what it says", () => {
  for (const c of P) {
    const out = run(c);
    (c.calls ?? []).forEach((call, i) => {
      const got = out.refusals[i];
      if (call.refused === undefined) assertEquals(got, null, `${c.id} call ${i} succeeds`);
      else assert(got !== null, `${c.id} call ${i} is refused`);
      if (typeof call.refused === "string") assert(got?.includes(call.refused), `${c.id} call ${i}: ${got}`);
    });
  }
  const p16 = run(P.find((c) => c.id === "P16")!);
  assertEquals(p16.grants, [], "P16: a refused grant writes no grant row");
  const p15 = run(P.find((c) => c.id === "P15")!);
  assertEquals(p15.grants, [{ account: 0, endsIn: 30 }], "P15: the first's grant survives the second's deletion");
});

Deno.test("S2m: a grant is live exactly when it entitles its account, unless a paid row wins (§5)", () => {
  for (const c of P) {
    const out = run(c);
    for (const [answers, grants] of [[out.before, out.grantsBefore], [out.after, out.grants]] as const) {
      answers.forEach((answer, i) => {
        const live = grants.filter((g) => g.account === i && grantState(g) === "live");
        const row = c.accounts[i].row;
        const tester = answer?.plan === "tester";
        assertEquals(tester, live.length > 0 && !isPaid(row), `${c.id} account ${i}: tester iff live and unpaid`);
        if (live.length > 0 && isPaid(row)) assertEquals(answer, row, `${c.id}: paid wins, column for column`);
      });
    }
  }
});

Deno.test("S2m: §9's named states: P2 live and paid, P8 expired, P9 ended, P13 expired then live", () => {
  const states = (id: string, when: "grantsBefore" | "grants") =>
    run(P.find((c) => c.id === id)!)[when].map((g) => grantState(g));
  assertEquals(states("P2", "grants"), ["live"]);
  assert(isPaid(P.find((c) => c.id === "P2")!.accounts[0].row), "P2's paid flag is set");
  assertEquals(states("P8", "grants"), ["expired"]);
  assertEquals(states("P9", "grants"), ["ended"]);
  assertEquals(states("P13", "grantsBefore"), ["expired", "ended"]);
  assertEquals(states("P13", "grants"), ["live", "ended"]);
});

Deno.test("S2m: §5's order and §3.2's edges: ended before expired, ends_at now is expired, trialing wins", () => {
  const g = (endsIn: number, endedAgo?: number): Grant => ({ account: 0, endsIn, endedAgo });
  assertEquals(grantState(g(-5, 1)), "ended", "ended early and past ends_at reads ended");
  assertEquals(grantState(g(0)), "expired", "now() >= ends_at at the boundary");
  assertEquals(viewRow({ ...NONE }, [g(0)]), NONE, "now() < ends_at is strict");
  const trialing = { status: "trialing", plan: "monthly", periodEnd: 7 } as const;
  assertEquals(viewRow({ ...trialing }, [g(30)]), trialing, "trialing is paid and wins");
  assertEquals(viewRow(null, []), NONE, "no row, no grant: the coalesce answers none");
});

Deno.test("S2m: comp_extend refuses as the SQL does, the date before the open grant", () => {
  const extend = (grants: Grant[], endsIn: number) =>
    run({
      id: "X",
      given: "model only",
      accounts: [{ email: "x-0@example.invalid", row: { ...NONE } }],
      grants,
      calls: [{ fn: "comp_extend", account: 0, endsIn }],
      expect: [NONE],
    }).refusals[0];
  assertEquals(extend([], -1), "the new ends_at is not in the future", "no open grant and a past date: the date");
  assertEquals(extend([{ account: 0, endsIn: 30, endedAgo: 1 }], 0), "the new ends_at is not in the future");
  assertEquals(extend([], 30), "no open grant for this account", "no open grant, a future date");
  assertEquals(extend([{ account: 0, endsIn: 30 }], 45), null);
});

Deno.test("S2m: the model holds no Deno.test, so S3's fake lookup can import it without a suite", async () => {
  const text = await Deno.readTextFile(new URL("testers_model.ts", import.meta.url));
  assert(!/Deno\s*\.\s*test\b/.test(text), "testers_model.ts registers no test");
});

// The S6 follow-up (2026-10-20, minors 2 and 3): the end-date guards on comp_grant and comp_extend,
// and admin.testers' two new columns, over GUARD_CASES.
const one = (calls: TesterCase["calls"], grants: Grant[] = []) =>
  run({ id: "X", given: "model only", accounts: [{ email: "x-0@example.invalid", row: { ...NONE } }], grants, calls, expect: [NONE] });

Deno.test("S6 follow-up: every G row's answers, before and after its calls, and its refusals are the model's", () => {
  assertEquals(GUARD_CASES.map((c) => c.id), ["G1", "G2", "G3", "G4", "G5", "G6", "G7", "G8", "G9", "G10"]);
  for (const c of GUARD_CASES) {
    const out = run(c);
    assertEquals(out.before, c.expectBefore ?? c.expect, `${c.id} before its calls`);
    assertEquals(out.after, c.expect, `${c.id} at the end`);
    (c.calls ?? []).forEach((call, i) => {
      assertEquals(out.refusals[i], call.refused ?? null, `${c.id} call ${i}: the refusal, word for word`);
    });
  }
  assertEquals(run(GUARD_CASES.find((c) => c.id === "G3")!).grants, [], "G3: a refused grant writes no grant row");
});

Deno.test("S6 follow-up: comp_grant refuses a date not after now() or past a year, the date before the account", () => {
  const grant = (endsIn: number, grants: Grant[] = []) => one([{ fn: "comp_grant", account: 0, endsIn }], grants).refusals[0];
  assertEquals(grant(-1), GRANT_NOT_FUTURE);
  assertEquals(grant(0), GRANT_NOT_FUTURE, "now() itself is not after now()");
  assertEquals(grant(367), GRANT_TOO_FAR);
  assertEquals(grant(1), null);
  assertEquals(grant(365), null, "a year less the calendar's slack is inside");
  assertEquals(grant(-1, [{ account: 0, endsIn: 30 }]), GRANT_NOT_FUTURE, "the date before the open grant");
  assertEquals(grant(367, [{ account: 0, endsIn: 30 }]), GRANT_TOO_FAR, "the ceiling before the open grant");
  assertEquals(grant(60, [{ account: 0, endsIn: 30 }]), "an open grant exists for this account; use admin.comp_extend");
  assertEquals(
    run({ id: "X", given: "model only", accounts: [], grants: [], calls: [{ fn: "comp_grant", account: "unknown", endsIn: 367 }], expect: [] })
      .refusals[0],
    GRANT_TOO_FAR,
    "the ceiling before the foreign key",
  );
});

Deno.test("S6 follow-up: comp_extend's ceiling comes after its past-date check and before the open grant", () => {
  const extend = (endsIn: number, grants: Grant[] = []) => one([{ fn: "comp_extend", account: 0, endsIn }], grants).refusals[0];
  assertEquals(extend(-1), EXTEND_NOT_FUTURE);
  assertEquals(extend(367), EXTEND_TOO_FAR, "no open grant and a date past a year: the date");
  assertEquals(extend(367, [{ account: 0, endsIn: 30 }]), EXTEND_TOO_FAR);
  assertEquals(extend(365, [{ account: 0, endsIn: 30 }]), null);
  assertEquals(extend(365), "no open grant for this account");
});

Deno.test("S6 follow-up: no call in any case table is 366 days out, where the calendar decides", () => {
  for (const c of [...CASES, ...GUARD_CASES]) {
    for (const call of c.calls ?? []) assert(call.endsIn !== 366, `${c.id}: 366 days is a year on some dates`);
  }
});

Deno.test("S6 follow-up: admin.testers' entitlement_status and has_subscription, beside state and paid", () => {
  const rows = GUARD_CASES.filter((c) => c.testers !== undefined);
  assertEquals(rows.map((c) => c.id), ["G6", "G7", "G8", "G9", "G10"]);
  for (const c of rows) {
    const out = run(c);
    assertEquals(out.grants.map((g) => testersRow(c.accounts[g.account], g)), c.testers, c.id);
  }
  const g6 = testersRow(GUARD_CASES.find((c) => c.id === "G6")!.accounts[0], { account: 0, endsIn: 30 });
  assertEquals([g6.paid, g6.entitlementStatus, g6.hasSubscription], [false, "past_due", true], "dunned, not paid");
  const orphan = testersRow({ email: "x-0@example.invalid", row: null, subscription: true }, { account: 0, endsIn: 30 });
  assertEquals([orphan.entitlementStatus, orphan.hasSubscription], ["none", false], "no row: none and no subscription");
});
