// Email plan T4.5 (spec D12, §4.8 step 3, test 13). The resolve runs whole over an in-memory store, a
// scripted model and the real entitlement gate over a fake session, so nothing here leaves the process.
// Every uid, title and id is invented for these tests.
import { assert, assertEquals } from "@std/assert";
import { ModelRefused, ScriptedModel } from "../_shared/judge_anthropic.ts";
import { type EntitlementStatus, requireActiveEntitlementWith } from "../_shared/entitlement.ts";
import type { JudgmentRow } from "../_shared/judge_log.ts";
import type { ModelRow } from "../_shared/judge_models.ts";
import { pullHandler, type PullStore } from "../mail-pull/handler.ts";
import {
  type ClaimRowRef,
  MAX_CANDIDATES,
  MAX_CLAIMS,
  type ResolveCharges,
  resolveHandler,
  type ResolveStore,
} from "./handler.ts";

const ACCT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const OTHER = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const M1 = "11111111-1111-4111-8111-111111111111";
const NOW = Date.UTC(2026, 9, 2, 12, 0, 0);
/** D20's message uid from a small number, and its n-th claim's key. */
const uid = (n: number) => `mail:${n.toString(16).padStart(32, "0")}`;
const key = (n: number, i: number) => `${uid(n)}:${i}`;

const ROW: ModelRow = {
  kind: "mail_resolve",
  provider: "openrouter",
  model_id: "test/resolve-model",
  prompt_version: "mail_resolve-1",
  grammar_version: "mail_resolve-1",
  max_tokens: 640,
  sampling: { temperature: 0 },
  route: {},
  precision: "fp8",
  usd_per_m_in: 0.14,
  usd_per_m_out: 1,
};

const change = (target: string, o: Record<string, unknown> = {}) => ({
  kind: "change",
  course: "CHEM-105",
  target,
  field: "due",
  value: "2026-10-09",
  item: null,
  topic: null,
  evidence: "moved to Friday",
  why: "the lab moved",
  confidence: 0.9,
  source: "model",
  resolution: null,
  ...o,
});
const newItem = (title: string) =>
  change("", {
    kind: "new_item",
    target: null,
    field: null,
    value: null,
    item: { title, due: "2026-10-12", item_kind: "task", effort_hours: 2, importance: 3 },
  });
const info = () => change("", { kind: "course_info", target: null, field: null, value: null, topic: "room" });

interface Row extends ClaimRowRef {
  accountId: string;
}

/** `mail_claims` as the resolve sees it; every call is recorded by name. */
class FakeStore implements ResolveStore {
  rowsHeld: Row[] = [];
  calls: string[] = [];
  /** Runs before every write: another writer, between this call's read and its write. */
  beforeAnswer?: (id: number) => void;
  add(accountId: string, n: number, claims: unknown[]): Row {
    const row = { id: this.rowsHeld.length + 1, accountId, uid: uid(n), claims };
    this.rowsHeld.push(row);
    return row;
  }
  rows(accountId: string, uids: string[]) {
    this.calls.push("rows");
    return Promise.resolve(
      this.rowsHeld.filter((r) => r.accountId === accountId && uids.includes(r.uid))
        .map(({ id, uid, claims }) => ({ id, uid, claims: structuredClone(claims) })),
    );
  }
  answer(accountId: string, id: number, open: number[], claims: unknown[]) {
    this.calls.push("answer");
    this.beforeAnswer?.(id);
    const row = this.rowsHeld.find((r) => r.id === id && r.accountId === accountId);
    if (row === undefined) return Promise.resolve(false);
    const stillOpen = open.every((i) => (row.claims[i] as Record<string, unknown>)?.resolution == null);
    if (!stillOpen) return Promise.resolve(false);
    row.claims = structuredClone(claims);
    return Promise.resolve(true);
  }
  resolution(n: number, i: number): unknown {
    const row = this.rowsHeld.find((r) => r.uid === uid(n) && r.accountId === ACCT);
    return (row?.claims[i] as Record<string, unknown>)?.resolution;
  }
}

const verify = (t: string) =>
  Promise.resolve(
    t === "session" ? { id: ACCT, email: null } : t === "other" ? { id: OTHER, email: null } : null,
  );
const gate = (status: EntitlementStatus) => (req: Request) =>
  requireActiveEntitlementWith(req, {
    verify,
    lookup: () => Promise.resolve({ plan: "monthly", status, current_period_end: null }),
  });

