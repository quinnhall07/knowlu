import { assert, assertEquals, assertRejects } from "@std/assert";
import { rpcReply, insertRequest } from "./judge_db.ts";

const SHARED = new URL(".", import.meta.url);
/** One level up from `_shared/`: the directory holding every function, `_shared/` included. */
const FUNCTIONS = new URL("..", SHARED);
/** Two levels up and across: the migrations, read to learn which SQL functions take an account. */
const MIGRATIONS = new URL("../migrations/", FUNCTIONS);
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

/**
 * Every SQL function in ANY migration whose parameter list names `p_account` — derived, not
 * listed, so a new account-keyed function is covered the day it is written (C2 final review S-3).
 * A `.rpc(` on one of these without `p_account` in the same call is a cross-account write or read
 * with no database backstop, exactly as an unfiltered `.select(` is.
 */
export async function accountKeyedFunctions(dir: URL = MIGRATIONS): Promise<Set<string>> {
  const out = new Set<string>();
  for await (const entry of Deno.readDir(dir)) {
    if (!entry.isFile || !entry.name.endsWith(".sql")) continue;
    const sql = await Deno.readTextFile(new URL(entry.name, dir));
    for (
      const m of sql.matchAll(
        /create\s+(?:or\s+replace\s+)?function\s+(?:"?public"?\s*\.\s*)?"?(\w+)"?\s*\(([^)]*)\)/gi,
      )
    ) {
      if (/\bp_account\b/i.test(m[2])) out.add(m[1]);
    }
  }
  assert(out.size > 0, "no account-keyed SQL function found — the scan cannot be right");
  return out;
}

/**
 * The one scan, applied to one file's SOURCE TEXT. Three things, all of them the same thing:
 *
 *  1. every `.select(` on an account-scoped table names an account;
 *  2. every `.update(` on one does too (C2 final review S-3 — the old scan read selects only, so a
 *     PATCH that could have rewritten every account's rows was invisible to it);
 *  3. every `.rpc(` on a function whose SQL signature takes `p_account` passes one.
 *
 * Both quote styles are read (`` `…` `` and `"…"`), because which one a call site uses is a matter
 * of whether it happens to interpolate. A `.rpc(` the parser cannot read at all fails loud rather
 * than passing silently — the same discipline `migrations_test.ts`'s parse-count check applies.
 */
