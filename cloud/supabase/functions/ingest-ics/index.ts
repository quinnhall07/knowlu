import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { decryptString, importAesKey } from "../_shared/crypto.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { guardedFetch, MAX_BODY_BYTES, MAX_REDIRECTS } from "../_shared/guarded_fetch.ts";
import { icsHandler } from "./handler.ts";

// Imported once, lazily, and kept: `importAesKey` is a `crypto.subtle` call and re-importing it
// per request is work for nothing. Built at first use, never at module scope, so a missing secret
// is the handler's named 502 and not an opaque boot failure.
let key: CryptoKey | null = null;
async function sourcesKey(): Promise<CryptoKey> {
  if (key === null) {
    const raw = Deno.env.get("SOURCES_ENC_KEY");
    if (raw === undefined || raw === "") throw new Error("the function is missing SOURCES_ENC_KEY");
    key = await importAesKey(raw);
  }
  return key;
}

Deno.serve(icsHandler(requireActiveEntitlement, {
  async urlFor(accountId: string): Promise<string | null> {
    // `account_id=eq.` is the whole access control — the service role bypasses RLS, and
    // `judge_db_test.ts` scans for a select on a scoped table that omits it.
    const rows = await sharedDb().select(
      `sources?account_id=eq.${accountId}&kind=eq.lms_ics&select=url_ciphertext,url_iv`,
    ) as Array<{ url_ciphertext: string; url_iv: string }>;
    if (rows.length === 0) return null;
    return await decryptString(await sourcesKey(), rows[0].url_ciphertext, rows[0].url_iv);
  },
  // C2 final review F-1 (regrading m36): through the same guard `/events` uses. The URL here is
  // the account's stored LMS capability URL — a string a student pasted into the wizard, not a
  // safer string for having been round-tripped through `sources`. https only, port 443, a
  // hostname that resolves publicly, re-checked on every redirect hop, and a bounded body. A
  // refusal throws exactly as an unfetchable feed does, so the handler answers its existing named
  // 502 and reflects nothing about which guard refused.
  //
  // `ICS_USER_AGENT` is not passed through: `guardedFetch` sends its own browser-shaped
  // `User-Agent` and `Accept`, which is the same reason `ICS_USER_AGENT` existed (some LMS hosts
  // 403 a bare client). The constant stays exported because `handler_test.ts` pins its shape.
  fetchText(url: string): Promise<string> {
    return guardedFetch(url, { maxBytes: MAX_BODY_BYTES, timeoutMs: 20_000, maxHops: MAX_REDIRECTS });
  },
  now: () => new Date(),
}));