interface World {
  store: FakeStore;
  model: ScriptedModel;
  events: string[];
  judgments: JudgmentRow[];
  budget: boolean;
  cap: boolean;
  post(body: unknown, session?: string, method?: string): Promise<Response>;
}

function world(replies: Array<Record<string, unknown> | Error> = [], status: EntitlementStatus = "active") {
  const w = {
    store: new FakeStore(),
    events: [],
    judgments: [],
    budget: true,
    cap: true,
  } as unknown as World;
  const scripted = new ScriptedModel(replies);
  w.model = scripted;
  const charges: ResolveCharges = {
    withinBudget: () => (w.events.push("budget"), Promise.resolve(w.budget)),
    charge: () => (w.events.push("charge"), Promise.resolve(w.cap)),
    recordTokens: () => (w.events.push("tokens"), Promise.resolve()),
  };
  const handler = resolveHandler({
    entitle: gate(status),
    store: w.store,
    charges,
    log: { write: (row) => (w.judgments.push(row), Promise.resolve(`j-${w.judgments.length}`)) },
    pin: () =>
      Promise.resolve({
        row: ROW,
        model: { complete: (r) => (w.events.push("model"), scripted.complete(r)) },
      }),
    now: () => NOW,
  });
  w.post = (body, session = "session", method = "POST") =>
    handler(
      new Request("http://127.0.0.1/functions/v1/mail-resolve", {
        method,
        headers: { authorization: `Bearer ${session}`, "content-type": "application/json" },
        ...(method === "POST" ? { body: typeof body === "string" ? body : JSON.stringify(body) } : {}),
      }),
    );
  return w;
}

const CANDIDATES = [
  { id: "n-101", title: "Lab 3 report", due: "2026-10-08T23:59" },
  { id: "n-102", title: "Lab 3 prelab", due: null },
  { id: "n-103", title: "Problem set 4", due: "2026-10-12" },
];

async function answers(res: Response): Promise<Array<{ claim: string; target_id: string | null }>> {
  assertEquals(res.status, 200);
  return (await res.json()).answers;
}

// ---------------------------------------------------------------- the plan's four (test 13)

Deno.test("one_call_per_course_batch_stores_each_answer_on_its_claim", async () => {
  const w = world([{
    answers: [
      { claim: key(1, 1), target_id: "n-101" },
      { claim: key(2, 0), target_id: null },
      { claim: key(1, 2), target_id: "n-103" },
    ],
  }]);
  w.store.add(ACCT, 1, [info(), change("Lab 3 writeup"), newItem("Problem set four")]);
  w.store.add(ACCT, 2, [
    change("the essay", { kind: "receipt", field: null, value: null, source: "template" }),
  ]);
  const got = await answers(
    await w.post({ claims: [key(1, 1), key(2, 0), key(1, 2)], candidates: CANDIDATES }),
  );
  // One model call for the whole batch, and the answers in the batch's order.
  assertEquals(w.model.seen.length, 1);
  assertEquals(got, [
    { claim: key(1, 1), target_id: "n-101" },
    { claim: key(2, 0), target_id: null },
    { claim: key(1, 2), target_id: "n-103" },
  ]);
  // The prompt names each claim as the stored claim names it (a change's or receipt's target, a new
  // item's title), and every candidate; nothing else of the claim (no evidence, sender or why).
  const user = w.model.seen[0].user;
  for (const line of ["names: Lab 3 writeup", "names: the essay", "names: Problem set four", "id n-102"]) {
    assert(user.includes(line), line);
  }
  assert(!user.includes("moved to Friday") && !user.includes("the lab moved"));
  // Each answer is stored on its own claim; the course_info claim beside them is left as it was.
  assertEquals(w.store.resolution(1, 1), { target_id: "n-101" });
  assertEquals(w.store.resolution(1, 2), { target_id: "n-103" });
  assertEquals(w.store.resolution(2, 0), { target_id: null });
  assertEquals(w.store.resolution(1, 0), null);
  // Every other field of a stored claim is unchanged.
  const stored = w.store.rowsHeld[0].claims[1] as Record<string, unknown>;
  assertEquals<unknown>({ ...stored, resolution: null }, change("Lab 3 writeup"));
  // One judgment row: the mail kind, its own origin, the pin, counts only.
  assertEquals(w.judgments.length, 1);
  const j = w.judgments[0];
  assertEquals([j.kind, j.origin, j.tier, j.outcome, j.model], [
    "mail_resolve",
    "forward",
    3,
    "answered",
    ROW.model_id,
  ]);
  assertEquals(j.fields, { claims: "3", named: "2" });
  assert(!JSON.stringify(j).includes("Lab 3"));
});

