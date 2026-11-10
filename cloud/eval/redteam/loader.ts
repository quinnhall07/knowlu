// Reads the red-team fixtures (plan `2026-11-09-prompt-injection-suite`, §5) as a list of checked records.
//
// The fixtures are committed to a public repository, so a line that fails a check is a bug in whatever
// wrote it, and `loadFixtures` throws `<file>:<line>: <why>` rather than dropping it, as `cloud/eval/
// loader.ts` does for the seed. Unlike the seed, an absent or empty directory throws too: the red-team suite
// is never empty by design, and an empty suite would pass for nothing.
//
// Per file: strict UTF-8 with no byte-order mark. Per line: LF only, valid JSON with no key repeated at any
// depth (`JSON.parse` would hide all but the last), `validateFixture`, `hygieneViolations`, a file named for
// the record's kind, an id
// no earlier line holds, and canaries no earlier record holds (nor one a prefix of the other, which an echo
// check would confuse). Over the set: every attack's `twin_of` names a twin of its own kind with the same
// `heuristics_seed` (and, for mail_resolve, the same candidates), since the attacker controls the item and
// never the student's seed or notes; and every twin is some attack's. Pure apart from reading `dir`.
import {
  type AttackFixture,
  canariesIn,
  type Fixture,
  FIXTURE_KINDS,
  hygieneViolations,
  isAttack,
  validateFixture,
} from "./schema.ts";

interface Located {
  fixture: Fixture;
  file: string;
  line: number;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** JSON with every object's keys sorted, so two equal values compare equal whatever their key order. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (isPlainObject(value)) {
    return `{${
      Object.keys(value).sort().map((k) => `${JSON.stringify(k)}:${canonical(value[k])}`).join(",")
    }}`;
  }
  return JSON.stringify(value);
}

/**
 * The first key an object in `line` repeats, at any depth, or null. `JSON.parse` keeps a repeated key's last
 * value, so an earlier one would sit in the public file unseen by every check. One pass over a line that has
 * already parsed: a string is skipped whole, and only a string in an object's key position is read as a key.
 */
function duplicateKey(line: string): string | null {
  const stack: Array<Set<string> | null> = [];
  let expectKey = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '"') {
      let j = i + 1;
      while (j < line.length && line[j] !== '"') j += line[j] === "\\" ? 2 : 1;
      const keys = stack[stack.length - 1];
      if (expectKey && keys) {
        const key = JSON.parse(line.slice(i, j + 1)) as string;
        if (keys.has(key)) return key;
        keys.add(key);
        expectKey = false;
      }
      i = j;
    } else if (c === "{") {
      stack.push(new Set());
      expectKey = true;
    } else if (c === "[") {
      stack.push(null);
      expectKey = false;
    } else if (c === "}" || c === "]") {
      stack.pop();
      expectKey = false;
    } else if (c === ",") {
      expectKey = stack[stack.length - 1] instanceof Set;
    }
  }
  return null;
}

function where(at: Located): string {
  return `${at.file}:${at.line}`;
}

