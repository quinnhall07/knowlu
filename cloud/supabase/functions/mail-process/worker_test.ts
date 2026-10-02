// Email plan T3e (spec §4.7 steps 4–6, D8, D9, D23 as amended by PQ6 (a); tests 11 and 7's expiry half;
// plan P7's sweep). The handler runs whole, with the real model path (`model.ts`), the real charges and the
// real judgment sink over an in-memory `Db` whose RPCs model the SQL: `charge_allowance` with its opening
// rule (keyed on `allowance_opened_at`, PQ6 (a)), `refund_allowance`, `charge_call` with its first-two-days
// doubling, `record_tokens` and the raised `enforce_budget`. S3 is a fake bucket on one listener at
// 127.0.0.1:0; the model is a spy. Every address, course and message here is invented for these tests.
import { assert, assertEquals } from "@std/assert";
import { type JudgeModel, ModelRefused, type ModelRequest } from "../_shared/judge_anthropic.ts";
import type { Db } from "../_shared/judge_db.ts";
import { judgmentSink } from "../_shared/judge_log.ts";
import type { ModelRow } from "../_shared/judge_models.ts";
import { mailUid, parseHeaders } from "../_shared/mime.ts";
import { s3Client } from "../_shared/s3.ts";
import {
  type ClaimsRow,
  handle,
  type Mailbox,
  type ProcessConfig,
  type ProcessDeps,
  type ProcessStore,
  type SweepStore,
  type WorkItem,
} from "./handler.ts";
import { mailCharges, modelPath } from "./model.ts";

const BUCKET = "mail-test-bucket";
const DAY = 86_400_000;
/** Friday 2 October 2026, 15:00 UTC. */
const T0 = Date.UTC(2026, 9, 2, 15, 0, 0);
const MARKER = "zq-marker-7731";
const TOKEN = "test-jobs-token-0000";
const CFG: ProcessConfig = { jobsToken: TOKEN };
const enc = new TextEncoder();
const iso = (ms: number) => new Date(ms).toISOString();
const utcDay = (ms: number) => iso(ms).slice(0, 10);

const ROW: ModelRow = {
  kind: "mail_claim",
  provider: "openrouter",
  model_id: "test/mail-model",
  prompt_version: "mail_claim-1",
  grammar_version: "mail_claim-1",
  max_tokens: 640,
  sampling: { temperature: 0 },
  route: {},
  precision: "fp8",
  usd_per_m_in: 0.14,
  usd_per_m_out: 1.0,
};

// ---------------------------------------------------------------- the database's RPCs, in memory

interface Allowance {
  openedAt: number | null;
  left: number;
  usd: number;
  until: number | null;
}
interface Usage {
  calls: number;
  inTokens: number;
  outTokens: number;
}