/** `mail-pull`'s store over the same rows: one live mailbox, every row undelivered. */
function pullOver(store: FakeStore): PullStore {
  const head = (r: Row) => ({
    id: r.id,
    addressId: M1,
    batchId: null,
    uid: r.uid,
    orderingDate: new Date(NOW - 60_000).toISOString(),
    queuedAt: new Date(NOW - 60_000).toISOString(),
  });
  const mailbox = {
    id: M1,
    provider: "m365",
    status: "active",
    reenabledAt: null,
    lapsedSince: null,
    lastReceivedAt: null,
    unbound: 0,
    unauthenticated: 0,
    expired: 0,
  };
  return {
    deliver: () => Promise.resolve(),
    mailboxes: () => Promise.resolve([mailbox]),
    batches: () => Promise.resolve([]),
    pendingWork: () => Promise.resolve([]),
    closeBatch: () => Promise.resolve(true),
    undelivered: () => Promise.resolve(store.rowsHeld.map(head)),
    claims: () =>
      Promise.resolve(store.rowsHeld.map((r) => ({
        ...head(r),
        messageDate: null,
        senderName: null,
        senderAddr: null,
        phase: "live" as const,
        authenticity: "aligned" as const,
        claims: r.claims,
        judgmentId: null,
      }))),
  };
}

Deno.test("a_stored_answer_is_replayed_on_a_re_pull_with_no_model_call", async () => {
  const w = world([{ answers: [{ claim: key(1, 0), target_id: "n-102" }] }]);
  w.store.add(ACCT, 1, [change("prelab")]);
  const batch = { claims: [key(1, 0)], candidates: CANDIDATES };
  assertEquals(await answers(await w.post(batch)), [{ claim: key(1, 0), target_id: "n-102" }]);
  assertEquals(w.model.seen.length, 1);
  // The re-pull: the claim comes back to the device carrying its stored answer.
  const pull = pullHandler({ entitle: gate("active"), store: pullOver(w.store), now: () => NOW });
  const pulled = await pull(
    new Request("http://127.0.0.1/functions/v1/mail-pull", {
      method: "POST",
      headers: { authorization: "Bearer session" },
      body: "{}",
    }),
  );
  assertEquals(pulled.status, 200);
  assertEquals((await pulled.json()).messages[0].claims[0].resolution, { target_id: "n-102" });
  // Asked again (the device lost the answer before it acked): the same answer, no call, no charge.
  w.events.length = 0;
  assertEquals(await answers(await w.post(batch)), [{ claim: key(1, 0), target_id: "n-102" }]);
  assertEquals(w.model.seen.length, 1);
  assertEquals(w.events, []);
  assertEquals(w.judgments.length, 1);
  // A stored none replays as none too, and a batch mixing stored and new claims asks only the new.
  const v = world([{ answers: [{ claim: key(4, 0), target_id: "n-103" }] }]);
  v.store.add(ACCT, 3, [change("essay", { resolution: { target_id: null } })]);
  v.store.add(ACCT, 4, [change("set four")]);
  assertEquals(await answers(await v.post({ claims: [key(3, 0), key(4, 0)], candidates: CANDIDATES })), [
    { claim: key(3, 0), target_id: null },
    { claim: key(4, 0), target_id: "n-103" },
  ]);
  assert(!v.model.seen[0].user.includes(key(3, 0)));
  assert(v.model.seen[0].user.includes(key(4, 0)));
});

