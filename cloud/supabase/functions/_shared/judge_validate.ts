// Validation is the one place a bad answer is rejected (cloud design §5.2, §5.4 measure 3): the
// numbers are CLAMPED rather than refused — a 900-hour estimate is a bad answer, not a broken
// one — and the confidence floor is the single gate. This is the server-side twin of
// `judge::parse_reply` and `judge::judge_task`'s known-course rule, and it must not drift from
// them: the device applies the same clamps to whatever comes back.
//
// `judge_due.ts` imports `ABSOLUTE_DUE_RE` from here and the mail region calls its `resolveDue`: a cycle
// that is safe because neither module touches the other's exports while it loads, only inside functions.
import { resolveDue } from "./judge_due.ts";
import type { MailClaim, Topic } from "./mail_templates.ts";

export const CONFIDENCE_FLOOR = 0.6;
export const MAX_REASON_CHARS = 140;

export type Kind = "task" | "event" | "email";

/** The wire shape of a resolved `due`: `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`. Shared with
 * `judge_due.ts` (stream J Task T4) so the resolver's absolute-date passthrough and this
 * function's final check can never drift apart — one pattern, not two copies of it. */
export const ABSOLUTE_DUE_RE = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?$/;
/** Why an answer did not become a verdict. A CLOSED set — `judgments.cause` checks it. */
export type Cause = "below floor" | "incomplete" | "model failed" | "refused" | "truncated";

export interface Validated {
  ok: boolean;
  verdict?: Record<string, unknown>;
  cause?: Cause;
}

/** The four words `eventledger::VALID_VERDICTS` will accept (CHECKPOINT J-1, ruled 2026-09-22:
 * `unsure` added, additive — a vault contract, never a rename). */
export const EVENT_VERDICTS = ["obligation", "opportunity", "drop", "unsure"] as const;
/** The five email tiers of cloud design §5.3, and stream J Task T9's sixth: `completion`, "this
 * email confirms the student already submitted or finished a specific piece of work". Its `title`
 * is the work's name, which the device matches against its own task titles. */
export const EMAIL_TIERS = ["task", "borderline", "event", "opportunity", "information", "completion"] as const;

// Added for `cloud/eval/schema.ts` (C2 Task 13): this file had no item-field or labelled-field
// lists before — nothing here validated `request.item`'s shape, only the model's answer — so
// these are new, additive exports and change no existing behaviour; `validate()` above is
// untouched. Each list is transcribed from the one place the device actually builds a request body
// (`engine/src/cloudmodel.rs`'s `task_request` / `event_request` / `email_request`), not guessed
// from the prompt text, so a field the device stops sending — or a label the pipeline stops
// scoring — is one file to update, not two.
/** `request.item`'s keys, by kind — the exact shape `cloudmodel.rs` sends today. */
export const TASK_ITEM_FIELDS = ["id", "title", "body", "source_uid", "created_by", "course", "due"] as const;
export const EVENT_ITEM_FIELDS = [
  "uid",
  "title",
  "start",
  "end",
  "source",
  "organizer",
  "location",
  "url",
  "description",
  "categories",
  "audiences",
  "series_uid",
] as const;
export const EMAIL_ITEM_FIELDS = ["message_id", "subject", "from", "date", "text"] as const;

/**
 * `theirs`'s keys, by kind — the fields a human's correction can label. Task and event are ruling
 * R-C2-E10's shapes verbatim; R-C2-E10 does not fix an email shape, so `EMAIL_LABELLED_FIELDS`
 * below is instead derived from `validate()`'s own email verdict fields, minus the two that are
 * never a label a correction would carry (`why`, a rationale string; `confidence`, the model's own
 * certainty, not a human's).
 */
export const TASK_LABELLED_FIELDS = ["effort_hours", "importance", "course"] as const;
export const EVENT_LABELLED_FIELDS = ["verdict"] as const;
export const EMAIL_LABELLED_FIELDS = [
  "tier",
  "title",
  "course",
  "due",
  "effort_hours",
  "importance",
] as const;

function num(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function clamp(value: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, value));
}

/**
 * One line, clipped by CHARACTERS, and safe for both writers that consume it: `write`'s
 * single-line frontmatter surgery and `eventledger`'s middle-dot-separated ledger line. A double
 * quote becomes an apostrophe and the separator becomes a dash, because `eventledger::why_problem`
 * refuses both outright and a refused why is a judgment thrown away.
 */
