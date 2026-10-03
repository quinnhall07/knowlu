/**
 * Completing Gmail's forwarding confirmation server-side (email plan T2g.2; spec §4.3 step 4, §4.6 step 5,
 * E3). `mail_bind.ts` has already decided the message is Google's confirmation for this mailbox's own
 * requester inside its window, and handed over its one allowed link. Here:
 *
 * 1. The link is checked again against `confirmUrlAllowed`.
 * 2. The page is fetched cookie-less (`credentials: "omit"`, no cookie header, no jar) with
 *    `redirect: "manual"`: any 3xx is refused, never followed.
 * 3. The page must hold exactly one `<form>` posting by POST, whose action, resolved against the link, is
 *    on the same allow-list. Its named `hidden` and `submit` inputs are sent, nothing else.
 * 4. The form is posted the same way, cookie-less with `redirect: "manual"`; only a 2xx answer completes.
 *    A 3xx here is not followed either, and reads `post_redirect`, apart from the page's `redirect`: §4.6
 *    step 5 refuses only the page's redirect, so whether Google answers the POST with a
 *    Post/Redirect/Get, and so whether this refusal stays, is the live proof's step 4 to decide.
 *
 * Every outcome is a fixed word; no link, page or field leaves this file, and nothing here logs or throws.
 */
import { confirmUrlAllowed } from "../_shared/mail_bind.ts";

export type ConfirmOutcome =
  | "confirmed"
  | "link"
  | "redirect"
  | "page"
  | "form"
  | "action"
  | "post"
  | "post_redirect";
export type ConfirmFetch = (url: string, init: RequestInit) => Promise<Response>;

const TIMEOUT_MS = 8_000;
const MAX_PAGE = 256 * 1024;
const MAX_FIELDS = 32;
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

/** Completes Google's forwarding confirmation at `link`; `confirmed`, or the word for why not. */
export async function completeConfirmation(link: string, fetchImpl: ConfirmFetch): Promise<ConfirmOutcome> {
  if (!confirmUrlAllowed(link)) return "link";
  try {
    const page = await fetchImpl(link, init("GET", { headers: { accept: "text/html" } }));
    if (page.status >= 300 && page.status < 400) {
      await page.body?.cancel();
      return "redirect";
    }
    if (!page.ok) {
      await page.body?.cancel();
      return "page";
    }
    const html = await boundedText(page);
    if (html === null) return "page";
    const form = theForm(html, link);
    if (typeof form === "string") return form;
    const res = await fetchImpl(
      form.action,
      init("POST", {
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: form.fields.toString(),
      }),
    );
    await res.body?.cancel();
    if (res.status >= 300 && res.status < 400) return "post_redirect";
    return res.ok ? "confirmed" : "post";
  } catch {
    return "page";
  }
}
