// The pipeline of cloud design §5.2, and the only place the order of those steps is written down:
//
//   rules (tier 2, free) -> monthly budget -> daily cap -> constrained call (tier 3)
//                        -> validate -> record tokens -> log -> reply
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

  const checked = validate(req.kind, answer.json, req.heuristics_seed);
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
