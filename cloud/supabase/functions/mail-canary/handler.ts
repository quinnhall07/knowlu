/**
 * The periodic canary (email spec §4.10, D14, §13 Q2 (a); plan T4.3, P9): one cron call an hour,
 * authenticated by `x-knowlu-job-token` (`MAIL_JOBS_TOKEN`, the Vault token the cron sends). Each run:
 *
 * 1. **Judges the canaries that did not come back.** A canary to a mailbox that was never `active` (no
 *    canary has ever arrived, `last_canary_ok_at` is null) is a setup canary: once the setup's three minutes
 *    are over it sets a `pending` or `confirmed` mailbox `blocked` (D16). Any other is periodic: after two
 *    hours it sets an `active` mailbox `quiet`. A canary that a later one overtook (the mailbox's
 *    `last_canary_ok_at` is at or after its `sent_at`) judges nothing, and each write is a compare-and-set on
 *    the status, so a mailbox the device set `off`, or one already judged, is left alone. Intake turns a
 *    judged mailbox `active` again when a canary arrives.
 * 2. **Sends a canary to each mailbox that has gone quiet**, through `_shared/canary_send.ts` (P9), to its
 *    `mailbox_hint` (checked against its provider when the hint was set, T4.1). A mailbox is sent one only
 *    when all of these hold:
 *    - it is `active` or `quiet` (a `quiet` one's arrival is what turns it `active` again), has its hint, and
 *      its account is entitled;
 *    - no bound mail for 24 hours, or 48 when the silence touches a Saturday or Sunday (UTC; the service
 *      knows no student's zone), read from `last_received_at`. A `quiet` mailbox is exempt: bound mail does
 *      not clear `quiet`, only a canary's arrival does, so it is sent one every 72 hours until one comes
 *      back, whatever its bound mail (else a lost canary leaves it `quiet` for good while mail flows);
 *    - at least 72 hours since that mailbox's last canary (`last_canary_sent_at`, setup's included);
 *    - the student has had no Knowlu email this UTC day: no canary to any of the account's mailboxes and
 *      no annual reminder (`billing_reminders.last_reminded_at`), and none sent earlier in this run (P9).
 *    The turn is taken first, as `POST /canary` takes it (`canaries_today` on `last_canary_sent_at`'s
 *    UTC day, a compare-and-set); a refused send takes its canary row back and keeps the turn spent.
 *    The longest-unchecked mailboxes go first, at most `MAX_SENDS` a run.
 *
 * **Nothing here logs an address, a hint or a nonce**: the answer and the one log line are counts.
 */
import type { SendCanary } from "../_shared/canary_send.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { newNonce } from "../_shared/mail_bind.ts";
import { equalHex } from "../_shared/stripe.ts";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
/** D14: the setup waits up to three minutes for its canary. */
export const SETUP_WAIT_MS = 3 * MINUTE;
/** §4.10: a periodic canary not back within two hours sets the mailbox `quiet`. */
export const PERIODIC_WAIT_MS = 2 * HOUR;
/** §13 Q2 (a): bound-mail silence on a weekday, and over a weekend. */
export const WEEKDAY_SILENCE_MS = 24 * HOUR;
export const WEEKEND_SILENCE_MS = 48 * HOUR;
/** §13 Q2 (a): a mailbox's canaries are at least this far apart. */
export const CANARY_GAP_MS = 72 * HOUR;
/** The most canaries one run sends; the rest wait an hour. */
export const MAX_SENDS = 100;
/** Statuses a periodic canary goes to. */
export const SENDABLE: readonly string[] = ["active", "quiet"];
/** What a missing setup canary turns `blocked`, and what a missing periodic one turns `quiet`. */
export const NEVER_ACTIVE: readonly string[] = ["pending", "confirmed"];
export const WATCHED: readonly string[] = ["active"];

export interface JobConfig {
  /** `MAIL_JOBS_TOKEN`: the cron's `x-knowlu-job-token`. */
  jobsToken: string;
}

/** One mailbox the run may send a canary to. */
export interface CanaryMailbox {
  id: string;
  accountId: string;
  status: string;
  mailboxHint: string | null;
  lastReceivedAt: string | null;
  lastCanarySentAt: string | null;
  canariesToday: number;
}

