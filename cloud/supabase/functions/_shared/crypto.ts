/**
 * AES-256-GCM over one short string. The only thing this system stores server-side that is a secret
 * of the user's rather than of ours is the LMS capability URL — anyone holding it can read that
 * student's whole schedule — so it is encrypted before it reaches Postgres and the key lives in the
 * function's environment (`SOURCES_ENC_KEY`), not in the database.
 *
 * GCM, not CBC: the tag is what makes a tampered row fail to decrypt instead of decrypting to
 * something else.
 */
function b64ToBytes(b64: string): Uint8Array<ArrayBuffer> {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

function bytesToB64(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
}

/** The one hex encoder (R-C1-59 M3): `_shared/stripe.ts`, `account/index.ts` and
 * `billing-checkout/index.ts` each wrote this out separately before it lived here. */
export function toHex(bytes: ArrayBuffer | Uint8Array): string {
  return Array.from(new Uint8Array(bytes)).map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** Case-folded before hashing: the subject of a consent row and an account's email are matched
 * without regard to case everywhere else in this system, so the hash must be too. */
export async function sha256Hex(s: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s.toLowerCase()));
  return toHex(digest);
}

export async function importAesKey(base64Key: string): Promise<CryptoKey> {
  const raw = b64ToBytes(base64Key);
  if (raw.length !== 32) throw new Error("SOURCES_ENC_KEY must be 32 bytes, base64");
  return await crypto.subtle.importKey("raw", raw, { name: "AES-GCM" }, false, ["encrypt", "decrypt"]);
}

export async function encryptString(
  key: CryptoKey,
  plaintext: string,
): Promise<{ ciphertext: string; iv: string }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const box = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, new TextEncoder().encode(plaintext));
  return { ciphertext: bytesToB64(new Uint8Array(box)), iv: bytesToB64(iv) };
}

export async function decryptString(key: CryptoKey, ciphertext: string, iv: string): Promise<string> {
  const plain = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: b64ToBytes(iv) },
    key,
    b64ToBytes(ciphertext),
  );
  return new TextDecoder().decode(plain);
}
