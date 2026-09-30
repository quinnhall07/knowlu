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
import { EVENT_VERDICTS } from "../_shared/judge_validate.ts";

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
/**
 * F7: labels the device reports on a named judgment: an `unsure` event answered on its card
 * (`verdict`), an `amend` card from a judged write rejected (`decision`). Accepted **only** on a row
 * that carries a `judgment_id`, and each has a closed vocabulary, so a sentence has nowhere to hide.
 * A label on an `email` judgment is refused outright (Gmail Limited Use: an email-derived decision
 * never reaches the shared calibration/rules path).
 */
export const LABEL_FIELDS: readonly string[] = ["verdict", "decision"];
/** The judgment kinds a `judgment_kind` may name (`judgments.kind`). */
export const JUDGMENT_KINDS: readonly string[] = ["task", "event", "email"];
const DECISION_KINDS: readonly string[] = ["task", "event"];
const DECISION_OUTCOMES: readonly string[] = ["approved", "rejected"];

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

/** A UUID, the shape of `judgments.id`, and the only thing a `judgment_id` may be. */
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function isUuid(s: unknown): s is string {
  return typeof s === "string" && UUID.test(s);
}

function isTimestamp(s: unknown): boolean {
  return typeof s === "string" && ISO_TIMESTAMP.test(s) && Number.isFinite(Date.parse(s));
}

/** A duration in milliseconds: a non-negative safe integer, at most one day. */
function isMs(n: unknown): boolean {
  return Number.isSafeInteger(n) && (n as number) >= 0 && (n as number) <= 86_400_000;
}

/** A number-like string — what `scalar` on the device turns a JSON number into (`"2.0"`, `"0.5"`,
 * never exponent notation in practice), checked loosely rather than re-parsed as JSON: the device
 * already produced this string, and this is a belt, not the primary guard. */
function isNumberLike(s: unknown): boolean {
  return typeof s === "string" && s.trim() !== "" && Number.isFinite(Number(s));
}

/** Fix round 1 (C1, ruling R-C1-39), the belt to `is_wire_value`'s brace on the device: `domain`,
 * `effort_confidence` and `status` are free-text inputs in the console with no vocabulary check
 * anywhere in the write path, so a `VALUED_FIELDS` correction's `ours`/`theirs` can be a sentence
 * even though the device is supposed to have dropped it already. This mirrors the device's own rule
 * — a numeric literal, or a short closed-vocabulary-shaped token — and never 400s on it: a value that
 * fails both is nulled and the row is kept, the same "it changed" treatment a FLAGGED field already
 * gets below. */
function isWireValue(s: unknown): boolean {
  return isNumberLike(s) || isToken(s);
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
  /** F7: the judgment a `LABEL_FIELDS` row labels: a UUID, and only on a label row. */
  judgment_id?: string | null;
  /** F7: that judgment's kind, one of `JUDGMENT_KINDS`, and never `email` on a label row. */
  judgment_kind?: string | null;
}

export interface Deps {
  verify: VerifyToken;
  saveEvents: (rows: unknown[]) => Promise<void>;
  saveCorrections: (rows: unknown[]) => Promise<void>;
  /** F7: which of `ids` are `judgments` rows of `accountId`, each mapped to its real `judgments.kind`
   * (lowercased id → kind). Called at most once per batch, and only when the batch holds an id-bearing
   * row; a rejection fails the whole batch before anything saves. */
  ownedJudgments: (accountId: string, ids: string[]) => Promise<Map<string, string>>;
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

