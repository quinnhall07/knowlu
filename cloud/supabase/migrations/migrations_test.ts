// Static pins on the migrations. There is no Docker in this plan, so nothing here applies SQL:
// these are the invariants that would otherwise only be discovered on a project that already has
// rows in it, which is the wrong time to discover them.
import { assert, assertEquals, assertThrows } from "@std/assert";

const HERE = new URL(".", import.meta.url);

async function ours(): Promise<Array<[string, string]>> {
  const out: Array<[string, string]> = [];
  for await (const entry of Deno.readDir(HERE)) {
    if (entry.name.startsWith("20260911") && entry.name.endsWith(".sql")) {
      out.push([entry.name, await Deno.readTextFile(new URL(entry.name, HERE))]);
    }
  }
  out.sort();
  assert(out.length > 0, "C2's migrations are the 20260911 ones; C1's are 20260910 (R-X-8)");
  return out;
}

/**
 * Every `.sql` file directly under `dir`, C1's and every other stream's included — not only this
 * stream's own `20260911…` ones (R-C2-E36: a SECURITY DEFINER function created by any migration,
 * in any stream, is a privilege gap the guard below must be able to see). `dir` defaults to this
 * directory; it takes a parameter, not an env var, so a scratch reproduction can point it at a copy
 * without touching this file.
 */
export async function everyMigrationFile(dir: URL = HERE): Promise<Array<[string, string]>> {
  const out: Array<[string, string]> = [];
  for await (const entry of Deno.readDir(dir)) {
    if (entry.isFile && entry.name.endsWith(".sql")) {
      out.push([entry.name, await Deno.readTextFile(new URL(entry.name, dir))]);
    }
  }
  out.sort();
  return out;
}

/** One identifier, in any of the forms `supabase db diff` (or a person) can write it: bare
 * (`foo`), schema-qualified (`public.foo`), or either half double-quoted (`"public".foo`,
 * `public."foo"`, `"public"."foo"`). `name` is either `(\w+)` — a capturing group, for parsing an
 * unknown name out of a definition — or an already-known literal name, for searching a revoke
 * statement for it; either way the NAME ALONE compares unquoted, so a definition and a revoke that
 * spell the same function differently still match (R-C2-E37c). */
function identPattern(name: string): string {
  return `(?:"?public"?\\s*\\.\\s*)?"?${name}"?`;
}

/**
 * A function definition, anywhere: `create [or replace] function`, the identifier (§`IDENT`), its
 * parameter list, and — non-greedily — everything up to and through its `AS $tag$ … $tag$` body,
 * whatever the dollar-quote tag is (`$$`, `$fn$`, …; the SAME tag must close it, so a body
 * containing an unrelated `$$` is never mistaken for the end), through the terminating `;` —
 * because Postgres allows trailing clauses AFTER the body (`as $$ … $$ language sql security
 * definer;`), and a definer test that stopped at the closing tag would miss one (R-C2-E37b).
 *
 * Group 1 is the HEADER ALONE — from `create` through the opening `as $tag$`, never the body —
 * because the `returns trigger` exemption below must never be satisfied by anything a body merely
 * CONTAINS (a string literal, a comment that survived stripping) rather than the real return type
 * the function was actually declared with (R-C2-E37a). Group 2 is the name; group 3 the tag.
 */
const FUNCTION_DEFINITION = new RegExp(
  `(create\\s+(?:or\\s+replace\\s+)?function\\s+${identPattern("(\\w+)")}\\s*\\([^)]*\\)[\\s\\S]*?\\bas\\s+(\\$[A-Za-z_]*\\$))[\\s\\S]*?\\3[\\s\\S]*?;`,
  "gi",
);

/** How many times the bare `create [or replace] function` phrase appears — independent of whether
 * `FUNCTION_DEFINITION` above can actually parse what follows. Used only by the parse-count check
 * (R-C2-E37e): a shape the definition regex cannot parse (a digit in the dollar tag, a
 * `begin atomic` body, a single-quoted body, a schema this codebase does not otherwise use, …)
 * must never silently vanish from the scan — it must fail loud instead. */
const RAW_FUNCTION_KEYWORD = /create\s+(?:or\s+replace\s+)?function/gi;

/** Strips every `--` line comment (R-C2-E37d): a commented-out `-- revoke execute …` must not
 * satisfy the guard, and a commented-out definition must not be scanned at all — by either the
 * definition regex or the raw keyword count above, so the two stay in agreement on real code only.
 * Line-based, not string-literal-aware: nothing in this corpus puts `--` inside a string (checked
 * by hand), and a guard that is too strict here fails loud (R-C2-E37e) rather than silently. */
