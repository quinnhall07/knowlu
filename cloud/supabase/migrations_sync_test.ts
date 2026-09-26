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

/** `20260912000300_sync_plaintext.sql` by name, not `.at(-1)` and not a whole-corpus join
 * (R-C3′-exec-10 review I1): `20260912000400_sync_note_path_check.sql` is the first migration in
 * this stream that patches a constraint without recreating the table, so it is textually last
 * while defining none of what the three tests below pin — `.at(-1)` would read the wrong file. A
 * whole-corpus join would read the RIGHT text today, but it can never go red again: migrations are
 * forward-only, so a still-true substring is never removed from an OLDER file even if a later
 * migration alters or drops the check it names — exactly the kind of silent drift this suite exists
 * to catch. 000300 is still the one file that actually defines `sync_records`/`sync_notes`'s shape
 * (000400 touches only `sync_notes_path_check`), so naming it keeps these three as tight as they
 * were before 000400 existed. */
async function plaintextShapeSql(): Promise<string> {
  const found = (await migrations()).find((m) => m.name === "20260912000300_sync_plaintext.sql");
  assert(found, "expected 20260912000300_sync_plaintext.sql to still exist");
  return found!.sql;
}

/** `20260912000100_sync.sql` by name (re-review of R-C3′-exec-10 review I1): `sync_prune`'s real
 * `delete ... and not keep;` clause lives only here, untouched by both 000300 and 000400 —
 * `plaintextShapeSql()` does not cover this file, and the same substring also happens to appear in
 * 000300's own header COMMENT (prose about what 000300 does not redefine), which is not the same
 * thing as the live SQL enforcing the property. */
async function syncPruneSql(): Promise<string> {
  const found = (await migrations()).find((m) => m.name === "20260912000100_sync.sql");
  assert(found, "expected 20260912000100_sync.sql to still exist");
  return found!.sql;
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
  const sql = (await plaintextShapeSql()).toLowerCase();
  assert(sql.includes("account_id  uuid        not null references public.accounts (id) on delete cascade"), "records cascade from the account");
  assert(sql.includes("octet_length(body) between 2 and 16384"), "a record's body is bounded in bytes");
  assert(sql.includes("octet_length(body) between 1 and 131072"), "a note's body is bounded in bytes");
  assert(sql.includes("record_hash ~ '^[0-9a-f]{64}$'"), "the content hash is 64 hex characters");
  assert(sql.includes("device ~ '^[0-9a-f]{16}$'"), "the device token is opaque, 16 hex characters");
});

Deno.test("a note's path is checked, not trusted", async () => {
  // The path is the note's primary key now, and it is a string a client sends. Without this a
  // pushed `../../etc/hosts` would sit in the table waiting for a restore to write it.
  //
  // R-C3′-exec-10 review I1: named at 000300, not the corpus — the folder/markdown text below
  // still appears in 000300's OWN (superseded) check even though 20260912000400 is what enforces it
  // on the server now, and the two climb-out siblings this test also pins were never moved. Reading
  // 000300 by name, rather than any-file-ever, is what lets this test go red again if a future
  // migration ever weakened one of ITS OWN checks without 000300 changing.
  const sql = await plaintextShapeSql();
  assert(sql.includes("(tasks|approvals|archive|courses|issues|info)/"), "only the six note folders");
  assert(sql.includes("\\.md$"), "and only markdown");
  assert(sql.includes("path !~ "), "and a path that can climb out is refused by its own check");
});

/** Commitment-model spec §10 phase 1s: the path check that is live once `commitments/` syncs. Read
 * by name — it is not stamped in C3's day, so `migrations()` never sees it, and 000300's assertion
 * above keeps pinning that file's own (superseded) six-folder check. */
const COMMITMENTS_PATH_CHECK = "20260926000100_sync_note_path_check_commitments.sql";

