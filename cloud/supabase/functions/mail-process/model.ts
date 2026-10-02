/**
 * `mail-process`'s model path (email plan T3e; spec §4.7 steps 4–6, D8, D9, D23 as amended by PQ6 (a)): an
 * item no screen or template decided. In this order, and never another:
 *
 * 5. **Charge, before the model.** A `backfill` item charges the account's allowance (`charge_allowance`,
 *    which opens one when none was opened in the last 120 days); when that refuses, it charges the day's
 *    `mail_claim` cap as a `live` or `recover` item always does (`charge_call`). Then `enforce_budget`,
 *    whose ceiling is raised by the allowance's dollars while it is open, must pass. Capped or over budget:
 *    no model call; the item waits to the next UTC day, and its third attempt expires it (D8).
 * 4. **The small model**, kind `mail_claim` at its pinned row (prompt and grammar `mail-1`): the subject,
 *    sender, Date, known courses and the scrubbed text cut to 2,000 characters. Its tokens are recorded
 *    (`record_tokens`: tokens, never calls, so an allowance call leaves the day's cap alone). The answer is
 *    checked by `validateMailClaims` against the same text the model saw.
 *    - `model failed` (no answer: the upstream, the network, the clock): a backfill judgment charged to the
 *      allowance is given back (`refund_allowance`), and the item waits as a capped one does, never an
 *      immediate re-charge.
 *    - a refusal, a truncation or a failed validation reached the model: it stays charged, and the item is
 *      finished with no claim.
 * 6. **Queue and forget.** One `mail_claims` row when any claim is not `noise`; the row and the object are
 *    deleted on every final path. The text is never written anywhere.
 *
 * One `judgments` row per call or refusal, kind `mail_claim`, origin `forward`, item id the message's uid;
 * its fields are the claim count and kinds, never a title, an evidence span or a sender.
 *
 * **Privacy:** every log line is a fixed phrase.
 */
import { type JudgeModel, ModelRefused } from "../_shared/judge_anthropic.ts";
import { MAIL_DAILY_CAP, MONTHLY_CEILING_USD } from "../_shared/judge_caps.ts";
import type { Db } from "../_shared/judge_db.ts";
import type { JudgmentRow, JudgmentSink } from "../_shared/judge_log.ts";
import type { ModelRow } from "../_shared/judge_models.ts";
import { buildMailClaimPrompt, mailPromptHash, mailPromptText } from "../_shared/judge_prompts.ts";
import { type Cause, validateMailClaims } from "../_shared/judge_validate.ts";
import { mailUid } from "../_shared/mime.ts";
import type { S3Client } from "../_shared/s3.ts";
import { scrubForPrompt } from "../_shared/scrub.ts";
import {
  claimsRow,
  expireItem,
  finishItem,
  type ModelInput,
  type ProcessStore,
  type WorkItem,
} from "./handler.ts";

/** The charges of §4.7 step 5, each one statement in SQL. Each that cannot reach the database refuses, as
 * `capStore` does: an unreachable cap is no licence to call a metered model. */
export interface MailCharges {
  /** `charge_allowance`: one backfill judgment from the account's allowance, opening it when due. */
  allowance(account: string): Promise<boolean>;
  /** `refund_allowance`: one judgment back after a call the model never answered. */
  refundAllowance(account: string): Promise<boolean>;
  /** `charge_call` for `mail_claim` at its daily cap. */
  daily(account: string): Promise<boolean>;
  /** `enforce_budget` at `MONTHLY_CEILING_USD` (the SQL adds an open allowance's dollars). Read per item. */
  withinBudget(account: string): Promise<boolean>;
  /** `record_tokens` for `mail_claim`; never throws. */
  recordTokens(account: string, inTokens: number, outTokens: number): Promise<void>;
}

export function mailCharges(db: Db): MailCharges {
  const yes = async (call: Promise<unknown>) => {
    try {
      return await call === true;
    } catch {
      return false;
    }
  };
  return {
    allowance: (account) => yes(db.rpc("charge_allowance", { p_account: account })),
    refundAllowance: (account) => yes(db.rpc("refund_allowance", { p_account: account })),
    daily: (account) =>
      yes(
        db.rpc("charge_call", { p_account: account, p_kind: "mail_claim", p_cap: MAIL_DAILY_CAP.mail_claim }),
      ),
    withinBudget: (account) =>
      yes(db.rpc("enforce_budget", { p_account: account, p_ceiling: MONTHLY_CEILING_USD })),
    async recordTokens(account, inTokens, outTokens) {
      try {
        await db.rpc("record_tokens", {
          p_account: account,
          p_kind: "mail_claim",
          p_in: inTokens,
          p_out: outTokens,
        });
      } catch {
        // As `capStore.recordTokens`: the next `enforce_budget` still reads every call that did record.
      }
    },
  };
}

