import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { checkNote, checkRecord, isDeviceToken, MAX_ROWS, sha256Hex } from "../_shared/sync_rows.ts";

export interface PushDeps {
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  bytesUsed: (accountId: string) => Promise<number>;
  ceiling: () => Promise<number>;
  saveRecords: (rows: unknown[]) => Promise<void>;
  saveNotes: (rows: unknown[]) => Promise<void>;
}

interface Body { device?: unknown; records?: unknown; notes?: unknown }

function listOf(x: unknown, what: string): unknown[] {
  if (x === undefined) return [];
  if (!Array.isArray(x)) throw fail(400, `${what} must be a list`);
  if (x.length > MAX_ROWS) throw fail(400, `${what}: at most ${MAX_ROWS} a call`);
  return x;
}

export async function handle(req: Request, deps: PushDeps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const { account_id } = await deps.requireEntitled(req);
  const body = await readJson<Body>(req);
  if (!isDeviceToken(body.device)) throw fail(400, "a push carries an opaque device token");
  const device = body.device as string;

  // **Validate, hash-check and de-duplicate before anything is written.** A batch is all or nothing:
  // a student whose last row is malformed must not end up with a half-stored push and a cursor that
  // has moved past the rest of it.
  const records = new Map<string, Record<string, unknown>>();
  for (const raw of listOf(body.records, "records")) {
    const row = checkRecord(raw, device, account_id);
    const actual = await sha256Hex(row.body as string);
    if (actual !== row.record_hash) {
      throw fail(400, "a record's hash is not the sha256 of its body");
    }
    records.set(actual, row);
  }
  const notes = new Map<string, Record<string, unknown>>();
  for (const raw of listOf(body.notes, "notes")) {
    const row = checkNote(raw, device, account_id);
    notes.set(row.path as string, row);
  }

  const [used, cap] = await Promise.all([deps.bytesUsed(account_id), deps.ceiling()]);
  const adding = [...records.values(), ...notes.values()]
    .reduce((n, r) => n + (typeof r.body === "string" ? new TextEncoder().encode(r.body).length : 0), 0);
  if (cap > 0 && used + adding > cap) {
    // Not a failure of the device and not something a retry fixes: the device turns this into one
    // named line and one Good-to-know item, and the records stay in the journal.
    throw fail(413, "this account's copy is at its size limit");
  }

  await deps.saveRecords([...records.values()]);
  await deps.saveNotes([...notes.values()]);
  return json(200, { records: records.size, notes: notes.size, bytes_used: used, bytes_ceiling: cap });
}
