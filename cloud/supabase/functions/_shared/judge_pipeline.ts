// The pipeline of cloud design §5.2, and the only place the order of those steps is written down:
//
//   rules (tier 2, free) -> breaker -> monthly budget -> daily cap -> constrained call (tier 3)
//                        -> refund if the model never answered | validate -> record tokens
//                        -> log -> reply
//
// **Rules come before the cap on purpose.** "Rules retire model calls" (§5.4 measure 1) and "a
// rule still spends the account's model allowance" cannot both be true; the first one is the
// design, and it is also what makes promotion worth having on a capped account.
//
// Everything the pipeline needs arrives as a parameter, so the whole of it is exercised by
// `deno test` with a scripted model, an in-memory sink and no database at all. The handlers on top
// of it are thin by design: a handler that computed anything would be a second place to fix a bug.
import type { JudgeModel } from "./judge_anthropic.ts";
import { ModelRefused } from "./judge_anthropic.ts";
import { type CapStore, DAILY_CAP, MONTHLY_CEILING_USD } from "./judge_caps.ts";
import { resolveDue } from "./judge_due.ts";
import type { JudgmentRow, JudgmentSink } from "./judge_log.ts";
import type { ModelRow } from "./judge_models.ts";
import { buildPrompt, promptHash } from "./judge_prompts.ts";
import { featureMap, type RuleTable } from "./judge_rules.ts";
import { type Cause, CONFIDENCE_FLOOR, type Kind, validate } from "./judge_validate.ts";

// One import site for a handler and one for a test. Nothing under `_shared/` imports THIS module:
// an ESM cycle whose value export is read at module scope is a temporal-dead-zone crash at deploy
// time, not a compile error.
export { CONFIDENCE_FLOOR, DAILY_CAP, MONTHLY_CEILING_USD };
export type { CapStore, Cause, JudgmentRow, JudgmentSink, Kind, ModelRow, RuleTable };

export interface JudgeRequest {
  kind: Kind;
  item: Record<string, unknown>;
  heuristics_seed: Record<string, unknown>;
  /** What the caller's engine can take beyond the original vocabulary. Today one word matters:
   * `"unsure"` on an event request (final review item 2, see `gateUnsure`). */
  accepts?: readonly string[];
  /** The student's timezone, an IANA name: the vault's own `config/ingest.yaml` `timezone`, sent
   * by the device. Read only by the email `due` resolver, so a relative phrase resolves against
   * the email's local date (`judge_due.ts`'s module doc); absent, the Date header's own offset is
   * the only clock. Never part of the prompt. */
  timezone?: string;
}

export interface JudgeReply {
  verdict: Record<string, unknown> | null;
  tier: 0 | 2 | 3;
  outcome: "answered" | "low confidence" | "capped";
  cause?: Cause;
  model?: string;
  prompt_version?: string;
  grammar_version?: string;
  /** The `judgments` row this reply came from, so a later correction can name it. */
  judgment_id?: string;
}

export interface PipelineDeps {
  row: ModelRow;
  model: JudgeModel;
  rules: RuleTable;
  caps: CapStore;
  log: JudgmentSink;
  origin: JudgmentRow["origin"];
  now: () => number;
}

/**
 * What goes in `judgments.fields`: the verdict's own field values, plus the promotion features.
 *
 * Both halves are keys or values the note already carries in the open — a course slug, an hour
 * figure, an importance, a three-word title prefix that is the note's filename. The free-text
 * fields (`importance_reason`, `why`, `title`) are dropped here, and `a_body_token_reaches_no_
 * judgment_row` is what keeps that true.
 *
 * **The feature map merges LAST** (C2 final review S-6). The two halves share a namespace, and a
 * verdict field could one day be named `source` or `course`-like enough to collide with a feature
 * key. `promote_rules` builds a rule's verdict as `fields` MINUS the feature keys, so a feature
 * the verdict had overwritten would be subtracted as though it were still there — the rule would
 * be promoted on a value that is not the one it was looked up by, and would never fire. Nothing
 * collides today; this is the order that keeps that a property rather than a coincidence.
 */
