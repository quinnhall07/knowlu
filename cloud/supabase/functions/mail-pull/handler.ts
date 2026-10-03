/**
 * `POST /mail-pull {ack}` (email spec §4.8 step 1, D10, §6.3; plan T4.4): the device's read of its queued
 * `mail_claims`, one row per message, and of its mailboxes' states. Session plus entitlement, like
 * `gmail-read`.
 *
 * 1. **Ack first.** `ack` lists the `mail:<h>` uids (D20: `mail:` and 32 lower-case hex) the device
 *    applied from earlier answers; each of the account's undelivered rows with that uid is stamped
 *    `delivered_at`. An entry of any other shape is dropped before it can reach a filter, and an ack past
 *    `ACK_CAP` entries is cut there: the device never holds more than a few pages to ack.
 * 2. **Close what has closed.** A backfill or recover batch is closed when the received count reaches the
 *    count the device reported sent, 2 hours after the device closed the send, or 14 days after it opened
 *    (§4.8). The pull stamps `closed_at` on one it finds closed; the nightly sweep stamps the timed-out
 *    ones nobody pulled.
 * 3. **Withhold.** A batch's messages are delivered only once it is closed **and none of its items still
 *    waits in `mail_work`**: received counts arrivals at intake, before `mail-process` has queued their
 *    claims, so a batch closed by count can still be missing claims. The work is read before the claims
 *    (`mail-process` queues a claim before it deletes the work row), so delivered, it comes whole. A
 *    mailbox's live messages queued at or after the opening of one of its batches that is not yet ready,
 *    and its later batches, ready or not, wait behind it, and then come after it. A claim of a batch
 *    opened after the read holds its whole mailbox until the next pull.
 * 4. **Order.** Every answer is in (ordering date, uid) order (the ordering date is `mail-process`'s, the
 *    earlier of the Date header and the received time, so a message dated 2099 orders at its arrival). A
 *    delivered batch is one block, placed at its opening and holding its messages in that order, then the
 *    live messages that waited behind it, in that order too.
 * 5. **One page.** At most `PAGE` messages. `more` says deliverable messages remain past this page, and
 *    `draining` says some of them belong to a closed batch, which the device drains past its three live
 *    rounds (§4.8 step 1).
 *
 * Each of the account's mailboxes but a revoked one rides on every answer, with its status, `reenabled_at`
 * and counters (§4.10's *Shown*); a revoked mailbox's messages are never delivered. No token, hash, hint
 * or key is in the answer. **Nothing here logs** a uid, a sender or a claim.
 */
import type { Entitle } from "../_shared/judge_handler.ts";
import { asResponse, fail, json, methodNotAllowed } from "../_shared/http.ts";

/** D20's message uid, the only shape an ack entry may have. */
export const ACK_SHAPE = /^mail:[0-9a-f]{32}$/;
/** The most uids one ack applies; a legitimate ack is a few pages long. */
export const ACK_CAP = 500;
/** The most messages one answer carries. */
export const PAGE = 100;
/** §4.8's timeouts: after the device closed the send, and after the batch opened. */
export const SENT_GRACE_MS = 2 * 3_600_000;
export const OPEN_LIMIT_MS = 14 * 86_400_000;
/** A body is an ack of at most `ACK_CAP` uids; anything much larger is refused unread. */
const BODY_LIMIT = 64 * 1024;

/** A `mail_addresses` row as the pull answers it. */
export interface PullMailbox {
  id: string;
  provider: string;
  status: string;
  reenabledAt: string | null;
  lapsedSince: string | null;
  lastReceivedAt: string | null;
  unbound: number;
  unauthenticated: number;
  expired: number;
}
/** A `mail_batches` row, never its nonce's HMAC. */
export interface PullBatch {
  id: string;
  addressId: string;
  phase: "backfill" | "recover";
  sent: number | null;
  received: number;
  openedAt: string;
  sentAt: string | null;
  closedAt: string | null;
}
/** What the order needs of an undelivered `mail_claims` row. */
export interface ClaimHead {
  id: number;
  addressId: string;
  batchId: string | null;
  uid: string;
  orderingDate: string;
  queuedAt: string;
}
/** The whole row the device is sent: the claims and the message's metadata, never its text. */
export interface ClaimRow extends ClaimHead {
  messageDate: string | null;
  senderName: string | null;
  senderAddr: string | null;
  phase: "live" | "backfill" | "recover";
  authenticity: "aligned" | "unaligned";
  claims: unknown[];
  judgmentId: string | null;
}
export interface PullStore {
  /** Stamps `delivered_at` on the account's undelivered rows whose uid is listed. */
  deliver(accountId: string, uids: string[], atIso: string): Promise<void>;
  /** Every `mail_addresses` row of the account, whatever its status, in a stable order. */
  mailboxes(accountId: string): Promise<PullMailbox[]>;
  /** Every batch of these mailboxes (the caller passes only the account's own). */
  batches(addressIds: string[]): Promise<PullBatch[]>;
  /** Which of these batches still have a `mail_work` row of the account's. */
  pendingWork(accountId: string, batchIds: string[]): Promise<string[]>;
  /** Stamps `closed_at` only while it is null; false, with nothing written, otherwise. */
  closeBatch(id: string, atIso: string): Promise<boolean>;
  /** The account's undelivered rows, as heads. */
  undelivered(accountId: string): Promise<ClaimHead[]>;
  /** The account's rows with these ids, whole. */
  claims(accountId: string, ids: number[]): Promise<ClaimRow[]>;
}
export interface PullDeps {
  /** A session and an active entitlement (`requireActiveEntitlement`). */
  entitle: Entitle;
  store: PullStore;
  now(): number;
}

