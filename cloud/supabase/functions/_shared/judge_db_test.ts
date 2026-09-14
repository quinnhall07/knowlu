import { assert } from "@std/assert";

const SHARED = new URL(".", import.meta.url);
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
];

Deno.test("every read of an account-scoped table names an account", async () => {
  for await (const entry of Deno.readDir(SHARED)) {
    if (!entry.name.endsWith(".ts") || entry.name.endsWith("_test.ts")) continue;
    const source = await Deno.readTextFile(new URL(entry.name, SHARED));
    // Matches `db.select(`, `sharedDb().select(` and `client.select(` alike — the call, not the
    // receiver, because the receiver's name is the one thing a refactor changes.
    for (const m of source.matchAll(/\.select\(\s*`([^`]+)`/g)) {
      const path = m[1];
      const table = path.split("?")[0].trim();
      if (!SCOPED.includes(table)) continue;
      assert(
        path.includes("account_id=eq.") || path.includes("account_id.eq."),
        `${entry.name}: a select on '${table}' with no account_id filter. The service role bypasses ` +
          `RLS, so this filter is the whole access control (Task 2 step 7).`,
      );
    }
  }
});
