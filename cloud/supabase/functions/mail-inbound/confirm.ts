/**
 * Completing Gmail's forwarding confirmation server-side (email plan T2g.2; spec §4.3 step 4, §4.6 step 5
 * as amended 2026-10-18 by R2, E3). `mail_bind.ts` has already decided the message is Google's
 * confirmation for this mailbox's own requester inside its window, and handed over its one allowed link.
 * Here:
 *
 * 1. The link is checked again against `confirmUrlAllowed`.
 * 2. The page is fetched cookie-less (`credentials: "omit"`, no cookie header, no jar) with
 *    `redirect: "manual"`. A 3xx is followed only to a target, resolved against the current URL, that is
 *    on the same allow-list (`redirect_off_list` otherwise, a missing Location included), each hop sent
 *    the same way; nothing a response sets is carried to the next request.
 * 3. The page must hold exactly one `<form>` posting by POST, whose action, resolved against the page's
 *    final URL, is on the same allow-list. Its named `hidden` and `submit` inputs are sent, nothing else.
 * 4. The form is posted the same way; a 2xx answer completes. A 3xx is Google's Post/Redirect/Get: it is
 *    followed by a body-less GET under the same rule, and the final page's 2xx completes; an off-list or
 *    over-budget POST redirect reads `post_redirect`.
 *
 * **One budget of three hops** (`MAX_HOPS`) covers the page and the POST together; the hop past it reads
 * `redirect` on the page and `post_redirect` after the POST.
 *
 * Every outcome is a fixed word, and nothing here throws. The only lines it logs, through the caller's
 * `log`, are each hop's (`confirm hop <n>: …`, `confirm hop off list: …`, `confirm hop too many: …`): the
 * target's host and path (`hopShape`), never its query or fragment, which read as the markers `(query)`
 * and `(fragment)`; a `vf-` code reads `vf-(code)`, an address-bearing segment `(address)`, one over
 * 32 characters `(long)` and any other that is not an ordinary word `(opaque)`, so a short token never
 * reaches the log. No page, form field or link value leaves this file.
 */
import { confirmUrlAllowed } from "../_shared/mail_bind.ts";

export type ConfirmOutcome =
  | "confirmed"
  | "link"
  | "redirect"
  | "redirect_off_list"
  | "page"
  | "form"
  | "action"
  | "post"
  | "post_redirect";
export type ConfirmFetch = (url: string, init: RequestInit) => Promise<Response>;
/** Where a hop line goes; the caller's fixed-prefix logger. */
export type ConfirmLog = (line: string) => void;

const TIMEOUT_MS = 8_000;
const MAX_PAGE = 256 * 1024;
const MAX_FIELDS = 32;
/** R2: at most three redirects, across the page and the POST together. */
const MAX_HOPS = 3;
const MAX_SEGMENT = 32;
const MAX_SHAPE = 200;
/** A path segment a hop line may show verbatim: 1 to 4 digits (`0`, `2026`), or letter runs joined by
 * single `-`, `_` or `.`, ending in at most two digits (`mail`, `SetSID`, `v2`, `favicon.ico`). */
const PLAIN_SEGMENT = /^(?:[0-9]{1,4}|[A-Za-z]+(?:[-_.][A-Za-z]+)*[0-9]{0,2})$/;
const MAX_WORD = 24;

/** Whether a segment reads as an ordinary word rather than a token: `PLAIN_SEGMENT`, at most `MAX_WORD`
 * characters, fewer than three lower-to-upper case changes, and, at 12 characters or more, fewer than
 * four capitals. Anything else, a short mixed letters-and-digits run included, is opaque. */
function plainSegment(seg: string): boolean {
  if (seg.length > MAX_WORD || !PLAIN_SEGMENT.test(seg)) return false;
  const changes = (seg.match(/[a-z][A-Z]/g) ?? []).length;
  const capitals = (seg.match(/[A-Z]/g) ?? []).length;
  return changes < 3 && (seg.length < 12 || capitals < 4);
}
const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };

/** HTML character references in an attribute value, decoded once. */
function unescape(v: string): string {
  return v.replace(/&(#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[a-zA-Z]{2,8});/g, (m, body: string) => {
    if (!body.startsWith("#")) return ENTITIES[body.toLowerCase()] ?? m;
    const hex = body[1] === "x" || body[1] === "X";
    const cp = parseInt(body.slice(hex ? 2 : 1), hex ? 16 : 10);
    return cp > 0 && cp <= 0x10ffff && !(cp >= 0xd800 && cp <= 0xdfff) ? String.fromCodePoint(cp) : m;
  });
}

