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
import { oneLine } from "./judge_validate.ts";
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
// lets the model say so honestly, and `judge_validate.ts`'s `record_verdict` path writes it like
// any other verdict, so the uid is never re-asked (events spec §7's one-verdict-per-uid-forever
// rule already does the rest).
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
    return [
      "You estimate effort and importance for one university assignment.",
      "Rules:",
      "- effort_hours: the realistic time in hours to finish this one assignment, 0.25 to 40.",
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
