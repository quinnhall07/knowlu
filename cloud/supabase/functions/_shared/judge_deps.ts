// The live wiring: a PostgREST client with the service role, the pinned model row, the account's
// rules, the cap store, the judgment sink. Split out of the handlers so every handler test can
// build the same `PipelineDeps` from fakes and never reach a project.
import { AnthropicModel } from "./judge_anthropic.ts";
import { capStore } from "./judge_caps.ts";
import { type Db, serviceDb } from "./judge_db.ts";
import { judgmentSink } from "./judge_log.ts";
import { modelRow } from "./judge_models.ts";
import type { JudgmentRow, Kind, PipelineDeps } from "./judge_pipeline.ts";
import { ruleTable } from "./judge_rules.ts";

/// Built at first use, never at module scope: a throw at module scope is a boot failure with an
/// opaque message, and every other failure in this codebase is a named status.
let db: Db | null = null;
export function sharedDb(): Db {
  if (db === null) db = serviceDb();
  return db;
}

export async function liveDeps(
  kind: Kind,
  origin: JudgmentRow["origin"] = "device",
): Promise<PipelineDeps> {
  const apiKey = Deno.env.get("ANTHROPIC_API_KEY");
  if (apiKey === undefined || apiKey === "") throw new Error("the function is missing ANTHROPIC_API_KEY");
  const client = sharedDb();
  return {
    row: await modelRow(client, kind),
    model: new AnthropicModel({ apiKey }),
    rules: ruleTable(client),
    caps: capStore(client),
    log: judgmentSink(client),
    origin,
    now: () => Date.now(),
  };
}
