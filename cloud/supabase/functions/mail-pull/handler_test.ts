// Email plan T4.4 (spec §4.8 step 1, D10, §4.10's *Shown*; test 12). The pull runs whole over an
// in-memory store and the real entitlement gate over a fake session and a fake `entitlements` row, so
// nothing here leaves the process. Every uid, sender and address is invented for these tests.
import { assert, assertEquals } from "@std/assert";
import { type EntitlementStatus, requireActiveEntitlementWith } from "../_shared/entitlement.ts";
import { parseHeaders } from "../_shared/mime.ts";
import { claimsRow, type WorkItem } from "../mail-process/handler.ts";
import {
  ACK_CAP,
  type ClaimHead,
  type ClaimRow,
  PAGE,
  type PullBatch,
  pullHandler,
  type PullMailbox,
  type PullStore,
} from "./handler.ts";

const ACCT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const OTHER = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const M1 = "11111111-1111-4111-8111-111111111111";
const M2 = "22222222-2222-4222-8222-222222222222";
const M3 = "33333333-3333-4333-8333-333333333333";
const MO = "44444444-4444-4444-8444-444444444444"; // the other account's
const B1 = "55555555-5555-4555-8555-555555555555";
const B2 = "66666666-6666-4666-8666-666666666666";
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const NOW = Date.UTC(2026, 9, 2, 12, 0, 0);
const iso = (ms: number) => new Date(ms).toISOString();
/** D20's uid shape, from a small number: `mail:` and 32 hex. */
const uid = (n: number) => `mail:${n.toString(16).padStart(32, "0")}`;

interface Row extends ClaimRow {
  accountId: string;
  deliveredAt: string | null;
}
interface Box extends PullMailbox {
  accountId: string;
}

/** The database as `mail-pull` sees it; every call is recorded by name. */
class FakeStore implements PullStore {
  boxes: Box[] = [];
  batchRows: PullBatch[] = [];
  work: { accountId: string; batchId: string }[] = [];
  rows: Row[] = [];
  calls: string[] = [];
  delivered: string[][] = [];
  private n = 0;

  deliver(accountId: string, uids: string[], atIso: string) {
    this.calls.push("deliver");
    this.delivered.push([...uids]);
    for (const r of this.rows) {
      if (r.accountId === accountId && uids.includes(r.uid) && r.deliveredAt === null) r.deliveredAt = atIso;
    }
    return Promise.resolve();
  }
  mailboxes(accountId: string) {
    this.calls.push("mailboxes");
    return Promise.resolve(
      this.boxes.filter((b) => b.accountId === accountId).map(({ accountId: _, ...b }) => ({ ...b })),
    );
  }
  batches(addressIds: string[]) {
    this.calls.push("batches");
    return Promise.resolve(
      this.batchRows.filter((b) => addressIds.includes(b.addressId)).map((b) => ({ ...b })),
    );
  }
  pendingWork(accountId: string, batchIds: string[]) {
    this.calls.push("pendingWork");
    return Promise.resolve(
      batchIds.filter((b) => this.work.some((w) => w.accountId === accountId && w.batchId === b)),
    );
  }
  closeBatch(id: string, atIso: string) {
    this.calls.push("closeBatch");
    const b = this.batchRows.find((x) => x.id === id);
    if (b === undefined || b.closedAt !== null) return Promise.resolve(false);
    b.closedAt = atIso;
    return Promise.resolve(true);
  }
  undelivered(accountId: string): Promise<ClaimHead[]> {
    this.calls.push("undelivered");
    return Promise.resolve(
      this.rows.filter((r) => r.accountId === accountId && r.deliveredAt === null)
        .sort((a, b) => a.id - b.id)
        .map((r) => ({
          id: r.id,
          addressId: r.addressId,
          batchId: r.batchId,
          uid: r.uid,
          orderingDate: r.orderingDate,
          queuedAt: r.queuedAt,
        })),
    );
  }
  claims(accountId: string, ids: number[]) {
    this.calls.push("claims");
    return Promise.resolve(
      this.rows.filter((r) => r.accountId === accountId && ids.includes(r.id))
        .map(({ accountId: _a, deliveredAt: _d, ...r }) => ({ ...r })),
    );
  }

