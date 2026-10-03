/**
 * The wire for `mail` (email plan T4.1): the environment, the store over PostgREST, S3 (delete only, here)
 * and DNS, then `handler.ts`. `verify_jwt = false` (config.toml): the handler authenticates every request
 * itself, a session for the list and the delete and a session plus entitlement for the rest (§6.3).
 * Secrets, named only (spec §5.1): `MAIL_DOMAIN`, `MAIL_ADDRESS_KEY`, `MAIL_ENC_KEY`, `MAIL_BUCKET`,
 * `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`; and for the canary (T4.2, plan P9) the
 * project's existing `EMAIL_API_URL` and `EMAIL_API_KEY`, the Resend endpoint and key `billing-jobs` uses.
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is cancelled
 * unread (a constraint error can echo a hash), and `asResponse` answers a bare 500.
 */
import { requireUser } from "../_shared/auth.ts";
import { decryptString, encryptString, importAesKey } from "../_shared/crypto.ts";
import { authGetUser, type Rest, restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { canarySender } from "../_shared/canary_send.ts";
import { s3Client } from "../_shared/s3.ts";
import { hmacKeyFromBase64, nonceHmac, tokenHmac } from "../mail-inbound/keys.ts";
import {
  type BatchRow,
  type HeldRow,
  type MailDeps,
  mailHandler,
  type MailRow,
  type MailStore,
} from "./handler.ts";
import { mxResolver } from "./provider.ts";
import { MAX_OPEN_BATCHES } from "./routes.ts";

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
/** A Postgres array literal of timestamps, for the `mints_30d` compare-and-set. */
const arrayLiteral = (xs: string[]) => encodeURIComponent(`{${xs.map((x) => `"${x}"`).join(",")}}`);

interface AddressDb {
  id: string;
  account_id: string;
  provider: MailRow["provider"];
  status: string;
  token_hmac: string;
  token_ciphertext: string;
  token_iv: string;
  retired_token_hmac: string | null;
  retired_until: string | null;
  mailbox_hint: string | null;
  lapsed_since: string | null;
  reenabled_at: string | null;
  backfill_since: string | null;
  last_received_at: string | null;
  last_canary_ok_at: string | null;
  last_canary_sent_at: string | null;
  canaries_today: number;
  confirm_until: string | null;
  unbound: number;
  unauthenticated: number;
  expired: number;
  unbound_own_since_live: number;
  unbound_foreign_since_live: number;
}
const COLUMNS = "id,account_id,provider,status,token_hmac,token_ciphertext,token_iv,retired_token_hmac," +
  "retired_until,mailbox_hint,lapsed_since,reenabled_at,backfill_since,last_received_at,last_canary_ok_at," +
  "last_canary_sent_at,canaries_today,confirm_until,unbound,unauthenticated,expired,unbound_own_since_live," +
  "unbound_foreign_since_live";
const fromDb = (r: AddressDb): MailRow => ({
  id: r.id,
  accountId: r.account_id,
  provider: r.provider,
  status: r.status,
  tokenHmac: r.token_hmac,
  tokenCiphertext: r.token_ciphertext,
  tokenIv: r.token_iv,
  retiredTokenHmac: r.retired_token_hmac,
  retiredUntil: r.retired_until,
  mailboxHint: r.mailbox_hint,
  lapsedSince: r.lapsed_since,
  reenabledAt: r.reenabled_at,
  backfillSince: r.backfill_since,
  lastReceivedAt: r.last_received_at,
  lastCanaryOkAt: r.last_canary_ok_at,
  lastCanarySentAt: r.last_canary_sent_at,
  canariesToday: r.canaries_today,
  confirmUntil: r.confirm_until,
  unbound: r.unbound,
  unauthenticated: r.unauthenticated,
  expired: r.expired,
  unboundOwnSinceLive: r.unbound_own_since_live,
  unboundForeignSinceLive: r.unbound_foreign_since_live,
});

function storeOver(rest: Rest): MailStore {
  const mine = (accountId: string, address: string) => `id=eq.${id(address)}&account_id=eq.${id(accountId)}`;
  /** A PATCH that answers how many rows it changed (their ids only). */
  const patched = async (what: string, table: string, query: string, body: unknown) => {
    const init = { method: "PATCH", body: JSON.stringify(body) };
    return (await rows<unknown>(await call(rest, what, `${table}?${query}`, init, "return=representation")))
      .length;
  };
  return {
    async addresses(accountId) {
      const q = `account_id=eq.${id(accountId)}&select=${COLUMNS}&order=id`;
      return (await rows<AddressDb>(await call(rest, "addresses", `mail_addresses?${q}`))).map(fromDb);
    },
    async ensureAccount(accountId, fpKey) {
      const body = JSON.stringify({
        account_id: accountId,
        fp_key_ciphertext: fpKey.ciphertext,
        fp_key_iv: fpKey.iv,
      });
      const init = { method: "POST", body };
      await (await call(
        rest,
        "account",
        "mail_accounts",
        init,
        "resolution=ignore-duplicates,return=minimal",
      ))
        .body?.cancel();
    },
    async mints(accountId) {
      const q = `account_id=eq.${id(accountId)}&select=mints_30d&limit=1`;
      const [r] = await rows<{ mints_30d: string[] | null }>(await call(rest, "mints", `mail_accounts?${q}`));
      return r?.mints_30d ?? [];
    },
    async setMints(accountId, seen, next) {
      const q = `account_id=eq.${id(accountId)}&mints_30d=eq.${arrayLiteral(seen)}&select=account_id`;
      return await patched("mints", "mail_accounts", q, { mints_30d: next }) > 0;
    },
    async insertAddress(accountId, provider, token) {
      const body = JSON.stringify({
        account_id: accountId,
        provider,
        token_hmac: token.tokenHmac,
        token_ciphertext: token.tokenCiphertext,
        token_iv: token.tokenIv,
      });
      const res = await call(
        rest,
        "mint",
        "mail_addresses?select=id",
        { method: "POST", body },
        "return=representation",
      );
      const [r] = await rows<{ id: string }>(res);
      if (r === undefined) throw new Error("store mint empty");
      return r.id;
    },
    async setHint(accountId, address, hint) {
      const q = `${mine(accountId, address)}&mailbox_hint=is.null&select=id`;
      return await patched("hint", "mail_addresses", q, { mailbox_hint: hint }) > 0;
    },
    async rotate(accountId, address, fromHmac, next) {
      const q = `${mine(accountId, address)}&token_hmac=eq.${id(fromHmac)}&select=id`;
      return await patched("rotate", "mail_addresses", q, {
        token_hmac: next.tokenHmac,
        token_ciphertext: next.tokenCiphertext,
        token_iv: next.tokenIv,
        retired_token_hmac: fromHmac,
        retired_until: next.retiredUntil,
        ...(next.status === null ? {} : { status: next.status }),
      }) > 0;
    },
    async revoke(accountId, address) {
      const init = { method: "PATCH", body: JSON.stringify({ status: "revoked" }) };
      await (await call(rest, "revoke", `mail_addresses?${mine(accountId, address)}`, init, "return=minimal"))
        .body?.cancel();
    },
    async workKeys(accountId, address) {
      // Paged by id, past PostgREST's row cap: a backfill can leave thousands of items.
      const keys: string[] = [];
      for (let last = 0;;) {
        const q = `address_id=eq.${id(address)}&account_id=eq.${id(accountId)}&id=gt.${last}` +
          `&select=id,s3_key&order=id&limit=1000`;
        const page = await rows<{ id: number; s3_key: string }>(
          await call(rest, "work keys", `mail_work?${q}`),
        );
        keys.push(...page.map((p) => p.s3_key));
        if (page.length < 1000) return keys;
        last = page[page.length - 1].id;
      }
    },
    async deleteMailAddress(address) {
      const init = { method: "POST", body: JSON.stringify({ p_address: address }) };
      // `returns void`: the answer carries no body worth reading.
      await (await call(rest, "delete_mail_address", "rpc/delete_mail_address", init)).body?.cancel();
    },
    // T4.2.
    async takeCanary(accountId, address, seen, next) {
      const last = seen.lastSentAt === null
        ? "last_canary_sent_at=is.null"
        : `last_canary_sent_at=eq.${id(seen.lastSentAt)}`;
      const q = `${
        mine(accountId, address)
      }&status=neq.revoked&canaries_today=eq.${seen.count}&${last}&select=id`;
      return await patched("canary turn", "mail_addresses", q, {
        canaries_today: next.count,
        last_canary_sent_at: next.lastSentAt,
      }) > 0;
    },
    async insertCanary(address, nonceHmac, sentAt) {
      const body = JSON.stringify({ address_id: address, nonce_hmac: nonceHmac, sent_at: sentAt });
      const init = { method: "POST", body };
      const [r] = await rows<{ id: string }>(
        await call(rest, "canary", "mail_canaries?select=id", init, "return=representation"),
      );
      if (r === undefined) throw new Error("store canary empty");
      return r.id;
    },
    async deleteCanary(address, canary) {
      const q = `id=eq.${id(canary)}&address_id=eq.${id(address)}`;
      await (await call(rest, "canary undo", `mail_canaries?${q}`, { method: "DELETE" }, "return=minimal"))
        .body?.cancel();
    },
    async setBackfillSince(accountId, address, since) {
      const q = `${mine(accountId, address)}&backfill_since=is.null&select=id`;
      return await patched("backfill window", "mail_addresses", q, { backfill_since: since }) > 0;
    },
    async openBatches(address) {
      const q = `address_id=eq.${id(address)}&closed_at=is.null&select=id&limit=${MAX_OPEN_BATCHES + 1}`;
      return (await rows<{ id: string }>(await call(rest, "open batches", `mail_batches?${q}`))).length;
    },
    async insertBatch(b) {
      const body = JSON.stringify({
        address_id: b.addressId,
        phase: b.phase,
        nonce_hmac: b.nonceHmac,
        since: b.since,
        expected: b.expected,
      });
      const init = { method: "POST", body };
      const [r] = await rows<{ id: string }>(
        await call(rest, "batch", "mail_batches?select=id", init, "return=representation"),
      );
      if (r === undefined) throw new Error("store batch empty");
      return r.id;
    },
    async batch(batch) {
      const q = `id=eq.${
        id(batch)
      }&select=id,address_id,phase,since,expected,sent,received,sent_at,closed_at&limit=1`;
      const [r] = await rows<BatchDb>(await call(rest, "batch", `mail_batches?${q}`));
      return r === undefined ? null : {
        id: r.id,
        addressId: r.address_id,
        phase: r.phase,
        since: r.since,
        expected: r.expected,
        sent: r.sent,
        received: r.received,
        sentAt: r.sent_at,
        closedAt: r.closed_at,
      };
    },
    async markSent(address, batch, sent, sentAt) {
      const q = `id=eq.${id(batch)}&address_id=eq.${id(address)}&sent=is.null&closed_at=is.null&select=id`;
      return await patched("sent", "mail_batches", q, { sent, sent_at: sentAt }) > 0;
    },
    async fpKey(accountId) {
      const q = `account_id=eq.${id(accountId)}&select=fp_key_ciphertext,fp_key_iv&limit=1`;
      const [r] = await rows<{ fp_key_ciphertext: string; fp_key_iv: string }>(
        await call(rest, "fp key", `mail_accounts?${q}`),
      );
      return r === undefined ? null : { ciphertext: r.fp_key_ciphertext, iv: r.fp_key_iv };
    },
    async held(accountId, from, to) {
      // Paged past PostgREST's row cap: a backfill's window can hold thousands.
      const q =
        `account_id=eq.${id(accountId)}&received_minute=gte.${id(from)}&received_minute=lte.${id(to)}` +
        `&select=fp_hash,received_minute&order=received_minute,msg_hash&limit=1000`;
      const out: HeldRow[] = [];
      for (let offset = 0;; offset += 1000) {
        const page = await rows<{ fp_hash: string; received_minute: string }>(
          await call(rest, "held", `mail_seen?${q}&offset=${offset}`),
        );
        out.push(...page.map((p) => ({ fp: p.fp_hash, minute: p.received_minute })));
        if (page.length < 1000) return out;
      }
    },
    async setState(accountId, address, patch) {
      const body = {
        ...(patch.status === undefined ? {} : { status: patch.status }),
        ...(patch.reenabledAt === undefined ? {} : { reenabled_at: patch.reenabledAt }),
        ...(patch.confirmUntil === undefined ? {} : { confirm_until: patch.confirmUntil }),
      };
      const q = `${mine(accountId, address)}&status=neq.revoked&select=id`;
      return await patched("state", "mail_addresses", q, body) > 0;
    },
  };
}

interface BatchDb {
  id: string;
  address_id: string;
  phase: BatchRow["phase"];
  since: string;
  expected: number;
  sent: number | null;
  received: number;
  sent_at: string | null;
  closed_at: string | null;
}

async function wire(): Promise<MailDeps> {
  const rest = restFromEnv();
  const addressKey = await hmacKeyFromBase64(env("MAIL_ADDRESS_KEY"));
  const encKey = await importAesKey(env("MAIL_ENC_KEY"));
  const s3 = s3Client({
    bucket: env("MAIL_BUCKET"),
    region: env("AWS_REGION"),
    accessKeyId: env("AWS_ACCESS_KEY_ID"),
    secretAccessKey: env("AWS_SECRET_ACCESS_KEY"),
  }, { fetch: globalThis.fetch });
  return {
    authenticate: async (req) => ({ account_id: (await requireUser(req, (t) => authGetUser(rest, t))).id }),
    entitle: requireActiveEntitlement,
    store: storeOver(rest),
    s3,
    tokenHmac: (t) => tokenHmac(addressKey, t),
    encrypt: (s) => encryptString(encKey, s),
    decrypt: (c, iv) => decryptString(encKey, c, iv),
    mailDomain: env("MAIL_DOMAIN").trim().toLowerCase(),
    now: () => Date.now(),
    resolveMx: mxResolver(),
    nonceHmac: (n) => nonceHmac(addressKey, n),
    // The provider `billing-jobs` sends through (P9): its endpoint and key, named only.
    sendCanary: canarySender({
      url: env("EMAIL_API_URL"),
      key: env("EMAIL_API_KEY"),
      fetch: globalThis.fetch,
    }),
  };
}

let wired: Promise<(req: Request) => Promise<Response>> | null = null;

Deno.serve(async (req) => {
  try {
    wired ??= wire().then(mailHandler);
    const handler = await wired.catch((e) => {
      wired = null;
      throw e;
    });
    return await handler(req);
  } catch (e) {
    return asResponse(e);
  }
});
