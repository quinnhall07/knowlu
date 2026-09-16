// POST /events — fetch one event feed on the student's behalf.
//
// This exists because a desktop `ureq` cannot get past several campus HTML sources: they answer a
// non-browser agent with a challenge page or a redirect chain it will not follow. A server can
// present the headers a browser presents and follow the chain, which is the whole reason §3.1
// moves the fetch (and nothing else) into the cloud.
//
// **Transport only.** No judgment happens here, no vault is touched, and the reply is the page's
// bytes. It is also an outbound fetch from our infrastructure on a URL a client supplied, so the
// URL is checked before it is used: https only, port 443, a public hostname that RESOLVES to a
// public address — never a private, loopback or reserved one — an SSRF against the project's own
// metadata endpoint is the failure this guards, on the initial URL and on every redirect hop.
import type { Entitle } from "../_shared/judge_handler.ts";
// C2 final review F-1: the guard moved to `_shared/guarded_fetch.ts` — `/ingest-ics` and
// `/ingest-calendar` fetch student-supplied URLs too and were doing it unguarded. Re-exported
// here so every existing importer of `events/handler.ts` keeps working unchanged.
export * from "../_shared/guarded_fetch.ts";
import { allowedUrl, guardedFetch } from "../_shared/guarded_fetch.ts";

export function eventsHandler(
  entitle: Entitle,
  fetchText: (url: string) => Promise<string> = (url) => guardedFetch(url),
): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      await entitle(req);
      const body = await req.json().catch(() => ({})) as { url?: unknown };
      const url = typeof body.url === "string" ? body.url : "";
      if (!allowedUrl(url)) return Response.json({ error: "not a fetchable https URL" }, { status: 400 });
      try {
        return Response.json({ body: await fetchText(url) });
      } catch {
        // The cause is swallowed deliberately: a transport error's text quotes the URL (and a
        // guard refusal names which guard), and this body reaches the device and its log.
        return Response.json({ error: "the source could not be fetched" }, { status: 502 });
      }
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`events: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "fetch failed" }, { status: 500 });
    }
  };
}
