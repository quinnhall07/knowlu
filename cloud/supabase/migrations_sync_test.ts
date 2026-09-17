import { assert, assertEquals } from "@std/assert";

const DIR = new URL("./migrations/", import.meta.url);

/** C3's own migrations, and only C3's. C1 filters `20260910…` and C2 filters `20260911…` from their
 * own sides (R-X-8); a helper that read the whole directory would turn someone else's suite red for a
 * reason that has nothing to do with their change. */
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

Deno.test("C3's migrations are stamped in C3's day", async () => {
  const mine = await migrations();
  assert(mine.length > 0, "no C3 migrations found");
  for (const m of mine) assert(m.name.startsWith("20260912"), `${m.name}: C3's are stamped 20260912…`);
});

Deno.test("row-level security is on for every table C3 creates, and none has a client write policy", async () => {
  const created: string[] = [];
  const secured: string[] = [];
  for (const m of await migrations()) {
    for (const c of m.sql.matchAll(/create\s+table\s+(?:if\s+not\s+exists\s+)?public\.(\w+)/gi)) {
      created.push(c[1]);
    }
    for (const s of m.sql.matchAll(/alter\s+table\s+public\.(\w+)\s+enable\s+row\s+level\s+security/gi)) {
      secured.push(s[1]);
    }
    for (const p of m.sql.matchAll(/create\s+policy\s+\w+\s+on\s+public\.\w+\s+for\s+(\w+)/gi)) {
      assertEquals(p[1].toLowerCase(), "select", `${m.name}: only select policies; writes go through an edge function`);
    }
  }
  assert(created.length >= 4, `expected the four sync tables, found ${created.join(", ")}`);
  for (const t of created) assert(secured.includes(t), `public.${t} has no 'enable row level security'`);
});

/** Every column line inside a `create table public.…( … );` block, with `--` comments stripped.
 *
 * **Not a word scan over the raw SQL**: the comments in this migration say "path" four times
 * explaining that the path never leaves the device, and `set search_path = public` on every function
 * says it again. What is under test is the columns, so the columns are what is read. */
function columnLines(sql: string): string[] {
  const out: string[] = [];
  for (const block of sql.matchAll(/create\s+table\s+public\.\w+\s*\(([\s\S]*?)\n\);/gi)) {
    for (const line of block[1].split("\n")) {
      const code = line.replace(/--.*$/, "").trim();
      if (code) out.push(code);
    }
  }
  return out;
}

Deno.test("no column C3 creates could hold a note, a path or a title", async () => {
  // The whole promise of §5.5 in one test: what is not ciphertext is an opaque token, a hash, an IV,
  // a counter or a timestamp. A column called `path`, `title`, `body` or `field` would be a design
  // failure, not a typo.
  const lines = (await migrations()).flatMap((m) => columnLines(m.sql));
  assert(lines.length >= 15, `the scan found suspiciously few column lines: ${lines.length}`);
  for (const line of lines) {
    for (const word of ["path", "title", "body", "note_text", "field", "email", "summary", "subject"]) {
      assert(!line.toLowerCase().includes(word), `a column line names ${word}: ${line}`);
    }
  }
});

Deno.test("the ciphertext columns are bounded and the opaque columns are shaped", async () => {
  const sql = (await migrations()).map((m) => m.sql).join("\n").toLowerCase();
  assert(sql.includes("device ~ '^[0-9a-f]{16}$'"), "the device token is 16 hex characters");
  assert(sql.includes("record_hash ~ '^[0-9a-f]{64}$'"), "the record hash is a keyed mac, 64 hex");
  assert(sql.includes("note_ref ~ '^[0-9a-f]{64}$'"), "the note ref is a keyed mac of the path, never the path");
  assert(sql.includes("key_fingerprint ~ '^[0-9a-f]{8}$'"), "the key generation is a fingerprint, not a key");
  assert(sql.includes("length(ciphertext)"), "every ciphertext column is length-bounded");
});

Deno.test("retention never deletes a record a human wrote", async () => {
  // P3's recommended shape, and the half that is a correctness property rather than a storage one:
  // `journal::human_set` is what judge-once reads, and it reads records. A `sync_prune` without this
  // clause quietly costs a restored machine its attribution.
  const sql = (await migrations()).map((m) => m.sql).join("\n").toLowerCase();
  assert(sql.includes("and not keep"), "sync_prune must exempt the records the device marked `keep`");
  assert(sql.includes("keep        boolean     not null default false"), "and the column must exist");
});

Deno.test("the account purge names every sync table", async () => {
  // Hand-off H5. The foreign key cascades anyway; this list is what the privacy policy's deletion
  // paragraph is written from, and a table missing from it is a table nobody remembers to mention.
  // **Derived from the migration, not from a hand-typed list**: a fourth table added here later is
  // a fourth table this test demands, with no second place to remember.
  const index = await Deno.readTextFile(new URL("./functions/account/index.ts", import.meta.url));
  const purge = index.slice(index.indexOf("purge:"), index.indexOf("deleteAuthUser:"));
  const created = (await migrations()).flatMap((m) =>
    [...m.sql.matchAll(/create\s+table\s+public\.(\w+)/gi)].map((c) => c[1])
  );
  assertEquals(created.sort(), ["sync_generation", "sync_notes", "sync_records", "sync_usage"]);
  for (const t of created) {
    assert(purge.includes(`"${t}"`), `DELETE /account does not purge ${t}`);
  }
});
