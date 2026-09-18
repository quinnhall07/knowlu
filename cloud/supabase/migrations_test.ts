import { assert, assertEquals } from "@std/assert";

const DIR = new URL("./migrations/", import.meta.url);

/** C1's own migrations, and **only** C1's (R-X-8). C2 puts its `20260911…` files in this same
 * directory and merges after C1; a helper that read the whole directory would turn this suite red on
 * C2's first commit for a reason that has nothing to do with C2's change. C2's own test filters the
 * same way, from its own side. */
const MINE = /^20260910\d{6}_[a-z0-9_]+\.sql$/;

async function migrations(): Promise<{ name: string; sql: string }[]> {
  const out: { name: string; sql: string }[] = [];
  for await (const e of Deno.readDir(DIR)) {
    if (e.isFile && e.name.endsWith(".sql") && MINE.test(e.name)) {
      out.push({ name: e.name, sql: await Deno.readTextFile(new URL(e.name, DIR)) });
    }
  }
  out.sort((a, b) => a.name.localeCompare(b.name));
  return out;
}

/** Every `.sql` in the directory, C2's included — used only by the one test that must see them all. */
async function allMigrationNames(): Promise<string[]> {
  const out: string[] = [];
  for await (const e of Deno.readDir(DIR)) {
    if (e.isFile && e.name.endsWith(".sql")) out.push(e.name);
  }
  return out.sort();
}

Deno.test("every migration is stamped and lower case, and C1's are in C1's day", async () => {
  // The SHAPE is asserted over every file in the directory, C2's included — a migration nobody can
  // order is a problem whoever wrote it.
  for (const name of await allMigrationNames()) {
    assert(/^\d{14}_[a-z0-9_]+\.sql$/.test(name), `${name}: not <YYYYMMDDHHMMSS>_<name>.sql`);
  }
  // The DAY is asserted only over C1's own (R-X-8): C1 owns 2026-09-10, C2 owns 2026-09-11, and
  // C2 merges after C1 into this same directory.
  const mine = await migrations();
  assert(mine.length > 0, "no C1 migrations found");
  for (const m of mine) {
    assert(m.name.startsWith("20260910"), `${m.name}: C1's migrations are stamped 20260910…`);
  }
});

Deno.test("no migration ever creates a birthdate column", async () => {
  // Spec §9, minors: the 18+ gate is an attestation boolean with a timestamp. A birthdate is
  // personal data we would then have to protect, and it buys nothing.
  for (const m of await migrations()) {
    for (const word of ["birthdate", "birth_date", "date_of_birth", " dob ", "birthday"]) {
      assert(!m.sql.toLowerCase().includes(word), `${m.name} names ${word.trim()}`);
    }
  }
});

Deno.test("row-level security is enabled on every table these migrations create", async () => {
  const created: string[] = [];
  const secured: string[] = [];
  for (const m of await migrations()) {
    for (const c of m.sql.matchAll(/create\s+table\s+(?:if\s+not\s+exists\s+)?public\.(\w+)/gi)) {
      created.push(c[1].toLowerCase());
    }
    for (const s of m.sql.matchAll(/alter\s+table\s+public\.(\w+)\s+enable\s+row\s+level\s+security/gi)) {
      secured.push(s[1].toLowerCase());
    }
  }
  assert(created.length > 0, "no tables created");
  for (const t of created) assert(secured.includes(t), `public.${t} has no RLS`);
});

Deno.test("no table has a client write policy — every write is an edge function's", async () => {
  // This is what makes "the Stripe webhook is the only writer of entitlements" an enforced
  // property rather than a convention: the service role bypasses RLS and nothing else may write.
  for (const m of await migrations()) {
    for (
      const p of m.sql.matchAll(/create\s+policy\s+(\w+)[\s\S]*?for\s+(select|insert|update|delete|all)/gi)
    ) {
      assertEquals(p[2].toLowerCase(), "select", `policy ${p[1]} grants ${p[2]} to a client`);
    }
  }
});

Deno.test("entitlements is readable by its owner and by nothing else", async () => {
  const all = await migrations();
  const sql = all.map((m) => m.sql).join("\n").toLowerCase();
  assert(sql.includes("create policy entitlements_select_own"), "no owner-read policy on entitlements");
  const policies = [...sql.matchAll(/create\s+policy\s+(\w+)\s+on\s+public\.entitlements/g)].map((m) => m[1]);
  assertEquals(policies, ["entitlements_select_own"], `entitlements has extra policies: ${policies}`);
});

