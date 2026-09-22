// The pinned `event` row, read from the migrations exactly as a fresh database would end up with it:
// the seed insert first (max_tokens and the rest), then every `update models set … where kind =
// 'event'` in filename order, later ones winning. E1 runs against THIS row — the model id, the
// OpenRouter route and precision the product pins — so its number is about the model the product
// actually calls, not one chosen for the experiment.
import type { ModelRow } from "../../../cloud/supabase/functions/_shared/judge_models.ts";

const MIGRATIONS = new URL("../../../cloud/supabase/migrations/", import.meta.url);

function stripComments(sql: string): string {
  return sql.split("\n").map((line) => line.replace(/--.*$/, "")).join("\n");
}

function str(stmt: string, key: string): string | undefined {
  return new RegExp(`\\b${key}\\s*=\\s*'([^']*)'`, "i").exec(stmt)?.[1];
}

function num(stmt: string, key: string): number | undefined {
  const m = new RegExp(`\\b${key}\\s*=\\s*([0-9.]+)`, "i").exec(stmt)?.[1];
  return m === undefined ? undefined : Number(m);
}

function json(stmt: string, key: string): Record<string, unknown> | undefined {
  const m = new RegExp(`\\b${key}\\s*=\\s*'(\\{[^']*\\})'::jsonb`, "i").exec(stmt)?.[1];
  return m === undefined ? undefined : JSON.parse(m);
}

/** The event row from a list of migration texts, in apply order. Pure, so it is tested on SQL. */
export function eventRowFromSql(texts: string[]): ModelRow {
  const row: Partial<ModelRow> = { kind: "event" };
  for (const text of texts) {
    for (const raw of stripComments(text).split(";")) {
      const stmt = raw.trim();
      const seed = /\(\s*'event'\s*,\s*'([^']*)'\s*,\s*'([^']*)'\s*,\s*'([^']*)'\s*,\s*'([^']*)'\s*,\s*(\d+)\s*\)/i
        .exec(stmt);
      if (/^insert\s+into\s+models\s*\(\s*kind\s*,\s*provider\s*,\s*model_id\s*,\s*prompt_version\s*,\s*grammar_version\s*,\s*max_tokens\s*\)/i.test(stmt) && seed) {
        row.provider = seed[1];
        row.model_id = seed[2];
        row.prompt_version = seed[3];
        row.grammar_version = seed[4];
        row.max_tokens = Number(seed[5]);
        continue;
      }
      if (!/^update\s+models\s+set\b/i.test(stmt) || !/where\s+kind\s*=\s*'event'\s*$/i.test(stmt)) continue;
      row.provider = str(stmt, "provider") ?? row.provider;
      row.model_id = str(stmt, "model_id") ?? row.model_id;
      row.precision = str(stmt, "precision") ?? row.precision;
      row.prompt_version = str(stmt, "prompt_version") ?? row.prompt_version;
      row.grammar_version = str(stmt, "grammar_version") ?? row.grammar_version;
      row.max_tokens = num(stmt, "max_tokens") ?? row.max_tokens;
      row.usd_per_m_in = num(stmt, "usd_per_m_in") ?? row.usd_per_m_in;
      row.usd_per_m_out = num(stmt, "usd_per_m_out") ?? row.usd_per_m_out;
      row.sampling = json(stmt, "sampling") ?? row.sampling;
      row.route = json(stmt, "route") ?? row.route;
    }
  }
  if (row.provider !== "openrouter") {
    throw new Error(`the event row's provider is '${row.provider}', not openrouter — E1 calls OpenRouter only`);
  }
  for (const key of ["model_id", "max_tokens", "sampling", "route", "precision", "usd_per_m_in", "usd_per_m_out"]) {
    if ((row as Record<string, unknown>)[key] === undefined) throw new Error(`the event row has no ${key}`);
  }
  return row as ModelRow;
}

export async function eventRowFromMigrations(): Promise<ModelRow> {
  const names: string[] = [];
  for await (const entry of Deno.readDir(MIGRATIONS)) {
    if (entry.isFile && entry.name.endsWith(".sql")) names.push(entry.name);
  }
  names.sort();
  const texts = await Promise.all(names.map((n) => Deno.readTextFile(new URL(n, MIGRATIONS))));
  return eventRowFromSql(texts);
}
