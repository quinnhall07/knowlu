/**
 * The device's `mail` routes, part 2 (email spec §6.3, §4.2, §4.10, §4.11; plan T4.2). `handler.ts`
 * authenticates and entitles every request here before it is dispatched, and every row is read by the
 * caller's account, so another account's id reads as not found.
 *
 * - `POST /canary {address_id}` sends one delivery check to the mailbox's own address (§4.10's limits): only
 *   to a hint that fits the mailbox's provider (`provider.ts`'s `hintFits`, the hint route's one test), and
 *   at most three per address per UTC day (`canaries_today` on `last_canary_sent_at`'s day, a compare-and-set).
 *   The turn is taken before the send, so a refused send spends it; the canary row is taken back.
 * - `POST /batches {address_id, phase, since, expected}` opens a backfill or recover batch and answers its
 *   nonce, the only time it leaves (kept as its HMAC). BW: a backfill starts on a date at most 120 days
 *   before the service's UTC day, less one for a device whose local day is behind UTC's; the first one
 *   fixes `backfill_since`, and every later backfill must start there. A recover starts at an instant no
 *   earlier than `backfill_since` (or, before the first backfill, that backfill's own bound), so a device
 *   bug cannot forward mail from before the window; and no more than 180 days back, the `mail_seen`
 *   horizon, past which nothing could be told apart from what was already processed. At most ten open
 *   batches a mailbox (intake reads twenty), open by §4.8's rules at the time (`_shared/mail_batch.ts`):
 *   the ones those rules close are stamped `closed_at` here, as the pull stamps them.
 * - `POST /batches/<id>/sent {sent}` records the device's count once (`sent`, `sent_at`); the batch stays
 *   open for intake until the pull (or the mailbox's next `POST /batches`) sees it closed, or the sweep
 *   times it out.
 * - `POST /reconcile {address_id, since, until}` answers the account's fingerprint key and the held
 *   `(fp_hash, received_minute)` pairs in the window widened by the 15-minute match (§4.11 step 3). The
 *   window starts no earlier than a recover may, and spans at most 181 days. The body may carry nothing
 *   else, so nothing about a message the service does not hold reaches it.
 * - `POST /addresses/<id>/state {state}`: `off` and `elsewhere` set `off`, `reenabled` stamps
 *   `reenabled_at`, `confirming` opens a Gmail mailbox's 15-minute confirm window (P21).
 * - `GET /provider?domain=` reads the domain's MX (PS: the device asks only when no curated row names it).
 *
 * **Nothing here logs.** Refusals are fixed sentences; none echoes a body, a hint, a nonce or a key.
 */
import { newNonce } from "../_shared/mail_bind.ts";
import { fail, json, readJson } from "../_shared/http.ts";
import { isClosed } from "../_shared/mail_batch.ts";
import type { MailDeps, MailRow, StatePatch } from "./handler.ts";
import { DOMAIN, hintFits, providerOf } from "./provider.ts";

export const MAX_CANARIES_PER_DAY = 3;
export const CANARY_CAP = "this mailbox has had three delivery checks today";
export const NO_HINT = "this mailbox's own address is not set";
export const HINT_UNFIT = "this address does not fit the mailbox's provider";
export const BACKFILL_DAYS = 120;
export const BACKFILL_TOO_WIDE = "a backfill reaches back at most 120 days";
/** How far back a recover may reach: `mail_seen`'s 180 days. */
export const RECOVER_DAYS = 180;
export const RECOVER_TOO_WIDE = "a recover reaches back at most 180 days";
export const RECOVER_BEFORE_WINDOW = "a recover starts no earlier than this mailbox's backfill window";
export const RECONCILE_BEFORE_WINDOW = "a reconcile starts no earlier than this mailbox's backfill window";
export const MAX_OPEN_BATCHES = 10;
export const OPEN_BATCH_CAP = "this mailbox has ten open batches";
/** The most unstamped batches the cap reads; the cap keeps a mailbox's set to ten. */
export const OPEN_READ = 100;
export const MAX_COUNT = 100_000;
export const CONFIRM_WINDOW_MS = 15 * 60_000;
/** §4.11 step 3: a listed message is held when a held entry's minute is within this of its own. */
export const MATCH_MS = 15 * 60_000;
export const MAX_WINDOW_DAYS = 181;

