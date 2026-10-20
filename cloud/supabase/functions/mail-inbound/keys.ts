/**
 * The keyed hashes of email forwarding (spec D3 and §4.6 steps 2, 5 and 7; plan T2g.1). One form each,
 * defined here so every writer and reader computes the same bytes:
 *
 * - **A token's lookup HMAC** (`mail_addresses.token_hmac`, `retired_token_hmac`): HMAC-SHA256 under
 *   `MAIL_ADDRESS_KEY` of `token:<token>`, lower-case hex. `POST /mail/addresses` and Rotate (T4.1) write
 *   it with `tokenHmac`; intake finds a mailbox by it.
 * - **A batch or canary nonce's HMAC** (`mail_batches.nonce_hmac`, `mail_canaries.nonce_hmac`): the same
 *   key over `nonce:<nonce>`. The prefixes keep a token and a nonce from ever hashing alike.
 * - **`mail_seen`'s two hashes**, under the account's own fingerprint key `k` (`mail_accounts`, encrypted):
 *   `msg_hash` = HMAC_k of `["msg", <messageKey>]` and `fp_hash` = HMAC_k of `["fp", <sender>, <subject>]`,
 *   each as JSON. `<messageKey>` is `mime.ts`'s (the Message-ID, else Date, From and Subject); `<sender>`
 *   is the From address, lower-cased; `<subject>` is the decoded Subject with its whitespace collapsed,
 *   lower-cased. Reconcile on the device (§4.11 step 3, T4.4) computes `fp_hash` the same way from the key
 *   `POST /mail/reconcile` answers. The sender's form is provisional until T0 S2 records which form each
 *   provider's message list shows; a change there changes `fingerprintInput` and nothing else.
 *
 * Nothing here logs, and no input or output leaves in an error.
 */
import { type MailHeaders, messageKey } from "../_shared/mime.ts";

const enc = new TextEncoder();

/** An HMAC-SHA256 signing key from raw bytes. */
export async function hmacKey(raw: Uint8Array): Promise<CryptoKey> {
  return await crypto.subtle.importKey("raw", new Uint8Array(raw), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
  ]);
}

/** An HMAC key from a secret held as base64 of 32 bytes (`MAIL_ADDRESS_KEY`, a decrypted fingerprint key).
 * Throws a bare error naming neither value on any other shape. */
export async function hmacKeyFromBase64(b64: string): Promise<CryptoKey> {
  let raw: Uint8Array;
  try {
    raw = Uint8Array.from(atob(b64.trim()), (c) => c.charCodeAt(0));
  } catch {
    throw new Error("mail key: not base64");
  }
  if (raw.length !== 32) throw new Error("mail key: not 32 bytes");
  return await hmacKey(raw);
}

/** HMAC-SHA256 of `text` (UTF-8), lower-case hex. */
export async function hmacHex(key: CryptoKey, text: string): Promise<string> {
  const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, enc.encode(text)));
  return Array.from(mac, (b) => b.toString(16).padStart(2, "0")).join("");
}

export const tokenHmac = (key: CryptoKey, token: string) => hmacHex(key, `token:${token}`);
export const nonceHmac = (key: CryptoKey, nonce: string) => hmacHex(key, `nonce:${nonce}`);

/** What `fp_hash` is computed over: the sender and the normalised subject. */
export function fingerprintInput(sender: string, subject: string): string {
  const fold = (s: string) => s.replace(/\s+/g, " ").trim().toLowerCase();
  return JSON.stringify(["fp", fold(sender), fold(subject)]);
}

/** `mail_seen`'s `msg_hash` and `fp_hash` for one original, under its account's key. */
export async function seenHashes(key: CryptoKey, head: MailHeaders): Promise<{ msgHash: string; fpHash: string }> {
  return {
    msgHash: await hmacHex(key, JSON.stringify(["msg", messageKey(head)])),
    fpHash: await hmacHex(key, fingerprintInput(head.fromAddress ?? "", head.subject ?? "")),
  };
}
