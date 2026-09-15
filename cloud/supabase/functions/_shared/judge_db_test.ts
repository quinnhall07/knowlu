import { assert } from "@std/assert";

const SHARED = new URL(".", import.meta.url);
/** One level up from `_shared/`: the directory holding every function, `_shared/` included. */
const FUNCTIONS = new URL("..", SHARED);
/** Tables whose every row belongs to one account. `models` is global and is not one. */
const SCOPED = [
  "judgments",
  "usage_daily",
  "rules",
  "budget_alerts",
  "corrections",
  "gmail_seen",
  "gmail_queue",
  "google_accounts",
  "sources",
  // `google_state` deliberately stays OUT of this list: it is keyed by its single-use nonce, and
  // the callback has no account until that row is read — there is no account_id to scope by yet.
];

/** The one scan, applied to one file: every `.select(\`…\`)` on a scoped table must name an account. */
async function scanForUnscopedSelects(url: URL, label: string): Promise<void> {
  const source = await Deno.readTextFile(url);
  // Matches `db.select(`, `sharedDb().select(` and `client.select(` alike — the call, not the
  // receiver, because the receiver's name is the one thing a refactor changes.
  for (const m of source.matchAll(/\.select\(\s*`([^`]+)`/g)) {
    const path = m[1];
    const table = path.split("?")[0].trim();
    if (!SCOPED.includes(table)) continue;
    assert(
      path.includes("account_id=eq.") || path.includes("account_id.eq."),
      `${label}: a select on '${table}' with no account_id filter. The service role bypasses ` +
        `RLS, so this filter is the whole access control (Task 2 step 7).`,
    );
  }
}

Deno.test("every read of an account-scoped table names an account", async () => {
  for await (const entry of Deno.readDir(SHARED)) {
    if (!entry.name.endsWith(".ts") || entry.name.endsWith("_test.ts")) continue;
    await scanForUnscopedSelects(new URL(entry.name, SHARED), `_shared/${entry.name}`);
  }
  // R-C2-E20 fix 1 (retires plan Task 15 step 3's widening): `_shared/` was never the whole
  // surface — a function's own `index.ts` or `handler.ts` can call `.select(` too, and
  // `ingest-ics/index.ts` and `ingest-calendar/index.ts` were the first to do it. Same scan, same
  // scoped-table list, same `account_id` requirement, now applied to every function directory
  // beside `_shared/` as well.
  for await (const dirEntry of Deno.readDir(FUNCTIONS)) {
    if (!dirEntry.isDirectory || dirEntry.name === "_shared") continue;
    for (const file of ["index.ts", "handler.ts"]) {
      const url = new URL(`${dirEntry.name}/${file}`, FUNCTIONS);
      try {
        await scanForUnscopedSelects(url, `${dirEntry.name}/${file}`);
      } catch (e) {
        if (e instanceof Deno.errors.NotFound) continue;
        throw e;
      }
    }
  }
});