Deno.test("the live note-path check names the seven note folders, commitments/ among them", async () => {
  const sql = await Deno.readTextFile(new URL(COMMITMENTS_PATH_CHECK, DIR));
  assert(sql.includes("drop constraint sync_notes_path_check,"), "it replaces the one constraint, in place");
  assert(sql.includes("add constraint sync_notes_path_check check ("), "and re-declares it under the same name");
  assert(
    sql.includes("path ~ '^(tasks|approvals|archive|courses|issues|info|commitments)/[A-Za-z0-9._ /-]+\\.md$'"),
    "the seven folders in `ids::NOTE_FOLDERS` order, with 000400's unbounded class",
  );
  assert(
    sql.includes("char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303"),
    "and 000400's separate length check (Postgres caps a bound repetition at 255)",
  );
  assert(!sql.includes("{1,300}"), "no bound repetition over DUPMAX");
  // It is the latest `*sync_note_path_check*.sql` — the one the engine's tripwire reads.
  const names: string[] = [];
  for await (const e of Deno.readDir(DIR)) {
    if (e.isFile && e.name.includes("sync_note_path_check") && e.name.endsWith(".sql")) names.push(e.name);
  }
  names.sort();
  assertEquals(names.at(-1), COMMITMENTS_PATH_CHECK);
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
  // R-C3′-exec-10 review I1, re-review: named at 000300 for three of these four — that is the file
  // that turned `keep` into a generated column, a live property of 000300's own SQL. The first
  // assertion is different: `sync_prune`'s real `delete ... and not keep;` clause lives only in
  // 000100 (untouched by 000300 or 000400), and the same substring in 000300 is prose in a header
  // COMMENT, not the SQL that enforces anything — so it reads 000100 by name instead.
  const pruneSql = await syncPruneSql();
  assert(pruneSql.includes("and not keep;"), "sync_prune must exempt the records marked `keep`");
  const sql = await plaintextShapeSql();
  assert(/keep\s+boolean\s+not null generated always as/.test(sql), "`keep` is generated, not sent");
  assert(sql.includes("'agent:%'"), "an agent's record is not kept");
  assert(sql.includes("in ('set', 'create')"), "only a set or a create is a human decision worth keeping");
});

Deno.test("the account purge names every table C3′ leaves, and no table it dropped", async () => {
  // Hand-off H1. The foreign key cascades anyway; this list is what the privacy policy's deletion
  // paragraph is written from, and a table missing from it is a table nobody remembers to mention —
  // while a table NAMED in it that no longer exists is a 404 on every account deletion.
  const index = await Deno.readTextFile(new URL("./functions/account/index.ts", import.meta.url));
  const purge = index.slice(index.indexOf("purge:"), index.indexOf("deleteAuthUser:"));
  for (const t of await liveTables()) assert(purge.includes(`"${t}"`), `DELETE /account does not purge ${t}`);
  // The quoted NAME, not the word: the comment above the list is allowed to say where
  // `sync_generation` went (hand-off H1 does), and only a string literal in the list is a purge.
  assert(!purge.includes('"sync_generation"'), "sync_generation is gone; purging it is a 404 every time");
});

/** Every single-quoted string literal that follows a regex operator (`~`, `!~`, `~*`, `!~*`) or
 * `similar to` — comments stripped first, so a commented-out example pattern is never scanned and
 * neither is a jsonb literal (`'{"a": 1}'::jsonb` has no such operator before it). SQL doubles an
 * embedded quote (`''`) rather than escaping it; this corpus never actually has one inside a regex
 * literal, but the pattern still consumes a doubled quote as literal content rather than stopping on
 * it, so a future one would not be silently truncated. */
function regexLiteralsIn(sql: string): string[] {
  const stripped = sql
    .split("\n")
    .map((line) => {
      const at = line.indexOf("--");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
  const re = /(?:~\*?|!~\*?|similar\s+to)\s*'((?:[^']|'')*)'/gi;
  return [...stripped.matchAll(re)].map((m) => m[1].replace(/''/g, "'"));
}

