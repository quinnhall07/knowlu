/**
 * The wire for `mail-pull` (email plan T4.4): the store over PostgREST, then `handler.ts`.
 * `verify_jwt = false` (config.toml): the handler authenticates every request itself, a session plus an
 * active entitlement (§6.3). No secret of its own: the service role through `restFromEnv`.
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is cancelled
 * unread, and `asResponse` answers a bare 500.
 */
import { type Rest, restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse } from "../_shared/http.ts";
import {
  type ClaimHead,
  type ClaimRow,
  type PullBatch,
  pullHandler,
  type PullMailbox,
  type PullStore,
} from "./handler.ts";

/** One PostgREST call; a failure throws a fixed message and its body is never read. */
async function call(rest: Rest, what: string, path: string, init: RequestInit = {}, prefer?: string) {
  const res = await rest.fetch(`${rest.url}/rest/v1/${path}`, {
    ...init,
    headers: {
      apikey: rest.serviceKey,
      authorization: `Bearer ${rest.serviceKey}`,
      "content-type": "application/json",
      ...(prefer === undefined ? {} : { prefer }),
    },
  });
  if (!res.ok) {
    await res.body?.cancel();
    throw new Error(`store ${what} ${res.status}`);
  }
  return res;
}
const rows = async <T>(res: Response) => await res.json() as T[];
const id = (s: string) => encodeURIComponent(s);
/** A PostgREST `in.(…)` list of uuids or uids, each already checked against its shape by the caller. */
const list = (xs: Array<string | number>) => `(${xs.map((x) => id(String(x))).join(",")})`;
/** Undelivered rows read per page, and the most read in one pull (30 days of live mail and a backfill). */
const HEAD_PAGE = 1000;
const MAX_HEADS = 10_000;

interface MailboxDb {
  id: string;
  provider: string;
  status: string;
  reenabled_at: string | null;
  lapsed_since: string | null;
  last_received_at: string | null;
  unbound: number;
  unauthenticated: number;
  expired: number;
}
interface BatchDb {
  id: string;
  address_id: string;
  phase: PullBatch["phase"];
  sent: number | null;
  received: number;
  opened_at: string;
  sent_at: string | null;
  closed_at: string | null;
}
interface HeadDb {
  id: number;
  address_id: string;
  batch_id: string | null;
  uid: string;
  ordering_date: string;
  queued_at: string;
}
interface ClaimDb extends HeadDb {
  message_date: string | null;
  sender_name: string | null;
  sender_addr: string | null;
  phase: ClaimRow["phase"];
  authenticity: ClaimRow["authenticity"];
  claims: unknown[];
  judgment_id: string | null;
}
const HEAD = "id,address_id,batch_id,uid,ordering_date,queued_at";
const head = (r: HeadDb): ClaimHead => ({
  id: r.id,
  addressId: r.address_id,
  batchId: r.batch_id,
  uid: r.uid,
  orderingDate: r.ordering_date,
  queuedAt: r.queued_at,
});

function storeOver(rest: Rest): PullStore {
  const account = (accountId: string) => `account_id=eq.${id(accountId)}`;
  return {
    async deliver(accountId, uids, atIso) {
      const q = `${account(accountId)}&uid=in.${list(uids)}&delivered_at=is.null`;
      const init = { method: "PATCH", body: JSON.stringify({ delivered_at: atIso }) };
      await (await call(rest, "deliver", `mail_claims?${q}`, init, "return=minimal")).body?.cancel();
    },
    async mailboxes(accountId) {
      const q =
        `${account(accountId)}&select=id,provider,status,reenabled_at,lapsed_since,last_received_at,` +
        `unbound,unauthenticated,expired&order=id`;
      return (await rows<MailboxDb>(await call(rest, "mailboxes", `mail_addresses?${q}`))).map((
        r,
      ): PullMailbox => ({
        id: r.id,
        provider: r.provider,
        status: r.status,
        reenabledAt: r.reenabled_at,
        lapsedSince: r.lapsed_since,
        lastReceivedAt: r.last_received_at,
        unbound: r.unbound,
        unauthenticated: r.unauthenticated,
        expired: r.expired,
      }));
    },
    async batches(addressIds) {
      if (addressIds.length === 0) return [];
      // A mailbox opens few batches and closed ones are pruned at 30 days, so one read holds them all.
      const q = `address_id=in.${list(addressIds)}&select=id,address_id,phase,sent,received,opened_at,` +
        `sent_at,closed_at&order=opened_at,id&limit=1000`;
      return (await rows<BatchDb>(await call(rest, "batches", `mail_batches?${q}`))).map((r): PullBatch => ({
        id: r.id,
        addressId: r.address_id,
        phase: r.phase,
        sent: r.sent,
        received: r.received,
        openedAt: r.opened_at,
        sentAt: r.sent_at,
        closedAt: r.closed_at,
      }));
    },
    async pendingWork(accountId, batchIds) {
      // One row each is enough to know: a backfill can leave thousands of work rows in one batch.
      const waiting = await Promise.all(batchIds.map(async (b) => {
        const q = `${account(accountId)}&batch_id=eq.${id(b)}&select=id&limit=1`;
        return (await rows<unknown>(await call(rest, "pending work", `mail_work?${q}`))).length > 0;
      }));
      return batchIds.filter((_, i) => waiting[i]);
    },
    async closeBatch(batch, atIso) {
      const q = `id=eq.${id(batch)}&closed_at=is.null&select=id`;
      const init = { method: "PATCH", body: JSON.stringify({ closed_at: atIso }) };
      return (await rows<unknown>(
        await call(rest, "close", `mail_batches?${q}`, init, "return=representation"),
      ))
        .length > 0;
    },
    async undelivered(accountId) {
      // Paged by id, past PostgREST's row cap; heads only, so a long backlog stays small.
      const out: ClaimHead[] = [];
      for (let last = 0; out.length < MAX_HEADS;) {
        const q = `${account(accountId)}&delivered_at=is.null&id=gt.${last}&select=${HEAD}&order=id` +
          `&limit=${HEAD_PAGE}`;
        const page = await rows<HeadDb>(await call(rest, "undelivered", `mail_claims?${q}`));
        out.push(...page.map(head));
        if (page.length < HEAD_PAGE) break;
        last = page[page.length - 1].id;
      }
      return out;
    },
    async claims(accountId, ids) {
      const q = `${account(accountId)}&id=in.${list(ids)}&select=${HEAD},message_date,sender_name,` +
        `sender_addr,phase,authenticity,claims,judgment_id`;
      return (await rows<ClaimDb>(await call(rest, "claims", `mail_claims?${q}`))).map((r): ClaimRow => ({
        ...head(r),
        messageDate: r.message_date,
        senderName: r.sender_name,
        senderAddr: r.sender_addr,
        phase: r.phase,
        authenticity: r.authenticity,
        claims: r.claims,
        judgmentId: r.judgment_id,
      }));
    },
  };
}

let handler: ((req: Request) => Promise<Response>) | null = null;

Deno.serve(async (req) => {
  try {
    handler ??= pullHandler({
      entitle: requireActiveEntitlement,
      store: storeOver(restFromEnv()),
      now: () => Date.now(),
    });
    return await handler(req);
  } catch (e) {
    return asResponse(e);
  }
});
