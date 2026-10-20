// Email plan T3a: claims from templated notification mail, with no model call (spec §4.7 step 3, D9's
// second step; §5.1's `receipt` kind as amended 2026-10-02, PQ4).
//
// A row is a vendor's notification as data: its exact sending addresses, its subject, its body and the one
// claim it yields. Adding a vendor is a row; the code below never names one (`the_code_names_no_vendor`
// reads this file and allows vendor words only between the two markers). The submission receipts are not
// restated here: `lms_receipts.ts`'s `RECEIPT_TEMPLATES` is their one home, read as `receipt` rows.
//
// **Only an `aligned` message matches** (§4.6 step 6). Authenticity is the word for the From domain, and a
// row matches only the vendor's exact address, so `aligned` here is aligned for the vendor's domain. An
// `unaligned` message, or one no row matches, answers null and goes to the model, which can never emit a
// `receipt` (T3b refuses one, T3c's grammar offers none).
//
// **Ambiguity is no claim, never a guess.** A row's due goes through `judge_due.ts`; a change whose due the
// resolver cannot place makes no claim (the model is asked instead). A course is a known course matched by
// its tokens, and none or more than one reads null. A title or evidence carrying a control character, DEL,
// U+0085 or a line separator makes no claim (T3b refuses those in any claim, §10 R11).
//
// **A row reads the whole text, and is checked against the whole text.** `mail-process` holds a template's
// claims to the model's checks against the text scrubbed but never cut (Quinn's ruling of 2026-10-04): the
// model's 2,000-character cut (`MAIL_TEXT_CHARS`) bounds what a model is shown, not what a row may read, so
// a vendor's longer preamble never sends its mail to the model. Evidence the scrub changes (an address in a
// title) stays a claim here, for that check to refuse, and a refused claim deletes the mail unread.
//
// **A notification that carries no D9 item is noise** (`NOISE_TEMPLATES`, read by `templateNoise`): the
// screen's last rule (`mail-process/screen.ts`) deletes it with no model call. It matches on the vendor's
// exact address and the whole subject shape only, on `aligned` mail only, and never a subject a claim row
// reads.
//
// Nothing here logs, and the claim carries only the title, the resolved due, the course and a verbatim
// evidence span cut at the end of the last field read, so text after it (a confirmation number) is never
// in a claim.
import { resolveDue } from "./judge_due.ts";
import { MAX_REASON_CHARS, oneLine } from "./judge_validate.ts";
import { RECEIPT_TEMPLATES } from "./lms_receipts.ts";
import type { Authenticity } from "./mail_bind.ts";

export type Topic = "schedule" | "room" | "instructor" | "office_hours" | "policy" | "other";

/** The one claim a row yields. A row's fixed effort and importance are data (D9: a `new_item` carries both). */
export type TemplateClaim =
  | { kind: "change"; field: "due" | "cancelled" }
  | { kind: "new_item"; item_kind: "task" | "exam" | "event"; effort_hours: number; importance: number }
  | { kind: "course_info"; topic: Topic }
  | { kind: "receipt" };

export interface MailTemplate {
  /** The vendor's name, the start of the claim's `why`. */
  label: string;
  /** The vendor's own sending addresses, lower-case, compared exactly against the From address. */
  senders: readonly string[];
  /** Matched against the trimmed subject line. */
  subject: RegExp;
  /** Matched against the text with every whitespace run folded to one space. Named groups: `title` (every
   * kind but `course_info`), `due` (a change of `due`; optional on a `new_item`), `course` (optional; with
   * none, the subject and text are scanned for a known course). */
  body: RegExp;
  claim: TemplateClaim;
  /** The rest of the claim's `why`, after the label: fixed text, never anything of the message. */
  why: string;
}

/** A vendor's notification that carries no D9 item (§4.7 step 3's kinds): noise, read on the sender and
 * the subject alone. Never a body pattern: a noise row deletes the mail unread, and the captures it is
 * written from keep only their subjects. */
