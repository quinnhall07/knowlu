// POST /events — fetch one event feed on the student's behalf.
//
// This exists because a desktop `ureq` cannot get past several campus HTML sources: they answer a
// non-browser agent with a challenge page or a redirect chain it will not follow. A server can
// present the headers a browser presents and follow the chain, which is the whole reason §3.1
// moves the fetch (and nothing else) into the cloud.
//
// **Transport only.** No judgment happens here, no vault is touched, and the reply is the page's
// bytes. It is also an outbound fetch from our infrastructure on a URL a client supplied, so the
// URL is checked before it is used: https only, a public hostname, never a private or loopback
// address — an SSRF against the project's own metadata endpoint is the failure this prevents.
import type { Entitle } from "../_shared/judge_handler.ts";

export const EVENT_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";
export const MAX_BODY_BYTES = 8 * 1024 * 1024;

const PRIVATE =
  /^(localhost$|127\.|10\.|192\.168\.|169\.254\.|172\.(1[6-9]|2\d|3[01])\.|0\.|\[?::1\]?$|\[?fd[0-9a-f]{2}:)/i;

export function allowedUrl(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== "https:") return false;
  if (url.hostname === "" || PRIVATE.test(url.hostname)) return false;
  // A bare label ("intranet") resolves differently per network and is never a public feed.
  if (!url.hostname.includes(".")) return false;
  return true;
}

export function eventsHandler(
  entitle: Entitle,
  fetchText: (url: string) => Promise<string> = liveFetch,
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
        // The cause is swallowed deliberately: a transport error's text quotes the URL, and this
        // body reaches the device and its log.
        return Response.json({ error: "the source could not be fetched" }, { status: 502 });
      }
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`events: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "fetch failed" }, { status: 500 });
    }
  };
}

async function liveFetch(url: string): Promise<string> {
  const response = await fetch(url, {
    headers: {
      "User-Agent": EVENT_USER_AGENT,
      Accept: "text/html,application/xhtml+xml,application/json;q=0.9,text/calendar;q=0.9,*/*;q=0.8",
      "Accept-Language": "en-US,en;q=0.9",
    },
    redirect: "follow",
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.byteLength > MAX_BODY_BYTES) throw new Error("body too large");
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}