function stripLineComments(sql: string): string {
  return sql
    .split("\n")
    .map((line) => {
      const at = line.indexOf("--");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
}

/** The exact marker a writing, non-definer function's definition must carry on the line
 * IMMEDIATELY before its `create [or replace] function` line to be exempted from the widened
 * write-revoke rule below (R-C2-E47 fix 1, I1) — checked against the ORIGINAL text, never the
 * comment-stripped one, since the marker is itself a `--` comment. `matchIndex` is an offset into
 * `strippedSql`; line counts are identical between the stripped and original text because
 * `stripLineComments` only blanks part of a line, never removes a newline. */
const WRITE_REVOKE_MARKER = "-- rpc: authenticated by design";
function markerLineBefore(originalSql: string, strippedSql: string, matchIndex: number): boolean {
  const lineNumber = strippedSql.slice(0, matchIndex).split("\n").length - 1;
  const before = originalSql.split("\n")[lineNumber - 1];
  return before !== undefined && before.trim() === WRITE_REVOKE_MARKER;
}

/**
 * `files`, sorted, from earliest to latest: for every SECURITY DEFINER function creation, AND for
 * every non-definer function whose body writes (`insert`/`update`/`delete`, R-C2-E47 fix 1, I1), a
 * `revoke execute … from …` naming both `anon` and `authenticated` somewhere from THAT file onward
 * (R-C2-E29 fixed it; R-C2-E36 widens where this guard is allowed to look for both the creation and
 * the fix; R-C2-E37 closes the three ways a definer function could still pass unseen). The
 * definer/invoker split only decides whose privileges a call runs with — it never decides whether
 * `anon`/`authenticated` can reach the function over PostgREST at all — so a writing INVOKER
 * function is exactly as reachable as a writing DEFINER one, and the old guard's silent `if (!
 * security definer) continue` was a gap of exactly that shape (`charge_call`, `record_tokens`,
 * `enforce_budget` all had it). A `returns trigger` function is exempt BY KIND, not by name or by
 * migration — it cannot be called through PostgREST at all. A non-definer writing function may
 * instead be exempt BY MARKER (`WRITE_REVOKE_MARKER` on the line directly above its `create`) when
 * its write is already safe some OTHER way (e.g. RLS-with-no-policy on the table it writes) and
 * revoking it is deliberately out of scope for the task that found it — never silently, and every
 * exempted name (by kind or by marker) is returned so a caller can assert on it directly rather
 * than the exemption swallowing the cases that prove it fires. `parsed` is the total definitions
 * this run actually parsed, across every file, so a caller can pin the corpus's own count.
 */
export function assertExecuteRevoked(
  files: Array<[string, string]>,
): { exemptedTriggers: string[]; exemptedByMarker: string[]; parsed: number } {
  const stripped = files.map(([name, sql]) => [name, stripLineComments(sql)] as [string, string]);
  const exemptedTriggers: string[] = [];
  const exemptedByMarker: string[] = [];
  let parsed = 0;
  for (let i = 0; i < stripped.length; i++) {
    const [name, sql] = stripped[i];
    const originalSql = files[i][1];
    const rawCount = (sql.match(RAW_FUNCTION_KEYWORD) ?? []).length;
    const definitions = [...sql.matchAll(FUNCTION_DEFINITION)];
    assert(
      rawCount === definitions.length,
      `${name}: found ${rawCount} 'create function' occurrence(s) but the definition regex parsed ` +
        `${definitions.length} — a shape it cannot parse (R-C2-E37e). Every function this file ` +
        `creates must be a shape the scan can see, or it is a privilege gap the scan cannot see either.`,
    );
    for (const d of definitions) {
      parsed += 1;
      const header = d[1];
      const fn = d[2];
      if (/returns\s+trigger/i.test(header)) {
        exemptedTriggers.push(fn);
        continue;
      }
      const isDefiner = /security\s+definer/i.test(d[0]);
      const writes = /\b(insert|update|delete)\b/i.test(d[0]);
      if (!isDefiner && !writes) continue;
      if (!isDefiner && writes && markerLineBefore(originalSql, sql, d.index ?? -1)) {
        exemptedByMarker.push(fn);
        continue;
      }
      const rest = stripped.slice(i).map(([, s]) => s).join("\n");
      // A previous migration's own `revoke … from public` may still be sitting right there
      // (forward-only: it is never edited out) — every match counts, not just the first, so a
      // narrower fix-up revoke later in the corpus still satisfies this.
      const revokeRe = new RegExp(
        `revoke\\s+execute\\s+on\\s+function\\s+${identPattern(fn)}\\s*\\([^)]*\\)\\s+from\\s+([^;]+);`,
        "gi",
      );
      const matches = [...rest.matchAll(revokeRe)];
      assert(
        matches.length > 0,
        `${name}: ${fn} is ${isDefiner ? "SECURITY DEFINER" : "a writing function"} with no ` +
          `'revoke execute … from …' in this or a later migration, and carries no ` +
          `'${WRITE_REVOKE_MARKER}' marker (R-C2-E29 / R-C2-E47 fix 1)`,
      );
      const ok = matches.some((m) => {
        const from = m[1].toLowerCase();
        return from.includes("anon") && from.includes("authenticated");
      });
      assert(
        ok,
        `${name}: ${fn} has no revoke naming both anon and authenticated — 'from public' alone leaves both callable`,
      );
    }
  }
  return { exemptedTriggers, exemptedByMarker, parsed };
}

Deno.test("every table this stream creates has row level security enabled", async () => {
  for (const [name, sql] of await ours()) {
    for (const m of sql.matchAll(/create table if not exists (\w+)/g)) {
      assert(
        sql.includes(`alter table ${m[1]} enable row level security;`),
        `${name}: ${m[1]} must have RLS enabled. Every C2 function runs with the service role and ` +
          `scopes by account_id itself, so a table reachable by the anon key is one nobody meant to expose.`,
      );
    }
  }
});

Deno.test("every SECURITY DEFINER or writing function in every migration has execute revoked, unless marked authenticated-by-design", async () => {
  // R-C2-E29 / R-C2-E36: `revoke execute … from public` removes only the PUBLIC entry. Supabase
  // grants execute to `anon` and `authenticated` explicitly on every new function by default, so a
  // SECURITY DEFINER function — which runs with the DEFINING role's privileges, not the caller's —
  // stays reachable over PostgREST with the anon key unless both are named too. Scoped to EVERY
  // migration, not only this stream's own `20260911…` ones: a gap in a later stream's own function
  // is exactly as live a hole as one in this stream's. R-C2-E47 fix 1 (I1) widened the guard: a
  // non-definer function that writes (`insert`/`update`/`delete`) is just as reachable over
  // PostgREST as a definer one, so it now needs the same revoke unless marked
  // `-- rpc: authenticated by design` on the line directly before its `create`.
  const { parsed } = assertExecuteRevoked(await everyMigrationFile());
  // R-C2-E37e: pinned so a future function that silently stops being parsed (rather than being
  // caught by the per-file count check) still shows up here as a number that moved without a
  // reason on the diff. Ten single functions plus `store_google_grant`'s own `create or replace`
  // in both 20260911000200 and 20260911000300, plus Task 12's three non-definer functions in
  // 20260911000400 (`judgment_features`, `backfill_correction_judgments`, `promote_rules`), plus
  // fix 1's re-issue of the same three in 20260911000500 — counted by hand against today's corpus.
  assertEquals(parsed, 17, "today's corpus should parse exactly 17 function creations");
});

Deno.test("a writing, non-definer function with no revoke and no marker is caught by the widened guard", () => {
  // R-C2-E47 fix 1: before this fix, `!security definer` short-circuited the whole check — a
  // writing INVOKER function was invisible to it no matter what it wrote or who could call it.
  const sql = `
create or replace function sneaky_writer(p_account uuid)
returns void
language sql
security invoker
as $$
  insert into some_table (account_id) values (p_account);
$$;
`;
  assertThrows(
    () => assertExecuteRevoked([["synthetic-writer.sql", sql]]),
    Error,
    "sneaky_writer",
  );
});

Deno.test("the marker exempts a writing function only from the line directly above its create, never from further up", () => {
  const marked = `
-- Called only from the service role, never a public bearer.
-- rpc: authenticated by design
create or replace function marked_writer(p_account uuid)
returns void
language sql
security invoker
as $$
  insert into some_table (account_id) values (p_account);
$$;
`;
  const { exemptedByMarker } = assertExecuteRevoked([["synthetic-marked.sql", marked]]);
  assertEquals(exemptedByMarker, ["marked_writer"]);

  const markedTooFarUp = `
-- rpc: authenticated by design
--
create or replace function almost_marked_writer(p_account uuid)
returns void
language sql
security invoker
as $$
  insert into some_table (account_id) values (p_account);
$$;
`;
  assertThrows(
    () => assertExecuteRevoked([["synthetic-almost-marked.sql", markedTooFarUp]]),
    Error,
    "almost_marked_writer",
  );
});

Deno.test("the widened guard actually reached and exempted C2's three pre-existing capacity functions", async () => {
  // R-C2-E47 fix 1: `charge_call`, `record_tokens` and `enforce_budget` (20260911000100) are the
  // real writing, non-definer functions the widened guard found with no revoke — pre-existing,
  // out of scope for this task to revoke, and marked rather than silently grandfathered.
  const { exemptedByMarker } = assertExecuteRevoked(await everyMigrationFile());
  for (const fn of ["charge_call", "record_tokens", "enforce_budget"]) {
    assert(exemptedByMarker.includes(fn), `expected ${fn} among the marker-exempted functions, got: ${exemptedByMarker}`);
  }
});

Deno.test("a trigger function is exempt from the execute-revoke guard by kind, not by name — and the scan proves it by finding one", async () => {
  // R-C2-E36: C1's `handle_new_user` (`returns trigger`) is SECURITY DEFINER with no execute
  // revoke of its own, and rightly so — a trigger function cannot be called through PostgREST at
  // all, so revoking EXECUTE from it protects nothing. The risk in exempting by kind is exempting
  // silently: this asserts the scan actually reached and recognised `handle_new_user`, rather than
  // the exemption papering over a scan that never got that far (a schema-qualified name, for one,
  // used to be exactly that gap — see the RED evidence in the task report).
  const { exemptedTriggers } = assertExecuteRevoked(await everyMigrationFile());
  assert(
    exemptedTriggers.includes("handle_new_user"),
    `expected handle_new_user among the trigger-exempted functions, got: ${exemptedTriggers}`,
  );
});

// R-C2-E37: five synthetic-SQL regression cases, one per adversarial shape the re-review found.
// Each is a small string through the exported `assertExecuteRevoked` — no directory, no real
// migration file needed, and no earlier RED evidence would have caught these (the real corpus
// never carried any of these shapes; that's exactly how they went unnoticed).

Deno.test("(a) the trigger exemption reads only the header, never the body", () => {
  // Before R-C2-E37a this function would have been wrongly exempted: `/returns\s+trigger/i`
  // tested the WHOLE match, and this body's own text (a string literal, not a comment — comment
  // stripping would not save it either) contains exactly that phrase.
  const sql = `
create or replace function sneaky_definer(p_account uuid)
returns text
language plpgsql
security definer
set search_path = public, extensions
as $$
begin
  raise exception 'this body returns trigger nonsense on purpose, and must not exempt me';
  return 'x';
end;
$$;
`;
  assertThrows(
    () => assertExecuteRevoked([["synthetic-a.sql", sql]]),
    Error,
    "sneaky_definer",
  );
});

Deno.test("(b) SECURITY DEFINER trailing the body still counts", () => {
  // Postgres allows `language`/`security` after the closing dollar tag. Before R-C2-E37b the
  // match ended at the closing tag, so this function read as non-definer and passed silently
  // with no revoke at all.
  const sql = `
create or replace function trailing_definer(p_account uuid)
returns text
as $$
  select 'x';
$$ language sql security definer;
`;
  assertThrows(
    () => assertExecuteRevoked([["synthetic-b.sql", sql]]),
    Error,
    "trailing_definer",
  );
});

Deno.test("(c) a double-quoted, schema-qualified identifier is parsed and matched unquoted", () => {
  // The shape `supabase db diff` emits. Definition and revoke spell the identifier two different
  // ways (fully quoted vs bare) — both must resolve to the same unquoted name for the guard to
  // pass at all.
  const sql = `
create or replace function "public"."quoted_definer"(p_account uuid)
returns text
language sql
security definer
as $$
  select 'x';
$$;
revoke execute on function quoted_definer(uuid) from public, anon, authenticated;
`;
  assertExecuteRevoked([["synthetic-c.sql", sql]]); // must not throw
});

Deno.test("(d) comments are stripped before either scan", () => {
  // (d1) A commented-out revoke must not satisfy the guard.
  const commentedRevoke = `
create or replace function commented_revoke_definer(p_account uuid)
returns text
language sql
security definer
as $$
  select 'x';
$$;
-- revoke execute on function commented_revoke_definer(uuid) from public, anon, authenticated;
`;
  assertThrows(
    () => assertExecuteRevoked([["synthetic-d1.sql", commentedRevoke]]),
    Error,
    "commented_revoke_definer",
  );

  // (d2) A commented-out DEFINITION must not be scanned at all — the parse-count check (e) must
  // not misfire on it either: zero raw `create function` occurrences, zero parsed, in agreement.
  const commentedDefinition = `
-- create or replace function ghost_definer(p_account uuid)
-- returns text
-- language sql
-- security definer
-- as $$
--   select 'x';
-- $$;
`;
  const { exemptedTriggers } = assertExecuteRevoked([["synthetic-d2.sql", commentedDefinition]]);
  assertEquals(exemptedTriggers, []);
});

Deno.test("(e) a shape the definition regex cannot parse fails loud via the parse-count check", () => {
  // A dollar-quote tag with a digit in it (`$tag1$`) is valid Postgres but this file's tag pattern
  // is letters/underscore only, so `FUNCTION_DEFINITION` parses zero definitions here even though
  // one real `create function` exists — exactly the silent-miss shape this check exists to catch.
  // A correct revoke is present and irrelevant: without the count check this passes with zero
  // iterations, silently, whether or not any revoke exists at all.
  const sql = `
create or replace function unparseable_tag_definer(p_account uuid)
returns text
language sql
security definer
as $tag1$
  select 'x';
$tag1$;
revoke execute on function unparseable_tag_definer(uuid) from public, anon, authenticated;
`;
  const err = assertThrows(() => assertExecuteRevoked([["synthetic-e.sql", sql]]), Error);
  const message = err instanceof Error ? err.message : String(err);
  assert(message.includes("synthetic-e.sql"), message);
  assert(message.includes("found 1"), message);
  assert(message.includes("parsed 0"), message);
});

Deno.test("the judgments table has nowhere to put a body", async () => {
  const sql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  const start = sql.indexOf("create table if not exists judgments");
  const create = sql.slice(start, sql.indexOf(");", start));
  // Column NAMES, not substrings: `item_id text not null` legitimately contains the word "text".
  for (const line of create.split("\n").slice(1)) {
    const column = /^\s{2}([a-z_]+)\s/.exec(line)?.[1];
    if (column === undefined) continue;
    assert(
      !["body", "title", "prompt", "reply", "text", "subject", "snippet", "description", "message"].includes(
        column,
      ),
      `cloud design §5.2/§5.6: the judgment log holds ids, field values, confidences and the ` +
        `promotion features — never the body. A column called '${column}' is how that stops being true.`,
    );
  }
});

Deno.test("no migration in this stream drops or truncates a table", async () => {
  // Word-boundary, not substring: TRUNCATE's TABLE keyword is optional in Postgres
  // (`TRUNCATE [TABLE] [ONLY] name`), so the bare word must be caught too — and `\b` is what keeps
  // that same bare word from also matching the `judgments.cause` enum literal `'truncated'`.
  for (const [name, raw] of await ours()) {
    const sql = raw.toLowerCase();
    for (const forbidden of ["drop table", "drop column", "truncate"]) {
      assert(
        !new RegExp(`\\b${forbidden}\\b`).test(sql),
        `${name}: migrations are forward-only (${forbidden})`,
      );
    }
  }
});

Deno.test("C2 never creates a table C1 owns; it only alters one", async () => {
  // Ruling R-X-2: `corrections` is C1's, with C1's columns (`ts`, `received_at`, nullable
  // `ours`/`theirs`, and a `kind` that is the NOTE kind). C2 adds two nullable columns and an
  // index and touches nothing else — a second `create table` would be two schemas racing.
  for (const [name, sql] of await ours()) {
    for (
      const owned of [
        "corrections",
        "accounts",
        "entitlements",
        "consents",
        "sources",
        "telemetry_events",
        "issues",
      ]
    ) {
      assert(
        !sql.includes(`create table if not exists ${owned}`) && !sql.includes(`create table public.${owned}`),
        `${name}: ${owned} is C1's table (R-X-2 / R-X-1). Alter it, never create it.`,
      );
    }
  }
  const sql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  assert(sql.includes("alter table public.corrections add column if not exists judgment_id uuid"));
  assert(sql.includes("alter table public.corrections add column if not exists judgment_kind text"));
});

Deno.test("the seeded model pins are the cheapest Haiku-class id, one per kind, with their price and sampling", async () => {
  const sql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  const rows = [...sql.matchAll(/\('(task|event|email)',\s*'anthropic',\s*'([a-z0-9.-]+)'/g)];
  assertEquals(rows.length, 3, "one pinned model per kind (cloud design §5.2)");
  for (const [, , modelId] of rows) {
    // §11 R8: the eval suite picks the model, not taste. This starts at the cheapest tier and is
    // only ever changed by a migration row the eval justifies.
    assertEquals(modelId, "claude-haiku-4-5");
  }
  // The price belongs beside the pin, so a pin change reprices history correctly, and the sampling
  // belongs there too, so a pin above Haiku does not 400 every call.
  assert(sql.includes("usd_per_m_in") && sql.includes("usd_per_m_out"));
  assert(sql.includes("sampling"));
});
