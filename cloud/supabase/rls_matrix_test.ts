// The static twin of the pgTAP coverage case C1 (RLS pgTAP plan, D4). It runs in the required `cloud`
// job, needs no Docker and reads files only, so a migration that adds a table, a view or an `admin`
// relation cannot slip past the RLS matrix even before `cloud-rls` is required. It derives the live
// relation set from the migration text and compares it, both ways, with `tests/_matrix.psql`.
import { assert, assertEquals, assertThrows } from "@std/assert";

const MIGRATIONS = new URL("./migrations/", import.meta.url);
const MATRIX = new URL("./tests/_matrix.psql", import.meta.url);

export type Kind = "table" | "view";

/** What the migrations leave, in order: each live relation (`schema.name`) with its kind, the policy
 * names on each live table, and every `create schema` name. */
export interface Live {
  relations: Map<string, Kind>;
  policies: Map<string, Set<string>>;
  schemas: string[];
}

/** Every `.sql` file directly under `migrations/`, sorted, earliest first. */
async function everyMigrationFile(): Promise<Array<[string, string]>> {
  const out: Array<[string, string]> = [];
  for await (const entry of Deno.readDir(MIGRATIONS)) {
    if (entry.isFile && entry.name.endsWith(".sql")) {
      out.push([entry.name, await Deno.readTextFile(new URL(entry.name, MIGRATIONS))]);
    }
  }
  out.sort();
  return out;
}

/** SQL text with every comment blanked, and the ranges of its literals. */
interface Lexed {
  code: string;
  literals: Array<[number, number]>;
}

const DOLLAR_TAG = /\$(?:[A-Za-z_][A-Za-z0-9_]*)?\$/y;
const PLAIN_IDENT = /^[a-z_][a-z0-9_]*$/;

/**
 * One pass over `sql`. It blanks `--` line comments and `/* … *\/` block comments, nested as Postgres
 * nests them, so a commented-out statement is neither counted nor parsed (a `/* down: drop table x; *\/`
 * must never drop `x`). Blanking keeps every newline and offset, so line numbers survive. It records
 * the range of each top-level single-quoted string and dollar-quoted (`$tag$ … $tag$`) body. The text
 * inside a body is still lexed, so a plpgsql comment there is blanked too. It fails loud on:
 * - an unterminated comment, string or body;
 * - a `*\/` with no `/*` before it;
 * - a quoted identifier with an uppercase or non-word character (the parser reads names unquoted and
 *   lowercased, so `"Foo"` or `"a b"` would be read as some other name).
 */
function lex(file: string, sql: string): Lexed {
  const out = sql.split("");
  const literals: Array<[number, number]> = [];
  const dollars: Array<{ tag: string; at: number }> = [];
  const blank = (from: number, to: number) => {
    for (let k = from; k < to; k++) if (out[k] !== "\n") out[k] = " ";
  };
  let i = 0;
  while (i < sql.length) {
    if (sql.startsWith("--", i)) {
      const nl = sql.indexOf("\n", i);
      const stop = nl === -1 ? sql.length : nl;
      blank(i, stop);
      i = stop;
    } else if (sql.startsWith("/*", i)) {
      let depth = 0;
      let j = i;
      while (j < sql.length) {
        if (sql.startsWith("/*", j)) {
          depth += 1;
          j += 2;
        } else if (sql.startsWith("*/", j)) {
          depth -= 1;
          j += 2;
          if (depth === 0) break;
        } else j += 1;
      }
      assert(depth === 0, `${file}: a /* block comment is never closed`);
      blank(i, j);
      i = j;
    } else if (sql.startsWith("*/", i)) {
      throw new Error(`${file}: a */ with no /* before it`);
    } else if (sql[i] === "'") {
      let j = i + 1;
      while (j < sql.length && !(sql[j] === "'" && sql[j + 1] !== "'")) j += sql[j] === "'" ? 2 : 1;
      assert(j < sql.length, `${file}: a string is never closed`);
      if (dollars.length === 0) literals.push([i, j + 1]);
      i = j + 1;
    } else if (sql[i] === '"') {
      const j = sql.indexOf('"', i + 1);
      assert(j !== -1, `${file}: a quoted identifier is never closed`);
      assert(
        PLAIN_IDENT.test(sql.slice(i + 1, j)),
        `${file}: a quoted identifier with an uppercase or non-word character; this twin reads names ` +
          `unquoted and lowercased, so it would read another name`,
      );
      i = j + 1;
    } else {
      DOLLAR_TAG.lastIndex = i;
      const m = sql[i] === "$" ? DOLLAR_TAG.exec(sql) : null;
      if (!m) {
        i += 1;
        continue;
      }
      const top = dollars[dollars.length - 1];
      if (top && top.tag === m[0]) {
        dollars.pop();
        if (dollars.length === 0) literals.push([top.at, i + m[0].length]);
      } else dollars.push({ tag: m[0], at: i });
      i += m[0].length;
    }
  }
  assert(dollars.length === 0, `${file}: a ${dollars[0]?.tag} body is never closed`);
  return { code: out.join(""), literals };
}

