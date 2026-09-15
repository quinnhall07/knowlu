import { assert, assertEquals } from "@std/assert";
import { featureMap, features, titlePrefix } from "./judge_rules.ts";

Deno.test("a task is keyed on created_by plus the title prefix, most specific first", () => {
  const item = { title: "CS 100 Lab 02 math_lib", created_by: "zybooks" };
  assertEquals(features("task", item), [
    ["created_by+title_prefix", "zybooks|CS 100 Lab"],
    ["title_prefix", "CS 100 Lab"],
  ]);
  assertEquals(titlePrefix("CS 100 Lab 02 math_lib"), "CS 100 Lab");
});

Deno.test("an event is keyed on organizer, source, series and the title prefix", () => {
  const item = { title: "AI Club Kickoff", organizer: "AI Club", source: "engage", series_uid: "engage:series:9" };
  assertEquals(features("event", item).map(([f]) => f), ["organizer", "source", "series", "title_prefix"]);
});

Deno.test("featureMap and the SQL twin name the same features", async () => {
  // A rule promoted on one key and looked up by another never fires, and nothing would say so —
  // the model would simply keep being asked. So the two implementations are pinned against each
  // other rather than against a comment.
  const sql = await Deno.readTextFile(
    new URL("../../migrations/20260911000400_rule_promotion.sql", import.meta.url),
  );
  const inSql = [...sql.matchAll(/\('(created_by\+title_prefix|title_prefix|organizer|source|series)',/g)]
    .map((m) => m[1]);
  const inTs = new Set([
    ...features("task", { title: "a b c", created_by: "x" }).map(([f]) => f),
    ...features("event", { title: "a b c", organizer: "o", source: "s", series_uid: "u" }).map(([f]) => f),
  ]);
  for (const feature of inSql) assert(inTs.has(feature), `the SQL promotes on '${feature}' and the lookup does not`);
  for (const feature of inTs) assert(inSql.includes(feature), `the lookup keys on '${feature}' and the SQL does not`);
});

Deno.test("the feature map carries keys and never free text", () => {
  const map = featureMap("task", { title: "CS 100 Lab 02 TRIPWIRE-9f2c", created_by: "zybooks" });
  assertEquals(map, { title_prefix: "CS 100 Lab", created_by: "zybooks" });
  assertEquals(JSON.stringify(map).includes("TRIPWIRE"), false);
});

/** Strips both `--` line comments and block comments (ruling F1's fix, widened): the
 * migration's own prose explains its history in words that would otherwise trip a raw-text
 * assertion meant for the CODE — the `(a)…(d)` block comment literally contains the string
 * `min(j.fields)` while explaining why that expression must never appear as code again, and the
 * R5 line comment literally contains `'global'` while explaining why no code path ever writes
 * one. Stripped once, up front, and every raw-text assertion below reads the stripped text. */
function stripComments(sql: string): string {
  return sql
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => {
      const at = line.indexOf("--");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
}

Deno.test("promote_rules is scheduled, deterministic, and never promotes a global rule", async () => {
  const sql = await Deno.readTextFile(
    new URL("../../migrations/20260911000400_rule_promotion.sql", import.meta.url),
  );
  const code = stripComments(sql);
  // (a) `min(jsonb)` does not exist in PostgreSQL and would error on the first call.
  assert(!/min\s*\(\s*j\.fields\s*\)/.test(code), "there is no min() aggregate for jsonb");
  assert(code.includes("array_agg"), "the representative answer is picked deterministically");
  // (b) the schedule exists and the extension it needs was enabled in 20260911000100.
  assert(code.includes("cron.schedule('knowlu-promote-rules'"));
  // (c) §11 R5: nothing here writes or activates a global rule.
  assert(!code.includes("'global'"), "global rules are hand-reviewed and this job never writes one");
  assert(code.includes("'account'"));
});
