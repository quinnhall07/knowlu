/**
 * The device's `mail` routes (email spec §6.3; plan T4.1): the secret addresses. T4.2's routes (the
 * canary, batches, reconcile, the state and the provider) live in `routes.ts` and are dispatched here.
 *
 * - `GET /addresses` lists the account's mailboxes, each with its own address so Settings can show it
 *   again (D3), and never a token's HMAC, ciphertext or IV. Each row carries its unbound reason (Quinn's
 *   unbound-display ruling, 2026-10-03): `unbound_reason` (`skipped`, `leaked` or null), `unbound_note`
 *   (the sentence), `unbound_since_live` and `suggest_rotate` (`unboundReason`).
 * - `POST /addresses {provider}` mints one (§4.1 step 2): 128 random bits in lower-case base32, kept as
 *   `keys.ts`'s HMAC under `MAIL_ADDRESS_KEY` and as AES-GCM ciphertext under `MAIL_ENC_KEY`. A fourth live
 *   address (409, `LIVE_CAP`) or a seventh mint in 30 days (429, `MINT_CAP`) is refused by name. The first
 *   mint opens the account's `mail_accounts` row with a fresh fingerprint key (§4.6 step 7).
 * - `POST /addresses/<id>/hint {hint}` sets `mailbox_hint` once, to an address that fits the mailbox's
 *   provider (P21; `provider.ts`'s `hintFits`: only a provable mismatch is refused, so a school behind a
 *   filtering gateway's MX still fits) and that none of the account's other mailboxes holds.
 * - `POST /addresses/<id>/rotate` mints a new token for the mailbox and retires the old one for seven
 *   days (D3); intake ends that grace early when a canary comes back through the new address. On Gmail
 *   the mailbox reads `pending` again, since the new address needs its own confirm (§4.3 steps 3–5). The
 *   same update zeroes both since-live counts, so a `leaked` row stops suggesting Rotate once rotated.
 * - `DELETE /addresses/<id>` revokes and purges (§4.12 step 2, D17): revoke, read the mailbox's `work/`
 *   keys, delete those objects, then `delete_mail_address`.
 *
 * **Auth (§6.3, test 16b):** the list and the delete need a session only, so a lapsed or canceled account
 * can always see and disconnect its mailboxes; every other request, an unknown route included, passes the
 * entitlement gate first. Every row is read by the caller's account, so another account's id reads as
 * not found. **Nothing here logs** a token, an address, a hint or a key: errors leave as a status and a
 * fixed sentence (`_shared/http.ts`).
 */
import type { SendCanary } from "../_shared/canary_send.ts";
import type { BatchClock } from "../_shared/mail_batch.ts";
import type { Entitle } from "../_shared/judge_handler.ts";
import { asResponse, fail, json, methodNotAllowed, readJson, subPath } from "../_shared/http.ts";
import type { S3Client } from "../_shared/s3.ts";
import { DOMAIN, hintFits, type MailProvider, PROVIDERS, type ResolveMx } from "./provider.ts";
import { batchSent, canary, openBatch, provider, reconcile, setState } from "./routes.ts";

/** D3's caps, by name: the device shows these sentences as they are. */
export const LIVE_CAP = "this account already has three live mail addresses";
export const MINT_CAP = "this account has made six mail addresses in the last 30 days";
export const MAX_LIVE = 3;
export const MAX_MINTS = 6;
export const MINT_WINDOW_MS = 30 * 86_400_000;
/** Rotate's grace (D3): the old token binds until the new address's canary, at most this long. */
export const ROTATE_GRACE_MS = 7 * 86_400_000;
/** Quinn's unbound-display ruling (2026-10-03): this many unbound live forwards with no bound live forward
 * between give the row a reason. */