Deno.test("no reporting view may be read below the minimum cohort", async () => {
  const sql = (await migrations()).map((m) => m.sql).join("\n").toLowerCase();
  // Fix round 1, item 6: `create or replace view` and `create materialized view` are both ways to
  // define a readable view, and neither may skip the floor check by skipping this regex.
  const VIEW_DEF = /create\s+(?:or\s+replace\s+)?(?:materialized\s+)?view\s+public\.(\w+)/g;
  const views = [...sql.matchAll(VIEW_DEF)].map((m) => m[1]);
  for (const v of views) {
    if (v === "billing_subscribers") continue; // an operational read of one row per subscriber, not a slice
    const start = sql.search(
      new RegExp(`create\\s+(?:or\\s+replace\\s+)?(?:materialized\\s+)?view\\s+public\\.${v}\\b`),
    );
    const body = sql.slice(start).split(";")[0];
    assert(
      body.includes("count(distinct account_id) >= 10"),
      `view ${v} has no minimum cohort — product plan §7, spec §6`,
    );
  }
});

Deno.test("a view is defined after the columns it reads (R-C1-34)", async () => {
  // The defect the staging batch found: `billing_subscribers` read three `entitlements` columns that
  // the same file added further down. Reviewed, never applied — so the order was never exercised.
  for (const m of await migrations()) {
    const sql = m.sql.toLowerCase();
    const views = [...sql.matchAll(/create\s+(?:or\s+replace\s+)?view\s+public\.(\w+)[\s\S]*?;/g)];
    for (const add of sql.matchAll(/alter\s+table\s+public\.(\w+)\s+add\s+column\s+(\w+)/g)) {
      const [, table, col] = add;
      for (const v of views) {
        if (v.index! < add.index! && v[0].includes(`.${col}`)) {
          assert(false, `${m.name}: view public.${v[1]} reads ${table}.${col} before it is added`);
        }
      }
    }
  }
});

Deno.test("the OAuth migration adds no table, no policy, no birthdate column — and no raise", async () => {
  const sql = await Deno.readTextFile(
    new URL("./migrations/20260917000100_oauth_consent.sql", import.meta.url),
  );
  // **The `--` lines come off first.** This migration's comment block explains at length what the
  // function no longer reads, so an assertion over the raw text would be an assertion about the
  // prose. `migrations/migrations_test.ts` strips comments before scanning for the same reason.
  const code = sql.split("\n").filter((l) => !l.trim().startsWith("--")).join("\n");
  // C1's three rules are pinned over the whole directory elsewhere; this one is about THIS file:
  // it replaces one function and nothing else, so a reviewer never has to diff schema to be sure.
  assert(!/create\s+table/i.test(code), "this migration creates no table");
  assert(!/create\s+policy/i.test(code), "…and no policy: RLS is C1's and stays as it is");
  assert(!/\bbirth|\bdob\b|date_of_birth/i.test(code), "no birthdate column, in this file or any other");
  assert(
    code.includes("create or replace function public.handle_new_user()"),
    "the trigger's function is replaced",
  );
  // R-C1b-3. The function reads NOTHING out of the sign-up's metadata and raises nothing: `/otp`
  // with `create_user: true` is reachable by anyone holding the public anon key, so a trigger that
  // believed that request's `data` would stamp an `age_18` consent row for an address whose owner
  // never attested to anything. The consent row is `POST /account/consent`'s to write, behind a
  // session, and the 18+ tooth is `billing-checkout`'s 403.
  assert(!/raise\s+exception/i.test(code), "no raise survives in the replaced function");
  // Asserted over the SOURCE of the values, never their names: `age_attested_at` and `tos_version`
  // are columns this migration still writes (as nulls), so banning those words would ban the insert.
  assert(!code.includes("raw_user_meta_data"), "the trigger reads none of the sign-up's own metadata");
  // A word boundary, not `.includes("public.consents")`: this function's own `search_path` is
  // `public, extensions`, so an unqualified `insert into consents` would resolve to the same table
  // and pass a check that only banned the schema-qualified spelling (nit 8).
  assert(
    !/\bconsents\b/i.test(code),
    "…and writes no consent row: that is the route's, behind a session",
  );
});
