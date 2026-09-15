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
}

/// Some LMS hosts 403 a request with no browser-shaped User-Agent, exactly as the event feeds do.
export const ICS_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

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
      return Response.json({ ics, courses });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-ics: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "ingest failed" }, { status: 500 });
    }
  };
}