export function scanSource(rawSource: string, label: string, accountKeyed: Set<string>): void {
  const source = withoutCommentLines(rawSource);
  for (const m of source.matchAll(/\.(select|update)\(\s*(?:`([^`]+)`|"([^"]+)")/g)) {
    const method = m[1];
    const path = (m[2] ?? m[3]).trim();
    const table = path.split("?")[0].trim();
    if (!SCOPED.includes(table)) continue;
    assert(
      path.includes("account_id=eq.") || path.includes("account_id.eq."),
      `${label}: a ${method} on '${table}' with no account_id filter. The service role bypasses ` +
        `RLS, so this filter is the whole access control (Task 2 step 7).`,
    );
  }

  const rpcCalls = [...source.matchAll(/\.rpc\(\s*["'`](\w+)["'`]\s*,\s*(\{[\s\S]*?\})\s*\)/g)];
  const rawRpc = (source.match(/\.rpc\(/g) ?? []).length;
  assert(
    rpcCalls.length === rawRpc,
    `${label}: found ${rawRpc} '.rpc(' call(s) but could read the name and arguments of ` +
      `${rpcCalls.length} — a shape this scan cannot see is a scoping gap it cannot see either.`,
  );
  for (const call of rpcCalls) {
    const [, fn, args] = call;
    if (!accountKeyed.has(fn)) continue;
    assert(
      /\bp_account\b/.test(args),
      `${label}: '${fn}' takes p_account in SQL and this call does not pass one. The service ` +
        `role bypasses RLS inside the function too, so the argument is the whole access control.`,
    );
  }
}

// Drops whole-line comments — a line whose first non-space characters open or continue one.
// Prose in this codebase talks ABOUT `.select(` and `.rpc(` (the header of `judge_db.ts` does
// exactly that, describing this scan), and a comment must neither satisfy the scan nor trip its
// fail-loud parse count. Line-based and never inside a line, so a URL's slashes inside a string
// literal are untouched; no line of real code in this corpus starts with a comment token.
function withoutCommentLines(source: string): string {
  return source
    .split("\n")
    .map((line) => (/^\s*(\/\/|\/\*|\*)/.test(line) ? "" : line))
    .join("\n");
}

async function scanFile(url: URL, label: string, accountKeyed: Set<string>): Promise<void> {
  scanSource(await Deno.readTextFile(url), label, accountKeyed);
}

Deno.test("every read, write and account-keyed RPC in every function file names an account", async () => {
  const accountKeyed = await accountKeyedFunctions();
  for await (const entry of Deno.readDir(SHARED)) {
    if (!entry.name.endsWith(".ts") || entry.name.endsWith("_test.ts")) continue;
    await scanFile(new URL(entry.name, SHARED), `_shared/${entry.name}`, accountKeyed);
  }
  // R-C2-E20 fix 1 retired the `_shared`-only scan: a function's own `index.ts` or `handler.ts`
  // calls `.select(` too. C2 final review S-3 retires the two-fixed-names version as well — a
  // third file in a function's directory (`google-connect/disconnect.ts`,
  // `google-callback/exchange.ts`, `ingest-coursework/parse_*.ts`) is deployed in exactly the same
  // bundle and was simply never looked at. Every non-test `.ts` in every function directory now is.
  let walked = 0;
  for await (const dirEntry of Deno.readDir(FUNCTIONS)) {
    if (!dirEntry.isDirectory || dirEntry.name === "_shared") continue;
    for await (const file of Deno.readDir(new URL(`${dirEntry.name}/`, FUNCTIONS))) {
      if (!file.isFile || !file.name.endsWith(".ts") || file.name.endsWith("_test.ts")) continue;
      walked += 1;
      await scanFile(
        new URL(`${dirEntry.name}/${file.name}`, FUNCTIONS),
        `${dirEntry.name}/${file.name}`,
        accountKeyed,
      );
    }
  }
  // A floor, not a pin: the point is that the walk reached more than the two files it used to, and
  // that a directory with a helper beside its handler is not silently half-scanned.
  assert(walked >= 30, `the walk should reach every function file, reached ${walked}`);
});

Deno.test("the widened scan catches an unscoped update, an unscoped RPC, and an unreadable rpc call", async () => {
  const accountKeyed = await accountKeyedFunctions();

  // (1) A PATCH with no account filter — invisible to the select-only scan this replaces, and a
  // rewrite of every account's rows if it ever shipped.
  assert(
    Error.isError(
      catchSync(() =>
        scanSource(
          'await sharedDb().update(`gmail_queue?delivered_at=is.null`, { delivered_at: "x" });',
          "synthetic/update.ts",
          accountKeyed,
        )
      ),
    ),
  );
  const unscopedUpdate = catchSync(() =>
    scanSource(
      'await db.update("google_accounts?status=neq.revoked", { status: "revoked" });',
      "synthetic/update-double-quoted.ts",
      accountKeyed,
    )
  );
  assert(String(unscopedUpdate).includes("google_accounts"), String(unscopedUpdate));

  // (2) An account-keyed RPC called without its account.
  const unscopedRpc = catchSync(() =>
    scanSource('await db.rpc("delete_google_grant", { p_scope: "x" });', "synthetic/rpc.ts", accountKeyed)
  );
  assert(String(unscopedRpc).includes("delete_google_grant"), String(unscopedRpc));
  // A function that takes no account is not the scan's business.
  scanSource('await db.rpc("take_google_state", { p_nonce: n });', "synthetic/rpc-ok.ts", accountKeyed);

  // (3) A `.rpc(` shape the scan cannot read fails loud rather than passing unseen.
  const unreadable = catchSync(() => scanSource("await db.rpc(name, args);", "synthetic/rpc-dynamic.ts", accountKeyed));
  assert(String(unreadable).includes("could read the name and arguments of 0"), String(unreadable));

  // And the scan really does reach a helper file that is neither index.ts nor handler.ts.
  await assertRejects(
    () => scanFile(new URL("does-not-exist.ts", SHARED), "synthetic/missing", accountKeyed),
    Deno.errors.NotFound,
  );
});

/** `assertThrows` returns the error but insists the call throws; these cases want the error VALUE
 * for a substring check, and one of them must not throw at all. */
function catchSync(fn: () => void): unknown {
  try {
    fn();
    return null;
  } catch (e) {
    return e;
  }
}

// C2 final review F-2: `gmail-read`'s `markSeen` calls `insert("gmail_seen", row, false)` every
// round, including for a uid it has already marked — `gmail_seen`'s own read used to truncate
// silently past PostgREST's row cap, so the same uid could come back around and hit the table's
// `(account_id, uid)` primary key a second time. A bare POST 409s on that; `onConflict` makes it
// `resolution=ignore-duplicates` instead, a no-op.
//
// `insertRequest` is the pure path/header logic `serviceDb()`'s real `insert` builds from, pulled
// out on purpose: the CI `deno test` invocation's `--allow-env` list is fixed to four Anthropic
// variables and carries no `--allow-net`, so a test that reached `serviceDb()` itself (real env
// vars, a stubbed `fetch`) would need permissions this suite is not run with. Testing the pure
// function proves the same shape with no permission at all.
Deno.test("insert's onConflict carries on_conflict in the path and ignore-duplicates in Prefer", () => {
  assertEquals(
    insertRequest("gmail_seen", false, "account_id,uid"),
    { path: "gmail_seen?on_conflict=account_id,uid", prefer: "return=minimal,resolution=ignore-duplicates" },
  );
});

Deno.test("insert with no onConflict is unchanged from before F-2: no query string, no resolution clause", () => {
  assertEquals(insertRequest("gmail_seen", false), { path: "gmail_seen", prefer: "return=minimal" });
  assertEquals(insertRequest("judgments", true), { path: "judgments", prefer: "return=representation" });
});

Deno.test("an RPC reply with no body — a function that returns void — is null, not a parse error", async () => {
  // Found by the P3 live pass on staging, 2026-09-17: `delete_google_grant` returns void, PostgREST
  // answers with an EMPTY body, and `.json()` on it threw — after the delete had committed — so
  // `google-connect`'s DELETE answered 502 "Google could not be reached to disconnect; try again"
  // for a grant that was already revoked at Google and gone from the row, and every retry would
  // have said the same. Empty means null; a real body is parsed exactly as before.
  assertEquals(await rpcReply(new Response(null, { status: 204 })), null);
  assertEquals(await rpcReply(new Response("", { status: 200 })), null);
  assertEquals(await rpcReply(new Response("true", { status: 200 })), true);
  assertEquals(await rpcReply(new Response('"7df2ba9c"', { status: 200 })), "7df2ba9c");
  assertEquals(await rpcReply(new Response('{"a":1}', { status: 200 })), { a: 1 });
});