export const UNBOUND_STREAK = 2;
/** The reason when they came from the student's own school or provider, while the canary binds. */
export const UNBOUND_SKIPPED = "some forwarded mail could not be verified and was skipped";
/** The reason when `UNBOUND_STREAK` of them came from outside: the address may have leaked. */
export const UNBOUND_LEAKED =
  "mail that did not come through your forwarding is reaching this address; Rotate gives it a new one";
/** The statuses in which a canary has come back through the forwarding (D14). */
const CANARY_BINDS = ["active", "quiet"];

/**
 * The row's unbound reason (Quinn's unbound-display ruling, 2026-10-03). `leaked`, the only one that
 * suggests Rotate, when `UNBOUND_STREAK` unbound live forwards from outside the student's own school or
 * provider arrived with no bound one between, whatever the forwarding's state. Else `skipped` when that
 * many arrived from anywhere while the canary binds (`active`, `quiet`); before it binds, or once
 * forwarding is off or lapsed, the status is the explanation. The count is every unbound live forward
 * since the last bound one.
 */
function unboundReason(r: MailRow) {
  const since = r.unboundOwnSinceLive + r.unboundForeignSinceLive;
  const reason = r.unboundForeignSinceLive >= UNBOUND_STREAK
    ? "leaked" as const
    : since >= UNBOUND_STREAK && CANARY_BINDS.includes(r.status)
    ? "skipped" as const
    : null;
  return {
    unbound_reason: reason,
    unbound_note: reason === "leaked" ? UNBOUND_LEAKED : reason === "skipped" ? UNBOUND_SKIPPED : null,
    unbound_since_live: since,
    suggest_rotate: reason === "leaked",
  };
}

/** A `mail_addresses` row as this function reads it; times are ISO strings, as PostgREST answers them. */
export interface MailRow {
  id: string;
  accountId: string;
  provider: MailProvider;
  status: string;
  tokenHmac: string;
  tokenCiphertext: string;
  tokenIv: string;
  retiredTokenHmac: string | null;
  retiredUntil: string | null;
  mailboxHint: string | null;
  lapsedSince: string | null;
  reenabledAt: string | null;
  backfillSince: string | null;
  lastReceivedAt: string | null;
  lastCanaryOkAt: string | null;
  /** §4.10's limit: canaries sent on the UTC day of `lastCanarySentAt`. */
  lastCanarySentAt: string | null;
  canariesToday: number;
  /** Gmail's confirm window (P21). */
  confirmUntil: string | null;
  unbound: number;
  unauthenticated: number;
  expired: number;
  /** Unbound live forwards since the last bound one, from the student's own school or provider. */
  unboundOwnSinceLive: number;
  /** The same, from anywhere else. */
  unboundForeignSinceLive: number;
}
/** What Rotate writes in one update. Both since-live counts go back to 0 with the old token (§9.5's Rotate
 * row, open for Quinn): they described the old address, and leaving them would keep "Rotate gives it a new
 * one" on the row after the student has rotated. The cumulative `unbound` stays. */
