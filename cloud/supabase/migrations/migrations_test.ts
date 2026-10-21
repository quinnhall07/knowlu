// Static pins on the migrations. There is no Docker in this plan, so nothing here applies SQL:
// these are the invariants that would otherwise only be discovered on a project that already has
// rows in it, which is the wrong time to discover them.
import { assert, assertEquals, assertNotEquals, assertThrows } from "@std/assert";

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
 * spell the same function differently still match (R-C2-E37c). The schema may be `public` or
 * `admin` (friends-testers §8.7) and nothing wider, so a function in a third schema still fails the
 * parse-count check loud. */
function identPattern(name: string): string {
  return `(?:"?(?:public|admin)"?\\s*\\.\\s*)?"?${name}"?`;
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
  // authenticated`), plus T3e's `expire_mail_work` (writing, revoked the same way), plus L3 T1's
  // two in `<stamp>_event_allowance.sql` (`charge_judge_allowance` and `refund_judge_allowance`,
  // writing, non-definer, each revoked from `public, anon, authenticated`), plus the email cloud
  // follow-ups' two in <stamp>_mail_follow_ups.sql (`count_batch_receipt` and
  // `mail_mark_lapsed_from`, writing, invoker, each revoked from `public, anon, authenticated`),
  // plus L8 S2c1's three in <stamp>_testers.sql (`admin.comp_grant`, `admin.comp_extend`,
  // `admin.comp_end`; writing, each revoked from `public, anon, authenticated, service_role`), plus
  // L8 S2c2's `admin.comp_sweep` in the same file (writing, revoked the same way), plus L8 S2d's
  // `public.count_route` in the same file (writing, revoked from `public, anon, authenticated`,
  // granted to `service_role`). Recount at the L8 merge of main (2026-10-20): 40 before the three
  // lanes, + L3's 2 + the follow-ups' 2 + L8's 5 = 49. Plus the S6 follow-up's `create or replace` of
  // `admin.comp_grant` and `admin.comp_extend` in <stamp>_comp_guards.sql (writing, each revoked again
  // from `public, anon, authenticated, service_role` in that file): 51.
  assertEquals(parsed, 51, "today's corpus should parse exactly 51 function creations");
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
  // below proves it can. C3's `sync_limits` (revoked, 20260912000100) is the fifth. L8's
  // `effective_entitlements` (revoked, <stamp>_testers.sql, friends-testers S2b) is the sixth,
  // `admin.testers` (revoked, the same file, S2e1) the seventh, and `admin.tester_usage_daily` and
  // `admin.tester_spend_monthly` (revoked, the same file, S2e2) the eighth and ninth. The S6
  // follow-up's `create or replace view admin.testers` (<stamp>_comp_guards.sql, revoked again in that
  // file) is the tenth, so `testers` is listed twice.
  assertEquals(parsed, 10, "today's corpus should parse exactly 10 view creations");
  // Asserted by name, not merely counted: the exemption must be seen to fire on a real view rather
  // than papering over a scan that never reached one.
  assertEquals(invoker, ["billing_subscribers"]);
  assertEquals(revoked, ["telemetry_daily", "correction_rates", "monthly_spend", "sync_limits", "effective_entitlements", "testers", "tester_usage_daily", "tester_spend_monthly", "testers"]);
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
  for (const fn of ["charge_call", "record_tokens", "enforce_budget", "refund_call", "charge_judge_allowance", "refund_judge_allowance"]) {
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

const PIN_KINDS: readonly PinKind[] = ["task", "event", "email", "mail_claim", "mail_resolve", "mail_digest"];

/** A `set` list as one kind sees it: each `<key> = case kind when '<k>' then <v> … [else <v>] end`
 *  becomes `<key> = <v>` for that kind (`null` when no arm names it and there is no `else`). */
function resolveCaseKind(set: string, kind: PinKind): string {
  const value = String.raw`'(?:[^']|'')*'(?:::\w+)?|[\w.]+`;
  const arm = new RegExp(String.raw`when\s+'(\w+)'\s+then\s+(${value})\s*`, "gi");
  const kase = new RegExp(String.raw`\bcase\s+kind\s+((?:when\s+'\w+'\s+then\s+(?:${value})\s*)+)(?:else\s+(${value})\s*)?end\b`, "gi");
  return set.replace(kase, (_, arms: string, otherwise?: string) => {
    for (const a of arms.matchAll(arm)) if (a[1] === kind) return a[2];
    return otherwise ?? "null";
  });
}

/** Also reads the several-kinds form, `update models set … where kind in ('<kind>', …);`: the
 *  statement applies to every listed kind, with any `case kind when …` value resolved per kind. */
function lastModelPinBlocks(files: Array<[string, string]>): Partial<Record<PinKind, string>> {
  const blocks: Partial<Record<PinKind, string>> = {};
  const re =
    /update\s+(?:public\.)?models\s+set([\s\S]*?)where\s+kind\s*(?:=\s*'(task|event|email|mail_claim|mail_resolve|mail_digest)'|in\s*\(([^)]*)\))\s*;/gi;
  for (const [, sql] of files) {
    for (const m of stripLineComments(sql).matchAll(re)) {
      if (m[2] !== undefined) {
        blocks[m[2] as PinKind] = m[1];
        continue;
      }
      for (const k of m[3].matchAll(/'(\w+)'/g)) {
        const kind = k[1] as PinKind;
        if (PIN_KINDS.includes(kind)) blocks[kind] = resolveCaseKind(m[1], kind);
      }
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

Deno.test("the mail migration is unchanged", async () => {
  // SHA-256 of 20261002000200_mail.sql as committed at 87bf8a6 (`git show 87bf8a6:<path>`), its
  // last change. Applied on staging; never edited; a change is a new file.
  const bytes = await Deno.readFile(new URL("20261002000200_mail.sql", HERE));
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hex = [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
  assertEquals(hex, "d62c59cb0352f6e6eb0851388017ccaa0e89ab54f8fc73e81713c4be3c258895");
});

Deno.test("the mail_claim max_tokens migration is unchanged", async () => {
  // SHA-256 of 20261005000100_mail_claim_max_tokens.sql as committed at 2966037
  // (`git show 2966037:<path>`), its only commit. Applied on staging; never edited; a change is a
  // new file.
  const bytes = await Deno.readFile(new URL("20261005000100_mail_claim_max_tokens.sql", HERE));
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hex = [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
  assertEquals(hex, "1faed37a5c900325b2e1106249177a80aae63e6891c0bc326e709098a915fb18");
});

Deno.test("the mail last_lapsed_at migration is unchanged", async () => {
  // SHA-256 of 20261019000100_mail_last_lapsed_at.sql as committed at fc3fc42
  // (`git show fc3fc42:<path>`), its only commit. Applied on staging; never edited; a change is a
  // new file.
  const bytes = await Deno.readFile(new URL("20261019000100_mail_last_lapsed_at.sql", HERE));
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hex = [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
  assertEquals(hex, "62336f3ac1b8fe41fb4852e128ad3a340b4547d599671b6423462437770f129f");
});

Deno.test("the mail max_tokens re-pin migration is unchanged", async () => {
  // SHA-256 of 20261020000100_mail_max_tokens_repin.sql as committed at 42d4d12
  // (`git show 42d4d12:<path>`), its only commit. Applied on staging; never edited; a change is a
  // new file.
  const bytes = await Deno.readFile(new URL("20261020000100_mail_max_tokens_repin.sql", HERE));
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hex = [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
  assertEquals(hex, "a1eb2848387b307a2443674faebcf894bbfa18074b2c497dec4ae3046206de8d");
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
      "unauthenticated", "expired", "unbound_own_since_live", "unbound_foreign_since_live",
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

/** Each kind's `max_tokens` as the whole corpus leaves it: the value its `insert into models` row seeded,
 *  then every later `update models … max_tokens = N … where kind = '<kind>';`, file by file in order. */
function currentMaxTokens(files: Array<[string, string]>): Partial<Record<PinKind, number>> {
  const out: Partial<Record<PinKind, number>> = {};
  const seed = /\(\s*'(task|event|email|mail_claim|mail_resolve|mail_digest)'\s*,(?:\s*'[^']*'\s*,){4}\s*(\d+)\s*\)/gi;
  for (const [, raw] of files) {
    const sql = stripLineComments(raw);
    for (const m of sql.matchAll(/insert\s+into\s+(?:public\.)?models\b[^;]*;/gi)) {
      for (const row of m[0].matchAll(seed)) out[row[1] as PinKind] ??= Number(row[2]);
    }
    for (const m of sql.matchAll(/update\s+(?:public\.)?models\s+set([^;]*?)where\s+kind\s*=\s*'(\w+)'\s*;/gi)) {
      const set = /\bmax_tokens\s*=\s*(\d+)/i.exec(m[1]);
      if (set) out[m[2] as PinKind] = Number(set[1]);
    }
    // One statement for several kinds: `set max_tokens = case kind when '<kind>' then N … end, …
    // where kind in ('<kind>', …);` — a kind moves only if both its `when` arm and the `in` list name it.
    const multi =
      /update\s+(?:public\.)?models\s+set\s+max_tokens\s*=\s*case\s+kind\s+((?:when\s+'\w+'\s+then\s+\d+\s+)+)end\b[^;]*?where\s+kind\s+in\s*\(([^)]*)\)\s*;/gi;
    for (const m of sql.matchAll(multi)) {
      const listed = new Set([...m[2].matchAll(/'(\w+)'/g)].map((k) => k[1]));
      for (const arm of m[1].matchAll(/when\s+'(\w+)'\s+then\s+(\d+)/gi)) {
        if (listed.has(arm[1])) out[arm[1] as PinKind] = Number(arm[2]);
      }
    }
  }
  return out;
}

const MAIL_CLAIM_MAX_TOKENS = "20261005000100_mail_claim_max_tokens.sql";

Deno.test("mail_claim_alone_is_re_pinned_to_1024_max_tokens_by_one_statement", async () => {
  // Quinn, 2026-10-05, before T18b's pin check (P8's re-pin path): five claims at the prompt's limits
  // (title 200, evidence 140, why 140 chars) need ~690+ output tokens, past email's 640, and a
  // `max_tokens` stop drops every claim as `truncated`. 20261002000200_mail.sql is applied and stays.
  const files = await everyMigrationFile();
  const raw = files.find(([n]) => n === MAIL_CLAIM_MAX_TOKENS)?.[1];
  assert(raw !== undefined, `expected ${MAIL_CLAIM_MAX_TOKENS}`);
  const { name: mail } = await mailMigration();
  assert(MAIL_CLAIM_MAX_TOKENS > mail, `stamped after ${mail}, the row's seed`);
  const statements = stripLineComments(raw!).split(";").filter((s) => s.trim() !== "");
  assertEquals(statements.length, 1, `${MAIL_CLAIM_MAX_TOKENS}: one statement`);
  assertEquals(
    statements[0].replace(/\s+/g, " ").trim(),
    "update public.models set max_tokens = 1024, since = current_date where kind = 'mail_claim'",
  );
  // As this migration left the rows; the T18b re-pin below moves both mail kinds later.
  const through = files.filter(([n]) => n <= MAIL_CLAIM_MAX_TOKENS);
  assertEquals(currentMaxTokens(through), { task: 256, event: 256, email: 640, mail_claim: 1024, mail_resolve: 640 });
  const before = files.filter(([n]) => n < MAIL_CLAIM_MAX_TOKENS);
  assertEquals(currentMaxTokens(before).mail_claim, 640, "the applied mail migration's seed is unchanged");
});

const MAIL_MAX_TOKENS_REPIN = "20261020000100_mail_max_tokens_repin.sql";

Deno.test("mail_resolve_and_mail_claim_are_re_pinned_to_1280_and_1536_max_tokens_by_one_statement", async () => {
  // Quinn, 2026-10-20, T18b's pin check (P8's re-pin path): a 15-claim mail_resolve batch stopped at
  // 640/640 with 0/15 answered, and a five-obligation mail_claim used 1008 of 1024. One statement
  // moves both rows; 20261002000200_mail.sql and 20261005000100 are applied and stay.
  const files = await everyMigrationFile();
  const raw = files.find(([n]) => n === MAIL_MAX_TOKENS_REPIN)?.[1];
  assert(raw !== undefined, `expected ${MAIL_MAX_TOKENS_REPIN}`);
  assert(MAIL_MAX_TOKENS_REPIN > MAIL_CLAIM_MAX_TOKENS, `stamped after ${MAIL_CLAIM_MAX_TOKENS}`);
  assert(MAIL_MAX_TOKENS_REPIN > MAIL_LAST_LAPSED_AT, `stamped after ${MAIL_LAST_LAPSED_AT}, the branch's newest`);
  const statements = stripLineComments(raw!).split(";").filter((s) => s.trim() !== "");
  assertEquals(statements.length, 1, `${MAIL_MAX_TOKENS_REPIN}: one statement`);
  assertEquals(
    statements[0].replace(/\s+/g, " ").trim(),
    "update public.models set max_tokens = case kind when 'mail_resolve' then 1280 when 'mail_claim' then 1536 end, " +
      "since = current_date where kind in ('mail_resolve', 'mail_claim')",
  );
  assertEquals(currentMaxTokens(files), { task: 256, event: 256, email: 640, mail_claim: 1536, mail_resolve: 1280 });
  const before = files.filter(([n]) => n < MAIL_MAX_TOKENS_REPIN);
  assertEquals(currentMaxTokens(before), { task: 256, event: 256, email: 640, mail_claim: 1024, mail_resolve: 640 });
  // The pin stays email's model, route and price: only max_tokens and since move.
  for (const kind of ["mail_claim", "mail_resolve"] as const) {
    assertEquals(currentPin(files, kind), currentPin(before, kind), `${kind}: only max_tokens moves`);
  }
});

Deno.test("the pin parser reads a where-kind-in re-pin per kind, so a scratch re-pin that also moves model_id is caught", async () => {
  // Review of 42d4d12 (minor 1): `lastModelPinBlocks` once matched only `where kind = '<kind>';`, so
  // the "only max_tokens moves" check above compared identical pins. An in-test corpus, no file edit.
  const files = await everyMigrationFile();
  const raw = files.find(([n]) => n === MAIL_MAX_TOKENS_REPIN)![1];
  const blocks = lastModelPinBlocks([[MAIL_MAX_TOKENS_REPIN, raw]]);
  assertEquals(blocks.mail_resolve?.replace(/\s+/g, " ").trim(), "max_tokens = 1280, since = current_date");
  assertEquals(blocks.mail_claim?.replace(/\s+/g, " ").trim(), "max_tokens = 1536, since = current_date");
  assertEquals(Object.keys(blocks).sort(), ["mail_claim", "mail_resolve"], "only the listed kinds");
  const before = files.filter(([n]) => n < MAIL_MAX_TOKENS_REPIN);
  const tampered = raw.replace(/since = current_date/, "since = current_date, model_id = 'x'");
  assert(tampered !== raw, "the scratch re-pin differs from the file");
  const scratch: Array<[string, string]> = [...before, [MAIL_MAX_TOKENS_REPIN, tampered]];
  for (const kind of ["mail_claim", "mail_resolve"] as const) {
    assertEquals(currentPin(scratch, kind).model_id, "'x'", `${kind}: the scratch re-pin's model_id is read`);
    assertNotEquals(currentPin(scratch, kind), currentPin(before, kind), `${kind}: "only max_tokens moves" would fail`);
  }
  // A kind the `case` names but the `in` list does not is untouched; a listed kind with no arm is null.
  const odd = "update public.models set max_tokens = case kind when 'task' then 9 when 'event' then 7 end " +
    "where kind in ('event', 'email');";
  const oddBlocks = lastModelPinBlocks([["scratch.sql", odd]]);
  assertEquals(Object.keys(oddBlocks).sort(), ["email", "event"]);
  assertEquals(oddBlocks.event?.trim(), "max_tokens = 7");
  assertEquals(oddBlocks.email?.trim(), "max_tokens = null");
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
  // statement, so two deliveries at once both count. Quinn's unbound-display ruling (2026-10-03): an
  // unbound live forward also runs one of two counts since the last bound live forward (`unbound_own`,
  // `unbound_foreign`), and a bound one (`live`, or `unauthenticated`: it bound, then failed step 6) sets
  // both back to 0, writing nothing when they already are.
  const f = await mailFunction("count_mail_arrival");
  assert(/count_mail_arrival\s*\(\s*p_address uuid, p_counter text\s*\)/.test(f.header), f.header);
  assert(f.header.includes("returns void") && !f.header.includes("security definer"), `${f.name}: invoker`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertServiceRoleOnly(f, "count_mail_arrival", "uuid, text");
  const bound = "p_counter in ('live', 'unauthenticated')";
  assert(
    f.body.includes(
      "update public.mail_addresses set " +
        "unbound = unbound + (p_counter in ('unbound', 'unbound_own', 'unbound_foreign'))::int, " +
        "unauthenticated = unauthenticated + (p_counter = 'unauthenticated')::int, " +
        `unbound_own_since_live = case when ${bound} then 0 ` +
        "else unbound_own_since_live + (p_counter = 'unbound_own')::int end, " +
        `unbound_foreign_since_live = case when ${bound} then 0 ` +
        "else unbound_foreign_since_live + (p_counter = 'unbound_foreign')::int end " +
        "where id = p_address " +
        "and (p_counter <> 'live' or unbound_own_since_live + unbound_foreign_since_live > 0);",
    ),
    `${f.name}: one statement, one mailbox, the named counter only`,
  );
  const cols = tableColumns((await mailMigration()).sql, "mail_addresses");
  for (const c of ["unbound_own_since_live", "unbound_foreign_since_live"]) {
    const def = (cols.get(c) ?? "").replace(/\s+/g, " ");
    assertEquals(def, `${c} integer not null default 0 check (${c} >= 0)`, `${f.name}: ${c} is a count`);
  }
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

// Review of the lapsed fix (2026-10-19), #1: `lapsed_since` keeps the FIRST lapse's instant, so a lapse
// that opens while a `covered` report is in flight is stored under the same stamp, and `covered` keyed on
// that stamp alone wiped it. `last_lapsed_at` records each lapse separately: intake sets it on every move
// into `lapsed`, the device echoes the one it read before its pass, and `covered` clears only while both
// stamps are still the ones it echoes.
const MAIL_LAST_LAPSED_AT = "20261019000100_mail_last_lapsed_at.sql";

Deno.test("mail_addresses_gains_last_lapsed_at_backfilled_from_lapsed_since_and_never_null_beside_a_stamp", async () => {
  const files = await everyMigrationFile();
  const raw = files.find(([n]) => n === MAIL_LAST_LAPSED_AT)?.[1];
  assert(raw !== undefined, `expected ${MAIL_LAST_LAPSED_AT}`);
  // Forward-only: a new file after every applied one; the mail migration is never edited.
  // Stamped after every file before it; a later migration is named here when it lands.
  // A later lane's file is named by its suffix, never its stamp (stamp rule M4); L8's testers file
  // is left out by its suffix (friends-testers §8.4, M4), so its restamp moves no line here; so is
  // its S6 follow-up, <stamp>_comp_guards.sql.
  const LATER_LANES = /^\d{14}_(?:testers|comp_guards)\.sql$/;
  assertEquals(
    files.map(([n]) => n).filter((n) => n > MAIL_LAST_LAPSED_AT && !LATER_LANES.test(n))
      .map((n) => EVENT_ALLOWANCE.test(n) ? "<stamp>_event_allowance.sql" : n),
    // Both later files by their suffix, never their stamp (M4).
    [MAIL_MAX_TOKENS_REPIN, "<stamp>_event_allowance.sql", (await followUpsMigration()).name],
    "stamped after every other file but the named later ones",
  );
  const { name: mail, sql: mailSql } = await mailMigration();
  assert(!tableColumns(mailSql, "mail_addresses").has("last_lapsed_at"), `${mail} stays as applied`);
  const statements = stripLineComments(raw!).split(";").map((s) => s.replace(/\s+/g, " ").trim().toLowerCase())
    .filter((s) => s !== "");
  assertEquals(statements, [
    // Nullable: a mailbox that never lapsed has none.
    "alter table public.mail_addresses add column if not exists last_lapsed_at timestamptz",
    // A row lapsed before this file keeps its one known instant, so a `covered` for it can still match.
    "update public.mail_addresses set last_lapsed_at = lapsed_since where lapsed_since is not null and last_lapsed_at is null",
    // A stamped gap always names its last lapse, or `covered` could never clear it.
    "alter table public.mail_addresses add constraint mail_addresses_last_lapse_stamped " +
    "check (lapsed_since is null or last_lapsed_at is not null)",
  ]);
});

// L8 S2a0 (friends-testers spec §8.7, §9 layer i): the guards must see the `admin` schema before any
// `admin.*` SQL exists. Synthetic strings only, never the corpus; `identPattern` admits `public` or
// `admin` and nothing wider, so a third schema still fails loud.
function thrownMessage(fn: () => unknown): string {
  const err = assertThrows(fn, Error);
  return err instanceof Error ? err.message : String(err);
}

const ADMIN_DEFINER = `
create function admin.x() returns int as $$ select 1 $$ language sql security definer;
`;

Deno.test("S2a0: an unrevoked admin.x definer function parses as x and is reported", () => {
  const message = thrownMessage(() => assertExecuteRevoked([["synthetic-admin-fn.sql", ADMIN_DEFINER]]));
  assert(message.includes(": x is SECURITY DEFINER with no 'revoke execute"), message);
});

Deno.test("S2a0: admin.x revoked from anon and authenticated passes, parsed as one definition", () => {
  const sql = ADMIN_DEFINER + "revoke execute on function admin.x() from public, anon, authenticated;\n";
  const { parsed } = assertExecuteRevoked([["synthetic-admin-fn-revoked.sql", sql]]);
  assertEquals(parsed, 1);
});

Deno.test("S2a0: admin.v parses as v, not admin; its revoke is found and its absence reported", () => {
  const view = "create view admin.v as select 1;\n";
  const ok = assertViewsGuarded([
    ["synthetic-admin-view.sql", view + "revoke all on admin.v from anon, authenticated;\n"],
  ]);
  assertEquals(ok.revoked, ["v"]);
  assertEquals(ok.parsed, 1);
  const message = thrownMessage(() => assertViewsGuarded([["synthetic-admin-view-bare.sql", view]]));
  assert(message.includes("the view v declares no"), message);
});

Deno.test("S2a0: quoted admin forms behave as the quoted public forms do", () => {
  const fn = `
create or replace function "admin"."qx"(p_account uuid)
returns text
language sql
security definer
as $$
  select 'x';
$$;
revoke execute on function admin."qx"(uuid) from public, anon, authenticated;
`;
  assertEquals(assertExecuteRevoked([["synthetic-admin-quoted-fn.sql", fn]]).parsed, 1);
  const unrevoked = fn.slice(0, fn.indexOf("revoke"));
  const fnMessage = thrownMessage(() => assertExecuteRevoked([["synthetic-admin-quoted-fn-bare.sql", unrevoked]]));
  assert(fnMessage.includes(": qx is SECURITY DEFINER"), fnMessage);
  const view = assertViewsGuarded([
    ["synthetic-admin-quoted-view.sql", `create view "admin"."qv" as select 1;\nrevoke all on "admin".qv from anon, authenticated;\n`],
  ]);
  assertEquals(view.revoked, ["qv"]);
});

Deno.test("S2a0: a function in any third schema still fails loud, so the widening admits only admin", () => {
  const sql = `
create function other.x() returns int as $$ select 1 $$ language sql security definer;
revoke execute on function other.x() from public, anon, authenticated;
`;
  const message = thrownMessage(() => assertExecuteRevoked([["synthetic-other-fn.sql", sql]]));
  assert(message.includes("found 1") && message.includes("parsed 0"), message);
  // A view in a third schema is never silently guarded either: it reads as a view named after the
  // schema, and no revoke can name that, so it is reported.
  const viewSql = "create view other.v as select 1;\nrevoke all on other.v from anon, authenticated;\n";
  thrownMessage(() => assertViewsGuarded([["synthetic-other-view.sql", viewSql]]));
});

// L8 S2a (friends-testers spec §3.1, §3.5, §6, §9 layer i). The rule-1 tripwire: (a) no
// address-shaped token in any migration's raw text, comments and `$$` bodies included; (b) every
// write to `comp_grants` or `comp_audit` (exactly the three DML shapes of §6) lies inside an
// `admin.comp_*` function body, and an insert into `comp_grants` carries no quoted literal.
const ADDRESS_TOKEN = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
export function addressTokens(files: Array<[string, string]>): string[] {
  return files.flatMap(([name, raw]) => [...raw.matchAll(ADDRESS_TOKEN)].map((m) => `${name}: ${m[0]}`));
}

/** `comp_grants` or `comp_audit`, bare, `public.`-qualified or quoted, and never a longer name. */
const GRANT_TABLE = `(?:"?public"?\\s*\\.\\s*)?"?(?:comp_grants|comp_audit)"?(?![\\w"])`;
/** §6's three write shapes and nothing else: `insert into <t>`, `update [only] <t> [alias] set`,
 * `delete from [only] <t>`. A DML word elsewhere (a revoke, `on delete set null`) is not a write. */
const GRANT_WRITE = new RegExp(
  `\\b(?:insert\\s+into\\s+${GRANT_TABLE}|update\\s+(?:only\\s+)?${GRANT_TABLE}(?:\\s+(?:as\\s+)?\\w+)?\\s+set\\b|` +
    `delete\\s+from\\s+(?:only\\s+)?${GRANT_TABLE})`,
  "gi",
);
/** A named `admin.comp_*` function; group 1 is its dollar tag, group 2 its body. */
const ADMIN_COMP_FUNCTION =
  /create\s+(?:or\s+replace\s+)?function\s+"?admin"?\s*\.\s*"?comp_\w+"?\s*\([^)]*\)[\s\S]*?\bas\s+(\$[A-Za-z_]*\$)([\s\S]*?)\1/gi;

/** The `[start, end)` offsets of every `admin.comp_*` function body in comment-stripped SQL. */
function compBodies(sql: string): Array<[number, number]> {
  return [...sql.matchAll(ADMIN_COMP_FUNCTION)].map((m) => {
    const end = (m.index ?? 0) + m[0].length - m[1].length;
    return [end - m[2].length, end];
  });
}

export function grantWriteProblems(files: Array<[string, string]>): string[] {
  const out: string[] = [];
  for (const [name, raw] of files) {
    const sql = stripLineComments(raw);
    const bodies = compBodies(sql);
    for (const w of sql.matchAll(GRANT_WRITE)) {
      const at = w.index ?? 0;
      if (!bodies.some(([s, e]) => at >= s && at < e)) out.push(`${name}: '${w[0]}' outside an admin.comp_* function`);
      const stop = sql.indexOf(";", at);
      const statement = sql.slice(at, stop === -1 ? undefined : stop);
      if (/^insert/i.test(w[0]) && /comp_grants/i.test(w[0]) && statement.includes("'")) {
        out.push(`${name}: an insert into comp_grants carries a quoted literal`);
      }
    }
  }
  return out;
}

const LEAK = "do $$ begin insert into public.comp_grants values ('friend@example.invalid'); end $$;\n";
const ADMIN_GRANT_FN = `create function admin.comp_grant(p_account uuid, p_ends timestamptz) returns uuid
language plpgsql security definer as $$ declare g uuid; begin
  insert into public.comp_grants (account_id, ends_at) values (p_account, p_ends) returning id into g;
  return g; end $$;
`;
// §9's must-pass shapes, and the select-only revoke and grant that S2a's SQL now carries in their
// place (the S2-S4R fixups); none is a write.
const MUST_PASS = `revoke insert, update, delete, truncate on public.comp_grants from service_role;
revoke insert, update, delete, truncate on public.comp_audit from service_role;
revoke all on public.comp_grants from service_role;
grant select on public.comp_grants to service_role;
revoke all on public.comp_audit from service_role;
grant select on public.comp_audit to service_role;
create table public.comp_audit (id uuid primary key, grant_id uuid references public.comp_grants on delete set null,
  account_id uuid not null references public.accounts(id) on delete cascade, at timestamptz);
create unique index comp_grants_one_open on public.comp_grants (account_id) where ended_at is null;
comment on table public.comp_grants is 'rows insert only through admin functions; delete by the sweep';
`;

Deno.test("S2a tripwire: a do-block insert carrying an address is reported by both parts", () => {
  assertEquals(addressTokens([["leak.sql", LEAK]]), ["leak.sql: friend@example.invalid"]);
  const problems = grantWriteProblems([["leak.sql", LEAK]]);
  assert(problems.some((p) => p.includes("outside an admin.comp_* function")), problems.join("\n"));
  assert(problems.some((p) => p.includes("quoted literal")), problems.join("\n"));
});

Deno.test("S2a tripwire: an address in a -- comment is reported by (a)", () => {
  assertEquals(addressTokens([["c.sql", "-- reach k7@in.example.invalid\nselect 1;\n"]]).length, 1);
});

Deno.test("S2a tripwire: top-level writes in each DML shape are reported by (b), literal or not", () => {
  for (
    const sql of [
      "insert into comp_grants select * from x;",
      "update only public.comp_audit set at = now();",
      'delete from "comp_grants";',
      "update public.comp_grants g set ends_at = now();",
    ]
  ) {
    assertEquals(grantWriteProblems([["w.sql", sql]]).length, 1, sql);
  }
});

Deno.test("S2a tripwire: a parameterised insert inside admin.comp_grant passes; a literal there does not", () => {
  assertEquals(grantWriteProblems([["fn.sql", ADMIN_GRANT_FN]]), []);
  const literal = ADMIN_GRANT_FN.replace("(p_account, p_ends)", "('00000000-0000-0000-0000-000000000000', p_ends)");
  assertEquals(grantWriteProblems([["fn.sql", literal]]).length, 1);
});

Deno.test("S2a tripwire: S2a's revokes, foreign key, index and comment are not writes", () => {
  assertEquals(grantWriteProblems([["s2a.sql", MUST_PASS]]), []);
  assertEquals(addressTokens([["s2a.sql", MUST_PASS]]), []);
});

Deno.test("S2a tripwire: the whole corpus carries no address-shaped token and no stray grant write", async () => {
  const files = await everyMigrationFile();
  assertEquals(addressTokens(files), []);
  assertEquals(grantWriteProblems(files), []);
});

// L8 S2R-fix-r1 (friends-testers §6, rule 1): (b) sees only the three DML shapes, so a grant
// written as a call got through. (c) every call to `admin.comp_grant`, `comp_extend` or `comp_end`
// lies inside an `admin.comp_*` body; a `revoke`/`grant … on function`, `drop`/`alter`/`comment on
// function` names one and is no call. (d) no uuid-shaped token in any migration's raw text but the
// nil uuid, so a wrapper `admin.comp_*` that carries a tester's account id is caught too.
/** A call to the three grant writers, `admin.`-qualified, quoted or bare; group 1 marks a name a
 * `function` keyword introduces (a definition, a right, a comment), which is no call. */
const GRANT_CALL = /(\bfunction\s+)?(?:"?admin"?\s*\.\s*)?(?<![\w$])"?comp_(?:grant|extend|end)"?\s*\(/gi;
/** A statement that only names functions: a later name in its list is no call either. */
const NAMES_FUNCTIONS =
  /^\s*(?:(?:revoke|grant)\b[^;]*?\bon\s+function\s|(?:drop|alter|comment\s+on)\s+function\s)/i;

export function grantCallProblems(files: Array<[string, string]>): string[] {
  const out: string[] = [];
  for (const [name, raw] of files) {
    const sql = stripLineComments(raw);
    const bodies = compBodies(sql);
    for (const c of sql.matchAll(GRANT_CALL)) {
      const at = c.index ?? 0;
      if (c[1] || NAMES_FUNCTIONS.test(sql.slice(sql.lastIndexOf(";", at) + 1, at))) continue;
      if (!bodies.some(([s, e]) => at >= s && at < e)) {
        out.push(`${name}: '${c[0]}' outside an admin.comp_* function`);
      }
    }
  }
  return out;
}

/** Dashed or undashed, any case; a longer hex run is not one. */
const UUID_TOKEN =
  /(?<![0-9a-f])[0-9a-f]{8}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{12}(?![0-9a-f])/gi;
export function uuidTokens(files: Array<[string, string]>): string[] {
  return files.flatMap(([name, raw]) =>
    [...raw.matchAll(UUID_TOKEN)].filter((m) => !/^[0-]+$/.test(m[0])).map((m) => `${name}: ${m[0]}`)
  );
}

const ID = "3f2a9c1e-7b4d-4e8a-9c2f-1a2b3c4d5e6f"; // synthetic, no account's
const TOP_CALL = `select admin.comp_grant('${ID}'::uuid, now() + interval '90 days', 'friends', null);\n`;

Deno.test("S2R-fix-r1 tripwire: a top-level comp_grant call naming an account id is reported by (c) and (d)", () => {
  assertEquals(addressTokens([["t.sql", TOP_CALL]]), [], "(a) alone cannot see it");
  assertEquals(grantWriteProblems([["t.sql", TOP_CALL]]), [], "(b) alone cannot see it");
  assertEquals(grantCallProblems([["t.sql", TOP_CALL]]).length, 1);
  assertEquals(uuidTokens([["t.sql", TOP_CALL]]), [`t.sql: ${ID}`]);
});

Deno.test("S2R-fix-r1 tripwire: calls in a do block, a cron string, a non-comp body or unqualified are reported", () => {
  for (
    const sql of [
      "do $$ begin perform admin.comp_extend(a.id, now()) from public.accounts a; end $$;",
      "select cron.schedule('x', '1 * * * *', $c$select \"admin\".\"comp_end\" (a.id, 'done')$c$);",
      "create function public.f() returns void language sql as $$ select admin.comp_grant(null, now()) $$;",
      "set search_path = admin; select comp_grant(null, now());",
    ]
  ) {
    assertEquals(grantCallProblems([["c.sql", sql]]).length, 1, sql);
  }
});

Deno.test("S2R-fix-r1 tripwire: definitions, rights, comments and calls inside admin.comp_* bodies pass", () => {
  const sql = ADMIN_GRANT_FN + `create function admin.comp_sweep() returns void language sql as $$
  select admin.comp_end(g.account_id, 'swept') from public.comp_grants g $$;
revoke execute on function admin.comp_grant(uuid, timestamptz) from public, anon, authenticated;
grant execute on function admin.comp_extend(uuid, timestamptz), admin.comp_end(uuid, text) to postgres;
alter function admin.comp_end(uuid, text) set search_path = '';
comment on function admin.comp_grant(uuid, timestamptz) is 'by account id';
drop function if exists admin.comp_end(uuid);
select cron.schedule('knowlu-sweep-comp', '7 3 * * *', 'select admin.comp_sweep()');
create unique index comp_grants_one_open on public.comp_grants (account_id) where ended_at is null;
`;
  assertEquals(grantCallProblems([["fn.sql", sql]]), []);
  assertEquals(uuidTokens([["fn.sql", sql]]), []);
});

Deno.test("S2R-fix-r1 tripwire: (d) sees an undashed id and an id in a comment; the nil uuid passes", () => {
  const undashed = ID.replaceAll("-", "");
  assertEquals(uuidTokens([["u.sql", `select uuid '${undashed}';\n`]]), [`u.sql: ${undashed}`]);
  assertEquals(uuidTokens([["u.sql", `-- granted ${ID.toUpperCase()}\n`]]).length, 1);
  assertEquals(uuidTokens([["n.sql", "select '00000000-0000-0000-0000-000000000000'::uuid;\n"]]), []);
});

Deno.test("S2R-fix-r1 tripwire: the whole corpus carries no stray grant call and no uuid but the nil one", async () => {
  const files = await everyMigrationFile();
  assertEquals(grantCallProblems(files), []);
  assertEquals(uuidTokens(files), []);
});

/** The one `<stamp>_testers.sql`, comment-stripped; found by suffix, never by stamp (M4). */
async function testersMigration(): Promise<{ name: string; sql: string }> {
  const files = (await everyMigrationFile()).filter(([n]) => /^\d{14}_testers\.sql$/.test(n));
  assertEquals(files.length, 1, "exactly one <stamp>_testers.sql (friends-testers §8.4, M4)");
  return { name: files[0][0], sql: stripLineComments(files[0][1]) };
}
const COMP_TABLES = ["comp_grants", "comp_audit"];
const flatten = (sql: string) => stripLineComments(sql).replace(/\s+/g, " ").toLowerCase();

/** §3.5, made literal by the S2-S4R fixups: the service role holds select on comp_grants and
 * comp_audit and nothing else. Supabase's default privileges hand it every table right (references,
 * trigger and, on PG17, maintain among them), so the testers migration revokes all from it and grants
 * select back, in that order; no migration grants anything else on either table, revokes that select
 * from it elsewhere, or grants it every table in public at or after the testers migration. */
export function compRightsProblems(files: Array<[string, string]>, testers: string): string[] {
  const out: string[] = [];
  for (const [file, raw] of files) {
    const text = flatten(raw);
    if (file >= testers && /\bgrant [^;]* on all tables in schema public [^;]*\bservice_role\b/.test(text)) {
      out.push(`${file}: a schema-wide grant to the service role reaches the comp tables`);
    }
    for (const t of COMP_TABLES) {
      const on = `on (?:table )?(?:public\\.)?${t}\\b`;
      const rights = [...text.matchAll(new RegExp(`\\b(?:grant|revoke) [^;]*?${on}[^;]*;`, "g"))].map((m) => m[0]);
      for (const s of rights) if (s.startsWith("grant ") && s !== `grant select on public.${t} to service_role;`) out.push(`${file}: ${s}`);
      const svc = rights.filter((s) => /\bservice_role\b/.test(s));
      if (file === testers) {
        const want = `revoke all on public.${t} from service_role; grant select on public.${t} to service_role;`;
        if (svc.join(" ") !== want) out.push(`${file}: ${t} is not select-only for the service role: ${svc.join(" ")}`);
        if (!text.includes(`alter table public.${t} enable row level security;`)) out.push(`${file}: ${t} needs RLS on`);
        if (!text.includes(`revoke all on public.${t} from public, anon, authenticated;`)) out.push(`${file}: ${t} client revoke`);
      } else if (svc.some((s) => s.startsWith("revoke ") && /\b(?:select|all)\b/.test(s))) {
        out.push(`${file}: the export needs the service role's select on ${t}`);
      }
      if (new RegExp(`create policy [^;]*${on}`).test(text)) out.push(`${file}: a policy on ${t}`);
    }
  }
  return out;
}

Deno.test("S2a: comp_grants and comp_audit have RLS on, no client grant or policy, and the service role only reads", async () => {
  const { name } = await testersMigration();
  assertEquals(compRightsProblems(await everyMigrationFile(), name), []);
});

Deno.test("S2-S4R fixups: the select-only pin refuses a leftover right, a wider grant, the wrong order, a later revoke", async () => {
  const files = await everyMigrationFile();
  const { name } = await testersMigration();
  const edit = (f: (t: string) => string) => files.map(([n, t]): [string, string] => [n, n === name ? f(t) : t]);
  const revoke = "revoke all on public.comp_grants from service_role;";
  const grant = "grant select on public.comp_grants to service_role;";
  for (
    const [label, bad] of [
      ["the old four-right revoke", edit((t) => t.replace(revoke, "revoke insert, update, delete, truncate on public.comp_grants from service_role;"))],
      ["no select back", edit((t) => t.replace(grant, ""))],
      ["select and references", edit((t) => t.replace(grant, "grant select, references on public.comp_grants to service_role;"))],
      ["select to a client", edit((t) => `${t}\ngrant select on public.comp_audit to authenticated;\n`)],
      ["grant before revoke", edit((t) => t.replace(revoke, "@@").replace(grant, revoke).replace("@@", grant))],
      ["a later select revoke", [...files, ["29991231000000_later.sql", "revoke select on public.comp_audit from service_role;"]]],
      ["a later schema-wide grant", [...files, ["29991231000000_later.sql", "grant all on all tables in schema public to service_role;"]]],
    ] as Array<[string, Array<[string, string]>]>
  ) {
    assertNotEquals(bad, files, `${label}: the mutation must apply`);
    assert(compRightsProblems(bad, name).length > 0, `${label} must be refused`);
  }
});

Deno.test("S2a: the admin schema's rights are revoked from every API role, and PostgREST does not expose it", async () => {
  const { name, sql } = await testersMigration();
  const flat = flatten(sql);
  assert(/\bcreate schema (?:if not exists )?admin;/.test(flat), `${name}: creates the admin schema`);
  const revokes = [...flat.matchAll(/revoke (?:all|usage) on schema admin from ([^;]+);/g)].map((m) => m[1]);
  const roles = ["public", "anon", "authenticated", "service_role"];
  assert(revokes.some((f) => roles.every((r) => new RegExp(`\\b${r}\\b`).test(f))), `${name}: admin usage revoke`);
  for (const [file, raw] of await everyMigrationFile()) {
    assert(!/\bgrant [^;]* on schema admin\b/.test(flatten(raw)), `${file}: grants on the admin schema`);
  }
  const toml = await Deno.readTextFile(new URL("../config.toml", HERE));
  const schemas = /^\[api\][^[]*?^schemas\s*=\s*\[([^\]]*)\]/m.exec(toml);
  assert(schemas !== null && !/admin/.test(schemas[1]), "config.toml's [api] schemas must not expose admin");
});

Deno.test("S2a: the grant and audit columns are §3.1's and §3.5's; no address, hash, actor or free text in the audit", async () => {
  const { name, sql } = await testersMigration();
  const grants = tableColumns(sql, "comp_grants");
  const audit = tableColumns(sql, "comp_audit");
  assertEquals([...grants.keys()], ["id", "account_id", "cohort", "granted_at", "ends_at", "ended_at", "ended_reason", "note"]);
  assertEquals([...audit.keys()], ["id", "grant_id", "account_id", "action", "ends_at", "cohort", "db_role", "at"]);
  for (const col of [...grants.keys(), ...audit.keys()]) {
    assert(!/email|title|body|sender|subject|item_id/.test(col), `${name}: column ${col} could hold an address or content`);
  }
  for (const c of ["note", "reason", "actor", "email", "email_hash"]) assert(!audit.has(c), `${name}: comp_audit.${c}`);
  assert(grants.get("note")!.includes("check (char_length(note) <= 200)"), `${name}: the note's length bound`);
  assert(grants.get("ended_reason")!.includes("check (ended_reason in ('done', 'asked', 'misuse', 'other'))"));
  assert(audit.get("action")!.includes("check (action in ('grant', 'extend', 'end'))"), `${name}: the audit's actions`);
  assert(
    /create unique index \w+ on public\.comp_grants \(account_id\) where ended_at is null;/.test(flatten(sql)),
    `${name}: one open grant per account (§3.1)`,
  );
});

// L8 S2b (friends-testers §3.1, §3.2, §9 layer i, Amendment 2026-10-20): the view every gated call
// will read. A row with no open grant is the `entitlements` row passed through; the grant branch is
// the complement of the paid statuses; the grant is matched by account id alone, never an address.
const EE_ITEMS = [
  "a.id as account_id",
  "case when g.id is null then e.plan else 'tester' end as plan",
  "case when g.id is null then coalesce(e.status, 'none') else 'active' end as status",
  "case when g.id is null then e.current_period_end else g.ends_at end as current_period_end",
];
const EE_FROM = "from public.accounts a left join public.entitlements e on e.account_id = a.id " +
  "left join public.comp_grants g on g.account_id = a.id and g.ended_at is null and now() < g.ends_at " +
  "and coalesce(e.status, 'none') not in ('active', 'trialing');";

/** The flattened `create view public.effective_entitlements … ;`, or "" when there is none. */
const eeStatement = (sql: string) =>
  /create (?:or replace )?view public\.effective_entitlements as [^;]*;/.exec(flatten(sql))?.[0] ?? "";

/** The select list split at top-level commas (a comma inside parentheses separates nothing). */
function selectItems(stmt: string): string[] {
  const body = stmt.slice(stmt.indexOf(" as select ") + " as select ".length);
  const items: string[] = [];
  let depth = 0, cur = "";
  for (let i = 0; i < body.length && !(depth === 0 && body.startsWith(" from ", i)); i++) {
    if (body[i] === "(") depth++;
    if (body[i] === ")") depth--;
    if (body[i] === "," && depth === 0) {
      items.push(cur.trim());
      cur = "";
    } else cur += body[i];
  }
  return [...items, cur.trim()];
}

export function effectiveEntitlementsProblems(stmt: string): string[] {
  if (stmt === "") return ["no create view public.effective_entitlements"];
  const out: string[] = [];
  const items = selectItems(stmt);
  const names = items.map((i) => i.split(" as ").at(-1));
  if (names.join() !== "account_id,plan,status,current_period_end") out.push(`output names ${names}`);
  if (items.join() !== EE_ITEMS.join()) out.push(`select list ${items.join(" | ")}`);
  if (!stmt.endsWith(EE_FROM)) out.push("not the accounts left join entitlements, left join open grant (P0, P1, P3)");
  if ((stmt.match(/\bjoin\b/g) ?? []).length !== (stmt.match(/\bleft join\b/g) ?? []).length) out.push("an inner join");
  if (/\bwhere\b/.test(stmt)) out.push("a where clause drops an account's row (P0)");
  for (const bad of ["in ('canceled'", "in ('past_due'", "e.account_id is null", "auth.users", "encrypted_password", "email"]) {
    if (stmt.includes(bad)) out.push(`names ${bad}`);
  }
  return out;
}

Deno.test("S2b: effective_entitlements passes the Stripe row through, grants by account id on the complement of paid, four columns", async () => {
  const { name, sql } = await testersMigration();
  const stmt = eeStatement(sql);
  assertEquals(effectiveEntitlementsProblems(stmt), [], name);
  for (const pin of ["coalesce(e.status, 'none') not in ('active', 'trialing')", "g.account_id = a.id", "g.ended_at is null", "now() < g.ends_at"]) {
    assert(stmt.includes(pin), `${name}: ${pin}`);
  }
  const flat = flatten(sql);
  assert(flat.includes("revoke all on public.effective_entitlements from public, anon, authenticated;"), `${name}: revoke`);
  for (const [file, raw] of await everyMigrationFile()) {
    const text = flatten(raw);
    assert(!/\bgrant [^;]* on (?:table )?public\.effective_entitlements\b/.test(text), `${file}: a grant on the view`);
    assert(
      !/\brevoke [^;]* on (?:table )?public\.effective_entitlements [^;]*\bservice_role\b/.test(text),
      `${file}: the gate's lookup needs the service role's select on the view (A7)`,
    );
  }
});

Deno.test("S2b: the view pins refuse a list branch, an inner join, an auth.users read, a where and a missing column", async () => {
  const stmt = eeStatement((await testersMigration()).sql);
  const complement = "coalesce(e.status, 'none') not in ('active', 'trialing')";
  for (
    const [from, to] of [
      [complement, "(e.account_id is null or e.status in ('canceled', 'past_due'))"],
      ["left join public.entitlements", "join public.entitlements"],
      [" left join public.comp_grants", " join auth.users u on u.id = a.id left join public.comp_grants"],
      [" and now() < g.ends_at", " where now() < g.ends_at"],
      [", case when g.id is null then e.current_period_end else g.ends_at end as current_period_end", ""],
    ]
  ) {
    const bad = stmt.replace(from, to);
    assertNotEquals(bad, stmt, `the mutation ${from} must apply to the real view`);
    assert(effectiveEntitlementsProblems(bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S2b: the case table holds §9's live P rows, the none-row cases first; P4, P7, P10, P11 only reserved", async () => {
  const { CASES } = await import("./testers_cases.ts");
  const ids = CASES.map((c) => c.id);
  const live = ["P0", "P0x", "P1", "P2", "P3", "P3x", "P5", "P6", "P8", "P9", "P12", "P13", "P14", "P15", "P16"];
  assertEquals([...ids].sort(), [...live].sort());
  assertEquals(ids.slice(0, 2), ["P0", "P3"]);
  const noneRow = (c: (typeof CASES)[number]) =>
    c.accounts.length > 0 && c.accounts.every((a) => a.row?.status === "none" && !a.row.plan && a.row.periodEnd === null);
  const firstOther = CASES.findIndex((c) => !noneRow(c));
  assert(firstOther > 0 && CASES.slice(firstOther).every((c) => !noneRow(c)), `none-row cases first: ${ids}`);
  for (const c of CASES) {
    assertEquals(c.expect.length, c.accounts.length, `${c.id}: one answer per account`);
    for (const a of c.accounts) assert([a.email, a.authEmail ?? a.email].every((e) => e.endsWith("@example.invalid")), c.id);
  }
  const text = await Deno.readTextFile(new URL("testers_cases.ts", HERE));
  assert(/\/\/[^\n]*P4, P7, P10 and P11/.test(text), "the retired ids are reserved in a comment");
});

// L8 S2c1 (friends-testers §3.3, §3.5, §9 layer i, Amendment 2026-10-20): the three grant writers,
// each by account id. Each writes exactly one audit row on the grant's account and never deletes;
// nothing updates the audit; P12's refusal names admin.comp_extend; comp_extend takes the open grant
// alone, refuses a date not in the future, and never tests the grant's own ends_at (P13).
const GRANT_FNS = [["comp_grant", "uuid, timestamptz, text, text"], ["comp_extend", "uuid, timestamptz"], [
  "comp_end",
  "uuid, text",
]] as const;
const OPEN_ONE = "where account_id = p_account_id and ended_at is null returning";
const P12_MESSAGE = "an open grant exists for this account; use admin.comp_extend";

/** The flattened header (outside the body) and `$$` body of `admin.<fn>`; empty when not defined. */
function adminFunction(flat: string, fn: string): { header: string; body: string } {
  const m = new RegExp(`create (?:or replace )?function admin\\.${fn}\\([^)]*\\) (.*?) as \\$\\$(.*?)\\$\\$([^;]*);`)
    .exec(flat);
  return { header: m ? `${m[1]} ${m[3]}` : "", body: m?.[2] ?? "" };
}

export function grantFunctionProblems(sql: string): string[] {
  const flat = flatten(sql);
  const out: string[] = [];
  if (/\bupdate (?:only )?(?:"?public"?\s?\.\s?)?"?comp_audit\b/.test(flat)) out.push("an update of comp_audit");
  for (const [fn, args] of GRANT_FNS) {
    const { header, body } = adminFunction(flat, fn);
    if (body === "") {
      out.push(`no admin.${fn}`);
      continue;
    }
    const audit = `insert into public.comp_audit (grant_id, account_id, action, ends_at, cohort) ` +
      `values (v_id, p_account_id, '${fn.slice(5)}',`;
    if ((body.match(/insert into public\.comp_audit\b/g) ?? []).length !== 1 || !body.includes(audit)) {
      out.push(`${fn}: not exactly one audit row on the grant's account (A1)`);
    }
    if (/\bdelete from\b/.test(body)) out.push(`${fn}: deletes`);
    if (/security definer/.test(header) || !header.includes("set search_path = ''")) out.push(`${fn}: ${header}`);
    const revoke = `revoke execute on function admin.${fn}(${args}) from public, anon, authenticated, service_role;`;
    if (!flat.includes(revoke)) out.push(`${fn}: its revoke (A7)`);
    if (fn === "comp_grant") continue;
    if (!body.includes(OPEN_ONE) || !body.includes("if v_id is null then raise exception")) {
      out.push(`${fn}: not the open grant alone, or no refusal without one`);
    }
  }
  const grant = adminFunction(flat, "comp_grant").body;
  if (!grant.includes(`exception when unique_violation then raise exception '${P12_MESSAGE}';`)) out.push("P12's message");
  if (grant.indexOf("insert into public.comp_audit") < grant.indexOf("insert into public.comp_grants")) {
    out.push("comp_grant audits before it grants (A2)");
  }
  const extend = adminFunction(flat, "comp_extend").body;
  if (!extend.includes("if p_ends_at is null or p_ends_at <= now() then raise exception")) out.push("P13: a past date");
  if (/now\(\) ?<|> ?=? ?now\(\)/.test(extend)) out.push("P13: comp_extend tests a grant's ends_at");
  return out;
}

Deno.test("S2c1: comp_grant, comp_extend and comp_end are §3.5's, each auditing once, revoked from every API role", async () => {
  const { name, sql } = await testersMigration();
  assertEquals(grantFunctionProblems(sql), [], name);
});

Deno.test("S2c1: the grant pins refuse an ends_at test, a lost audit, an audit update, a wider revoke and P12's wording", async () => {
  const flat = flatten((await testersMigration()).sql);
  for (
    const [from, to] of [
      ["and ended_at is null returning id, cohort", "and ended_at is null and now() < ends_at returning id, cohort"],
      ["values (v_id, p_account_id, 'end',", "select (v_id, p_account_id, 'end',"],
      ["set ended_at = now(),", "set ended_at = now(), at = now() from public.comp_audit where false; update public.comp_audit set at = now(),"],
      ["admin.comp_end(uuid, text) from public, anon, authenticated, service_role;", "admin.comp_end(uuid, text) from public, anon, authenticated;"],
      [P12_MESSAGE, "already granted"],
      ["set search_path = '' as $$ declare v_id uuid; v_cohort", "security definer set search_path = '' as $$ declare v_id uuid; v_cohort"],
    ]
  ) {
    const bad = flat.replace(from, to);
    assertNotEquals(bad, flat, `the mutation ${from} must apply to the real functions`);
    assert(grantFunctionProblems(bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S2c1: the case table's A1, A2, A5 and A7 rows: one audit row per success, none per refusal", async () => {
  const { ADMIN_CASES } = await import("./testers_cases.ts");
  const byId = new Map(ADMIN_CASES.map((c) => [c.id, c]));
  for (const id of ["A1", "A2", "A5", "A7"]) assert(byId.has(id), id);
  for (const c of ADMIN_CASES) {
    assert(c.accounts.every((a) => a.email.endsWith("@example.invalid")), c.id);
    const seen = new Map<number | "unknown", string[]>();
    for (const call of c.calls) {
      const prior = seen.get(call.account) ?? [];
      const want = call.refused === undefined ? [...prior, call.fn.slice(5)] : prior;
      assertEquals([...call.audit].sort(), want.sort(), `${c.id} ${call.fn}`);
      seen.set(call.account, call.audit);
    }
    for (const s of c.asServiceRole ?? []) assertEquals(s.refused !== undefined, s.op !== "select", `${c.id} ${s.on}`);
  }
  assertEquals(byId.get("A2")!.calls.map((c) => c.refused), [undefined, "admin.comp_extend", "foreign key"]);
  assertEquals(byId.get("A5")!.billingSubscribers, [false]);
  assertEquals(byId.get("A7")!.asServiceRole?.length, 10, "six writes, the call, three selects");
});

// L8 S2c2 (friends-testers §3.1, §3.3, §9 layer i, Amendment 2026-10-20): deletion and retention,
// with no trigger: both grant tables cascade from accounts. admin.comp_sweep only deletes, a grant
// 30 days after it closed (ended_at or ends_at, whichever is first: least() skips a null ended_at)
// and an audit row at three years; its daily job's minute field is a number no fixed-minute job uses.
const SWEEP_GRANTS = "delete from public.comp_grants where least(ended_at, ends_at) < now() - interval '30 days';";
const SWEEP_AUDIT = "delete from public.comp_audit where at < now() - interval '3 years';";
const CASCADE = "account_id uuid not null references public.accounts(id) on delete cascade";

export function sweepProblems(sql: string): string[] {
  const flat = flatten(sql);
  const { header, body } = adminFunction(flat, "comp_sweep");
  const out = body === "" ? ["no admin.comp_sweep"] : [];
  if (!body.includes(SWEEP_GRANTS)) out.push("grants 30 days after closing (A4)");
  if (!body.includes(SWEEP_AUDIT)) out.push("audit rows at three years (A6)");
  if ((body.match(/\bdelete from\b/g) ?? []).length !== 2 || /\b(insert|update)\b/.test(body)) out.push("more than two deletes");
  if (/security definer/.test(header) || !header.includes("set search_path = ''")) out.push(`comp_sweep: ${header}`);
  const revoke = "revoke execute on function admin.comp_sweep() from public, anon, authenticated, service_role;";
  if (!flat.includes(revoke)) out.push("comp_sweep: its revoke");
  if (/\bcreate (?:or replace )?(?:constraint )?trigger\b|\breturns trigger\b/.test(flat)) out.push("a trigger");
  for (const t of COMP_TABLES) {
    if (tableColumns(flat, t).get("account_id") !== CASCADE) out.push(`${t}.account_id must cascade (A3, A3s)`);
  }
  const jobs = cronJobs(flat).filter((j) => j.job === "knowlu-sweep-comp");
  if (jobs.length !== 1 || jobs[0].command !== "select admin.comp_sweep();") out.push("one knowlu-sweep-comp job");
  return out;
}

/** The jobs in `others` whose minute field is a plain number equal to `when`'s, which must itself
 * be a fixed minute and hour once a day (§3.1). Every-minute and step jobs are skipped. */
export function minuteClashes(when: string, others: string[]): string[] {
  if (!/^\d{1,2} \d{1,2} \* \* \*$/.test(when)) return [`'${when}' is not a fixed minute and hour`];
  const minute = (w: string) => w.trim().split(/\s+/)[0];
  return others.filter((o) => /^\d{1,2}$/.test(minute(o)) && Number(minute(o)) === Number(minute(when)));
}

Deno.test("S2c2: comp_sweep deletes at 30 days and three years, daily; both tables cascade; no trigger", async () => {
  const { name, sql } = await testersMigration();
  assertEquals(sweepProblems(sql), [], name);
});

Deno.test("S2c2: the sweep pins refuse coalesce, a moved interval, a narrower revoke, no cascade, no job, a trigger", async () => {
  const flat = flatten((await testersMigration()).sql);
  for (
    const [from, to] of [
      ["least(ended_at, ends_at)", "coalesce(ended_at, ends_at)"],
      ["interval '30 days'", "interval '29 days'"],
      ["interval '3 years'", "interval '3 years 1 day'"],
      ["comp_sweep() from public, anon, authenticated, service_role;", "comp_sweep() from public, anon, authenticated;"],
      ["references public.accounts(id) on delete cascade, action", "references public.accounts(id), action"],
      ["$c$select admin.comp_sweep();$c$", "$c$select 1;$c$"],
      [" revoke execute on function admin.comp_sweep()", " create trigger t after delete on public.accounts for each row execute function admin.comp_sweep(); revoke execute on function admin.comp_sweep()"],
    ]
  ) {
    const bad = flat.replace(from, to);
    assertNotEquals(bad, flat, `the mutation ${from} must apply to the real SQL`);
    assert(sweepProblems(bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S2c2: the minute-field pin refuses a shared minute at another hour and skips every-minute and step jobs", () => {
  assertEquals(minuteClashes("23 8 * * *", ["23 7 * * *"]), ["23 7 * * *"]);
  assertEquals(minuteClashes("29 3 * * *", ["* * * * *", "11 * * * *", "*/29 * * * *"]), []);
  assertEquals(minuteClashes("29 * * * *", []).length, 1, "not once a day");
});

Deno.test("S2c2: knowlu-sweep-comp's minute field is used by no other fixed-minute job in the corpus", async () => {
  const texts = (await everyMigrationFile()).map(([, raw]) => stripLineComments(raw));
  const jobs = texts.flatMap(cronJobs);
  assertEquals(jobs.length, texts.join("\n").match(/cron\.schedule\(/g)?.length, "every job is read");
  const mine = jobs.filter((j) => j.job === "knowlu-sweep-comp");
  assertEquals(mine.length, 1);
  assertEquals(minuteClashes(mine[0].when, jobs.filter((j) => j !== mine[0]).map((j) => j.when)), []);
});

Deno.test("S2c2: the case table's A3, A3s, A4 and A6 rows agree with the cascades and the sweep's intervals", async () => {
  const { ADMIN_CASES } = await import("./testers_cases.ts");
  const byId = new Map(ADMIN_CASES.map((c) => [c.id, c]));
  assertEquals(["A3", "A3s"].map((id) => byId.get(id)?.deleteAccount?.as), ["owner", "service_role"]);
  for (const id of ["A3", "A3s", "A4", "A6"]) {
    const c = byId.get(id)!;
    assertEquals([c.rows?.length, c.expect.length], [c.accounts.length, c.accounts.length], id);
    c.rows!.forEach((r, i) => {
      const calls = c.calls.filter((k) => k.account === i);
      const granted = calls.filter((k) => k.fn === "comp_grant" && k.refused === undefined).length;
      const audit = calls.at(-1)?.audit.length ?? 0;
      const s = c.sweep?.find((x) => x.account === i);
      const swept = s && Math.max(s.endedAgo ?? -Infinity, -(s.endsIn ?? Infinity)) > 30 ? granted : 0;
      const years = /^3 years (-?\d+) days?$/.exec(s?.auditAgo ?? "");
      const aged = years !== null && Number(years[1]) > 0 ? audit : 0;
      const gone = c.deleteAccount?.account === i;
      const want = gone ? { grants: 0, audit: 0, unlinked: 0, routeDaily: 0 } : {
        grants: granted - swept, audit: audit - aged, unlinked: swept ? audit - aged : 0, routeDaily: c.routeDaily?.[i] ? 1 : 0,
      };
      assertEquals(r, want, `${id} account ${i}`);
      if (gone) assert(granted > 0 && c.routeDaily?.[i] && c.expect[i] === null, `${id}: both referential paths fire`);
    });
  }
  const a4 = byId.get("A4")!.sweep!;
  assert(a4.some((s) => s.endedAgo !== undefined && -(s.endsIn ?? 30) > s.endedAgo), "A4: least, not coalesce");
});

// L8 S2d (friends-testers §3.5, §4, §9 layer i, Q5, Q11): route_daily, a per-account count of gated
// calls in usage_daily's shape, cascading from accounts; and count_route, its one writer, one
// insert … on conflict, refusing a route outside §4's pattern (a check on the column), execute for
// the service role only. The service role keeps its rights on the table: the purge deletes from it.
const ROUTE_PATTERN = "^[a-z][a-z0-9-]{0,39}$";
const COUNT_ROUTE = "create function public.count_route(p_account uuid, p_route text) returns void language sql " +
  "set search_path = '' as $$ insert into public.route_daily (account_id, day, route, calls) " +
  "values (p_account, current_date, p_route, 1) on conflict (account_id, day, route) " +
  "do update set calls = route_daily.calls + 1; $$;";

export function routeDailyProblems(sql: string): string[] {
  const flat = flatten(sql);
  if (!/create table public\.route_daily \(/.test(flat)) return ["no route_daily"];
  const out: string[] = [];
  const cols = tableColumns(flat, "route_daily");
  if ([...cols.keys()].join() !== "account_id,day,route,calls") out.push(`columns ${[...cols.keys()]}`);
  if (cols.get("account_id") !== CASCADE) out.push("route_daily.account_id must cascade (A3, A3s)");
  if (cols.get("route") !== `route text not null check (route ~ '${ROUTE_PATTERN}')`) out.push("the route pattern (R1)");
  if (cols.get("calls") !== "calls integer not null default 0") out.push(`calls: ${cols.get("calls")}`);
  if (!tableItems(flat, "route_daily").includes("primary key (account_id, day, route)")) out.push("its key");
  if (!flat.includes("alter table public.route_daily enable row level security;")) out.push("RLS");
  if (!flat.includes("revoke all on public.route_daily from public, anon, authenticated;")) out.push("client revoke");
  if (!flat.includes(COUNT_ROUTE)) out.push("count_route is not one insert … on conflict, security invoker (R1)");
  if (!flat.includes("revoke execute on function public.count_route(uuid, text) from public, anon, authenticated;")) {
    out.push("count_route's revoke");
  }
  if (!flat.includes("grant execute on function public.count_route(uuid, text) to service_role;")) out.push("its grant");
  return out;
}

/** Across the corpus: no client grant or policy on route_daily, no revoke of the service role's
 * rights on it (the purge deletes from it), and no client execute on count_route. */
export function routeDailyCorpusProblems(files: Array<[string, string]>): string[] {
  const out: string[] = [];
  for (const [file, raw] of files) {
    const text = flatten(raw);
    // Doubled backslashes: in an ordinary string `\b` is a backspace, and these never matched (S2-S4R).
    const on = "on (?:table )?(?:public\\.)?route_daily\\b";
    if (new RegExp(`\\bgrant [^;]*${on}[^;]* to [^;]*\\b(anon|authenticated|public)\\b`).test(text)) out.push(`${file}: route_daily granted`);
    if (new RegExp(`create policy [^;]*${on}`).test(text)) out.push(`${file}: a policy on route_daily`);
    if (new RegExp(`\\brevoke [^;]*${on}[^;]* from [^;]*\\bservice_role\\b`).test(text)) out.push(`${file}: the purge deletes it`);
    if (/\bgrant execute on function [^;]*count_route\b[^;]* to [^;]*\b(anon|authenticated|public)\b/.test(text)) {
      out.push(`${file}: count_route granted`);
    }
  }
  return out;
}

Deno.test("S2d: route_daily is usage_daily's shape and cascades; count_route is one upsert, the service role's alone", async () => {
  const { name, sql } = await testersMigration();
  assertEquals(routeDailyProblems(sql), [], name);
  assertEquals(routeDailyCorpusProblems(await everyMigrationFile()), []);
});

Deno.test("S2-S4R fixups: each route_daily corpus pin fires on its own appended statement", async () => {
  const files = await everyMigrationFile();
  const { name } = await testersMigration();
  for (
    const [extra, want] of [
      ["grant select on public.route_daily to authenticated;", "route_daily granted"],
      ["grant all on table route_daily to anon, public;", "route_daily granted"],
      ["create policy p on public.route_daily for select using (true);", "a policy on route_daily"],
      ["revoke delete on public.route_daily from service_role;", "the purge deletes it"],
      ["grant execute on function public.count_route(uuid, text) to anon;", "count_route granted"],
    ]
  ) {
    const bad = files.map(([n, t]): [string, string] => [n, n === name ? `${t}\n${extra}\n` : t]);
    assertEquals(routeDailyCorpusProblems(bad), [`${name}: ${want}`], extra);
  }
});

Deno.test("S2d: the route pins refuse a definer, a second statement, no conflict clause, a looser pattern, no cascade", async () => {
  const flat = flatten((await testersMigration()).sql);
  for (
    const [from, to] of [
      ["p_route text) returns void language sql set", "p_route text) returns void language sql security definer set"],
      ["as $$ insert into public.route_daily", "as $$ delete from public.route_daily where false; insert into public.route_daily"],
      [" do update set calls = route_daily.calls + 1;", " do nothing;"],
      [ROUTE_PATTERN, "^[a-z0-9_-]+$"],
      ["references public.accounts(id) on delete cascade, day", "references public.accounts(id), day"],
      ["count_route(uuid, text) from public, anon, authenticated;", "count_route(uuid, text) from public;"],
    ]
  ) {
    const bad = flat.replace(from, to);
    assertNotEquals(bad, flat, `the mutation ${from} must apply to the real SQL`);
    assert(routeDailyProblems(bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S2d: the case table's R1 row counts twice, refuses a route outside the pinned pattern, denies authenticated", async () => {
  const { ROUTE_CASES } = await import("./testers_cases.ts");
  assertEquals(ROUTE_CASES.map((c) => c.id), ["R1"]);
  // The pattern the migration's route column checks, so the row and the SQL cannot drift apart.
  const check = /\broute text not null check \(route ~ '([^']+)'\)/.exec(flatten((await testersMigration()).sql));
  assert(check !== null, "route_daily.route carries the route pattern (§4)");
  const pattern = new RegExp(check[1]);
  for (const c of ROUTE_CASES) {
    assert(c.accounts.length === 1 && c.accounts[0].email.endsWith("@example.invalid"), c.id);
    const counted: Record<string, number> = {};
    for (const s of c.steps) {
      const want = s.as === "authenticated" ? "permission denied" : pattern.test(s.route) ? undefined : "check constraint";
      assertEquals(s.refused, want, `${c.id} ${s.as} ${s.route}`);
      if (want === undefined) counted[s.route] = (counted[s.route] ?? 0) + 1;
    }
    assertEquals(c.calls, counted, c.id);
  }
  for (const r of ["judge-task", "a", "x".repeat(40)]) assert(pattern.test(r), r);
  for (const r of ["", "Judge", "1judge", "judge_task", "x".repeat(41), "-x"]) assert(!pattern.test(r), r);
});

// L8 S2e1 (friends-testers §5, §9 layer i, Amendment 2026-10-20): admin.testers, the list Quinn
// grants and ends from, one row per grant. Its state is derived at read time in §5's order (ended,
// expired, live) from the negations of effective_entitlements' own grant match, and `paid` is the
// complement of the view's grant branch, so a grant is live exactly when it entitles its account
// unless a paid row wins. The address is read from accounts when the view is read, never stored.
const TESTERS_ITEMS = [
  "g.account_id",
  "a.email",
  "g.cohort",
  "case when g.ended_at is not null then 'ended' when now() >= g.ends_at then 'expired' else 'live' end as state",
  "coalesce(e.status, 'none') in ('active', 'trialing') as paid",
  "g.granted_at",
  "g.ends_at",
  "greatest((select max(u.day) from public.usage_daily u where u.account_id = g.account_id), " +
  "(select max(r.day) from public.route_daily r where r.account_id = g.account_id)) as last_active_day",
];
const TESTERS_FROM = "from public.comp_grants g left join public.accounts a on a.id = g.account_id " +
  "left join public.entitlements e on e.account_id = g.account_id;";
/** effective_entitlements' grant predicates (S2b's pins), each beside the negation admin.testers carries. */
const NEGATED: Array<[string, string]> = [
  ["g.ended_at is null", "case when g.ended_at is not null then 'ended'"],
  ["now() < g.ends_at", "when now() >= g.ends_at then 'expired' else 'live'"],
  ["coalesce(e.status, 'none') not in ('active', 'trialing')", "coalesce(e.status, 'none') in ('active', 'trialing') as paid"],
];
const TESTERS_REVOKE = "revoke all on admin.testers from public, anon, authenticated, service_role;";

export function testersViewProblems(sql: string): string[] {
  const flat = flatten(sql);
  const stmt = /create (?:or replace )?view admin\.testers as [^;]*;/.exec(flat)?.[0] ?? "";
  if (stmt === "") return ["no create view admin.testers"];
  const out: string[] = [];
  const items = selectItems(stmt);
  if (items.join(" | ") !== TESTERS_ITEMS.join(" | ")) out.push(`select list ${items.join(" | ")}`);
  if (stmt !== `create view admin.testers as select ${TESTERS_ITEMS.join(", ")} ${TESTERS_FROM}`) {
    out.push("not one row per grant: comp_grants left join accounts and entitlements, no where");
  }
  const ee = eeStatement(sql);
  for (const [view, mine] of NEGATED) {
    if (!ee.includes(view) || !stmt.includes(mine)) out.push(`not the view's own predicate: ${view} / ${mine}`);
  }
  if (!flat.includes(TESTERS_REVOKE)) out.push("its revoke");
  return out;
}

Deno.test("S2e1: admin.testers is one row per grant, §5's states from the view's own predicates, paid, revoked", async () => {
  const { name, sql } = await testersMigration();
  assertEquals(testersViewProblems(sql), [], name);
  for (const [file, raw] of await everyMigrationFile()) {
    assert(!/\bgrant [^;]* on (?:table )?admin\.testers\b/.test(flatten(raw)), `${file}: a grant on admin.testers`);
  }
});

Deno.test("S2e1: the testers pins refuse a moved state order, a shifted boundary, a narrower paid, a join, a where", async () => {
  const flat = flatten((await testersMigration()).sql);
  for (
    const [from, to] of [
      ["when g.ended_at is not null then 'ended' when now() >= g.ends_at then 'expired'", "when now() >= g.ends_at then 'expired' when g.ended_at is not null then 'ended'"],
      ["when now() >= g.ends_at then", "when now() > g.ends_at then"],
      ["coalesce(e.status, 'none') in ('active', 'trialing') as paid", "e.status = 'active' as paid"],
      ["left join public.accounts a on a.id = g.account_id", "join public.accounts a on a.id = g.account_id"],
      ["on e.account_id = g.account_id;", "on e.account_id = g.account_id where g.ended_at is null;"],
      ["admin.testers from public, anon, authenticated, service_role;", "admin.testers from public;"],
      ["a.email,", "(select u.email from auth.users u where u.id = g.account_id) as email,"],
    ]
  ) {
    const bad = flat.replace(from, to);
    assertNotEquals(bad, flat, `the mutation ${from} must apply to the real view`);
    assert(testersViewProblems(bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S2e1: the case table's state column: P2 live and paid, P8 expired, P9 ended, P13 expired then live", async () => {
  const { CASES } = await import("./testers_cases.ts");
  const { grantState, isPaid, run } = await import("./testers_model.ts");
  const byId = new Map(CASES.map((c) => [c.id, c]));
  assertEquals(CASES.filter((c) => c.testers !== undefined).map((c) => c.id).sort(), ["P13", "P2", "P8", "P9"]);
  assertEquals(byId.get("P2")!.testers, [{ state: "live", paid: true }]);
  assertEquals(byId.get("P8")!.testers, [{ state: "expired", paid: false }]);
  assertEquals(byId.get("P9")!.testers, [{ state: "ended", paid: false }]);
  assertEquals(byId.get("P13")!.testersBefore?.map((r) => r.state), ["expired", "ended"]);
  assertEquals(byId.get("P13")!.testers?.map((r) => r.state), ["live", "ended"]);
  for (const c of CASES.filter((c) => c.testers !== undefined)) {
    const out = run(c);
    const rows = (gs: typeof out.grants) => gs.map((g) => ({ state: grantState(g), paid: isPaid(c.accounts[g.account].row) }));
    assertEquals(rows(out.grantsBefore), c.testersBefore ?? c.testers, `${c.id} before its calls`);
    assertEquals(rows(out.grants), c.testers, `${c.id} at the end`);
  }
});

// L8 S2e2 (friends-testers §5, §9 layer i; the controller's ruling of 2026-10-20 on est_usd, option
// (b)): the two cost views Quinn reads. tester_spend_monthly sums monthly_spend's own usd per tester
// and month, so est_usd is monthly_spend's value and is never re-priced in this file (which must not
// name the pricing table: CI's eval gate greps migrations for it). tester_usage_daily holds counts and
// no dollars. Both cover only accounts that hold or held a grant, and name no item, field or message.
const dayOf = (t: string, col: string) => `(${t}.${col} at time zone 'utc')::date`;
const onDay = (t: string, day: string) => `where ${t}.account_id = d.account_id and ${day} = d.day`;
const COUNTED = (t: string, table: string, key: string, col: string) =>
  `(select jsonb_object_agg(${t}.${key}, ${t}.n) from (select ${col}.${key}, count(*) as n from public.${table} ${col} ` +
  `${onDay(col, dayOf(col, table === "judgments" ? "judged_at" : "seen_at"))} group by ${col}.${key}) ${t})`;
const USAGE_VIEW = "create view admin.tester_usage_daily as select d.account_id, d.day, " +
  "(select jsonb_object_agg(u.kind, jsonb_build_object('calls', u.calls, 'refunds', u.refunds, 'in_tokens', u.in_tokens, " +
  `'out_tokens', u.out_tokens)) from public.usage_daily u ${onDay("u", "u.day")}) as usage_by_kind, ` +
  `${COUNTED("t", "judgments", "tier", "j")} as judgments_by_tier, ${COUNTED("p", "mail_seen", "phase", "m")} as mail_by_phase, ` +
  `(select count(*) from public.gmail_seen s ${onDay("s", dayOf("s", "seen_at"))}) as gmail_messages, ` +
  `(select jsonb_object_agg(r.route, r.calls) from public.route_daily r ${onDay("r", "r.day")}) as calls_by_route ` +
  `from (select u.account_id, u.day from public.usage_daily u union select j.account_id, ${dayOf("j", "judged_at")} ` +
  `from public.judgments j union select m.account_id, ${dayOf("m", "seen_at")} from public.mail_seen m ` +
  `union select s.account_id, ${dayOf("s", "seen_at")} from public.gmail_seen s union select r.account_id, r.day ` +
  "from public.route_daily r) d where d.account_id in (select g.account_id from public.comp_grants g);";
const SPEND_VIEW = "create view admin.tester_spend_monthly as select x.account_id, x.month, x.is_total, x.est_usd, " +
  "b.usd as alert_usd, b.raised_at as alert_raised_at from (select ms.account_id, ms.month::date as month, " +
  "grouping(ms.account_id) = 1 as is_total, sum(ms.usd) as est_usd from public.monthly_spend ms " +
  "where ms.account_id in (select g.account_id from public.comp_grants g) " +
  "group by grouping sets ((ms.month::date, ms.account_id), (ms.month::date))) x " +
  "left join public.budget_alerts b on b.account_id = x.account_id and b.month = x.month;";
const LEAKS = ["item_id", "fields", "title", "sender", "subject", "body", "payload", "uid", "msg_hash", "fp_hash", "address_id", "received_minute", "email", "note"];

export function costViewsProblems(sql: string): string[] {
  const flat = flatten(sql);
  const out: string[] = [];
  const stmts: Record<string, string> = {};
  for (const [name, want] of [["tester_usage_daily", USAGE_VIEW], ["tester_spend_monthly", SPEND_VIEW]]) {
    const stmt = stmts[name] = new RegExp(`create (?:or replace )?view admin\\.${name} as [^;]*;`).exec(flat)?.[0] ?? "";
    if (stmt !== want) out.push(`admin.${name} is not the pinned view: ${stmt || "absent"}`);
    for (const w of LEAKS) if (new RegExp(`\\b${w}\\b`).test(stmt)) out.push(`admin.${name} names ${w}`);
    if (!flat.includes(`revoke all on admin.${name} from public, anon, authenticated, service_role;`)) out.push(`${name}'s revoke`);
  }
  if (/usd/.test(stmts.tester_usage_daily)) out.push("tester_usage_daily carries dollars (ruled out, option (b))");
  const spend = stmts.tester_spend_monthly;
  if (!spend.includes("sum(ms.usd) as est_usd from public.monthly_spend ms ")) out.push("est_usd is not monthly_spend's own usd");
  if (/usd_per_m|_tokens/.test(spend)) out.push("est_usd re-priced here, not monthly_spend's value");
  return out;
}

Deno.test("S2e2: the cost views are §5's as ruled: counts per day, monthly_spend's dollars per month, revoked", async () => {
  const { name, sql } = await testersMigration();
  assertEquals(costViewsProblems(sql), [], name);
  const raw = (await everyMigrationFile()).find(([n]) => n === name)![1];
  assert(!/\bmodels\b/i.test(raw), `${name}: names the pricing table, which starts the paid eval (ci.yml)`);
  for (const [file, text] of await everyMigrationFile()) {
    assert(!/\bgrant [^;]* on (?:table )?admin\.tester_(?:usage_daily|spend_monthly)\b/.test(flatten(text)), `${file}: a grant`);
  }
});

Deno.test("S2e2: the cost pins refuse a message key, dollars per day, a local day, every account, a share, no total, no alert", async () => {
  const flat = flatten((await testersMigration()).sql);
  for (
    const [from, to] of [
      ["(select count(*) from public.gmail_seen s", "(select jsonb_agg(s.uid) from public.gmail_seen s"],
      ["as calls_by_route from", "as calls_by_route, 0.0 as est_usd from"],
      ["(j.judged_at at time zone 'utc')::date = d.day", "j.judged_at::date = d.day"],
      [" from public.route_daily r) d where d.account_id in (select g.account_id from public.comp_grants g);", " from public.route_daily r) d;"],
      ["sum(ms.usd) as est_usd", "sum(ms.usd) / 30 as est_usd"],
      ["group by grouping sets ((ms.month::date, ms.account_id), (ms.month::date))", "group by ms.month::date, ms.account_id"],
      ["left join public.budget_alerts b", "join public.budget_alerts b"],
      ["admin.tester_spend_monthly from public, anon, authenticated, service_role;", "admin.tester_spend_monthly from public;"],
    ]
  ) {
    const bad = flat.replace(from, to);
    assertNotEquals(bad, flat, `the mutation ${from} must apply to the real views`);
    assert(costViewsProblems(bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S2-S4R fixups: the LEAKS pin names each leaked column, in either view, by itself", async () => {
  const flat = flatten((await testersMigration()).sql);
  for (const w of LEAKS) {
    for (
      const [view, from, to] of [
        ["tester_usage_daily", "(select count(*) from public.gmail_seen s", `(select count(s.${w}) from public.gmail_seen s`],
        ["tester_spend_monthly", "b.usd as alert_usd,", `b.usd as alert_usd, b.${w},`],
      ]
    ) {
      const bad = flat.replace(from, to);
      assertNotEquals(bad, flat, `the mutation ${from} must apply`);
      const problems = costViewsProblems(bad);
      assert(problems.includes(`admin.${view} names ${w}`), `${view} + ${w}: ${problems.join(" | ")}`);
    }
  }
});

Deno.test("S2e2: the case table's V1 and V2 rows: est_usd the hand-computed sum, the total row the testers', the alert on its month", async () => {
  const { SPEND_CASES } = await import("./testers_cases.ts");
  assertEquals(SPEND_CASES.map((c) => c.id), ["V1", "V2"]);
  const kinds = ["task", "event", "email", "mail_claim", "mail_resolve", "mail_digest"]; // usage_daily_kind_check
  const nano = (x: number) => Math.round(x * 1e9) / 1e9;
  const seen = new Set<string>();
  for (const c of SPEND_CASES) {
    for (const a of c.accounts) assert(a.email.endsWith("@example.invalid"), c.id);
    c.accounts.forEach((_, i) => assert(c.grants.some((g) => g.account === i), `${c.id}: account ${i} is a tester`));
    const months = [...new Set(c.usage.map((u) => `${u.day.slice(0, 8)}01`))];
    for (const m of months) {
      assert(!seen.has(m) && m < "2026-01-01", `${c.id}: ${m} is its own month, before any real usage`);
      seen.add(m);
    }
    for (const x of c.alerts ?? []) assert(months.includes(x.month), `${c.id}: an alert shows on a month with spend`);
    const want: typeof c.spend = [];
    for (const m of months) {
      let total = 0;
      c.accounts.forEach((_, i) => {
        const rows = c.usage.filter((u) => u.account === i && `${u.day.slice(0, 8)}01` === m);
        if (rows.length === 0) return;
        for (const u of rows) assert(kinds.includes(u.kind) && c.prices[u.kind] !== undefined, `${c.id}: ${u.kind}`);
        const usd = rows.reduce((s, u) => s + (u.inTokens * c.prices[u.kind].in + u.outTokens * c.prices[u.kind].out) / 1e6, 0);
        total += usd;
        const alert = c.alerts?.find((x) => x.account === i && x.month === m)?.usd ?? null;
        want.push({ account: i, month: m, estUsd: nano(usd), alertUsd: alert });
      });
      want.push({ account: "total", month: m, estUsd: nano(total), alertUsd: null });
    }
    assertEquals(c.spend.map((r) => ({ ...r, estUsd: nano(r.estUsd) })), want, c.id);
    const days = c.accounts.map((_, i) => new Set(c.usage.filter((u) => u.account === i).map((u) => u.day)).size);
    assertEquals(c.usageDays, days, `${c.id}: one tester_usage_daily row per day with usage`);
  }
  const v1 = SPEND_CASES[0];
  const priced = new Set(v1.usage.map((u) => `${v1.prices[u.kind].in}/${v1.prices[u.kind].out}`));
  assert(priced.size >= 2, "V1 spans two prices, so monthly_spend gives two rows and the view must sum them");
  assertEquals([SPEND_CASES[1].accounts.length, SPEND_CASES[1].alerts?.length], [2, 1], "V2: two testers, one alert");
});

// L8 S2f-a (friends-testers §9 layer iii, Amendment 2026-10-20): the staging proof, the prod gate. Not
// a migration (proofs/, never deployed): one transaction that only rolls back, its first statement the
// trigger precheck, every account made by a real sign-up's triggers from S1's recorded auth.users
// columns with no password, only @example.invalid addresses and no account id, and its P cases the
// case table's own rows, verbatim, so the cases are written once and run twice.
const PROOF = new URL("../proofs/testers_staging.sql", HERE);
/** S1's ledger row: the auth.users columns GoTrue fills on insert on staging. */
const S1_COLUMNS = new Set(("aud confirmation_token confirmed_at created_at email email_change email_change_confirm_status " +
  "email_change_token_current email_change_token_new email_confirmed_at id instance_id is_anonymous is_sso_user " +
  "last_sign_in_at phone_change phone_change_token raw_app_meta_data raw_user_meta_data reauthentication_token " +
  "recovery_token role updated_at recovery_sent_at").split(" "));

/** Top-level statements, flattened: line comments dropped, dollar-quoted bodies blanked to `$$`. */
const topLevel = (raw: string) =>
  stripLineComments(raw).replace(/(\$[A-Za-z_]*\$)[\s\S]*?\1/g, () => "$$").split(";")
    .map((s) => s.replace(/\s+/g, " ").trim().toLowerCase()).filter((s) => s !== "");

export function proofProblems(raw: string, cases: unknown): string[] {
  const out: string[] = [];
  const code = stripLineComments(raw);
  const stmts = topLevel(raw);
  if (stmts[0] !== "begin") out.push("does not open with begin;");
  if (stmts.at(-1) !== "rollback") out.push("does not close with rollback;");
  for (const s of stmts.slice(1, -1)) {
    if (/^(?:begin|start|end|commit|abort|prepare|rollback(?! to\b))\b/.test(s)) out.push(`ends the transaction early: ${s}`);
  }
  if (/\bcommit\b/i.test(raw)) out.push("names commit");
  for (const m of raw.matchAll(ADDRESS_TOKEN)) if (m[0].split("@")[1] !== "example.invalid") out.push(`address ${m[0]}`);
  if (uuidTokens([["proof", raw]]).length > 0) out.push("an id other than the nil uuid");
  const pre = /^\s*begin;\s*do\s+\$\$([\s\S]*?)\$\$\s*;/.exec(code)?.[1] ?? "";
  if (!["handle_new_user", "prosrc", "'raise'", "raise exception"].every((w) => pre.includes(w))) out.push("no trigger precheck first");
  const users = [...code.matchAll(/insert\s+into\s+auth\.users\s*\(([^)]*)\)/gi)];
  if (users.length !== 1) out.push(`${users.length} auth.users inserts, want one`);
  for (const col of users[0]?.[1].split(",").map((c) => c.trim()) ?? []) if (!S1_COLUMNS.has(col)) out.push(`column ${col} is not S1's`);
  if (/encrypted_password/i.test(code)) out.push("sets a password");
  if (/insert\s+into\s+(?:public\.)?(?:accounts|entitlements)\b/i.test(code)) out.push("inserts what the sign-up trigger writes");
  const blob = /\$cases\$([\s\S]*?)\$cases\$/.exec(code)?.[1];
  try {
    assertEquals(JSON.parse(blob ?? "null"), cases);
  } catch {
    out.push("the P cases are not testers_cases.ts's CASES");
  }
  return out;
}

Deno.test("S2f-a: the staging proof rolls back, prechecks the trigger, signs up with no password, runs CASES verbatim", async () => {
  const { CASES } = await import("./testers_cases.ts");
  assertEquals(proofProblems(await Deno.readTextFile(PROOF), CASES), []);
});

Deno.test("S2f-a: the proof pins refuse a commit, an early end, an address, an id, a password, a direct insert, a drift", async () => {
  const { CASES } = await import("./testers_cases.ts");
  const raw = await Deno.readTextFile(PROOF);
  for (
    const [bad, want] of [
      [raw.replace(/rollback;\s*$/, "commit;\n"), "close with rollback"],
      [raw.replace("begin;\n", "begin;\nend;\nbegin;\n"), "ends the transaction early"],
      [raw.replace(/rollback;\s*$/, "rollback to savepoint x;\nrollback;\nselect 1;\n"), "close with rollback"],
      [`${raw}-- friend@example.com\n`, "address"],
      [`${raw}-- ${ID}\n`, "nil uuid"],
      [raw.replace("is_sso_user)", "is_sso_user, encrypted_password)"), "password"],
      [raw.replace(/rollback;\s*$/, "insert into public.entitlements (account_id, status) select id, 'none' from public.accounts;\nrollback;\n"), "sign-up trigger"],
      [raw.replace('"endsIn":30', '"endsIn":31'), "CASES"],
      [raw.replace("prosrc", "proname"), "precheck"],
    ]
  ) {
    assertNotEquals(bad, raw, `the mutation for ${want} must apply to the real proof`);
    const problems = proofProblems(bad, CASES);
    assert(problems.some((p) => p.includes(want)), `${want}: ${problems.join(" | ")}`);
  }
});

// L8 S2f-b (friends-testers §9 layer iii; V1 restated by the controller's ruling of 2026-10-20 on
// est_usd): the proof's A, R and V cases are the case table's own ADMIN_CASES, ROUTE_CASES and
// SPEND_CASES, verbatim, each run by its own runner. A7's statements and A3s's deletion run as the
// service role in a savepoint that sets the role local and puts the caller's role back, and the
// price pin the V cases set is only ever updated (the whole proof rolls back).
const PROOF_TABLES = [["admin", "a_case", "ADMIN_CASES"], ["route", "r_case", "ROUTE_CASES"], ["spend", "v_case", "SPEND_CASES"]] as const;
const ROLE_SWITCHES = ["set local role %I', p_role)", "set local role %I', v_prev)"];

export function proofTableProblems(raw: string, tables: Record<string, unknown>): string[] {
  const out: string[] = [];
  const code = stripLineComments(raw);
  for (const [tag, runner, name] of PROOF_TABLES) {
    const m = new RegExp(String.raw`select pg_temp\.run\('${runner}', \$${tag}\$([\s\S]*?)\$${tag}\$::jsonb\);`).exec(code);
    if (!m) {
      out.push(`${name} is not run by pg_temp.${runner}`);
      continue;
    }
    try {
      assertEquals(JSON.parse(m[1]), tables[name]);
    } catch {
      out.push(`the rows are not testers_cases.ts's ${name}`);
    }
  }
  if (!code.includes("as_role('service_role', 'delete from public.accounts where id = $1'")) out.push("A3s does not delete as service_role");
  if (!code.includes("as_role('service_role', pg_temp.a7_sql(")) out.push("A7 does not run as service_role");
  const roles = [...code.matchAll(/\b(?:re)?set\s+(?:local\s+)?role\b[^;]*/gi)].map((m) => m[0].replace(/\s+/g, " "));
  if (JSON.stringify(roles) !== JSON.stringify(ROLE_SWITCHES) || !code.includes("v_prev text := current_user")) {
    out.push(`role switches ${roles.join(" | ")}: want one set local role and one restore of the caller's role`);
  }
  if (/\b(?:insert\s+into|delete\s+from|truncate(?:\s+table)?)\s+(?:public\.)?models\b/i.test(code)) out.push("writes the price pin other than by update");
  return out;
}

Deno.test("S2f-b: the staging proof runs ADMIN_CASES, ROUTE_CASES and SPEND_CASES verbatim, the service role's steps in a savepoint", async () => {
  const tables = await import("./testers_cases.ts");
  assertEquals(proofTableProblems(await Deno.readTextFile(PROOF), tables), []);
});

Deno.test("S2f-b: the proof pins refuse a drifted A, R or V row, a wrong runner, A3s as the owner, an unrestored role, a price write", async () => {
  const tables = await import("./testers_cases.ts");
  const raw = await Deno.readTextFile(PROOF);
  assertEquals(proofTableProblems(raw, tables), [], "each mutation below is the only problem");
  for (
    const [from, to, want] of [
      ['"auditAgo":"3 years 1 day"', '"auditAgo":"3 years"', "not testers_cases.ts's ADMIN_CASES"],
      ['"calls":{"judge-task":2}', '"calls":{"judge-task":3}', "not testers_cases.ts's ROUTE_CASES"],
      ['"estUsd":3,', '"estUsd":4,', "not testers_cases.ts's SPEND_CASES"],
      ["select pg_temp.run('v_case', $spend$", "select pg_temp.run('a_case', $spend$", "SPEND_CASES is not run by pg_temp.v_case"],
      ["'delete from public.accounts where id = $1'", "'delete from auth.users where id = $1'", "A3s does not delete as service_role"],
      ["execute format('set local role %I', v_prev);", "reset role;", "role switches"],
      ["update public.models set", "delete from public.models where false; update public.models set", "price pin"],
    ]
  ) {
    const bad = raw.replace(from, () => to);
    assertNotEquals(bad, raw, `the mutation for ${want} must apply to the real proof`);
    const problems = proofTableProblems(bad, tables);
    assert(problems.length === 1 && problems[0].includes(want), `${want}: ${problems.join(" | ")}`);
  }
});

// L3 T1, the first-day event allowance (spec 2026-10-06 §2, §3 step 3, §5; §9 tests 1-7). Textual,
// as the mail allowance's pins are: nothing here applies SQL (plan Q-C), so these pins are the only
// proof of the 14-day expiry, the no-reopening rule, the refund bounds and the concurrency shape. The
// file is found by its suffix, never its stamp (stamp rule M4), so a rename moves no line here.
const EVENT_ALLOWANCE = /^\d{14}_event_allowance\.sql$/;

/** The one `<stamp>_event_allowance.sql`: its raw text, and comment-stripped, whitespace-normalised,
 *  lower-cased. */
async function eventAllowanceMigration(): Promise<{ name: string; raw: string; sql: string }> {
  const files = (await everyMigrationFile()).filter(([n]) => EVENT_ALLOWANCE.test(n));
  assertEquals(files.length, 1, "exactly one <stamp>_event_allowance.sql (plan T1)");
  const [name, raw] = files[0];
  return { name, raw, sql: stripLineComments(raw).replace(/\s+/g, " ").trim().toLowerCase() };
}

/** `fn`'s header (through `as $tag$`) and body (inside the tags), normalised as above, from its LAST
 *  definition in the corpus, which must be the event-allowance migration's. */
async function allowanceFunction(fn: string): Promise<{ name: string; sql: string; header: string; body: string }> {
  const { name, sql } = await eventAllowanceMigration();
  const last = await lastFunction(fn);
  assertEquals(last.name, name, `${fn}: the event-allowance migration's definition governs`);
  const m = new RegExp(FUNCTION_DEFINITION.source, "i").exec(last.text)!;
  const rest = m[0].slice(m[1].length);
  const flat = (s: string) => s.replace(/\s+/g, " ").trim().toLowerCase();
  return { name, sql, header: flat(m[1]), body: flat(rest.slice(0, rest.indexOf(m[3]))) };
}

/** The text strictly between `from` and the first `to` after it. */
function between(text: string, from: string, to: string): string {
  const a = text.indexOf(from);
  assert(a !== -1, `'${from}' not found`);
  const b = text.indexOf(to, a + from.length);
  assert(b !== -1, `'${to}' not found after '${from}'`);
  return text.slice(a + from.length, b).trim();
}

Deno.test("event_allowance_opens_on_the_first_overflow_with_2000_minus_one_and_14_days", async () => {
  const f = await allowanceFunction("charge_judge_allowance");
  assert(/charge_judge_allowance\s*\(\s*p_account uuid, p_kind text\s*\)/.test(f.header), `${f.name}: (p_account uuid, p_kind text)`);
  assert(f.header.includes("returns boolean"), `${f.name}: answers charged or not, as charge_call`);
  assert(f.header.includes("language plpgsql"), `${f.name}: plpgsql`);
  // The opening row: now, 14 days, the size written as `2000 - 1` (T2a's size mirror reads that form).
  assert(
    f.body.includes(
      "insert into public.judge_allowances (account_id, kind, opened_at, until, calls_left, refunds) " +
        "values (p_account, p_kind, now(), now() + interval '14 days', 2000 - 1, 0) " +
        "on conflict (account_id, kind) do update",
    ),
    `${f.name}: the one insert opens the row with now(), 14 days, 2000 - 1 and no refunds`,
  );
  assert(f.body.includes("returning true into charged;"), `${f.name}: charged only when the row was written`);
  assert(f.body.includes("return coalesce(charged, false);"), `${f.name}: no row written answers false`);
});

Deno.test("event_allowance_is_one_time_and_never_reopens", async () => {
  const f = await allowanceFunction("charge_judge_allowance");
  // The conflict arm sets only `calls_left`, and only while the row is open and not spent. Everything
  // between `do update` and `returning` is compared whole, so an extra `or`, a reordered precedence or
  // a second `set` target fails here. The row's own columns stay qualified: `excluded` is in scope too.
  assertEquals(
    between(f.body, "do update", "returning"),
    "set calls_left = judge_allowances.calls_left - 1 " +
      "where judge_allowances.until > now() and judge_allowances.calls_left > 0",
    `${f.name}: the conflict arm decrements only, and only while open`,
  );
  // No renewal: mail's `charge_allowance` reopens after 120 days; this one never does.
  const { name, raw } = await eventAllowanceMigration();
  assert(!/interval\s+'120 days'/i.test(raw), `${name}: no 120-day renewal`);
  // One-time means "no row", never "row expired": nothing but the account cascade ever deletes it.
  for (const [file, text] of await everyMigrationFile()) {
    const code = stripLineComments(text);
    assert(
      !/\b(?:delete\s+from|truncate(?:\s+table)?)\s+(?:only\s+)?(?:"?public"?\s*\.\s*)?"?judge_allowances"?/i.test(code),
      `${file}: no migration deletes from judge_allowances`,
    );
  }
});

Deno.test("charge_judge_allowance_is_one_statement_on_a_unique_key", async () => {
  // Why one statement is the concurrency proof (spec test 3): two first charges race to insert the
  // same `(account_id, kind)`. The second waits on the primary key, then takes the `do update` arm,
  // and Postgres re-checks its `where` on the locked row; two charges against `calls_left = 1`
  // serialise the same way. An `update`, then an `insert` when not found, would race instead: the
  // loser hits the key, throws, and `chargeAllowance` answers false while the allowance had room.
  // The house count of `\b(insert|update|delete)\b` is 2 for an upsert, so the shape is pinned instead.
  const f = await allowanceFunction("charge_judge_allowance");
  assert(f.header.includes("security invoker"), `${f.name}: security invoker, as charge_call`);
  assert(!f.header.includes("security definer"), `${f.name}: never security definer`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertEquals((f.body.match(/\binsert\b/g) ?? []).length, 1, `${f.name}: one insert`);
  assert(f.body.includes("insert into public.judge_allowances"), `${f.name}: the insert is the allowance row's`);
  assertEquals((f.body.match(/\bupdate\b/g) ?? []).length, 1, `${f.name}: one update`);
  assert(f.body.includes("on conflict (account_id, kind) do update"), `${f.name}: the update is the conflict arm`);
  assert(!/\bupdate\s+(?:public\.)?judge_allowances\b/.test(f.body), `${f.name}: no standalone update`);
  for (const word of ["delete", "select", "if", "found"]) {
    assert(!new RegExp(`\\b${word}\\b`).test(f.body), `${f.name}: no '${word}' (no read, no branch on a prior result)`);
  }
  const statements = between(f.body, " begin ", " end;").split(";").map((s) => s.trim()).filter((s) => s !== "");
  assertEquals(statements.length, 2, `${f.name}: the insert and the return, nothing else`);
  assert(statements[0].startsWith("insert into public.judge_allowances"), `${f.name}: the insert first`);
  assertEquals(statements[1], "return coalesce(charged, false)", `${f.name}: then the return`);
  // The conflict target is the table's primary key, so the conflict arm is a row lock.
  const { name, raw } = await eventAllowanceMigration();
  assert(tableItems(stripLineComments(raw), "judge_allowances").includes("primary key (account_id, kind)"), `${name}: the key`);
});

Deno.test("judge_allowances_kind_is_checked_to_event", async () => {
  const { name, raw } = await eventAllowanceMigration();
  const kind = tableColumns(stripLineComments(raw), "judge_allowances").get("kind");
  assertEquals(kind, "kind text not null check (kind in ('event'))", `${name}: events only (spec Q4)`);
  // And the corpus's live check is still this one: no later lane widens it (plan Q-F, L5's Q41).
  const last = await lastCheckWords("judge_allowances", "kind");
  assertEquals(last.words, ["event"], `${last.name}: judge_allowances.kind admits 'event' alone`);
});

Deno.test("refund_judge_allowance_gives_back_only_while_open_and_within_both_bounds", async () => {
  // Spec §5: a `model failed` call the allowance charged is given back, but only while the allowance
  // is open, never past its size, and never more than its size over its life. The lifetime `refunds`
  // bound is what stops a device on a dead provider charging, failing and refunding forever: each
  // `/judge-event` request is its own invocation, so F1's breaker never trips across a run.
  const f = await allowanceFunction("refund_judge_allowance");
  assert(/refund_judge_allowance\s*\(\s*p_account uuid, p_kind text\s*\)/.test(f.header), `${f.name}: (p_account uuid, p_kind text)`);
  assert(f.header.includes("returns boolean"), `${f.name}: answers refunded or not, as refund_call`);
  assert(f.header.includes("language plpgsql"), `${f.name}: plpgsql`);
  assert(f.header.includes("security invoker") && !f.header.includes("security definer"), `${f.name}: security invoker`);
  assert(f.header.includes("set search_path = public, extensions"), `${f.name}: a pinned search_path`);
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 1, `${f.name}: one write`);
  assert(!/\binsert\b/.test(f.body), `${f.name}: never opens a missing row`);
  assert(!/\bselect\b/.test(f.body), `${f.name}: no read before the write`);
  assertEquals(
    between(f.body, "update public.judge_allowances", "returning"),
    "set calls_left = calls_left + 1, refunds = refunds + 1 " +
      "where account_id = p_account and kind = p_kind and until > now() and calls_left < 2000 and refunds < 2000",
    `${f.name}: one back and one counted, only while open and inside both bounds`,
  );
  assert(!/opened_at|until =/.test(f.body), `${f.name}: never sets opened_at or until`);
  assert(f.body.includes("returning true into done;"), `${f.name}: refunded only when the row was written`);
  assert(f.body.includes("return coalesce(done, false);"), `${f.name}: no row written answers false`);
});

Deno.test("judge_allowances_is_service_role_only_and_purged_with_the_account", async () => {
  const { name, raw, sql } = await eventAllowanceMigration();
  assert(sql.includes("alter table public.judge_allowances enable row level security;"), `${name}: RLS on`);
  assert(sql.includes("revoke all on public.judge_allowances from anon, authenticated;"), `${name}: no client grant`);
  for (const [file, text] of await everyMigrationFile()) {
    const code = stripLineComments(text).replace(/\s+/g, " ").toLowerCase();
    for (const g of code.matchAll(/grant [^;]* on (?:table )?(?:public\.)?judge_allowances to ([^;]+);/g)) {
      assert(!/\b(anon|authenticated|public)\b/.test(g[1]), `${file}: judge_allowances is granted to ${g[1]}`);
    }
    assert(!/create policy [^;]* on (?:public\.)?judge_allowances\b/.test(code), `${file}: no policy on judge_allowances`);
  }
  // The account purge removes it: the row lives exactly as long as the account.
  const account = tableColumns(stripLineComments(raw), "judge_allowances").get("account_id");
  assertEquals(account, "account_id uuid not null references public.accounts (id) on delete cascade", `${name}: purged with the account`);
  assertServiceRoleOnly({ name, sql }, "charge_judge_allowance", "uuid, text");
  assertServiceRoleOnly({ name, sql }, "refund_judge_allowance", "uuid, text");
});

Deno.test("the_event_allowance_migration_reissues_no_cap_function", async () => {
  // Spec Q5, Q6: `charge_call`, `refund_call` and `enforce_budget` keep their SQL, so their own pins
  // (the first-two-days test and F1's) still pass with no edit. The file defines its two functions only.
  const { name, raw } = await eventAllowanceMigration();
  const defined = [...stripLineComments(raw).matchAll(FUNCTION_DEFINITION)].map((m) => m[2].toLowerCase());
  assertEquals(defined, ["charge_judge_allowance", "refund_judge_allowance"], `${name}: its two functions and no other`);
  // `ci.yml`'s eval gate runs (and spends) for any migration naming the `models` table; this one
  // changes no pin, so its text, comments included, never uses that word on its own.
  assert(!/\bmodels\b/i.test(raw), `${name}: never names the models table`);
});

// Email cloud follow-ups, T2 (docs/plans/2026-10-20-email-cloud-follow-ups-plan.md §2 and T2; Quinn's Q3 and
// Q4, 2026-10-20). One forward-only file, found by its suffix and never by its stamp (M4): Rotate's prior
// status (`rotated_from`, D22's widened reading), one count per SES receipt (`count_batch_receipt`), and a
// held item's mailbox marked lapsed from its arrival (`mail_mark_lapsed_from`). Textual, like the suite.
const FOLLOW_UPS = /^\d{14}_mail_follow_ups\.sql$/;

/** The one `<stamp>_mail_follow_ups.sql`: its name, its raw text and its comment-stripped code. */
async function followUpsMigration(): Promise<{ name: string; raw: string; code: string }> {
  const files = (await everyMigrationFile()).filter(([n]) => FOLLOW_UPS.test(n));
  assertEquals(files.length, 1, "exactly one <stamp>_mail_follow_ups.sql (M4: found by suffix)");
  return { name: files[0][0], raw: files[0][1], code: stripLineComments(files[0][1]) };
}

/** As `mailFunction`, for a function whose LAST definition is the follow-ups file's. */
async function followUpsFunction(fn: string): Promise<{ name: string; sql: string; header: string; body: string }> {
  const { name, code } = await followUpsMigration();
  const last = await lastFunction(fn);
  assertEquals(last.name, name, `${fn}: the follow-ups migration's definition governs`);
  const m = new RegExp(FUNCTION_DEFINITION.source, "i").exec(last.text)!;
  const rest = m[0].slice(m[1].length);
  const flat = (s: string) => s.replace(/\s+/g, " ").trim().toLowerCase();
  return { name, sql: flat(code), header: flat(m[1]), body: flat(rest.slice(0, rest.indexOf(m[3]))) };
}

Deno.test("the_follow_ups_migration_is_found_by_suffix_and_stamped_after_every_other_file", async () => {
  const names = (await everyMigrationFile()).map(([n]) => n);
  const { name } = await followUpsMigration();
  // Forward-only (M2, M6): a new file after every applied one. A later migration is named here when it lands,
  // by its suffix, never its stamp (M4): L8's testers file, restamped after this one (C1, 2026-10-20),
  // and its S6 follow-up after that.
  assertEquals(
    names.filter((n) => n > name).map((n) => n.replace(/^\d{14}_(testers|comp_guards)\.sql$/, "<stamp>_$1.sql")),
    ["<stamp>_testers.sql", "<stamp>_comp_guards.sql"],
    `${name}: stamped after every other file but the named later ones`,
  );
  for (const applied of [MAIL_LAST_LAPSED_AT, MAIL_MAX_TOKENS_REPIN]) {
    assert(name > applied, `${name}: stamped after ${applied}, which stays as applied`);
  }
});

Deno.test("it_adds_rotated_from_with_its_status_check_and_counted_receipts_defaulting_empty", async () => {
  const { name, code } = await followUpsMigration();
  const flat = (s: string) => s.replace(/\s+/g, " ").trim().toLowerCase();
  // Every statement outside the two function definitions, exactly, in order.
  const statements = code.replace(FUNCTION_DEFINITION, "").split(";").map(flat).filter((s) => s !== "");
  // `rotated_from` admits exactly the statuses `mail_addresses.status` does (the mail migration's nine).
  const status = await lastCheckWords("mail_addresses", "status");
  assertEquals(status.words.length, 9, `${status.name}: the nine statuses`);
  const nine = status.words.map((w) => `'${w}'`).join(", ");
  assertEquals(statements, [
    // Nullable: a mailbox never rotated, or whose grace ended by canary, has none.
    "alter table public.mail_addresses add column if not exists rotated_from text",
    // One named check, and nothing ties it to the grace (review r1, M7): D22 reads it only while
    // `retired_until` is in the future, so a stale value is harmless.
    "alter table public.mail_addresses add constraint mail_addresses_rotated_from_status " +
    `check (rotated_from is null or rotated_from in (${nine}))`,
    // The SES receipts already counted toward `received`; empty for every existing batch.
    "alter table public.mail_batches add column if not exists counted_receipts text[] not null default '{}'",
    "revoke execute on function public.count_batch_receipt(uuid, uuid, text, integer) from public, anon, authenticated",
    "grant execute on function public.count_batch_receipt(uuid, uuid, text, integer) to service_role",
    "revoke execute on function public.mail_mark_lapsed_from(uuid, timestamptz) from public, anon, authenticated",
    "grant execute on function public.mail_mark_lapsed_from(uuid, timestamptz) to service_role",
  ], `${name}: the columns, the check and the grants, and nothing else`);
  // No new table, so no new row-level security; nothing dropped, truncated or re-created.
  assert(!/create\s+table|\bdrop\b|\btruncate\b/i.test(code), `${name}: creates, drops and truncates no table`);
});

Deno.test("count_batch_receipt_is_one_guarded_update_with_no_read_before_it", async () => {
  // Plan §1.3: an SNS redelivery (a failure after the count, or two deliveries at once) read `received`
  // and wrote `received + n` again, closing the batch early. One guarded update counts a receipt once:
  // a concurrent second delivery waits on the row lock, re-checks the `where` and finds the id there.
  const f = await followUpsFunction("count_batch_receipt");
  assert(
    /count_batch_receipt\s*\(\s*p_batch uuid, p_address uuid, p_receipt text, p_n integer\s*\)/.test(f.header),
    `${f.name}: count_batch_receipt(p_batch uuid, p_address uuid, p_receipt text, p_n integer)`,
  );
  assert(f.header.includes("returns boolean"), `${f.name}: answers counted or not`);
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 1, `${f.name}: one write`);
  assertEquals((f.body.match(/\bselect\b/g) ?? []).length, 0, `${f.name}: no read before the write`);
  assert(
    f.body.includes(
      "update public.mail_batches set received = received + p_n, counted_receipts = counted_receipts || p_receipt " +
        "where id = p_batch and address_id = p_address and p_n >= 0 and p_receipt ~ '^[0-9a-f]{64}$' " +
        "and not (p_receipt = any (counted_receipts)) returning true into counted;",
    ),
    `${f.name}: one batch of one mailbox, a SHA-256 hex id, and never a receipt already counted`,
  );
  assert(f.body.includes("return coalesce(counted, false);"), `${f.name}: no row written answers false`);
  assert(!/mail_addresses|mail_work|mail_seen|closed_at/.test(f.body), `${f.name}: writes the one batch row only`);
});

Deno.test("mail_mark_lapsed_from_never_touches_off_or_revoked_and_only_moves_the_stamp_earlier", async () => {
  // Quinn's Q3 (2026-10-20): a held item marks its mailbox lapsed from the item's arrival, so the device's
  // reconcile reaches back over it. `least` skips a null, so an unstamped row takes `p_since` and an
  // earlier stamp is kept; a row already `lapsed` is rewritten only when the stamp moves earlier, and then
  // `last_lapsed_at` moves too (voiding a `covered` in flight, spec §4.11 step 3). `off` and `revoked` are
  // left as intake's own mark leaves them. Every write sets `last_lapsed_at`, so
  // `mail_addresses_last_lapse_stamped` holds.
  const f = await followUpsFunction("mail_mark_lapsed_from");
  assert(
    /mail_mark_lapsed_from\s*\(\s*p_address uuid, p_since timestamptz\s*\)/.test(f.header),
    `${f.name}: mail_mark_lapsed_from(p_address uuid, p_since timestamptz)`,
  );
  assert(f.header.includes("returns boolean"), `${f.name}: answers written or not`);
  assertEquals((f.body.match(/\b(insert|update|delete)\b/g) ?? []).length, 1, `${f.name}: one write`);
  assertEquals((f.body.match(/\bselect\b/g) ?? []).length, 0, `${f.name}: no read before the write`);
  assert(
    f.body.includes(
      "update public.mail_addresses " +
        "set status = 'lapsed', lapsed_since = least(lapsed_since, p_since), last_lapsed_at = now() " +
        "where id = p_address and status not in ('revoked', 'off') and p_since <= now() " +
        "and (status <> 'lapsed' or lapsed_since is null or lapsed_since > p_since) returning true into marked;",
    ),
    `${f.name}: one mailbox, never off or revoked, never a future stamp, and only ever earlier`,
  );
  assert(f.body.includes("return coalesce(marked, false);"), `${f.name}: no row written answers false`);
  assert(!/greatest|account_id|mail_work|mail_seen/.test(f.body), `${f.name}: one mailbox's stamp, never later`);
});

Deno.test("both_functions_are_invoker_with_public_extensions_revoked_from_clients_and_granted_to_service_role", async () => {
  // Review r1, I1: the pattern every function in the mail migration uses (`charge_allowance`,
  // `count_mail_arrival`). The only caller is service_role, so a definer function would bypass RLS for
  // no gain.
  const { name, code } = await followUpsMigration();
  assert(!/security\s+definer/i.test(code), `${name}: no function here is security definer`);
  for (const [fn, args] of [["count_batch_receipt", "uuid, uuid, text, integer"], ["mail_mark_lapsed_from", "uuid, timestamptz"]]) {
    const f = await followUpsFunction(fn);
    assert(f.header.includes("language plpgsql"), `${f.name}: ${fn} is plpgsql`);
    assert(f.header.includes("security invoker"), `${f.name}: ${fn} is security invoker`);
    assert(!f.header.includes("security definer") && !f.body.includes("security definer"), `${f.name}: ${fn}`);
    assert(f.header.includes("set search_path = public, extensions"), `${f.name}: ${fn} pins its search_path`);
    assertServiceRoleOnly(f, fn, args);
  }
});

Deno.test("the_follow_ups_migration_never_names_the_table_the_eval_gate_watches", async () => {
  // ci.yml runs the paid eval gate for any changed migration that `grep -qiE`s the pin table's name, so
  // the raw text, comments included, never carries it in any case (plan §3).
  const { name, raw } = await followUpsMigration();
  assert(!/\bmodels\b/i.test(raw), `${name}: names the pin table, which would run the paid eval gate`);
});

// L8's S6 follow-up (2026-10-20, minors 2 and 3; friends-testers §3.3, §5, §8.3). One forward-only file,
// found by its suffix (M4): comp_grant refuses an ends_at not after now() or past now() + 1 year,
// comp_extend gains the same ceiling, and admin.testers gains entitlement_status and has_subscription
// at its end. Every other byte of the two functions and the view is the testers file's, so the pin
// rebuilds each statement from the testers file and compares the whole.
const COMP_GUARDS = /^\d{14}_comp_guards\.sql$/;

/** The one `<stamp>_comp_guards.sql`: its name and raw text. */
async function compGuardsMigration(): Promise<{ name: string; raw: string }> {
  const files = (await everyMigrationFile()).filter(([n]) => COMP_GUARDS.test(n));
  assertEquals(files.length, 1, "exactly one <stamp>_comp_guards.sql (M4: found by suffix)");
  return { name: files[0][0], raw: files[0][1] };
}

const GUARD_GRANT_SQL = "if p_ends_at is null or p_ends_at <= now() then raise exception 'the ends_at is not in the future'; " +
  "end if; if p_ends_at > now() + interval '1 year' then raise exception 'the ends_at is more than a year away'; end if;";
const GUARD_EXTEND_PAST = "if p_ends_at is null or p_ends_at <= now() then raise exception 'the new ends_at is not in the future'; end if;";
const GUARD_EXTEND_SQL = "if p_ends_at > now() + interval '1 year' then raise exception 'the new ends_at is more than a year away'; end if;";
const GUARD_COLUMNS = "coalesce(e.status, 'none') as entitlement_status, e.stripe_subscription_id is not null as has_subscription";

/** The flattened `create [or replace] function admin.<fn>(…) … ;` statement, or "". */
const adminFunctionStatement = (flat: string, fn: string) =>
  new RegExp(`create (?:or replace )?function admin\\.${fn}\\([^)]*\\) .*? as \\$\\$.*?\\$\\$[^;]*;`).exec(flat)?.[0] ?? "";
const testersViewStatement = (flat: string) => /create (?:or replace )?view admin\.testers as [^;]*;/.exec(flat)?.[0] ?? "";

/** What the guards file must be, rebuilt from the testers file: each statement, then the whole list. */
export function compGuardsProblems(testersSql: string, guardsRaw: string): string[] {
  const was = flatten(testersSql);
  const now = flatten(guardsRaw);
  const out: string[] = [];
  const replaced = (s: string) => s.replace(/^create (function|view) /, "create or replace $1 ");
  const want: Record<string, string> = {
    comp_grant: replaced(adminFunctionStatement(was, "comp_grant")).replace(" begin begin ", ` begin ${GUARD_GRANT_SQL} begin `),
    comp_extend: replaced(adminFunctionStatement(was, "comp_extend")).replace(GUARD_EXTEND_PAST, `${GUARD_EXTEND_PAST} ${GUARD_EXTEND_SQL}`),
    testers: replaced(testersViewStatement(was)).replace(" as last_active_day from ", ` as last_active_day, ${GUARD_COLUMNS} from `),
  };
  for (const [what, stmt] of Object.entries(want)) {
    if (!stmt.includes("or replace") || !/now\(\) \+ interval '1 year'|has_subscription/.test(stmt)) out.push(`${what}: not rebuilt`);
  }
  if (adminFunctionStatement(now, "comp_grant") !== want.comp_grant) out.push("comp_grant is not the testers file's plus the date guards");
  if (adminFunctionStatement(now, "comp_extend") !== want.comp_extend) out.push("comp_extend is not the testers file's plus the ceiling");
  if (testersViewStatement(now) !== want.testers) out.push("admin.testers is not the testers file's plus two columns at its end");
  const revokes = [
    "revoke execute on function admin.comp_grant(uuid, timestamptz, text, text) from public, anon, authenticated, service_role",
    "revoke execute on function admin.comp_extend(uuid, timestamptz) from public, anon, authenticated, service_role",
    "revoke all on admin.testers from public, anon, authenticated, service_role",
  ];
  for (const r of revokes) if (!was.includes(`${r};`)) out.push(`the testers file lacks ${r}`);
  const blank = (s: string) => s.replace(/\$\$.*?\$\$/g, "$$$$").replace(/;$/, "");
  const statements = topLevel(guardsRaw);
  const wantStatements = [blank(want.comp_grant), blank(want.comp_extend), revokes[0], revokes[1], blank(want.testers), revokes[2]];
  if (JSON.stringify(statements) !== JSON.stringify(wantStatements)) out.push(`statements: ${statements.join(" | ")}`);
  return out;
}

Deno.test("S6 follow-up: the guards file is found by suffix and stamped after every other file", async () => {
  const { name } = await compGuardsMigration();
  const { name: testers } = await testersMigration();
  const names = (await everyMigrationFile()).map(([n]) => n);
  assertEquals(names.at(-1), name, `${name}: stamped after every other file; a later migration is named here when it lands`);
  assert(name > testers, `${name}: after ${testers}, which stays as applied`);
});

Deno.test("S6 follow-up: comp_grant and comp_extend gain the date guards, admin.testers two end columns, and nothing else moves", async () => {
  const { name, raw } = await compGuardsMigration();
  assertEquals(compGuardsProblems((await testersMigration()).sql, raw), [], name);
  for (const fn of ["comp_grant", "comp_extend"]) assertEquals((await lastFunction(fn)).name, name, `${fn}: the guards file governs`);
  for (const fn of ["comp_end", "comp_sweep"]) assertEquals((await lastFunction(fn)).name, (await testersMigration()).name, fn);
  assert(!/\bmodels\b/i.test(raw), `${name}: names the pin table, which would run the paid eval gate`);
  assert(!/security\s+definer/i.test(stripLineComments(raw)), `${name}: no definer`);
});

Deno.test("S6 follow-up: the guard pins refuse a moved ceiling, a lost null check, a late guard, a middle column, a lost revoke", async () => {
  const { raw } = await compGuardsMigration();
  const testers = (await testersMigration()).sql;
  assertEquals(compGuardsProblems(testers, raw), [], "each mutation below is the only change");
  for (
    const [from, to] of [
      ["interval '1 year' then\n    raise exception 'the ends_at", "interval '2 years' then\n    raise exception 'the ends_at"],
      ["interval '1 year' then\n    raise exception 'the new", "interval '1 year 1 day' then\n    raise exception 'the new"],
      ["p_ends_at > now() + interval '1 year' then\n    raise exception 'the new", "p_ends_at >= now() + interval '1 year' then\n    raise exception 'the new"],
      ["if p_ends_at is null or p_ends_at <= now() then\n    raise exception 'the ends_at", "if p_ends_at <= now() then\n    raise exception 'the ends_at"],
      ["'the ends_at is more than a year away'", "'too far'"],
      ["       g.granted_at,", "       e.stripe_subscription_id is not null as has_subscription,\n       g.granted_at,"],
      ["is not null as has_subscription", "is not null as subscribed"],
      ["revoke all on admin.testers from public, anon, authenticated, service_role;", "revoke all on admin.testers from public, anon, authenticated;"],
      ["language plpgsql\nset search_path = ''", "language plpgsql\nsecurity definer\nset search_path = ''"],
    ]
  ) {
    const bad = raw.replace(from, to);
    assertNotEquals(bad, raw, `the mutation ${from} must apply to the real file`);
    assert(compGuardsProblems(testers, bad).length > 0, `${from} -> ${to} must be refused`);
  }
});

Deno.test("S6 follow-up: the G rows' refusals are the SQL's own words, and their columns the view's", async () => {
  const { GUARD_CASES, GRANT_NOT_FUTURE, GRANT_TOO_FAR, EXTEND_NOT_FUTURE, EXTEND_TOO_FAR } = await import("./testers_cases.ts");
  const flat = flatten((await compGuardsMigration()).raw);
  for (const m of [GRANT_NOT_FUTURE, GRANT_TOO_FAR, EXTEND_NOT_FUTURE, EXTEND_TOO_FAR]) {
    assert(flat.includes(`raise exception '${m}';`), `the migration raises '${m}'`);
  }
  const refusals = new Set(GUARD_CASES.flatMap((c) => (c.calls ?? []).map((k) => k.refused).filter((r) => r !== undefined)));
  assertEquals([...refusals].sort(), [GRANT_NOT_FUTURE, GRANT_TOO_FAR, EXTEND_TOO_FAR].sort(), "each new refusal is a G row's");
  for (const c of GUARD_CASES) {
    for (const a of c.accounts) assert(a.email.endsWith("@example.invalid"), c.id);
    assertEquals(c.expect.length, c.accounts.length, `${c.id}: one answer per account`);
  }
  assert(flat.includes(GUARD_COLUMNS), "entitlement_status and has_subscription, the names the G rows' runner reads");
});

// The S6 follow-up's layer iii: proofs/comp_guards_staging.sql, in the testers proof's form (one
// transaction that only rolls back, the trigger precheck first, sign-ups by S1's columns with no
// password, GUARD_CASES verbatim), plus the year's exact edge, which whole days cannot state.
const GUARD_PROOF = new URL("../proofs/comp_guards_staging.sql", HERE);

export function guardProofProblems(raw: string): string[] {
  const code = flatten(raw);
  const out: string[] = [];
  for (
    const pin of [
      "admin.comp_grant(v_id, now() + interval '1 year', 'friends', null)",
      "admin.comp_grant(v_id, now() + interval '1 year 1 second', 'friends', null)",
      "admin.comp_extend(v_id, now() + interval '1 year 1 second')",
      "admin.comp_grant(v_id, null, 'friends', null)",
      "information_schema.columns",
      "has_function_privilege",
      "has_table_privilege",
    ]
  ) if (!code.includes(pin)) out.push(`no ${pin}`);
  if (/\bmodels\b/i.test(raw)) out.push("names the pin table");
  if (/\b(?:insert\s+into|update|delete\s+from)\s+(?:public\.)?comp_(?:grants|audit)\b/.test(code)) out.push("writes a grant table directly");
  return out;
}

Deno.test("S6 follow-up: the guards proof rolls back, prechecks, signs up with no password and runs GUARD_CASES verbatim", async () => {
  const { GUARD_CASES } = await import("./testers_cases.ts");
  const raw = await Deno.readTextFile(GUARD_PROOF);
  assertEquals(proofProblems(raw, GUARD_CASES), []);
  assertEquals(guardProofProblems(raw), []);
  for (
    const [bad, want] of [
      [raw.replace(/rollback;\s*$/, "commit;\n"), "close with rollback"],
      [raw.replace('"endsIn":367', '"endsIn":366'), "CASES"],
      [raw.replace("interval '1 year 1 second', 'friends'", "interval '2 years', 'friends'"), "1 year 1 second"],
    ]
  ) {
    assertNotEquals(bad, raw, `the mutation for ${want} must apply to the real proof`);
    const problems = [...proofProblems(bad, GUARD_CASES), ...guardProofProblems(bad)];
    assert(problems.some((p) => p.includes(want)), `${want}: ${problems.join(" | ")}`);
  }
});

Deno.test("S6 follow-up: the testers proof makes a past-dated grant at a future date and backdates it, as the guards require", async () => {
  const code = flatten(await Deno.readTextFile(PROOF));
  // flatten lower-cases, so the case table's endsIn key reads endsin here.
  assert(
    code.includes("perform admin.comp_grant(v_id, greatest(pg_temp.days(x -> 'endsin'), now() + interval '1 day'), 'friends', null);"),
    "the grant at a future date",
  );
  assert(code.includes("update public.comp_grants set ends_at = pg_temp.days(x -> 'endsin') where account_id = v_id and ended_at is null;"), "then backdated");
});
