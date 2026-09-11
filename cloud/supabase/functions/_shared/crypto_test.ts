import { assert, assertEquals } from "@std/assert";
import { decryptString, encryptString, importAesKey } from "./crypto.ts";

// 32 zero bytes, base64. A fixed key in a test is not a credential of anything.
const KEY_B64 = btoa(String.fromCharCode(...new Uint8Array(32)));

Deno.test("a string round-trips through AES-GCM", async () => {
  const key = await importAesKey(KEY_B64);
  const box = await encryptString(key, "https://lms.example.invalid/feed/abc.ics");
  assertEquals(await decryptString(key, box.ciphertext, box.iv), "https://lms.example.invalid/feed/abc.ics");
});

Deno.test("the same plaintext encrypts differently every time — the IV is random", async () => {
  const key = await importAesKey(KEY_B64);
  const a = await encryptString(key, "same");
  const b = await encryptString(key, "same");
  assert(a.ciphertext !== b.ciphertext);
  assert(a.iv !== b.iv);
});

Deno.test("a tampered ciphertext does not decrypt — GCM authenticates", async () => {
  const key = await importAesKey(KEY_B64);
  const box = await encryptString(key, "https://lms.example.invalid/feed/abc.ics");
  const flipped = box.ciphertext.slice(0, -2) + (box.ciphertext.endsWith("AA") ? "AB" : "AA");
  let threw = false;
  try {
    await decryptString(key, flipped, box.iv);
  } catch {
    threw = true;
  }
  assert(threw, "a tampered ciphertext decrypted");
});

Deno.test("a key that is not 32 bytes is refused, by name", async () => {
  let threw = false;
  try {
    await importAesKey(btoa("short"));
  } catch (e) {
    threw = true;
    assert(String(e).includes("32 bytes"));
  }
  assert(threw);
});