export type RotateNext = NewToken & {
  retiredUntil: string;
  status: string | null;
  unboundOwnSinceLive: 0;
  unboundForeignSinceLive: 0;
};
/** A `mail_batches` row as `POST /batches/<id>/sent` reads it (never its nonce's HMAC). */
export interface BatchRow {
  id: string;
  addressId: string;
  phase: "backfill" | "recover";
  since: string;
  expected: number;
  sent: number | null;
  received: number;
  sentAt: string | null;
  closedAt: string | null;
}
/** A `mail_batches` row as the open-batch cap reads it. */
export interface OpenBatch extends BatchClock {
  id: string;
}
export interface NewBatch {
  addressId: string;
  phase: "backfill" | "recover";
  nonceHmac: string;
  /** ISO instant: the window's start. */
  since: string;
  expected: number;
}
/** One `mail_seen` entry as reconcile answers it: `fp_hash` and `received_minute` (§4.11 step 3). */
export interface HeldRow {
  fp: string;
  minute: string;
}
/** What `POST /addresses/<id>/state` writes. */
export interface StatePatch {
  status?: "off";
  reenabledAt?: string;
  confirmUntil?: string;
}
/** A token as it is kept: its lookup HMAC and its ciphertext with the IV. */
export interface NewToken {
  tokenHmac: string;
  tokenCiphertext: string;
  tokenIv: string;
}
export interface MailStore {
  /** Every `mail_addresses` row of the account, whatever its status, in a stable order. */
  addresses(accountId: string): Promise<MailRow[]>;
  /** The account's `mail_accounts` row with this fingerprint key, unless it already has one. */
  ensureAccount(accountId: string, fpKey: { ciphertext: string; iv: string }): Promise<void>;
  /** `mail_accounts.mints_30d`, as stored. */
  mints(accountId: string): Promise<string[]>;
  /** Writes `next` only while the column still holds `seen`; false, with nothing written, otherwise. */
  setMints(accountId: string, seen: string[], next: string[]): Promise<boolean>;
  /** A new `pending` mailbox; its id. */
  insertAddress(accountId: string, provider: MailProvider, token: NewToken): Promise<string>;
  /** Sets the hint only while it is null; false, with nothing written, otherwise. */
  setHint(accountId: string, id: string, hint: string): Promise<boolean>;
  /** Retires `fromHmac` until `retiredUntil` and keeps `next` (its token, both since-live counts, and
   * `status` when not null) only while the row's token is still `fromHmac`; false, with nothing written,
   * otherwise (a concurrent Rotate won). */
  rotate(accountId: string, id: string, fromHmac: string, next: RotateNext): Promise<boolean>;
  /** `status = 'revoked'`. */
  revoke(accountId: string, id: string): Promise<void>;
  /** The mailbox's `mail_work` S3 keys, all of them. */
  workKeys(accountId: string, id: string): Promise<string[]>;
  /** The `delete_mail_address` RPC (D17): every row that serves the mailbox, then the mailbox. */
  deleteMailAddress(id: string): Promise<void>;
  // T4.2.
  /** Writes `next` to `canaries_today` and `last_canary_sent_at` only while they still hold `seen`. */
  takeCanary(
    accountId: string,
    id: string,
    seen: { count: number; lastSentAt: string | null },
    next: { count: number; lastSentAt: string },
  ): Promise<boolean>;
  /** A new `mail_canaries` row; its id. */
  insertCanary(addressId: string, nonceHmac: string, sentAt: string): Promise<string>;
  deleteCanary(addressId: string, id: string): Promise<void>;
  /** Sets `backfill_since` (a date) only while it is null; false, with nothing written, otherwise. */
  setBackfillSince(accountId: string, id: string, since: string): Promise<boolean>;
  /** The mailbox's batches with `closed_at` null, oldest first, at most `limit`, as §4.8's rules read them
   * (`_shared/mail_batch.ts`): some may be closed by those rules and not yet stamped. */
  unstampedBatches(addressId: string, limit: number): Promise<OpenBatch[]>;
  /** Stamps `closed_at` on the mailbox's batch only while it is null; false, with nothing written,
   * otherwise. */
  closeBatch(addressId: string, id: string, atIso: string): Promise<boolean>;
  insertBatch(b: NewBatch): Promise<string>;
  /** One batch by id, whichever account's: the caller checks its mailbox is the account's. */
  batch(id: string): Promise<BatchRow | null>;
  /** Writes `sent` and `sent_at` only while the batch is open and unsent; false otherwise. */
  markSent(addressId: string, id: string, sent: number, sentAt: string): Promise<boolean>;
  /** `mail_accounts`' encrypted fingerprint key, or null when the account has none. */
  fpKey(accountId: string): Promise<{ ciphertext: string; iv: string } | null>;
  /** The account's `mail_seen` entries whose received minute lies in `[from, to]`. */
  held(accountId: string, from: string, to: string): Promise<HeldRow[]>;
  /** Applies `patch` while the mailbox is the account's and not revoked; false otherwise. */
  setState(accountId: string, id: string, patch: StatePatch): Promise<boolean>;
  /** Sets `lapsed_since` null only while it is still the instant `stamp` and the mailbox is the account's
   * and neither `lapsed` nor `revoked`; false, with nothing written, otherwise. */
  clearLapsed(accountId: string, id: string, stamp: string): Promise<boolean>;
}
export interface MailDeps {
  /** A session alone: `GET /addresses` and `DELETE /addresses/<id>`. */
  authenticate: Entitle;
  /** A session and an active entitlement: everything else. */
  entitle: Entitle;
  store: MailStore;
  s3: Pick<S3Client, "delete">;
  /** `keys.ts`'s `tokenHmac` under `MAIL_ADDRESS_KEY`. */
  tokenHmac(token: string): Promise<string>;
  /** AES-GCM under `MAIL_ENC_KEY` (`_shared/crypto.ts`). */
  encrypt(plain: string): Promise<{ ciphertext: string; iv: string }>;
  decrypt(ciphertext: string, iv: string): Promise<string>;
  /** `MAIL_DOMAIN`. */
  mailDomain: string;
  now(): number;
  resolveMx: ResolveMx;
  /** `keys.ts`'s `nonceHmac` under `MAIL_ADDRESS_KEY`: how batch and canary nonces are kept. */
  nonceHmac(nonce: string): Promise<string>;
  /** `_shared/canary_send.ts`'s sender over Resend. */
  sendCanary: SendCanary;
  /** `mail_bind.ts`'s `newNonce`, unless a test pins one. */
  newNonce?: () => string;
}