/** The largest bound repetition count (`{n}`, `{m,n}`, `{m,}`) in `pattern` that exceeds Postgres's
 * DUPMAX of 255, or `undefined` if every bound in it is at or under the cap. Both numbers of a
 * `{m,n}` form are checked, not just the second: `{300,1}` is nonsense Postgres would refuse for a
 * different reason, but a scan that only read the second number would miss a `{300,}` open bound. */
function maxBoundOver255(pattern: string): number | undefined {
  let worst: number | undefined;
  for (const m of pattern.matchAll(/\{(\d+)(?:,(\d*))?\}/g)) {
    for (const g of [m[1], m[2]]) {
      if (g === undefined || g === "") continue;
      const n = Number(g);
      if (n > 255 && (worst === undefined || n > worst)) worst = n;
    }
  }
  return worst;
}

Deno.test("no regex literal's bound repetition count exceeds Postgres's DUPMAX of 255, except one named, expiring exemption", async () => {
  // Postgres's regex engine caps a bound repetition count at 255 (DUPMAX) and raises 2201B "invalid
  // regular expression: invalid repetition count(s)" the first time a ROW is checked against a
  // pattern over it — not when the migration that declares the CHECK is applied. That is exactly how
  // 20260912000300_sync_plaintext.sql's `[A-Za-z0-9._ /-]{1,300}` bound on `sync_notes_path_check`
  // sat on staging, invisible, until the controller's live smoke actually inserted a note (found
  // 2026-09-22): every insert into `sync_notes` failed. R-C3′-exec-10's fix is
  // 20260912000400_sync_note_path_check.sql, which drops and replaces that one constraint — 000300
  // itself is never edited, since it is already applied to staging and migrations are forward-only.
  //
  // The exemption below names 000300's superseded literal exactly, and is real only as long as 000400
  // actually drops `sync_notes_path_check`: an exemption that outlived the fix it was named for would
  // hide the next regression this guard exists to catch.
  const EXEMPT_FILE = "20260912000300_sync_plaintext.sql";
  const SUPERSEDED_PATTERN = "^(tasks|approvals|archive|courses|issues|info)/[A-Za-z0-9._ /-]{1,300}\\.md$";

  const files = await migrations();
  let sawExemption = false;
  for (const m of files) {
    for (const literal of regexLiteralsIn(m.sql)) {
      const bad = maxBoundOver255(literal);
      if (bad === undefined) continue;
      if (m.name === EXEMPT_FILE && literal === SUPERSEDED_PATTERN) {
        sawExemption = true;
        continue;
      }
      assert(
        false,
        `${m.name}: regex literal ${JSON.stringify(literal)} has a repetition bound of ${bad}, over ` +
          `Postgres's DUPMAX of 255 (2201B "invalid repetition count(s)", found on staging by the ` +
          `controller's smoke of 2026-09-22) — only ${EXEMPT_FILE}'s superseded path check is exempt, ` +
          "and only because 20260912000400 replaces it",
      );
    }
  }
  assert(
    sawExemption,
    `expected to find ${EXEMPT_FILE}'s superseded {1,300} path check as the named exemption — if it is ` +
      "gone, this test's exemption is stale and should be removed along with it",
  );

  // The exemption is real, not merely declared: the file that supersedes 000300's path check must
  // actually drop `sync_notes_path_check`, or the over-255 pattern above is still the live
  // constraint Postgres enforces on every insert.
  const successor = files.find((f) => f.name === "20260912000400_sync_note_path_check.sql");
  assert(successor, "expected 20260912000400_sync_note_path_check.sql to supersede 000300's path check");
  assert(
    successor!.sql.includes("drop constraint sync_notes_path_check"),
    "the exemption only holds if 000400 actually drops sync_notes_path_check — otherwise the " +
      "over-255 pattern named above is still the live constraint",
  );
});
