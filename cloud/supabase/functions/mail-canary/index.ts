/**
 * The wire for `mail-canary` (email plan T4.3): the environment, the store over PostgREST and Resend, then
 * `handler.ts`. `verify_jwt = false` (config.toml): the hourly cron (`knowlu-mail-canary`, at :11) sends
 * `x-knowlu-job-token`, and that token is the check. Secrets, named only: `MAIL_JOBS_TOKEN`,
 * `MAIL_ADDRESS_KEY` (a canary's nonce is kept as its HMAC, as `mail` keeps it), and the project's
 * `EMAIL_API_URL` and `EMAIL_API_KEY`, the Resend endpoint and key `billing-jobs` uses (P9).
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is cancelled
 * unread, and the handler logs only that a store call failed.
 */
import { canarySender } from "../_shared/canary_send.ts";
import { type Rest, restFromEnv } from "../_shared/db.ts";
import { isActive, lookupFrom } from "../_shared/entitlement.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { hmacKeyFromBase64, nonceHmac } from "../mail-inbound/keys.ts";
import { type CanaryDeps, type CanaryMailbox, type CanaryStore, handle, type JobConfig } from "./handler.ts";
import { candidatesQuery, SENDABLE_IN } from "./query.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

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
/** A timestamp inside a PostgREST logic tree is quoted (it holds `.` and `:`), then the tree is encoded. */
const tree = (s: string) => encodeURIComponent(s);
const DAY = 86_400_000;
/** Canaries are pruned at 30 days (P7); nothing older is read. */
const KEPT_DAYS = 30;
const OUTSTANDING = 1000;

interface OutstandingDb {
  id: string;
  address_id: string;
  sent_at: string;
  mail_addresses: { status: string; last_canary_ok_at: string | null } | null;
}
interface CandidateDb {
  id: string;
  account_id: string;
  status: string;
  mailbox_hint: string | null;
  last_received_at: string | null;
  last_canary_sent_at: string | null;
  canaries_today: number;
}

function storeOver(rest: Rest): CanaryStore {
  const patched = async (what: string, table: string, query: string, body: unknown) => {
    const init = { method: "PATCH", body: JSON.stringify(body) };
    return (await rows<unknown>(await call(rest, what, `${table}?${query}`, init, "return=representation")))
      .length;
  };
  const lookup = lookupFrom(rest);
  return {
    async outstanding(before) {
      const floor = new Date(Date.parse(before) - KEPT_DAYS * DAY).toISOString();
      const q = `arrived_at=is.null&sent_at=lt.${id(before)}&sent_at=gt.${id(floor)}` +
        `&select=id,address_id,sent_at,mail_addresses(status,last_canary_ok_at)&order=sent_at.asc&limit=${OUTSTANDING}`;
      return (await rows<OutstandingDb>(await call(rest, "outstanding", `mail_canaries?${q}`)))
        .filter((r) => r.mail_addresses !== null)
        .map((r) => ({
          canaryId: r.id,
          addressId: r.address_id,
          sentAt: r.sent_at,
          status: r.mail_addresses!.status,
          lastCanaryOkAt: r.mail_addresses!.last_canary_ok_at,
        }));
    },
    async judge(address, from, to, sentAt) {
      const q = `id=eq.${id(address)}&status=in.(${from.join(",")})` +
        `&or=${tree(`(last_canary_ok_at.is.null,last_canary_ok_at.lt."${sentAt}")`)}&select=id`;
      return await patched("judge", "mail_addresses", q, { status: to }) > 0;
    },
    async candidates(canaryBefore, silentBefore) {
      const q = candidatesQuery(canaryBefore, silentBefore);
      return (await rows<CandidateDb>(await call(rest, "candidates", `mail_addresses?${q}`)))
        .map((r): CanaryMailbox => ({
          id: r.id,
          accountId: r.account_id,
          status: r.status,
          mailboxHint: r.mailbox_hint,
          lastReceivedAt: r.last_received_at,
          lastCanarySentAt: r.last_canary_sent_at,
          canariesToday: r.canaries_today,
        }));
    },
    async accountMail(accountId) {
      const boxes = await rows<{ last_canary_sent_at: string | null }>(
        await call(
          rest,
          "account canaries",
          `mail_addresses?account_id=eq.${id(accountId)}&select=last_canary_sent_at`,
        ),
      );
      const [reminder] = await rows<{ last_reminded_at: string }>(
        await call(
          rest,
          "reminder",
          `billing_reminders?account_id=eq.${id(accountId)}&select=last_reminded_at`,
        ),
      );
      return {
        canariesSentAt: boxes.map((b) => b.last_canary_sent_at),
        remindedAt: reminder?.last_reminded_at ?? null,
      };
    },
    entitled: async (accountId) => isActive(await lookup(accountId)),
    async takeCanary(address, seen, next) {
      const last = seen.lastSentAt === null
        ? "last_canary_sent_at=is.null"
        : `last_canary_sent_at=eq.${id(seen.lastSentAt)}`;
      const q = `id=eq.${
        id(address)
      }&status=${SENDABLE_IN}&canaries_today=eq.${seen.count}&${last}&select=id`;
      return await patched("canary turn", "mail_addresses", q, {
        canaries_today: next.count,
        last_canary_sent_at: next.lastSentAt,
      }) > 0;
    },
    async insertCanary(address, nonceHmac, sentAt) {
      const body = JSON.stringify({ address_id: address, nonce_hmac: nonceHmac, sent_at: sentAt });
      const [r] = await rows<{ id: string }>(
        await call(
          rest,
          "canary",
          "mail_canaries?select=id",
          { method: "POST", body },
          "return=representation",
        ),
      );
      if (r === undefined) throw new Error("store canary empty");
      return r.id;
    },
    async deleteCanary(address, canary) {
      const q = `id=eq.${id(canary)}&address_id=eq.${id(address)}`;
      await (await call(rest, "canary undo", `mail_canaries?${q}`, { method: "DELETE" }, "return=minimal"))
        .body?.cancel();
    },
  };
}

async function wire(): Promise<{ cfg: JobConfig; deps: CanaryDeps }> {
  const rest = restFromEnv();
  const addressKey = await hmacKeyFromBase64(env("MAIL_ADDRESS_KEY"));
  return {
    cfg: { jobsToken: env("MAIL_JOBS_TOKEN") },
    deps: {
      store: storeOver(rest),
      nonceHmac: (n) => nonceHmac(addressKey, n),
      now: () => Date.now(),
      // The provider `billing-jobs` sends through (P9): its endpoint and key, named only.
      sendCanary: canarySender({
        url: env("EMAIL_API_URL"),
        key: env("EMAIL_API_KEY"),
        fetch: globalThis.fetch,
      }),
    },
  };
}

let wired: Promise<{ cfg: JobConfig; deps: CanaryDeps }> | null = null;

Deno.serve(async (req) => {
  try {
    wired ??= wire();
    const { cfg, deps } = await wired.catch((e) => {
      wired = null;
      throw e;
    });
    return await handle(req, cfg, deps);
  } catch (e) {
    return asResponse(e);
  }
});
