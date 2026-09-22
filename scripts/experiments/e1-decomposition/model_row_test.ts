import { assertEquals, assertThrows } from "@std/assert";
import { eventRowFromMigrations, eventRowFromSql } from "./model_row.ts";

Deno.test("the event row is read from the migrations as they stand: the pinned model, route, precision, price", async () => {
  const row = await eventRowFromMigrations();
  assertEquals(row.kind, "event");
  assertEquals(row.provider, "openrouter");
  assertEquals(row.model_id, "ibm-granite/granite-4.2-8b");
  assertEquals(row.precision, "bf16 (CoreWeave)");
  assertEquals(row.route, { order: ["CoreWeave"], allow_fallbacks: false, zdr: true, require_parameters: true });
  assertEquals(row.sampling, { temperature: 0, reasoning: { enabled: false } });
  assertEquals(row.max_tokens, 256);
  assertEquals(row.usd_per_m_in, 0.10);
  assertEquals(row.usd_per_m_out, 0.15);
  assertEquals(row.prompt_version, "event-4");
  assertEquals(row.grammar_version, "event-3");
});

Deno.test("a task update ahead of the event one never leaks into the event row", () => {
  const sql = [
    "insert into models (kind, provider, model_id, prompt_version, grammar_version, max_tokens) values",
    "  ('event', 'anthropic', 'm0', 'event-1', 'event-1', 256);",
    "-- a comment; with a semicolon and where kind = 'event'",
    "update models set model_id = 'task-model', precision = 'p-task' where kind = 'task';",
    "update models set provider = 'openrouter', model_id = 'event-model', precision = 'p-event',",
    "  usd_per_m_in = 0.5, usd_per_m_out = 1.5,",
    `  sampling = '{"temperature": 0}'::jsonb,`,
    `  route = '{"order": ["X"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb`,
    "where kind = 'event';",
  ].join("\n");
  const row = eventRowFromSql([sql]);
  assertEquals(row.model_id, "event-model");
  assertEquals(row.precision, "p-event");
  assertEquals(row.max_tokens, 256);
  assertEquals(row.usd_per_m_out, 1.5);
  assertEquals(row.route.order, ["X"]);
});

Deno.test("an event row that never reached OpenRouter is refused rather than guessed", () => {
  assertThrows(() =>
    eventRowFromSql([
      "insert into models (kind, provider, model_id, prompt_version, grammar_version, max_tokens) values ('event', 'anthropic', 'm', 'e', 'e', 256);",
    ])
  );
});