/** The SQL of `20261002000200_mail.sql` and its predecessors, as the charges and the sink call it. */
class FakeDb implements Db {
  /** One row per account that set up mail (`mail_accounts`); the allowance starts closed. */
  accounts = new Map<string, Allowance>([["acct-1", { openedAt: null, left: 0, usd: 0, until: null }]]);
  /** `usage_daily`, keyed `account|day|kind`. */
  usage = new Map<string, Usage>();
  /** This month's spend outside `usage_daily`'s mail rows, in dollars (`monthly_spend`'s other kinds). */
  otherSpend = 0;
  judgments: Record<string, unknown>[] = [];
  constructor(readonly clock: { now: number }, readonly events: string[]) {
    // An account judging for a while: `charge_call`'s first-two-days doubling is behind it.
    this.usage.set(`acct-1|${utcDay(clock.now - 30 * DAY)}|task`, { calls: 1, inTokens: 0, outTokens: 0 });
  }
  use(account: string, kind: string): Usage {
    const key = `${account}|${utcDay(this.clock.now)}|${kind}`;
    let u = this.usage.get(key);
    if (u === undefined) this.usage.set(key, u = { calls: 0, inTokens: 0, outTokens: 0 });
    return u;
  }
  select(): Promise<unknown[]> {
    return Promise.reject(new Error("no select in this test"));
  }
  update(): Promise<void> {
    return Promise.reject(new Error("no update in this test"));
  }
  insert(table: string, row: Record<string, unknown>) {
    assertEquals(table, "judgments");
    const written = { ...row, id: `j-${this.judgments.length + 1}` };
    this.judgments.push(written);
    return Promise.resolve(written);
  }
  rpc(fn: string, args: Record<string, unknown>): Promise<unknown> {
    this.events.push(fn);
    const now = this.clock.now;
    const account = String(args.p_account);
    const a = this.accounts.get(account);
    if (fn === "charge_allowance") {
      if (a === undefined) return Promise.resolve(false);
      const opening = a.openedAt === null || a.openedAt <= now - 120 * DAY;
      if (!opening && !(a.until !== null && a.until > now && a.left > 0)) return Promise.resolve(false);
      if (opening) Object.assign(a, { openedAt: now, until: now + 14 * DAY, usd: 1.0, left: 2000 });
      a.left -= 1;
      return Promise.resolve(true);
    }
    if (fn === "refund_allowance") {
      if (a === undefined || a.until === null || a.until <= now || a.left >= 2000) {
        return Promise.resolve(false);
      }
      a.left += 1;
      return Promise.resolve(true);
    }
    if (fn === "charge_call") {
      const days = [...this.usage.keys()].filter((k) => k.startsWith(`${account}|`)).map((k) =>
        k.split("|")[1]
      );
      const first = days.length === 0 ? null : days.sort()[0];
      const u = this.use(account, String(args.p_kind));
      u.calls += 1;
      const cap = Number(args.p_cap);
      return Promise.resolve(
        first === null || first >= utcDay(now - DAY) ? u.calls <= cap * 2 : u.calls <= cap,
      );
    }
    if (fn === "record_tokens") {
      const u = this.use(account, String(args.p_kind));
      u.inTokens += Number(args.p_in);
      u.outTokens += Number(args.p_out);
      return Promise.resolve(null);
    }
    if (fn === "enforce_budget") {
      const raised = a !== undefined && a.until !== null && a.until > now ? a.usd : 0;
      return Promise.resolve(this.spent(account) < Number(args.p_ceiling) + raised);
    }
    return Promise.reject(new Error(`unexpected rpc ${fn}`));
  }
  /** `monthly_spend` for the account this month: the mail rows priced at the pin, plus the rest. */
  spent(account: string): number {
    const month = utcDay(this.clock.now).slice(0, 7);
    let usd = this.otherSpend;
    for (const [key, u] of this.usage) {
      const [acct, day, kind] = key.split("|");
      if (acct === account && day.startsWith(month) && kind === "mail_claim") {
        usd += (u.inTokens * ROW.usd_per_m_in + u.outTokens * ROW.usd_per_m_out) / 1e6;
      }
    }
    return usd;
  }
  calls(kind: string, at = this.clock.now): Usage {
    return this.usage.get(`acct-1|${utcDay(at)}|${kind}`) ?? { calls: 0, inTokens: 0, outTokens: 0 };
  }
}

// ---------------------------------------------------------------- the tables the worker and the sweep touch

interface Batch {
  id: string;
  sentAt: number | null;
  openedAt: number;
  closedAt: number | null;
}
interface Queued extends ClaimsRow {
  queuedAt: number;
  deliveredAt: number | null;
}