const DAY = 86_400_000;
const NOT_FOUND = "no such mailbox";
const DNS_DOWN = "the address's mail domain could not be checked; try again";
const BUSY = "the mailbox is busy; try again";
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const DATE = /^\d{4}-\d{2}-\d{2}$/;
const INSTANT = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d{1,9})?)?(?:Z|[+-]\d{2}:\d{2})$/;

const iso = (ms: number) => new Date(ms).toISOString();
const utcDay = (ms: number) => Math.floor(ms / DAY) * DAY;

/** A real `YYYY-MM-DD` date as its UTC midnight, or null. */
function dateMs(v: unknown): number | null {
  if (typeof v !== "string" || !DATE.test(v)) return null;
  const ms = Date.parse(`${v}T00:00:00Z`);
  return Number.isFinite(ms) && iso(ms).slice(0, 10) === v ? ms : null;
}
/** An ISO date-time with its zone (or a date, read as its UTC midnight), or null. */
function instantMs(v: unknown): number | null {
  if (typeof v !== "string") return null;
  if (DATE.test(v)) return dateMs(v);
  if (!INSTANT.test(v)) return null;
  const ms = Date.parse(v);
  return Number.isFinite(ms) ? ms : null;
}
const isCount = (v: unknown): v is number =>
  Number.isSafeInteger(v) && (v as number) >= 0 && (v as number) <= MAX_COUNT;

/** The body as an object holding no key outside `keys`; anything else is a 400 that echoes nothing. */
async function body(req: Request, keys: readonly string[]): Promise<Record<string, unknown>> {
  const b = await readJson<unknown>(req, 4096);
  if (b === null || typeof b !== "object" || Array.isArray(b)) throw fail(400, "body must be a JSON object");
  if (Object.keys(b).some((k) => !keys.includes(k))) {
    throw fail(400, "the body carries a field this route does not take");
  }
  return b as Record<string, unknown>;
}

/** One of the account's own live mailboxes; anything else, another account's or a revoked one, is a 404. */
async function own(accountId: string, id: unknown, deps: MailDeps): Promise<MailRow> {
  if (typeof id !== "string" || !UUID.test(id)) throw fail(404, NOT_FOUND);
  const row = (await deps.store.addresses(accountId))
    .find((r) => r.id === id && r.accountId === accountId && r.status !== "revoked");
  if (row === undefined) throw fail(404, NOT_FOUND);
  return row;
}

// ---------------------------------------------------------------- POST /canary

/** Takes one of the address's three canaries of the UTC day `now` falls in. */
async function takeCanary(accountId: string, first: MailRow, now: number, deps: MailDeps): Promise<void> {
  let row = first;
  for (let tries = 0; tries < 8; tries++) {
    const last = row.lastCanarySentAt === null ? NaN : Date.parse(row.lastCanarySentAt);
    const today = Number.isFinite(last) && utcDay(last) === utcDay(now) ? row.canariesToday : 0;
    if (today >= MAX_CANARIES_PER_DAY) throw fail(429, CANARY_CAP);
    const seen = { count: row.canariesToday, lastSentAt: row.lastCanarySentAt };
    if (await deps.store.takeCanary(accountId, row.id, seen, { count: today + 1, lastSentAt: iso(now) })) {
      return;
    }
    row = await own(accountId, row.id, deps);
  }
  throw fail(503, BUSY);
}

export async function canary(req: Request, accountId: string, deps: MailDeps): Promise<Response> {
  const b = await body(req, ["address_id"]);
  const row = await own(accountId, b.address_id, deps);
  const hint = row.mailboxHint;
  if (hint === null) throw fail(409, NO_HINT);
  let fits: boolean;
  try {
    fits = await hintFits(row.provider, hint.slice(hint.lastIndexOf("@") + 1), deps.resolveMx);
  } catch {
    throw fail(503, DNS_DOWN);
  }
  if (!fits) throw fail(422, HINT_UNFIT);
  const now = deps.now();
  await takeCanary(accountId, row, now, deps);
  const nonce = (deps.newNonce ?? newNonce)();
  const sentAt = iso(now);
  const id = await deps.store.insertCanary(row.id, await deps.nonceHmac(nonce), sentAt);
  try {
    await deps.sendCanary(hint, nonce);
  } catch {
    // Never sent: take the row back so the periodic check never waits on it. The turn stays spent.
    await deps.store.deleteCanary(row.id, id).catch(() => {});
    throw fail(502, "the delivery check could not be sent; try again");
  }
  return json(201, { canary_id: id, address_id: row.id, sent_at: sentAt });
}

// ---------------------------------------------------------------- POST /batches

