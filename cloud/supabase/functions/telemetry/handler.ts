/**
 * `POST /telemetry` — spec §6, classes (a) and (b), collected under the terms.
 *
 * The device has already filtered this; refusing again here is not distrust of the device, it is the
 * only place the rule holds for a client somebody else wrote. Refusals are all 400, chief among them:
 *   * an action outside `engine/src/uievents.rs`'s eleven;
 *   * anything that is not a token where an id or a view name belongs (that is where a title would
 *     hide);
 *   * a timestamp that is not ISO-8601 (loose `Date.parse` is not a free-text check);
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

/** ISO-8601, the one shape `Date.parse` is trusted for — V8 parses plenty of loose, non-ISO
 * strings (including a stray sentence) as a date, and that is exactly the hiding spot this
 * endpoint exists to close. */
const ISO_TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,6})?(Z|[+-]\d{2}:\d{2})$/;

function isTimestamp(s: unknown): boolean {
  return typeof s === "string" && ISO_TIMESTAMP.test(s) && Number.isFinite(Date.parse(s));
}

/** A duration in milliseconds: a non-negative safe integer, at most one day. */
function isMs(n: unknown): boolean {
  return Number.isSafeInteger(n) && (n as number) >= 0 && (n as number) <= 86_400_000;
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
  /** R-X-3: the judged item, for C2's eval suite — under the class-(c) opt-in. C1 builds no
   * opt-in (Task 1's consent kinds are `tos | privacy | age_18 | auto_renew`; there is no row to
   * consult), so this field is accepted but **always ignored**: the column stays for C2 to fill
   * once it adds the server-side lookup, and nothing sent here ever reaches it. */
  request?: Record<string, unknown> | null;
}

export interface Deps {
  verify: VerifyToken;
  saveEvents: (rows: unknown[]) => Promise<void>;
  saveCorrections: (rows: unknown[]) => Promise<void>;
}

/** The last row wins, keyed on exactly the migration's own unique-constraint columns — a batch
 * retried after a partial failure carries the same rows twice, and this is where that becomes one
 * write instead of two before it ever reaches `deps`. */
function dedupeBy<T>(rows: T[], key: (row: T) => string): T[] {
  const byKey = new Map<string, T>();
  for (const row of rows) byKey.set(key(row), row);
  return [...byKey.values()];
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const body = await readJson<{ events?: unknown; corrections?: unknown }>(req);
  if (body.events !== undefined && !Array.isArray(body.events)) {
    throw fail(400, "events must be an array");
  }
  if (body.corrections !== undefined && !Array.isArray(body.corrections)) {
    throw fail(400, "corrections must be an array");
  }
  const events = (body.events ?? []) as EventIn[];
  const corrections = (body.corrections ?? []) as CorrectionIn[];
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
    if (e.ms != null && !isMs(e.ms)) throw fail(400, "ms must be a duration in milliseconds, at most a day");
    return {
      account_id: user.id,
      ts: e.ts,
      session: e.session,
      view: e.view,
      action: e.action,
      object_id: e.object_id ?? "",
      object_kind: e.object_kind ?? null,
      ms: e.ms ?? null,
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
      // R-X-3, and the ruling that closed it: C1 has no opt-in to consult, so this is always null.
      request: null,
    };
  });

  // `telemetry_events_once` and `corrections_once` (20260910000400_telemetry.sql) name these
  // tuples; a batch resent after a dropped connection must land on the same rows within the batch,
  // not just across retries.
  const dedupedEvents = dedupeBy(
    eventRows,
    (r) => JSON.stringify([r.account_id, r.session, r.ts, r.action, r.object_id]),
  );
  const dedupedCorrections = dedupeBy(
    correctionRows,
    (r) => JSON.stringify([r.account_id, r.ts, r.item_id, r.field]),
  );

  if (dedupedEvents.length) await deps.saveEvents(dedupedEvents);
  if (dedupedCorrections.length) await deps.saveCorrections(dedupedCorrections);
  return json(200, { events: dedupedEvents.length, corrections: dedupedCorrections.length });
}
