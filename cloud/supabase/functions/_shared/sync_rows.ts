/**
 * What a pushed row may be. **Every field is opaque or bounded** — that is the whole module.
 *
 * The device has already built these; refusing again here is not distrust of the device, it is the
 * only place the rule holds for a client somebody else wrote. And there is one refusal this file
 * makes that the device does not: **an unknown key is a 400**, because the single way a patched
 * client could turn this store into a plaintext store is by sending a field beside the ciphertext
 * and hoping we write it.
 */
import { fail } from "./http.ts";

/** The server's own batch cap, and the device's page size. Matches C1's `/telemetry`. */
export const MAX_ROWS = 500;
/** 16 KiB of plaintext, base64, with room: the `sync_records.ciphertext` check constraint. */
export const MAX_RECORD_CIPHERTEXT = 24576;
/** 128 KiB of plaintext, base64, with room: the `sync_notes.ciphertext` check constraint. */
export const MAX_NOTE_CIPHERTEXT = 196608;

const DEVICE_RE = /^[0-9a-f]{16}$/;
const HASH_RE = /^[0-9a-f]{64}$/;
/** Eight hex characters of a SHA-256 of the key. Not the key, not reversible, not a secret. */
const FINGERPRINT_RE = /^[0-9a-f]{8}$/;
/** Twelve bytes of nonce is sixteen base64 characters, and base64's own alphabet. */
const IV_RE = /^[A-Za-z0-9+/]{16}$/;
const B64_RE = /^[A-Za-z0-9+/]+={0,2}$/;

export function isDeviceToken(x: unknown): boolean {
  return typeof x === "string" && DEVICE_RE.test(x);
}
export function isHash(x: unknown): boolean {
  return typeof x === "string" && HASH_RE.test(x);
}
export function isFingerprint(x: unknown): boolean {
  return typeof x === "string" && FINGERPRINT_RE.test(x);
}
export function isIv(x: unknown): boolean {
  return typeof x === "string" && IV_RE.test(x);
}
function isCiphertext(x: unknown, cap: number): boolean {
  return typeof x === "string" && x.length > 0 && x.length <= cap && B64_RE.test(x);
}

export interface RecordIn {
  hash: string;
  iv: string;
  ciphertext: string;
  /** The one cleartext bit about a record's content: the device says a human wrote it, and
   * `sync_prune` never deletes one. Absent means false. */
  keep?: boolean;
}

export interface NoteIn {
  ref: string;
  deleted?: boolean;
  iv?: string;
  ciphertext?: string;
}

function noExtras(row: Record<string, unknown>, allowed: string[], what: string): void {
  for (const key of Object.keys(row)) {
    if (!allowed.includes(key)) throw fail(400, `a ${what} carries an unknown field ${JSON.stringify(key)}`);
  }
}

/** One pushed journal record → the row `sync_records` takes. **Throws a `Response`** on anything else. */
export function checkRecord(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a record is not an object");
  const row = raw as Record<string, unknown>;
  noExtras(row, ["hash", "iv", "ciphertext", "keep"], "record");
  if (!isHash(row.hash)) throw fail(400, "a record has no usable hash");
  if (!isIv(row.iv)) throw fail(400, "a record has no usable iv");
  if (!isCiphertext(row.ciphertext, MAX_RECORD_CIPHERTEXT)) {
    throw fail(400, `a record's ciphertext is empty, not base64, or over ${MAX_RECORD_CIPHERTEXT} characters`);
  }
  if (row.keep !== undefined && typeof row.keep !== "boolean") throw fail(400, "keep must be a boolean");
  return {
    account_id: accountId,
    device,
    record_hash: row.hash as string,
    iv: row.iv as string,
    ciphertext: row.ciphertext as string,
    keep: row.keep === true,
  };
}

/** One pushed note → the row `sync_notes` takes. A tombstone carries no bytes; a live note carries both. */
export function checkNote(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a note is not an object");
  const row = raw as Record<string, unknown>;
  noExtras(row, ["ref", "deleted", "iv", "ciphertext"], "note");
  if (!isHash(row.ref)) throw fail(400, "a note has no usable ref");
  const deleted = row.deleted === true;
  if (deleted) {
    if (row.iv !== undefined || row.ciphertext !== undefined) {
      throw fail(400, "a deleted note carries no bytes");
    }
    return { account_id: accountId, note_ref: row.ref as string, device, deleted: true, iv: null, ciphertext: null };
  }
  if (!isIv(row.iv)) throw fail(400, "a note has no usable iv");
  if (!isCiphertext(row.ciphertext, MAX_NOTE_CIPHERTEXT)) {
    throw fail(400, `a note's ciphertext is empty, not base64, or over ${MAX_NOTE_CIPHERTEXT} characters`);
  }
  return {
    account_id: accountId,
    note_ref: row.ref as string,
    device,
    deleted: false,
    iv: row.iv as string,
    ciphertext: row.ciphertext as string,
  };
}
