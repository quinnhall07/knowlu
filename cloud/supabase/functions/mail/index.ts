/**
 * The wire for `mail` (email plan T4.1): the environment, the store over PostgREST, S3 (delete only, here)
 * and DNS, then `handler.ts`. `verify_jwt = false` (config.toml): the handler authenticates every request
 * itself, a session for the list and the delete and a session plus entitlement for the rest (§6.3).
 * Secrets, named only (spec §5.1): `MAIL_DOMAIN`, `MAIL_ADDRESS_KEY`, `MAIL_ENC_KEY`, `MAIL_BUCKET`,
 * `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`.
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is cancelled
 * unread (a constraint error can echo a hash), and `asResponse` answers a bare 500.
 */
import { requireUser } from "../_shared/auth.ts";
import { decryptString, encryptString, importAesKey } from "../_shared/crypto.ts";
import { authGetUser, type Rest, restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { s3Client } from "../_shared/s3.ts";
import { hmacKeyFromBase64, tokenHmac } from "../mail-inbound/keys.ts";
import { type MailDeps, mailHandler, type MailRow, type MailStore } from "./handler.ts";
import { mxResolver } from "./provider.ts";

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
  unbound: number;
  unauthenticated: number;
  expired: number;
}
const COLUMNS = "id,account_id,provider,status,token_hmac,token_ciphertext,token_iv,retired_token_hmac," +
  "retired_until,mailbox_hint,lapsed_since,reenabled_at,backfill_since,last_received_at,last_canary_ok_at," +
  "unbound,unauthenticated,expired";
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
  unbound: r.unbound,
  unauthenticated: r.unauthenticated,
  expired: r.expired,
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
  };
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