export interface NoiseTemplate {
  /** The vendor's name (the code-names-no-vendor test reads it). */
  label: string;
  /** The vendor's own sending addresses, lower-case, compared exactly against the From address. */
  senders: readonly string[];
  /** The whole trimmed subject's shape: anchored at both ends, no flags. */
  subject: RegExp;
}

/** §5.1's claim, as amended 2026-10-02 (five kinds). A template's is always `source: template`. */
export interface MailClaim {
  kind: "new_item" | "change" | "course_info" | "noise" | "receipt";
  course: string | null;
  target: string | null;
  field: "due" | "cancelled" | null;
  value: string | null;
  item: {
    title: string;
    due: string | null;
    item_kind: "task" | "exam" | "event";
    effort_hours: number;
    importance: number;
  } | null;
  topic: Topic | null;
  evidence: string;
  why: string;
  confidence: number;
  source: "template" | "model";
  resolution: { target_id: string | null } | null;
}

/** What `mail-process` hands over after §4.7 step 1's parse: the From and Date headers, the subject, the
 * text before any cut, and the work item's authenticity word (§4.6 step 6). */
export interface TemplateMessage {
  from: string;
  subject: string;
  text: string;
  date: string;
  authenticity: Authenticity;
}

export interface TemplateContext {
  knownCourses: readonly string[];
  /** The account's IANA timezone, as `judge_due.ts` takes it. */
  timeZone?: string | null;
}

export const EVIDENCE_CHARS = 140;
const TITLE_CHARS = 200;

// vendor rows: begin
/** The notification rows (§4.7 step 3: new assignment, due soon, due date changed, announcement). A row is
 * written from a real message's exact sender, subject and body, never guessed, so this list grows as real
 * notifications are captured. A kind with no row goes to the model, which costs a call but writes nothing
 * a template would not.
 *
 * **Blackboard** (T3a.2, from 23 real notifications captured 2026-10-04). Its text/plain part (and its
 * HTML, reduced) is a style block, then a numeric id (`NNNNN.YYYYTT`), the course section code (there is
 * no course-name line), a marker line, the item's title and
 * `Due <weekday>, <month> <day>, <year> <h:mm:ss> <AM|PM> <zone>`. The title may itself say "Due" (a due
 * restated in its name), so it runs to the first `Due <weekday>,`. The course is found by scanning the
 * subject and text (its name may hold spaces, and a cross-listed course names two sections). The `due`
 * group stops before the zone: the vendor writes the time on the recipient's clock and stamps its Date
 * in UTC, and `mail-process` has no account timezone, so a zoned due would land on the UTC clock (11:59
 * PM as the next morning). Only these two markers were seen; any other marker has no row.
 * Not rows, by design: announcements, "New announcement in <course>" or a professor's "<course>: <title>"
 * (free text, and one may itself move a due: the model reads it). "New content", "New grade and feedback"
 * and "Gradable item is overdue" carry no D9 item: they are `NOISE_TEMPLATES` rows. The receipt is
 * `lms_receipts.ts`'s row. Effort and importance are fixed (D9): an hour, and the default importance 3. */
const BLACKBOARD_DUE =
  "Due (?<due>(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday), [A-Z][a-z]+ \\d{1,2}, \\d{4} " +
  "\\d{1,2}:\\d{2}:\\d{2} [AP]M) [A-Z]{2,5}\\b";

export const MAIL_TEMPLATES: readonly MailTemplate[] = [
  {
    label: "Blackboard",
    senders: ["do-not-reply@blackboard.com"],
    subject: /^New gradable item added to \S.*$/,
    body: new RegExp(`\\bNew gradable item (?<title>.+?) ${BLACKBOARD_DUE}`),
    claim: { kind: "new_item", item_kind: "task", effort_hours: 1, importance: 3 },
    why: "new gradable item",
  },
  {
    label: "Blackboard",
    senders: ["do-not-reply@blackboard.com"],
    subject: /^\S.* is due soon in \S.*$/,
    body: new RegExp(`\\bAssignment due soon (?<title>.+?) ${BLACKBOARD_DUE}`),
    claim: { kind: "new_item", item_kind: "task", effort_hours: 1, importance: 3 },
    why: "assignment due soon",
  },
];