export function fieldsOf(
  verdict: Record<string, unknown> | null,
  kind: Kind,
  item: Record<string, unknown>,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [name, value] of Object.entries(verdict ?? {})) {
    if (["confidence", "why", "importance_reason", "title"].includes(name)) continue;
    out[name] = value === null ? "null" : String(value);
  }
  return { ...out, ...featureMap(kind, item) };
}

function itemId(item: Record<string, unknown>): string {
  for (const key of ["id", "uid", "message_id"]) {
    const value = item[key];
    if (typeof value === "string" && value !== "") return value;
  }
  return "";
}

export async function judge(
  accountId: string,
  req: JudgeRequest,
  deps: PipelineDeps,
): Promise<JudgeReply> {
  return gateUnsure(req, await judgeUngated(accountId, req, deps));
}

/**
 * Final review item 2: the capability gate for event-3's fourth verdict word. An engine from
 * before stream J's T1 rejects `unsure` (its `VALID_VERDICTS` has three words), treats the reply as
 * a failure and asks — and pays — again every slot. So a request that does not declare
 * `accepts: ["unsure"]` receives exactly the pre-T1 shape instead: no verdict, `low confidence`,
 * `below floor` — today's behaviour for that device, never worse. The `judgments` row keeps what
 * the model actually said; only the reply to the old device is reshaped.
 */
function gateUnsure(req: JudgeRequest, reply: JudgeReply): JudgeReply {
  if (req.kind !== "event" || reply.verdict?.verdict !== "unsure") return reply;
  if (Array.isArray(req.accepts) && req.accepts.includes("unsure")) return reply;
  return { ...reply, verdict: null, outcome: "low confidence", cause: "below floor" };
}

