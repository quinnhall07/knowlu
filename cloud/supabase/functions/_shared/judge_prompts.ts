// The three prompts and the three schemas, versioned. Server-side from C2 on, so that changing
// how a judgment is asked for is a deploy and not an app release (cloud design §10).
//
// The task prompt is `judge::prompt_for` moved across the boundary, with the same four bounds —
// body 1200 characters, weights 600, preferences 600, reason 140 — and the same rules, because
// the frozen behaviour these bounds protect is "the question can never be pushed out of the
// context by a note that pasted a syllabus". Fix round 1 (Task 3 review): each bound still caps
// the CONTENT `clip` keeps — a clipped field's own marker (13 characters, " …[truncated]") is
// appended beyond the bound, not carved out of it, so a fully clipped 1200-character body prompt
// emits 1213 characters, the marker being the frame telling the model it cut something, not part
// of what it cut. `judge_prompts_test.ts` pins the exact count.
import { ABSOLUTE_DUE_RE, type MailKind, oneLine } from "./judge_validate.ts";
import { scrubForPrompt } from "./scrub.ts";

export const MAX_BODY_CHARS = 1200;
export const MAX_WEIGHTS_CHARS = 600;
export const MAX_PREFS_CHARS = 600;

export interface Prompt {
  system: string;
  user: string;
  schema: Record<string, unknown>;
}

// Provider swap Task 3 (prompt_version task-2 / event-2 / email-2): a clipped field used to give
// the model no sign that its last sentence might be a cut-off fragment rather than the note's own
// ending — a body that trailed off mid-word read as complete. The marker names the cut so the
// model can weigh (or ask for) the missing tail instead of treating a truncated body as a full one.
// The marker is appended BEYOND `max`, never carved out of it (fix round 1, Task 3 review): `max`
// bounds the content this function keeps, and the 13-character marker is the frame around that
// content, not a thirteenth of it — a clipped MAX_BODY_CHARS body emits 1213 characters, not 1200.
function clip(text: string, max: number): string {
  const chars = [...text];
  return chars.length > max ? chars.slice(0, max).join("") + " …[truncated]" : text;
}