export function oneLine(text: string, max: number): string {
  const collapsed = text
    .replace(/["\u2028\u2029]/g, "'")
    .replace(/\s+/g, " ")
    .replace(/ \u00b7 /g, " - ")
    .trim();
  return [...collapsed].slice(0, max).join("");
}

export function validate(
  kind: Kind,
  answer: Record<string, unknown>,
  seed: Record<string, unknown>,
): Validated {
  const confidence = num(answer.confidence);
  if (confidence === null) return { ok: false, cause: "incomplete" };

  if (kind === "task") {
    const effort = num(answer.effort_hours);
    const importance = num(answer.importance);
    const reason = oneLine(
      typeof answer.importance_reason === "string" ? answer.importance_reason : "",
      MAX_REASON_CHARS,
    );
    if (effort === null || importance === null || reason === "") return { ok: false, cause: "incomplete" };
    if (confidence < CONFIDENCE_FLOOR) return { ok: false, cause: "below floor" };
    const known = Array.isArray(seed.known_courses) ? seed.known_courses as string[] : [];
    const raw = typeof answer.course === "string" ? answer.course.trim() : "";
    return {
      ok: true,
      verdict: {
        course: raw !== "" && known.includes(raw) ? raw : null,
        effort_hours: clamp(effort, 0.25, 40),
        importance: clamp(Math.round(importance), 1, 5),
        importance_reason: reason,
        confidence: clamp(confidence, 0, 1),
      },
    };
  }

  if (kind === "event") {
    const verdict = typeof answer.verdict === "string" ? answer.verdict : "";
    const why = oneLine(typeof answer.why === "string" ? answer.why : "", MAX_REASON_CHARS);
    if (!(EVENT_VERDICTS as readonly string[]).includes(verdict) || why === "") {
      return { ok: false, cause: "incomplete" };
    }
    // `unsure` IS the model's honest answer to "I can't tell from this text" — gating it behind
    // the SAME confidence floor that forces a guess among obligation/opportunity/drop in the first
    // place would just reintroduce defect B for the one answer meant to close it: the floor exists
    // to keep a shaky obligation/opportunity/drop guess out of the ledger, not to punish an event
    // that honestly declined to guess. Every other verdict still clears the floor as before.
    if (verdict !== "unsure" && confidence < CONFIDENCE_FLOOR) {
      return { ok: false, cause: "below floor" };
    }
    return { ok: true, verdict: { verdict, why, confidence: clamp(confidence, 0, 1) } };
  }

  const tier = typeof answer.tier === "string" ? answer.tier : "";
  const why = oneLine(typeof answer.why === "string" ? answer.why : "", MAX_REASON_CHARS);
  if (!(EMAIL_TIERS as readonly string[]).includes(tier) || why === "") {
    return { ok: false, cause: "incomplete" };
  }
  const title = oneLine(typeof answer.title === "string" ? answer.title : "", 200);
  // T9: a completion names the work it completes, or the device has nothing to match — and a
  // promoted rule never carries a title (`fieldsOf` drops it), so this is also what sends a
  // title-less rule answer on to the model rather than queueing an unmatchable completion.
  if (tier === "completion" && title === "") return { ok: false, cause: "incomplete" };
  if (confidence < CONFIDENCE_FLOOR) return { ok: false, cause: "below floor" };
  const known = Array.isArray(seed.known_courses) ? seed.known_courses as string[] : [];
  const course = typeof answer.course === "string" && known.includes(answer.course) ? answer.course : null;
  const effort = num(answer.effort_hours);
  const importance = num(answer.importance);
  return {
    ok: true,
    verdict: {
      tier,
      why,
      title,
      course,
      // T4: the model no longer resolves a relative phrase itself, so by the time an answer
      // reaches this function its `due` is either already the wire shape (`judge_due.ts` resolved
      // it, or the email stated an absolute date the model copied verbatim) or something that
      // failed to resolve — and an unresolved phrase is exactly as untrustworthy here as it always
      // was, so it is still dropped to `null` rather than written into a vault.
      due: typeof answer.due === "string" && ABSOLUTE_DUE_RE.test(answer.due) ? answer.due : null,
      effort_hours: effort === null ? null : clamp(effort, 0.25, 40),
      importance: importance === null ? null : clamp(Math.round(importance), 1, 5),
      confidence: clamp(confidence, 0, 1),
    },
  };
}

// ---- Email (plan T3b): mail claims and resolve answers (spec §4.7 step 4, §5.1 as amended 2026-10-02,
// D9, D12; §10 R11). Unlike `validate()`, nothing here is clamped into range but the confidence: a claim
// is data the device acts on without re-judging it, so a value off its set is a refused claim.

/** The judgment kinds email adds (plan P6). Their own type, so no `Record<Kind, …>` changes. A `receipt`
 * is a claim kind, never a judgment kind, so it is not here. */
export const MAIL_KINDS = ["mail_claim", "mail_resolve", "mail_digest"] as const;
export type MailKind = (typeof MAIL_KINDS)[number];

/** D9: up to five claims a message; a sixth and later are dropped unread. */
export const MAX_MAIL_CLAIMS = 5;
export const MAIL_EVIDENCE_CHARS = 140;
const MAIL_TITLE_CHARS = 200;
/** §5.1's fixed `course_info` topics. */
export const MAIL_TOPICS = [
  "schedule",
  "room",
  "instructor",
  "office_hours",
  "policy",
  "other",
] as const satisfies readonly Topic[];
const ITEM_KINDS = ["task", "exam", "event"] as const;

/** What a claim is checked against: the message as the model (or a template) read it. */
export interface MailValidateContext {
  /** The From header as parsed, display name and address. A line break in it refuses the whole reply. */
  from: string;
  /** The subject and the text the model saw (§4.7 step 1: cut and scrubbed), so the evidence is checked
   * against what was read. Evidence must lie inside one of the two, never across them. */
  subject: string;
  text: string;
  /** The Date header: `judge_due.ts`'s reference. */
  date: string;
  knownCourses: readonly string[];
  timeZone?: string | null;
}

export interface MailValidated {
  /** True when at least one claim was kept. */
  ok: boolean;
  claims: MailClaim[];
  /** Each refused claim's cause, in reply order. */
  refused: Cause[];
  /** When nothing was kept: the first refusal's cause, else `incomplete`. */
  cause?: Cause;
}

/** §10 R11: a C0 control other than tab, DEL, U+0085, U+2028 or U+2029. Each is a line boundary for the
 * device's `pystr::splitlines`, or a control no claim needs; a value carrying one is refused, never cleaned. */
function breaksLine(text: string): boolean {
  for (const ch of text) {
    const c = ch.codePointAt(0) ?? 0;
    if ((c < 0x20 && c !== 0x09) || c === 0x7f || c === 0x85 || c === 0x2028 || c === 0x2029) return true;
  }
  return false;
}

function fold(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

/** `due` when it is in the wire shape AND names a real day and time, else null. `resolveDue` passes a
 * shaped due through verbatim, so `2026-09-31` or `T24:00` stops here, as the device's `due_date` reads it
 * (the engine lane's T5.3 finding): the server and the device agree on which dues exist. */
function realDue(due: string | null): string | null {
  if (due === null || !ABSOLUTE_DUE_RE.test(due)) return null;
  const [year, month, day] = [due.slice(0, 4), due.slice(5, 7), due.slice(8, 10)].map(Number);
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1];
  if (days === undefined || day < 1 || day > days) return null;
  if (due.length > 10 && (Number(due.slice(11, 13)) > 23 || Number(due.slice(14, 16)) > 59)) return null;
  return due;
}

/** A claim's due: through the resolver against the Date header, then held to a real calendar day. */
function mailDue(raw: unknown, ctx: MailValidateContext): string | null {
  return typeof raw === "string" ? realDue(resolveDue(raw, ctx.date, ctx.timeZone)) : null;
}

function present(value: unknown): boolean {
  return value !== null && value !== undefined;
}

type Checked = { claim: MailClaim } | { cause: Cause };
const INCOMPLETE: Checked = { cause: "incomplete" };

/** One claim against §5.1: its shape first (`incomplete`), the 0.6 floor last, as `validate()` orders them. */
function mailClaim(
  raw: unknown,
  ctx: MailValidateContext,
  source: "model" | "template",
  folded: { subject: string; text: string },
): Checked {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) return INCOMPLETE;
  const a = raw as Record<string, unknown>;
  const kind = a.kind;
  if (
    kind !== "new_item" && kind !== "change" && kind !== "course_info" && kind !== "noise" &&
    kind !== "receipt"
  ) {
    return INCOMPLETE;
  }
  // PQ4: a receipt comes only from a template. The source is the caller's word, never the answer's.
  if (kind === "receipt" && source !== "template") return INCOMPLETE;
  const confidence = num(a.confidence);
  if (confidence === null || typeof a.why !== "string" || typeof a.evidence !== "string") return INCOMPLETE;
  const target = typeof a.target === "string" ? a.target : "";
  if ([a.why, a.evidence, target].some(breaksLine)) return INCOMPLETE;
  // D9: the evidence occurs verbatim in the message after whitespace folding, or the claim is refused.
  const evidence = fold(a.evidence);
  if (evidence === "" || !(folded.text.includes(evidence) || folded.subject.includes(evidence))) {
    return INCOMPLETE;
  }
  const why = oneLine(a.why, MAX_REASON_CHARS);
  if (why === "") return INCOMPLETE;
  const course = typeof a.course === "string" && ctx.knownCourses.includes(a.course.trim())
    ? a.course.trim()
    : null;
  const base: MailClaim = {
    kind,
    course,
    target: null,
    field: null,
    value: null,
    item: null,
    topic: null,
    evidence: [...evidence].slice(0, MAIL_EVIDENCE_CHARS).join("").trim(),
    why,
    confidence: clamp(confidence, 0, 1),
    source,
    resolution: null,
  };
  const named = oneLine(target, MAIL_TITLE_CHARS);
  let claim: MailClaim;
  if (kind === "change") {
    if (named === "") return INCOMPLETE;
    if (a.field === "due") claim = { ...base, target: named, field: "due", value: mailDue(a.value, ctx) };
    else if (a.field === "cancelled" && a.value === "true") {
      claim = { ...base, target: named, field: "cancelled", value: "true" };
    } else return INCOMPLETE;
  } else if (kind === "receipt") {
    if (named === "" || [a.field, a.value, a.item, a.topic].some(present)) return INCOMPLETE;
    claim = { ...base, target: named };
  } else if (kind === "course_info") {
    const topic = MAIL_TOPICS.find((t) => t === a.topic);
    if (topic === undefined) return INCOMPLETE;
    claim = { ...base, topic };
  } else if (kind === "new_item") {
    const it = a.item;
    if (it === null || typeof it !== "object" || Array.isArray(it)) return INCOMPLETE;
    const i = it as Record<string, unknown>;
    if (typeof i.title !== "string" || breaksLine(i.title)) return INCOMPLETE;
    const title = oneLine(i.title, MAIL_TITLE_CHARS);
    const itemKind = ITEM_KINDS.find((k) => k === i.item_kind);
    const effort = num(i.effort_hours);
    const importance = num(i.importance);
    if (title === "" || itemKind === undefined || effort === null || effort < 0.25 || effort > 40) {
      return INCOMPLETE;
    }
    if (importance === null || !Number.isInteger(importance) || importance < 1 || importance > 5) {
      return INCOMPLETE;
    }
    const due = mailDue(i.due, ctx);
    claim = { ...base, item: { title, due, item_kind: itemKind, effort_hours: effort, importance } };
  } else claim = base;
  if (confidence < CONFIDENCE_FLOOR) return { cause: "below floor" };
  return { claim };
}

/**
 * A `mail_claim` reply (or a template's claims, `source: "template"`) to §5.1's claims. A refused claim is
 * no claim and the rest stand; a From that breaks a line refuses the whole reply. An unknown course and an
 * unresolvable due read null; a sixth claim is dropped.
 */
export function validateMailClaims(
  answer: Record<string, unknown>,
  ctx: MailValidateContext,
  source: "model" | "template",
): MailValidated {
  if (breaksLine(ctx.from)) return { ok: false, claims: [], refused: [], cause: "incomplete" };
  const raw = Array.isArray(answer.claims) ? answer.claims.slice(0, MAX_MAIL_CLAIMS) : [];
  const folded = { subject: fold(ctx.subject), text: fold(ctx.text) };
  const claims: MailClaim[] = [];
  const refused: Cause[] = [];
  for (const one of raw) {
    const checked = mailClaim(one, ctx, source, folded);
    if ("claim" in checked) claims.push(checked.claim);
    else refused.push(checked.cause);
  }
  if (claims.length > 0) return { ok: true, claims, refused };
  return { ok: false, claims, refused, cause: refused[0] ?? "incomplete" };
}

/** One `mail-resolve` batch (D12): its claims' keys and its candidates' note ids, as sent. */
export interface ResolveBatch {
  claims: readonly string[];
  candidates: readonly string[];
}

/**
 * A `mail_resolve` reply, `{answers: [{claim, target_id}]}`, to one resolution per batch claim, in batch
 * order. An id outside the batch's candidates, an unanswered claim, or a claim answered two different ways
 * reads none (`target_id: null`), never a guess; an answer for a claim outside the batch is ignored.
 */
export function validateMailResolve(
  answer: Record<string, unknown>,
  batch: ResolveBatch,
): Array<{ target_id: string | null }> {
  const said = new Map<string, Set<string | null>>();
  for (const one of Array.isArray(answer.answers) ? answer.answers : []) {
    if (one === null || typeof one !== "object") continue;
    const { claim, target_id } = one as Record<string, unknown>;
    if (typeof claim !== "string" || !batch.claims.includes(claim)) continue;
    const id = typeof target_id === "string" && batch.candidates.includes(target_id) ? target_id : null;
    said.set(claim, (said.get(claim) ?? new Set<string | null>()).add(id));
  }
  return batch.claims.map((claim) => {
    const ids = [...(said.get(claim) ?? [])];
    return { target_id: ids.length === 1 ? ids[0] : null };
  });
}
