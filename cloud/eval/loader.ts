// Reads the eval seed as a list of validated, scrubbed records.
//
// Empty by decision (ruling R-C2-E12, 2026-09-14): `cloud/eval/seed/` carries no `*.jsonl` file at
// merge, and fills later from consented corrections (a later stream's opt-in) — an absent or empty
// directory is exactly that state, and `loadSeed` returns `[]` for it rather than erroring.
//
// An invalid line is different: the seed is committed to this repository, so a line that fails
// either check is a bug in whatever wrote it, and `loadSeed` throws naming the file and the line
// rather than silently dropping it — a dropped case is a hole in the eval suite nobody sees.
import { scrubViolations, type SeedRecord, validateSeedRecord } from "./schema.ts";

export async function loadSeed(dir: URL = new URL("./seed/", import.meta.url)): Promise<SeedRecord[]> {
  const names: string[] = [];
  try {
    for await (const entry of Deno.readDir(dir)) {
      if (entry.isFile && entry.name.endsWith(".jsonl")) names.push(entry.name);
    }
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) return [];
    throw e;
  }
  names.sort((a, b) => a.localeCompare(b));

  const records: SeedRecord[] = [];
  for (const name of names) {
    const text = await Deno.readTextFile(new URL(name, dir));
    const lines = text.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (line.trim() === "") continue;
      const lineNo = i + 1;

      let parsed: unknown;
      try {
        parsed = JSON.parse(line);
      } catch (e) {
        throw new Error(
          `${name}:${lineNo}: not valid JSON (${e instanceof Error ? e.message : String(e)})`,
        );
      }

      const shapeViolations = validateSeedRecord(parsed);
      if (shapeViolations.length > 0) {
        throw new Error(`${name}:${lineNo}: ${shapeViolations.join("; ")}`);
      }
      const record = parsed as SeedRecord;

      const scrub = scrubViolations(record);
      if (scrub.length > 0) {
        throw new Error(`${name}:${lineNo}: ${scrub.join("; ")}`);
      }

      records.push(record);
    }
  }
  return records;
}