Deno.test("an_id_outside_the_batch_reads_none", async () => {
  const w = world([{
    answers: [
      { claim: key(1, 0), target_id: "n-999" },
      { claim: key(1, 1), target_id: "n-101" },
      { claim: key(1, 1), target_id: "n-102" },
      { claim: key(7, 0), target_id: "n-101" },
    ],
  }]);
  w.store.add(ACCT, 1, [change("lab"), change("Lab 3")]);
  // The other account's claim, and a course_info claim, are never asked about and never written.
  const theirs = w.store.add(OTHER, 7, [change("Lab 3")]);
  w.store.add(ACCT, 8, [info()]);
  const got = await answers(
    await w.post({ claims: [key(1, 0), key(1, 1), key(7, 0), key(8, 0), key(9, 0)], candidates: CANDIDATES }),
  );
  assertEquals(got, [
    { claim: key(1, 0), target_id: null }, // an id the batch never offered
    { claim: key(1, 1), target_id: null }, // answered two ways
    { claim: key(7, 0), target_id: null }, // not this account's
    { claim: key(8, 0), target_id: null }, // nothing to resolve
    { claim: key(9, 0), target_id: null }, // no such claim
  ]);
  const user = w.model.seen[0].user;
  assert(user.includes(key(1, 0)) && user.includes(key(1, 1)));
  for (const k of [key(7, 0), key(8, 0), key(9, 0)]) assert(!user.includes(k), k);
  // The model's nones are stored (its answer is data); nothing else was written.
  assertEquals(w.store.resolution(1, 0), { target_id: null });
  assertEquals(w.store.resolution(1, 1), { target_id: null });
  assertEquals(w.store.resolution(8, 0), null);
  assertEquals((theirs.claims[0] as Record<string, unknown>).resolution, null);
  assertEquals(w.judgments[0].fields, { claims: "2", named: "0" });
});

Deno.test("a_resolve_call_is_charged_before_the_model", async () => {
  const reply = { answers: [{ claim: key(1, 0), target_id: "n-101" }] };
  const batch = { claims: [key(1, 0)], candidates: CANDIDATES };
  const w = world([reply]);
  w.store.add(ACCT, 1, [change("Lab 3")]);
  await answers(await w.post(batch));
  assertEquals(w.events, ["budget", "charge", "model", "tokens"]);

  // The day's mail_resolve calls spent: no call, nothing stored, a capped row, a named 429.
  const capped = world([reply]);
  capped.store.add(ACCT, 1, [change("Lab 3")]);
  capped.cap = false;
  const res = await capped.post(batch);
  assertEquals(res.status, 429);
  assertEquals((await res.json()).error, "capped");
  assertEquals(capped.events, ["budget", "charge"]);
  assertEquals(capped.model.seen.length, 0);
  assertEquals(capped.store.resolution(1, 0), null);
  assertEquals([capped.judgments[0].tier, capped.judgments[0].outcome], [0, "capped"]);

  // Over the monthly budget: not even charged.
  const over = world([reply]);
  over.store.add(ACCT, 1, [change("Lab 3")]);
  over.budget = false;
  const res2 = await over.post(batch);
  assertEquals(res2.status, 429);
  assertEquals((await res2.json()).error, "over budget");
  assertEquals(over.events, ["budget"]);
  assertEquals(over.model.seen.length, 0);
});

// ---------------------------------------------------------------- plan pins beside test 13

Deno.test("a_failed_model_call_stores_nothing_and_answers_its_cause_by_name", async () => {
  const cases: Array<[Error, string]> = [
    [new Error("upstream 503"), "model failed"],
    [new ModelRefused("refusal"), "refused"],
    [new Error("stop_reason max_tokens"), "truncated"],
  ];
  for (const [error, cause] of cases) {
    const w = world([error]);
    w.store.add(ACCT, 1, [change("Lab 3")]);
    const res = await w.post({ claims: [key(1, 0)], candidates: CANDIDATES });
    assertEquals(res.status, 502, cause);
    assertEquals((await res.json()).error, cause);
    // Never a stored none: only an answer the model gave is data.
    assertEquals(w.store.resolution(1, 0), null, cause);
    assertEquals(w.store.calls, ["rows"], cause);
    assertEquals([w.judgments[0].outcome, w.judgments[0].cause], ["low confidence", cause]);
    assert(!JSON.stringify(w.judgments[0]).includes("upstream"));
  }
});

Deno.test("a_batch_with_nothing_to_ask_makes_no_call_and_no_charge", async () => {
  const w = world([]);
  w.store.add(ACCT, 1, [change("Lab 3"), info()]);
  // No candidate: every claim reads none, unstored, uncharged.
  assertEquals(await answers(await w.post({ claims: [key(1, 0)], candidates: [] })), [
    { claim: key(1, 0), target_id: null },
  ]);
  // Only claims with nothing to resolve: the same.
  assertEquals(await answers(await w.post({ claims: [key(1, 1)], candidates: CANDIDATES })), [
    { claim: key(1, 1), target_id: null },
  ]);
  assertEquals(w.events, []);
  assertEquals(w.judgments, []);
  assertEquals(w.store.resolution(1, 0), null);
});