/** A relation name: unqualified (the C2 files' spelling; `search_path` puts it in `public`) or
 * qualified by `public` or `admin`, either half optionally double-quoted. Group a is the schema,
 * group b the name. A third schema does not parse, so it fails the parse-count check loud. */
const REL = String.raw`(?:"?(public|admin)"?\s*\.\s*)?"?(\w+)"?`;
/** The name ends here: no further identifier character, dot or quote follows it. */
const END = String.raw`(?![\w."])`;

type Shape =
  | "createTable"
  | "dropTable"
  | "createView"
  | "dropView"
  | "createPolicy"
  | "dropPolicy"
  | "createSchema";

/** One loose and one strict regex per statement shape. The loose one counts every occurrence of the
 * statement's keywords; the strict one reads its names. In each file the two counts must agree, so a
 * spelling the strict parser misses (`create unlogged table`, a third schema, `drop table … cascade`,
 * whose dependents this twin cannot know, a two-name `drop table`) fails loud instead of vanishing. */
const SHAPES: Record<Shape, { loose: RegExp; strict: RegExp }> = {
  createTable: {
    loose: /\bcreate\s+(?:\w+\s+){0,2}table\b/gi,
    strict: new RegExp(String.raw`\bcreate\s+table\s+(?:if\s+not\s+exists\s+)?${REL}\s*\(`, "gi"),
  },
  dropTable: {
    loose: /\bdrop\s+table\b/gi,
    strict: new RegExp(String.raw`\bdrop\s+table\s+(?:if\s+exists\s+)?${REL}\s*(?:restrict\s*)?;`, "gi"),
  },
  createView: {
    loose: /\bcreate\s+(?:\w+\s+){0,3}view\b/gi,
    strict: new RegExp(
      String
        .raw`\bcreate\s+(?:or\s+replace\s+)?(?:materialized\s+)?view\s+(?:if\s+not\s+exists\s+)?${REL}${END}`,
      "gi",
    ),
  },
  dropView: {
    loose: /\bdrop\s+(?:materialized\s+)?view\b/gi,
    strict: new RegExp(
      String.raw`\bdrop\s+(?:materialized\s+)?view\s+(?:if\s+exists\s+)?${REL}\s*(?:restrict\s*)?;`,
      "gi",
    ),
  },
  createPolicy: {
    loose: /\bcreate\s+policy\b/gi,
    strict: new RegExp(String.raw`\bcreate\s+policy\s+"?(\w+)"?\s+on\s+${REL}${END}`, "gi"),
  },
  dropPolicy: {
    loose: /\bdrop\s+policy\b/gi,
    strict: new RegExp(
      String
        .raw`\bdrop\s+policy\s+(?:if\s+exists\s+)?"?(\w+)"?\s+on\s+${REL}\s*(?:(?:cascade|restrict)\s*)?;`,
      "gi",
    ),
  },
  createSchema: {
    loose: /\bcreate\s+schema\b/gi,
    strict: new RegExp(String.raw`\bcreate\s+schema\s+(?:if\s+not\s+exists\s+)?"?(\w+)"?${END}`, "gi"),
  },
};

/** A rename or a schema move would carry a relation or a policy out from under its name. This twin
 * does not follow one, so any occurrence fails loud. */
const UNFOLLOWED =
  /\balter\s+(?:table|view|materialized\s+view|policy)\b[^;]*?\b(?:rename\s+to|set\s+schema)\b/gi;

/** A drop inside a literal: a `do $$ … $$` block runs at migration time, a function body does not, and
 * the text alone cannot tell them apart, so any occurrence fails loud. */
const DROP_ANY = /\bdrop\s+(?:materialized\s+)?(?:table|view|policy)\b/gi;