/** An unarrived canary with its mailbox's status and `last_canary_ok_at`. */
export interface Outstanding {
  canaryId: string;
  addressId: string;
  sentAt: string;
  status: string;
  lastCanaryOkAt: string | null;
}

export interface CanaryStore {
  /** Every unarrived canary sent before `before` (and inside the 30 days canaries are kept), oldest first. */
  outstanding(before: string): Promise<Outstanding[]>;
  /** Sets the mailbox `to` only while its status is one of `from` and no canary has arrived at or after
   * `sentAt` (`last_canary_ok_at` null or earlier); true when it wrote. */
  judge(
    addressId: string,
    from: readonly string[],
    to: "quiet" | "blocked",
    sentAt: string,
  ): Promise<boolean>;
  /** The mailboxes that may be due: a superset of what the run sends to, which re-checks every rule. The
   * real store answers `active` and `quiet` mailboxes with a hint whose last canary is older than
   * `canaryBefore` and whose last bound mail is older than `silentBefore` (or absent, or the mailbox is
   * `quiet`); its query is `query.ts`'s `candidatesQuery`. */
  candidates(canaryBefore: string, silentBefore: string): Promise<CanaryMailbox[]>;
  /** Knowlu's mail to the account: each of its mailboxes' last canary, and its last annual reminder. */
  accountMail(accountId: string): Promise<{ canariesSentAt: (string | null)[]; remindedAt: string | null }>;
  entitled(accountId: string): Promise<boolean>;
  /** Writes `next` to `canaries_today` and `last_canary_sent_at` only while they still hold `seen` and the
   * mailbox is still `active` or `quiet`. */
  takeCanary(
    addressId: string,
    seen: { count: number; lastSentAt: string | null },
    next: { count: number; lastSentAt: string },
  ): Promise<boolean>;
  insertCanary(addressId: string, nonceHmac: string, sentAt: string): Promise<string>;
  deleteCanary(addressId: string, id: string): Promise<void>;
}

export interface CanaryDeps {
  store: CanaryStore;
  sendCanary: SendCanary;
  /** `keys.ts`'s `nonceHmac` under `MAIL_ADDRESS_KEY`, as `mail` keeps a canary's nonce. */
  nonceHmac(nonce: string): Promise<string>;
  now(): number;
  newNonce?(): string;
}

export interface RunCounts {
  blocked: number;
  quiet: number;
  sent: number;
  failed: number;
}

const log = (phrase: string) => console.log(`mail-canary: ${phrase}`);
const iso = (ms: number) => new Date(ms).toISOString();
const utcDay = (ms: number) => Math.floor(ms / DAY) * DAY;
const at = (s: string | null) => (s === null ? NaN : Date.parse(s));

/** True when the silence `(from, to]` touches a Saturday or a Sunday (UTC). */
function touchesWeekend(from: number, to: number): boolean {
  for (let day = utcDay(from); day <= to; day += DAY) {
    const wd = new Date(day).getUTCDay();
    if (wd === 0 || wd === 6) return true;
  }
  return false;
}

/** §13 Q2 (a): no bound mail for 24 hours on a weekday, 48 over a weekend. Never any is silent. */
export function quietLongEnough(lastReceivedAt: string | null, now: number): boolean {
  const last = at(lastReceivedAt);
  if (!Number.isFinite(last)) return true;
  const silent = now - last;
  if (silent >= WEEKEND_SILENCE_MS) return true;
  return silent >= WEEKDAY_SILENCE_MS && !touchesWeekend(last, now);
}

/** Step 1: each unarrived canary past its wait sets its mailbox `blocked` or `quiet`. */
async function judge(deps: CanaryDeps, now: number, counts: RunCounts): Promise<void> {
  for (const c of await deps.store.outstanding(iso(now - SETUP_WAIT_MS))) {
    const sent = at(c.sentAt);
    const ok = at(c.lastCanaryOkAt);
    if (Number.isFinite(ok) && ok >= sent) continue;
    if (c.lastCanaryOkAt === null) {
      if (
        NEVER_ACTIVE.includes(c.status) &&
        await deps.store.judge(c.addressId, NEVER_ACTIVE, "blocked", c.sentAt)
      ) {
        counts.blocked++;
      }
    } else if (now - sent >= PERIODIC_WAIT_MS && WATCHED.includes(c.status)) {
      if (await deps.store.judge(c.addressId, WATCHED, "quiet", c.sentAt)) counts.quiet++;
    }
  }
}