/** A start tag's attributes, names lower-cased; the first of a repeated name wins (as a browser's). */
function attributes(tag: string): Map<string, string> {
  const out = new Map<string, string>();
  const inner = tag.replace(/^<[a-zA-Z]+/, "").replace(/\/?>$/, "");
  for (const m of inner.matchAll(/([^\s"'<>\/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g)) {
    const name = m[1].toLowerCase();
    if (!out.has(name)) out.set(name, unescape(m[2] ?? m[3] ?? m[4] ?? ""));
  }
  return out;
}

/** The body as text, read no further than `MAX_PAGE` bytes; `null` past it. */
async function boundedText(res: Response): Promise<string | null> {
  const reader = res.body?.getReader();
  if (reader === undefined) return "";
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > MAX_PAGE) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const all = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) {
    all.set(c, at);
    at += c.length;
  }
  return new TextDecoder().decode(all);
}

/** The page's one form, as the action to post to and the fields to send; a refusal word otherwise. */
function theForm(page: string, base: string): { action: string; fields: URLSearchParams } | ConfirmOutcome {
  const forms = page.match(/<form\b[^>]*>[\s\S]*?<\/form\s*>/gi) ?? [];
  if (forms.length !== 1 || (page.match(/<form\b/gi) ?? []).length !== 1) return "form";
  const form = forms[0];
  const attrs = attributes(form.match(/^<form\b[^>]*>/i)![0]);
  if ((attrs.get("method") ?? "").toLowerCase() !== "post") return "form";
  let action: string;
  try {
    action = new URL(attrs.get("action") ?? "", base).href;
  } catch {
    return "action";
  }
  if (!confirmUrlAllowed(action)) return "action";
  const fields = new URLSearchParams();
  for (const tag of form.match(/<input\b[^>]*>/gi) ?? []) {
    const a = attributes(tag);
    const type = (a.get("type") ?? "text").toLowerCase();
    const name = a.get("name");
    if (name === undefined || name === "" || (type !== "hidden" && type !== "submit")) continue;
    if ([...fields.keys()].length >= MAX_FIELDS) return "form";
    fields.append(name, a.get("value") ?? "");
  }
  return { action, fields };
}

const init = (method: string, extra: RequestInit = {}): RequestInit => ({
  method,
  redirect: "manual",
  credentials: "omit",
  signal: AbortSignal.timeout(TIMEOUT_MS),
  ...extra,
});

/** A hop's target as a log line may carry it: the host and path (the scheme only when it is not https),
 * never the user info, query or fragment, whose presence reads as a marker. A `vf-` segment's code, an
 * address-bearing segment, a long one and any other that is not an ordinary word (`plainSegment`) are
 * replaced; the whole is capped at `MAX_SHAPE`. */
export function hopShape(target: URL | null): string {
  if (target === null) return "(no location)";
  // A `data:` or `javascript:` target's "path" is its payload: the scheme alone is named.
  if (target.protocol !== "https:" && target.protocol !== "http:") return `${target.protocol} (not http)`;
  const path = target.pathname.split("/").map((seg) =>
    seg.startsWith("vf-")
      ? "vf-(code)"
      : /@|%40/i.test(seg)
      ? "(address)"
      : seg.length > MAX_SEGMENT
      ? "(long)"
      : seg === "" || plainSegment(seg)
      ? seg
      : "(opaque)"
  ).join("/");
  const scheme = target.protocol === "https:" ? "" : `${target.protocol}//`;
  let shape = `${scheme}${target.host}${path}`;
  if (shape.length > MAX_SHAPE) shape = `${shape.slice(0, MAX_SHAPE)}(cut)`;
  const marks = [
    target.username !== "" || target.password !== "" ? " (userinfo)" : "",
    target.href.split("#")[0].includes("?") ? " (query)" : "",
    target.href.includes("#") ? " (fragment)" : "",
  ];
  return shape + marks.join("");
}

const isRedirect = (res: Response) => res.status >= 300 && res.status < 400;
const getPage = (url: string, fetchImpl: ConfirmFetch) =>
  fetchImpl(url, init("GET", { headers: { accept: "text/html" } }));

/** Follows `res`'s redirects from `url` while the shared budget lasts, each target on the allow-list and
 * fetched by `getPage`; the last answer and its URL, or why the chain was refused. */
async function follow(
  res: Response,
  url: string,
  fetchImpl: ConfirmFetch,
  hops: { used: number },
  log: ConfirmLog,
): Promise<{ res: Response; url: string } | "too_many" | "off_list"> {
  while (isRedirect(res)) {
    await res.body?.cancel();
    const location = res.headers.get("location");
    let target: URL | null = null;
    try {
      target = location === null ? null : new URL(location, url);
    } catch {
      log("confirm hop off list: (unparsable)");
      return "off_list";
    }
    if (hops.used >= MAX_HOPS) {
      log(`confirm hop too many: ${hopShape(target)}`);
      return "too_many";
    }
    if (target === null || !confirmUrlAllowed(target.href)) {
      log(`confirm hop off list: ${hopShape(target)}`);
      return "off_list";
    }
    hops.used++;
    log(`confirm hop ${hops.used}: ${hopShape(target)}`);
    url = target.href;
    res = await getPage(url, fetchImpl);
  }
  return { res, url };
}

/** Completes Google's forwarding confirmation at `link`; `confirmed`, or the word for why not. Each
 * redirect followed or refused is one line to `log` (`hopShape`); without one, nothing is logged. */
export async function completeConfirmation(
  link: string,
  fetchImpl: ConfirmFetch,
  log: ConfirmLog = () => {},
): Promise<ConfirmOutcome> {
  if (!confirmUrlAllowed(link)) return "link";
  const hops = { used: 0 };
  try {
    const reached = await follow(await getPage(link, fetchImpl), link, fetchImpl, hops, log);
    if (reached === "too_many") return "redirect";
    if (reached === "off_list") return "redirect_off_list";
    const page = reached.res;
    if (!page.ok) {
      await page.body?.cancel();
      return "page";
    }
    const html = await boundedText(page);
    if (html === null) return "page";
    const form = theForm(html, reached.url);
    if (typeof form === "string") return form;
    const posted = await fetchImpl(
      form.action,
      init("POST", {
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: form.fields.toString(),
      }),
    );
    const after = await follow(posted, form.action, fetchImpl, hops, log);
    if (typeof after === "string") return "post_redirect";
    await after.res.body?.cancel();
    return after.res.ok ? "confirmed" : "post";
  } catch {
    return "page";
  }
}
