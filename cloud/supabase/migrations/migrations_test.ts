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

/**
 * A view definition, anywhere: `create [or replace] [materialized] view`, the identifier
 * (§`IDENT`), and — non greedily — everything up to and through the `as` that opens its body.
 * Group 1 is the HEADER ALONE (never the body), because the `security_invoker` test below must
 * read the view's own declared storage option and never a phrase the SELECT merely contains.
 * Group 2 is the name.
 *
 * R2-5: `materialized` is optional so a matview is not invisible to this scan. Supabase grants
 * `select` on a matview to `anon`/`authenticated` the same way it does an ordinary view, and a
 * matview cannot take `with (security_invoker = true)` at all (there is no such storage option for
 * one) — so every matview this corpus ever creates MUST be caught by the revoke branch below, never
 * silently skipped.
 */
const VIEW_DEFINITION = new RegExp(
  `(create\\s+(?:or\\s+replace\\s+)?(?:materialized\\s+)?view\\s+${identPattern("(\\w+)")}[\\s\\S]*?\\bas\\b)`,
  "gi",
);

/** How many times the bare `create [or replace] [materialized] view` phrase appears — the same
 * fail-loud pairing `RAW_FUNCTION_KEYWORD` gives the function scan (R-C2-E37e): a view shape
 * (including a matview) the definition regex cannot parse must never silently vanish from the
 * scan. */
const RAW_VIEW_KEYWORD = /create\s+(?:or\s+replace\s+)?(?:materialized\s+)?view/gi;

/**
 * `files`, sorted, from earliest to latest: every view any migration creates must EITHER declare
 * `with (security_invoker = true)` — so PostgREST reads it with the CALLER's privileges and RLS on
 * the underlying tables applies — OR be named by a `revoke … on … <view> from … anon,
 * authenticated` line in that or a later migration (C2 final review C-1).
 *
 * A `security definer` view (the Postgres DEFAULT, and what `with (security_invoker = false)`
 * spells out) runs its SELECT as the view's OWNER, which bypasses RLS on every table it reads —
 * and Supabase grants `select` to `anon` and `authenticated` on a new view the same way it grants
 * `execute` on a new function. `monthly_spend` (20260911000100) was exactly that gap: no
 * `security_invoker`, no revoke, and `GET /rest/v1/monthly_spend` answered 200 to the anon key
 * while C1's `telemetry_daily` — same shape, but revoked — answered 401. Every view name found is
 * returned, split by which of the two ways it is guarded, so a caller can assert on the real
 * corpus rather than trust an empty scan; `parsed` is the total this run actually parsed.
 */
export function assertViewsGuarded(
  files: Array<[string, string]>,
): { invoker: string[]; revoked: string[]; parsed: number } {
  const stripped = files.map(([name, sql]) => [name, stripLineComments(sql)] as [string, string]);
  const invoker: string[] = [];
  const revoked: string[] = [];
  let parsed = 0;
  for (let i = 0; i < stripped.length; i++) {
    const [name, sql] = stripped[i];
    const rawCount = (sql.match(RAW_VIEW_KEYWORD) ?? []).length;
    const definitions = [...sql.matchAll(VIEW_DEFINITION)];
    assert(
      rawCount === definitions.length,
      `${name}: found ${rawCount} 'create view' occurrence(s) but the definition regex parsed ` +
        `${definitions.length} — a shape it cannot parse. Every view this file creates must be a ` +
        `shape the scan can see, or it is a privilege gap the scan cannot see either.`,
    );
    for (const d of definitions) {
      parsed += 1;
      const header = d[1];
      const view = d[2];
      if (/security_invoker\s*=\s*true/i.test(header)) {
        invoker.push(view);
        continue;
      }
      const rest = stripped.slice(i).map(([, s]) => s).join("\n");
      const revokeRe = new RegExp(
        `revoke\\s+(?:all\\s+privileges|all|select)(?:\\s*\\([^)]*\\))?\\s+on\\s+(?:table\\s+)?` +
          `${identPattern(view)}\\s+from\\s+([^;]+);`,
        "gi",
      );
      const matches = [...rest.matchAll(revokeRe)];
      assert(
        matches.length > 0,
        `${name}: the view ${view} declares no 'with (security_invoker = true)' and has no ` +
          `'revoke … on … from …' in this or a later migration, so it is readable with the anon ` +
          `key (C2 final review C-1)`,
      );
      const ok = matches.some((m) => {
        const from = m[1].toLowerCase();
        return from.includes("anon") && from.includes("authenticated");
      });
      assert(
        ok,
        `${name}: ${view} has no revoke naming both anon and authenticated — 'from public' alone leaves both readable`,
      );
      revoked.push(view);
    }
  }
  return { invoker, revoked, parsed };
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
  // fix 1's re-issue of the same three in 20260911000500, plus the provider swap's `create or
  // replace function export_training_rows` in 20260916000100 (still SECURITY INVOKER,
  // non-writing, so it needs no new revoke — it is still one more definition this scan parses),
  // plus C1b's `create or replace function public.handle_new_user()` in 20260917000100 (a trigger
  // function, so exempt from the revoke by kind — and still one more definition parsed), plus
  // R-C1b-exec-8's metadata trim in 20260922000100: `trimmed_user_metadata` (SQL, non-definer,
  // non-writing — the widened guard does not require its revoke, but the file carries one anyway
  // and that revoke is still one more definition parsed) and `trim_user_metadata` (SECURITY
  // DEFINER, `returns trigger`, so exempt from the revoke by kind exactly as handle_new_user is,
  // and carries a revoke of its own regardless),
  // plus C3's four in 20260912000100 (`sync_ceiling_bytes`, non-writing so no revoke needed;
  // `sync_prune`, writing, revoked from `public, anon, authenticated`; and the two trigger
  // functions `sync_notes_stamp_rev` and `sync_usage_bump`, exempt by kind), plus fix round 2's
  // `create or replace function sync_usage_bump()` in 20260912000200 (the same trigger function
  // re-issued so a DELETE never inserts a usage row — still exempt by kind, still one more
  // definition this scan parses), plus C3′'s own re-issue of `sync_usage_bump()` in
  // 20260912000300 (the amendment's plaintext tables read `body` instead of `ciphertext`; same
  // trigger function, exempt by kind, one more definition parsed), plus C1c's R-C1c-7
  // `create or replace function charge_call` in 20260923000100 (still a writing, non-definer
  // function — revoked again in the same file, and one more definition parsed), plus Gmail
  // connect D14's `create or replace function delete_google_grant` in 20260929000200 (SECURITY
  // DEFINER, revoked from `public, anon, authenticated` again in the same file) —
  // counted by hand against today's corpus: C3′ adds six, C1c one and D14 one, on top of the 21
  // both streams inherited (the C3′ merge of main, PR #13); plus F1's `refund_call` in
  // 20261002000100 (writing, non-definer, revoked from `public, anon, authenticated` in the same
  // file — live proof 2026-10-02); plus email forwarding T1.2's three
  // re-issues in 20261002000200_mail.sql (`export_training_rows`, non-writing; `backfill_correction_judgments`
  // and `promote_rules`, writing, each revoked from `public, anon, authenticated` again in that file),
  // plus T1.3's three in the same file (`delete_mail_address`, SECURITY DEFINER; `charge_allowance`
  // and the re-issued `enforce_budget`, writing; each revoked from `public, anon, authenticated`),
  // plus the T1.3 review's `refund_allowance` (writing, revoked the same way), plus T2g.1's two
  // (`mail_hand_off` and `count_mail_arrival`, writing, each revoked from `public, anon,
  // authenticated`), plus T3e's `expire_mail_work` (writing, revoked the same way).
  assertEquals(parsed, 40, "today's corpus should parse exactly 40 function creations");
});

Deno.test("every view in every migration is either security_invoker or revoked from anon and authenticated", async () => {
  // C2 final review C-1. Same shape as the function guard above and for the same reason: the
  // definer/invoker split decides WHOSE privileges the read runs with, never whether `anon` can
  // reach the object over PostgREST at all — so a `security definer` view (the Postgres default)
  // needs the revoke that a `security_invoker = true` view does not.
  const { invoker, revoked, parsed } = assertViewsGuarded(await everyMigrationFile());
  // Pinned so a view that silently stops being parsed shows up as a number that moved without a
  // reason on the diff: C1's `billing_subscribers` (invoker), `telemetry_daily` and
  // `correction_rates` (revoked, 20260910000400), and C2's `monthly_spend` (revoked,
  // 20260911000900) — counted by hand against today's corpus. R2-5 widened `VIEW_DEFINITION` and
  // `RAW_VIEW_KEYWORD` to see `create materialized view` too; none of the five is a materialized
  // view — today's corpus creates none, not because the scan cannot see one: the synthetic case
  // below proves it can. C3's `sync_limits` (revoked, 20260912000100) is the fifth.
  assertEquals(parsed, 5, "today's corpus should parse exactly 5 view creations");
  // Asserted by name, not merely counted: the exemption must be seen to fire on a real view rather
  // than papering over a scan that never reached one.
  assertEquals(invoker, ["billing_subscribers"]);
  assertEquals(revoked, ["telemetry_daily", "correction_rates", "monthly_spend", "sync_limits"]);
});

