/**
 * `mail-inbound` (email plan T2g.1; spec §4.6 steps 1–4 and 7–8, D2, D3, D8). SES stores each message in
 * S3 under `inbound/` and SNS posts a notification here; this handler verifies it, finds the mailbox,
 * checks entitlement, fetches the object, classifies it (`mail_bind.ts`), dedups and hands the original
 * off under `work/` for `mail-process`. In order:
 *
 * 1. **Verify** (`sns.ts`): a refused notification answers 403 and nothing else is read.
 * 2. **Find**, from the notification alone: each recipient at `MAIL_DOMAIN` whose token's HMAC names a
 *    live mailbox (a token retired by Rotate counts until `retired_until`). None: **deleted unread**,
 *    counted globally (a fixed log line), never per account.
 * 3. **Entitlement**: a mailbox whose account is not `active` or `trialing` is marked `lapsed` (with
 *    `lapsed_since`, kept from the first lapse); with no entitled mailbox left the message is deleted
 *    unread and no `mail_seen` row is written.
 * 4. **Fetch** the object once.
 * 5. **Classify** per mailbox. A live `aligned` forward is handed off; a live `unaligned` one is deleted
 *    and counted `unauthenticated`; `unbound` is deleted and counted on the address. A canary, a Gmail
 *    confirmation and a batch send are T2g.2's: until it lands each is deleted with nothing recorded.
 * 7. **Dedup** on `mail_seen`'s keyed `msg_hash` (`keys.ts`): already there, deleted, done.
 * 8. **Hand off**: put the original under `work/`; only after the put, `store.handOff` writes the
 *    `mail_work` and `mail_seen` rows in one transaction (the `mail_hand_off` RPC; it answers false when a
 *    concurrent delivery won, and the put is taken back out); then `last_received_at` for a live item.
 *    The `inbound/` object is deleted last, and the answer is 200. A failed put or row write answers 500
 *    with no row written and the `inbound/` object left, so SNS's retry is processed as new.
 *
 * **Privacy:** every log line is a fixed phrase. No address, token, key, header value, subject, body or
 * object key is logged, put in an error or answered; a store or S3 failure is logged by its kind only.
 */
import { type BindBatch, type BindCanary, type BindDeps, type BindMailbox, classifyArrival } from "../_shared/mail_bind.ts";
import { parseHeaders } from "../_shared/mime.ts";
import { allowedKey, type S3Client, S3Error } from "../_shared/s3.ts";
import { type SnsDeps, verifySns } from "../_shared/sns.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { seenHashes } from "./keys.ts";

export interface InboundConfig {
  /** `MAIL_SNS_TOPIC_ARN`. */
  topicArn: string;
  /** `MAIL_DOMAIN`: the domain every Knowlu address is at. */
  mailDomain: string;
  /** `MAIL_BUCKET`: a notification naming another bucket is not ours to act on. */
  bucket: string;
  /** Knowlu's canary sender (`mail_bind.ts`'s `BindConfig`). */
  canaryFrom: string;
}
/** A `mail_addresses` row as intake reads it; times are ISO strings, as PostgREST answers them. */
export interface AddressRow {
  id: string;
  accountId: string;
  provider: BindMailbox["provider"];
  status: string;
  tokenHmac: string;
  retiredTokenHmac: string | null;
  retiredUntil: string | null;
  lapsedSince: string | null;
  mailboxHint: string | null;
  confirmUntil: string | null;
  forwardHint: string | null;
}
export type Phase = "live" | "backfill" | "recover";
export interface WorkRow {
  accountId: string;
  addressId: string;
  batchId: string | null;
  s3Key: string;
  phase: Phase;
  authenticity: "aligned" | "unaligned";
}
export interface SeenRow {
  accountId: string;
  addressId: string;
  msgHash: string;
  fpHash: string;
  /** ISO, truncated to the minute. */
  receivedMinute: string;
  phase: Phase;
}
export interface InboundStore {
  /** The row whose `token_hmac` or `retired_token_hmac` is `hmac`, whatever its status; else null. */
  addressByTokenHmac(hmac: string): Promise<AddressRow | null>;
  entitled(accountId: string): Promise<boolean>;
  /** `status = 'lapsed'`, and `lapsed_since = now()` only when it is null. */
  markLapsed(addressId: string): Promise<void>;
  /** The mailbox's open batches and its recent canaries, for `classifyArrival`. */
  bindRows(addressId: string): Promise<{ batches: BindBatch[]; canaries: BindCanary[] }>;
  /** The account's fingerprint key, decrypted. */
  fpKey(accountId: string): Promise<CryptoKey>;
  seen(accountId: string, msgHash: string): Promise<boolean>;
  /** Both rows in one transaction; false (and nothing written) when the `mail_seen` row exists. */
  handOff(work: WorkRow, seen: SeenRow): Promise<boolean>;
  count(addressId: string, counter: "unbound" | "unauthenticated"): Promise<void>;
  /** `last_received_at`. */
  received(addressId: string, iso: string): Promise<void>;
}
export interface InboundDeps {
  sns: SnsDeps;
  s3: S3Client;
  store: InboundStore;
  bind: BindDeps;
  /** `keys.ts`'s `tokenHmac` under `MAIL_ADDRESS_KEY`. */
  tokenHmac(token: string): Promise<string>;
  now(): number;
  /** A fresh `work/` key; a random UUID by default. */
  workKey?(): string;
}