class FakeStore implements ProcessStore, SweepStore {
  work: Array<WorkItem & { msgHash: string }> = [];
  /** `mail_seen`, keyed `account|msg_hash`. */
  seen = new Map<string, { seenAt: number }>();
  claims: Queued[] = [];
  counters: Record<string, number> = {};
  batches: Batch[] = [];
  canaries: Array<{ id: string; sentAt: number }> = [];
  boxes: Mailbox[] = [{ id: "addr-1", provider: "google", mailboxHint: "jane@gmail.com" }];
  constructor(readonly clock: { now: number }) {}
  due(nowIso: string, limit: number) {
    const ready = this.work.filter((w) => Date.parse(w.nextAttemptAt) <= Date.parse(nowIso));
    return Promise.resolve(
      ready.sort((a, b) => Date.parse(a.arrivedAt) - Date.parse(b.arrivedAt)).slice(0, limit).map((w) => ({
        ...w,
      })),
    );
  }
  lease(item: WorkItem, untilIso: string) {
    const row = this.work.find((w) => w.id === item.id);
    if (row === undefined || row.nextAttemptAt !== item.nextAttemptAt) return Promise.resolve(false);
    row.nextAttemptAt = untilIso;
    return Promise.resolve(true);
  }
  mailboxes() {
    return Promise.resolve(this.boxes);
  }
  knownCourses() {
    return Promise.resolve(["xx-101", "yy-220"]);
  }
  queue(row: ClaimsRow) {
    this.claims.push({ ...row, queuedAt: this.clock.now, deliveredAt: null });
    return Promise.resolve();
  }
  finish(workId: string) {
    this.work = this.work.filter((w) => w.id !== workId);
    return Promise.resolve();
  }
  count(addressId: string, counter: string) {
    this.counters[`${addressId}:${counter}`] = (this.counters[`${addressId}:${counter}`] ?? 0) + 1;
    return Promise.resolve();
  }
  wait(workId: string, untilIso: string, attempts: number) {
    const row = this.work.find((w) => w.id === workId);
    if (row !== undefined) Object.assign(row, { nextAttemptAt: untilIso, attempts });
    return Promise.resolve();
  }
  /** `expire_mail_work`: the work row, its seen row and the `expired` count, in one transaction. */
  expire(workId: string) {
    const row = this.work.find((w) => w.id === workId);
    if (row === undefined) return Promise.resolve(false);
    this.work = this.work.filter((w) => w.id !== workId);
    this.seen.delete(`${row.accountId}|${row.msgHash}`);
    this.counters[`${row.addressId}:expired`] = (this.counters[`${row.addressId}:expired`] ?? 0) + 1;
    return Promise.resolve(true);
  }
  /** `mail_hand_off`'s seen insert: false for a Message-ID the account already holds. */
  handOff(account: string, msgHash: string): boolean {
    if (this.seen.has(`${account}|${msgHash}`)) return false;
    this.seen.set(`${account}|${msgHash}`, { seenAt: this.clock.now });
    return true;
  }
  expirable(arrivedByIso: string, limit: number) {
    const by = Date.parse(arrivedByIso);
    return Promise.resolve(
      this.work.filter((w) => Date.parse(w.arrivedAt) <= by).slice(0, limit).map((w) => ({
        id: w.id,
        s3Key: w.s3Key,
      })),
    );
  }
  closeBatches(nowIso: string, sentByIso: string, openedByIso: string) {
    let n = 0;
    for (const b of this.batches) {
      const timedOut = (b.sentAt !== null && b.sentAt <= Date.parse(sentByIso)) ||
        b.openedAt <= Date.parse(openedByIso);
      if (b.closedAt === null && timedOut) {
        b.closedAt = Date.parse(nowIso);
        n++;
      }
    }
    return Promise.resolve(n);
  }
  pruneSeen(seenByIso: string) {
    const before = this.seen.size;
    for (const [k, v] of this.seen) if (v.seenAt <= Date.parse(seenByIso)) this.seen.delete(k);
    return Promise.resolve(before - this.seen.size);
  }
  pruneClaims(deliveredByIso: string, queuedByIso: string) {
    const delivered = (c: Queued) => c.deliveredAt !== null && c.deliveredAt <= Date.parse(deliveredByIso);
    const stale = (c: Queued) => c.deliveredAt === null && c.queuedAt <= Date.parse(queuedByIso);
    const counts = {
      delivered: this.claims.filter(delivered).length,
      undelivered: this.claims.filter(stale).length,
    };
    this.claims = this.claims.filter((c) => !delivered(c) && !stale(c));
    return Promise.resolve(counts);
  }
  pruneBatches(closedByIso: string) {
    const before = this.batches.length;
    this.batches = this.batches.filter((b) => b.closedAt === null || b.closedAt > Date.parse(closedByIso));
    return Promise.resolve(before - this.batches.length);
  }
  pruneCanaries(sentByIso: string) {
    const before = this.canaries.length;
    this.canaries = this.canaries.filter((c) => c.sentAt > Date.parse(sentByIso));
    return Promise.resolve(before - this.canaries.length);
  }
}

// ---------------------------------------------------------------- the spy model and the world

type Script = (req: ModelRequest) => Record<string, unknown> | Error;

class SpyModel implements JudgeModel {
  asked: ModelRequest[] = [];
  /** The answer to every call: claims, or an error to throw. A noise claim by default. */
  script: Script = () => ({ claims: [noiseClaim()] });
  constructor(readonly events: string[]) {}
  complete(req: ModelRequest) {
    this.events.push("model");
    this.asked.push(req);
    const out = this.script(req);
    if (out instanceof Error) return Promise.reject(out);
    return Promise.resolve({ json: out, inputTokens: 300, outputTokens: 50 });
  }
}

function noiseClaim() {
  return {
    kind: "noise",
    course: null,
    target: null,
    field: null,
    value: null,
    item: null,
    topic: null,
    evidence: "Lab 5",
    why: "nothing to do",
    confidence: 0.9,
  };
}
function changeClaim(evidence = "Lab 5 is now due Friday at noon") {
  return {
    ...noiseClaim(),
    kind: "change",
    course: "xx-101",
    target: "Lab 5",
    field: "due",
    value: "2026-10-09T12:00",
    evidence,
    why: "the deadline moved",
  };
}

interface World {
  clock: { now: number };
  /** RPC names and `model`, in the order they happened. */
  events: string[];
  db: FakeDb;
  store: FakeStore;
  model: SpyModel;
  bucket: Map<string, Uint8Array>;
  logs: string[];
  deps: ProcessDeps;
  /** Put `raw` under `work/` as intake would: the object, its `mail_work` row and its `mail_seen` row. */
  put(raw: string, o?: Partial<WorkItem>): WorkItem;
  /** One cron call; `{mode: "sweep"}` with `sweep`. */
  run(sweep?: boolean): Promise<Response>;
}