/**
 * The earliest instant a recover or reconcile of the mailbox may start (spec D7 as amended 2026-10-02:
 * every resume or recover of the backfill reads the same start). Mail after setup is later still, so
 * nothing either routine needs lies before it: `backfill_since`'s UTC midnight once the first backfill
 * fixed it, else the bound that first backfill would have.
 */
function windowFloor(row: MailRow, now: number): number {
  const fixed = row.backfillSince === null ? null : dateMs(row.backfillSince);
  return fixed ?? utcDay(now) - (BACKFILL_DAYS + 1) * DAY;
}

/** The batch's window start (ISO) under BW's rules; fixes `backfill_since` on a mailbox's first backfill. */
async function windowStart(
  accountId: string,
  row: MailRow,
  phase: "backfill" | "recover",
  since: unknown,
  now: number,
  deps: MailDeps,
): Promise<string> {
  if (phase === "recover") {
    const at = instantMs(since);
    if (at === null || at > now) throw fail(400, "since must be a past date or time");
    if (at < windowFloor(row, now)) throw fail(422, RECOVER_BEFORE_WINDOW);
    if (at < now - RECOVER_DAYS * DAY) throw fail(422, RECOVER_TOO_WIDE);
    return iso(at);
  }
  const day = dateMs(since);
  if (day === null || day > utcDay(now) + DAY) {
    throw fail(400, "a backfill's since must be a date, YYYY-MM-DD");
  }
  let fixed = row.backfillSince;
  if (fixed === null) {
    // 120 days before the setup day, which is the device's own: at most one day behind the UTC day.
    if (day < utcDay(now) - (BACKFILL_DAYS + 1) * DAY) throw fail(422, BACKFILL_TOO_WIDE);
    fixed = await deps.store.setBackfillSince(accountId, row.id, since as string)
      ? since as string
      : (await own(accountId, row.id, deps)).backfillSince; // a concurrent first batch fixed it
  }
  if (fixed !== since) throw fail(409, `this mailbox's backfill starts ${fixed}`);
  return iso(day);
}

/**
 * How many of the mailbox's batches are open at `now` by §4.8's rules (`_shared/mail_batch.ts`), the same
 * ones the pull reads: a backfill opens and closes many batches between two slots, so `closed_at` alone
 * would refuse a mailbox whose batches have all closed. Each one found closed is stamped as the pull stamps
 * it, before the cap is read, so the mailbox's unstamped set stays within the cap and intake's read of
 * twenty sees every batch that can still bind. The set is read oldest first, so were it ever past
 * `OPEN_READ` (a race between opens), the oldest, the likeliest closed, are the ones stamped.
 */
async function openBatches(address: string, now: number, deps: MailDeps): Promise<number> {
  const unstamped = await deps.store.unstampedBatches(address, OPEN_READ);
  let open = 0;
  for (const b of unstamped) {
    if (isClosed(b, now)) await deps.store.closeBatch(address, b.id, iso(now));
    else open++;
  }
  return open;
}

export async function openBatch(req: Request, accountId: string, deps: MailDeps): Promise<Response> {
  const b = await body(req, ["address_id", "phase", "since", "expected"]);
  const phase = b.phase;
  if (phase !== "backfill" && phase !== "recover") throw fail(400, "phase must be 'backfill' or 'recover'");
  if (!isCount(b.expected)) throw fail(400, "expected must be a whole number from 0 to 100000");
  const now = deps.now();
  const row = await own(accountId, b.address_id, deps);
  // A batch binds by its sends' From, the mailbox's own address: without it nothing could ever bind.
  if (row.mailboxHint === null) throw fail(409, NO_HINT);
  if (await openBatches(row.id, now, deps) >= MAX_OPEN_BATCHES) throw fail(409, OPEN_BATCH_CAP);
  const since = await windowStart(accountId, row, phase, b.since, now, deps);
  const nonce = (deps.newNonce ?? newNonce)();
  const id = await deps.store.insertBatch({
    addressId: row.id,
    phase,
    nonceHmac: await deps.nonceHmac(nonce),
    since,
    expected: b.expected,
  });
  return json(201, { batch_id: id, address_id: row.id, phase, nonce, since: b.since, expected: b.expected });
}

// ---------------------------------------------------------------- POST /batches/<id>/sent