function qualified(schema: string | undefined, name: string): string {
  return `${(schema ?? "public").toLowerCase()}.${name.toLowerCase()}`;
}

/**
 * The relations `files` (sorted, earliest first) leave, with the policies on each live table and every
 * `create schema` name. Statements apply in file order and, within a file, in text order: a
 * `drop table` removes the table and its policies; a later `create table` brings the name back with
 * none; a `create policy` must name a table that is live at that point. Every file lexes cleanly
 * (`lex`), passes the parse-count check for every shape, and holds no rename, no schema move and no
 * drop inside a string or a dollar-quoted body.
 */
export function liveRelations(files: Array<[string, string]>): Live {
  const relations = new Map<string, Kind>();
  const policies = new Map<string, Set<string>>();
  const schemas: string[] = [];
  for (const [file, raw] of files) {
    const { code: sql, literals } = lex(file, raw);
    for (const d of sql.matchAll(DROP_ANY)) {
      const at = d.index ?? -1;
      assert(
        !literals.some(([from, to]) => at >= from && at < to),
        `${file}: a drop inside a string or a dollar-quoted body; this twin cannot tell whether it runs`,
      );
    }
    const unfollowed = sql.match(UNFOLLOWED) ?? [];
    assert(
      unfollowed.length === 0,
      `${file}: ${unfollowed.length} rename or schema move(s); this twin cannot follow a relation or ` +
        `a policy to a new name, so it must learn the shape before the migration can land`,
    );
    const events: Array<{ at: number; shape: Shape; m: RegExpMatchArray }> = [];
    for (const shape of Object.keys(SHAPES) as Shape[]) {
      const { loose, strict } = SHAPES[shape];
      const looseCount = (sql.match(loose) ?? []).length;
      const parsed = [...sql.matchAll(strict)];
      assert(
        looseCount === parsed.length,
        `${file}: ${looseCount} '${shape}' statement(s) by the loose count, but the strict parser ` +
          `read ${parsed.length}. A spelling this twin cannot read is a relation the matrix cannot see.`,
      );
      for (const m of parsed) events.push({ at: m.index ?? -1, shape, m });
    }
    events.sort((a, b) => a.at - b.at);
    for (const { shape, m } of events) {
      switch (shape) {
        case "createTable": {
          const rel = qualified(m[1], m[2]);
          relations.set(rel, "table");
          if (!policies.has(rel)) policies.set(rel, new Set());
          break;
        }
        case "dropTable": {
          const rel = qualified(m[1], m[2]);
          if (relations.get(rel) === "table") relations.delete(rel);
          policies.delete(rel);
          break;
        }
        case "createView":
          relations.set(qualified(m[1], m[2]), "view");
          break;
        case "dropView": {
          const rel = qualified(m[1], m[2]);
          if (relations.get(rel) === "view") relations.delete(rel);
          break;
        }
        case "createPolicy": {
          const rel = qualified(m[2], m[3]);
          assert(
            relations.get(rel) === "table",
            `${file}: policy ${m[1].toLowerCase()} names ${rel}, which is not a live table here`,
          );
          policies.get(rel)?.add(m[1].toLowerCase());
          break;
        }
        case "dropPolicy":
          policies.get(qualified(m[2], m[3]))?.delete(m[1].toLowerCase());
          break;
        case "createSchema":
          schemas.push(m[1].toLowerCase());
          break;
      }
    }
  }
  return { relations, policies, schemas };
}

export type Access = "own" | "service" | "invoker_view" | "closed_view";

/** One `tests.matrix` row, with the columns of D3. */
export interface Row {
  line: number;
  rel: string;
  kind: Kind;
  access: Access;
  keyCol: string | null;
  touchCol: string | null;
  grants: "default" | "revoked";
  policies: string[];
  viewInsertCode: string | null;
}

/** D3's columns, in order: the matrix insert's column list must be exactly this. */
const MATRIX_COLUMNS = [
  "rel",
  "kind",
  "access",
  "key_col",
  "touch_col",
  "grants",
  "policies",
  "view_insert_code",
];
const MATRIX_INSERT = /\binsert\s+into\s+tests\s*\.\s*matrix\b/gi;
const MATRIX_HEAD = /\binsert\s+into\s+tests\s*\.\s*matrix\s*\(([^)]*)\)\s*values\b/gi;