const B32 = "abcdefghijklmnopqrstuvwxyz234567";
/** D3's token: 128 random bits as lower-case base32 with no padding, 26 characters. */
export function newToken(bytes: Uint8Array = crypto.getRandomValues(new Uint8Array(16))): string {
  let out = "";
  let value = 0;
  let bits = 0;
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
    value &= (1 << bits) - 1;
  }
  return bits > 0 ? out + B32[(value << (5 - bits)) & 31] : out;
}

/** A fresh fingerprint key `k` (§4.6 step 7): 32 random bytes, base64, as intake's `fpKey` reads it. */
function newFpKey(): string {
  return btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(32))));
}

const iso = (ms: number) => new Date(ms).toISOString();
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const ONE = /^\/addresses\/([^/]+)$/;
const ACTION = /^\/addresses\/([^/]+)\/(rotate|hint|state)$/;
const SENT = /^\/batches\/([^/]+)\/sent$/;
/** T4.2's fixed routes: path, method, handler. */
const FIXED: Record<
  string,
  [string, (req: Request, accountId: string, deps: MailDeps) => Promise<Response>]
> = {
  "/canary": ["POST", canary],
  "/batches": ["POST", openBatch],
  "/reconcile": ["POST", reconcile],
  "/provider": ["GET", (req, _accountId, deps) => provider(req, deps)],
};
const live = (rows: MailRow[]) => rows.filter((r) => r.status !== "revoked").length;
const NOT_FOUND = "no such mailbox";

async function keep(token: string, deps: MailDeps): Promise<NewToken> {
  const box = await deps.encrypt(token);
  return { tokenHmac: await deps.tokenHmac(token), tokenCiphertext: box.ciphertext, tokenIv: box.iv };
}

/** One of the account's own mailboxes by its id; anything else, another account's included, is a 404. */
async function find(
  accountId: string,
  id: string,
  deps: MailDeps,
): Promise<{ row: MailRow; rows: MailRow[] }> {
  if (!UUID.test(id)) throw fail(404, NOT_FOUND);
  const rows = await deps.store.addresses(accountId);
  const row = rows.find((r) => r.id === id && r.accountId === accountId);
  if (row === undefined) throw fail(404, NOT_FOUND);
  return { row, rows };
}