async function withWorld(fn: (w: World) => Promise<void>): Promise<void> {
  const clock = { now: T0 };
  const events: string[] = [];
  const db = new FakeDb(clock, events);
  const store = new FakeStore(clock);
  const model = new SpyModel(events);
  const bucket = new Map<string, Uint8Array>();
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    const key = decodeURIComponent(new URL(req.url).pathname.slice(1));
    const body = new Uint8Array(await req.arrayBuffer());
    if (req.method === "GET") {
      const got = bucket.get(key);
      return got ? new Response(new Uint8Array(got)) : new Response("<Error/>", { status: 404 });
    }
    if (req.method === "PUT") {
      bucket.set(key, body);
      return new Response(null, { status: 200 });
    }
    if (req.method === "DELETE") {
      bucket.delete(key);
      return new Response(null, { status: 204 });
    }
    return new Response("<Error/>", { status: 405 });
  });
  const local = (raw: string, init?: RequestInit) => {
    const u = new URL(raw);
    return fetch(`http://127.0.0.1:${server.addr.port}${u.pathname}${u.search}`, init);
  };
  const s3 = s3Client(
    { bucket: BUCKET, region: "us-east-2", accessKeyId: "AKIDTESTONLY0000000", secretAccessKey: "test-only" },
    { fetch: local, now: () => new Date(clock.now) },
  );
  const now = () => clock.now;
  const w = { clock, events, db, store, model, bucket, logs: [] as string[] } as World;
  w.deps = {
    store,
    s3,
    rules: { lookup: () => Promise.resolve(null) },
    now,
    templates: [],
    model: modelPath({
      store,
      s3,
      now,
      charges: mailCharges(db),
      log: judgmentSink(db),
      pin: () => Promise.resolve({ row: ROW, model }),
    }),
  };
  let n = 0;
  w.put = (raw, o = {}) => {
    const item: WorkItem = {
      id: String(++n),
      accountId: "acct-1",
      addressId: "addr-1",
      batchId: null,
      s3Key: `work/item-${n}`,
      phase: "live",
      authenticity: "aligned",
      arrivedAt: iso(clock.now - 3_600_000 + n * 1000),
      nextAttemptAt: iso(clock.now - 60_000),
      attempts: 0,
      batchOpenedAt: null,
      ...o,
    };
    bucket.set(item.s3Key, enc.encode(raw));
    store.work.push({ ...item, msgHash: `h-${n}` });
    store.handOff(item.accountId, `h-${n}`);
    return item;
  };
  w.run = (sweep = false) =>
    handle(
      new Request("http://127.0.0.1/mail-process", {
        method: "POST",
        headers: { "content-type": "application/json", "x-knowlu-job-token": TOKEN },
        body: sweep ? JSON.stringify({ mode: "sweep" }) : "{}",
      }),
      CFG,
      w.deps,
    );
  const saved = { ...console };
  const capture = (...args: unknown[]) => w.logs.push(args.map(String).join(" "));
  Object.assign(console, { log: capture, info: capture, warn: capture, error: capture, debug: capture });
  try {
    await fn(w);
  } finally {
    Object.assign(console, saved);
    await server.shutdown();
  }
}

let mid = 0;
/** One original from a professor; the marker is in its Message-ID, its Subject and a line of its text no
 * claim quotes. */
function mail(o: { subject?: string; body?: string } = {}): string {
  return [
    `From: Prof Doe <prof@school.test>`,
    "To: jane@gmail.com",
    `Subject: ${o.subject ?? `Lab 5 ${MARKER}`}`,
    "Date: Fri, 02 Oct 2026 09:00:00 -0500",
    `Message-ID: <${MARKER}.${++mid}@origin.test>`,
  ].join("\r\n") + `\r\n\r\n${o.body ?? `Lab 5 is now due Friday at noon.\r\nSee you there. ${MARKER}`}\r\n`;
}
const work = (w: World) => w.store.work.map((r) => r.id);
const objects = (w: World) => [...w.bucket.keys()].filter((k) => k.startsWith("work/")).sort();
const modelCalls = (w: World) => w.events.filter((e) => e === "model").length;
/** The next UTC midnight after `ms`. */
const midnight = (ms: number) =>
  iso(Date.UTC(new Date(ms).getUTCFullYear(), new Date(ms).getUTCMonth(), new Date(ms).getUTCDate() + 1));
