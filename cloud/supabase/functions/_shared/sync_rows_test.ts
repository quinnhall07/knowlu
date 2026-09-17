import { assert, assertEquals, assertThrows } from "@std/assert";
import {
  checkNote,
  checkRecord,
  isDeviceToken,
  isFingerprint,
  isHash,
  isIv,
  MAX_NOTE_CIPHERTEXT,
  MAX_RECORD_CIPHERTEXT,
} from "./sync_rows.ts";

const DEVICE = "0123456789abcdef";
const HASH = "a".repeat(64);
const IV = "AAAAAAAAAAAAAAAA";

Deno.test("the four opaque shapes are checked, not trusted", () => {
  assert(isFingerprint("0a1b2c3d"));
  assert(!isFingerprint("0A1B2C3D"), "upper case is a different string and would read as a new generation");
  assert(!isFingerprint("0a1b2c3"));
  assert(isDeviceToken(DEVICE));
  assert(!isDeviceToken("0123456789ABCDEF"), "upper case is a different token and would split a device in two");
  assert(!isDeviceToken("short"));
  assert(!isDeviceToken(7));
  assert(isHash(HASH));
  assert(!isHash("a".repeat(63)));
  assert(isIv(IV));
  assert(!isIv("AAAA"));
});

Deno.test("a good record row comes back as the row the database takes", () => {
  const row = checkRecord({ hash: HASH, iv: IV, ciphertext: "Zm9v" }, DEVICE, "acct-1");
  assertEquals(row, { account_id: "acct-1", device: DEVICE, record_hash: HASH, iv: IV, ciphertext: "Zm9v", keep: false });
  const kept = checkRecord({ hash: HASH, iv: IV, ciphertext: "Zm9v", keep: true }, DEVICE, "acct-1");
  assertEquals(kept.keep, true, "the device's retention bit survives, and nothing else does");
  assertThrows(() => checkRecord({ hash: HASH, iv: IV, ciphertext: "Zm9v", keep: "yes" }, DEVICE, "acct-1"));
});

Deno.test("a record row with anything extra is refused, by name", () => {
  // The one place a client could smuggle a plaintext field in beside the ciphertext. An unknown key
  // is a 400, not something quietly dropped: dropping it would hide a client that thinks it is
  // sending something we are storing.
  const e = assertThrows(() => checkRecord({ hash: HASH, iv: IV, ciphertext: "Zm9v", path: "tasks/a.md" }, DEVICE, "acct-1")) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a record over the ciphertext cap is refused rather than truncated", () => {
  const e = assertThrows(() => checkRecord({ hash: HASH, iv: IV, ciphertext: "A".repeat(24577) }, DEVICE, "acct-1")) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a note tombstone carries no bytes and a live note must carry both", () => {
  assertEquals(
    checkNote({ ref: HASH, deleted: true }, DEVICE, "acct-1"),
    { account_id: "acct-1", note_ref: HASH, device: DEVICE, deleted: true, iv: null, ciphertext: null },
  );
  assertEquals(
    checkNote({ ref: HASH, iv: IV, ciphertext: "Zm9v" }, DEVICE, "acct-1"),
    { account_id: "acct-1", note_ref: HASH, device: DEVICE, deleted: false, iv: IV, ciphertext: "Zm9v" },
  );
  // Half a note is the one shape the table's own check constraint would reject with a 502; catching
  // it here makes it a 400 that says which half is missing.
  const e = assertThrows(() => checkNote({ ref: HASH, iv: IV }, DEVICE, "acct-1")) as Response;
  assertEquals(e.status, 400);
  const f = assertThrows(() => checkNote({ ref: HASH, deleted: true, ciphertext: "Zm9v" }, DEVICE, "acct-1")) as Response;
  assertEquals(f.status, 400);
});

Deno.test("both ciphertext caps match their columns", async () => {
  // The two caps live in two files — this module and `20260912000100_sync.sql` — and nothing else
  // holds them together. 16 KiB and 128 KiB of plaintext, base64, with room.
  assertEquals(MAX_RECORD_CIPHERTEXT, 24576);
  assertEquals(MAX_NOTE_CIPHERTEXT, 196608);
  const sql = await Deno.readTextFile(new URL("../../migrations/20260912000100_sync.sql", import.meta.url));
  assert(sql.includes(`between 1 and ${MAX_RECORD_CIPHERTEXT}`), "sync_records.ciphertext");
  assert(sql.includes(`between 1 and ${MAX_NOTE_CIPHERTEXT}`), "sync_notes.ciphertext");
});
