// Validation is the one place a bad answer is rejected (cloud design §5.2, §5.4 measure 3): the
// numbers are CLAMPED rather than refused — a 900-hour estimate is a bad answer, not a broken
// one — and the confidence floor is the single gate. This is the server-side twin of
// `judge::parse_reply` and `judge::judge_task`'s known-course rule, and it must not drift from
// them: the device applies the same clamps to whatever comes back.
export const CONFIDENCE_FLOOR = 0.6;
export const MAX_REASON_CHARS = 140;

export type Kind = "task" | "event" | "email";
/** Why an answer did not become a verdict. A CLOSED set — `judgments.cause` checks it. */
export type Cause = "below floor" | "incomplete" | "model failed" | "refused" | "truncated";

export interface Validated {
  ok: boolean;
  verdict?: Record<string, unknown>;
  cause?: Cause;
}

/** The three words `eventledger::VALID_VERDICTS` will accept. */
export const EVENT_VERDICTS = ["obligation", "opportunity", "drop"] as const;
/** The five email tiers of cloud design §5.3. */
export const EMAIL_TIERS = ["task", "borderline", "event", "opportunity", "information"] as const;

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
      title: oneLine(typeof answer.title === "string" ? answer.title : "", 200),
      course,
      due: typeof answer.due === "string" && /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?$/.test(answer.due)
        ? answer.due
        : null,
      effort_hours: effort === null ? null : clamp(effort, 0.25, 40),
      importance: importance === null ? null : clamp(Math.round(importance), 1, 5),
      confidence: clamp(confidence, 0, 1),
    },
  };
}