/**
 * The matrix is one statement, and only its rows are read. The `values` keyword ends its line, and
 * each row is a whole line of its own: leading whitespace, then the eight D3 columns in order, then
 * `,` (or `;` on the last row). Spaces after a comma may pad a column, and a trailing `--` comment is
 * blanked first. The shape (the values are the matrix's to state):
 *
 *   insert into tests.matrix (rel, kind, access, key_col, touch_col, grants, policies, view_insert_code) values
 *     ('public.accounts', 'table', 'own', 'id', 'tos_version', 'default', '{accounts_select_own}', null),
 *     ('admin.testers', 'view', 'closed_view', null, null, 'revoked', '{}', '42501');
 *
 * `rel` is schema-qualified (`public` or `admin`). `policies` is a text-array literal, `'{}'` for
 * none. `view_insert_code` is one of the SQLSTATEs the V3 probe catches, or `null`. Rows in any other
 * statement (a future `tests.rpc_allowlist` row) are not this parser's.
 */
const ROW = new RegExp(
  String.raw`^\s+\('((?:public|admin)\.[a-z_][a-z0-9_]*)',\s*'(table|view)',\s*` +
    String.raw`'(own|service|invoker_view|closed_view)',\s*(null|'[a-z_][a-z0-9_]*'),\s*` +
    String.raw`(null|'[a-z_][a-z0-9_]*'),\s*'(default|revoked)',\s*` +
    String.raw`'\{((?:[a-z_][a-z0-9_]*)(?:,[a-z_][a-z0-9_]*)*)?\}',\s*` +
    String.raw`(null|'(?:42501|55000|0A000)')\)\s*[,;]\s*$`,
);
/** Every line inside the matrix insert that is meant to be a row: whitespace, then `(`. */
const ROW_START = /^\s+\(/;

function unquote(v: string): string | null {
  return v === "null" ? null : v.slice(1, -1);
}

/**
 * The rows of `_matrix.psql`'s text, read from its one `insert into tests.matrix (…) values … ;`
 * statement, whose column list must be D3's. The count check: every line inside the statement that
 * begins with whitespace and `(` must be read by the strict `ROW` regex; any other non-blank text
 * there is unreadable too; the failure names each such line.
 */
export function parseMatrix(text: string): Row[] {
  const code = lex("_matrix.psql", text).code;
  const inserts = [...code.matchAll(MATRIX_INSERT)].length;
  const heads = [...code.matchAll(MATRIX_HEAD)];
  assert(
    inserts === 1 && heads.length === 1,
    `_matrix.psql: ${inserts} 'insert into tests.matrix' statement(s), ${heads.length} of them with a ` +
      `column list and 'values'; there must be exactly one, with both`,
  );
  const head = heads[0];
  assertEquals(
    head[1].split(",").map((c) => c.trim().toLowerCase()),
    MATRIX_COLUMNS,
    "_matrix.psql: the matrix insert's column list must be D3's, in order",
  );
  const bodyStart = (head.index ?? 0) + head[0].length;
  const semi = code.indexOf(";", bodyStart);
  assert(semi !== -1, "_matrix.psql: the matrix insert has no terminating ';'");
  const firstLine = code.slice(0, bodyStart).split("\n").length;
  const rows: Row[] = [];
  const unread: number[] = [];
  let started = 0;
  code.slice(bodyStart, semi + 1).split("\n").forEach((line, k) => {
    const lineNo = firstLine + k;
    if (k === 0 || !ROW_START.test(line)) {
      if (line.trim() !== "") unread.push(lineNo);
      return;
    }
    started += 1;
    const m = line.match(ROW);
    if (!m) {
      unread.push(lineNo);
      return;
    }
    rows.push({
      line: lineNo,
      rel: m[1],
      kind: m[2] as Kind,
      access: m[3] as Access,
      keyCol: unquote(m[4]),
      touchCol: unquote(m[5]),
      grants: m[6] as "default" | "revoked",
      policies: m[7] ? m[7].split(",") : [],
      viewInsertCode: unquote(m[8]),
    });
  });
  assert(
    rows.length === started && unread.length === 0,
    `_matrix.psql: ${started} row line(s) in the matrix insert, ${rows.length} read by the strict row ` +
      `regex; unreadable line(s): ${unread.join(", ")}`,
  );
  return rows;
}

/** Coverage, both ways: every live relation has exactly one row of its kind, and every row names a
 * live relation. Each problem names its relation. */
export function coverageProblems(live: Live, rows: Row[]): string[] {
  const problems: string[] = [];
  const seen = new Map<string, Row>();
  for (const row of rows) {
    if (seen.has(row.rel)) problems.push(`${row.rel}: more than one matrix row (line ${row.line})`);
    seen.set(row.rel, row);
  }
  for (const [rel, kind] of [...live.relations].sort()) {
    const row = seen.get(rel);
    if (!row) problems.push(`${rel}: a live ${kind} with no matrix row`);
    else if (row.kind !== kind) problems.push(`${rel}: a live ${kind}, but its row says ${row.kind}`);
  }
  for (const rel of [...seen.keys()].sort()) {
    if (!live.relations.has(rel)) problems.push(`${rel}: a matrix row with no live relation`);
  }
  return problems;
}

/** Policies and row shape: a table that a live policy names is `own` and lists exactly those
 * policies; a `service` or view row lists none; a table row is `own` or `service` with no
 * `view_insert_code`, and a view row is `invoker_view` or `closed_view` with one. */
export function policyProblems(live: Live, rows: Row[]): string[] {
  const problems: string[] = [];
  for (const row of rows) {
    if (row.kind === "table" && !(row.access === "own" || row.access === "service")) {
      problems.push(`${row.rel}: a table row with access ${row.access}`);
    }
    if (row.kind === "view" && !(row.access === "invoker_view" || row.access === "closed_view")) {
      problems.push(`${row.rel}: a view row with access ${row.access}`);
    }
    if ((row.kind === "view") !== (row.viewInsertCode !== null)) {
      problems.push(`${row.rel}: view_insert_code is for view rows, and every view row needs one`);
    }
    if (row.access !== "own" && row.policies.length > 0) {
      problems.push(`${row.rel}: a ${row.access} row lists policies ${row.policies.join(",")}`);
    }
    if (row.kind !== "table" || live.relations.get(row.rel) !== "table") continue;
    const names = [...(live.policies.get(row.rel) ?? [])].sort();
    const listed = [...row.policies].sort();
    if (names.length > 0 && row.access !== "own") {
      problems.push(`${row.rel}: live policies ${names.join(",")}, but the row is ${row.access}`);
    }
    if (names.length === 0 && row.access === "own") {
      problems.push(`${row.rel}: an own row, but no live policy names the table`);
    }
    if (row.access === "own" && names.join(",") !== listed.join(",")) {
      problems.push(`${row.rel}: live policies {${names.join(",")}}, the row lists {${listed.join(",")}}`);
    }
  }
  return problems;
}

/** Every `create schema` names `admin`; each other name is a problem. */
export function schemaProblems(live: Live): string[] {
  return live.schemas.filter((s) => s !== "admin").map((s) => `create schema ${s}: only admin is allowed`);
}

async function readMatrix(): Promise<string> {
  try {
    return await Deno.readTextFile(MATRIX);
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) {
      throw new Error(
        "cloud/supabase/tests/_matrix.psql does not exist: the RLS matrix (D3) must name every " +
          "relation the migrations leave",
      );
    }
    throw e;
  }
}