/** Takes one of the account's six mints in 30 days, pruning the ones that left the window. */
async function takeMint(accountId: string, deps: MailDeps): Promise<void> {
  const now = deps.now();
  for (let tries = 0; tries < 8; tries++) {
    const seen = await deps.store.mints(accountId);
    const kept = seen.filter((t) => Date.parse(t) > now - MINT_WINDOW_MS);
    if (kept.length >= MAX_MINTS) throw fail(429, MINT_CAP);
    if (await deps.store.setMints(accountId, seen, [...kept, iso(now)])) return;
  }
  throw fail(503, "the account is busy; try again");
}

async function view(r: MailRow, deps: MailDeps) {
  return {
    address_id: r.id,
    provider: r.provider,
    address: `${await deps.decrypt(r.tokenCiphertext, r.tokenIv)}@${deps.mailDomain}`,
    mailbox_hint: r.mailboxHint,
    status: r.status,
    grace_until: r.retiredUntil,
    lapsed_since: r.lapsedSince,
    reenabled_at: r.reenabledAt,
    backfill_since: r.backfillSince,
    last_received_at: r.lastReceivedAt,
    last_canary_ok_at: r.lastCanaryOkAt,
    unbound: r.unbound,
    unauthenticated: r.unauthenticated,
    expired: r.expired,
    ...unboundReason(r),
  };
}

/** Whether `s` (trimmed, lower-cased) reads as one mailbox address on a dotted domain. */
function isAddress(s: string): boolean {
  if (s.length > 254) return false;
  const at = s.indexOf("@");
  if (at < 1 || at > 64 || at !== s.lastIndexOf("@")) return false;
  return /^[!-~]+$/.test(s.slice(0, at)) && DOMAIN.test(s.slice(at + 1));
}

async function mint(req: Request, accountId: string, deps: MailDeps): Promise<Response> {
  const body = await readJson<{ provider?: unknown } | null>(req, 4096);
  const provider = body?.provider;
  if (typeof provider !== "string" || !PROVIDERS.includes(provider as MailProvider)) {
    throw fail(400, "provider must be 'm365', 'google' or 'other'");
  }
  if (live(await deps.store.addresses(accountId)) >= MAX_LIVE) throw fail(409, LIVE_CAP);
  await deps.store.ensureAccount(accountId, await deps.encrypt(newFpKey()));
  await takeMint(accountId, deps);
  const token = newToken();
  const id = await deps.store.insertAddress(accountId, provider as MailProvider, await keep(token, deps));
  // Two mints at once can each see two live addresses: the one whose recount finds more than three takes
  // its own (fresh, unused) row back, so the cap holds, and its mint stays spent.
  if (live(await deps.store.addresses(accountId)) > MAX_LIVE) {
    await deps.store.deleteMailAddress(id);
    throw fail(409, LIVE_CAP);
  }
  return json(201, { address_id: id, address: `${token}@${deps.mailDomain}`, provider, status: "pending" });
}

async function rotate(accountId: string, id: string, deps: MailDeps): Promise<Response> {
  const { row } = await find(accountId, id, deps);
  if (row.status === "revoked") throw fail(404, NOT_FOUND);
  await takeMint(accountId, deps);
  const token = newToken();
  const until = iso(deps.now() + ROTATE_GRACE_MS);
  const next = {
    ...await keep(token, deps),
    retiredUntil: until,
    status: row.provider === "google" ? "pending" : null,
    unboundOwnSinceLive: 0 as const,
    unboundForeignSinceLive: 0 as const,
  };
  if (!await deps.store.rotate(accountId, row.id, row.tokenHmac, next)) {
    throw fail(409, "the mailbox's address changed meanwhile; try again");
  }
  return json(200, { address_id: row.id, address: `${token}@${deps.mailDomain}`, grace_until: until });
}