async function judgeUngated(
  accountId: string,
  req: JudgeRequest,
  deps: PipelineDeps,
): Promise<JudgeReply> {
  const started = deps.now();
  const base = {
    account_id: accountId,
    kind: req.kind,
    item_id: itemId(req.item),
    model: null,
    prompt_version: null,
    grammar_version: null,
    prompt_hash: null,
    origin: deps.origin,
  };
  const fields = (v: Record<string, unknown> | null) => fieldsOf(v, req.kind, req.item);

  // Tier 2 first, and free.
  const rule = await deps.rules.lookup(accountId, req.kind, req.item);
  if (rule !== null) {
    const checked = validate(req.kind, rule, req.heuristics_seed);
    if (checked.ok && checked.verdict !== undefined) {
      const id = await deps.log.write({
        ...base,
        tier: 2,
        outcome: "answered",
        cause: null,
        confidence: Number(checked.verdict.confidence ?? 1),
        fields: fields(checked.verdict),
        ms: deps.now() - started,
      });
      return {
        verdict: checked.verdict,
        tier: 2,
        outcome: "answered",
        ...(id === null ? {} : { judgment_id: id }),
      };
    }
    // A rule that no longer validates is a rule that has gone stale — fall through to the model
    // rather than answering with it, and let the promotion job's own evidence retire it.
  }

  // F1 (live proof 2026-10-02): this invocation has already seen `BREAKER_FAILURES` model calls in
  // a row fail, so the provider is down. Answer exactly as a failed call does — the device already
  // reads `model failed` as an outage — but at tier 0, uncharged, without sending anything.
  if (deps.caps.tripped()) {
    await deps.log.write({
      ...base,
      tier: 0,
      outcome: "low confidence",
      cause: "model failed",
      confidence: 0,
      fields: {},
      ms: deps.now() - started,
    });
    return { verdict: null, tier: 0, outcome: "low confidence", cause: "model failed" };
  }

  // The monthly ceiling before the daily cap: an account over budget must not even spend its
  // allowance, and `withinBudget` is memoised per invocation so this is one query, not sixty.
  if (!await deps.caps.withinBudget(accountId)) {
    await deps.log.write({
      ...base,
      tier: 0,
      outcome: "capped",
      cause: null,
      confidence: 0,
      fields: {},
      ms: deps.now() - started,
    });
    return { verdict: null, tier: 0, outcome: "capped" };
  }
  if (!await deps.caps.charge(accountId, req.kind)) {
    await deps.log.write({
      ...base,
      tier: 0,
      outcome: "capped",
      cause: null,
      confidence: 0,
      fields: {},
      ms: deps.now() - started,
    });
    return { verdict: null, tier: 0, outcome: "capped" };
  }

  const prompt = buildPrompt(req.kind, req.item, req.heuristics_seed);
  const pinned = {
    model: deps.row.model_id,
    prompt_version: deps.row.prompt_version,
    grammar_version: deps.row.grammar_version,
    prompt_hash: await promptHash(req.kind),
  };

  let answer;
  try {
    answer = await deps.model.complete({
      model: deps.row.model_id,
      system: prompt.system,
      user: prompt.user,
      schema: prompt.schema,
      maxTokens: deps.row.max_tokens,
      sampling: deps.row.sampling,
      route: deps.row.route,
    });
  } catch (e) {
    // The failure text is deliberately not carried into the row: §5.6 forbids a body in the log,
    // and a provider's error string is the one place a prompt can come back out. The CAUSE is
    // structural and that is what a later reader needs (ruling R-3a-20) — and the three kinds of
    // unusable answer are three different faults with three different fixes.
    const cause: Cause = e instanceof ModelRefused
      ? "refused"
      : e instanceof Error && e.message.includes("max_tokens")
      ? "truncated"
      : "model failed";
    // F1: `model failed` means the model never answered (transport, auth, provider, timeout, an
    // error envelope, an unparseable reply), so the call `charge` counted is given back — an outage
    // must not spend the student's day. A refusal or a truncation is the model answering; it stays
    // charged. The charge before the call is untouched: it is what bounds a hot loop in flight.
    if (cause === "model failed") await deps.caps.refund(accountId, req.kind);
    const id = await deps.log.write({
      ...base,
      ...pinned,
      tier: 3,
      outcome: "low confidence",
      cause,
      confidence: 0,
      fields: {},
      ms: deps.now() - started,
    });
    return {
      verdict: null,
      tier: 3,
      outcome: "low confidence",
      cause,
      ...pinned,
      ...(id === null ? {} : { judgment_id: id }),
    };
  }

  // Recorded whatever the verdict turns out to be: the tokens were spent either way, and the
  // monthly budget is only as honest as this line.
  await deps.caps.recordTokens(accountId, req.kind, answer.inputTokens, answer.outputTokens);

  // T4: the model answers `due` with the deadline phrase as written (or an absolute date only
  // when the email itself stated one) — this is the one place between the model and `validate`
  // where a phrase like "Friday" becomes a calendar date, resolved against the email's own Date
  // line (`req.item.date`), on the student's clock (`req.timezone`), rather than guessed by the model. `resolveDue` itself is where a phrase
  // that names a span rather than one day (e.g. "next week") is refused to `null` — see
  // `judge_due.ts`. Task and event answers have no `due` field (`judge_prompts.ts`'s
  // `TASK_SCHEMA`/`EVENT_SCHEMA`), so this only ever touches email.
  const resolved = req.kind === "email"
    ? { ...answer.json, due: resolveDue(typeof answer.json.due === "string" ? answer.json.due : null, String(req.item.date ?? ""), req.timezone) }
    : answer.json;

  const checked = validate(req.kind, resolved, req.heuristics_seed);
  if (!checked.ok || checked.verdict === undefined) {
    const cause = checked.cause ?? "incomplete";
    const id = await deps.log.write({
      ...base,
      ...pinned,
      tier: 3,
      outcome: "low confidence",
      cause,
      confidence: Number(answer.json.confidence ?? 0),
      fields: {},
      ms: deps.now() - started,
    });
    return {
      verdict: null,
      tier: 3,
      outcome: "low confidence",
      cause,
      ...pinned,
      ...(id === null ? {} : { judgment_id: id }),
    };
  }

  const id = await deps.log.write({
    ...base,
    ...pinned,
    tier: 3,
    outcome: "answered",
    cause: null,
    confidence: Number(checked.verdict.confidence ?? 0),
    fields: fields(checked.verdict),
    ms: deps.now() - started,
  });
  return {
    verdict: checked.verdict,
    tier: 3,
    outcome: "answered",
    ...pinned,
    ...(id === null ? {} : { judgment_id: id }),
  };
}
