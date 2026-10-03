/**
 * The real store's PostgREST query for `CanaryStore.candidates` (email plan T4.3), kept apart from
 * `index.ts` (which serves on import) so its filter is tested beside the handler's.
 */
import { SENDABLE } from "./handler.ts";

/** A timestamp inside a PostgREST logic tree is quoted (it holds `.` and `:`), then the tree is encoded. */
const tree = (s: string) => encodeURIComponent(s);
/** The most mailboxes one run reads as candidates, the longest-unchecked first. */
const CANDIDATES = 500;
export const SENDABLE_IN = `in.(${SENDABLE.join(",")})`;

/** `active` and `quiet` mailboxes with a hint whose last canary is older than `canaryBefore` (or absent) and
 * whose last bound mail is older than `silentBefore` (or absent); a `quiet` one passes the silence clause
 * whatever its bound mail, as the handler's filter lets it. */
export function candidatesQuery(canaryBefore: string, silentBefore: string): string {
  const due = tree(
    `(or(last_canary_sent_at.is.null,last_canary_sent_at.lt."${canaryBefore}"),` +
      `or(status.eq.quiet,last_received_at.is.null,last_received_at.lt."${silentBefore}"))`,
  );
  return `status=${SENDABLE_IN}&mailbox_hint=not.is.null&and=${due}` +
    "&select=id,account_id,status,mailbox_hint,last_received_at,last_canary_sent_at,canaries_today" +
    `&order=last_canary_sent_at.asc.nullsfirst,id.asc&limit=${CANDIDATES}`;
}