const iso = (ms: number) => new Date(ms).toISOString();
const at = (s: string | null) => s === null ? NaN : Date.parse(s);

/** §4.8's three ways a batch closes, read at `now`; a stamped `closed_at` is closed already. */
export function isClosed(b: PullBatch, now: number): boolean {
  if (b.closedAt !== null) return true;
  if (b.sent !== null && b.received >= b.sent) return true;
  if (b.sentAt !== null && now - at(b.sentAt) >= SENT_GRACE_MS) return true;
  return now - at(b.openedAt) >= OPEN_LIMIT_MS;
}

/** One delivered message in the answer's order, and whether it came from a batch. */
export interface Placed {
  head: ClaimHead;
  fromBatch: boolean;
}
interface Block {
  batch: PullBatch;
  own: ClaimHead[];
  tail: ClaimHead[];
}

/** (ordering date, uid), then the row id, so equal keys still sort the same way every time. */
function byOrder(a: ClaimHead, b: ClaimHead): number {
  return at(a.orderingDate) - at(b.orderingDate) || (a.uid < b.uid ? -1 : a.uid > b.uid ? 1 : 0) ||
    a.id - b.id;
}

/**
 * Steps 3 and 4, pure: which of `heads` are delivered now, and in what order. `pending` names the batches
 * with work still waiting; `mailboxes` are the account's mailboxes that are not revoked.
 */
export function arrange(
  now: number,
  batches: PullBatch[],
  pending: ReadonlySet<string>,
  heads: ClaimHead[],
  mailboxes: ReadonlySet<string>,
): Placed[] {
  const known = new Map(batches.map((b) => [b.id, b]));
  const ready = new Set(batches.filter((b) => isClosed(b, now) && !pending.has(b.id)).map((b) => b.id));
  // Each mailbox holds everything from the earliest opening of a batch that is not yet ready: its live
  // mail and any later batch, ready or not.
  const holdFrom = new Map<string, number>();
  const hold = (address: string, from: number) => {
    const was = holdFrom.get(address);
    if (was === undefined || from < was) holdFrom.set(address, from);
  };
  for (const b of batches) if (!ready.has(b.id)) hold(b.addressId, at(b.openedAt));
  // A claim of a batch the read did not hold (a pruned batch's claims have a null batch, so this one opened
  // after the read): its opening is unknown, so its whole mailbox waits for the next pull.
  for (const h of heads) if (h.batchId !== null && !known.has(h.batchId)) hold(h.addressId, -Infinity);
  const blocks = new Map<string, Block>();
  const loose: ClaimHead[] = [];
  for (const h of heads) {
    if (!mailboxes.has(h.addressId)) continue;
    const from = holdFrom.get(h.addressId) ?? Infinity;
    if (h.batchId !== null) {
      const b = known.get(h.batchId);
      if (b === undefined || !ready.has(b.id) || at(b.openedAt) >= from) continue;
      const block = blocks.get(b.id) ?? { batch: b, own: [], tail: [] };
      blocks.set(b.id, block);
      block.own.push(h);
      continue;
    }
    if (at(h.queuedAt) >= from) continue;
    loose.push(h);
  }
  // A live message queued at or after a delivered block's opening comes after the latest such block of
  // its mailbox; any other is a unit of its own.
  const units: Array<{ key: number; tie: string; items: Placed[] }> = [];
  for (const h of loose) {
    let behind: Block | undefined;
    for (const block of blocks.values()) {
      const opened = at(block.batch.openedAt);
      if (block.batch.addressId !== h.addressId || opened > at(h.queuedAt)) continue;
      if (behind === undefined || opened > at(behind.batch.openedAt)) behind = block;
    }
    if (behind !== undefined) behind.tail.push(h);
    else units.push({ key: at(h.orderingDate), tie: h.uid, items: [{ head: h, fromBatch: false }] });
  }
  for (const block of blocks.values()) {
    units.push({
      key: at(block.batch.openedAt),
      tie: block.batch.id,
      items: [
        ...block.own.sort(byOrder).map((head) => ({ head, fromBatch: true })),
        ...block.tail.sort(byOrder).map((head) => ({ head, fromBatch: false })),
      ],
    });
  }
  units.sort((a, b) =>
    a.key - b.key || (a.tie < b.tie ? -1 : a.tie > b.tie ? 1 : 0) || a.items[0].head.id - b.items[0].head.id
  );
  return units.flatMap((u) => u.items);
}

