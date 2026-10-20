/**
 * When a backfill or recover batch is closed (email spec §4.8), in one place: `mail-pull` reads it to
 * withhold and stamp batches, and `mail`'s `POST /batches` reads it so its ten-open cap counts only the
 * batches that are open at `now`, not the ones nobody has stamped yet.
 *
 * A batch is closed once the received count reaches the count the device reported sent, 2 hours after the
 * device closed the send, or 14 days after it opened. A stamped `closed_at` is closed already. Times are
 * ISO strings, as PostgREST answers them. `mail-process`'s nightly sweep stamps the two timeouts in SQL.
 */

/** §4.8's timeouts: after the device closed the send, and after the batch opened. */
export const SENT_GRACE_MS = 2 * 3_600_000;
export const OPEN_LIMIT_MS = 14 * 86_400_000;

/** What the rules read of a `mail_batches` row. */
export interface BatchClock {
  sent: number | null;
  received: number;
  openedAt: string;
  sentAt: string | null;
  closedAt: string | null;
}

const at = (s: string | null) => s === null ? NaN : Date.parse(s);

/** §4.8's three ways a batch closes, read at `now`; a stamped `closed_at` is closed already. */
export function isClosed(b: BatchClock, now: number): boolean {
  if (b.closedAt !== null) return true;
  if (b.sent !== null && b.received >= b.sent) return true;
  if (b.sentAt !== null && now - at(b.sentAt) >= SENT_GRACE_MS) return true;
  return now - at(b.openedAt) >= OPEN_LIMIT_MS;
}