Deno.test("the migrations parse into a live relation set that holds today's landmarks", async () => {
  const live = liveRelations(await everyMigrationFile());
  // Qualified and unqualified spellings, `if not exists`, `or replace`, and `admin` all land.
  for (const rel of ["public.accounts", "public.models", "public.mail_accounts", "public.route_daily"]) {
    assertEquals(live.relations.get(rel), "table", `${rel} should be a live table`);
  }
  for (const rel of ["public.billing_subscribers", "public.monthly_spend", "admin.testers"]) {
    assertEquals(live.relations.get(rel), "view", `${rel} should be a live view`);
  }
  // 20260912000300 drops sync_generation for good, and drops and recreates sync_records: the
  // recreated table carries only the policy created after the recreate.
  assert(!live.relations.has("public.sync_generation"), "sync_generation was dropped");
  assertEquals([...(live.policies.get("public.sync_records") ?? [])], ["sync_records_select_own"]);
  assert(!live.policies.has("public.sync_generation"), "a dropped table's policies go with it");
});

Deno.test("create schema in every migration names only admin", async () => {
  const live = liveRelations(await everyMigrationFile());
  assertEquals(schemaProblems(live), []);
  assert(live.schemas.includes("admin"), "the testers migration creates admin");
});

Deno.test("every live relation has exactly one matrix row, and every matrix row a live relation", async () => {
  const live = liveRelations(await everyMigrationFile());
  const problems = coverageProblems(live, parseMatrix(await readMatrix()));
  assert(problems.length === 0, `the RLS matrix lags the migrations:\n${problems.join("\n")}`);
});

