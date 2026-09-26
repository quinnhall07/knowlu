/**
 * The only database access C3′ adds. Five calls over C1's `Rest`, every one scoped by
 * `account_id=eq.<id>` — the service role bypasses RLS, so that predicate IS the access control,
 * and `sync_rows_test.ts`'s scan (Task 12) refuses a query in this file without it.
 */
import { Rest, restSelect, restUpsert } from "./db.ts";

/**
 * How far behind `now` a read stops. **Two desktops cannot strand a row between them.**
 * `seq` is taken at INSERT and becomes visible at COMMIT, so a row can become visible with a `seq`
 * lower than one a previous pull already returned; a cursor that stepped past it would never fetch
 * it again. Ten seconds is far longer than a batch insert takes and far shorter than a slot.
 */
export const READ_LAG_SECONDS = 10;

export async function saveRecords(rest: Rest, rows: unknown[]): Promise<void> {
  if (rows.length === 0) return;
  // `sync_records_once` is `(account_id, record_hash)`: a batch re-sent after a dropped connection
  // lands on the rows it landed on the first time.
  await restUpsert(rest, "sync_records", rows, "account_id,record_hash");
}

export async function saveNotes(rest: Rest, rows: unknown[]): Promise<void> {
  if (rows.length === 0) return;
  await restUpsert(rest, "sync_notes", rows, "account_id,path");
}

export async function bytesUsed(rest: Rest, accountId: string): Promise<number> {
  const rows = await restSelect<{ bytes: number }>(rest, "sync_usage", `select=bytes&account_id=eq.${accountId}`);
  return rows[0]?.bytes ?? 0;
}

export async function ceiling(rest: Rest): Promise<number> {
  const rows = await restSelect<{ ceiling: number }>(rest, "sync_limits", "select=ceiling&limit=1");
  return Number(rows[0]?.ceiling ?? 0);
}

export async function readRecords(
  rest: Rest, accountId: string, after: number, limit: number, now: Date,
): Promise<unknown[]> {
  const cut = new Date(now.getTime() - READ_LAG_SECONDS * 1000).toISOString();
  return await restSelect(
    rest,
    "sync_records",
    `select=seq,device,record_hash,body&account_id=eq.${accountId}&seq=gt.${after}` +
      `&received_at=lt.${cut}&order=seq.asc&limit=${limit}`,
  );
}

export async function readNotes(
  rest: Rest, accountId: string, after: number, limit: number, now: Date,
): Promise<unknown[]> {
  const cut = new Date(now.getTime() - READ_LAG_SECONDS * 1000).toISOString();
  return await restSelect(
    rest,
    "sync_notes",
    `select=rev,device,path,deleted,body&account_id=eq.${accountId}&rev=gt.${after}` +
      `&updated_at=lt.${cut}&order=rev.asc&limit=${limit}`,
  );
}
