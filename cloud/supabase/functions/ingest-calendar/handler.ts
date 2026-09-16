// GET /ingest-calendar?name=<feed name> — the account's calendar, as ICS.
//
// Two sources, one shape:
//   * `google_calendar` — the Google grant's own events for the engine's window, rendered as ICS.
//     Read-only, `calendar.readonly` only, and the grant is the same one Gmail uses when the
//     student has taken that step too (§11a).
//   * `calendar_ics` — the secret iCal address the wizard captured, fetched server-side. This is
//     the path C1 ships and the fallback while the OAuth scope is unverified; serving it here as
//     well means the address stops needing to be in the vault once the account has it.
//
// The secret address is a capability URL, exactly like the LMS one: it never appears in a reply,
// an error body or a log line.
import { decryptString } from "../_shared/crypto.ts";
import type { Entitle } from "../_shared/judge_handler.ts";

/** The engine's own window (`calfeed::HORIZON_DAYS`). Asking for more would be discarded. */
export const HORIZON_DAYS = 28;
export const CAL_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

/**
 * The `?name=` -> source mapping, and the ONLY place it is decided (ruling R-X-9).
 *
 *   `personal` (and the default) -> the account's `sources` row where `kind = 'calendar_ics'`
 *   `google`                     -> the account's `google_accounts` row, if its `scopes` carry
 *                                   `calendar.readonly`
 *   anything else                -> a 404 that names the name
 *
 * **`sources` has no `name` column** — its primary key is `(account_id, kind)` — and **the Google
 * calendar is not a `sources` row at all**: a grant has no URL, and both `url_ciphertext` and
 * `url_iv` are `not null`. `google_calendar` exists in C1's check constraint as a reserved value
 * so neither stream ever has to edit the other's constraint, and C2 never writes a row of that
 * kind. The two names are the two `calendars:` entries the vault carries, which is what makes the
 * name in the query string and the name in the vault the same word.
 */
export const CALENDAR_NAMES = { personal: "calendar_ics", google: "google_accounts" } as const;

export interface CalendarDeps {
  /** The account's `sources` row of kind `calendar_ics`, or null. */
  personalSource(accountId: string): Promise<{ ciphertext: string; iv: string } | null>;
  /** A fresh Google access token, or null when the grant is gone or lacks the calendar scope. */
  calendarTokenFor(accountId: string): Promise<string | null>;
  /** Google Calendar's events for the window, already normalised. */
  googleEvents(accessToken: string, from: Date, to: Date): Promise<
    Array<{
      uid: string;
      summary: string;
      start: string;
      end: string;
      allDay: boolean;
    }>
  >;
  fetchText(url: string): Promise<string>;
  encKey(): Promise<CryptoKey>;
  now(): Date;
}

function stamp(value: string, allDay: boolean): string {
  // `YYYYMMDD` for an all-day event, `YYYYMMDDTHHMMSSZ` otherwise — the two forms
  // `calfeed::parse_dt` reads. Google returns RFC 3339, so this is a reshape and not a parse.
  const d = new Date(value);
  const pad = (n: number) => String(n).padStart(2, "0");
  const day = `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}`;
  if (allDay) return day;
  return `${day}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`;
}

function escapeIcs(text: string): string {
  return text.replace(/\\/g, "\\\\").replace(/;/g, "\\;").replace(/,/g, "\\,").replace(/\r?\n/g, "\\n");
}

export function toIcs(
  events: Array<{ uid: string; summary: string; start: string; end: string; allDay: boolean }>,
): string {
  const lines = ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Knowlu//ingest-calendar//EN"];
  for (const e of events) {
    lines.push("BEGIN:VEVENT");
    lines.push(`UID:${escapeIcs(e.uid)}`);
    lines.push(`SUMMARY:${escapeIcs(e.summary)}`);
    lines.push(e.allDay ? `DTSTART;VALUE=DATE:${stamp(e.start, true)}` : `DTSTART:${stamp(e.start, false)}`);
    lines.push(e.allDay ? `DTEND;VALUE=DATE:${stamp(e.end, true)}` : `DTEND:${stamp(e.end, false)}`);
    lines.push("END:VEVENT");
  }
  lines.push("END:VCALENDAR");
  // CRLF, because that is what RFC 5545 says and what every other feed the engine reads uses.
  return lines.join("\r\n") + "\r\n";
}

export function calendarHandler(entitle: Entitle, deps: CalendarDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "GET") return Response.json({ error: "GET only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const name = new URL(req.url).searchParams.get("name") ?? "personal";
      if (!(name in CALENDAR_NAMES)) {
        return Response.json({ error: `no calendar named '${name}' for this account` }, { status: 404 });
      }
      if (name === "personal") {
        const source = await deps.personalSource(account_id);
        if (source === null) {
          return Response.json({ error: "no calendar named 'personal' for this account" }, { status: 404 });
        }
        const url = await decryptString(await deps.encKey(), source.ciphertext, source.iv);
        let ics: string;
        try {
          ics = await deps.fetchText(url);
        } catch {
          // The cause is swallowed: a transport error's text quotes the secret address.
          return Response.json({ error: "the calendar could not be fetched" }, { status: 502 });
        }
        if (!ics.includes("BEGIN:VCALENDAR")) {
          return Response.json({ error: "the calendar is not an ICS response" }, { status: 502 });
        }
        return Response.json({ ics, source: "calendar_ics" });
      }
      const token = await deps.calendarTokenFor(account_id);
      if (token === null) {
        // Not connected, revoked, or connected for Gmail only — all one answer, and the device
        // degrades to `using snapshot` rather than losing the day's capacity model.
        return Response.json({ error: "the Google calendar is not connected" }, { status: 409 });
      }
      const from = deps.now();
      const to = new Date(from.getTime() + HORIZON_DAYS * 86_400_000);
      try {
        return Response.json({
          ics: toIcs(await deps.googleEvents(token, from, to)),
          source: "google_calendar",
        });
      } catch {
        return Response.json({ error: "the calendar could not be fetched" }, { status: 502 });
      }
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-calendar: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "calendar failed" }, { status: 500 });
    }
  };
}
