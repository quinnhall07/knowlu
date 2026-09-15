import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { importAesKey } from "../_shared/crypto.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { CAL_USER_AGENT, calendarHandler } from "./handler.ts";

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

Deno.serve(calendarHandler(requireActiveEntitlement, {
  async personalSource(accountId: string): Promise<{ ciphertext: string; iv: string } | null> {
    // `account_id=eq.` is the whole access control — the service role bypasses RLS, and
    // `judge_db_test.ts` scans for a select on a scoped table that omits it. The primary key is
    // `(account_id, kind)`, so this is at most one row and there is no `name` to match on.
    const rows = await sharedDb().select(
      `sources?account_id=eq.${accountId}&kind=eq.calendar_ics&select=url_ciphertext,url_iv`,
    ) as Array<{ url_ciphertext: string; url_iv: string }>;
    if (rows.length === 0) return null;
    return { ciphertext: rows[0].url_ciphertext, iv: rows[0].url_iv };
  },
  // R-C2-E7: the Google arm of `/ingest-calendar` is a stub until Task 10. This wiring always
  // answers "not connected" — the handler's own 409 — rather than reaching for a
  // `read_google_grant` RPC or a `google_accounts` table that do not exist yet. Task 10 replaces
  // only this function's body; the seam in `handler.ts` (`CalendarDeps.calendarTokenFor` /
  // `googleEvents`) does not change.
  calendarTokenFor(_accountId: string): Promise<string | null> {
    return Promise.resolve(null);
  },
  googleEvents(
    _accessToken: string,
    _from: Date,
    _to: Date,
  ): Promise<Array<{ uid: string; summary: string; start: string; end: string; allDay: boolean }>> {
    throw new Error("google calendar is not connected (Task 10)");
  },
  async fetchText(url: string): Promise<string> {
    const response = await fetch(url, { headers: { "User-Agent": CAL_USER_AGENT } });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await response.text();
  },
  encKey: sourcesKey,
  now: () => new Date(),
}));
