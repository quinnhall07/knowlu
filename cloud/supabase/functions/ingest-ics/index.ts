import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { decryptString, importAesKey } from "../_shared/crypto.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { ICS_USER_AGENT, icsHandler } from "./handler.ts";

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
  async fetchText(url: string): Promise<string> {
    const response = await fetch(url, { headers: { "User-Agent": ICS_USER_AGENT } });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await response.text();
  },
  now: () => new Date(),
}));