Deno.test("a view with neither security_invoker nor a revoke is caught by the view guard", () => {
  // The exact shape `monthly_spend` had before 20260911000900: no storage option, no revoke, and
  // therefore a 200 to the anon key.
  const sql = `
create or replace view leaky_spend as
select account_id, sum(usd) as usd from usage_daily group by 1;
`;
  assertThrows(
    () => assertViewsGuarded([["synthetic-view.sql", sql]]),
    Error,
    "leaky_spend",
  );

  // `from public` alone is not enough: Supabase's own grants to anon/authenticated survive it.
  const publicOnly = `
create view public.half_revoked with (security_invoker = false) as select 1 as one;
revoke all on public.half_revoked from public;
`;
  assertThrows(
    () => assertViewsGuarded([["synthetic-view-public.sql", publicOnly]]),
    Error,
    "anon and authenticated",
  );

  // And a revoke in a LATER migration satisfies it, exactly as the function guard allows.
  const later = assertViewsGuarded([
    ["synthetic-view-a.sql", `create view public.fixed_later with (security_invoker = false) as select 1 as one;`],
    ["synthetic-view-b.sql", `revoke all on public.fixed_later from anon, authenticated;`],
  ]);
  assertEquals(later.revoked, ["fixed_later"]);
});

Deno.test("R2-5: a materialized view is visible to the guard, not invisible to it", () => {
  // A matview cannot declare `with (security_invoker = true)` at all — there is no such storage
  // option for one — so an unrevoked matview must be caught exactly as an unrevoked ordinary view
  // is. Pre-fix, `RAW_VIEW_KEYWORD`/`VIEW_DEFINITION` matched only `create [or replace] view`, so
  // `create materialized view` matched neither: invisible to the scan AND to its own fail-loud
  // parse-count check.
  const leaky = `
create materialized view leaky_matview as
select account_id, sum(usd) as usd from usage_daily group by 1;
`;
  assertThrows(
    () => assertViewsGuarded([["synthetic-matview.sql", leaky]]),
    Error,
    "leaky_matview",
  );

  // And a revoke in a LATER migration satisfies it, exactly as an ordinary view.
  const later = assertViewsGuarded([
    ["synthetic-matview-a.sql", `create materialized view public.fixed_matview as select 1 as one;`],
    ["synthetic-matview-b.sql", `revoke all on public.fixed_matview from anon, authenticated;`],
  ]);
  assertEquals(later.revoked, ["fixed_matview"]);

  // `create or replace` composes with `materialized` in the regex regardless of whether Postgres
  // itself accepts that combination — the scan must parse the shape wherever it appears, not
  // silently assume no migration could ever write it.
  const orReplace = assertViewsGuarded([
    ["synthetic-matview-c.sql", `create or replace materialized view public.replaced_matview as select 1 as one;`],
    ["synthetic-matview-d.sql", `revoke all on public.replaced_matview from anon, authenticated;`],
  ]);
  assertEquals(orReplace.revoked, ["replaced_matview"]);
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

Deno.test("the three cap-store functions are revoked, not marked — R-C2-E48 fix 2 corrected fix 1's marker", async () => {
  // R-C2-E47 fix 1 marked `charge_call`, `record_tokens` and `enforce_budget` (20260911000100) as
  // `-- rpc: authenticated by design`, reasoning they were pre-existing and out of scope. R-C2-E48
  // ruled that was wrong: they are this stream's OWN functions (C2 Task 1), called only through
  // the service role (`_shared/judge_caps.ts`'s `capStore`) with zero app/engine call sites naming
  // a user session — so they belong to the same uniform "no C2 SQL function is reachable from a
  // browser" rule every other C2 function follows, and 20260911000600_caps_privileges.sql revokes
  // them for real instead. This asserts the marker is gone and the guard found a real revoke for
  // all three — not that they are silently exempt either way.
  const { exemptedByMarker } = assertExecuteRevoked(await everyMigrationFile());
  for (const fn of ["charge_call", "record_tokens", "enforce_budget", "refund_call"]) {
    assert(!exemptedByMarker.includes(fn), `${fn} must be revoked, not marker-exempt: ${exemptedByMarker}`);
  }
  // No genuine C0/C1 app-called function ever carried this marker (confirmed by hand against
  // today's corpus), so removing fix 1's three false ones empties the list entirely.
  assertEquals(exemptedByMarker, [], "no function in today's corpus should be marker-exempt");
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

// The Haiku-era seed pin, below, is superseded by the provider swap section further down this
// file (`providerSwapFiles`/`lastModelPinBlocks` and the three Deno.test cases after it) — this
// test still pins what 20260911000100_judgment_service.sql itself seeded, unedited, forward-only.
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

// ---------------------------------------------------------------------------------------------
// The provider swap (Quinn's ruling of 2026-09-16 on R8, option 1): the three kinds re-pinned off
// Anthropic onto OpenRouter, and the training export narrowed further. Forward-only, so these read
// the LAST matching definition across the corpus rather than any one file — the same shape as
// `assertExecuteRevoked`/`assertViewsGuarded` above.
// ---------------------------------------------------------------------------------------------

/** Every migration whose file name contains "provider_swap", sorted earliest to latest. */
async function providerSwapFiles(dir: URL = HERE): Promise<Array<[string, string]>> {
  return (await everyMigrationFile(dir)).filter(([name]) => name.includes("provider_swap"));
}

/** The LAST `update models set … where kind = '<kind>';` block for each kind, scanning `files` in
 *  order (earliest to latest) so a later re-pin overwrites an earlier one — exactly what applying
 *  the migrations in order would do to the row. Group 1 (the block) excludes the `update … set`
 *  and `where …;` wrapper so a caller can search its assignments alone. */
/** Every kind a `models` row may carry: the three device kinds and, since email forwarding T1.2
 *  (spec §5.1, plan P8), the mail kinds. */
type PinKind = "task" | "event" | "email" | "mail_claim" | "mail_resolve" | "mail_digest";

function lastModelPinBlocks(files: Array<[string, string]>): Partial<Record<PinKind, string>> {
  const blocks: Partial<Record<PinKind, string>> = {};
  const re =
    /update\s+(?:public\.)?models\s+set([\s\S]*?)where\s+kind\s*=\s*'(task|event|email|mail_claim|mail_resolve|mail_digest)'\s*;/gi;
  for (const [, sql] of files) {
    for (const m of stripLineComments(sql).matchAll(re)) {
      blocks[m[2] as PinKind] = m[1];
    }
  }
  return blocks;
}

Deno.test("the last provider_swap migration re-pins all three kinds to OpenRouter with their model id, precision, price, prompt version, sampling and route", async () => {
  const files = await providerSwapFiles();
  assert(files.length > 0, "expected at least one migration whose name contains 'provider_swap'");
  const blocks = lastModelPinBlocks(files);

  const expectPin = (
    kind: "task" | "event" | "email",
    modelId: string,
    precision: string,
    promptVersion: string,
    usdIn: string,
    usdOut: string,
    upstream: string,
  ) => {
    const block = blocks[kind];
    assert(block !== undefined, `no 'update models … where kind = '${kind}'' found in any provider_swap migration`);
    const b = block!;
    assert(b.includes("provider = 'openrouter'"), `${kind}: provider must be re-pinned to openrouter: ${b}`);
    assert(b.includes(`model_id = '${modelId}'`), `${kind}: model_id mismatch: ${b}`);
    assert(b.includes(`precision = '${precision}'`), `${kind}: precision mismatch: ${b}`);
    assert(b.includes(`prompt_version = '${promptVersion}'`), `${kind}: prompt_version mismatch: ${b}`);
    assert(b.includes(`usd_per_m_in = ${usdIn}`), `${kind}: usd_per_m_in mismatch: ${b}`);
    assert(b.includes(`usd_per_m_out = ${usdOut}`), `${kind}: usd_per_m_out mismatch: ${b}`);
    assert(
      b.includes(`sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb`),
      // R-PS-6: only reserved keys (temperature, reasoning) ever go in a row's sampling.
      `${kind}: sampling mismatch (must carry only temperature and reasoning): ${b}`,
    );
    assert(
      b.includes(
        `route = '{"order": ["${upstream}"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb`,
      ),
      `${kind}: route mismatch: ${b}`,
    );
    assert(b.includes("since = current_date"), `${kind}: since must be re-stamped to current_date: ${b}`);
  };

  expectPin("task", "ibm-granite/granite-4.2-8b", "bf16 (CoreWeave)", "task-2", "0.10", "0.15", "CoreWeave");
  expectPin("event", "ibm-granite/granite-4.2-8b", "bf16 (CoreWeave)", "event-2", "0.10", "0.15", "CoreWeave");
  expectPin("email", "qwen/qwen3.5-35b-a3b", "fp8 (DeepInfra)", "email-2", "0.14", "1.00", "DeepInfra");
});

Deno.test("the seed's max_tokens (256/256/640) are correct, and no *provider_swap* migration sets max_tokens", async () => {
  // Two separate claims, both checked: (1) the ORIGINAL seed really is 256 for task and event
  // classifications and 640 for email's wider schema; (2) no migration whose name contains
  // "provider_swap" touches max_tokens at all. Together they say the seeded value stands — but
  // this only scans *provider_swap* files, not the full corpus, so it would not catch some OTHER,
  // differently-named future migration setting max_tokens; it is scoped to what this task changes.
  const seedSql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  const seeded = (kind: string): number => {
    const m = seedSql.match(new RegExp(`\\('${kind}',\\s*'[^']*',\\s*'[^']*',\\s*'[^']*',\\s*'[^']*',\\s*(\\d+)\\)`));
    assert(m, `no seed insert row found for kind '${kind}'`);
    return Number(m![1]);
  };
  assertEquals(seeded("task"), 256);
  assertEquals(seeded("event"), 256);
  assertEquals(seeded("email"), 640);

  const blocks = lastModelPinBlocks(await providerSwapFiles());
  for (const kind of ["task", "event", "email"] as const) {
    assert(
      blocks[kind] !== undefined && !/max_tokens/i.test(blocks[kind]!),
      `${kind}: the re-pin must not set max_tokens — the seeded value must stand untouched`,
    );
  }
});

// ---------------------------------------------------------------------------------------------
// Stream J Task T1, CHECKPOINT J-1 (ruled 2026-09-22): `unsure` becomes a fourth event verdict
// word. This migration re-pins only the `event` row's `prompt_version` (the prompt text changed)
// and `grammar_version` (the schema's verdict enum gained a member) — never `task` or `email`.
// ---------------------------------------------------------------------------------------------

Deno.test("the event_unsure migration re-pins only the event row's prompt_version and grammar_version", async () => {
  const sql = await Deno.readTextFile(new URL("20260922120100_event_unsure.sql", HERE));
  assert(/update\s+models\s+set/i.test(sql), "expected an 'update models set …' statement");
  assert(sql.includes("prompt_version = 'event-3'"), sql);
  assert(sql.includes("grammar_version = 'event-2'"), sql);
  assert(sql.includes("since = current_date"), sql);
  assert(/where\s+kind\s*=\s*'event'/i.test(sql), sql);
  assert(!sql.includes("prompt_version = 'task-"), "task must not be touched by this migration");
  assert(!sql.includes("prompt_version = 'email-"), "email must not be touched by this migration");
});

// Stream J final review item 3: T4 + T9's `email-3` migration.
Deno.test("the last gmail_queue_tier_check admits all six email tiers, including completion", async () => {
  const marker = /add\s+constraint\s+gmail_queue_tier_check\s+check\s*\(([^;]*)\)\s*;/gi;
  let last: { name: string; check: string } | undefined;
  for (const [name, sql] of await everyMigrationFile()) {
    for (const m of stripLineComments(sql).matchAll(marker)) last = { name, check: m[1] };
  }
  assert(last !== undefined, "no migration names gmail_queue_tier_check");
  for (const tier of ["task", "borderline", "event", "opportunity", "information", "completion"]) {
    assert(last!.check.includes(`'${tier}'`), `${last!.name}: the last tier check must admit '${tier}'`);
  }
});

Deno.test("the email_due_resolver migration re-pins only the email row, to email-3", async () => {
  const sql = stripLineComments(await Deno.readTextFile(new URL("20260922120200_email_due_resolver.sql", HERE)));
  const updates = [...sql.matchAll(/update\s+models\s+set\s+([^;]*);/gi)].map((m) => m[1]);
  assertEquals(updates.length, 1, "exactly one update of models");
  assert(updates[0].includes("prompt_version = 'email-3'"), updates[0]);
  assert(/where\s+kind\s*=\s*'email'\s*$/i.test(updates[0].trim()), updates[0]);
});

Deno.test("the last definition of export_training_rows excludes the gmail_api, events and forward origins", async () => {
  // Email forwarding T1.2 (spec §5.1, §8): a forwarded mail's judgment never becomes a training row.
  // Moved to the new last definition and naming all three origins, so stronger, never looser.
  // Google's Limited Use policy binds Calendar-API data exactly as it binds Gmail's, and `origin`
  // cannot yet tell a Google-Calendar event from an ICS one — so every `events` row is excluded
  // too, on top of the original `gmail_api` exclusion, until an origin split exists (a C4
  // refinement). Forward-only: this reads the LAST `create or replace function
  // export_training_rows` across every migration, not merely the original one, because a later
  // migration's redefinition is what actually governs the function today.
  const files = await everyMigrationFile();
  const marker = "create or replace function export_training_rows";
  let last: { name: string; body: string } | undefined;
  for (const [name, sql] of files) {
    const idx = sql.lastIndexOf(marker);
    if (idx !== -1) last = { name, body: sql.slice(idx) };
  }
  assert(last !== undefined, "no migration defines export_training_rows");
  assert(
    last!.body.includes("origin not in ('gmail_api', 'events', 'forward')"),
    `${last!.name}: the LAST export_training_rows definition must exclude all three origins ` +
      `(cloud design §5.3/§9, provider swap Task 2, email forwarding spec §5.1)`,
  );
});

Deno.test("the last charge_call doubles the cap on an account's first two judging days (R-C1c-7)", async () => {
  // R-C1c-7: both live proofs of the first day showed the same thing — a new account's first slot
  // judges a whole semester at once (every task, every calendar event, the email backlog), so the
  // steady-state DAILY_CAP (`_shared/judge_caps.ts`, unchanged by this task) cut the run off partway
  // through. Quinn's ruling doubles the effective cap for the account's first two UTC judging days;
  // the SQL alone decides that, never `judge_caps.ts`. Forward-only, the same way the
  // export_training_rows test just above reads the LAST definition: a later migration's
  // `create or replace` is what actually governs the function today, not the original one.
  const files = await everyMigrationFile();
  const marker = "create or replace function charge_call";
  let last: { name: string; body: string } | undefined;
  for (const [name, sql] of files) {
    const stripped = stripLineComments(sql);
    const idx = stripped.lastIndexOf(marker);
    if (idx !== -1) last = { name, body: stripped.slice(idx) };
  }
  assert(last !== undefined, "no migration defines charge_call");
  // Normalised-whitespace substring checks, not a SQL parser (the plan's own instruction) — this
  // guard only needs to prove the shape is present, not that it compiles.
  const normalized = last!.body.replace(/\s+/g, " ").toLowerCase();
  assert(
    normalized.includes("select min(day) into first_day from usage_daily where account_id = p_account"),
    `${last!.name}: charge_call must read the account's first judging day from usage_daily`,
  );
  assert(
    normalized.includes("current_date - 1"),
    `${last!.name}: charge_call must compare the first judging day against current_date - 1`,
  );
  assert(
    normalized.includes("after <= p_cap * 2"),
    `${last!.name}: charge_call must return after <= p_cap * 2 on an account's first two judging days`,
  );
  assert(
    normalized.includes("after <= p_cap;"),
    `${last!.name}: charge_call must return after <= p_cap once the first two judging days have passed`,
  );
  const inserts = normalized.match(/insert into usage_daily/g) ?? [];
  assertEquals(
    inserts.length,
    1,
    `${last!.name}: charge_call must still make exactly one insert into usage_daily`,
  );
  assert(
    normalized.includes("on conflict"),
    `${last!.name}: charge_call's single insert must still be the on-conflict upsert`,
  );
});

// Gmail connect D14 (spec §4.1's migration bullet, §8.3 item 6; Q9 (a) with (ii)): Disconnect also
// deletes the account's `gmail_queue` and `gmail_seen` rows, and keeps its `judgments`. Forward-only,
// read the way `gmail_queue_tier_check`'s test reads its constraint: the LAST definition across every
// migration governs the function today, so that is the one these pins read.
async function lastDeleteGoogleGrant(): Promise<{ name: string; sql: string; header: string; body: string }> {
  let last: { name: string; sql: string; header: string; body: string } | undefined;
  for (const [name, raw] of await everyMigrationFile()) {
    const sql = stripLineComments(raw);
    for (const m of sql.matchAll(FUNCTION_DEFINITION)) {
      if (m[2].toLowerCase() !== "delete_google_grant") continue;
      // m[0] runs from `create` through the terminating `;`; the body is what sits between the
      // opening dollar tag (the end of the header, m[1]) and the same tag closing it.
      const rest = m[0].slice(m[1].length);
      last = { name, sql, header: m[1], body: rest.slice(0, rest.indexOf(m[3])) };
    }
  }
  assert(last !== undefined, "no migration defines delete_google_grant");
  return last!;
}

/** Every `delete from <table> …;` statement in `body`, whitespace-normalised and lower-cased. */
function deletesFrom(body: string, table: string): string[] {
  const re = new RegExp(`delete\\s+from\\s+${identPattern(table)}\\b([^;]*);`, "gi");
  return [...body.matchAll(re)].map((m) => m[1].replace(/\s+/g, " ").trim().toLowerCase());
}

Deno.test("the last delete_google_grant purges the account's gmail_queue and gmail_seen rows", async () => {
  const last = await lastDeleteGoogleGrant();
  for (const table of ["gmail_queue", "gmail_seen", "google_accounts"]) {
    const deletes = deletesFrom(last.body, table);
    assertEquals(deletes.length, 1, `${last.name}: exactly one delete from ${table}`);
    // Filtered on the account and on nothing looser: the where clause is exactly this one term.
    assertEquals(deletes[0], "where account_id = p_account", `${last.name}: ${table}'s delete`);
  }
  // The Vault secret still goes, by the id read off the row before the row is deleted.
  const normalized = last.body.replace(/\s+/g, " ").toLowerCase();
  assert(
    normalized.includes("select secret_id into sid from google_accounts where account_id = p_account"),
    `${last.name}: the secret id is read from the account's row`,
  );
  assert(normalized.includes("delete from vault.secrets where id = sid"), `${last.name}: the Vault secret is deleted`);
});

Deno.test("the last delete_google_grant keeps judgments (Q9 (a)(ii))", async () => {
  const last = await lastDeleteGoogleGrant();
  assert(!/delete\s+from\s+(?:public\.)?judgments/i.test(last.body), `${last.name}: must not delete judgments`);
  assert(!/judgments/i.test(last.body), `${last.name}: the body must not reference judgments at all`);
  assert(/gmail_queue/i.test(last.body), `${last.name}: must be the D14 definition, which names gmail_queue`);
});

Deno.test("the last delete_google_grant is still security definer, service-role only", async () => {
  const last = await lastDeleteGoogleGrant();
  const header = last.header.replace(/\s+/g, " ").toLowerCase();
  assert(header.includes("returns void"), `${last.name}: same signature, returns void`);
  assert(/delete_google_grant\s*\(\s*p_account uuid\s*\)/.test(header), `${last.name}: same signature`);
  assert(header.includes("security definer"), `${last.name}: security definer`);
  assert(header.includes("set search_path = public, vault, extensions"), `${last.name}: pinned search_path`);
  const file = last.sql.replace(/\s+/g, " ").toLowerCase();
  assert(
    file.includes("revoke execute on function delete_google_grant(uuid) from public, anon, authenticated;"),
    `${last.name}: the same file re-issues the revoke`,
  );
  assert(
    file.includes("grant execute on function delete_google_grant(uuid) to service_role;"),
    `${last.name}: the same file re-issues the service-role grant`,
  );
});

Deno.test("the original google migration is unchanged", async () => {
  // SHA-256 of 20260911000200_google.sql as committed at main 4cf3661 (`git show 4cf3661:<path>`),
  // the Gmail connect stream's base. Applied migrations are never edited; D14 is a new file.
  const bytes = await Deno.readFile(new URL("20260911000200_google.sql", HERE));
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hex = [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
  assertEquals(hex, "c86e118dc0bed63875b4785870ccc1bfed15e5299599bd48ce4c2b7d18feeda0");
});

// Email forwarding T1.1 (spec §5.1; test 14's RLS half). The seven mail tables hold a student's mail
// metadata, the address tokens and the claims, so only the service role reaches them, and every row
// goes when its account goes. Textual, like the pins above: nothing here applies SQL (plan P7).
const MAIL_TABLES = [
  "mail_accounts",
  "mail_addresses",
  "mail_batches",
  "mail_canaries",
  "mail_seen",
  "mail_work",
  "mail_claims",
];

/** The one `<stamp>_mail.sql`, comment-stripped. */
async function mailMigration(): Promise<{ name: string; sql: string }> {
  const files = (await everyMigrationFile()).filter(([n]) => /^\d{14}_mail\.sql$/.test(n));
  assertEquals(files.length, 1, "exactly one <stamp>_mail.sql (spec §5.1)");
  return { name: files[0][0], sql: stripLineComments(files[0][1]) };
}

/** Each column or table constraint of `create table … <table> ( … );`, split at top-level commas,
 * whitespace-normalised and lower-cased. */
function tableItems(sql: string, table: string): string[] {
  const head = new RegExp(`create\\s+table\\s+(?:if\\s+not\\s+exists\\s+)?${identPattern(table)}\\s*\\(`, "i");
  const m = head.exec(sql);
  assert(m !== null, `no create table for ${table}`);
  const items: string[] = [];
  let depth = 0;
  let cur = "";
  for (const ch of sql.slice(m.index + m[0].length)) {
    if (ch === "(") depth++;
    if (ch === ")" && depth-- === 0) break;
    if (ch === "," && depth === 0) {
      items.push(cur);
      cur = "";
    } else cur += ch;
  }
  items.push(cur);
  return items.map((s) => s.replace(/\s+/g, " ").trim().toLowerCase()).filter((s) => s.length > 0);
}

/** The column definitions of `table`, by name (table constraints left out). */
function tableColumns(sql: string, table: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const item of tableItems(sql, table)) {
    if (/^(primary key|unique|foreign key|check|constraint)\b/.test(item)) continue;
    out.set(item.split(" ")[0], item);
  }
  return out;
}

Deno.test("every_mail_table_has_rls_on_and_no_anon_or_authenticated_grant", async () => {
  const { name, sql } = await mailMigration();
  const created = [...sql.matchAll(/create\s+table\s+(?:if\s+not\s+exists\s+)?(?:public\.)?(\w+)/gi)]
    .map((m) => m[1].toLowerCase()).sort();
  assertEquals(created, [...MAIL_TABLES].sort(), `${name}: the seven tables of §5.1, and only those`);
  const flat = sql.replace(/\s+/g, " ").toLowerCase();
  for (const t of MAIL_TABLES) {
    assert(flat.includes(`alter table public.${t} enable row level security;`), `${name}: ${t} needs RLS on`);
    const revokes = [...flat.matchAll(new RegExp(`revoke all on (?:table )?public\\.${t} from ([^;]+);`, "g"))];
    assert(
      revokes.some((r) => /\banon\b/.test(r[1]) && /\bauthenticated\b/.test(r[1])),
      `${name}: ${t} must revoke Supabase's default anon and authenticated grants`,
    );
  }
  // No migration, this one or any later one, grants a mail table to a client role or opens a policy
  // on one: only the service role reads them, as `google_accounts` (§5.1).
  for (const [file, raw] of await everyMigrationFile()) {
    const text = stripLineComments(raw).replace(/\s+/g, " ").toLowerCase();
    for (const t of MAIL_TABLES) {
      for (const g of text.matchAll(new RegExp(`grant [^;]*\\bon (?:table )?(?:public\\.)?${t}\\b([^;]*);`, "g"))) {
        assert(!/\bto\b[^;]*\b(anon|authenticated|public)\b/.test(g[1]), `${file}: grants ${t} to a client role`);
      }
      assert(!new RegExp(`create policy [^;]*\\bon (?:public\\.)?${t}\\b`).test(text), `${file}: a policy on ${t}`);
    }
  }
});

Deno.test("every_mail_account_id_cascades_from_accounts", async () => {
  const { name, sql } = await mailMigration();
  const cascades = /\breferences public\.accounts ?\( ?id ?\) on delete cascade\b/;
  // Five tables carry the account; batches and canaries reach it through their mailbox.
  for (const t of ["mail_accounts", "mail_addresses", "mail_seen", "mail_work", "mail_claims"]) {
    const col = tableColumns(sql, t).get("account_id");
    assert(col !== undefined, `${name}: ${t} has no account_id`);
    assert(col.startsWith("account_id uuid "), `${name}: ${t}.account_id is a uuid`);
    assert(/\bnot null\b|\bprimary key\b/.test(col), `${name}: ${t}.account_id is never null`);
    assert(cascades.test(col), `${name}: ${t}.account_id must reference accounts(id) on delete cascade`);
  }
  const viaMailbox = /\breferences public\.mail_addresses ?\( ?id ?\) on delete cascade\b/;
  for (const t of ["mail_batches", "mail_canaries"]) {
    const cols = tableColumns(sql, t);
    assert(!cols.has("account_id"), `${name}: ${t} is scoped by its mailbox (§5.1)`);
    const col = cols.get("address_id");
    assert(col !== undefined && /\bnot null\b/.test(col), `${name}: ${t}.address_id is never null`);
    assert(viaMailbox.test(col!), `${name}: ${t}.address_id must cascade from mail_addresses`);
  }
  // Where a row names both, they are one account's: a mailbox of another account cannot be named.
  for (const t of ["mail_seen", "mail_work", "mail_claims"]) {
    const items = tableItems(sql, t);
    assert(
      items.includes(
        "foreign key (address_id, account_id) references public.mail_addresses (id, account_id) on delete cascade",
      ),
      `${name}: ${t}'s mailbox must belong to its account`,
    );
    assert(/^address_id uuid not null$/.test(tableColumns(sql, t).get("address_id") ?? ""), `${name}: ${t}.address_id`);
  }
  assert(tableItems(sql, "mail_addresses").includes("unique (id, account_id)"), `${name}: the composite key's target`);
  // Nothing in the file reaches another account's data through a looser key.
  assert(!/on delete (set null|no action|restrict)[^,]*account/i.test(sql), `${name}: an account key that does not cascade`);
});

Deno.test("mail_seen_is_keyed_by_account_and_msg_hash_and_token_hmac_is_unique", async () => {
  const { name, sql } = await mailMigration();
  const seen = tableItems(sql, "mail_seen");
  assert(seen.includes("primary key (account_id, msg_hash)"), `${name}: mail_seen's key is (account_id, msg_hash)`);
  const cols = tableColumns(sql, "mail_seen");
  for (const c of ["msg_hash", "fp_hash", "received_minute"]) {
    assert(/\bnot null\b/.test(cols.get(c) ?? ""), `${name}: mail_seen.${c} is never null`);
  }
  const token = tableColumns(sql, "mail_addresses").get("token_hmac") ?? "";
  assert(/^token_hmac text not null unique$/.test(token), `${name}: token_hmac is text, not null and unique: ${token}`);
  const retired = tableColumns(sql, "mail_addresses").get("retired_token_hmac") ?? "";
  assert(/^retired_token_hmac text unique$/.test(retired), `${name}: a retired token is looked up as the live one is`);
});

Deno.test("the_mail_tables_have_section_5_1s_columns_and_words", async () => {
  // Spec §5.1, column for column. Two columns §5.1's table implies rather than names:
  // `mail_addresses.account_id` (its header: every account_id cascades) and the ciphertext/IV pairs.
  const { name, sql } = await mailMigration();
  const want: Record<string, string[]> = {
    mail_accounts: [
      "account_id", "fp_key_ciphertext", "fp_key_iv", "digest", "allowance_opened_at", "allowance_left",
      "allowance_usd", "allowance_until", "mints_30d",
    ],
    mail_addresses: [
      "id", "account_id", "provider", "token_hmac", "token_ciphertext", "token_iv", "retired_token_hmac",
      "retired_until", "mailbox_hint", "forward_hint", "status", "lapsed_since", "reenabled_at", "confirm_until",
      "backfill_since", "last_received_at", "last_canary_sent_at", "last_canary_ok_at", "canaries_today", "unbound",
      "unauthenticated", "expired",
    ],
    mail_batches: [
      "id", "address_id", "phase", "nonce_hmac", "since", "expected", "sent", "received", "opened_at", "sent_at",
      "closed_at",
    ],
    mail_canaries: ["id", "address_id", "nonce_hmac", "sent_at", "arrived_at"],
    mail_seen: ["account_id", "msg_hash", "fp_hash", "received_minute", "address_id", "phase", "seen_at"],
    mail_work: [
      "id", "account_id", "address_id", "batch_id", "s3_key", "phase", "authenticity", "arrived_at", "attempts",
      "next_attempt_at", "msg_hash",
    ],
    mail_claims: [
      "id", "account_id", "address_id", "batch_id", "uid", "message_date", "ordering_date", "sender_name",
      "sender_addr", "phase", "authenticity", "claims", "judgment_id", "queued_at", "delivered_at",
    ],
  };
  for (const t of MAIL_TABLES) {
    assertEquals([...tableColumns(sql, t).keys()].sort(), [...want[t]].sort(), `${name}: ${t}'s columns`);
  }
  const words = (t: string, c: string) => (tableColumns(sql, t).get(c) ?? "").replace(/"/g, "'");
  const oneOf = (t: string, c: string, vals: string[]) =>
    assert(
      words(t, c).includes(`check (${c} in (${vals.map((v) => `'${v}'`).join(", ")}))`),
      `${name}: ${t}.${c} must be one of ${vals.join(", ")}: ${words(t, c)}`,
    );
  oneOf("mail_addresses", "provider", ["m365", "google", "other"]);
  oneOf("mail_addresses", "status", [
    "pending", "confirmed", "active", "quiet", "blocked", "off", "paused_send", "lapsed", "revoked",
  ]);
  oneOf("mail_batches", "phase", ["backfill", "recover"]);
  for (const t of ["mail_seen", "mail_work", "mail_claims"]) oneOf(t, "phase", ["live", "backfill", "recover"]);
  for (const t of ["mail_work", "mail_claims"]) oneOf(t, "authenticity", ["aligned", "unaligned"]);
  assert(/\bdefault false\b/.test(words("mail_accounts", "digest")), `${name}: digest defaults to false (D13)`);
  // PQ6 (a) and BW (T1.3's renames): the allowance's key is a moment, the mailbox's window start a day.
  assertEquals(words("mail_accounts", "allowance_opened_at"), "allowance_opened_at timestamptz", `${name}: PQ6 (a)`);
  assertEquals(words("mail_addresses", "backfill_since"), "backfill_since date", `${name}: BW's window start`);
  // Neither old name survives anywhere in cloud/ (spelt in halves so this file does not match itself).
  const old = new RegExp(`\\b(?:allowance_)?${"term"}_${"start"}\\b`);
  const walk = async (dir: URL): Promise<void> => {
    for await (const e of Deno.readDir(dir)) {
      if (e.name === "node_modules" || e.name.startsWith(".")) continue;
      const at = new URL(e.isDirectory ? `${e.name}/` : e.name, dir);
      if (e.isDirectory) await walk(at);
      else if (/\.(ts|sql|md|toml|json)$/.test(e.name)) {
        assert(!old.test(await Deno.readTextFile(at)), `${at.pathname}: an old allowance or window column name`);
      }
    }
  };
  await walk(new URL("../../", HERE));
  assert(/^claims jsonb not null\b/.test(words("mail_claims", "claims")), `${name}: claims is jsonb`);
  assert(
    /references public\.judgments ?\( ?id ?\) on delete set null/.test(words("mail_claims", "judgment_id")),
    `${name}: a claim outlives nothing it should not, and never blocks a judgment's deletion`,
  );
  // Nothing here holds a body, a subject, a header or a raw token: hashes, words and the claims only.
  for (const t of MAIL_TABLES) {
    for (const c of tableColumns(sql, t).keys()) {
      assert(!/(body|subject|header|^token$|nonce$|^message_id$|^text$)/.test(c), `${name}: ${t}.${c}`);
    }
  }
});

// Email forwarding T1.2 (spec §5.1's "Changes to existing objects", constraint by constraint; test 14's
// constraint half; plan P8). Textual, like every pin above: the words the LAST definition of each
// check admits, read in the order Postgres applies the corpus (file by file, top to bottom).
const DEVICE_KINDS = ["task", "event", "email"];
const MAIL_KINDS = ["mail_claim", "mail_resolve", "mail_digest"];

/** The text between the `(` that ends at `start` and its matching `)`. */
function parenBody(sql: string, start: number): string {
  let depth = 0;
  let out = "";
  for (const ch of sql.slice(start)) {
    if (ch === "(") depth++;
    if (ch === ")" && depth-- === 0) break;
    out += ch;
  }
  return out;
}

/** Where `check (<column> in (…))` on `table` is defined in one comment-stripped file, in text order:
 *  the inline column check of `create table` (Postgres names it `<table>_<column>_check`), a column
 *  added with its check, or a re-issue under that default name. Each entry is [offset, how, list]. */
function checkDefinitions(sql: string, table: string, column: string): Array<[number, string, string]> {
  const t = identPattern(table);
  const inList = `check\\s*\\(\\s*${column}\\s+in\\s*\\(([^)]*)\\)\\s*\\)`;
  const found: Array<[number, string, string]> = [];
  for (const m of sql.matchAll(new RegExp(`create\\s+table\\s+(?:if\\s+not\\s+exists\\s+)?${t}\\s*\\(`, "gi"))) {
    const body = parenBody(sql, m.index! + m[0].length);
    const c = new RegExp(`(?:^|,)\\s*${column}\\s[^,]*?${inList}`, "i").exec(body);
    if (c) found.push([m.index!, "create", c[1]]);
  }
  const added = `alter\\s+table\\s+${t}\\s+add\\s+column\\s+(?:if\\s+not\\s+exists\\s+)?${column}\\s[^;]*?${inList}`;
  for (const m of sql.matchAll(new RegExp(added, "gi"))) found.push([m.index!, "add column", m[1]]);
  const reissued = `alter\\s+table\\s+${t}\\s+add\\s+constraint\\s+${table}_${column}_check\\s+${inList}`;
  for (const m of sql.matchAll(new RegExp(reissued, "gi"))) found.push([m.index!, "add constraint", m[1]]);
  return found.sort((a, b) => a[0] - b[0]);
}

/** The words the corpus's LAST definition of `<table>_<column>_check` admits, and its file. */
async function lastCheckWords(table: string, column: string): Promise<{ name: string; words: string[] }> {
  let last: { name: string; words: string[] } | undefined;
  for (const [name, raw] of await everyMigrationFile()) {
    const defs = checkDefinitions(stripLineComments(raw), table, column);
    if (defs.length > 0) {
      last = { name, words: [...defs[defs.length - 1][2].matchAll(/'([^']*)'/g)].map((m) => m[1]) };
    }
  }
  assert(last !== undefined, `no migration defines ${table}.${column}'s check`);
  return last!;
}

/** A re-issued check must drop the old one first, or the `add` fails on the name it already holds. */
function assertDroppedBeforeReadded(name: string, sql: string, table: string, column: string): void {
  const re = checkDefinitions(sql, table, column).filter(([, how]) => how === "add constraint");
  assertEquals(re.length, 1, `${name}: ${table}_${column}_check is re-issued exactly once`);
  const drop = new RegExp(
    `alter\\s+table\\s+${identPattern(table)}\\s+drop\\s+constraint\\s+if\\s+exists\\s+${table}_${column}_check\\s*;`,
    "i",
  ).exec(sql);
  assert(drop !== null && drop.index < re[0][0], `${name}: ${table}_${column}_check is dropped before it is re-added`);
}

Deno.test("models_usage_daily_and_judgments_accept_the_three_mail_kinds", async () => {
  // Without `usage_daily`, `charge_call` answers 23514, `capStore.charge` reads it as capped, and every
  // claim would be `capped` (spec §5.1); without `models`, no pin and no `monthly_spend` price.
  const { name, sql } = await mailMigration();
  for (const table of ["models", "usage_daily", "judgments"]) {
    const last = await lastCheckWords(table, "kind");
    assertEquals(last.name, name, `${table}.kind: the mail migration re-issues the check`);
    assertEquals([...last.words].sort(), [...DEVICE_KINDS, ...MAIL_KINDS].sort(), `${table}.kind's words`);
    assertDroppedBeforeReadded(name, sql, table, "kind");
  }
});

Deno.test("judgments_origin_accepts_forward", async () => {
  const { name, sql } = await mailMigration();
  const last = await lastCheckWords("judgments", "origin");
  assertEquals(last.name, name, "judgments.origin: the mail migration re-issues the check");
  assertEquals([...last.words].sort(), ["device", "events", "forward", "gmail_api"], "judgments.origin's words");
  assertDroppedBeforeReadded(name, sql, "judgments", "origin");
});

Deno.test("corrections_rules_and_both_eval_tables_still_refuse_mail_kinds", async () => {
  // Deliberately not widened (spec §5.1): a mail judgment never becomes a rule, a correction or an
  // eval row, so a mail kind arriving at any of these is a 23514, never a silent row.
  const { name } = await mailMigration();
  const cases: Array<[string, string]> = [
    ["corrections", "judgment_kind"],
    ["rules", "kind"],
    ["eval_cases", "kind"],
    ["eval_runs", "kind"],
  ];
  for (const [table, column] of cases) {
    const last = await lastCheckWords(table, column);
    assert(last.name !== name, `${table}.${column}: the mail migration must not re-issue it`);
    assertEquals([...last.words].sort(), [...DEVICE_KINDS].sort(), `${table}.${column} admits the device kinds only`);
  }
  for (const [file, raw] of await everyMigrationFile()) {
    const text = stripLineComments(raw);
    for (const [table] of cases) {
      const re = new RegExp(`alter\\s+table\\s+${identPattern(table)}\\s[^;]*\\bmail_`, "i");
      assert(!re.test(text), `${file}: ${table} must never name a mail kind`);
    }
  }
});

/** The LAST definition of function `fn` across the corpus: its file and its comment-stripped text. */
async function lastFunction(fn: string): Promise<{ name: string; text: string }> {
  let last: { name: string; text: string } | undefined;
  for (const [name, raw] of await everyMigrationFile()) {
    for (const m of stripLineComments(raw).matchAll(FUNCTION_DEFINITION)) {
      if (m[2].toLowerCase() === fn) last = { name, text: m[0] };
    }
  }
  assert(last !== undefined, `no migration defines ${fn}`);
  return last!;
}

Deno.test("promote_rules_and_the_corrections_backfill_pass_over_forward_judgments", async () => {
  // Otherwise `judgment_features` would offer a `mail_claim` feature and the `rules` insert would abort
  // the whole nightly promotion for every account; the back-fill would write a mail kind into
  // `corrections.judgment_kind` and abort the same nightly job (spec §5.1).
  const { name, sql } = await mailMigration();
  for (const fn of ["promote_rules", "backfill_correction_judgments"]) {
    const last = await lastFunction(fn);
    assertEquals(last.name, name, `${fn}: the mail migration re-issues it`);
    const aliases = [...last.text.matchAll(/\b(?:public\.)?judgments\s+(\w+)/gi)].map((m) => m[1]);
    assert(aliases.length > 0, `${fn} reads judgments`);
    const filters = [...last.text.matchAll(/\b(\w+)\.origin\s*<>\s*'forward'/gi)].map((m) => m[1]);
    assertEquals(filters.sort(), [...aliases].sort(), `${fn}: every read of judgments passes over 'forward'`);
    assert(!/security\s+definer/i.test(last.text), `${fn} stays security invoker`);
    const revoke = new RegExp(
      `revoke\\s+execute\\s+on\\s+function\\s+public\\.${fn}\\(\\)\\s+from\\s+public,\\s*anon,\\s*authenticated;`,
      "i",
    );
    assert(revoke.test(sql), `${name}: ${fn} is revoked again beside its re-issue`);
    assert(new RegExp(`grant\\s+execute\\s+on\\s+function\\s+public\\.${fn}\\(\\)\\s+to\\s+service_role;`, "i").test(sql));
  }
});

const PIN_KEYS = ["provider", "model_id", "precision", "usd_per_m_in", "usd_per_m_out", "sampling", "route"];

/** A kind's pin as the corpus leaves it: each key's value from the LAST `update models` that sets it,
 *  folded file by file, so a later re-pin of one key (email-3's prompt version) keeps the others. */
function currentPin(files: Array<[string, string]>, kind: PinKind): Record<string, string> {
  const pin: Record<string, string> = {};
  for (const file of files) {
    const block = lastModelPinBlocks([file])[kind];
    if (block === undefined) continue;
    for (const key of PIN_KEYS) {
      const m = new RegExp(`\\b${key}\\s*=\\s*('(?:[^']|'')*'(?:::jsonb)?|[0-9.]+)`, "i").exec(block);
      if (m) pin[key] = m[1];
    }
  }
  return pin;
}

Deno.test("mail_claim_and_mail_resolve_are_pinned_provisionally_to_the_email_models_route", async () => {
  // Plan P8: email's current model, route and zero-retention provider, marked provisional in a SQL
  // comment; test 17's live check (T18a) confirms each pin or a one-statement migration re-pins it.
  const files = await everyMigrationFile();
  const { name, sql } = await mailMigration();
  const raw = files.find(([n]) => n === name)![1];
  const email = currentPin(files, "email");
  assertEquals(Object.keys(email).sort(), [...PIN_KEYS].sort(), "email's pin is fully read");
  const inserts = [...sql.matchAll(/insert\s+into\s+(?:public\.)?models\s*\(([^)]*)\)\s*values([\s\S]*?);/gi)];
  assertEquals(inserts.length, 1, `${name}: one insert into models`);
  assert(/on\s+conflict\s*\(\s*kind\s*\)\s*do\s+nothing\s*$/i.test(inserts[0][2]), `${name}: a re-run inserts nothing`);
  const blocks = lastModelPinBlocks([[name, raw]]);
  for (const kind of ["mail_claim", "mail_resolve"] as const) {
    assert(new RegExp(`\\(\\s*'${kind}'\\s*,`).test(inserts[0][2]), `${name}: a models row for ${kind}`);
    assert(blocks[kind] !== undefined, `${name}: ${kind} is pinned by its own update`);
    assert(/since\s*=\s*current_date/i.test(blocks[kind]!), `${kind}: since is stamped`);
    const pin = currentPin(files, kind);
    for (const key of PIN_KEYS) assertEquals(pin[key], email[key], `${kind}.${key} is email's`);
    assert(pin.route.includes(`"zdr": true`) && pin.route.includes(`"allow_fallbacks": false`), `${kind}: ZDR only`);
  }
  // `mail_digest` gets a row only if T16 is built.
  for (const [file, text] of files) {
    const code = stripLineComments(text);
    for (const m of code.matchAll(/insert\s+into\s+(?:public\.)?models\b[^;]*;/gi)) {
      assert(!m[0].includes("'mail_digest'"), `${file}: no models row for mail_digest until T16`);
    }
    assert(lastModelPinBlocks([[file, text]]).mail_digest === undefined, `${file}: mail_digest is not pinned`);
  }
  // The comment block directly above the insert says so, not merely some line of the file.
  const lines = raw.split("\n");
  let at = lines.findIndex((l) => /^\s*insert\s+into\s+(?:public\.)?models\b/i.test(l));
  assert(at > 0, `${name}: the models insert starts a line`);
  const above: string[] = [];
  while (--at >= 0 && lines[at].trim().startsWith("--")) above.unshift(lines[at]);
  assert(/\bprovisional\b/i.test(above.join("\n")), `${name}: the pins are marked provisional in a SQL comment`);
});

// Email forwarding T1.3 (spec §5.1's RPCs and cron, test 14's RPC half and 11's database half; D17,
// D23 as amended 2026-10-02 by PQ6 (a); plan P7). Textual: the behaviour is smoke-tested on staging.

/** A definition's header (through `as $tag$`) and body (inside the tags), whitespace-normalised and
 *  lower-cased, from the LAST definition of `fn`, which must be the mail migration's. */
async function mailFunction(fn: string): Promise<{ name: string; sql: string; header: string; body: string }> {
  const { name, sql } = await mailMigration();
  const last = await lastFunction(fn);
  assertEquals(last.name, name, `${fn}: the mail migration's definition governs`);
  const m = new RegExp(FUNCTION_DEFINITION.source, "i").exec(last.text)!;
  const rest = m[0].slice(m[1].length);
  const flat = (s: string) => s.replace(/\s+/g, " ").trim().toLowerCase();
  return { name, sql: flat(sql), header: flat(m[1]), body: flat(rest.slice(0, rest.indexOf(m[3]))) };
}

function assertServiceRoleOnly(f: { name: string; sql: string }, fn: string, args: string): void {
  assert(
    f.sql.includes(`revoke execute on function public.${fn}(${args}) from public, anon, authenticated;`),
    `${f.name}: ${fn} is revoked from every client role`,
  );
  assert(f.sql.includes(`grant execute on function public.${fn}(${args}) to service_role;`), `${f.name}: ${fn}`);
}

Deno.test("delete_mail_address_revokes_then_deletes_one_mailboxs_rows_delivered_claims_included", async () => {
  const f = await mailFunction("delete_mail_address");
  assert(/delete_mail_address\s*\(\s*p_address uuid\s*\)/.test(f.header), `${f.name}: delete_mail_address(p_address uuid)`);
  assert(f.header.includes("returns void"), `${f.name}: returns void`);
  assert(f.header.includes("security definer"), `${f.name}: security definer (spec §5.1)`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "delete_mail_address", "uuid");
  // Revoke first, then each child table, filtered on the one mailbox and nothing else (so delivered
  // claims go too), then the row itself.
  const steps = [
    "update public.mail_addresses set status = 'revoked' where id = p_address;",
    "delete from public.mail_seen where address_id = p_address;",
    "delete from public.mail_work where address_id = p_address;",
    "delete from public.mail_claims where address_id = p_address;",
    "delete from public.mail_batches where address_id = p_address;",
    "delete from public.mail_canaries where address_id = p_address;",
    "delete from public.mail_addresses where id = p_address;",
  ];
  let at = -1;
  for (const s of steps) {
    const next = f.body.indexOf(s);
    assert(next > at, `${f.name}: '${s}' in order (revoke, children, then the row)`);
    at = next;
  }
  const writes = f.body.match(/\b(insert|update|delete)\b/g) ?? [];
  assertEquals(writes.length, steps.length, `${f.name}: no other write`);
  assert(!/delivered_at|mail_accounts|judgments|account_id/.test(f.body), `${f.name}: one mailbox, never the account`);
});

// The opening test of PQ6 (a): no allowance yet, or the last one opened 120 days or more ago.
const OPENS = "(allowance_opened_at is null or allowance_opened_at <= now() - interval '120 days')";

Deno.test("charge_allowance_is_one_statement_like_charge_call", async () => {
  const f = await mailFunction("charge_allowance");
  assert(/charge_allowance\s*\(\s*p_account uuid\s*\)/.test(f.header), `${f.name}: charge_allowance(p_account uuid)`);
  assert(f.header.includes("returns boolean"), `${f.name}: answers charged or not, as charge_call`);
  assert(!f.header.includes("security definer"), `${f.name}: security invoker, as charge_call`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "charge_allowance", "uuid");
  // One statement: two concurrent charges serialise on the row, so neither can open twice or overdraw.
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 1, `${f.name}: one write`);
  assertEquals((f.body.match(/\bselect\b/g) ?? []).length, 0, `${f.name}: no read before the write`);
  assert(f.body.includes("update public.mail_accounts set"), `${f.name}: the write is mail_accounts'`);
  // Opening sets the period's start, 14 days, $1.00 and 2,000 judgments; then the one charge.
  for (const set of [
    `allowance_opened_at = case when ${OPENS} then now() else allowance_opened_at end`,
    `allowance_until = case when ${OPENS} then now() + interval '14 days' else allowance_until end`,
    `allowance_usd = case when ${OPENS} then 1.00 else allowance_usd end`,
    `allowance_left = case when ${OPENS} then 2000 else allowance_left end - 1`,
  ]) assert(f.body.includes(set), `${f.name}: ${set}`);
  // Otherwise only what is left, and nothing after allowance_until.
  assert(
    f.body.includes(`where account_id = p_account and (${OPENS} or (allowance_until > now() and allowance_left > 0))`),
    `${f.name}: charges only on opening, or while the allowance is open and not spent`,
  );
  assert(f.body.includes("returning true into charged;"), `${f.name}: charged only when the row was written`);
  assert(f.body.includes("return coalesce(charged, false);"), `${f.name}: no row written answers false`);
});

Deno.test("refund_allowance_gives_one_judgment_back_only_while_the_allowance_is_open", async () => {
  // Review of T1.3, after F1 (20261002000100_refund_failed_call.sql): `charge_allowance` charges
  // before the call, so without a refund a provider outage during a first backfill would spend the
  // whole allowance on calls the model never answered, and the backfill would trickle at the daily
  // cap for 120 days: the outcome D23 exists to prevent.
  const f = await mailFunction("refund_allowance");
  assert(/refund_allowance\s*\(\s*p_account uuid\s*\)/.test(f.header), `${f.name}: refund_allowance(p_account uuid)`);
  assert(f.header.includes("returns boolean"), `${f.name}: answers refunded or not, as refund_call`);
  assert(!f.header.includes("security definer"), `${f.name}: security invoker, as charge_allowance`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "refund_allowance", "uuid");
  // One guarded statement: concurrent refunds and charges serialise on the row, and the `where` is
  // re-checked on the locked row, so the allowance never passes 2,000.
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 1, `${f.name}: one write`);
  assertEquals((f.body.match(/\bselect\b/g) ?? []).length, 0, `${f.name}: no read before the write`);
  assert(
    f.body.includes(
      "update public.mail_accounts set allowance_left = allowance_left + 1 " +
        "where account_id = p_account and allowance_until > now() and allowance_left < 2000",
    ),
    `${f.name}: one judgment back, only inside allowance_until and never above 2,000`,
  );
  // It never opens, extends or re-prices an allowance: only `allowance_left` is set.
  assert(!/allowance_opened_at|allowance_usd|allowance_until =|case when/.test(f.body), `${f.name}: sets nothing else`);
  assert(f.body.includes("returning true into done;"), `${f.name}: refunded only when the row was written`);
  assert(f.body.includes("return coalesce(done, false);"), `${f.name}: no row written answers false`);
});

Deno.test("the_allowance_period_is_keyed_on_allowance_opened_at_and_never_on_a_mailbox", async () => {
  const charge = await mailFunction("charge_allowance");
  const budget = await mailFunction("enforce_budget");
  const refund = await mailFunction("refund_allowance");
  for (const f of [charge, budget, refund]) {
    assert(!/mail_addresses|mail_batches|backfill_since|address/.test(f.body), `${f.name}: reads no mailbox`);
  }
  assert(charge.body.includes(OPENS), `${charge.name}: the period opens on allowance_opened_at alone`);
  // Disconnect deletes nothing the opening test reads.
  const del = await mailFunction("delete_mail_address");
  assert(!/mail_accounts|allowance/.test(del.body), `${del.name}: Disconnect never touches the allowance`);
});

Deno.test("enforce_budget_raises_the_ceiling_by_the_allowance_only_while_it_is_open", async () => {
  const f = await mailFunction("enforce_budget");
  assert(/enforce_budget\s*\(\s*p_account uuid, p_ceiling numeric\s*\)/.test(f.header), `${f.name}: same signature`);
  assert(f.header.includes("returns boolean") && !f.header.includes("security definer"), `${f.name}: as before`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "enforce_budget", "uuid, numeric");
  assert(
    f.body.includes(
      "select coalesce(sum(allowance_usd), 0) into raised from public.mail_accounts " +
        "where account_id = p_account and allowance_until > now();",
    ),
    `${f.name}: the allowance's dollars, only inside its 14 days`,
  );
  assert(f.body.includes("if spent < p_ceiling + raised then return true;"), `${f.name}: the raised ceiling`);
  assert(!/spent < p_ceiling then/.test(f.body), `${f.name}: no comparison against the bare ceiling`);
  assert(
    f.body.includes("from public.monthly_spend where account_id = p_account and month = date_trunc('month', current_date)"),
    `${f.name}: still this month's spend, allowance calls' tokens included`,
  );
  assertEquals((f.body.match(/insert into public\.budget_alerts/g) ?? []).length, 1, `${f.name}: one alert insert`);
});

/** Every `cron.schedule('<job>', '<when>', $tag$ … $tag$)` in `sql`: name, schedule and command. */
function cronJobs(sql: string): Array<{ job: string; when: string; command: string }> {
  const re = /cron\.schedule\(\s*'([^']+)'\s*,\s*'([^']+)'\s*,\s*(\$\w*\$)([\s\S]*?)\3\s*\)/gi;
  return [...sql.matchAll(re)].map((m) => ({ job: m[1], when: m[2], command: m[4].replace(/\s+/g, " ").trim() }));
}

Deno.test("the_mail_cron_jobs_use_the_vault_token", async () => {
  // Spec §5.1's cron and §6.3's auth; plan P7 (the sweep is a mode of mail-process). As the billing
  // job's guard (20260910000700): the URL and the token come from Vault, a missing one raises, and
  // the request carries the token in `x-knowlu-job-token`, never a literal.
  const { name, sql } = await mailMigration();
  const jobs = cronJobs(sql);
  const want: Record<string, { when: RegExp; url: string; body: string }> = {
    "knowlu-mail-process": { when: /^\* \* \* \* \*$/, url: "mail_process_url", body: "'{}'::jsonb" },
    "knowlu-mail-canary": { when: /^\d{1,2} \* \* \* \*$/, url: "mail_canary_url", body: "'{}'::jsonb" },
    "knowlu-mail-sweep": { when: /^\d{1,2} \d{1,2} \* \* \*$/, url: "mail_process_url", body: `'{"mode": "sweep"}'::jsonb` },
  };
  assertEquals(jobs.map((j) => j.job).sort(), Object.keys(want).sort(), `${name}: the three mail jobs`);
  for (const j of jobs) {
    const w = want[j.job];
    assert(w.when.test(j.when), `${name}: ${j.job} runs at '${j.when}'`);
    const c = j.command.toLowerCase();
    const secret = (s: string) =>
      `(select decrypted_secret from vault.decrypted_secrets where name = '${s}' order by created_at desc limit 1)`;
    assert(c.includes(`v_url text := ${secret(w.url)};`), `${name}: ${j.job}'s URL is Vault's ${w.url}`);
    assert(c.includes(`v_token text := ${secret("mail_jobs_token")};`), `${name}: ${j.job}'s token is Vault's`);
    assert(c.includes("if v_url is null or v_token is null then raise exception"), `${name}: ${j.job} fails loudly`);
    assert(c.includes("'x-knowlu-job-token', v_token"), `${name}: ${j.job} sends the Vault token`);
    assert(c.includes("perform net.http_post("), `${name}: ${j.job} calls through pg_net`);
    assert(c.includes(`body := ${w.body}`), `${name}: ${j.job}'s body is ${w.body}`);
    assert(/timeout_milliseconds := \d+/.test(c), `${name}: ${j.job} waits for the function`);
    assertEquals((c.match(/'x-knowlu-job-token'/g) ?? []).length, 1, `${name}: ${j.job}: one token header`);
  }
  // The nightly sweep shares no minute with another nightly job in the corpus.
  const sweep = jobs.find((j) => j.job === "knowlu-mail-sweep")!.when;
  for (const [file, raw] of await everyMigrationFile()) {
    for (const j of cronJobs(stripLineComments(raw))) {
      if (j.job !== "knowlu-mail-sweep") assert(j.when !== sweep, `${file}: ${j.job} shares the sweep's '${sweep}'`);
    }
  }
});

// Email forwarding T2g.1 (spec §4.6 steps 5 and 8, D8): intake's two writes PostgREST cannot make alone.

Deno.test("mail_hand_off_writes_the_seen_row_then_the_work_row_in_one_transaction", async () => {
  // Spec §4.6 step 8: only after the put, the `mail_work` and `mail_seen` rows in one transaction. A
  // function body is one transaction: a failed work insert takes the seen row back, and a concurrent
  // delivery of the same Message-ID loses on the primary key and writes nothing.
  const f = await mailFunction("mail_hand_off");
  const args = "p_account uuid, p_address uuid, p_batch uuid, p_s3_key text, p_phase text, " +
    "p_authenticity text, p_msg_hash text, p_fp_hash text, p_received_minute timestamptz";
  assert(f.header.includes(`mail_hand_off( ${args} )`) || f.header.includes(`mail_hand_off(${args})`), f.header);
  assert(f.header.includes("returns boolean"), `${f.name}: answers queued or duplicate`);
  assert(!f.header.includes("security definer"), `${f.name}: security invoker, as charge_call`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "mail_hand_off", "uuid, uuid, uuid, text, text, text, text, text, timestamptz");
  const steps = [
    // Checkpoint B (minor): only a live mailbox takes work. The share lock holds Disconnect's `revoke` until
    // this transaction ends, so its `mail_work` key read sees this row; once `revoked`, nothing is written,
    // the caller deletes its put, and no object is left with nothing pointing at it.
    "perform 1 from public.mail_addresses where id = p_address and account_id = p_account " +
    "and status <> 'revoked' for share;",
    "if not found then return false; end if;",
    "insert into public.mail_seen (account_id, msg_hash, fp_hash, received_minute, address_id, phase) " +
    "values (p_account, p_msg_hash, p_fp_hash, date_trunc('minute', p_received_minute), p_address, p_phase) " +
    "on conflict (account_id, msg_hash) do nothing returning true into fresh;",
    "if fresh is null then return false; end if;",
    "insert into public.mail_work (account_id, address_id, batch_id, s3_key, phase, authenticity, msg_hash) " +
    "values (p_account, p_address, p_batch, p_s3_key, p_phase, p_authenticity, p_msg_hash);",
    "return true;",
  ];
  let at = -1;
  for (const s of steps) {
    const next = f.body.indexOf(s);
    assert(next > at, `${f.name}: '${s}' in order (live mailbox, seen, the duplicate's way out, work)`);
    at = next;
  }
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 2, `${f.name}: the two inserts only`);
});

Deno.test("mail_claims_holds_one_row_per_mailbox_and_uid", async () => {
  // Checkpoint B (minor): `mail-process` queues before it deletes the work row, so a failed delete runs the
  // item again. The key makes the second queue a no-op (`on_conflict=address_id,uid`, ignore-duplicates).
  const { name, sql } = await mailMigration();
  const flat = sql.replace(/\s+/g, " ").toLowerCase();
  assert(
    flat.includes("create unique index if not exists mail_claims_uid on public.mail_claims (address_id, uid);"),
    `${name}: mail_claims is unique on (address_id, uid), a whole-table index PostgREST's on_conflict can name`,
  );
});

Deno.test("count_mail_arrival_adds_one_to_one_mailboxs_unbound_or_unauthenticated", async () => {
  // Spec §4.6 steps 5 and 6: unbound and unauthenticated mail is counted on the address, by one
  // statement, so two deliveries at once both count.
  const f = await mailFunction("count_mail_arrival");
  assert(/count_mail_arrival\s*\(\s*p_address uuid, p_counter text\s*\)/.test(f.header), f.header);
  assert(f.header.includes("returns void") && !f.header.includes("security definer"), `${f.name}: invoker`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "count_mail_arrival", "uuid, text");
  assert(
    f.body.includes(
      "update public.mail_addresses set " +
        "unbound = unbound + (p_counter = 'unbound')::int, " +
        "unauthenticated = unauthenticated + (p_counter = 'unauthenticated')::int " +
        "where id = p_address;",
    ),
    `${f.name}: one statement, one mailbox, the named counter only`,
  );
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 1, `${f.name}: one write`);
});

// Email forwarding T3e (spec §4.7 step 5, D8, test 7's expiry half): an item whose object expired first
// is counted `expired` and loses its `mail_seen` row, so recover's re-forward of it is accepted.

Deno.test("expire_mail_work_deletes_the_work_row_and_its_seen_row_and_counts_expired_in_one_transaction", async () => {
  const f = await mailFunction("expire_mail_work");
  assert(/expire_mail_work\s*\(\s*p_work bigint\s*\)/.test(f.header), `${f.name}: expire_mail_work(p_work bigint)`);
  assert(f.header.includes("returns boolean"), `${f.name}: answers expired, or already gone`);
  assert(!f.header.includes("security definer"), `${f.name}: security invoker, as mail_hand_off`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "expire_mail_work", "bigint");
  // The work row names its seen row by the keyed hash intake wrote into both (`mail_hand_off`).
  const steps = [
    "delete from public.mail_work where id = p_work returning account_id, address_id, msg_hash into gone;",
    "if not found then return false; end if;",
    "delete from public.mail_seen where account_id = gone.account_id and msg_hash = gone.msg_hash;",
    "update public.mail_addresses set expired = expired + 1 where id = gone.address_id;",
    "return true;",
  ];
  let at = -1;
  for (const s of steps) {
    const next = f.body.indexOf(s);
    assert(next > at, `${f.name}: '${s}' in order (work, the second caller's way out, seen, the count)`);
    at = next;
  }
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 3, `${f.name}: the three writes only`);
});

