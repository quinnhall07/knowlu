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
// Fix round 1, item 3 (Task 3 review; Quinn's preference): the prompt path drops bare `key` from
// its own credential keyword list. `scrub`'s report-path `CREDENTIAL` above keeps it (a pasted log
// really does use bare `key=`), but a judge prompt carries ordinary course mail, where "Answer
// key: Problem Set 3 is posted" is not a secret — `key` alone is too common a word in course
// material to spend as a credential keyword here. `api[_-]?key` (and `apikey` via the `[_-]?`)
// stays: that spelling is a credential shape on either path.
const PROMPT_CREDENTIAL =
  /\b(password|passwd|pwd|token|secret|api[_-]?key|authorization|bearer)\s*[:=]\s*(?:bearer\s+)?\S+/gi;
const TOKEN = /\b[A-Za-z0-9_.-]{20,}\b/g;
// Fix round 1, item 1 (Task 3 review): the first version of this rule kept `_`, `.` and `-` in the
// run, matching `TOKEN` above — so a whole separator-joined identifier was one run, and a real one
// cleared the 20-character floor: `MATH-301-002-Fall2026`, `Syllabus_ECON_202_Spring2026.docx` and
// `assignment_3_final_draft.pdf` all became `<token>`, deleting the `course` and `title` signal
// the email prompt exists to extract. This rule's class is separator-free (`_`, `.` and `-` end a
// run rather than joining it): a course code or a filename is built from short human-readable
// parts joined by those characters, so no PART of one clears a useful floor on its own, while a
// genuinely opaque token either has no separators at all (a session id, a confirmation code) or —
// a JWT — has three separately opaque, separately digit-bearing dot-joined parts, each one already
// long enough alone (`scrub_test.ts` pins a real JWT's parts at 20, 27 and 16 characters). The
// floor is 16, not 20: it is the shortest of those three JWT parts and the pinned 26-character
// confirmation code, chosen so it still clears every real positive while letting `Fall2026` (8)
// and `Spring2026` (10) survive. No `\b` here (unlike `TOKEN`): the class already excludes every
// character `\w`'s definition would otherwise call a boundary partner (`_`), so the class itself —
// not a word-boundary assertion — is what stops a match at a separator.
const DIGIT_TOKEN = /(?=[A-Za-z0-9]*\d)[A-Za-z0-9]{16,}/g;

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
 * email branch): URLs, email addresses, the named-credential and bearer patterns (minus bare
 * `key`, fix round 1 item 3), and separator-free opaque runs that carry a digit. Deliberately NOT
 * `scrub`: `TOKEN`'s plain 20-character floor would redact an ordinary long word out of a
 * student's own message, and neither `WINUSER` nor `NOTE` has anything to catch in mail a student
 * did not write about their own machine or vault.
 */
export function scrubForPrompt(text: string): string {
  return text
    .replace(EMAIL, "<email>")
    .replace(URL_RE, "<url>")
    .replace(PROMPT_CREDENTIAL, "$1=<secret>")
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