  /** One queued `mail_claims` row; its id is the insertion order. */
  add(o: {
    n: number;
    ordering: number;
    queued?: number;
    address?: string;
    account?: string;
    batch?: string | null;
    phase?: ClaimRow["phase"];
    messageDate?: string | null;
  }): string {
    const batch = o.batch ?? null;
    this.rows.push({
      id: ++this.n,
      accountId: o.account ?? ACCT,
      addressId: o.address ?? M1,
      batchId: batch,
      uid: uid(o.n),
      messageDate: o.messageDate === undefined ? iso(o.ordering) : o.messageDate,
      orderingDate: iso(o.ordering),
      senderName: "Course Notices",
      senderAddr: "notices@lms.test",
      phase: o.phase ?? (batch === null ? "live" : "backfill"),
      authenticity: "aligned",
      claims: [{ kind: "noise-free", n: o.n }],
      judgmentId: null,
      queuedAt: iso(o.queued ?? o.ordering),
      deliveredAt: null,
    });
    return uid(o.n);
  }
  /** A batch on `address`, opened at `opened`; open and unsent unless `o` says otherwise. */
  batch(id: string, opened: number, o: Partial<PullBatch> = {}): PullBatch {
    const b: PullBatch = {
      id,
      addressId: M1,
      phase: "backfill",
      sent: null,
      received: 0,
      openedAt: iso(opened),
      sentAt: null,
      closedAt: null,
      ...o,
    };
    this.batchRows.push(b);
    return b;
  }
}

const box = (id: string, o: Partial<Box> = {}): Box => ({
  id,
  accountId: ACCT,
  provider: "m365",
  status: "active",
  reenabledAt: null,
  lapsedSince: null,
  lastReceivedAt: iso(NOW - HOUR),
  unbound: 0,
  unauthenticated: 0,
  expired: 0,
  ...o,
});

/** The real gate over a fake session (`session` is the account, `other` the other one) and a fake row. */
const verify = (t: string) =>
  Promise.resolve(
    t === "session" ? { id: ACCT, email: null } : t === "other" ? { id: OTHER, email: null } : null,
  );
const gate = (status: EntitlementStatus) => (req: Request) =>
  requireActiveEntitlementWith(req, {
    verify,
    lookup: () => Promise.resolve({ plan: "monthly", status, current_period_end: null }),
  });

interface Message {
  uid: string;
  address_id: string;
  batch_id: string | null;
  phase: string;
  message_date: string | null;
  ordering_date: string;
  [key: string]: unknown;
}
interface Answer {
  messages: Message[];
  mailboxes: Record<string, unknown>[];
  more: boolean;
  draining: boolean;
}
interface World {
  store: FakeStore;
  clock: number;
  pull(ack?: unknown, session?: string, method?: string): Promise<Response>;
  answer(ack?: unknown): Promise<Answer>;
}

function world(status: EntitlementStatus = "active"): World {
  const store = new FakeStore();
  store.boxes = [box(M1), box(M2, { provider: "google" }), box(MO, { accountId: OTHER })];
  const w = { store, clock: NOW } as World;
  const handler = pullHandler({ entitle: gate(status), store, now: () => w.clock });
  w.pull = (ack, session = "session", method = "POST") =>
    handler(
      new Request("http://127.0.0.1/functions/v1/mail-pull", {
        method,
        headers: { authorization: `Bearer ${session}`, "content-type": "application/json" },
        ...(method === "POST" ? { body: JSON.stringify(ack === undefined ? {} : { ack }) } : {}),
      }),
    );
  w.answer = async (ack) => {
    const res = await w.pull(ack);
    assertEquals(res.status, 200);
    return await res.json() as Answer;
  };
  return w;
}
const uids = (a: Answer) => a.messages.map((m) => m.uid);

// ---------------------------------------------------------------- order

