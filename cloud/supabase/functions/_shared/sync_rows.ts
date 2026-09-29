/**
 * What a pushed row may be. **Every field is bounded and shaped** — that is the whole module.
 *
 * The device has already built these; refusing again here is not distrust of the device, it is the
 * only place the rule holds for a client somebody else wrote. Two refusals this file makes that the
 * device does not: **an unknown key is a 400**, because a field beside the body is the one way a
 * patched client could turn this store into something it is not; and **a path is checked**, because
 * the path is the note's primary key now and a restore is what would write it.
 */
import { fail } from "./http.ts";

/** The server's own batch cap, and the device's page size. Matches C1's `/telemetry`. */
export const MAX_ROWS = 500;
/** `sync_records.body`'s `octet_length` check. BYTES, not characters — see `bytes()`. */
export const MAX_RECORD_BYTES = 16384;
/** `sync_notes.body`'s `octet_length` check. */
export const MAX_NOTE_BYTES = 131072;
/** The transport cap on one whole pushed body — `readJson`'s own limit, not the account's ceiling
 * (R-C3′-exec-7): 500 rows at either row cap is well past 1 MiB, so a legitimate full batch must fit
 * under this before the ceiling is ever checked. Over it is `_shared/http.ts`'s own 413; the ceiling
 * below is a 403, a different reason the device can tell apart. */
export const MAX_PUSH_BYTES = 4194304;

const DEVICE_RE = /^[0-9a-f]{16}$/;
const HASH_RE = /^[0-9a-f]{64}$/;
/** The same rule `engine/src/sync.rs::is_note_path` enforces and the same one the column checks:
 * one of `ids::NOTE_FOLDERS`, markdown, no `..` segment and no empty segment. The group is written
 * in `NOTE_FOLDERS` order as it reads once both the grades and the commitment-model branches have
 * merged (grades spec §7): `commitments` is accepted before the engine that writes it ships, so a
 * push is never refused whichever branch merges first. */
export const NOTE_PATH_RE = /^(tasks|approvals|archive|courses|issues|info|commitments|grades)\/[A-Za-z0-9._ /-]{1,300}\.md$/;

export function isDeviceToken(x: unknown): boolean {
  return typeof x === "string" && DEVICE_RE.test(x);
}
export function isHash(x: unknown): boolean {
  return typeof x === "string" && HASH_RE.test(x);
}
export function isNotePath(x: unknown): boolean {
  return typeof x === "string" && NOTE_PATH_RE.test(x) && !/(^|\/)\.\.(\/|$)/.test(x) && !x.includes("//");
}
/** Postgres's `octet_length` on the device's side of the wire. `String.length` is UTF-16 units and
 * would let a vault of accented Spanish past a cap the column then refuses. */
export function bytes(s: string): number {
  return new TextEncoder().encode(s).length;
}

/** The content hash, **defined once** (review M7): the handler computes it to check a pushed row and
 * the handler's own suite computes it to build one, and two copies of a hash function are two
 * chances for a test to agree with a bug. `crypto.subtle` is the platform's, so this imports nothing. */
export async function sha256Hex(s: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export interface RecordIn { hash: string; body: string }
export interface NoteIn { path: string; deleted?: boolean; body?: string }

function noExtras(row: Record<string, unknown>, allowed: string[], what: string): void {
  for (const key of Object.keys(row)) {
    if (!allowed.includes(key)) throw fail(400, `a ${what} carries an unknown field ${JSON.stringify(key)}`);
  }
}

/** One pushed journal record → the row `sync_records` takes. **Throws a `Response`** on anything else. */
export function checkRecord(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a record is not an object");
  const row = raw as Record<string, unknown>;
  // `keep` is NOT allowed: it is a generated column decided from the record itself.
  noExtras(row, ["hash", "body"], "record");
  if (!isHash(row.hash)) throw fail(400, "a record has no usable hash");
  if (typeof row.body !== "string" || row.body.length === 0) throw fail(400, "a record has no body");
  // A NUL byte is valid JSON text but `body::jsonb` (the `keep` generated column) rejects it with
  // 22P05, and the migration's own comment names the validator as the only guard before that 5xx —
  // named here, before it ever reaches the database.
  if (row.body.includes("\u0000")) throw fail(400, "a record's body contains a null byte");
  if (bytes(row.body) > MAX_RECORD_BYTES) {
    throw fail(400, `a record's body is over ${MAX_RECORD_BYTES} bytes`);
  }
  // Parsed here so the table's generated `keep` column never meets something it cannot cast, and so
  // a client that sends a blob of text instead of a record is told which of the two it did.
  let parsed: unknown;
  try {
    parsed = JSON.parse(row.body);
  } catch {
    throw fail(400, "a record's body is not JSON");
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw fail(400, "a record's body is not a journal record");
  }
  const rec = parsed as Record<string, unknown>;
  for (const field of ["op", "actor"]) {
    if (typeof rec[field] !== "string" || (rec[field] as string).length === 0) {
      throw fail(400, `a record's body has no ${field}`);
    }
  }
  return { account_id: accountId, device, record_hash: row.hash as string, body: row.body as string };
}

/** One pushed note → the row `sync_notes` takes. A tombstone carries no bytes; a live note carries them. */
export function checkNote(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a note is not an object");
  const row = raw as Record<string, unknown>;
  noExtras(row, ["path", "deleted", "body"], "note");
  if (!isNotePath(row.path)) throw fail(400, "a note has no usable path");
  if (row.deleted !== undefined && typeof row.deleted !== "boolean") {
    throw fail(400, "deleted must be a boolean");
  }
  const deleted = row.deleted === true;
  if (deleted) {
    if (row.body !== undefined) throw fail(400, "a deleted note carries no bytes");
    return { account_id: accountId, path: row.path as string, device, deleted: true, body: null };
  }
  if (typeof row.body !== "string" || row.body.length === 0) throw fail(400, "a note has no body");
  // Same reason as `checkRecord`'s: the `text` column takes it, but a restore that reads it back
  // through anything JSON-shaped would not, and a named 400 beats a 502 the student cannot act on.
  if (row.body.includes("\u0000")) throw fail(400, "a note's body contains a null byte");
  if (bytes(row.body) > MAX_NOTE_BYTES) throw fail(400, `a note's body is over ${MAX_NOTE_BYTES} bytes`);
  return { account_id: accountId, path: row.path as string, device, deleted: false, body: row.body as string };
}