/** Knowlu's canary sender (spec §4.10, plan P9). The canary's sender (T4.2, `_shared/canary_send.ts`)
 * sends from exactly this address, DKIM-signed for its domain, so intake recognises what it sent. */
export const MAIL_CANARY_FROM = "canary@knowlu.com";

/** The only thing intake logs: a fixed phrase. */
const log = (phrase: string) => console.log(`mail-inbound: ${phrase}`);
const done = () => json(200, { ok: true });
const MAX_RECIPIENTS = 50;
const TOKEN = /^[a-z2-7]{26}$/;

/** What intake reads from a verified SES notification: the object, the recipients and the envelope. */
interface Receipt {
  key: string;
  recipients: string[];
  mailFrom: string | null;
  spfPass: boolean;
  /** SES's receipt time (epoch ms), never later than now. */
  arrivedAt: number;
}
const rec = (x: unknown): Record<string, unknown> | null =>
  typeof x === "object" && x !== null && !Array.isArray(x) ? x as Record<string, unknown> : null;

/** A `Received` notification of the S3 action for our bucket, naming an object under `inbound/`; else
 * null (SES's setup notification, or anything else on the topic). */
function receiptOf(message: string, cfg: InboundConfig, now: number): Receipt | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(message);
  } catch {
    return null;
  }
  const top = rec(parsed);
  const receipt = rec(top?.receipt);
  const action = rec(receipt?.action);
  const key = action?.objectKey;
  if (top?.notificationType !== "Received" || action?.type !== "S3" || action.bucketName !== cfg.bucket) return null;
  if (typeof key !== "string" || !key.startsWith("inbound/") || !allowedKey(key)) return null;
  if (!Array.isArray(receipt?.recipients)) return null;
  const recipients = receipt.recipients.filter((r): r is string => typeof r === "string").slice(0, MAX_RECIPIENTS);
  const source = rec(top.mail)?.source;
  const stamp = typeof receipt.timestamp === "string" ? Date.parse(receipt.timestamp) : NaN;
  return {
    key,
    recipients,
    mailFrom: typeof source === "string" && source !== "" ? source : null,
    spfPass: rec(receipt.spfVerdict)?.status === "PASS",
    arrivedAt: Number.isFinite(stamp) && stamp <= now ? stamp : now,
  };
}

interface Found {
  row: AddressRow;
  /** The recipient, lower-cased, whose token found the row. */
  recipient: string;
}

/** Whether `hmac` binds `row` now: its own token, or its retired one inside Rotate's grace; never revoked. */
function binds(row: AddressRow, hmac: string, now: number): boolean {
  if (row.status === "revoked") return false;
  if (row.tokenHmac === hmac) return true;
  return row.retiredTokenHmac === hmac && row.retiredUntil !== null && Date.parse(row.retiredUntil) > now;
}

/** Step 2: the mailboxes the recipients at our domain name, each once. */
async function find(n: Receipt, cfg: InboundConfig, deps: InboundDeps): Promise<Found[]> {
  const domain = cfg.mailDomain.trim().toLowerCase();
  const out: Found[] = [];
  for (const raw of n.recipients) {
    const recipient = raw.trim().toLowerCase();
    const at = recipient.lastIndexOf("@");
    if (at <= 0 || recipient.slice(at + 1) !== domain) continue;
    const token = recipient.slice(0, at);
    const hmac = TOKEN.test(token) ? await deps.tokenHmac(token) : null;
    const row = hmac === null ? null : await deps.store.addressByTokenHmac(hmac);
    if (row === null || hmac === null || !binds(row, hmac, deps.now())) {
      log("unknown or revoked recipient, deleted unread");
      continue;
    }
    if (!out.some((f) => f.row.id === row.id)) out.push({ row, recipient });
  }
  return out;
}

const minute = (ms: number) => new Date(Math.floor(ms / 60_000) * 60_000).toISOString();
const epoch = (iso: string | null) => {
  const ms = iso === null ? NaN : Date.parse(iso);
  return Number.isFinite(ms) ? ms : null;
};

