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

export const EVENT_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";
export const MAX_BODY_BYTES = 8 * 1024 * 1024;
export const FETCH_TIMEOUT_MS = 20_000;
export const MAX_REDIRECTS = 5;

// A trailing-dot FQDN ("localhost.") is the same host DNS-wise but defeats a bare `$` anchor —
// caught here for the cheap, synchronous case; `hostResolvesPublicly` below is what actually
// closes this (and the two names/addresses that resolve to a private one, which no hostname regex
// can catch at all: `a.127.0.0.1.nip.io`, `metadata.google.internal`).
const PRIVATE_HOSTNAME =
  /^(localhost\.?$|127\.|10\.|192\.168\.|169\.254\.|172\.(1[6-9]|2\d|3[01])\.|0\.|\[?::1\]?$|\[?::\]?$|\[?fd[0-9a-f]{2}:)/i;

/**
 * The syntactic guard: does this URL even have the SHAPE a public https feed could have? Cheap,
 * synchronous, and checked before any DNS lookup or fetch — https only, port 443 (or none), a
 * non-empty hostname with at least one dot (a bare label like "intranet" resolves differently per
 * network and is never a public feed), and nothing in `PRIVATE_HOSTNAME`'s syntactic denylist.
 * This is necessary but not sufficient: a hostname can pass every check here and still resolve to
 * a private address, which is what `hostResolvesPublicly` is for.
 */
export function allowedUrl(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== "https:") return false;
  if (url.hostname === "" || PRIVATE_HOSTNAME.test(url.hostname)) return false;
  if (!url.hostname.includes(".")) return false;
  // Nothing off 443: a feed on a nonstandard port is not a public calendar site, and a port is
  // one more way to reach a service this guest is not the internet's front door for.
  if (url.port !== "" && url.port !== "443") return false;
  return true;
}

/**
 * Is this a loopback, link-local, private or reserved address? Takes the RESOLVED address, never
 * the hostname — that is `hostResolvesPublicly`'s job. Classifies an IPv4-mapped IPv6 address
 * (`::ffff:a.b.c.d`) by its embedded IPv4.
 */
function isPrivateAddress(addr: string): boolean {
  const mapped = /^::ffff:(\d+\.\d+\.\d+\.\d+)$/i.exec(addr);
  const ip = mapped ? mapped[1] : addr;
  if (ip.includes(".")) {
    const parts = ip.split(".").map(Number);
    if (parts.length !== 4 || parts.some((p) => !Number.isInteger(p) || p < 0 || p > 255)) return true;
    const [a, b] = parts;
    if (a === 127 || a === 10 || a === 0) return true;
    if (a === 192 && b === 168) return true;
    if (a === 169 && b === 254) return true;
    if (a === 172 && b >= 16 && b <= 31) return true;
    if (a === 100 && b >= 64 && b <= 127) return true; // 100.64.0.0/10 (carrier-grade NAT)
    if (a === 198 && (b === 18 || b === 19)) return true; // 198.18.0.0/15 (benchmarking)
    return false;
  }
  const low = ip.toLowerCase();
  if (low === "::1" || low === "::") return true;
  if (low.startsWith("fc") || low.startsWith("fd")) return true; // fc00::/7 (unique local)
  if (["fe8", "fe9", "fea", "feb"].some((p) => low.startsWith(p))) return true; // fe80::/10
  return false;
}

/** A literal IPv4 or IPv6 address in a URL's `hostname`, or `null` for an actual name. Checked
 * before DNS resolution: querying a resolver with a literal address is unreliable and unnecessary
 * — `isPrivateAddress` applies directly. */
function ipLiteral(hostname: string): string | null {
  const bare = hostname.replace(/^\[|\]$/g, "");
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(bare) || bare.includes(":")) return bare;
  return null;
}

export type ResolveDns = (hostname: string, recordType: "A" | "AAAA") => Promise<string[]>;

function defaultResolveDns(hostname: string, recordType: "A" | "AAAA"): Promise<string[]> {
  return recordType === "A" ? Deno.resolveDns(hostname, "A") : Deno.resolveDns(hostname, "AAAA");
}