Deno.test("claims_come_in_ordering_date_then_uid_order", async () => {
  const w = world();
  // Queued in an order that is neither the ordering date's nor the uid's.
  w.store.add({ n: 3, ordering: NOW - HOUR });
  w.store.add({ n: 1, ordering: NOW - 3 * HOUR, queued: NOW - 10 * MINUTE });
  w.store.add({ n: 2, ordering: NOW - HOUR, queued: NOW - 20 * MINUTE });
  w.store.add({ n: 4, ordering: NOW - 2 * HOUR, address: M2 });
  // The other account's message is never in this account's answer.
  w.store.add({ n: 5, ordering: NOW - 4 * HOUR, account: OTHER, address: MO });
  const a = await w.answer();
  assertEquals(uids(a), [uid(1), uid(4), uid(2), uid(3)]);
  // Each message carries the claims and the few facts reconcile needs (§4.7 step 6), never the text.
  assertEquals(Object.keys(a.messages[0]).sort(), [
    "address_id",
    "authenticity",
    "batch_id",
    "claims",
    "judgment_id",
    "message_date",
    "ordering_date",
    "phase",
    "sender_addr",
    "sender_name",
    "uid",
  ]);
  assertEquals(a.messages[0].claims, [{ kind: "noise-free", n: 1 }]);
  assertEquals(a.messages[1].address_id, M2);
  assertEquals([a.more, a.draining], [false, false]);
  // A pull without an ack delivers nothing: the same answer comes back until the device acks it.
  assertEquals(uids(await w.answer()), uids(a));
});

Deno.test("a_message_dated_2099_orders_at_its_arrival", async () => {
  const w = world();
  // mail-process's own row for a live forward whose Date header is 2099 and that arrived 30 minutes ago.
  const item: WorkItem = {
    id: "9",
    accountId: ACCT,
    addressId: M1,
    batchId: null,
    s3Key: "work/x",
    phase: "live",
    authenticity: "aligned",
    arrivedAt: iso(NOW - 30 * MINUTE),
    nextAttemptAt: iso(NOW - 30 * MINUTE),
    attempts: 0,
    batchOpenedAt: null,
  };
  const raw = "From: Course Notices <notices@lms.test>\r\nDate: Thu, 01 Jan 2099 09:00:00 +0000\r\n" +
    "Subject: Lab moved\r\nMessage-ID: <x-2099@lms.test>\r\n\r\nbody\r\n";
  const headers = parseHeaders(raw);
  const row = await claimsRow(item, { headers, from: "", subject: "", date: "", text: "" }, [], null);
  assertEquals(row.orderingDate, iso(NOW - 30 * MINUTE));
  w.store.add({ n: 1, ordering: NOW - HOUR });
  w.store.add({ n: 2, ordering: Date.parse(row.orderingDate), messageDate: row.messageDate });
  w.store.add({ n: 3, ordering: NOW - 10 * MINUTE });
  const a = await w.answer();
  assertEquals(uids(a), [uid(1), uid(2), uid(3)]);
  // The header's date still rides along for the card (D21); only the order uses the arrival.
  assertEquals(a.messages[1].message_date, "2099-01-01T09:00:00.000Z");
  assertEquals(a.messages[1].ordering_date, iso(NOW - 30 * MINUTE));
});

// ---------------------------------------------------------------- withholding

/** A backfill batch B1 on M1 opened three hours ago with three originals, and one live message queued
 * before it opened. */
function backfill(o: Partial<PullBatch>): World {
  const w = world();
  w.store.batch(B1, NOW - 3 * HOUR, o);
  w.store.add({ n: 1, ordering: NOW - 4 * HOUR });
  w.store.add({ n: 12, ordering: NOW - 30 * DAY, queued: NOW - 2 * HOUR, batch: B1 });
  w.store.add({ n: 11, ordering: NOW - 60 * DAY, queued: NOW - 2 * HOUR, batch: B1 });
  w.store.add({ n: 13, ordering: NOW - 10 * DAY, queued: NOW - HOUR, batch: B1 });
  return w;
}

