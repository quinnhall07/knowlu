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
  // lexically latest `*sync_note_path_check*.sql` is the one Postgres enforces.
  const migrationsDir = new URL("../../migrations/", import.meta.url);
  const pathChecks: string[] = [];
  for await (const e of Deno.readDir(migrationsDir)) {
    if (e.isFile && e.name.includes("sync_note_path_check") && e.name.endsWith(".sql")) pathChecks.push(e.name);
  }
  pathChecks.sort();
  assert(pathChecks.length > 0, "expected at least one *sync_note_path_check*.sql");
  const pathCheckSql = await Deno.readTextFile(new URL(pathChecks.at(-1)!, migrationsDir));
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
