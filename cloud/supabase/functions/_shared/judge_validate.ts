// Validation is the one place a bad answer is rejected (cloud design §5.2, §5.4 measure 3): the
// numbers are CLAMPED rather than refused — a 900-hour estimate is a bad answer, not a broken
// one — and the confidence floor is the single gate. This is the server-side twin of
// `judge::parse_reply` and `judge::judge_task`'s known-course rule, and it must not drift from
// them: the device applies the same clamps to whatever comes back.
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

/** The three words `eventledger::VALID_VERDICTS` will accept. */
export const EVENT_VERDICTS = ["obligation", "opportunity", "drop"] as const;
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
    if (confidence < CONFIDENCE_FLOOR) return { ok: false, cause: "below floor" };
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