/** Fill today's `mail_claim` daily cap (120 calls). */
const fillDailyCap = (w: World) => (w.db.use("acct-1", "mail_claim").calls = 120);
const charged = (w: World) => w.events.filter((e) => e.startsWith("charge_"));
const BACKFILL: Partial<WorkItem> = { phase: "backfill", batchId: "b-1" };
const allowance = (w: World) => w.db.accounts.get("acct-1");

// ---------------------------------------------------------------- test 11: the charge, before the model

Deno.test("a_backfill_item_charges_the_allowance_not_the_daily_cap", async () => {
  await withWorld(async (w) => {
    w.put(mail(), BACKFILL);
    assertEquals((await w.run()).status, 200);
    assertEquals(modelCalls(w), 1);
    assertEquals(charged(w), ["charge_allowance"]);
    assertEquals(w.db.calls("mail_claim").calls, 0);
    // The first backfill charge opened the allowance (PQ6 (a)): 2,000 judgments, $1.00 and 14 days.
    assertEquals(allowance(w), { openedAt: T0, left: 1999, usd: 1.0, until: T0 + 14 * DAY });
    // Live and recover items charge the day's cap, never the allowance.
    w.events.length = 0;
    w.put(mail());
    w.put(mail(), { phase: "recover", batchId: "b-2" });
    await w.run();
    assertEquals(charged(w), ["charge_call", "charge_call"]);
    assertEquals(w.db.calls("mail_claim").calls, 2);
    assertEquals(allowance(w)?.left, 1999);
  });
});

Deno.test("the_2001st_backfill_item_waits_for_the_daily_cap", async () => {
  await withWorld(async (w) => {
    const charges = mailCharges(w.db);
    for (let i = 0; i < 2000; i++) assert(await charges.allowance("acct-1"), `allowance judgment ${i + 1}`);
    assertEquals(allowance(w)?.left, 0);
    w.events.length = 0;
    // The 2,001st falls to the day's cap, and is judged while the cap has room.
    w.put(mail(), BACKFILL);
    await w.run();
    assertEquals(charged(w), ["charge_allowance", "charge_call"]);
    assertEquals(modelCalls(w), 1);
    assertEquals(w.db.calls("mail_claim").calls, 1);
    // With the day's cap spent, the next one waits for it: no call, the next UTC day, its object kept.
    fillDailyCap(w);
    const next = w.put(mail(), BACKFILL);
    await w.run();
    assertEquals(modelCalls(w), 1);
    assertEquals(w.store.work.map((r) => [r.id, r.attempts, r.nextAttemptAt]), [[next.id, 1, midnight(T0)]]);
    assertEquals(objects(w), [next.s3Key]);
  });
});

Deno.test("a_model_failure_on_a_backfill_item_refunds_its_allowance_judgment_and_waits_to_retry", async () => {
  await withWorld(async (w) => {
    w.model.script = () => new Error("upstream 503");
    const item = w.put(mail(), BACKFILL);
    await w.run();
    assertEquals(w.events, ["charge_allowance", "enforce_budget", "model", "refund_allowance"]);
    assertEquals(allowance(w)?.left, 2000, "the judgment the model never answered is given back");
    assertEquals(w.store.work.map((r) => [r.id, r.attempts, r.nextAttemptAt]), [[item.id, 1, midnight(T0)]]);
    assertEquals(objects(w), [item.s3Key]);
    // Never an immediate re-charge: a run later the same day takes nothing and charges nothing.
    w.events.length = 0;
    w.clock.now = T0 + 30 * 60_000;
    await w.run();
    assertEquals(w.events, []);
    // The next UTC day it is tried again, and answered.
    w.model.script = () => ({ claims: [changeClaim()] });
    w.clock.now = Date.parse(midnight(T0)) + 60_000;
    await w.run();
    assertEquals(modelCalls(w), 1);
    assertEquals([work(w), objects(w), w.store.claims.length, allowance(w)?.left], [[], [], 1, 1999]);
    // A refusal, a truncation and a failed validation reached the model: each stays charged, and its item is
    // finished with no claim.
    const outs = [
      new ModelRefused("the model declined"),
      new Error("the reply hit max_tokens (640) for model test/mail-model"),
      { claims: [changeClaim("words this mail never said")] },
    ];
    for (const out of outs) {
      w.model.script = () => out;
      const left = allowance(w)?.left ?? 0;
      w.put(mail(), BACKFILL);
      await w.run();
      assertEquals(allowance(w)?.left, left - 1);
      assertEquals([work(w), objects(w), w.store.claims.length], [[], [], 1]);
    }
    // (The log was cleared after the failure's own refund.)
    assertEquals(
      w.events.filter((e) => e === "refund_allowance").length,
      0,
      "only the model failure refunds",
    );
    assertEquals(w.db.judgments.slice(-3).map((j) => j.cause), ["refused", "truncated", "incomplete"]);
  });
});

