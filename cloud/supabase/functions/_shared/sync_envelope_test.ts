import { assert, assertEquals } from "@std/assert";
import { decryptString, encryptString, importAesKey } from "./crypto.ts";

const VECTORS = JSON.parse(await Deno.readTextFile(new URL("./sync_vectors.json", import.meta.url))) as {
  key: string;
  plaintext: string;
  rust: { iv: string; ciphertext: string };
  deno: { iv: string; ciphertext: string };
};

Deno.test("Deno opens the envelope Rust sealed", async () => {
  // If this ever fails, a restore fails — and it fails silently, months later, on the one machine
  // that needed it. The device's `seal` appends GCM's tag to the ciphertext, which is what
  // WebCrypto does too; this is the executable form of that sentence.
  const key = await importAesKey(VECTORS.key);
  assertEquals(await decryptString(key, VECTORS.rust.ciphertext, VECTORS.rust.iv), VECTORS.plaintext);
});

Deno.test("Deno opens its own, and the two envelopes are not the same bytes", async () => {
  const key = await importAesKey(VECTORS.key);
  assertEquals(await decryptString(key, VECTORS.deno.ciphertext, VECTORS.deno.iv), VECTORS.plaintext);
  assert(VECTORS.rust.iv !== VECTORS.deno.iv, "a fresh nonce per envelope");
});

Deno.test("a round trip through this project's own crypto survives the awkward characters", async () => {
  const key = await importAesKey(VECTORS.key);
  const box = await encryptString(key, VECTORS.plaintext);
  assertEquals(await decryptString(key, box.ciphertext, box.iv), VECTORS.plaintext);
  assertEquals(box.iv.length, 16);
});