/** P9: the account had a Knowlu email on `now`'s UTC day. */
async function emailedToday(deps: CanaryDeps, accountId: string, now: number): Promise<boolean> {
  const { canariesSentAt, remindedAt } = await deps.store.accountMail(accountId);
  const today = (s: string | null) => Number.isFinite(at(s)) && utcDay(at(s)) === utcDay(now);
  return canariesSentAt.some(today) || today(remindedAt);
}

/** Sends one canary to `box`; false when the turn was taken by someone else or the send was refused. */
async function sendOne(deps: CanaryDeps, box: CanaryMailbox, hint: string, now: number, counts: RunCounts) {
  const last = at(box.lastCanarySentAt);
  const today = Number.isFinite(last) && utcDay(last) === utcDay(now) ? box.canariesToday : 0;
  const seen = { count: box.canariesToday, lastSentAt: box.lastCanarySentAt };
  if (!await deps.store.takeCanary(box.id, seen, { count: today + 1, lastSentAt: iso(now) })) return false;
  const nonce = (deps.newNonce ?? newNonce)();
  const id = await deps.store.insertCanary(box.id, await deps.nonceHmac(nonce), iso(now));
  try {
    await deps.sendCanary(hint, nonce);
  } catch {
    // Never sent: the row goes so step 1 never waits on it; the turn stays spent, as `POST /canary`'s does.
    await deps.store.deleteCanary(box.id, id).catch(() => {});
    counts.failed++;
    return false;
  }
  counts.sent++;
  return true;
}

/** Step 2: one canary to each mailbox gone quiet, under §13 Q2's spacing and P9's one email a day. */
async function send(deps: CanaryDeps, now: number, counts: RunCounts): Promise<void> {
  const due = (await deps.store.candidates(iso(now - CANARY_GAP_MS), iso(now - WEEKDAY_SILENCE_MS)))
    .filter((b) =>
      SENDABLE.includes(b.status) && b.mailboxHint !== null &&
      !(at(b.lastCanarySentAt) > now - CANARY_GAP_MS) &&
      (b.status === "quiet" || quietLongEnough(b.lastReceivedAt, now))
    )
    .sort((a, b) =>
      (at(a.lastCanarySentAt) || 0) - (at(b.lastCanarySentAt) || 0) || a.id.localeCompare(b.id)
    );
  const emailed = new Set<string>();
  const entitled = new Map<string, boolean>();
  for (const box of due) {
    if (counts.sent + counts.failed >= MAX_SENDS) break;
    if (emailed.has(box.accountId)) continue;
    let ok = entitled.get(box.accountId);
    if (ok === undefined) entitled.set(box.accountId, ok = await deps.store.entitled(box.accountId));
    if (!ok) continue;
    if (await emailedToday(deps, box.accountId, now)) {
      emailed.add(box.accountId);
      continue;
    }
    if (await sendOne(deps, box, box.mailboxHint as string, now, counts)) emailed.add(box.accountId);
  }
}

/** One run: judge, then send. */
export async function run(deps: CanaryDeps): Promise<RunCounts> {
  const now = deps.now();
  const counts: RunCounts = { blocked: 0, quiet: 0, sent: 0, failed: 0 };
  await judge(deps, now, counts);
  await send(deps, now, counts);
  return counts;
}

/** One cron call. 401 without the job token; the answer is counts only. */
export async function handle(req: Request, cfg: JobConfig, deps: CanaryDeps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const token = req.headers.get("x-knowlu-job-token") ?? "";
  if (cfg.jobsToken === "" || !equalHex(token, cfg.jobsToken)) return fail(401, "not a job caller");
  await req.body?.cancel();
  try {
    const counts = await run(deps);
    log(`${counts.blocked} blocked, ${counts.quiet} quiet, ${counts.sent} sent, ${counts.failed} failed`);
    return json(200, { ok: true, ...counts });
  } catch {
    log("run failed (store)");
    return fail(500, "the run failed");
  }
}