Deno.test("a_batch_is_withheld_until_received_reaches_sent_or_a_timeout_then_delivered_whole", async () => {
  const withheld: Array<[string, Partial<PullBatch>]> = [
    ["still sending", { received: 3 }],
    ["one short, closed an hour ago", { sent: 4, received: 3, sentAt: iso(NOW - HOUR) }],
    ["one short, closed a minute under two hours ago", {
      sent: 4,
      received: 3,
      sentAt: iso(NOW - 2 * HOUR + MINUTE),
    }],
  ];
  for (const [why, o] of withheld) {
    const w = backfill(o);
    const a = await w.answer();
    assertEquals(uids(a), [uid(1)], why);
    assertEquals(w.store.batchRows[0].closedAt, null, why);
  }
  const whole = [uid(11), uid(12), uid(13)];
  const delivered: Array<[string, Partial<PullBatch>, string | null]> = [
    ["received reaches sent", { sent: 3, received: 3, sentAt: iso(NOW - 5 * MINUTE) }, iso(NOW)],
    ["received passes sent", { sent: 2, received: 3, sentAt: iso(NOW - 5 * MINUTE) }, iso(NOW)],
    ["two hours after the device closed it", { sent: 4, received: 3, sentAt: iso(NOW - 2 * HOUR) }, iso(NOW)],
    ["already closed by the sweep", { sent: 4, received: 3, closedAt: iso(NOW - DAY) }, iso(NOW - DAY)],
  ];
  for (const [why, o, closed] of delivered) {
    const w = backfill(o);
    const a = await w.answer();
    // Whole and in ordering-date order; the live message queued before the batch opened comes first.
    assertEquals(uids(a), [uid(1), ...whole], why);
    assert(a.messages.slice(1).every((m) => m.batch_id === B1 && m.phase === "backfill"), why);
    assertEquals(w.store.batchRows[0].closedAt, closed, why);
  }
  // Fourteen days after it opened, a batch the device never closed is delivered whole too.
  const w = world();
  w.store.batch(B1, NOW - 14 * DAY, { received: 2 });
  w.store.add({ n: 12, ordering: NOW - 30 * DAY, queued: NOW - 13 * DAY, batch: B1 });
  w.store.add({ n: 11, ordering: NOW - 60 * DAY, queued: NOW - 13 * DAY, batch: B1 });
  assertEquals(uids(await w.answer()), [uid(11), uid(12)]);
  assertEquals(w.store.batchRows[0].closedAt, iso(NOW));
  // A minute short of fourteen days, it is still withheld.
  const early = world();
  early.store.batch(B1, NOW - 14 * DAY + MINUTE, { received: 2 });
  early.store.add({ n: 11, ordering: NOW - 60 * DAY, queued: NOW - 13 * DAY, batch: B1 });
  assertEquals(uids(await early.answer()), []);
});

Deno.test("a_closed_batch_is_withheld_while_any_of_its_items_still_waits_to_be_processed", async () => {
  // Received reaches sent at intake, before mail-process has queued every claim: delivering then would
  // split the batch across pulls. It waits until none of its work is left.
  const w = backfill({ sent: 3, received: 3, sentAt: iso(NOW - 5 * MINUTE) });
  w.store.work = [{ accountId: ACCT, batchId: B1 }];
  w.store.add({ n: 20, ordering: NOW - 20 * MINUTE });
  assertEquals(uids(await w.answer()), [uid(1)]);
  // Closed all the same (received reached sent), so intake binds nothing more to it.
  assertEquals(w.store.batchRows[0].closedAt, iso(NOW));
  w.store.work = [];
  assertEquals(uids(await w.answer()), [uid(1), uid(11), uid(12), uid(13), uid(20)]);
});

