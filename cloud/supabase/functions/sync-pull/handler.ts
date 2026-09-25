import { json, methodNotAllowed } from "../_shared/http.ts";

/** One page. The same number the device uses, and under PostgREST's own `max_rows` (1000) so a
 * short page always means "that is all", never "the server cut you off". */
export const MAX_PAGE = 500;

export interface PullDeps {
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  readRecords: (accountId: string, after: number, limit: number, now: Date) => Promise<unknown[]>;
  readNotes: (accountId: string, after: number, limit: number, now: Date) => Promise<unknown[]>;
  now: () => Date;
}

function cursor(params: URLSearchParams, key: string): number {
  const n = Number(params.get(key));
  return Number.isSafeInteger(n) && n > 0 ? n : 0;
}

export async function handle(req: Request, deps: PullDeps): Promise<Response> {
  if (req.method !== "GET") return methodNotAllowed(["GET"]);
  const { account_id } = await deps.requireEntitled(req);
  const params = new URL(req.url).searchParams;
  const asked = Number(params.get("limit"));
  const limit = Number.isSafeInteger(asked) && asked > 0 ? Math.min(asked, MAX_PAGE) : MAX_PAGE;
  const recordsAfter = cursor(params, "records_after");
  const notesAfter = cursor(params, "notes_after");
  // **One clock for both reads** (review I3): two `new Date()` calls would put the two windows a
  // millisecond apart, which is exactly the gap the lag exists to close.
  const now = deps.now();
  const [records, notes] = await Promise.all([
    deps.readRecords(account_id, recordsAfter, limit, now),
    deps.readNotes(account_id, notesAfter, limit, now),
  ]);
  const last = <T extends Record<string, unknown>>(rows: unknown[], key: string, fallback: number): number => {
    const row = rows.at(-1) as T | undefined;
    const v = row ? Number(row[key]) : NaN;
    return Number.isSafeInteger(v) ? v : fallback;
  };
  return json(200, {
    records,
    notes,
    record_cursor: last(records, "seq", recordsAfter),
    note_cursor: last(notes, "rev", notesAfter),
    more: records.length >= limit || notes.length >= limit,
  });
}
