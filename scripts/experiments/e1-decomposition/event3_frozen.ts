// E1's control arm: the event-3 prompt, FROZEN. Copied verbatim from commit 7c127e2
// ("feat(events): add the unsure verdict word and close the re-ask defect"),
// `cloud/supabase/functions/_shared/judge_prompts.ts` (EVENT_SCHEMA, systemTemplate("event")) and
// `judge_validate.ts` (validate()'s event branch). It lives here, not as a runtime switch in the
// product: the product carries ONE event prompt (the decomposed event-4), and this copy exists only
// so E1 can measure event-4 against what it replaced. `EVENT3_PROMPT_HASH` is the value
// `promptHash("event")` returned at 7c127e2 — `arms_test.ts` recomputes it over this copy, so the
// copy cannot drift from what was committed without a failing test.
//
// The user message is NOT frozen: both arms take it from the live `buildPrompt("event", …)`, which
// T3 does not change, so the two arms differ in exactly the system prompt, the schema and how the
// reply is read — the three things the decomposition is.
import {
  CONFIDENCE_FLOOR,
  EVENT_VERDICTS,
  MAX_REASON_CHARS,
  oneLine,
  type Validated,
} from "../../../cloud/supabase/functions/_shared/judge_validate.ts";

export const EVENT3_PROMPT_HASH = "b6324351b0a70976f818930a3dd2e1e66478c804f4a8ff01d4a9123b674b5e92";

export const EVENT3_SCHEMA = {
  type: "object",
  properties: {
    verdict: { type: "string", enum: ["obligation", "opportunity", "drop", "unsure"] },
    why: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["verdict", "why", "confidence"],
  additionalProperties: false,
};

export const EVENT3_SYSTEM = [
  "You decide whether one campus event is worth a student's attention.",
  "Rules:",
  "- verdict: obligation (the student is expected there), opportunity (worth offering), drop, or unsure.",
  "- unsure: answer unsure when the event's own text does not say whether it applies to this student — do not guess the audience to force obligation, opportunity or drop.",
  "- an event aimed at faculty, staff, alumni or graduate students is a drop.",
  "- a standing exhibit, an office-hours block or a recurring drop-in is a drop.",
  "- why: one line, under 140 characters, no line breaks, no double quotes.",
  "- confidence: 0 to 1.",
].join("\n");

/** `promptHash`'s formula (template + NUL + schema JSON, SHA-256 hex) over the frozen copy. */
export async function event3Hash(): Promise<string> {
  const bytes = new TextEncoder().encode(EVENT3_SYSTEM + "\u0000" + JSON.stringify(EVENT3_SCHEMA));
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** validate()'s event branch as of 7c127e2. */
export function validateEvent3(answer: Record<string, unknown>): Validated {
  const confidence = typeof answer.confidence === "number" && Number.isFinite(answer.confidence)
    ? answer.confidence
    : null;
  if (confidence === null) return { ok: false, cause: "incomplete" };
  const verdict = typeof answer.verdict === "string" ? answer.verdict : "";
  const why = oneLine(typeof answer.why === "string" ? answer.why : "", MAX_REASON_CHARS);
  if (!(EVENT_VERDICTS as readonly string[]).includes(verdict) || why === "") {
    return { ok: false, cause: "incomplete" };
  }
  if (verdict !== "unsure" && confidence < CONFIDENCE_FLOOR) {
    return { ok: false, cause: "below floor" };
  }
  return { ok: true, verdict: { verdict, why, confidence: Math.min(1, Math.max(0, confidence)) } };
}
