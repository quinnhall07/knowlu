import type { Sampling } from "./judge_anthropic.ts";
import type { Db } from "./judge_db.ts";
import type { Kind } from "./judge_validate.ts";

/// The pin. Changing one is a migration row with a date, so every historical judgment names the
/// model that made it — and carries that model's own sampling rules and price (§5.2, §5.4 m4).
export interface ModelRow {
  kind: Kind;
  provider: string;
  model_id: string;
  prompt_version: string;
  grammar_version: string;
  max_tokens: number;
  sampling: Sampling;
  /** Provider-routing preferences (OpenRouter's `provider` object: the pinned upstream, the
   *  zero-retention filter, fallback policy); `{}` for a provider that ignores it. */
  route: Record<string, unknown>;
  /** The upstream's own precision/quantization label, recorded beside the pin so a later reader
   *  does not have to guess it from the model id (e.g. "fp8", "bf16"). */
  precision: string;
  usd_per_m_in: number;
  usd_per_m_out: number;
}

/// There is no default and no fallback: a missing row is a deployment error, and answering with an
/// unpinned model would put a judgment in the log that names a model nobody chose.
export async function modelRow(db: Db, kind: Kind): Promise<ModelRow> {
  const rows = await db.select(
    `models?kind=eq.${kind}&select=kind,provider,model_id,prompt_version,grammar_version,max_tokens,sampling,route,precision,usd_per_m_in,usd_per_m_out`,
  );
  if (rows.length !== 1) throw new Error(`no pinned model for kind '${kind}'`);
  return rows[0] as ModelRow;
}