/** The vendors' notifications that carry no D9 item, from their real subjects (T3a.2's follow-up, Quinn's
 * 2026-10-04 captures: Blackboard's "New content in <course>", "New grade and feedback for <item> in
 * <course>" and "Gradable item is overdue in <course>"). Sender and subject shape only. */
export const NOISE_TEMPLATES: readonly NoiseTemplate[] = [
  { label: "Blackboard", senders: ["do-not-reply@blackboard.com"], subject: /^New content in \S.*$/ },
  {
    label: "Blackboard",
    senders: ["do-not-reply@blackboard.com"],
    subject: /^New grade and feedback for \S.* in \S.*$/,
  },
  {
    label: "Blackboard",
    senders: ["do-not-reply@blackboard.com"],
    subject: /^Gradable item is overdue in \S.*$/,
  },
];

/** The LMS and homework-platform vendors' own sending domains: the vendor half of §4.7 step 2's allow-list
 * (`mail-process/screen.ts`). Its own list, not the rows' senders, so a vendor with no row yet (every
 * notification row, under PQ10) is still allowed past the bulk screen and goes to the model. A domain
 * covers its subdomains; every row's sender is on it (a test holds that). Blackboard, Canvas (its mail is
 * Instructure's), and the two homework platforms Knowlu reads. */
export const VENDOR_DOMAINS: readonly string[] = [
  "blackboard.com",
  "instructure.com",
  "vhlcentral.com",
  "zybooks.com",
];
// vendor rows: end

/** Every row, notifications first, then each `RECEIPT_TEMPLATES` row read as a `receipt` row. */
export const TEMPLATES: readonly MailTemplate[] = [
  ...MAIL_TEMPLATES,
  ...RECEIPT_TEMPLATES.map((r): MailTemplate => ({
    label: r.label,
    senders: r.senders,
    subject: r.subject,
    body: r.body,
    claim: { kind: "receipt" },
    why: "submission receipt",
  })),
];

/** The address inside `Name <addr>`, or the whole header when there are no angle brackets. */
function senderAddress(from: string): string {
  const bracketed = from.match(/<([^<>]+)>\s*$/);
  return (bracketed ? bracketed[1] : from).trim().toLowerCase();
}

/** A C0 control, DEL, U+0085 or a line or paragraph separator: never in a claim (§10, R11). */
function hasBreak(text: string): boolean {
  for (const ch of text) {
    const c = ch.codePointAt(0) ?? 0;
    if (c < 0x20 || c === 0x7f || c === 0x85 || c === 0x2028 || c === 0x2029) return true;
  }
  return false;
}

function tokens(text: string): string[] {
  return text.toLowerCase().match(/[a-z]+|[0-9]+/g) ?? [];
}

function containsRun(hay: readonly string[], needle: readonly string[]): boolean {
  for (let i = 0; i + needle.length <= hay.length; i++) {
    if (needle.every((t, j) => hay[i + j] === t)) return true;
  }
  return false;
}

/**
 * The one known course `text` names, by tokens (letters and digits, case folded, any separators), so
 * `202640-XX-101-001`, `XX 101` and `XX101` all name `xx-101`, and `XX-1010` does not. A known course with
 * no number never matches (a word is not a code). None, or more than one, is null.
 */
export function matchCourse(text: string, known: readonly string[]): string | null {
  const hay = tokens(text);
  const hits = new Set<string>();
  for (const course of known) {
    const needle = tokens(course);
    if (!needle.some((t) => /^[0-9]/.test(t))) continue;
    if (containsRun(hay, needle)) hits.add(course);
  }
  return hits.size === 1 ? [...hits][0] : null;
}

/** A row's pattern without a `g` or `y` flag's carried state, and with match indices when asked. */
function fresh(pattern: RegExp, indices = false): RegExp {
  return new RegExp(pattern.source, pattern.flags.replace(/[dgy]/g, "") + (indices ? "d" : ""));
}

