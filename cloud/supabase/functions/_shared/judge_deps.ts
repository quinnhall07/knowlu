// The live wiring: a PostgREST client with the service role, the pinned model row, the account's
// rules, the cap store, the judgment sink. Split out of the handlers so every handler test can
// build the same `PipelineDeps` from fakes and never reach a project.
import { capStore } from "./judge_caps.ts";
import { type Db, serviceDb } from "./judge_db.ts";
import { judgmentSink } from "./judge_log.ts";
import { modelRow } from "./judge_models.ts";
import type { JudgmentRow, Kind, PipelineDeps } from "./judge_pipeline.ts";
import { modelFor } from "./judge_provider.ts";
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
  // F-4: `gmail-read` passes its own `PER_CALL_MS` here so the model client's real timeout matches
  // what its budget check reserves — every other caller leaves this unset and gets
  // `judge_anthropic.ts`'s own `CALL_TIMEOUT_MS`.
  modelTimeoutMs?: number,
): Promise<PipelineDeps> {
  const client = sharedDb();
  // The row is read FIRST: `modelFor` needs its `provider` column to pick the client and name the
  // right secret, so building the model before the row exists is not an option any more (it also
  // used to hard-code the one provider this file could ever build).
  const row = await modelRow(client, kind);
  return {
    row,
    model: modelFor(row, Deno.env.get, { timeoutMs: modelTimeoutMs }),
    rules: ruleTable(client),
    caps: capStore(client),
    log: judgmentSink(client),
    origin,
    now: () => Date.now(),
  };
}