Deno.test("charge_and_enforce_budget_run_before_the_spy_model", async () => {
  await withWorld(async (w) => {
    w.put(mail(), BACKFILL);
    await w.run();
    assertEquals(w.events, ["charge_allowance", "enforce_budget", "model", "record_tokens"]);
    w.events.length = 0;
    w.put(mail());
    await w.run();
    assertEquals(w.events, ["charge_call", "enforce_budget", "model", "record_tokens"]);
    // Over budget: the charge and the check, and no call; the item waits as a capped one does.
    w.db.otherSpend = 10;
    w.events.length = 0;
    const over = w.put(mail());
    await w.run();
    assertEquals(w.events, ["charge_call", "enforce_budget"]);
    assertEquals(w.store.work.map((r) => [r.id, r.attempts, r.nextAttemptAt]), [[over.id, 1, midnight(T0)]]);
  });
});

Deno.test("a_capped_item_makes_no_model_call_and_waits_to_the_next_utc_day_at_most_three_times", async () => {
  await withWorld(async (w) => {
    fillDailyCap(w);
    const item = w.put(mail());
    await w.run();
    assertEquals(w.store.work.map((r) => [r.id, r.attempts, r.nextAttemptAt]), [[item.id, 1, midnight(T0)]]);
    // The next UTC day the cap is spent again: a second wait.
    w.clock.now = Date.parse(midnight(T0)) + 60_000;
    fillDailyCap(w);
    await w.run();
    assertEquals(w.store.work.map((r) => [r.id, r.attempts, r.nextAttemptAt]), [[
      item.id,
      2,
      midnight(w.clock.now),
    ]]);
    assertEquals(objects(w), [item.s3Key]);
    // The third attempt is the last: still capped, the item expires, and its seen row goes with it (D8).
    w.clock.now = Date.parse(midnight(w.clock.now)) + 60_000;
    fillDailyCap(w);
    await w.run();
    assertEquals(modelCalls(w), 0);
    assertEquals([work(w), objects(w), w.store.claims], [[], [], []]);
    assertEquals(w.store.seen.has("acct-1|h-1"), false);
    assertEquals(w.store.counters, { "addr-1:expired": 1 });
  });
});

Deno.test("allowance_calls_add_tokens_to_usage_daily_but_no_calls", async () => {
  await withWorld(async (w) => {
    w.put(mail(), BACKFILL);
    await w.run();
    assertEquals(w.db.calls("mail_claim"), { calls: 0, inTokens: 300, outTokens: 50 });
    assert(w.db.spent("acct-1") > 0, "monthly_spend sees the allowance call's cost");
    // The same day's live cap is untouched: a live item is its first call.
    w.put(mail());
    await w.run();
    assertEquals(w.db.calls("mail_claim"), { calls: 1, inTokens: 600, outTokens: 100 });
  });
});

Deno.test("the_raised_ceiling_ends_with_the_allowance", async () => {
  await withWorld(async (w) => {
    // Over the bare $2.00 ceiling, under it plus the allowance's $1.00.
    w.db.otherSpend = 2.5;
    w.put(mail(), BACKFILL);
    w.put(mail());
    await w.run();
    assertEquals(modelCalls(w), 2, "the open allowance raises the account's ceiling for every call");
    // From `allowance_until` on, the ceiling is the bare one: no call, and the item waits.
    w.clock.now = T0 + 14 * DAY + 60_000;
    const late = w.put(mail(), BACKFILL);
    await w.run();
    assertEquals(modelCalls(w), 2);
    assertEquals(w.store.work.map((r) => [r.id, r.attempts]), [[late.id, 1]]);
  });
});

Deno.test("disconnect_and_reconnect_do_not_renew_the_allowance", async () => {
  await withWorld(async (w) => {
    w.put(mail(), BACKFILL);
    await w.run();
    const opened = { openedAt: T0, left: 1999, usd: 1.0, until: T0 + 14 * DAY };
    assertEquals(allowance(w), opened);
    // Disconnect deletes the mailbox and its rows; `mail_accounts` stays. A reconnected mailbox's backfill
    // inside the 14 days gets only what is left.
    w.store.boxes = [{ id: "addr-2", provider: "google", mailboxHint: "jane@gmail.com" }];
    w.clock.now = T0 + 5 * DAY;
    w.put(mail(), { addressId: "addr-2", phase: "backfill", batchId: "b-2" });
    await w.run();
    assertEquals(allowance(w), { ...opened, left: 1998 });
    assertEquals(w.db.calls("mail_claim").calls, 0);
    // After the 14 days, another reconnect's backfill gets none, and never a new allowance: the day's cap.
    w.store.boxes = [{ id: "addr-3", provider: "google", mailboxHint: "jane@gmail.com" }];
    w.clock.now = T0 + 20 * DAY;
    w.events.length = 0;
    w.put(mail(), { addressId: "addr-3", phase: "backfill", batchId: "b-3" });
    await w.run();
    assertEquals(allowance(w), { ...opened, left: 1998 });
    assertEquals(charged(w), ["charge_allowance", "charge_call"]);
    assertEquals(modelCalls(w), 1);
  });
});

