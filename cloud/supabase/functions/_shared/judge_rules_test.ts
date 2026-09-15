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

/** Every `*rule_promotion*` migration, filename order, concatenated (ruling R-C2-E47 fix 1):
 * `promote_rules()` is redefined by a later fix migration (`create or replace function`), and the
 * SQL twin's shape lives in whichever definition is LAST — reading only the original file would
 * silently keep pinning this suite against code that is no longer what runs. */
const MIGRATIONS_DIR = new URL("../../migrations/", import.meta.url);
async function ruleMigrationText(): Promise<string> {
  const files: Array<[string, string]> = [];
  for await (const entry of Deno.readDir(MIGRATIONS_DIR)) {
    if (entry.isFile && entry.name.includes("rule_promotion") && entry.name.endsWith(".sql")) {
      files.push([entry.name, await Deno.readTextFile(new URL(entry.name, MIGRATIONS_DIR))]);
    }
  }
  files.sort(([a], [b]) => a.localeCompare(b));
  assert(files.length > 0, "no *rule_promotion* migration found");
  return files.map(([, sql]) => sql).join("\n");
}

Deno.test("featureMap and the SQL twin name the same features", async () => {
  // A rule promoted on one key and looked up by another never fires, and nothing would say so —
  // the model would simply keep being asked. So the two implementations are pinned against each
  // other rather than against a comment.
  const sql = await ruleMigrationText();
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

/** Strips both `--` line comments and block comments (ruling F1's fix, widened): the migration's
 * own prose explains its history in words that would otherwise trip a raw-text assertion meant for
 * the CODE — the doc comments literally contain strings like `min(j.fields)` and `'global'` while
 * explaining why those must never appear as code. Every raw-text assertion below reads stripped
 * text. */
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

/** The LAST `create or replace function <name>(...)` body of that name in the concatenated
 * `*rule_promotion*` corpus — a fix migration redefines a function wholesale, so only the last
 * definition is what actually runs; an earlier, superseded body is history, not the contract.
 *
 * R-C2-E49 fix 3: case-insensitive, with an optional schema-qualifying `public.` (and either half
 * optionally double-quoted) — the same shape `identPattern` gives every identifier in
 * `migrations_test.ts` — so a future schema-qualified re-issue (`create or replace function
 * public.promote_rules()`) is still found as the LAST definition rather than silently falling back
 * to a superseded, unqualified one a literal-string search would have missed. Matched up to the
 * opening `(` only, not the whole argument list, since two functions in this corpus
 * (`promote_rules()`, `judgment_features(p_kind text, p_fields jsonb)`) do not share one shape. */
function lastFunctionBody(sql: string, name: string): string {
  const marker = new RegExp(
    `create\\s+or\\s+replace\\s+function\\s+(?:"?public"?\\s*\\.\\s*)?"?${name}"?\\s*\\(`,
    "gi",
  );
  let start = -1;
  for (const m of sql.matchAll(marker)) start = m.index;
  assert(start !== -1, `no ${name}(...) definition found in the *rule_promotion* migrations`);
  const close = sql.indexOf("\n$$;", start);
  assert(close !== -1, `could not find the closing $$; for the last ${name} body`);
  return sql.slice(start, close + 4);
}

/** The LAST `cron.schedule('knowlu-promote-rules', ...)` call in the concatenated corpus — a fix
 * migration may `cron.unschedule` and re-`cron.schedule` the same job with different SQL text, and
 * only the last schedule call is the one pg_cron actually holds. */
function lastCronSchedule(sql: string): string {
  const marker = "cron.schedule('knowlu-promote-rules'";
  const start = sql.lastIndexOf(marker);
  assert(start !== -1, "no cron.schedule('knowlu-promote-rules' call found in the *rule_promotion* migrations");
  // `$$);` — the dollar-quoted job body's own close, then the call's closing paren and semicolon —
  // not a plain `");"`, which also matches the end of every inner statement inside the $$ body
  // (e.g. `...judgments();`), finding the wrong, much-too-early close.
  const end = sql.indexOf("$$);", start);
  assert(end !== -1, "could not find the closing $$); for the last cron.schedule call");
  return sql.slice(start, end + 4);
}

Deno.test("promote_rules is scheduled, deterministic, and never promotes a global rule", async () => {
  const sql = await ruleMigrationText();
  const wholeCorpus = stripComments(sql);
  const body = stripComments(lastFunctionBody(sql, "promote_rules"));
  const featuresBody = stripComments(lastFunctionBody(sql, "judgment_features"));
  const cron = stripComments(lastCronSchedule(sql));

  // (a) `min(jsonb)` does not exist in PostgreSQL and would error on the first call.
  assert(!/min\s*\(\s*j\.fields\s*\)/.test(body), "there is no min() aggregate for jsonb");
  assert(body.includes("array_agg"), "the representative answer is picked deterministically");
  assert(body.includes("count(distinct"), "agreement is counted on distinct items, not judgment rows (M2)");

  // (b) the schedule exists, names the job, and — R-C2-E47 fix 1 (C1b) — is two schema-qualified
  // statements (pg_cron runs under its own search_path, not this migration's), never a two-call
  // target list (unspecified evaluation order).
  assert(cron.length > 0, "the schedule exists");
  assert(
    cron.includes("select public.backfill_correction_judgments();") &&
      cron.includes("select public.promote_rules();"),
    `the schedule must be two schema-qualified statements: ${cron}`,
  );

  // (c) §11 R5: nothing here writes or activates a global rule — checked on the LAST
  // `promote_rules` body (what actually runs today) AND, R-C2-E49 fix 3, over the WHOLE
  // comment-stripped `*rule_promotion*` corpus (every superseded body too — a fix migration that
  // re-issues `promote_rules` but leaves an old, still-live global-writing statement somewhere
  // else in an earlier file would pass the body-only check and still be a real hole).
  assert(!body.includes("'global'"), "global rules are hand-reviewed and this job never writes one");
  assert(body.includes("'account'"));
  assert(
    !wholeCorpus.includes("'global'"),
    "no *rule_promotion* migration, superseded or current, writes or activates a global rule",
  );

  // (d) R-C2-E47 fix 1 (I2): a prior decision — approved OR rejected — blocks re-promotion for 90
  // days, not only while active or undecided (which matched neither state of a rejection).
  assert(
    /decided_at\s*>\s*now\(\)\s*-\s*interval\s*'90 days'/.test(body),
    "a rejected (or approved) proposal must block re-promotion for 90 days",
  );

  // (e) R-C2-E47 fix 1 (I3): the expiry sweep never touches a global candidate awaiting hand review.
  assert(
    /delete\s+from\s+rules\s+where\s+scope\s*=\s*'account'/.test(body),
    "the expiry sweep must be scoped to account rules only",
  );

  // (f) R-C2-E49 fix 3 — the three minors fix 1 introduced but never pinned, on the last body of
  // the function each actually lives in:
  //   M1 — `created_by+title_prefix` requires a non-empty title prefix, not just a non-empty
  //   `created_by` (an earlier version promoted on `"zybooks|"`, a key the TypeScript lookup can
  //   never produce). Lives in `judgment_features`, not `promote_rules` — `promote_rules` only
  //   calls it.
  assert(
    featuresBody.includes("coalesce(p_fields->>'title_prefix', '') <> ''"),
    "the created_by+title_prefix feature must require a non-empty title prefix (M1)",
  );
  //   M3 — the representative answer's tie-break is a TOTAL order (`judged_at desc, id desc`), not
  //   `judged_at desc` alone, which is non-deterministic between judgments at the same timestamp.
  assert(
    body.includes("order by judged_at desc, id desc"),
    "the representative answer's tie-break must be a total order (M3)",
  );
  //   M11 — an all-empty verdict is never promoted, however many times it agreed.
  assert(
    body.includes("answer <> '{}'::jsonb"),
    "an all-empty verdict must not be promoted (M11)",
  );
});