  const correctionRows = corrections.map((c): CorrectionRow => {
    if (!isTimestamp(c.ts)) throw fail(400, "a correction has no usable ts");
    if (!isToken(c.item_id) || !isToken(c.kind)) throw fail(400, "item_id and kind must be tokens, not text");
    if (LABEL_FIELDS.includes(c.field)) return labelRow(user.id, c);
    const valued = VALUED_FIELDS.includes(c.field);
    const flagged = FLAGGED_FIELDS.includes(c.field);
    if (!valued && !flagged) throw fail(400, `field ${JSON.stringify(c.field)} is not a judged field`);
    // F7 (review M-10): a judgment id rides only on a label row. Task field corrections keep the
    // nightly server backfill, so this shape has no sender, and refusing it keeps the two-call split
    // below exact and keeps a per-user email join from arriving here by accident.
    if (c.judgment_id != null || c.judgment_kind != null) {
      throw fail(400, `field ${JSON.stringify(c.field)} does not take a judgment_id`);
    }
    // Fix round 1 (C1): a value that is neither number-like nor a token is free text and is nulled,
    // never 400ed — the row survives (a correction happened is still the signal), the sentence does
    // not travel.
    const keep = (v: string | null | undefined) => (isWireValue(v) ? v as string : null);
    return {
      account_id: user.id,
      ts: c.ts,
      item_id: c.item_id,
      field: c.field,
      ours: valued ? keep(c.ours) : null,
      theirs: valued ? keep(c.theirs) : null,
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

  // F7 (review I-5, first layer): the device now supplies `judgment_id`, a new trust boundary. One
  // lookup per batch, before anything is saved, so a failed lookup is a 5xx that saves nothing and
  // the device retries the whole batch next slot.
  const plain = dedupedCorrections.filter((r) => r.judgment_id === undefined);
  const labelled = dedupedCorrections.filter((r) => r.judgment_id !== undefined);
  // Fix round 1 (review M-1): the event card exists only for an `unsure` judgment, so a verdict row
  // whose `ours` is anything else, or whose answer is `unsure` again, labels nothing a student
  // corrected; it would mark a judgment wrong that nobody disagreed with. Dropped and counted as
  // `refused`, never 400ed, for the same retry reason as `unowned` below.
  const answerable = labelled.filter((r) =>
    r.field !== "verdict" || (r.ours === "unsure" && r.theirs !== "unsure")
  );
  let owned: CorrectionRow[] = [];
  if (answerable.length) {
    const ids = [...new Set(answerable.map((r) => r.judgment_id as string))];
    const mine = await deps.ownedJudgments(user.id, ids);
    // Dropped and counted as `unowned`, never 400ed: a judgment can legitimately vanish (retention,
    // account deletion), and a 400 would have the device resend the same batch forever.
    // Fix round 1 (review I-1): ownership alone is not enough. The claimed kind must be the
    // judgment's real kind, and a real `email` judgment is never labelled here, whatever the row
    // claims (Gmail Limited Use: `promote_rules` joins on `judgment_id` and reads `j.kind` from the
    // judgment, so a mislabelled row would carry an email-derived decision into the rules path).
    // The saved row carries the server's kind, never the device's.
    owned = answerable.flatMap((r) => {
      const real = mine.get(r.judgment_id as string);
      return real === undefined || real === "email" || real !== r.judgment_kind
        ? []
        : [{ ...r, judgment_kind: real }];
    });
  }

  if (dedupedEvents.length) await deps.saveEvents(dedupedEvents);
  // F7 decision 3: two calls, never one. A `merge-duplicates` upsert sets every column the payload
  // names, so a plain row sharing a payload with a labelled one would name `judgment_id: null` and
  // erase the nightly backfill's id on a re-send. Plain rows never name the column at all.
  if (plain.length) await deps.saveCorrections(plain);
  if (owned.length) await deps.saveCorrections(owned);
  const counts = { events: dedupedEvents.length, corrections: plain.length + owned.length };
  // `unowned` (not the caller's judgment, or not of the kind claimed) and `refused` (a verdict row
  // that answers nothing) appear only when the batch held a labelled row, so a batch from today's
  // app gets exactly the response it always got.
  const refused = labelled.length - answerable.length;
  return json(
    200,
    labelled.length ? { ...counts, unowned: answerable.length - owned.length, refused } : counts,
  );
}

/** A saved `corrections` row. `judgment_id`/`judgment_kind` are present only on a label row: a
 * plain row must not name the columns at all (decision 3). */
interface CorrectionRow {
  account_id: string;
  ts: string;
  item_id: string;
  field: string;
  ours: string | null;
  theirs: string | null;
  kind: string;
  request: null;
  judgment_id?: string;
  judgment_kind?: string;
}

/** F7 decision 2: a `LABEL_FIELDS` row, refused with 400 unless every value is from its closed
 * vocabulary, it names a well-formed judgment, and that judgment is not an `email` one. */
function labelRow(accountId: string, c: CorrectionIn): CorrectionRow {
  if (c.judgment_id == null) throw fail(400, `field ${JSON.stringify(c.field)} needs a judgment_id`);
  if (!isUuid(c.judgment_id)) throw fail(400, "judgment_id must be a UUID, not text");
  const kind = c.judgment_kind;
  if (typeof kind !== "string" || !JUDGMENT_KINDS.includes(kind)) {
    throw fail(400, "judgment_kind must be one of task, event, email");
  }
  // Global Constraint 14 (Gmail Limited Use): an email-derived decision is never reported to the
  // shared calibration/rules path. The device never sends one; this makes a device bug a refused
  // batch rather than a breach.
  if (kind === "email") throw fail(400, "an email judgment's label is not accepted");
  const verdicts = EVENT_VERDICTS as readonly string[];
  if (c.field === "verdict") {
    if (kind !== "event") throw fail(400, "a verdict labels an event judgment only");
    if (!verdicts.includes(c.ours as string) || !verdicts.includes(c.theirs as string)) {
      throw fail(400, "a verdict must be one of the event verdicts");
    }
  } else {
    if (!DECISION_KINDS.includes(kind)) throw fail(400, "a decision labels a task or event judgment only");
    if (c.ours !== "proposed" || !DECISION_OUTCOMES.includes(c.theirs as string)) {
      throw fail(400, "a decision is proposed, then approved or rejected");
    }
  }
  return {
    account_id: accountId,
    ts: c.ts,
    item_id: c.item_id,
    field: c.field,
    ours: c.ours as string,
    theirs: c.theirs as string,
    kind: c.kind,
    request: null,
    judgment_id: c.judgment_id.toLowerCase(),
    judgment_kind: kind,
  };
}
