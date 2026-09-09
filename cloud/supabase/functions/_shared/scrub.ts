/**
 * The redaction an issue report goes through. The device runs its own twin (`app/src/report.rs`) and
 * **shows the result to the user before anything is sent** — that preview is the documented consent
 * a human needs to read Gmail-derived text at all (Google's Limited Use, spec §5.3). This end runs it
 * again, because a report is the one payload a person opens and a client is not the only way to post.
 *
 * Order matters: URLs before tokens, or a URL's path segment reads as a token and the sentence loses
 * the shape a reader needs.
 */
const EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
const URL_RE = /\bhttps?:\/\/\S+/g;
// The Windows account name is usually a person's name, and the engine's stdout is full of paths that
// carry it. Only that one segment is replaced — the rest of the path is what makes a log readable.
const WINUSER = /(:\\Users\\)[^\\/\s"']+/gi;
// `.md` before the token rule, because a note's filename would match both and it is the more
// specific fact. Dots are inside the token class so a JWT is one token rather than three.
const NOTE = /\b[\w.-]+\.md\b/g;
const TOKEN = /\b[A-Za-z0-9_.-]{20,}\b/g;

export function scrub(text: string): string {
  return text
    .replace(EMAIL, "<email>")
    .replace(URL_RE, "<url>")
    .replace(WINUSER, "$1<user>")
    .replace(NOTE, "<note>")
    .replace(TOKEN, "<token>");
}

export function scrubJson(v: unknown): unknown {
  if (typeof v === "string") return scrub(v);
  if (Array.isArray(v)) return v.map(scrubJson);
  if (v && typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) out[k] = scrubJson(val);
    return out;
  }
  return v;
}
