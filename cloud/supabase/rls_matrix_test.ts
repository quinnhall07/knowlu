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

/** Strips every `--` line comment, as `migrations/migrations_test.ts` does, so a commented-out
 * statement is neither counted nor parsed. Line-based: it never removes a newline. */
function stripLineComments(sql: string): string {
  return sql
    .split("\n")
    .map((line) => {
      const at = line.indexOf("--");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
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

function qualified(schema: string | undefined, name: string): string {
  return `${(schema ?? "public").toLowerCase()}.${name.toLowerCase()}`;
}

/**
 * The relations `files` (sorted, earliest first) leave, with the policies on each live table and every
 * `create schema` name. Statements apply in file order and, within a file, in text order: a
 * `drop table` removes the table and its policies; a later `create table` brings the name back with
 * none; a `create policy` must name a table that is live at that point. Every file passes the
 * parse-count check for every shape, and holds no rename or schema move.
 */
export function liveRelations(files: Array<[string, string]>): Live {
  const relations = new Map<string, Kind>();
  const policies = new Map<string, Set<string>>();
  const schemas: string[] = [];
  for (const [file, raw] of files) {
    const sql = stripLineComments(raw);
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

/**
 * One matrix row, the whole line: two spaces, then the eight D3 columns in order, then `,` or `;`.
 * Spaces after a comma may pad a column; a trailing `--` comment is stripped first. The shape (the
 * values are the matrix's to state):
 *
 *   ('public.accounts', 'table', 'own', 'id', 'tos_version', 'default', '{accounts_select_own}', null),
 *   ('admin.testers', 'view', 'closed_view', null, null, 'revoked', '{}', '42501');
 *
 * `rel` is schema-qualified (`public` or `admin`); `policies` is a text-array literal, `'{}'` for
 * none; `view_insert_code` is one of the SQLSTATEs the V3 probe catches, or `null`.
 */
const ROW = new RegExp(
  String.raw`^  \('((?:public|admin)\.[a-z_][a-z0-9_]*)',\s*'(table|view)',\s*` +
    String.raw`'(own|service|invoker_view|closed_view)',\s*(null|'[a-z_][a-z0-9_]*'),\s*` +
    String.raw`(null|'[a-z_][a-z0-9_]*'),\s*'(default|revoked)',\s*` +
    String.raw`'\{((?:[a-z_][a-z0-9_]*)(?:,[a-z_][a-z0-9_]*)*)?\}',\s*` +
    String.raw`(null|'(?:42501|55000|0A000)')\)\s*[,;]\s*$`,
);
/** Every line that is meant to be a row: it begins `  ('`. */
const ROW_START = /^ {2}\('/;

function unquote(v: string): string | null {
  return v === "null" ? null : v.slice(1, -1);
}

/** The rows of `_matrix.psql`'s text. The count check: every line that begins `  ('` must be read
 * by the strict `ROW` regex, and the failure names each line that was not. */
export function parseMatrix(text: string): Row[] {
  const lines = stripLineComments(text).split("\n");
  const rows: Row[] = [];
  const unread: number[] = [];
  let started = 0;
  lines.forEach((line, i) => {
    if (!ROW_START.test(line)) return;
    started += 1;
    const m = line.match(ROW);
    if (!m) {
      unread.push(i + 1);
      return;
    }
    rows.push({
      line: i + 1,
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
    rows.length === started,
    `_matrix.psql: ${started} line(s) begin "  ('" but the strict row regex read ${rows.length}; ` +
      `unreadable line(s): ${unread.join(", ")}`,
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
  assertThrows(() => liveRelations([["a.sql", "alter table x rename to y;"]]), Error, "rename");
  // A commented-out statement is neither counted nor parsed.
  assertEquals(liveRelations([["a.sql", "-- create table ghost (id uuid);\n"]]).relations.size, 0);
});

Deno.test("a policy on a table that is not live is caught", () => {
  assertThrows(
    () => liveRelations([["a.sql", "create table x (id uuid);\ndrop table x;\ncreate policy p on x\n"]]),
    Error,
    "not a live table",
  );
});

Deno.test("a malformed matrix row trips the count check, naming its line", () => {
  const good = "  ('public.x', 'table', 'service', 'account_id', 'note', 'default', '{}', null),";
  assertEquals(parseMatrix(`insert into tests.matrix values\n${good}\n`).length, 1);
  for (
    const bad of [
      "  ('public.x', 'table', 'owner', 'account_id', 'note', 'default', '{}', null),", // unknown access
      "  ('x', 'table', 'service', 'account_id', 'note', 'default', '{}', null),", // unqualified rel
      "  ('public.x', 'table', 'service', 'account_id', 'note', 'default', '{}'),", // a column short
    ]
  ) {
    assertThrows(
      () => parseMatrix(`insert into tests.matrix values\n${good}\n${bad}\n`),
      Error,
      "line(s): 3",
    );
  }
});

Deno.test("each stray is named: an unlisted relation, a row with no relation, a misstated policy", () => {
  const live = liveRelations([[
    "a.sql",
    "create table x (id uuid);\ncreate table y (id uuid);\ncreate policy y_own on y\n",
  ]]);
  const rows = parseMatrix([
    "  ('public.y', 'table', 'service', 'id', 'note', 'default', '{y_own}', null),",
    "  ('public.z', 'view', 'closed_view', null, null, 'revoked', '{}', '55000');",
  ].join("\n"));
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