/** One row against one message: its claim, or null when the row does not match or a field it needs
 * cannot be read without a guess. */
function claimFrom(
  t: MailTemplate,
  message: TemplateMessage,
  folded: string,
  context: TemplateContext,
): MailClaim | null {
  const m = fresh(t.body, true).exec(folded);
  if (m === null) return null;
  const groups = m.groups ?? {};
  const spans = m.indices?.groups ?? {};
  const rawTitle = (groups.title ?? "").trim();
  if (t.claim.kind !== "course_info" && (rawTitle === "" || hasBreak(rawTitle))) return null;
  const title = oneLine(rawTitle, TITLE_CHARS);

  const rawDue = groups.due;
  const due = rawDue === undefined ? null : resolveDue(rawDue, message.date, context.timeZone);
  if (t.claim.kind === "change" && t.claim.field === "due" && due === null) return null;

  const course = groups.course !== undefined
    ? matchCourse(groups.course, context.knownCourses)
    : matchCourse(`${message.subject} ${folded}`, context.knownCourses);

  // Evidence: verbatim from the folded text, from the match's start to the end of the last field read.
  const ends = ["title", "due", "course"].map((g) => spans[g]?.[1])
    .filter((e): e is number => e !== undefined);
  const end = ends.length > 0 ? Math.max(...ends) : m.index + m[0].length;
  const evidence = [...folded.slice(m.index, end)].slice(0, EVIDENCE_CHARS).join("").trim();
  if (evidence === "" || hasBreak(evidence)) return null;

  const claim: MailClaim = {
    kind: t.claim.kind,
    course,
    target: null,
    field: null,
    value: null,
    item: null,
    topic: null,
    evidence,
    why: oneLine(`${t.label} ${t.why}`, MAX_REASON_CHARS),
    confidence: 1,
    source: "template",
    resolution: null,
  };
  switch (t.claim.kind) {
    case "change":
      return { ...claim, target: title, field: t.claim.field, value: t.claim.field === "due" ? due : "true" };
    case "receipt":
      return { ...claim, target: title };
    case "new_item": {
      const { item_kind, effort_hours, importance } = t.claim;
      return { ...claim, item: { title, due, item_kind, effort_hours, importance } };
    }
    case "course_info":
      return { ...claim, topic: t.claim.topic };
  }
}

/**
 * §4.7 step 3: the claims of the first row that matches, or null when none does and the model is asked.
 * Only an `aligned` message is read; a row matches on the vendor's exact From address, its subject and its
 * body, all three.
 */
export function templateClaims(
  message: TemplateMessage,
  context: TemplateContext,
  templates: readonly MailTemplate[] = TEMPLATES,
): MailClaim[] | null {
  if (message.authenticity !== "aligned") return null;
  const sender = senderAddress(message.from);
  const subject = message.subject.trim();
  const folded = message.text.replace(/\s+/g, " ").trim();
  if (folded === "") return null;
  for (const t of templates) {
    if (!t.senders.includes(sender) || !fresh(t.subject).test(subject)) continue;
    const claim = claimFrom(t, message, folded, context);
    if (claim !== null) return [claim];
  }
  return null;
}

/**
 * Vendor noise: an `aligned` message from a noise row's exact address whose whole subject is that row's
 * shape, and which no claim row (`templates`, sender and subject) would read. A claim row's subject wins
 * so a title that happens to read like a noise subject never loses its item.
 */
export function templateNoise(
  message: Pick<TemplateMessage, "from" | "subject" | "authenticity">,
  templates: readonly MailTemplate[] = TEMPLATES,
  noise: readonly NoiseTemplate[] = NOISE_TEMPLATES,
): boolean {
  if (message.authenticity !== "aligned") return false;
  const sender = senderAddress(message.from);
  const subject = message.subject.trim();
  const reads = (t: { senders: readonly string[]; subject: RegExp }) =>
    t.senders.includes(sender) && fresh(t.subject).test(subject);
  return noise.some(reads) && !templates.some(reads);
}