Deno.test("every table a live policy names is own with exactly those policies; no service or view row lists one", async () => {
  const live = liveRelations(await everyMigrationFile());
  const problems = policyProblems(live, parseMatrix(await readMatrix()));
  assert(problems.length === 0, `the RLS matrix misstates policies:\n${problems.join("\n")}`);
});

// Negative tests: each guard above fires, in the style of migrations/migrations_test.ts.

Deno.test("an unqualified create table if not exists is found, in public", () => {
  const live = liveRelations([["a.sql", "create table if not exists x (\n  id uuid primary key\n);\n"]]);
  assertEquals([...live.relations], [["public.x", "table"]]);
});

Deno.test("a dropped table is gone, and a recreated one is back without its old policies", () => {
  const live = liveRelations([
    [
      "a.sql",
      "create table public.x (id uuid);\ncreate table y (id uuid);\ncreate policy x_old on public.x\n",
    ],
    ["b.sql", "drop table if exists public.x;\ndrop table y;\ncreate table public.x (id uuid);\n"],
  ]);
  assertEquals([...live.relations], [["public.x", "table"]]);
  assertEquals([...(live.policies.get("public.x") ?? [])], []);
});

Deno.test("a spelling the strict parser misses fails the parse-count check loud", () => {
  for (
    const sql of [
      "create unlogged table x (id uuid);",
      "create table other.x (id uuid);",
      "drop table x cascade;",
    ]
  ) {
    assertThrows(
      () => liveRelations([["a.sql", `create table x (id uuid);\n${sql}\n`]]),
      Error,
      "strict parser",
    );
  }
  for (
    const sql of [
      "alter table x rename to y;",
      "alter view v rename to w;",
      "alter policy p on x rename to q;",
      "alter table x set schema admin;",
    ]
  ) {
    assertThrows(() => liveRelations([["a.sql", sql]]), Error, "rename or schema move", sql);
  }
  // A commented-out statement is neither counted nor parsed.
  assertEquals(liveRelations([["a.sql", "-- create table ghost (id uuid);\n"]]).relations.size, 0);
});

Deno.test("a block comment hides its statements, nested as Postgres nests them", () => {
  const live = liveRelations([[
    "a.sql",
    "create table grade_cache (id uuid);\n/* down: drop table grade_cache; */\n" +
    "/* outer /* inner */ drop table grade_cache; */\n/* -- a dash pair inside */ create table y (id uuid);\n",
  ]]);
  assertEquals([...live.relations].sort(), [["public.grade_cache", "table"], ["public.y", "table"]]);
  assertThrows(
    () => liveRelations([["a.sql", "/* never closed\ncreate table x (id uuid);"]]),
    Error,
    "never closed",
  );
  assertThrows(() => liveRelations([["a.sql", "/* a /* b */ c */ d */"]]), Error, "no /* before it");
});

Deno.test("a drop inside a string or a dollar-quoted body fails loud", () => {
  for (
    const sql of [
      "create table x (id uuid);\ndo $$ begin drop table x; end $$;",
      "create table x (id uuid);\ncreate function f() returns void as $fn$ drop view v; $fn$ language sql;",
      "create table x (id uuid);\ncomment on table x is 'drop policy p on x;';",
    ]
  ) {
    assertThrows(() => liveRelations([["a.sql", sql]]), Error, "drop inside a string", sql);
  }
});

Deno.test("a quoted identifier with an uppercase or non-word character fails loud", () => {
  assertEquals([...liveRelations([["a.sql", 'create table "public"."x" (id uuid);']]).relations.keys()], [
    "public.x",
  ]);
  for (const sql of ['create table "Grades" (id uuid);', 'create view "grade cache" as select 1;']) {
    assertThrows(() => liveRelations([["a.sql", sql]]), Error, "quoted identifier", sql);
  }
});

Deno.test("a dropped view is gone", () => {
  const live = liveRelations([[
    "a.sql",
    "create view v as select 1;\ncreate or replace view admin.w as select 1;\ndrop view if exists v;\n",
  ]]);
  assertEquals([...live.relations], [["admin.w", "view"]]);
});