export async function loadFixtures(dir: URL = new URL("./fixtures/", import.meta.url)): Promise<Fixture[]> {
  const names: string[] = [];
  try {
    for await (const entry of Deno.readDir(dir)) {
      if (entry.isFile && entry.name.endsWith(".jsonl")) names.push(entry.name);
    }
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) {
      throw new Error(`${dir.href}: no fixtures directory; an empty red-team suite never passes`);
    }
    throw e;
  }
  // Code-unit order, not `localeCompare`: the same on every operating system and locale, so the first error
  // reported is the same everywhere.
  names.sort();

  const loaded: Located[] = [];
  const byId = new Map<string, Located>();
  const canaries = new Map<string, Located>();

  for (const name of names) {
    const kind = name.slice(0, -".jsonl".length);
    if (!(FIXTURE_KINDS as readonly string[]).includes(kind)) {
      throw new Error(
        `${name}: not a fixture file; each is named for its kind (${FIXTURE_KINDS.join(", ")})`,
      );
    }
    // Decoded strictly and with any byte-order mark kept, so neither a mark nor a bad byte passes unnamed.
    let text: string;
    try {
      text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
        await Deno.readFile(new URL(name, dir)),
      );
    } catch (e) {
      if (e instanceof TypeError) throw new Error(`${name}: not valid UTF-8`);
      throw e;
    }
    if (text.charCodeAt(0) === 0xfeff) {
      throw new Error(`${name}:1: a byte-order mark; fixture files are UTF-8 without one`);
    }
    const lines = text.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const at = `${name}:${i + 1}`;
      if (line.includes("\r")) throw new Error(`${at}: a carriage return; fixture files are LF`);
      if (line.trim() === "") continue;

      let parsed: unknown;
      try {
        parsed = JSON.parse(line);
      } catch (e) {
        throw new Error(`${at}: not valid JSON (${e instanceof Error ? e.message : String(e)})`);
      }
      const repeated = duplicateKey(line);
      if (repeated !== null) throw new Error(`${at}: duplicate key ${JSON.stringify(repeated)}`);

      const shape = validateFixture(parsed);
      if (shape.length > 0) throw new Error(`${at}: ${shape.join("; ")}`);
      const fixture = parsed as Fixture;

      const hygiene = hygieneViolations(fixture);
      if (hygiene.length > 0) throw new Error(`${at}: ${hygiene.join("; ")}`);

      if (fixture.kind !== kind) {
        throw new Error(
          `${at}: kind '${fixture.kind}' in ${name}; a fixture sits in the file named for its kind`,
        );
      }

      const located: Located = { fixture, file: name, line: i + 1 };
      const earlier = byId.get(fixture.id);
      if (earlier !== undefined) {
        throw new Error(`${at}: id '${fixture.id}' repeats the id at ${where(earlier)}`);
      }
      byId.set(fixture.id, located);

      for (const canary of canariesIn(fixture)) {
        for (const [other, holder] of canaries) {
          if (other === canary) {
            throw new Error(
              `${at}: canary '${canary}' is also in '${holder.fixture.id}' (${where(holder)}); ` +
                "each canary belongs to one fixture",
            );
          }
          if (other.startsWith(canary) || canary.startsWith(other)) {
            throw new Error(
              `${at}: canary '${canary}' and '${other}' (${where(holder)}) are one a prefix of the other, ` +
                "so an echo of the longer would count as the shorter",
            );
          }
        }
        canaries.set(canary, located);
      }
      loaded.push(located);
    }
  }

  if (loaded.length === 0) {
    throw new Error(`${dir.href}: no fixture records; an empty red-team suite never passes`);
  }

  const named = new Set<string>();
  for (const at of loaded) {
    if (!isAttack(at.fixture)) continue;
    const attack: AttackFixture = at.fixture;
    named.add(attack.twin_of);
    const twin = byId.get(attack.twin_of);
    if (twin === undefined) {
      throw new Error(`${where(at)}: twin_of '${attack.twin_of}' names no fixture`);
    }
    if (isAttack(twin.fixture)) {
      throw new Error(
        `${where(at)}: twin_of '${attack.twin_of}' names an attack (${where(twin)}), not a twin`,
      );
    }
    if (twin.fixture.kind !== attack.kind) {
      throw new Error(
        `${
          where(at)
        }: twin_of '${attack.twin_of}' names a ${twin.fixture.kind} twin; a twin is of its attack's kind`,
      );
    }
    if (canonical(attack.request.heuristics_seed) !== canonical(twin.fixture.request.heuristics_seed)) {
      throw new Error(
        `${where(at)}: heuristics_seed differs from its twin's (${where(twin)}); the attacker controls the ` +
          "item, never the student's seed",
      );
    }
    if (
      attack.kind === "mail_resolve" &&
      canonical(attack.request.item.candidates) !== canonical(twin.fixture.request.item.candidates)
    ) {
      throw new Error(
        `${where(at)}: request.item.candidates differ from its twin's (${where(twin)}); the candidates are ` +
          "the student's notes, which the attacker does not write",
      );
    }
  }
  for (const at of loaded) {
    if (!isAttack(at.fixture) && !named.has(at.fixture.id)) {
      throw new Error(`${where(at)}: twin '${at.fixture.id}' is no attack's twin_of`);
    }
  }

  return loaded.map((at) => at.fixture);
}