export async function batchSent(
  req: Request,
  accountId: string,
  batchId: string,
  deps: MailDeps,
): Promise<Response> {
  const b = await body(req, ["sent"]);
  if (!isCount(b.sent)) throw fail(400, "sent must be a whole number from 0 to 100000");
  const NO_BATCH = "no such batch";
  if (!UUID.test(batchId)) throw fail(404, NO_BATCH);
  const batch = await deps.store.batch(batchId);
  if (batch === null) throw fail(404, NO_BATCH);
  // The batch is the account's only through its mailbox (`mail_batches` has no account column).
  await own(accountId, batch.addressId, deps).catch((e) => {
    throw e instanceof Response && e.status === 404 ? fail(404, NO_BATCH) : e;
  });
  const answer = (sent: number, sentAt: string | null) =>
    json(200, {
      batch_id: batch.id,
      expected: batch.expected,
      sent,
      sent_at: sentAt,
      received: batch.received,
    });
  if (batch.sent !== null) {
    if (batch.sent === b.sent) return answer(batch.sent, batch.sentAt);
    throw fail(409, "this batch's count was already sent");
  }
  if (batch.closedAt !== null) throw fail(409, "this batch timed out and is closed");
  const at = iso(deps.now());
  if (!await deps.store.markSent(batch.addressId, batch.id, b.sent, at)) {
    throw fail(409, "the batch changed meanwhile; try again");
  }
  return answer(b.sent, at);
}

// ---------------------------------------------------------------- POST /reconcile

export async function reconcile(req: Request, accountId: string, deps: MailDeps): Promise<Response> {
  const b = await body(req, ["address_id", "since", "until"]);
  const since = instantMs(b.since);
  const until = instantMs(b.until);
  if (since === null || until === null || since >= until) {
    throw fail(400, "since and until must be a window, the earlier first");
  }
  if (until - since > MAX_WINDOW_DAYS * DAY) throw fail(422, "a reconcile window spans at most 181 days");
  const row = await own(accountId, b.address_id, deps);
  if (since < windowFloor(row, deps.now())) throw fail(422, RECONCILE_BEFORE_WINDOW);
  const box = await deps.store.fpKey(accountId);
  if (box === null) throw fail(409, "this account has no mail key yet");
  const key = await deps.decrypt(box.ciphertext, box.iv);
  // The dedup set is the account's (§4.6 step 7): a message held through another of its mailboxes is held.
  const held = await deps.store.held(accountId, iso(since - MATCH_MS), iso(until + MATCH_MS));
  return json(200, {
    address_id: row.id,
    since: iso(since),
    until: iso(until),
    key,
    held: held.map((h) => ({ fp: h.fp, minute: iso(Date.parse(h.minute)) })),
  });
}

// ---------------------------------------------------------------- POST /addresses/<id>/state

const STATES = ["off", "elsewhere", "reenabled", "confirming"] as const;

export async function setState(
  req: Request,
  accountId: string,
  id: string,
  deps: MailDeps,
): Promise<Response> {
  const b = await body(req, ["state"]);
  const state = b.state;
  if (typeof state !== "string" || !(STATES as readonly string[]).includes(state)) {
    throw fail(400, "state must be 'off', 'elsewhere', 'reenabled' or 'confirming'");
  }
  const row = await own(accountId, id, deps);
  const now = deps.now();
  let patch: StatePatch;
  let extra: Record<string, string> = {};
  if (state === "reenabled") {
    const at = iso(now);
    patch = { reenabledAt: at };
    extra = { reenabled_at: at };
  } else if (state === "confirming") {
    if (row.provider !== "google") throw fail(422, "only a Gmail mailbox confirms its forwarding address");
    // Intake completes Google's confirm only for a requester equal to the hint (§4.6 step 5).
    if (row.mailboxHint === null) throw fail(409, NO_HINT);
    const until = iso(now + CONFIRM_WINDOW_MS);
    patch = { confirmUntil: until };
    extra = { confirm_until: until };
  } else {
    patch = { status: "off" };
  }
  if (!await deps.store.setState(accountId, row.id, patch)) throw fail(404, NOT_FOUND);
  return json(200, { address_id: row.id, state, status: patch.status ?? row.status, ...extra });
}

// ---------------------------------------------------------------- GET /provider?domain=

export async function provider(req: Request, deps: MailDeps): Promise<Response> {
  const domain = (new URL(req.url).searchParams.get("domain") ?? "").trim().toLowerCase();
  if (!DOMAIN.test(domain)) throw fail(400, "domain must be one mail domain");
  try {
    return json(200, { domain, provider: await providerOf(domain, deps.resolveMx) });
  } catch {
    throw fail(503, "the mail domain could not be checked; try again");
  }
}