async function hint(req: Request, accountId: string, id: string, deps: MailDeps): Promise<Response> {
  const body = await readJson<{ hint?: unknown } | null>(req, 4096);
  const value = typeof body?.hint === "string" ? body.hint.trim().toLowerCase() : "";
  if (!isAddress(value)) throw fail(400, "hint must be one email address");
  const { row, rows } = await find(accountId, id, deps);
  if (row.status === "revoked") throw fail(404, NOT_FOUND);
  const ALREADY = "this mailbox's own address is already set";
  if (row.mailboxHint !== null) throw fail(409, ALREADY);
  let fits: boolean;
  try {
    fits = await hintFits(row.provider, value.slice(value.indexOf("@") + 1), deps.resolveMx);
  } catch {
    throw fail(503, "the address's mail domain could not be checked; try again");
  }
  if (!fits) throw fail(422, "this address does not fit the mailbox's provider");
  if (rows.some((r) => r.id !== row.id && r.status !== "revoked" && r.mailboxHint === value)) {
    throw fail(409, "another of this account's mailboxes has this address");
  }
  if (!await deps.store.setHint(accountId, row.id, value)) throw fail(409, ALREADY);
  return json(200, { address_id: row.id, mailbox_hint: value });
}

/** D17's purge of one mailbox the caller has already scoped to its account: revoke (intake binds nothing
 * new from here on), read its `work/` keys and delete those objects, then `delete_mail_address`. A failed
 * object delete stops before the RPC, so the row stays (revoked) and a retry finishes the purge. */
export async function purgeAddress(
  deps: { store: Pick<MailStore, "revoke" | "workKeys" | "deleteMailAddress">; s3: Pick<S3Client, "delete"> },
  accountId: string,
  id: string,
): Promise<void> {
  await deps.store.revoke(accountId, id);
  const keys = await deps.store.workKeys(accountId, id);
  try {
    for (let at = 0; at < keys.length; at += 10) {
      await Promise.all(keys.slice(at, at + 10).map((k) => deps.s3.delete(k)));
    }
  } catch {
    throw fail(502, "the mailbox's stored mail could not be deleted; try again");
  }
  await deps.store.deleteMailAddress(id);
}

export function mailHandler(deps: MailDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      const path = subPath(req.url, "mail");
      // The list and the delete need a session alone; every other request is entitled first (16b).
      const ungated = (req.method === "GET" && path === "/addresses") ||
        (req.method === "DELETE" && ONE.test(path));
      const { account_id } = await (ungated ? deps.authenticate : deps.entitle)(req);
      if (path === "/addresses") {
        if (req.method === "GET") {
          const rows = await deps.store.addresses(account_id);
          return json(200, { addresses: await Promise.all(rows.map((r) => view(r, deps))) });
        }
        if (req.method === "POST") return await mint(req, account_id, deps);
        return methodNotAllowed(["GET", "POST"]);
      }
      const one = ONE.exec(path);
      if (one !== null) {
        if (req.method !== "DELETE") return methodNotAllowed(["DELETE"]);
        const { row } = await find(account_id, one[1], deps);
        await purgeAddress(deps, account_id, row.id);
        return json(200, { deleted: true });
      }
      const action = ACTION.exec(path);
      if (action !== null) {
        if (req.method !== "POST") return methodNotAllowed(["POST"]);
        if (action[2] === "state") return await setState(req, account_id, action[1], deps);
        return action[2] === "rotate"
          ? await rotate(account_id, action[1], deps)
          : await hint(req, account_id, action[1], deps);
      }
      const fixed = Object.hasOwn(FIXED, path) ? FIXED[path] : undefined;
      if (fixed !== undefined) {
        if (req.method !== fixed[0]) return methodNotAllowed([fixed[0]]);
        return await fixed[1](req, account_id, deps);
      }
      const sent = SENT.exec(path);
      if (sent !== null) {
        if (req.method !== "POST") return methodNotAllowed(["POST"]);
        return await batchSent(req, account_id, sent[1], deps);
      }
      return fail(404, "no such route");
    } catch (e) {
      return asResponse(e);
    }
  };
}