Deno.test("a_second_backfill_inside_120_days_of_the_opening_opens_nothing", async () => {
  await withWorld(async (w) => {
    w.put(mail(), BACKFILL);
    await w.run();
    w.clock.now = T0 + 119 * DAY;
    w.events.length = 0;
    w.put(mail(), { phase: "backfill", batchId: "b-2" });
    await w.run();
    assertEquals(allowance(w), { openedAt: T0, left: 1999, usd: 1.0, until: T0 + 14 * DAY });
    assertEquals(charged(w), ["charge_allowance", "charge_call"]);
  });
});

Deno.test("a_backfill_120_days_or_more_after_the_last_opening_opens_a_new_allowance", async () => {
  await withWorld(async (w) => {
    w.put(mail(), BACKFILL);
    await w.run();
    const reopened = T0 + 120 * DAY;
    w.clock.now = reopened;
    w.events.length = 0;
    w.put(mail(), { phase: "backfill", batchId: "b-2" });
    await w.run();
    assertEquals(allowance(w), { openedAt: reopened, left: 1999, usd: 1.0, until: reopened + 14 * DAY });
    assertEquals(charged(w), ["charge_allowance"]);
    assertEquals(w.db.calls("mail_claim").calls, 0);
  });
});

// ---------------------------------------------------------------- test 7's expiry half, and step 6

Deno.test("an_expired_item_loses_its_seen_row_and_a_later_recover_is_accepted", async () => {
  await withWorld(async (w) => {
    // The bucket's expiry took the object before the worker reached it.
    const gone = w.put(mail(), BACKFILL);
    w.bucket.delete(gone.s3Key);
    await w.run();
    assertEquals([work(w), modelCalls(w), w.store.claims], [[], 0, []]);
    assertEquals(w.store.seen.has("acct-1|h-1"), false);
    assertEquals(w.store.counters, { "addr-1:expired": 1 });
    // A later recover of the same message is accepted: intake's hand-off finds no seen row for it.
    assertEquals(w.store.handOff("acct-1", "h-1"), true);
    // The sweep's half: work whose object's lifecycle has run (S3 rounds one day up to the next UTC
    // midnight) expires; work whose object may still be there stays.
    const waiting = { nextAttemptAt: iso(T0 + DAY) };
    const old = w.put(mail(), { ...waiting, arrivedAt: "2026-09-30T15:00:00.000Z" });
    const edge = w.put(mail(), { ...waiting, arrivedAt: "2026-10-01T00:00:00.000Z" });
    const young = w.put(mail(), { ...waiting, arrivedAt: "2026-10-01T00:00:01.000Z" });
    const res = await w.run(true);
    assertEquals(res.status, 200);
    assertEquals((await res.json()).expired, 2);
    assertEquals(work(w), [young.id]);
    assertEquals(objects(w), [young.s3Key]);
    assertEquals([old, edge, young].map((i) => w.store.seen.has(`acct-1|h-${i.id}`)), [false, false, true]);
    assertEquals(w.store.counters, { "addr-1:expired": 3 });
  });
});

