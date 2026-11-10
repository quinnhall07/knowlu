// TDD, per CLAUDE.md: this file is written before `schema.ts`, `loader.ts` or any fixture exists. It fails
// to load (RED — two imports resolve to nothing) until T3b, and stays red until T4 writes `fixtures/`.
//
// T3a of the prompt-injection suite (plan `2026-11-09-prompt-injection-suite`, §5): the real fixture set.
// `loadFixtures` (see `schema_test.ts` for its contract) already refuses a bad line, a hygiene breach, a
// dangling twin and a shared canary; this file checks what only the whole set can show:
//   - the five kind files, and nothing else, each loading;
//   - the coverage table below: every class of §5 that applies to a kind has an attack of that kind;
//   - every attack's twin is a twin of its kind, and every twin is some attack's;
//   - every attack's `planted` strings occur in its own request and in no twin's request, compared as the
//     runner compares outputs (`plantedIn`, on the matching form), so an obfuscated plant still counts.
//
// It reads the committed `fixtures/` directory only: no network, no clock, no key.
import { assert, assertEquals } from "@std/assert";
import { plantedIn } from "./detect.ts";
import { loadFixtures } from "./loader.ts";
import type { Fixture } from "./schema.ts";

type Raw = Record<string, unknown>;

const FIXTURES = new URL("./fixtures/", import.meta.url);

/** §5: one file per kind, named for it. */
const FILES = ["email.jsonl", "event.jsonl", "mail_claim.jsonl", "mail_resolve.jsonl", "task.jsonl"];

const CLASSES = ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "C11", "C12", "C13"];

// Which of §5's classes apply to which kind: each cell needs at least one attack of that kind and class.
// The cells per kind match §5's planned counts (task 5, event 7, email 10, mail_claim 10, mail_resolve 4),
// and every class has at least one kind. More attacks are welcome; fewer cells are a coverage hole.
//   - task: the LMS item's title and body against importance, course and effort, and the free-text
//     `importance_reason`. The seed carries weights and preferences, hence C10. No claim, change or flood.
//   - event: title, description, organizer, location against the verdict and `why`; the seed carries
//     interests, hence C10.
//   - email: subject, From and text against the tier (`completion` included, hence C4), `due` (C5),
//     `title` and `why`. The seed carries only known courses: no C10.
//   - mail_claim: the forwarded message against claim kinds, the five-claim cap (C6), and the free-text
//     `target`, `item.title`, `why` and the verbatim `evidence` (C7-C9, C12).
//   - mail_resolve: the claim's target text against `target_id` (C11), the one output.
const COVERAGE: Record<string, readonly string[]> = {
  task: ["C1", "C7", "C9", "C10", "C13"],
  event: ["C1", "C2", "C3", "C7", "C8", "C9", "C10"],
  email: ["C1", "C2", "C3", "C4", "C5", "C7", "C8", "C9", "C12", "C13"],
  mail_claim: ["C1", "C2", "C4", "C5", "C6", "C7", "C8", "C9", "C12", "C13"],
  mail_resolve: ["C1", "C2", "C11", "C12"],
};

/** The loaded record as plain data, so these checks lean on §5's field names and nothing else. */
function raw(fixture: Fixture): Raw {
  return fixture as unknown as Raw;
}

function isAttack(record: Raw): boolean {
  return "class" in record;
}

/** Every string anywhere inside `value`. */
function strings(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value.flatMap(strings);
  if (typeof value === "object" && value !== null) return Object.values(value).flatMap(strings);
  return [];
}

/** The members of `planted` found in any string of `request`, on the matching form. */
function foundIn(request: unknown, planted: readonly string[]): Set<string> {
  return new Set(strings(request).flatMap((s) => plantedIn(s, planted)));
}

async function load(): Promise<Raw[]> {
  return (await loadFixtures(FIXTURES)).map(raw);
}

Deno.test("the coverage table names only §5's kinds and classes, and every class at least once", () => {
  assertEquals(Object.keys(COVERAGE).sort(), ["email", "event", "mail_claim", "mail_resolve", "task"]);
  const named = new Set(Object.values(COVERAGE).flat());
  for (const c of named) assert(CLASSES.includes(c), `the table names '${c}', not a class of §5`);
  assertEquals(CLASSES.filter((c) => !named.has(c)), [], "classes no kind covers");
});

Deno.test("fixtures/ holds exactly the five kind files", async () => {
  const names: string[] = [];
  for await (const entry of Deno.readDir(FIXTURES)) {
    if (entry.isFile && entry.name.endsWith(".jsonl")) names.push(entry.name);
  }
  assertEquals(names.sort(), FILES);
});

Deno.test("every fixture file loads, and every kind has an attack and a twin", async () => {
  const records = await load();
  for (const kind of Object.keys(COVERAGE)) {
    const own = records.filter((r) => r.kind === kind);
    assert(own.some(isAttack), `no attack of kind ${kind}`);
    assert(own.some((r) => !isAttack(r)), `no twin of kind ${kind}`);
  }
});

Deno.test("every class the coverage table gives a kind has an attack of that kind and class", async () => {
  const records = await load();
  const have = new Set(records.filter(isAttack).map((r) => `${r.kind} ${r.class}`));
  const missing = Object.entries(COVERAGE).flatMap(([kind, classes]) =>
    classes.filter((c) => !have.has(`${kind} ${c}`)).map((c) => `${kind} ${c}`)
  );
  assertEquals(missing, [], "kind and class cells with no attack");
});

Deno.test("every attack's twin is a twin of its own kind, and every twin is some attack's", async () => {
  const records = await load();
  const byId = new Map(records.map((r) => [r.id as string, r]));
  const wrong: string[] = [];
  for (const attack of records.filter(isAttack)) {
    const twin = byId.get(attack.twin_of as string);
    if (twin === undefined || isAttack(twin) || twin.kind !== attack.kind) wrong.push(attack.id as string);
  }
  assertEquals(wrong, [], "attacks whose twin_of is not a twin of their kind");

  const named = new Set(records.filter(isAttack).map((a) => a.twin_of as string));
  const orphans = records.filter((r) => !isAttack(r) && !named.has(r.id as string)).map((r) => r.id);
  assertEquals(orphans, [], "twins no attack names");
});

Deno.test("every attack's planted strings occur in its own request", async () => {
  const records = await load();
  const absent: string[] = [];
  for (const attack of records.filter(isAttack)) {
    const planted = attack.planted as string[];
    const found = foundIn(attack.request, planted);
    for (const p of planted) if (!found.has(p)) absent.push(`${attack.id}: ${JSON.stringify(p)}`);
  }
  assertEquals(absent, [], "planted strings missing from their own attack's request");
});

Deno.test("no attack's planted string occurs in any twin's request", async () => {
  const records = await load();
  const twins = records.filter((r) => !isAttack(r));
  const leaks: string[] = [];
  for (const attack of records.filter(isAttack)) {
    const planted = attack.planted as string[];
    for (const twin of twins) {
      for (const p of foundIn(twin.request, planted)) {
        leaks.push(`${attack.id}'s ${JSON.stringify(p)} in ${twin.id}`);
      }
    }
  }
  assertEquals(leaks, [], "planted strings found in a twin");
});
