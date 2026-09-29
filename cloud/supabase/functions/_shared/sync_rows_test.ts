import { assert, assertEquals, assertThrows } from "@std/assert";
import { checkNote, checkRecord, isNotePath, MAX_NOTE_BYTES, MAX_RECORD_BYTES, NOTE_PATH_RE } from "./sync_rows.ts";

const DEVICE = "0123456789abcdef";
const HASH = "a".repeat(64);
const ACCOUNT = "acct-1";
const RECORD = '{"actor":"student","device":"LAPTOP","id":"task_0000000001","op":"set","path":"tasks/x.md","ts":"2026-09-17T10:00:00.000Z","via":"dashboard"}';

Deno.test("a well-formed record becomes the row sync_records takes, and nothing else", () => {
  const row = checkRecord({ hash: HASH, body: RECORD }, DEVICE, ACCOUNT);
  assertEquals(Object.keys(row).sort(), ["account_id", "body", "device", "record_hash"]);
  assertEquals(row.account_id, ACCOUNT);
  assertEquals(row.record_hash, HASH);
});

Deno.test("`keep` cannot be sent — the server decides it", () => {
  // The amendment's improvement over the sealed design: the row is readable, so `keep` is a
  // generated column computed from the record. A client that sends one is telling the server
  // something the server already knows better, and the honest answer is a refusal, not a silent drop.
  const e = assertThrows(() => checkRecord({ hash: HASH, body: RECORD, keep: true }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a record whose body is not a journal record is a 400", () => {
  for (const body of ["", "null", "[]", '"a string"', "not json at all", '{"op":"set"}', '{"actor":"student"}']) {
    const e = assertThrows(() => checkRecord({ hash: HASH, body }, DEVICE, ACCOUNT), `${body} was accepted`) as Response;
    assertEquals(e.status, 400);
  }
});

Deno.test("a record over the byte cap is refused, and the cap is BYTES not characters", () => {
  // 16384 `é` is 16384 characters and 32768 bytes; the column's `octet_length` check would refuse
  // it after the validator let it through, and PostgREST's 400 says nothing a student could act on.
  const big = `{"actor":"student","op":"set","pad":"${"é".repeat(9000)}"}`;
  assert(big.length < MAX_RECORD_BYTES, "the test vector must be short in CHARACTERS to be a test");
  const e = assertThrows(() => checkRecord({ hash: HASH, body: big }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a record body carrying a null byte is a named 400, not a 502 from the database", () => {
  // R-C3′-exec-9 m2: `\u0000` is valid JSON text, and `JSON.parse` accepts it, but `body::jsonb`
  // (the `keep` generated column) rejects it with 22P05 — the migration's own comment names the
  // validator as the only guard standing between a client and that 5xx.
  const withNul = '{"actor":"student","op":"set","note":"a\u0000b"}';
  const e = assertThrows(() => checkRecord({ hash: HASH, body: withNul }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a note path is one of the note folders, markdown, and cannot climb out", () => {
  for (const ok of ["tasks/x.md", "courses/cs-100.md", "archive/a-b.md", "info/x.md", "issues/i.md", "approvals/amend-1.md"]) {
    assert(isNotePath(ok), ok);
  }
  for (const bad of [
    "state/journal/2026-09-17.jsonl", "config/ingest.yaml", "tasks/../../etc/hosts", "../tasks/x.md",
    "tasks//x.md", "tasks/x.txt", "tasks\\x.md", "/tasks/x.md", "tasks/", "", "TASKS/x.md",
  ]) {
    assert(!isNotePath(bad), `${bad} was accepted`);
  }
});

Deno.test("a grade note and a commitment note are note paths; state/grades.json is not (grades spec §7)", () => {
  // `grades/` is the engine's newest note folder. `commitments/` is accepted too, on purpose: the
  // commitment model's branch adds that folder on its own, and a path the server refuses wedges
  // every push (the batch is all-or-nothing), so the server takes both whichever branch merges first.
  for (const ok of ["grades/x.md", "grades/cs-100-hw-01.md", "commitments/x.md"]) {
    assert(isNotePath(ok), ok);
  }
  for (const bad of ["state/grades.json", "grades/x.json", "grades/", "grade/x.md", "grades/../tasks/a.md", "GRADES/x.md"]) {
    assert(!isNotePath(bad), `${bad} was accepted`);
  }
});

Deno.test("a tombstone carries no bytes and a live note carries them", () => {
  const dead = checkNote({ path: "tasks/x.md", deleted: true }, DEVICE, ACCOUNT);
  assertEquals(dead.deleted, true);
  assertEquals(dead.body, null);
  const live = checkNote({ path: "tasks/x.md", body: "---\nid: task_0000000001\n---\n" }, DEVICE, ACCOUNT);
  assertEquals(live.deleted, false);
  assert(typeof live.body === "string");
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", deleted: true, body: "x" }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a note over the byte cap is refused", () => {
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", body: "x".repeat(MAX_NOTE_BYTES + 1) }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a note body carrying a null byte is a named 400", () => {
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", body: "a\u0000b" }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("`deleted`, when sent, must be a boolean", () => {
  // R-C3′-exec-9 m5: `deleted: "true"` is not `=== true`, so with no type check it would fall
  // through to the live-note branch and be stored as a live note carrying `body: "x"` — the
  // opposite of what a client sending `deleted` meant.
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", deleted: "true", body: "x" }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("an unknown field beside the body is a 400 that names it", () => {
  // The single way a patched client could turn this store into something else is by sending a field
  // beside the body and hoping we write it.
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", body: "x", title: "CS 100 HW 1" }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

const MIGRATIONS = new URL("../../migrations/", import.meta.url);

/** `sql` with every `--` comment removed, so a header that quotes an older check is never read as code. */
function sqlCode(sql: string): string {
  return sql.split("\n").map((line) => (line.includes("--") ? line.slice(0, line.indexOf("--")) : line)).join("\n");
}

/** Every `*.sql` migration, whatever its day or name. */
async function allMigrations(): Promise<{ name: string; sql: string }[]> {
  const out: { name: string; sql: string }[] = [];
  for await (const e of Deno.readDir(MIGRATIONS)) {
    if (e.isFile && e.name.endsWith(".sql")) out.push({ name: e.name, sql: await Deno.readTextFile(new URL(e.name, MIGRATIONS)) });
  }
  return out;
}

/** The LIVE note-path check: of `files`, the latest in name order whose code (comments stripped)
 * re-adds `sync_notes_path_check`. Migrations apply in name order, so the last one to re-add the
 * constraint is the one Postgres enforces, whatever the file is called (W1 M1T1-important). */
function livePathCheck(files: { name: string; sql: string }[]): { name: string; sql: string } {
  // `\b` keeps the climb-out siblings (`sync_notes_path_check1`, `…2`) out.
  const adds = /\badd\s+constraint\s+sync_notes_path_check\b/i;
  let live: { name: string; sql: string } | undefined;
  for (const m of files) {
    if (adds.test(sqlCode(m.sql)) && (live === undefined || m.name > live.name)) live = m;
  }
  assert(live, "expected at least one migration that adds sync_notes_path_check");
  return live!;
}

/** The `|`-separated folder group of the one `path ~ '^(…)` in `sql`'s code. */
function folderGroup(name: string, sql: string): string {
  const groups = [...sqlCode(sql).matchAll(/path\s+~\s+'\^\(([^)]*)\)/g)];
  assertEquals(groups.length, 1, `exactly one folder group in ${name}`);
  return groups[0][1];
}

/** NOTE_PATH_RE's own folder group. */
const NOTE_GROUP = NOTE_PATH_RE.source.slice(2, NOTE_PATH_RE.source.indexOf(")"));

Deno.test("the live path check is chosen by what a migration declares, not by its file name", async () => {
  // W1 M1T1-important: a later migration that redefines `sync_notes_path_check` under another name
  // (two-desktop's planned `…_shared_settings.sql`) must be the one read, and its narrower group must
  // fail the group check below. The later migrations are in-memory strings; no file is written.
  const settings = {
    name: "20991231000100_shared_settings.sql",
    sql: String.raw`alter table public.sync_notes
  drop constraint sync_notes_path_check,
  add constraint sync_notes_path_check check (
    (
      path ~ '^(tasks|approvals|archive|courses|issues|info|commitments)/[A-Za-z0-9._ /-]+\.md$'
      and char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303
    )
    or path in ('config/campus.yaml', 'config/events.yaml')
  );
`,
  };
  const sibling = {
    name: "20991231000200_sync_notes_climb_out.sql",
    sql: String.raw`alter table public.sync_notes
  drop constraint sync_notes_path_check1,
  add constraint sync_notes_path_check1 check (path !~ '(^|/)\.\.(/|$)');
`,
  };
  const commentOnly = {
    name: "20991231000300_sync_note_path_check_note.sql",
    sql: String.raw`-- an example only:
-- alter table public.sync_notes add constraint sync_notes_path_check check (path ~ '^(tasks)/x\.md$');
select 1;
`,
  };
  const live = livePathCheck([...(await allMigrations()), settings, sibling, commentOnly]);
  assertEquals(live.name, settings.name, "the last migration that adds sync_notes_path_check is the live one");
  const group = folderGroup(live.name, live.sql);
  assert(group !== NOTE_GROUP, "and the group check refuses it: its group is not NOTE_PATH_RE's");
  assert(!group.split("|").includes("grades"), `the narrowed group lacks grades: ${group}`);
});

Deno.test("the byte bounds and the note path's character class match the migrations' own checks", async () => {
  // R-C3′-exec-9 m3, amended by R-C3′-exec-10: nothing else ties `MAX_RECORD_BYTES`,
  // `MAX_NOTE_BYTES` or `NOTE_PATH_RE`'s character class to the column checks a drift here would
  // fall through to as an unexplained 502 (`db.ts`'s `ok()` logs PostgREST's error text, which for a
  // check violation names the row). The two byte bounds are still 20260912000300_sync_plaintext.sql's
  // own, untouched — but the note path check itself moved to
  // 20260912000400_sync_note_path_check.sql, which drops and replaces `sync_notes_path_check`
  // because Postgres's regex engine caps a bound repetition count at 255 (DUPMAX) and 000300's
  // `{1,300}` bound failed every insert on staging (2201B "invalid repetition count(s)", found by
  // the controller's smoke of 2026-09-22; 000300 is never edited).
  const plaintextSql = await Deno.readTextFile(
    new URL("../../migrations/20260912000300_sync_plaintext.sql", import.meta.url),
  );
  assert(plaintextSql.includes(`between 2 and ${MAX_RECORD_BYTES})`), "sync_records.body's octet_length bound");
  assert(plaintextSql.includes(`between 1 and ${MAX_NOTE_BYTES})`), "sync_notes.body's octet_length bound");

  // The LIVE path check, not 000400 by name (grades spec §7): each migration that widens the folder
  // group drops and re-adds `sync_notes_path_check`, and migrations apply in name order, so the
  // latest migration whose code re-adds it is the one Postgres enforces, whatever it is called.
  const { name: pathCheckName, sql: pathCheckSql } = livePathCheck(await allMigrations());
  assertEquals(folderGroup(pathCheckName, pathCheckSql), NOTE_GROUP, `${pathCheckName}'s folder group is NOTE_PATH_RE's`);
  // The character class alone — the SQL's version is unbounded (`+`), since Postgres cannot express
  // a bound over 255, where the device's regex still bounds it ({1,300}); only the class itself is
  // shared text between the two, so it is derived from the constant rather than retyped.
  const charClass = NOTE_PATH_RE.source.match(/\[[^\]]+\]/)?.[0];
  assert(charClass, "NOTE_PATH_RE must contain a character class");
  assert(pathCheckSql.includes(`${charClass!}+`), "sync_notes.path's character class, unbounded in SQL");

  // The length bound the SQL checks separately (`char_length(...) between 4 and 303`): NOTE_PATH_RE's
  // own {1,300} plus the three characters of ".md", which sit outside the character class but
  // inside what `regexp_replace(path, '^[a-z]+/', '')` measures — 1+3=4 and 300+3=303, computed from
  // the constant so a future change to 300 cannot drift silently from the SQL's hand-typed 303.
  const bound = NOTE_PATH_RE.source.match(/\{(\d+),(\d+)\}/);
  assert(bound, "NOTE_PATH_RE must contain a {min,max} bound");
  const [, minStr, maxStr] = bound!;
  const minLen = Number(minStr) + 3;
  const maxLen = Number(maxStr) + 3;
  assert(
    pathCheckSql.includes(`between ${minLen} and ${maxLen}`),
    `sync_notes.path's length bound (expected 'between ${minLen} and ${maxLen}')`,
  );
});

Deno.test("every sync query is scoped to one account, and nothing bypasses C1's helpers", async () => {
  const src = await Deno.readTextFile(new URL("./sync_db.ts", import.meta.url));
  for (const table of ["sync_records", "sync_notes", "sync_usage"]) {
    for (const call of src.matchAll(new RegExp(`"${table}",\\s*\`([^\`]*)\``, "g"))) {
      assert(call[1].includes("account_id=eq."), `${table}: a query without account_id=eq.: ${call[1]}`);
    }
  }
  assertEquals([...src.matchAll(/rest\.fetch\(/g)].length, 0, "no raw fetch: C1's helpers are the only path");
  assert(!src.includes("sync_generation"), "the key generation is gone");
});
