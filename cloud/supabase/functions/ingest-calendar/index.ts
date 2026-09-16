import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { importAesKey } from "../_shared/crypto.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { accessTokenFromRefresh } from "../_shared/google_token.ts";
import { CALENDAR_SCOPE } from "../_shared/google_scopes.ts";
import { guardedFetch, MAX_BODY_BYTES, MAX_REDIRECTS } from "../_shared/guarded_fetch.ts";
import { calendarHandler } from "./handler.ts";

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
  async calendarTokenFor(accountId: string): Promise<string | null> {
    // Returns nothing when the grant was never made, was revoked, or carries only the Gmail scope
    // — `p_scope` is checked inside the RPC (Task 10 step 7), not here.
    const refreshToken = await sharedDb().rpc("read_google_grant", {
      p_account: accountId,
      p_scope: CALENDAR_SCOPE,
    });
    if (typeof refreshToken !== "string" || refreshToken === "") return null;
    const clientId = Deno.env.get("GOOGLE_CLIENT_ID") ?? "";
    // A missing client id is "not connected" here too — the same 409 `calendarHandler` already
    // gives for no grant at all, never a 500 for a deployment that has not set P2 yet.
    if (clientId === "") return null;
    return await accessTokenFromRefresh(refreshToken, {
      clientId,
      clientSecret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
      fetch: globalThis.fetch,
    });
  },
  async googleEvents(
    accessToken: string,
    from: Date,
    to: Date,
  ): Promise<Array<{ uid: string; summary: string; start: string; end: string; allDay: boolean }>> {
    // `singleEvents=true` is what makes Google expand recurrences, so `calfeed`'s own RRULE
    // handling never sees one.
    const url = new URL("https://www.googleapis.com/calendar/v3/calendars/primary/events");
    url.searchParams.set("singleEvents", "true");
    url.searchParams.set("orderBy", "startTime");
    url.searchParams.set("timeMin", from.toISOString());
    url.searchParams.set("timeMax", to.toISOString());
    const response = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const body = await response.json() as {
      items?: Array<{
        id: string;
        summary?: string;
        start: { dateTime?: string; date?: string };
        end: { dateTime?: string; date?: string };
      }>;
    };
    return (body.items ?? []).map((item) => ({
      uid: item.id,
      summary: item.summary ?? "",
      start: (item.start.dateTime ?? item.start.date) as string,
      end: (item.end.dateTime ?? item.end.date) as string,
      allDay: item.start.date !== undefined,
    }));
  },
  // C2 final review F-1 (regrading m36): the same guard `/events` and `/ingest-ics` use. The URL
  // here is the student's secret iCal address, decrypted from `sources` a few lines above — still
  // a string they pasted, still capable of naming a loopback or a metadata endpoint. A guard
  // refusal throws exactly as an unfetchable feed does, so the handler's existing named 502 is the
  // answer and nothing about the URL or the guard comes back. `CAL_USER_AGENT` is not passed:
  // `guardedFetch` sends its own browser-shaped headers, which is what that constant was for.
  fetchText(url: string): Promise<string> {
    return guardedFetch(url, { maxBytes: MAX_BODY_BYTES, timeoutMs: 20_000, maxHops: MAX_REDIRECTS });
  },
  encKey: sourcesKey,
  now: () => new Date(),
}));