Deno.test("an_answer_stored_by_another_call_first_wins", async () => {
  const w = world([{
    answers: [{ claim: key(1, 0), target_id: "n-101" }, { claim: key(1, 1), target_id: "n-103" }],
  }]);
  const row = w.store.add(ACCT, 1, [change("Lab 3"), change("set four")]);
  // Between this call's read and its write, another call stores claim 0's answer.
  let once = true;
  w.store.beforeAnswer = () => {
    if (!once) return;
    once = false;
    (row.claims[0] as Record<string, unknown>).resolution = { target_id: "n-102" };
  };
  const got = await answers(await w.post({ claims: [key(1, 0), key(1, 1)], candidates: CANDIDATES }));
  assertEquals(got, [{ claim: key(1, 0), target_id: "n-102" }, { claim: key(1, 1), target_id: "n-103" }]);
  assertEquals(w.store.resolution(1, 0), { target_id: "n-102" });
  assertEquals(w.store.resolution(1, 1), { target_id: "n-103" });
});

Deno.test("the_resolve_needs_a_session_an_entitlement_post_and_a_bounded_batch", async () => {
  const lapsed = world([], "canceled");
  lapsed.store.add(ACCT, 1, [change("Lab 3")]);
  const batch = { claims: [key(1, 0)], candidates: CANDIDATES };
  assertEquals((await lapsed.post(batch)).status, 402);
  assertEquals((await lapsed.post(batch, "nobody")).status, 401);
  assertEquals(lapsed.store.calls, []);
  const w = world([]);
  w.store.add(ACCT, 1, [change("Lab 3")]);
  const get = await w.post(undefined, "session", "GET");
  assertEquals(get.status, 405);
  await get.body?.cancel();
  // The bounds: 15 answers of about 30 tokens fit the pin's 640; a course's active items fit 200.
  assertEquals([MAX_CLAIMS, MAX_CANDIDATES], [15, 200]);
  const many = (n: number, f: (i: number) => unknown) => Array.from({ length: n }, (_, i) => f(i));
  const refused: unknown[] = [
    "{not json",
    [],
    {},
    { claims: [], candidates: CANDIDATES },
    { claims: [key(1, 0)] },
    { claims: [key(1, 0)], candidates: CANDIDATES, course: "CHEM-105" },
    { claims: ["mail:abc:0"], candidates: CANDIDATES },
    { claims: [`${uid(1)}:5`], candidates: CANDIDATES },
    { claims: [key(1, 0), key(1, 0)], candidates: CANDIDATES },
    { claims: many(MAX_CLAIMS + 1, (i) => key(i + 1, 0)), candidates: CANDIDATES },
    {
      claims: [key(1, 0)],
      candidates: many(MAX_CANDIDATES + 1, (i) => ({ id: `n-${i}`, title: "t", due: null })),
    },
    { claims: [key(1, 0)], candidates: [{ id: "", title: "t", due: null }] },
    {
      claims: [key(1, 0)],
      candidates: [{ id: "n-1", title: "t", due: null }, { id: "n-1", title: "u", due: null }],
    },
    { claims: [key(1, 0)], candidates: [{ id: "n-1\nn-2", title: "t", due: null }] },
    { claims: [key(1, 0)], candidates: [{ id: "n-1", title: 7, due: null }] },
    { claims: [key(1, 0)], candidates: [{ id: "n-1", title: "t", due: 20261009 }] },
    { claims: [key(1, 0)], candidates: [{ id: "n-1", title: "t", due: null, body: "x" }] },
    { claims: [key(1, 0)], candidates: [{ id: "n-1", title: "t".repeat(501), due: null }] },
  ];
  for (const body of refused) {
    const res = await w.post(body);
    assertEquals(res.status, 400, JSON.stringify(body).slice(0, 120));
    await res.body?.cancel();
  }
  const huge = await w.post({ claims: [key(1, 0)], candidates: CANDIDATES, pad: "x".repeat(300_000) });
  assertEquals(huge.status, 413);
  await huge.body?.cancel();
  // Nothing was read, charged or asked for any of them.
  assertEquals(w.store.calls, []);
  assertEquals(w.events, []);
});
