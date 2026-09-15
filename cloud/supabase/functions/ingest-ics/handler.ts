// GET /ingest-ics — the account's LMS calendar feed, fetched with the service role.
//
// The capability URL is a credential in all but name: anyone holding it reads the student's whole
// calendar. It lives in C1's `sources` table, encrypted at rest, is read here with the service
// role, and never appears in a reply, an error body or a log line (§3.1, §9). The device gets the
// feed text and parses it with the same `ingest::parse_ics` the golden `today.md` oracle covers.
import type { Entitle } from "../_shared/judge_handler.ts";

export interface IcsDeps {
  urlFor(accountId: string): Promise<string | null>;
  fetchText(url: string): Promise<string>;
  now(): Date;
}

/// Some LMS hosts 403 a request with no browser-shaped User-Agent, exactly as the event feeds do.
export const ICS_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

/**
 * The uids whose date is already past, for a **first** ingest only (R-OB-3).
 *
 * A deliberate 15-line scan and **not** a second ICS parser: it reads `UID` and the first of
 * `DTSTART` / `DTEND` per `VEVENT` and compares dates, and it does not unfold, unescape, expand a
 * recurrence or resolve a `TZID`. It cannot: the vault's timezone is on the device and so is
 * `parse_ics`, which is where the real reading happens and where the golden `today.md` oracle
 * covers it. **The device's own check is the guarantee**; this list is corroboration, and it is
 * what the wizard counts to say "14 upcoming items, 4 already past" on the finish panel.
 */
export function pastDueUids(ics: string, now: Date): string[] {
  const out: string[] = [];
  for (const block of ics.split(/BEGIN:VEVENT/i).slice(1)) {
    const body = block.split(/END:VEVENT/i)[0];
    const uid = /^UID:(.*)$/im.exec(body)?.[1]?.trim();
    if (uid === undefined || uid === "") continue;
    // `DTSTART` is what `ingest::parse_ics` reads as the due date; `DTEND` is the fallback for a
    // feed that carries only an end. Both forms: `20250902T045900Z` and a bare `20250902`.
    const stamp = (/^DTSTART[^:]*:(\d{8})/im.exec(body) ?? /^DTEND[^:]*:(\d{8})/im.exec(body))?.[1];
    if (stamp === undefined) continue;
    const day = new Date(Date.UTC(+stamp.slice(0, 4), +stamp.slice(4, 6) - 1, +stamp.slice(6, 8)));
    // Strictly before TODAY, never before *now*: an item due at 23:59 today is today's work, and
    // the one thing worse than importing a stale task is archiving a live one.
    const today = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()));
    if (day.getTime() < today.getTime()) out.push(uid);
  }
  return out;
}

export function icsHandler(entitle: Entitle, deps: IcsDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "GET") return Response.json({ error: "GET only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const url = await deps.urlFor(account_id);
      if (url === null) {
        return Response.json({ error: "no lms_ics source for this account" }, { status: 404 });
      }
      let ics: string;
      try {
        ics = await deps.fetchText(url);
      } catch {
        // Deliberately swallows the cause: a transport error's text is the one place the URL comes
        // back out, and this body reaches the device and its log.
        return Response.json({ error: "the calendar feed could not be fetched" }, { status: 502 });
      }
      if (!ics.includes("BEGIN:VCALENDAR")) {
        return Response.json({ error: "the calendar feed is not an ICS response" }, { status: 502 });
      }
      // A count the wizard can show ("we found 14 events"), and nothing that identifies the feed.
      const courses = (ics.match(/^BEGIN:VEVENT/gm) ?? []).length;
      // `first_run=1` is sent by the engine when the vault has no `today.md` (R-C2-9).
      const firstRun = new URL(req.url).searchParams.get("first_run") === "1";
      return Response.json({
        ics,
        courses,
        past_due_uids: firstRun ? pastDueUids(ics, deps.now()) : [],
      });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-ics: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "ingest failed" }, { status: 500 });
    }
  };
}
