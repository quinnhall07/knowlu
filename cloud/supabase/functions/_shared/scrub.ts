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
// Fix round 1, item 1: any scheme, not only the two web ones — `webcal://` is the same capability
// URL a calendar subscription or the LMS feed hands out, and it is the exact thing legal §9 names.
const URL_RE = /\b[a-z][a-z0-9+.-]*:\/\/\S+/gi;
// The Windows account name is usually a person's name — sometimes a two-word one — and the engine's
// stdout carries it with either slash. Only that one segment is replaced — the rest of the path is
// what makes a log readable. Fix round 1, item 2: no `\s` in the exclusion set, so a name with a
// space is consumed whole rather than truncated at its first word; `[:\\\/]` and `[\\\/]` around
// `Users` accept a forward slash too.
const WINUSER = /([:\\\/]Users[\\\/])[^\\\/"']+/gi;
// `.md` before the token rule, because a note's filename would match both and it is the more
// specific fact. Dots are inside the token class so a JWT is one token rather than three.
const NOTE = /\b[\w.-]+\.md\b/g;
// Fix round 1, item 3: a named credential is a secret whatever its length — the token rule's
// 20-character floor exists to catch opaque runs, not to decide what counts as a password.
// `bearer` is itself one of the keywords, so `Authorization: Bearer <token>` claims the whole
// header as one redaction instead of leaving the scheme name standing next to a token too short
// for TOKEN to have caught on its own. Fix round 1b (ruled): bare `key` joins the list too —
// `key=sk_live_…` in a pasted log is a credential shape, and `api[_-]?key` stays alongside it so
// both spellings match.
const CREDENTIAL =
  /\b(password|passwd|pwd|token|secret|api[_-]?key|key|authorization|bearer)\s*[:=]\s*(?:bearer\s+)?\S+/gi;
const TOKEN = /\b[A-Za-z0-9_.-]{20,}\b/g;
// The prompt's own token rule (provider swap Task 3): `TOKEN` above catches any opaque run of
// 20+ characters, alphabetic ones included, and that is the right floor for a report a person
// reads — but a judge prompt carries ordinary student mail, and a plain word can clear 20
// characters on its own (a German compound such as "Hausaufgabenbesprechungstermin" is 30). This
// rule requires at least one digit somewhere in the run, which a real opaque token — a session id,
// a JWT, a confirmation code — always has and a long natural-language word never does.
const DIGIT_TOKEN = /\b(?=[A-Za-z0-9_.-]*\d)[A-Za-z0-9_.-]{20,}\b/g;

export function scrub(text: string): string {
  return text
    .replace(EMAIL, "<email>")
    .replace(URL_RE, "<url>")
    .replace(WINUSER, "$1<user>")
    .replace(NOTE, "<note>")
    .replace(CREDENTIAL, "$1=<secret>")
    .replace(TOKEN, "<token>");
}

/**
 * The narrower scrub a judge prompt goes through (provider swap Task 3, `judge_prompts.ts`'s
 * email branch): URLs, email addresses, the named-credential and bearer patterns, and opaque runs
 * that carry a digit. Deliberately NOT `scrub`: `TOKEN`'s plain 20-character floor would redact an
 * ordinary long word out of a student's own message, and neither `WINUSER` nor `NOTE` has anything
 * to catch in mail a student did not write about their own machine or vault.
 */
export function scrubForPrompt(text: string): string {
  return text
    .replace(EMAIL, "<email>")
    .replace(URL_RE, "<url>")
    .replace(CREDENTIAL, "$1=<secret>")
    .replace(DIGIT_TOKEN, "<token>");
}

export function scrubJson(v: unknown): unknown {
  if (typeof v === "string") return scrub(v);
  if (Array.isArray(v)) return v.map(scrubJson);
  if (v && typeof v === "object") {
    const out: Record<string, unknown> = {};
    // Fix round 1, item 4: a key is a string too, and a Gmail-derived key in a `payload` object is
    // exactly the kind of stray PII this function exists to catch.
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) out[scrub(k)] = scrubJson(val);
    return out;
  }
  return v;
}
