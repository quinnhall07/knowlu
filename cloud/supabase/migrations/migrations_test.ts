// Static pins on the migrations. There is no Docker in this plan, so nothing here applies SQL:
// these are the invariants that would otherwise only be discovered on a project that already has
// rows in it, which is the wrong time to discover them.
import { assert, assertEquals } from "@std/assert";

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

/**
 * A function definition, anywhere: an optional `public.` schema prefix (Postgres's own default
 * schema, and the only one this codebase uses), the name, its parameter list, and — non-greedily —
 * everything up to and through its `AS $tag$ … $tag$` body, whatever the dollar-quote tag is
 * (`$$`, `$fn$`, …; group 3 is the tag and `\3` demands the SAME one close it, so a body that
 * itself contains an unrelated `$$` is never mistaken for the end).
 */
const FUNCTION_DEFINITION = /create\s+(?:or\s+replace\s+)?function\s+(?:public\.)?(\w+)\s*\(([^)]*)\)[\s\S]*?\bas\s+(\$[A-Za-z_]*\$)[\s\S]*?\3/gi;

/**
 * `files`, sorted, from earliest to latest: for every SECURITY DEFINER function creation, a
 * `revoke execute … from …` naming both `anon` and `authenticated` somewhere from THAT file
 * onward (R-C2-E29 fixed it; R-C2-E36 widens where this guard is allowed to look for both the
 * creation and the fix). A `returns trigger` function is exempt BY KIND, not by name or by
 * migration — it cannot be called through PostgREST at all (it takes no ordinary arguments and its
 * return type means nothing outside a trigger context) — and every exempted name is returned so a
 * caller can assert on it directly, rather than the exemption silently swallowing the one case
 * (C1's `handle_new_user`) that proves the rule fires.
 */
export function assertExecuteRevoked(files: Array<[string, string]>): { exemptedTriggers: string[] } {
  const exemptedTriggers: string[] = [];
  for (let i = 0; i < files.length; i++) {
    const [name, sql] = files[i];
    for (const d of sql.matchAll(FUNCTION_DEFINITION)) {
      const fn = d[1];
      if (/returns\s+trigger/i.test(d[0])) {
        exemptedTriggers.push(fn);
        continue;
      }
      if (!/security\s+definer/i.test(d[0])) continue;
      const rest = files.slice(i).map(([, s]) => s).join("\n");
      // A previous migration's own `revoke … from public` may still be sitting right there
      // (forward-only: it is never edited out) — every match counts, not just the first, so a
      // narrower fix-up revoke later in the corpus still satisfies this.
      const revokeRe = new RegExp(
        `revoke\\s+execute\\s+on\\s+function\\s+(?:public\\.)?${fn}\\s*\\([^)]*\\)\\s+from\\s+([^;]+);`,
        "gi",
      );
      const matches = [...rest.matchAll(revokeRe)];
      assert(
        matches.length > 0,
        `${name}: ${fn} is SECURITY DEFINER with no 'revoke execute … from …' in this or a later migration (R-C2-E29)`,
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
  return { exemptedTriggers };
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

Deno.test("every SECURITY DEFINER function in every migration has execute revoked from anon and authenticated", async () => {
  // R-C2-E29 / R-C2-E36: `revoke execute … from public` removes only the PUBLIC entry. Supabase
  // grants execute to `anon` and `authenticated` explicitly on every new function by default, so a
  // SECURITY DEFINER function — which runs with the DEFINING role's privileges, not the caller's —
  // stays reachable over PostgREST with the anon key unless both are named too. Scoped to EVERY
  // migration, not only this stream's own `20260911…` ones: a gap in a later stream's own function
  // is exactly as live a hole as one in this stream's.
  assertExecuteRevoked(await everyMigrationFile());
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
