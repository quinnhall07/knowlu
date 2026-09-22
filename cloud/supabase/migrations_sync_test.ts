import { assert, assertEquals } from "@std/assert";

const DIR = new URL("./migrations/", import.meta.url);

/** C3′'s own migrations, and only C3′'s. C1 filters `20260910…` and C2 filters `20260911…` from
 * their own sides (R-X-8); a helper that read the whole directory would turn someone else's suite
 * red for a reason that has nothing to do with their change. */
const MINE = /^20260912\d{6}_[a-z0-9_]+\.sql$/;

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

/** Every table C3′ leaves BEHIND: created by one of its migrations and not dropped by a later one.
 * Derived, never hand-typed, so a table added or dropped later cannot be forgotten in two places.
 *
 * Ops within one file are applied in the order they APPEAR in the text, not create-then-drop by
 * scan order: 20260912000300 drops `sync_records`/`sync_notes` early in the file (to widen the
 * primary key) and recreates both later in the same file, and a migration runs top to bottom, so
 * the table that survives is whichever statement is last on the page. Two separate `matchAll`
 * passes with no ordering would net that supersession to "gone" — a table that is very much still
 * there. */
async function liveTables(): Promise<string[]> {
  const live = new Set<string>();
  for (const m of await migrations()) {
    const ops: { at: number; kind: "create" | "drop"; name: string }[] = [];
    for (const c of m.sql.matchAll(/create\s+table\s+(?:if\s+not\s+exists\s+)?public\.(\w+)/gi)) {
      ops.push({ at: c.index, kind: "create", name: c[1] });
    }
    for (const d of m.sql.matchAll(/drop\s+table\s+(?:if\s+exists\s+)?public\.(\w+)/gi)) {
      ops.push({ at: d.index, kind: "drop", name: d[1] });
    }
    ops.sort((a, b) => a.at - b.at);
    for (const op of ops) op.kind === "create" ? live.add(op.name) : live.delete(op.name);
  }
  return [...live].sort();
}

Deno.test("C3′'s migrations are stamped in C3's day, and the two already on staging are untouched", async () => {
  const mine = await migrations();
  assert(mine.length >= 3, `expected the two applied migrations and C3′'s own, found ${mine.length}`);
  for (const m of mine) assert(m.name.startsWith("20260912"), `${m.name}: C3's are stamped 20260912…`);
  // A forward-only corpus: the shape correction is a NEW file, never an edit of an applied one.
  assertEquals(mine[0].name, "20260912000100_sync.sql");
  assertEquals(mine[1].name, "20260912000200_sync_usage_prune.sql");
  assertEquals(mine[2].name, "20260912000300_sync_plaintext.sql");
});

Deno.test("the account's copy is three tables: the records, the notes and the byte counter", async () => {
  // `sync_generation` held the fingerprint of the device key. Ruling 2 removed the key, so the
  // table is dropped rather than left as a row nobody writes and a name in a purge list.
  assertEquals(await liveTables(), ["sync_notes", "sync_records", "sync_usage"]);
});

Deno.test("row-level security is on for every table C3′ leaves, and none has a client write policy", async () => {
  const secured: string[] = [];
  for (const m of await migrations()) {
    for (const s of m.sql.matchAll(/alter\s+table\s+public\.(\w+)\s+enable\s+row\s+level\s+security/gi)) {
      secured.push(s[1]);
    }
    // Every `create policy` must spell `for` explicitly: `create policy p on public.x to
    // authenticated using (…)` with no `for` clause defaults to FOR ALL — a write policy — and
    // would slip past a scan that only matched the `for`-bearing shape.
    const everyPolicy = m.sql.match(/create\s+policy\s+\w+\s+on\s+public\.\w+/gi) ?? [];
    const forPolicies = [...m.sql.matchAll(/create\s+policy\s+\w+\s+on\s+public\.\w+\s+for\s+(\w+)/gi)];
    assertEquals(forPolicies.length, everyPolicy.length, `${m.name}: a 'create policy' with no explicit 'for' is a write policy`);
    for (const p of forPolicies) {
      assertEquals(p[1].toLowerCase(), "select", `${m.name}: only select policies; writes go through an edge function`);
    }
  }
  for (const t of await liveTables()) assert(secured.includes(t), `public.${t} has no 'enable row level security'`);
});