/** Step 1's ack: the body's `ack`, each entry of the uid shape, once, at most `ACK_CAP`. */
async function readAck(req: Request): Promise<string[]> {
  const text = await req.text();
  if (text.length > BODY_LIMIT) throw fail(413, `body over ${BODY_LIMIT} characters`);
  if (text.trim() === "") return [];
  let body: unknown;
  try {
    body = JSON.parse(text);
  } catch {
    throw fail(400, "body is not JSON");
  }
  const ack = typeof body === "object" && body !== null ? (body as { ack?: unknown }).ack : undefined;
  if (!Array.isArray(ack)) return [];
  const shaped = ack.filter((u): u is string => typeof u === "string" && ACK_SHAPE.test(u));
  return [...new Set(shaped)].slice(0, ACK_CAP);
}

const mailboxView = (b: PullMailbox) => ({
  address_id: b.id,
  provider: b.provider,
  status: b.status,
  reenabled_at: b.reenabledAt,
  lapsed_since: b.lapsedSince,
  last_received_at: b.lastReceivedAt,
  unbound: b.unbound,
  unauthenticated: b.unauthenticated,
  expired: b.expired,
});
const messageView = (r: ClaimRow) => ({
  uid: r.uid,
  address_id: r.addressId,
  batch_id: r.batchId,
  phase: r.phase,
  authenticity: r.authenticity,
  message_date: r.messageDate,
  ordering_date: r.orderingDate,
  sender_name: r.senderName,
  sender_addr: r.senderAddr,
  claims: r.claims,
  judgment_id: r.judgmentId,
});

export function pullHandler(deps: PullDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      if (req.method !== "POST") return methodNotAllowed(["POST"]);
      const { account_id } = await deps.entitle(req);
      const ack = await readAck(req);
      const now = deps.now();
      if (ack.length > 0) await deps.store.deliver(account_id, ack, iso(now));
      const boxes = (await deps.store.mailboxes(account_id)).filter((b) => b.status !== "revoked");
      const mailboxes = new Set(boxes.map((b) => b.id));
      // Batches and their waiting work are read before the claims: mail-process queues an item's claim
      // before it deletes the item's work row, so an item whose work is gone by the time `pendingWork`
      // answers has its claim in `undelivered`'s read, which comes after.
      const batches = mailboxes.size === 0 ? [] : await deps.store.batches([...mailboxes]);
      const closed = batches.filter((b) => isClosed(b, now));
      for (const b of closed) {
        if (b.closedAt === null) await deps.store.closeBatch(b.id, iso(now));
      }
      const pending = new Set(
        closed.length === 0 ? [] : await deps.store.pendingWork(account_id, closed.map((b) => b.id)),
      );
      const heads = mailboxes.size === 0 ? [] : await deps.store.undelivered(account_id);
      const placed = arrange(now, batches, pending, heads, mailboxes);
      const page = placed.slice(0, PAGE);
      const rows = page.length === 0 ? [] : await deps.store.claims(account_id, page.map((p) => p.head.id));
      const byId = new Map(rows.map((r) => [r.id, r]));
      return json(200, {
        messages: page.flatMap((p) => {
          const r = byId.get(p.head.id);
          return r === undefined ? [] : [messageView(r)];
        }),
        mailboxes: boxes.map(mailboxView),
        more: placed.length > PAGE,
        draining: placed.slice(PAGE).some((p) => p.fromBatch),
      });
    } catch (e) {
      return asResponse(e);
    }
  };
}