Deno.test("live_claims_queued_during_an_open_batch_come_after_it", async () => {
  const w = world();
  w.store.batch(B2, NOW - 2 * HOUR, { phase: "recover" });
  w.store.add({ n: 1, ordering: NOW - 3 * HOUR }); // queued before the batch opened
  w.store.add({ n: 31, ordering: NOW - 90 * MINUTE, queued: NOW - 90 * MINUTE }); // during it
  // During it, and dated before the batch opened: still after the batch.
  w.store.add({ n: 32, ordering: NOW - 150 * MINUTE, queued: NOW - 30 * MINUTE });
  w.store.add({ n: 21, ordering: NOW - 5 * HOUR, queued: NOW - 80 * MINUTE, batch: B2, phase: "recover" });
  w.store.add({ n: 22, ordering: NOW - HOUR, queued: NOW - 70 * MINUTE, batch: B2, phase: "recover" });
  // Inside the block, an equal ordering date falls to the uid, whatever the queue order.
  w.store.add({ n: 20, ordering: NOW - HOUR, queued: NOW - 60 * MINUTE, batch: B2, phase: "recover" });
  w.store.add({ n: 40, ordering: NOW - HOUR, address: M2 }); // another mailbox: never held by M1's batch
  const open = await w.answer();
  assertEquals(uids(open), [uid(1), uid(40)]);
  // The batch closes; it comes whole, then the live mail that waited behind it, each part in
  // (ordering date, uid) order. Message 22 is later than message 31 and message 32 is dated before the
  // batch opened, and the batch still comes first.
  Object.assign(w.store.batchRows[0], { sent: 3, received: 3, sentAt: iso(NOW - MINUTE) });
  const closed = await w.answer([uid(1), uid(40)]);
  assertEquals(uids(closed), [uid(21), uid(20), uid(22), uid(32), uid(31)]);
  assertEquals(closed.draining, false);
});

// ---------------------------------------------------------------- ack

Deno.test("ack_marks_delivered", async () => {
  const w = world();
  w.store.add({ n: 1, ordering: NOW - 2 * HOUR });
  w.store.add({ n: 2, ordering: NOW - HOUR });
  // The other account holds a row with the same uid; this account's ack never reaches it.
  w.store.add({ n: 1, ordering: NOW - 2 * HOUR, account: OTHER, address: MO });
  assertEquals(uids(await w.answer()), [uid(1), uid(2)]);
  w.clock = NOW + MINUTE;
  // The ack is applied before the read, so the answer already leaves out what it names.
  assertEquals(uids(await w.answer([uid(1)])), [uid(2)]);
  const mine = w.store.rows.filter((r) => r.uid === uid(1));
  assertEquals(mine.map((r) => [r.accountId, r.deliveredAt]), [
    [ACCT, iso(NOW + MINUTE)],
    [OTHER, null],
  ]);
  // A second ack of a delivered uid changes nothing.
  w.clock = NOW + 2 * MINUTE;
  assertEquals(uids(await w.answer([uid(1), uid(2)])), []);
  assertEquals(w.store.rows[0].deliveredAt, iso(NOW + MINUTE));
  assertEquals(w.store.rows[1].deliveredAt, iso(NOW + 2 * MINUTE));
});

Deno.test("an_ack_outside_the_mail_uid_shape_is_dropped", async () => {
  const w = world();
  w.store.add({ n: 1, ordering: NOW - HOUR });
  const u = uid(1);
  const bad = [
    `${u}:0`, // a claim's uid, not a message's
    u.toUpperCase(),
    `gmail:${"a".repeat(16)}`,
    `mail:${"g".repeat(32)}`,
    `mail:${"a".repeat(31)}`,
    ` ${u}`,
    `${u}\n`,
    `${u}),uid.neq.(x`,
    42,
    null,
    { uid: u },
  ];
  const a = await w.answer(bad);
  assertEquals(uids(a), [u]);
  assertEquals(w.store.calls.includes("deliver"), false);
  // An ack that is not a list is no ack at all.
  for (const notList of [u, { [u]: true }, 7]) assertEquals(uids(await w.answer(notList)), [u]);
  assertEquals(w.store.calls.includes("deliver"), false);
  // Past the cap the rest is dropped: a legitimate ack is never longer than the pages it answers.
  const many = Array.from({ length: ACK_CAP + 3 }, (_, i) => uid(1000 + i));
  await w.answer([...bad, ...many]);
  assertEquals(w.store.delivered, [many.slice(0, ACK_CAP)]);
});

// ---------------------------------------------------------------- mailboxes

