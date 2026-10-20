// Layer ii of docs/specs/2026-10-18-friends-testers-design.md §9 (task S2m): the TS model of §3.2's
// precedence and §5's admin.testers state order, run over every P row of testers_cases.ts. It proves
// the case table self-consistent; it does not prove the SQL (layer iii does, on staging).
import { assert, assertEquals } from "@std/assert";
import { CASES, type Grant } from "./testers_cases.ts";
import { grantState, isPaid, run, viewRow } from "./testers_model.ts";

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