Deno.test("the rows hold the student's own text, bounded in BYTES, and every one is scoped to an account", async () => {
  // **The inversion.** Until the amendment of 2026-09-17 this file asserted that no column could
  // hold a note, a path or a title. Ruling 2 reversed that: the service holds the tasks and notes so
  // the student's desktops stay in step, readable by us, encrypted at rest, deleted with the
  // account. What is still true — and what this test now pins — is that every row belongs to exactly
  // one account, that the payload is bounded, and that the bound is in BYTES on both sides of the
  // wire (`octet_length`, not `length`: Postgres counts characters and the device counts bytes, and
  // a vault full of accented Spanish would otherwise disagree with its own cap).
  const sql = (await migrations()).at(-1)!.sql.toLowerCase();
  assert(sql.includes("account_id  uuid        not null references public.accounts (id) on delete cascade"), "records cascade from the account");
  assert(sql.includes("octet_length(body) between 2 and 16384"), "a record's body is bounded in bytes");
  assert(sql.includes("octet_length(body) between 1 and 131072"), "a note's body is bounded in bytes");
  assert(sql.includes("record_hash ~ '^[0-9a-f]{64}$'"), "the content hash is 64 hex characters");
  assert(sql.includes("device ~ '^[0-9a-f]{16}$'"), "the device token is opaque, 16 hex characters");
});

Deno.test("a note's path is checked, not trusted", async () => {
  // The path is the note's primary key now, and it is a string a client sends. Without this a
  // pushed `../../etc/hosts` would sit in the table waiting for a restore to write it.
  const sql = (await migrations()).at(-1)!.sql;
  assert(sql.includes("(tasks|approvals|archive|courses|issues|info)/"), "only the six note folders");
  assert(sql.includes("\\.md$"), "and only markdown");
  assert(sql.includes("path !~ "), "and a path that can climb out is refused by its own check");
});

Deno.test("retention never deletes a record a human wrote, and the SERVER is what decides that", async () => {
  // P3's answer (Quinn, 2026-09-17), and the half that is a correctness property rather than a
  // storage one: `journal::human_set` is what judge-once reads, and it reads records. A `sync_prune`
  // without this clause quietly costs a restored machine its attribution.
  //
  // **What the amendment changed:** the device used to assert `keep` and the server had to believe
  // it, because the row was ciphertext. The row is plaintext now, so the rule is a generated column
  // computed from the record itself — `op` in (set, create) and an actor that is not an agent, which
  // is `provenance::is_agent`'s own `starts_with("agent:")` test. A client cannot lie about it and
  // cannot forget it.
  const sql = (await migrations()).map((m) => m.sql).join("\n");
  assert(sql.includes("and not keep"), "sync_prune must exempt the records marked `keep`");
  const last = (await migrations()).at(-1)!.sql;
  assert(/keep\s+boolean\s+not null generated always as/.test(last), "`keep` is generated, not sent");
  assert(last.includes("'agent:%'"), "an agent's record is not kept");
  assert(last.includes("in ('set', 'create')"), "only a set or a create is a human decision worth keeping");
});

Deno.test("the account purge names every table C3′ leaves, and no table it dropped", async () => {
  // Hand-off H1. The foreign key cascades anyway; this list is what the privacy policy's deletion
  // paragraph is written from, and a table missing from it is a table nobody remembers to mention —
  // while a table NAMED in it that no longer exists is a 404 on every account deletion.
  const index = await Deno.readTextFile(new URL("./functions/account/index.ts", import.meta.url));
  const purge = index.slice(index.indexOf("purge:"), index.indexOf("deleteAuthUser:"));
  for (const t of await liveTables()) assert(purge.includes(`"${t}"`), `DELETE /account does not purge ${t}`);
  assert(!purge.includes("sync_generation"), "sync_generation is gone; purging it is a 404 every time");
});