Deno.test("each_mailboxs_state_and_reenabled_at_ride_on_the_answer", async () => {
  const w = world();
  w.store.boxes = [
    box(M1, { status: "quiet", unbound: 2, expired: 1 }),
    box(M2, { provider: "google", status: "active", reenabledAt: iso(NOW - DAY) }),
    box(M3, { status: "revoked" }),
    box(MO, { accountId: OTHER, status: "blocked" }),
  ];
  // A revoked mailbox's claims are never delivered: the student disconnected it.
  w.store.add({ n: 1, ordering: NOW - HOUR, address: M3 });
  const a = await w.answer();
  assertEquals(a.messages, []);
  assertEquals(a.mailboxes, [
    {
      address_id: M1,
      provider: "m365",
      status: "quiet",
      reenabled_at: null,
      lapsed_since: null,
      last_received_at: iso(NOW - HOUR),
      unbound: 2,
      unauthenticated: 0,
      expired: 1,
    },
    {
      address_id: M2,
      provider: "google",
      status: "active",
      reenabled_at: iso(NOW - DAY),
      lapsed_since: null,
      last_received_at: iso(NOW - HOUR),
      unbound: 0,
      unauthenticated: 0,
      expired: 0,
    },
  ]);
  // No mailbox at all is an empty answer, never an error.
  w.store.boxes = [];
  assertEquals(await w.answer(), { messages: [], mailboxes: [], more: false, draining: false });
});

// ---------------------------------------------------------------- pages and the gate

Deno.test("an_answer_holds_at_most_a_page_and_says_more_and_whether_a_closed_batch_is_draining", async () => {
  const w = world();
  for (let i = 0; i < PAGE + 5; i++) w.store.add({ n: 100 + i, ordering: NOW - DAY + i * MINUTE });
  const first = await w.answer();
  assertEquals(first.messages.length, PAGE);
  assertEquals([first.more, first.draining], [true, false]);
  const second = await w.answer(uids(first));
  assertEquals(uids(second), Array.from({ length: 5 }, (_, i) => uid(100 + PAGE + i)));
  assertEquals([second.more, second.draining], [false, false]);
  // A closed batch past one page drains: the device keeps pulling past its three live rounds.
  const b = world();
  b.store.batch(B1, NOW - 3 * HOUR, { sent: PAGE + 1, received: PAGE + 1, sentAt: iso(NOW - HOUR) });
  for (let i = 0; i <= PAGE; i++) {
    b.store.add({ n: 500 + i, ordering: NOW - 60 * DAY + i * HOUR, queued: NOW - 2 * HOUR, batch: B1 });
  }
  const page = await b.answer();
  assertEquals([page.messages.length, page.more, page.draining], [PAGE, true, true]);
  const last = await b.answer(uids(page));
  assertEquals([uids(last), last.more, last.draining], [[uid(500 + PAGE)], false, false]);
});

Deno.test("the_pull_needs_a_session_an_entitlement_and_post", async () => {
  const w = world("canceled");
  w.store.add({ n: 1, ordering: NOW - HOUR });
  assertEquals((await w.pull([uid(1)])).status, 402);
  assertEquals((await w.pull([uid(1)], "nobody")).status, 401);
  // Nothing was read or written for either, the ack included.
  assertEquals(w.store.calls, []);
  assertEquals(w.store.rows[0].deliveredAt, null);
  const ok = world();
  const get = await ok.pull(undefined, "session", "GET");
  assertEquals(get.status, 405);
  await get.body?.cancel();
  // A body that is not JSON is refused by name, and an empty one is an empty ack.
  const handler = pullHandler({ entitle: gate("active"), store: ok.store, now: () => NOW });
  const post = (body: string) =>
    handler(
      new Request("http://127.0.0.1/functions/v1/mail-pull", {
        method: "POST",
        headers: { authorization: "Bearer session" },
        body,
      }),
    );
  const notJson = await post("{not json");
  assertEquals(notJson.status, 400);
  await notJson.body?.cancel();
  const empty = await post("");
  assertEquals(empty.status, 200);
  assertEquals((await empty.json()).messages, []);
});