Deno.test("a_queued_row_holds_the_claims_and_metadata_and_no_text", async () => {
  await withWorld(async (w) => {
    w.model.script = () => ({ claims: [changeClaim(), noiseClaim()] });
    const raw = mail();
    w.put(raw);
    await w.run();
    assertEquals([work(w), objects(w)], [[], []]);
    assertEquals(w.store.claims.length, 1);
    const { queuedAt: _q, deliveredAt: _d, ...row } = w.store.claims[0];
    const uid = await mailUid(parseHeaders(raw));
    assertEquals(row, {
      accountId: "acct-1",
      addressId: "addr-1",
      batchId: null,
      uid,
      messageDate: "2026-10-02T14:00:00.000Z",
      orderingDate: "2026-10-02T14:00:00.000Z",
      senderName: "Prof Doe",
      senderAddr: "prof@school.test",
      phase: "live",
      authenticity: "aligned",
      claims: [{
        kind: "change",
        course: "xx-101",
        target: "Lab 5",
        field: "due",
        value: "2026-10-09T12:00",
        item: null,
        topic: null,
        evidence: "Lab 5 is now due Friday at noon",
        why: "the deadline moved",
        confidence: 0.9,
        source: "model",
        resolution: null,
      }],
      judgmentId: "j-1",
    });
    assert(!JSON.stringify(row).includes(MARKER), "the queued row carries text past the claim");
    // The judgment row: the mail kind, its own origin, the pin, and nothing of the message.
    const j = w.db.judgments[0];
    assertEquals(
      [j.kind, j.origin, j.tier, j.outcome, j.item_id, j.model, j.prompt_version],
      ["mail_claim", "forward", 3, "answered", uid, ROW.model_id, ROW.prompt_version],
    );
    assert(
      !JSON.stringify(j).includes(MARKER) && !JSON.stringify(j).includes("Lab 5"),
      "a judgment holds text",
    );
    // The call was the pinned row's, with the mail-1 grammar.
    assertEquals([w.model.asked[0].model, w.model.asked[0].maxTokens], [ROW.model_id, ROW.max_tokens]);
    for (const line of w.logs) {
      assert(!line.includes(MARKER) && !line.includes("@"), "a log line holds the message");
    }
  });
});

// ---------------------------------------------------------------- P7: the nightly sweep

Deno.test("the_sweep_closes_timed_out_batches_and_prunes_by_age", async () => {
  await withWorld(async (w) => {
    const ago = (ms: number) => T0 - ms;
    const H = 3_600_000;
    w.store.batches = [
      { id: "sent-3h", sentAt: ago(3 * H), openedAt: ago(4 * H), closedAt: null }, // 2 hours after sent: closes
      { id: "sent-1h", sentAt: ago(1 * H), openedAt: ago(2 * H), closedAt: null }, // still open
      { id: "open-15d", sentAt: null, openedAt: ago(15 * DAY), closedAt: null }, // 14 days after opening: closes
      { id: "open-13d", sentAt: null, openedAt: ago(13 * DAY), closedAt: null }, // still open
      { id: "closed-31d", sentAt: ago(32 * DAY), openedAt: ago(33 * DAY), closedAt: ago(31 * DAY) }, // pruned
      { id: "closed-29d", sentAt: ago(30 * DAY), openedAt: ago(31 * DAY), closedAt: ago(29 * DAY) }, // kept
    ];
    w.store.seen.set("acct-1|old", { seenAt: ago(181 * DAY) });
    w.store.seen.set("acct-1|young", { seenAt: ago(179 * DAY) });
    const claim = (o: Partial<Queued>): Queued => ({
      accountId: "acct-1",
      addressId: "addr-1",
      batchId: null,
      uid: "mail:0",
      messageDate: null,
      orderingDate: iso(T0),
      senderName: null,
      senderAddr: null,
      phase: "live",
      authenticity: "aligned",
      claims: [],
      judgmentId: null,
      queuedAt: T0,
      deliveredAt: null,
      ...o,
    });
    w.store.claims = [
      claim({ uid: "delivered-8d", queuedAt: ago(9 * DAY), deliveredAt: ago(8 * DAY) }),
      claim({ uid: "delivered-6d", queuedAt: ago(9 * DAY), deliveredAt: ago(6 * DAY) }),
      claim({ uid: "undelivered-31d", queuedAt: ago(31 * DAY) }),
      claim({ uid: "undelivered-29d", queuedAt: ago(29 * DAY) }),
    ];
    w.store.canaries = [{ id: "c-31d", sentAt: ago(31 * DAY) }, { id: "c-29d", sentAt: ago(29 * DAY) }];
    const res = await w.run(true);
    assertEquals(res.status, 200);
    assertEquals(await res.json(), {
      ok: true,
      expired: 0,
      closed: 2,
      seen: 1,
      delivered: 1,
      undelivered: 1,
      batches: 1,
      canaries: 1,
    });
    assertEquals(w.store.batches.map((b) => [b.id, b.closedAt]), [
      ["sent-3h", T0],
      ["sent-1h", null],
      ["open-15d", T0],
      ["open-13d", null],
      ["closed-29d", ago(29 * DAY)],
    ]);
    assertEquals([...w.store.seen.keys()], ["acct-1|young"]);
    assertEquals(w.store.claims.map((c) => c.uid), ["delivered-6d", "undelivered-29d"]);
    assertEquals(w.store.canaries.map((c) => c.id), ["c-29d"]);
    // The sweep judges nothing, and its log lines are counts, never a row's contents.
    assertEquals(modelCalls(w), 0);
    for (const line of w.logs) assert(!line.includes("mail:") && !line.includes("acct-1"), line);
  });
});