/**
 * SSRF finding 4: a hostname can pass `allowedUrl` and still resolve to a private address — the
 * two measured examples (`a.127.0.0.1.nip.io`, `metadata.google.internal`) resolve legitimately
 * and no hostname pattern catches them. A failure of either record type is "no records" for that
 * type, not a refusal by itself; a host with NO records at all (both queries empty) is refused,
 * and so is a host where ANY returned address is private. **Residual risk**: this is a
 * point-in-time check — Deno's `fetch` cannot be pinned to the address just resolved, so a
 * DNS-rebinding attacker could in principle change the answer between this check and the fetch a
 * few lines below. Re-validating on every redirect hop narrows the window; it does not close it.
 */
async function hostResolvesPublicly(hostname: string, resolveDns: ResolveDns): Promise<boolean> {
  const literal = ipLiteral(hostname);
  if (literal !== null) return !isPrivateAddress(literal);
  const [a, aaaa] = await Promise.all([
    resolveDns(hostname, "A").catch(() => [] as string[]),
    resolveDns(hostname, "AAAA").catch(() => [] as string[]),
  ]);
  const addresses = [...a, ...aaaa];
  if (addresses.length === 0) return false;
  return addresses.every((addr) => !isPrivateAddress(addr));
}

/** Read a response body up to `maxBytes`, cancelling the stream the moment the running total
 * crosses it rather than buffering the whole thing first (SSRF finding 6). A `Content-Length`
 * check is a free early exit but is untrusted — a server can lie about it — so the streamed check
 * runs regardless of whether the header was present or believable. */
async function readBounded(response: Response, maxBytes: number): Promise<string> {
  const declared = Number(response.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > maxBytes) {
    await response.body?.cancel();
    throw new Error("body too large");
  }
  const reader = response.body?.getReader();
  if (!reader) return "";
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > maxBytes) {
      await reader.cancel();
      throw new Error("body too large");
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}

export interface GuardedFetchDeps {
  fetchImpl?: typeof fetch;
  resolveDns?: ResolveDns;
  maxBytes?: number;
  timeoutMs?: number;
  maxHops?: number;
}

/**
 * Fetch one URL as a real browser would, with every SSRF guard applied on the initial URL and on
 * EVERY redirect hop it follows (finding 3) — `redirect: "manual"` and a bounded loop, because
 * `redirect: "follow"` would let a public host answer with a redirect to a private one and void
 * the whole guard. `fetchImpl` and `resolveDns` are injectable so a test can prove every one of
 * these paths with no socket and no real DNS query; production supplies neither and gets the real
 * `fetch` and `Deno.resolveDns`.
 */
export async function guardedFetch(url: string, deps: GuardedFetchDeps = {}): Promise<string> {
  const fetchImpl = deps.fetchImpl ?? fetch;
  const resolveDns = deps.resolveDns ?? defaultResolveDns;
  const maxBytes = deps.maxBytes ?? MAX_BODY_BYTES;
  const timeoutMs = deps.timeoutMs ?? FETCH_TIMEOUT_MS;
  const maxHops = deps.maxHops ?? MAX_REDIRECTS;

  let current = url;
  for (let hop = 0;; hop++) {
    if (hop > maxHops) throw new Error("too many redirects");
    if (!allowedUrl(current)) throw new Error("blocked: not a fetchable https URL");
    if (!(await hostResolvesPublicly(new URL(current).hostname, resolveDns))) {
      throw new Error("blocked: the host resolves to a private address");
    }
    const response = await fetchImpl(current, {
      headers: {
        "User-Agent": EVENT_USER_AGENT,
        Accept: "text/html,application/xhtml+xml,application/json;q=0.9,text/calendar;q=0.9,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
      },
      redirect: "manual",
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (response.status >= 300 && response.status < 400) {
      const location = response.headers.get("location");
      if (location === null) throw new Error("redirect with no location");
      // Re-validated at the top of the next iteration — never followed on trust.
      current = new URL(location, current).toString();
      continue;
    }
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await readBounded(response, maxBytes);
  }
}

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
