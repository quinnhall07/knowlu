/**
 * `POST /telemetry` — spec §6, classes (a) and (b), collected under the terms.
 *
 * The device has already filtered this; refusing again here is not distrust of the device, it is the
 * only place the rule holds for a client somebody else wrote. Three refusals, all 400:
 *   * an action outside `engine/src/uievents.rs`'s eleven;
 *   * anything that is not a token where an id or a view name belongs (that is where a title would
 *     hide);
 *   * a correction on a field that is neither closed-vocabulary nor explicitly flagged.
 *
 * And one silent narrowing that is deliberate: a correction on a **content** field keeps its row and
 * loses its values. "The course was corrected" is the signal the eval suite needs; "from MATH 125 to
 * SPAN 101" is somebody's timetable.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";

/** `engine/src/uievents.rs`'s `ACTIONS`, in its order. `app/tests/telemetry.rs` pins the two lists
 * against each other, so a new action added to the engine fails a Rust test until it lands here. */
export const ACTIONS: readonly string[] = [
  "view_opened",
  "object_seen",
  "edit_started",
  "edit_committed",
  "edit_cancelled",
  "decision_made",
  "decision_deferred",
  "issue_opened",
  "sync_run",
  "delta_expanded",
  "why_expanded",
];

/** Judged fields whose correction may carry its values: numbers and closed vocabularies only. */
export const VALUED_FIELDS: readonly string[] = [
  "effort_hours",
  "importance",
  "domain",
  "effort_confidence",
  "status",
];
/** Judged fields whose correction is recorded as "it changed" and nothing more: a course is content. */
export const FLAGGED_FIELDS: readonly string[] = ["course"];

const MAX_ROWS = 500;

/**
 * `uievents::is_token`'s character class: alphanumeric, `_-:`, at most 64 characters, never empty.
 *
 * **Looser than the emitter for `object_id`, deliberately.** The engine checks `object_id` against
 * `ids::is_id` (`<prefix>_<hex>`), which is stricter; this end checks the token class, because the
 * property that matters here is "this is not free text" and a stricter check on a field whose format
 * the engine may extend would refuse rows for a reason that is not about privacy. `session` and
 * `view` are exactly the engine's rule.
 */
export function isToken(s: unknown): boolean {
  return typeof s === "string" && s.length > 0 && s.length <= 64 && /^[A-Za-z0-9_:-]+$/.test(s);
}

function isTimestamp(s: unknown): boolean {
  return typeof s === "string" && Number.isFinite(Date.parse(s));
}

export interface EventIn {
  ts: string;
  session: string;
  view: string;
  action: string;
  object_id?: string | null;
  object_kind?: string | null;
  ms?: number | null;
}

export interface CorrectionIn {
  ts: string;
  item_id: string;
  field: string;
  ours?: string | null;
  theirs?: string | null;
  kind: string;
  /** R-X-3: the judged item as sent, for C2's eval suite. Accepted **only** when the batch also
   * carries `opt_in_raw: true` — the class-(c) opt-in, which C1 does not build — and dropped
   * silently otherwise, so a client that sends it by mistake cannot make it content we hold. */
  request?: Record<string, unknown> | null;
}

export interface Deps {
  verify: VerifyToken;
  saveEvents: (rows: unknown[]) => Promise<void>;
  saveCorrections: (rows: unknown[]) => Promise<void>;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const body = await readJson<{ events?: EventIn[]; corrections?: CorrectionIn[]; opt_in_raw?: boolean }>(
    req,
  );
  const events = body.events ?? [];
  const corrections = body.corrections ?? [];
  // R-X-3. C1 ships no way to turn this on, so it is always false in practice; the field exists so
  // the rule is written down in the one place a row is created rather than promised in a document.
  const optInRaw = body.opt_in_raw === true;
  if (events.length > MAX_ROWS || corrections.length > MAX_ROWS) {
    throw fail(400, `at most ${MAX_ROWS} rows of each kind per batch`);
  }

  const eventRows = events.map((e) => {
    if (!ACTIONS.includes(e.action)) throw fail(400, `unknown action ${JSON.stringify(e.action)}`);
    if (!isTimestamp(e.ts)) throw fail(400, "an event has no usable ts");
    if (!isToken(e.session) || !isToken(e.view)) throw fail(400, "session and view must be tokens, not text");
    if (e.object_id != null && !isToken(e.object_id)) throw fail(400, "object_id must be an id, not text");
    if (e.object_kind != null && !isToken(e.object_kind)) {
      throw fail(400, "object_kind must be a token, not text");
    }
    return {
      account_id: user.id,
      ts: e.ts,
      session: e.session,
      view: e.view,
      action: e.action,
      object_id: e.object_id ?? "",
      object_kind: e.object_kind ?? null,
      ms: typeof e.ms === "number" ? Math.trunc(e.ms) : null,
    };
  });

  const correctionRows = corrections.map((c) => {
    if (!isTimestamp(c.ts)) throw fail(400, "a correction has no usable ts");
    if (!isToken(c.item_id) || !isToken(c.kind)) throw fail(400, "item_id and kind must be tokens, not text");
    const valued = VALUED_FIELDS.includes(c.field);
    const flagged = FLAGGED_FIELDS.includes(c.field);
    if (!valued && !flagged) throw fail(400, `field ${JSON.stringify(c.field)} is not a judged field`);
    return {
      account_id: user.id,
      ts: c.ts,
      item_id: c.item_id,
      field: c.field,
      ours: valued ? (c.ours ?? null) : null,
      theirs: valued ? (c.theirs ?? null) : null,
      kind: c.kind,
      request: optInRaw ? (c.request ?? null) : null,
    };
  });

  if (eventRows.length) await deps.saveEvents(eventRows);
  if (correctionRows.length) await deps.saveCorrections(correctionRows);
  return json(200, { events: eventRows.length, corrections: correctionRows.length });
}