Deno.test("a policy on a table that is not live is caught", () => {
  assertThrows(
    () => liveRelations([["a.sql", "create table x (id uuid);\ndrop table x;\ncreate policy p on x\n"]]),
    Error,
    "not a live table",
  );
});

const COLUMNS = "(rel, kind, access, key_col, touch_col, grants, policies, view_insert_code)";

/** A matrix statement around `rows`: the last row's trailing `,` becomes the closing `;`. */
function matrix(...rows: string[]): string {
  const body = rows.join("\n").replace(/,(\s*(?:--[^\n]*)?)$/, ";$1");
  return `insert into tests.matrix ${COLUMNS} values\n${body}\n`;
}

Deno.test("a malformed matrix row trips the count check, naming its line", () => {
  const good = "  ('public.x', 'table', 'service', 'account_id', 'note', 'default', '{}', null),";
  assertEquals(parseMatrix(matrix(good, good.replace("public.x", "public.y"))).length, 2);
  for (
    const bad of [
      "  ('public.x', 'table', 'owner', 'account_id', 'note', 'default', '{}', null),", // unknown access
      "  ('x', 'table', 'service', 'account_id', 'note', 'default', '{}', null),", // unqualified rel
      "  ('public.x', 'table', 'service', 'account_id', 'note', 'default', '{}'),", // a column short
      "public.x, table", // not a row at all, inside the statement
    ]
  ) {
    assertThrows(() => parseMatrix(matrix(good, bad, good)), Error, "line(s): 3", bad);
  }
  // A row on the `values` line is unreadable too.
  assertThrows(
    () => parseMatrix(`insert into tests.matrix ${COLUMNS} values ${good.trim().replace(/,$/, ";")}\n`),
    Error,
    "line(s): 1",
  );
});

Deno.test("only the matrix insert's rows are read, at any indentation, under D3's column list", () => {
  const row = "\t    ('public.x', 'table', 'service', 'account_id', 'note', 'default', '{}', null),";
  const elsewhere = "insert into tests.rpc_allowlist (fn) values\n  ('public.f');\n";
  assertEquals(parseMatrix(`${matrix(row)}${elsewhere}`).map((r) => r.rel), ["public.x"]);
  const swapped = COLUMNS.replace("key_col, touch_col", "touch_col, key_col");
  assertThrows(
    () => parseMatrix(`insert into tests.matrix ${swapped} values\n${row.replace(/,$/, ";")}\n`),
    Error,
    "column list must be D3's",
  );
  assertThrows(() => parseMatrix(elsewhere), Error, "exactly one");
  assertThrows(() => parseMatrix(matrix(row) + matrix(row)), Error, "exactly one");
});

Deno.test("each stray is named: an unlisted relation, a row with no relation, a misstated policy", () => {
  const live = liveRelations([[
    "a.sql",
    "create table x (id uuid);\ncreate table y (id uuid);\ncreate policy y_own on y\n",
  ]]);
  const rows = parseMatrix(matrix(
    "  ('public.y', 'table', 'service', 'id', 'note', 'default', '{y_own}', null),",
    "  ('public.z', 'view', 'closed_view', null, null, 'revoked', '{}', '55000'),",
  ));
  assertEquals(coverageProblems(live, rows), [
    "public.x: a live table with no matrix row",
    "public.z: a matrix row with no live relation",
  ]);
  assertEquals(policyProblems(live, rows), [
    "public.y: a service row lists policies y_own",
    "public.y: live policies y_own, but the row is service",
  ]);
  assertEquals(schemaProblems(liveRelations([["a.sql", "create schema if not exists other;"]])), [
    "create schema other: only admin is allowed",
  ]);
});

Deno.test("the row-shape checks fire: kind against access, view_insert_code, an own row with no policy", () => {
  const live = liveRelations([["a.sql", "create table t (id uuid);\ncreate view v as select 1;\n"]]);
  const rows = parseMatrix(matrix(
    "  ('public.t', 'table', 'own', 'id', 'note', 'default', '{t_own}', '55000'),",
    "  ('public.v', 'view', 'service', null, null, 'revoked', '{}', null),",
  ));
  assertEquals(policyProblems(live, rows), [
    "public.t: view_insert_code is for view rows, and every view row needs one",
    "public.t: an own row, but no live policy names the table",
    "public.t: live policies {}, the row lists {t_own}",
    "public.v: a view row with access service",
    "public.v: view_insert_code is for view rows, and every view row needs one",
  ]);
});