Deno.test("F1: refund_call gives one call back in one guarded statement, bounded, and is revoked", async () => {
  // Live proof 2026-10-02: a dead provider key charged 160 failed event judgments to one account.
  // `refund_call` gives a failed call back. What this pins, since nothing here runs SQL:
  //   * it is the LATEST migration's own file, stamped after every other one (forward-only);
  //   * one `update` (row-locked, so two concurrent refunds cannot both read the same `calls`),
  //     never an insert — a refund can never create a usage row or push `calls` below zero;
  //   * it is bounded by `refunds`, so a hot loop of failing calls is still stopped by the day;
  //   * it never touches `in_tokens`/`out_tokens`, which `monthly_spend` prices.
  const files = await everyMigrationFile();
  const [name, sql] = files.filter(([n]) => n.includes("refund_failed_call"))[0] ?? ["", ""];
  assert(name !== "", "the refund migration exists");
  const code = stripLineComments(sql);
  assert(/add\s+column\s+if\s+not\s+exists\s+refunds\s+integer\s+not\s+null\s+default\s+0/i.test(code));
  assert(/create\s+or\s+replace\s+function\s+refund_call\s*\(\s*p_account\s+uuid\s*,\s*p_kind\s+text\s*,\s*p_cap\s+integer\s*\)/i.test(code));
  assert(/update\s+usage_daily/i.test(code), "a refund is an update");
  assert(!/insert\s+into/i.test(code), "a refund never inserts");
  assert(/greatest\s*\(\s*u?\.?calls\s*-\s*1\s*,\s*0\s*\)/i.test(code), "calls never goes below 0");
  assert(/calls\s*>\s*0/i.test(code), "nothing to refund, nothing counted as refunded");
  assert(/refunds\s*<\s*allowance/i.test(code), "refunds are bounded by the day's allowance");
  assert(!/in_tokens|out_tokens/i.test(code), "the tokens a call spent are never refunded");
  assert(!/security\s+definer/i.test(code));
  assert(/revoke\s+execute\s+on\s+function\s+public\.refund_call\(uuid,\s*text,\s*integer\)\s+from\s+public,\s*anon,\s*authenticated/i.test(code));
  assert(/grant\s+execute\s+on\s+function\s+public\.refund_call\(uuid,\s*text,\s*integer\)\s+to\s+service_role/i.test(code));
});