export interface ModelPathDeps {
  store: ProcessStore;
  s3: S3Client;
  now(): number;
  charges: MailCharges;
  log: JudgmentSink;
  /** The `mail_claim` pin and its client, read once and kept. */
  pin(): Promise<{ row: ModelRow; model: JudgeModel }>;
}

/** §4.7 step 5's "at most three attempts". */
export const MAX_ATTEMPTS = 3;

const log = (phrase: string) => console.log(`mail-process: ${phrase}`);

/** The UTC midnight after `ms`, as an ISO time. */
export function nextUtcDay(ms: number): string {
  const d = new Date(ms);
  return new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate() + 1)).toISOString();
}

/** A capped, over-budget or failed attempt: the next UTC day, or expiry at the third. */
async function waitOrExpire(item: WorkItem, deps: ModelPathDeps, why: string): Promise<void> {
  const attempts = item.attempts + 1;
  if (attempts >= MAX_ATTEMPTS) {
    await expireItem(item, deps);
    log(`${why} on the last attempt, expired`);
    return;
  }
  await deps.store.wait(item.id, nextUtcDay(deps.now()), attempts);
  log(`${why}, waits for the next UTC day`);
}

function failureCause(e: unknown): Cause {
  if (e instanceof ModelRefused) return "refused";
  if (e instanceof Error && e.message.includes("max_tokens")) return "truncated";
  return "model failed";
}

/** §4.7 steps 4–6 for one item; `handler.ts` hands it over leased, parsed and screened. */
export function modelPath(deps: ModelPathDeps): (input: ModelInput) => Promise<void> {
  return async ({ item, message, knownCourses }) => {
    const started = deps.now();
    const account = item.accountId;
    // The pin before any charge: a missing row or key is a deployment fault that must not spend a judgment.
    const { row, model } = await deps.pin();
    const base: JudgmentRow = {
      account_id: account,
      kind: "mail_claim",
      item_id: await mailUid(message.headers),
      tier: 0,
      outcome: "capped",
      cause: null,
      confidence: 0,
      fields: {},
      model: null,
      prompt_version: null,
      grammar_version: null,
      prompt_hash: null,
      ms: 0,
      origin: "forward",
    };
    const record = (r: Partial<JudgmentRow>) => deps.log.write({ ...base, ...r, ms: deps.now() - started });

    // Step 5, before any call: the allowance for a backfill item while it lasts, else the day's cap; then the
    // budget, raised by the allowance while it is open.
    const fromAllowance = item.phase === "backfill" && await deps.charges.allowance(account);
    const charged = fromAllowance || await deps.charges.daily(account);
    if (!charged || !(await deps.charges.withinBudget(account))) {
      await record({});
      await waitOrExpire(item, deps, charged ? "over budget" : "capped");
      return;
    }

    const prompt = buildMailClaimPrompt(
      { subject: message.subject, from: message.from, date: message.date, text: message.text },
      { known_courses: knownCourses, time_zone: null },
    );
    const pinned = {
      model: row.model_id,
      prompt_version: row.prompt_version,
      grammar_version: row.grammar_version,
      prompt_hash: await mailPromptHash("mail_claim"),
      tier: 3,
    };
    let answer;
    try {
      answer = await model.complete({
        model: row.model_id,
        system: prompt.system,
        user: prompt.user,
        schema: prompt.schema,
        maxTokens: row.max_tokens,
        sampling: row.sampling,
        route: row.route,
      });
    } catch (e) {
      // The error's text is never carried anywhere: a provider's message is where a prompt can come back.
      const cause = failureCause(e);
      await record({ ...pinned, outcome: "low confidence", cause });
      if (cause === "model failed") {
        if (fromAllowance) await deps.charges.refundAllowance(account);
        await waitOrExpire(item, deps, "model failed");
        return;
      }
      await finishItem(item, deps);
      log(`model ${cause}, deleted`);
      return;
    }
    await deps.charges.recordTokens(account, answer.inputTokens, answer.outputTokens);

    // Checked against the text the model saw: cut and scrubbed (§4.7 step 1).
    const checked = validateMailClaims(answer.json, {
      from: message.from,
      subject: scrubForPrompt(message.subject),
      text: mailPromptText(message.text),
      date: message.date,
      knownCourses,
      timeZone: null,
    }, "model");
    const kinds = [...new Set(checked.claims.map((c) => c.kind))].sort().join(",");
    const id = await record(
      checked.ok
        ? {
          ...pinned,
          outcome: "answered",
          confidence: Math.max(...checked.claims.map((c) => c.confidence)),
          fields: { claims: String(checked.claims.length), kinds },
        }
        : { ...pinned, outcome: "low confidence", cause: checked.cause ?? "incomplete" },
    );
    const kept = checked.claims.filter((c) => c.kind !== "noise");
    if (kept.length > 0) await deps.store.queue(await claimsRow(item, message, kept, id));
    await finishItem(item, deps);
    log(
      kept.length > 0
        ? "queued (model), deleted"
        : checked.ok
        ? "noise (model), deleted"
        : "answer refused, deleted",
    );
  };
}
