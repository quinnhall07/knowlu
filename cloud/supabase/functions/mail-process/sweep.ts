/**
 * The nightly sweep (email plan P7; spec §5.1's `mail-sweep` cron, D8): a mode of `mail-process`, so its
 * rules are tested in Deno against a fake store. In order:
 *
 * 1. **Expire work** whose object's lifecycle has run. The bucket expires `work/` one day after a put,
 *    rounded up to the next UTC midnight (S3's rule), so an item that arrived at or before the start of
 *    yesterday (UTC) has no object left to read. Each is expired as the worker expires one
 *    (`expire_mail_work`: the work row, its `mail_seen` row and one `expired` count), and its object is
 *    deleted if AWS has not yet removed it. A later recover of the message is then accepted.
 * 2. **Close timed-out batches** (§4.8): 2 hours after the device closed the send, or 14 days after the
 *    batch opened. A batch whose received count reached the sent count is the pull's to close.
 * 3. **Prune by age** (§5.1's "Kept" column): `mail_seen` at 180 days; delivered claims 7 days after
 *    delivery and undelivered ones 30 days after queueing (counted); closed batches 30 days after closing;
 *    canaries 30 days after sending.
 *
 * It judges nothing and reads no object. **Privacy:** the log lines are fixed phrases with counts.
 */
import type { ProcessDeps } from "./handler.ts";

const DAY = 86_400_000;
const HOUR = 3_600_000;
/** Work rows read per round, and the most the sweep expires in one call. */
const ROUND = 200;
const MAX_EXPIRED = 5_000;

export interface SweepCounts {
  expired: number;
  closed: number;
  seen: number;
  delivered: number;
  undelivered: number;
  batches: number;
  canaries: number;
}

const iso = (ms: number) => new Date(ms).toISOString();
const log = (phrase: string) => console.log(`mail-process: sweep ${phrase}`);

/** The start of the UTC day holding `ms`. */
export function utcDayStart(ms: number): number {
  const d = new Date(ms);
  return Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate());
}

/** The latest arrival whose `work/` object has expired by `now`: put at `t`, an object expires at the UTC
 * midnight at or after `t + 1 day`, which has passed exactly when `t` is at or before yesterday's start. */
export function expiredBy(now: number): number {
  return utcDayStart(now) - DAY;
}

export async function sweep(deps: Pick<ProcessDeps, "store" | "s3" | "now">): Promise<SweepCounts> {
  const now = deps.now();
  const counts: SweepCounts = {
    expired: 0,
    closed: 0,
    seen: 0,
    delivered: 0,
    undelivered: 0,
    batches: 0,
    canaries: 0,
  };
  let looked = 0;
  while (looked < MAX_EXPIRED) {
    const due = await deps.store.expirable(iso(expiredBy(now)), ROUND);
    if (due.length === 0) break;
    let took = 0;
    for (const item of due) {
      looked++;
      if (await deps.store.expire(item.id)) {
        counts.expired++;
        took++;
      }
      await deps.s3.delete(item.s3Key).catch(() => log("object delete failed, left to the bucket's expiry"));
    }
    // A round whose every row another caller expired first ends the loop rather than spin.
    if (took === 0 || due.length < ROUND) break;
  }
  counts.closed = await deps.store.closeBatches(iso(now), iso(now - 2 * HOUR), iso(now - 14 * DAY));
  counts.seen = await deps.store.pruneSeen(iso(now - 180 * DAY));
  const claims = await deps.store.pruneClaims(iso(now - 7 * DAY), iso(now - 30 * DAY));
  counts.delivered = claims.delivered;
  counts.undelivered = claims.undelivered;
  counts.batches = await deps.store.pruneBatches(iso(now - 30 * DAY));
  counts.canaries = await deps.store.pruneCanaries(iso(now - 30 * DAY));
  log(
    `done: ${counts.expired} expired, ${counts.closed} batches closed, ${counts.seen} seen pruned, ` +
      `${counts.delivered} delivered and ${counts.undelivered} undelivered claims pruned, ` +
      `${counts.batches} batches and ${counts.canaries} canaries pruned`,
  );
  return counts;
}