/** Steps 7 and 8 for one original: dedup, put, then both rows in one transaction. True when queued. */
async function handOff(
  original: Uint8Array,
  row: AddressRow,
  item: { batchId: string | null; phase: Phase; authenticity: WorkRow["authenticity"]; receivedAt: number },
  deps: InboundDeps,
): Promise<boolean> {
  const { msgHash, fpHash } = await seenHashes(await deps.store.fpKey(row.accountId), parseHeaders(original));
  if (await deps.store.seen(row.accountId, msgHash)) {
    log("duplicate, deleted");
    return false;
  }
  const key = deps.workKey?.() ?? `work/${crypto.randomUUID()}`;
  await deps.s3.put(key, original);
  let queued: boolean;
  try {
    queued = await deps.store.handOff(
      {
        accountId: row.accountId,
        addressId: row.id,
        batchId: item.batchId,
        s3Key: key,
        phase: item.phase,
        authenticity: item.authenticity,
      },
      {
        accountId: row.accountId,
        addressId: row.id,
        msgHash,
        fpHash,
        receivedMinute: minute(item.receivedAt),
        phase: item.phase,
      },
    );
  } catch (e) {
    // No row was written: take the put back out, so nothing is left for the retry to trip over.
    await deps.s3.delete(key).catch(() => {});
    throw e;
  }
  if (!queued) {
    await deps.s3.delete(key);
    log("duplicate, deleted");
  }
  return queued;
}

/** Steps 5–8 for one mailbox the message is bound for. */
async function arrive(raw: Uint8Array, f: Found, n: Receipt, cfg: InboundConfig, deps: InboundDeps) {
  const { batches, canaries } = await deps.store.bindRows(f.row.id);
  const mailbox: BindMailbox = {
    id: f.row.id,
    provider: f.row.provider,
    mailboxHint: f.row.mailboxHint,
    confirmUntil: epoch(f.row.confirmUntil),
    forwardHint: f.row.forwardHint,
    batches,
    canaries,
  };
  const envelope = { mailFrom: n.mailFrom, spfPass: n.spfPass, recipient: f.recipient };
  const arrival = await classifyArrival(raw, mailbox, { canaryFrom: cfg.canaryFrom }, envelope, deps.bind);
  switch (arrival.kind) {
    case "unbound":
      await deps.store.count(f.row.id, "unbound");
      log("unbound, deleted");
      return;
    case "live": {
      if (arrival.authenticity !== "aligned") {
        await deps.store.count(f.row.id, "unauthenticated");
        log("unauthenticated, deleted");
        return;
      }
      const item = { batchId: null, phase: "live" as const, authenticity: arrival.authenticity };
      if (await handOff(raw, f.row, { ...item, receivedAt: n.arrivedAt }, deps)) {
        await deps.store.received(f.row.id, new Date(n.arrivedAt).toISOString());
        log("queued (live)");
      }
      return;
    }
    default:
      // A canary, a Gmail confirmation (completed or refused) and a batch send: their effects are
      // T2g.2's. Until then each is deleted with nothing recorded.
      log(`${arrival.kind}, no effect yet`);
  }
}

/** Steps 2–4 and the final delete: find, entitle, fetch once, then each bound mailbox in turn. */
async function intake(n: Receipt, cfg: InboundConfig, deps: InboundDeps): Promise<void> {
  const found = await find(n, cfg, deps);
  const entitled: Found[] = [];
  const answers = new Map<string, boolean>();
  for (const f of found) {
    let ok = answers.get(f.row.accountId);
    if (ok === undefined) answers.set(f.row.accountId, ok = await deps.store.entitled(f.row.accountId));
    if (ok) {
      entitled.push(f);
    } else {
      await deps.store.markLapsed(f.row.id);
      log("lapsed account, deleted unread");
    }
  }
  if (entitled.length > 0) {
    const raw = await deps.s3.get(n.key);
    for (const f of entitled) await arrive(raw, f, n, cfg, deps);
  }
  await deps.s3.delete(n.key);
}

/** One SNS delivery. 403 when it does not verify, 500 when a write failed (SNS retries), else 200. */
export async function handle(req: Request, cfg: InboundConfig, deps: InboundDeps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  let verified;
  try {
    verified = await verifySns(req, { topicArn: cfg.topicArn }, deps.sns);
  } catch (e) {
    const status = e instanceof Response ? e.status : 403;
    log(status === 403 ? "notification refused" : "subscription confirmation failed");
    return e instanceof Response ? e : fail(403, "forbidden");
  }
  if (verified.kind === "confirmed") {
    log("subscription confirmed");
    return done();
  }
  const n = receiptOf(verified.message, cfg, deps.now());
  if (n === null) {
    log("not a receipt, ignored");
    return done();
  }
  try {
    await intake(n, cfg, deps);
  } catch (e) {
    log(`failed (${e instanceof S3Error ? `s3 ${e.op} ${e.status}` : "store"})`);
    return fail(500, "intake failed");
  }
  return done();
}