const TASK_SCHEMA = {
  type: "object",
  properties: {
    course: { type: ["string", "null"] },
    effort_hours: { type: "number" },
    importance: { type: "integer", enum: [1, 2, 3, 4, 5] },
    importance_reason: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["course", "effort_hours", "importance", "importance_reason", "confidence"],
  additionalProperties: false,
};

// CHECKPOINT J-1 (ruled 2026-09-22, design (a)): `unsure` is a fourth, first-class verdict word,
// not an error path. Whether an event obliges a particular student is usually absent from the
// event's own text, and forcing a choice among the other three there measurably breaks both
// accuracy and calibration on the unanswerable slice (docs/notes/2026-09-22-jev-what-people-built.md
// §3) — the same failure a router that always guesses shows on a benchmark's hard slice. `unsure`
// lets the model say so honestly, and the device's `record_verdict` (`engine/src/eventledger.rs`)
// writes it like any other verdict, so the uid is never re-asked (events spec §7's
// one-verdict-per-uid-forever rule already does the rest).
const EVENT_SCHEMA = {
  type: "object",
  properties: {
    verdict: { type: "string", enum: ["obligation", "opportunity", "drop", "unsure"] },
    why: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["verdict", "why", "confidence"],
  additionalProperties: false,
};

// `type: ["integer","null"]` rather than an `enum` carrying a null member: a mixed-type enum is
// outside the safe subset of JSON Schema for structured outputs, and the range is re-checked by
// `validate` anyway (it clamps 1-5), so nothing is lost by asking for it in prose instead.
const EMAIL_SCHEMA = {
  type: "object",
  properties: {
    tier: { type: "string", enum: ["task", "borderline", "event", "opportunity", "information", "completion"] },
    title: { type: "string" },
    course: { type: ["string", "null"] },
    due: { type: ["string", "null"] },
    effort_hours: { type: ["number", "null"] },
    importance: { type: ["integer", "null"] },
    why: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["tier", "title", "course", "due", "effort_hours", "importance", "why", "confidence"],
  additionalProperties: false,
};

function str(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function list(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];
}

/**
 * The system prompt, with the per-account values left as placeholders.
 *
 * Split out from `buildPrompt` so `promptHash` can hash the TEMPLATE and mean it: `slice_hours` is
 * a per-account number, and rendering it into the hashed text would give two accounts different
 * system prompts under one hash — the one thing the hash exists to prevent (M1).
 */
export function systemTemplate(kind: "task" | "event" | "email"): string {
  if (kind === "task") {
    // task-3 (effort anchors, 2026-10-23; `<stamp>_task_effort_anchors.sql`): task-2's one effort
    // line gave no scale, and items judged from a title, a slug and a due date came back at a
    // median of 8 hours. The anchors, the above-8 rule and the vague-title rule fix the scale; the
    // schema, the 0.25-40 clamp and `{slice_hours}` do not move. The vague-title rule deliberately
    // asks for no lower confidence: an answer under CONFIDENCE_FLOOR (0.6) is `below floor`, no
    // verdict at all, and the device already labels every model estimate `effort_confidence: low`.
    // This now differs from the device's local `judge::prompt_for`, which the runtime-removal lane
    // deletes and which is not extended.
    return [
      "You estimate effort and importance for one university assignment.",
      "Rules:",
      "- effort_hours: the realistic time in hours for a typical undergraduate to finish this one assignment, 0.25 to 40.",
      "- effort anchors: a reading, discussion post or quiz 0.25 to 1; a weekly homework, problem set or lab 1 to 3; a short essay or report 2 to 4, a longer one 1 to 2 per page; a project milestone 3 to 8; an exam 3 to 6 of study.",
      "- answer above 8 only when the title or body clearly says a large project or a long paper.",
      "- a vague title with nothing else given: answer the middle of its likely type's range, never a large number.",
      "- points or a grade weight, when given, scale the estimate: a 5-point item is small.",
      "- a typical work session in this student's plan is {slice_hours} hours.",
      "- importance: 1 to 5, grounded in the grade weights given. With no weights given, answer 3 and say so in importance_reason.",
      "- importance_reason: one line, under 140 characters, no line breaks.",
      "- course: the slug given below, or null when the slug given is null.",
      "- confidence: 0 to 1, how sure you are of effort_hours and importance.",
    ].join("\n");
  }
  if (kind === "event") {
    return [
      "You decide whether one campus event is worth a student's attention.",
      "Rules:",
      "- verdict: obligation (the student is expected there), opportunity (worth offering), drop, or unsure.",
      "- unsure: answer unsure when the event's own text does not say whether it applies to this student — do not guess the audience to force obligation, opportunity or drop.",
      "- an event aimed at faculty, staff, alumni or graduate students is a drop.",
      "- a standing exhibit, an office-hours block or a recurring drop-in is a drop.",
      "- why: one line, under 140 characters, no line breaks, no double quotes.",
      "- confidence: 0 to 1.",
    ].join("\n");
  }
  return [
    "You triage one email for a university student, into exactly one of six tiers.",
    "Tiers:",
    "- task: it clearly creates work with a deadline the student must do.",
    "- borderline: it might create work; a human should decide.",
    "- event: it announces something happening at a stated date and time.",
    "- opportunity: an application, a scholarship, a job, a research post, a dinner worth offering.",
    "- completion: this email confirms the student already submitted or finished a specific piece of work.",
    "- information: everything else, including payment receipts, newsletters, notifications and marketing.",
    "Rules:",
    "- title: what the resulting task or card should be called, one line, under 200 characters; for completion, the name of that piece of work exactly as the email gives it.",
    "- A grade or feedback being posted is not completion: missing work can be graded too.",
    '- due: the deadline exactly as the email states it. Give the phrase as written ("Friday", "next week", "the end of the month", "tomorrow at 5pm") -- do NOT compute or resolve it yourself, that happens after you answer. Only when the email itself gives an explicit calendar date (for example "October 3" or "10/3") may you answer that date, as YYYY-MM-DD or YYYY-MM-DDTHH:MM. With no deadline stated, answer null.',
    "- effort_hours and importance: only for tier task, else null. importance is a whole number 1 to 5.",
    "- course: one of the known course slugs given below, or null.",
    "- with no message body, judge from the subject and sender alone and answer a confidence at or below 0.5.",
    "- why: one line, under 140 characters, no line breaks, no double quotes.",
    "- confidence: 0 to 1.",
  ].join("\n");
}

export function schemaFor(kind: "task" | "event" | "email"): Record<string, unknown> {
  return kind === "task" ? TASK_SCHEMA : kind === "event" ? EVENT_SCHEMA : EMAIL_SCHEMA;
}

export function buildPrompt(
  kind: "task" | "event" | "email",
  item: Record<string, unknown>,
  seed: Record<string, unknown>,
): Prompt {
  const schema = schemaFor(kind);
  if (kind === "task") {
    const slice = typeof seed.slice_hours === "number" ? seed.slice_hours : 1.5;
    const system = systemTemplate("task").replace("{slice_hours}", String(slice));
    const parts: string[] = [];
    const prefs = clip(str(seed.preferences), MAX_PREFS_CHARS);
    if (prefs !== "") parts.push(`The student's stated preferences:\n${prefs}`);
    parts.push(`Course slug: ${str(seed.course) || "null"}`);
    const weights = clip(str(seed.weights), MAX_WEIGHTS_CHARS);
    if (weights !== "") parts.push(`Grade weights:\n${weights}`);
    parts.push(`Title: ${oneLine(str(item.title), 200)}`);
    // An absent due date is absent, never the word "None": a model shown "Due: None" reliably
    // treats it as a date it failed to read rather than as an assignment without one.
    if (str(item.due) !== "") parts.push(`Due: ${str(item.due)}`);
    const body = clip(str(item.body).trim(), MAX_BODY_CHARS);
    if (body !== "") parts.push(`Body:\n${body}`);
    return { system, user: parts.join("\n"), schema };
  }

  if (kind === "event") {
    const parts = [
      `Title: ${oneLine(str(item.title), 200)}`,
      `When: ${str(item.start)} to ${str(item.end)}`,
      `Source: ${str(item.source)}`,
    ];
    if (str(item.organizer) !== "") parts.push(`Organizer: ${oneLine(str(item.organizer), 120)}`);
    if (str(item.location) !== "") parts.push(`Location: ${oneLine(str(item.location), 120)}`);
    if (list(item.categories).length > 0) parts.push(`Categories: ${list(item.categories).join(", ")}`);
    if (list(item.audiences).length > 0) parts.push(`Audiences: ${list(item.audiences).join(", ")}`);
    const description = clip(str(item.description).trim(), MAX_BODY_CHARS);
    if (description !== "") parts.push(`Description:\n${description}`);
    const interests = clip(str(seed.interests), MAX_PREFS_CHARS);
    if (interests !== "") parts.push(`The student's stated interests:\n${interests}`);
    return { system: systemTemplate("event"), user: parts.join("\n"), schema };
  }

  const known = list(seed.known_courses);
  // The scrub site (provider swap Task 3): the one place both the Gmail-read path
  // (`gmail-read/handler.ts`) and the device's `judge-email` path build the email prompt, because
  // both hand their item straight to `judge()`, which calls `buildPrompt` here. `From` is NEVER
  // scrubbed — `judge_rules.ts`'s `featureMap` maps `item.from` into the `source` promotion
  // feature, and `scrub`/`scrubForPrompt` would replace an email address in it.
  const subject = scrubForPrompt(str(item.subject));
  const parts = [
    `Subject: ${oneLine(subject, 200)}`,
    `From: ${oneLine(str(item.from), 200)}`,
    `Date: ${str(item.date)}`,
  ];
  if (known.length > 0) parts.push(`Known course slugs: ${known.join(", ")}`);
  // Scrub before clip, not after: clipping a token in half would leave a partial secret on the
  // wire, where scrubbing first replaces it with a short, fixed-length placeholder that clip then
  // has no reason to cut.
  const scrubbedText = scrubForPrompt(str(item.text).trim());
  const text = clip(scrubbedText, MAX_BODY_CHARS);
  parts.push(text !== "" ? `Message:\n${text}` : "Message: (no plain-text body)");
  return { system: systemTemplate("email"), user: parts.join("\n"), schema };
}

/**
 * The prompt hash logged on every judgment (§5.4 measure 4).
 *
 * It hashes the **template** (placeholders unrendered) and the **schema**, and never the item or
 * any per-account value. Reproducibility is fully served: `model` + `prompt_version` +
 * `grammar_version` + `prompt_hash` names exactly which prompt shape produced a row, and the item
 * can be rebuilt from the note. Hashing the rendered prompt instead would put a commitment to a
 * note's body in a table §5.6 says may never hold one — and would give two accounts with different
 * planner slices two different hashes for one prompt.
 */
export async function promptHash(kind: "task" | "event" | "email"): Promise<string> {
  const bytes = new TextEncoder().encode(systemTemplate(kind) + "\u0000" + JSON.stringify(schemaFor(kind)));
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

// ---- Email (plan T3c): the `mail-1` prompt and grammar (spec §4.7 step 4, §5.1 as amended 2026-10-02)
// and the `mail_resolve` prompt (D12). Their own functions, so the three kinds above keep their bytes and
// their pinned hashes. `mail_digest` (plan T16, optional) has no prompt until it is built.

/** §4.7 step 1: the text a `mail_claim` call may carry, in characters. */
export const MAIL_TEXT_CHARS = 2000;
export type MailPromptKind = Exclude<MailKind, "mail_digest">;

const MAIL_ITEM_SCHEMA = {
  type: "object",
  properties: {
    title: { type: "string" },
    due: { type: ["string", "null"] },
    item_kind: { type: "string", enum: ["task", "exam", "event"] },
    effort_hours: { type: "number" },
    importance: { type: "integer" },
  },
  required: ["title", "due", "item_kind", "effort_hours", "importance"],
  additionalProperties: false,
};

// PQ4: `receipt` is a claim kind only a template emits, so the grammar never offers it, and
// `validateMailClaims` refuses one whose source is the model. The sets the prose names (field, topic)
// are re-checked there; a value off its set is a refused claim, never a clamped one.
const MAIL_CLAIM_SCHEMA = {
  type: "object",
  properties: {
    claims: {
      type: "array",
      items: {
        type: "object",
        properties: {
          kind: { type: "string", enum: ["new_item", "change", "course_info", "noise"] },
          course: { type: ["string", "null"] },
          target: { type: ["string", "null"] },
          field: { type: ["string", "null"] },
          value: { type: ["string", "null"] },
          item: { anyOf: [MAIL_ITEM_SCHEMA, { type: "null" }] },
          topic: { type: ["string", "null"] },
          evidence: { type: "string" },
          why: { type: "string" },
          confidence: { type: "number" },
        },
        required: [
          "kind",
          "course",
          "target",
          "field",
          "value",
          "item",
          "topic",
          "evidence",
          "why",
          "confidence",
        ],
        additionalProperties: false,
      },
    },
  },
  required: ["claims"],
  additionalProperties: false,
};

/** `validateMailResolve`'s reply shape: one answer per claim key, a candidate's id or null. */
const MAIL_RESOLVE_SCHEMA = {
  type: "object",
  properties: {
    answers: {
      type: "array",
      items: {
        type: "object",
        properties: { claim: { type: "string" }, target_id: { type: ["string", "null"] } },
        required: ["claim", "target_id"],
        additionalProperties: false,
      },
    },
  },
  required: ["answers"],
  additionalProperties: false,
};

export function mailSchemaFor(kind: MailPromptKind): Record<string, unknown> {
  return kind === "mail_claim" ? MAIL_CLAIM_SCHEMA : MAIL_RESOLVE_SCHEMA;
}

export function mailSystemTemplate(kind: MailPromptKind): string {
  if (kind === "mail_resolve") {
    return [
      "You match claims from a university student's email to the items they already have in one course.",
      "Rules:",
      "- Each claim names an item the way the email did; each candidate is an item the student already has.",
      "- For each claim, answer the id of the one candidate it means, or null when none fits or more than one could.",
      "- Answer every claim once, by its claim key, and only with ids from the candidates given.",
    ].join("\n");
  }
  return [
    "You read one email for a university student and list what it says about their coursework, as claims.",
    "Kinds:",
    "- new_item: the email sets new work, an exam, or an event the student is expected at.",
    "- change: the email moves the deadline of an item it names, or cancels that item.",
    "- course_info: the email gives a course's schedule, room, instructor, office hours, policy or other standing information.",
    "- noise: everything else, including newsletters, marketing, payments, a confirmation that the student submitted work, and a grade being posted.",
    "Rules:",
    "- One claim per fact, at most five. An email that says nothing about the student's coursework is one noise claim.",
    "- course: one of the known course codes given below, or null.",
    "- target: for change only, the item exactly as the email names it; else null.",
    "- field: for change only, due when the deadline moved or cancelled when the item is cancelled; else null.",
    "- value: for a due change, the new deadline; for cancelled, the string true; else null.",
    "- item: for new_item only, else null. title: one line, under 200 characters. due: the deadline, or null. item_kind: task, exam or event. effort_hours: the realistic hours to finish it, 0.25 to 40. importance: a whole number 1 to 5.",
    '- Every deadline exactly as the email states it. Give the phrase as written ("Friday", "next week", "tomorrow at 5pm") -- do NOT compute or resolve it yourself, that happens after you answer. Only when the email itself gives an explicit calendar date (for example "October 3" or "10/3") may you answer that date, as YYYY-MM-DD or YYYY-MM-DDTHH:MM.',
    "- topic: for course_info only, one of schedule, room, instructor, office_hours, policy or other; else null.",
    "- evidence: the words in the subject or message that show the claim, copied exactly, one line, at most 140 characters.",
    "- with no message body, judge from the subject and sender alone and answer a confidence at or below 0.5.",
    "- why: one line, under 140 characters, no line breaks, no double quotes.",
    "- confidence: 0 to 1.",
  ].join("\n");
}

/**
 * The text a `mail_claim` call carries and its claims' evidence is checked against (§4.7 step 1):
 * scrubbed, then cut to `MAIL_TEXT_CHARS`. Scrub before the cut, as the email prompt does, so a token is
 * never cut in half and left partly on the wire. The caller passes this same string to
 * `validateMailClaims` as the text the model saw.
 */
export function mailPromptText(text: string): string {
  return [...scrubForPrompt(text.trim())].slice(0, MAIL_TEXT_CHARS).join("");
}

/**
 * `mail_claim`'s prompt: the subject (scrubbed), the sender, the Date header, the account's known course
 * codes, the time zone and `mailPromptText` of the text, and nothing else the caller's item holds. From is
 * not scrubbed, as on the email path: the sender is what a vendor's mail is judged by, and §7 names it.
 */
export function buildMailClaimPrompt(item: Record<string, unknown>, seed: Record<string, unknown>): Prompt {
  const parts = [
    `Subject: ${oneLine(scrubForPrompt(str(item.subject)), 200)}`,
    `From: ${oneLine(str(item.from), 200)}`,
    `Date: ${oneLine(str(item.date), 80)}`,
  ];
  const known = list(seed.known_courses).map((c) => oneLine(c, 40)).filter((c) => c !== "");
  if (known.length > 0) parts.push(`Known course codes: ${known.join(", ")}`);
  const zone = oneLine(str(seed.time_zone), 64);
  if (zone !== "") parts.push(`Time zone: ${zone}`);
  const raw = scrubForPrompt(str(item.text).trim());
  const text = mailPromptText(str(item.text));
  if (text === "") parts.push("Message: (no plain-text body)");
  else parts.push(`Message:\n${text}${[...raw].length > MAIL_TEXT_CHARS ? " …[truncated]" : ""}`);
  return { system: mailSystemTemplate("mail_claim"), user: parts.join("\n"), schema: MAIL_CLAIM_SCHEMA };
}

function records(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value)
    ? value.filter((v): v is Record<string, unknown> =>
      v !== null && typeof v === "object" && !Array.isArray(v)
    )
    : [];
}

/** A due in the wire shape, else `none`: the resolve prompt carries dates, never a phrase from a note. */
function wireDue(value: unknown): string {
  return typeof value === "string" && ABSOLUTE_DUE_RE.test(value) ? value : "none";
}

/**
 * `mail_resolve`'s prompt for one course batch (D12): each claim's key, the item as the mail named it and
 * its due, and each candidate's note id, title and due. Nothing else: no evidence, sender, body or course.
 */
export function buildMailResolvePrompt(batch: Record<string, unknown>): Prompt {
  const lines = ["Claims:"];
  for (const c of records(batch.claims)) {
    const key = oneLine(str(c.key), 80);
    if (key !== "") {
      lines.push(`- claim ${key} | names: ${oneLine(str(c.target), 200)} | due: ${wireDue(c.due)}`);
    }
  }
  lines.push("Candidates:");
  for (const c of records(batch.candidates)) {
    const id = oneLine(str(c.id), 80);
    if (id !== "") lines.push(`- id ${id} | title: ${oneLine(str(c.title), 200)} | due: ${wireDue(c.due)}`);
  }
  return { system: mailSystemTemplate("mail_resolve"), user: lines.join("\n"), schema: MAIL_RESOLVE_SCHEMA };
}

/** `promptHash` for a mail kind: the template and the schema, never a message or a per-account value. */
export async function mailPromptHash(kind: MailPromptKind): Promise<string> {
  const bytes = new TextEncoder().encode(
    mailSystemTemplate(kind) + "\u0000" + JSON.stringify(mailSchemaFor(kind)),
  );
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
