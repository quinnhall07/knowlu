import { assertEquals } from "@std/assert";
import { rulesHandler } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const PROPOSAL = {
  id: 41, kind: "task", feature: "created_by+title_prefix", value: "zybooks|CS 100 Lab",
  verdict: { effort_hours: "0.5", importance: "2" }, proposed_at: "2026-09-11",
};

function deps(overrides: Record<string, unknown> = {}) {
  return {
    pending: () => Promise.resolve([PROPOSAL]),
    decide: () => Promise.resolve(true),
    ...overrides,
  };
}

// F-6: renamed to what it proves — `PROPOSAL.id` is 41, a deliberately non-trivial value, so this
// fails if the handler ever answered an array position (0) instead of the proposal's own stored id.
Deno.test("GET lists this account's undecided proposals, each carrying its own stored id, not its array position", async () => {
  const reply = await (await rulesHandler(OK, deps())(new Request("http://127.0.0.1/judge-rules"))).json();
  assertEquals(reply.proposals.length, 1);
  assertEquals(reply.proposals[0].id, 41);
});

Deno.test("a decision names the account, so one account cannot decide another's proposal", async () => {
  const seen: unknown[] = [];
  const handler = rulesHandler(OK, deps({
    decide: (account: string, id: number, decision: string) => {
      seen.push([account, id, decision]);
      return Promise.resolve(true);
    },
  }));
  const response = await handler(new Request("http://127.0.0.1/judge-rules", {
    method: "POST", body: JSON.stringify({ id: 41, decision: "approved" }),
  }));
  assertEquals(response.status, 200);
  assertEquals(seen, [["acct-1", 41, "approved"]]);
});

Deno.test("a decision on nothing is a 404, not a silent success", async () => {
  const handler = rulesHandler(OK, deps({ decide: () => Promise.resolve(false) }));
  const response = await handler(new Request("http://127.0.0.1/judge-rules", {
    method: "POST", body: JSON.stringify({ id: 99, decision: "approved" }),
  }));
  assertEquals(response.status, 404);
});

Deno.test("a malformed decision is a 400", async () => {
  const handler = rulesHandler(OK, deps());
  for (const body of ['{"id":"41","decision":"approved"}', '{"id":41,"decision":"maybe"}', "{}"]) {
    const response = await handler(new Request("http://127.0.0.1/judge-rules", { method: "POST", body }));
    assertEquals(response.status, 400, body);
  }
});
