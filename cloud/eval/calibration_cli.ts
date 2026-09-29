// T6's CLI: read a JSON/JSONL file of `CalibrationRow`s, print `CalibrationGroupReport[]` as JSON.
// Everything that does real work lives in `calibration.ts`; this file is I/O and argument parsing
// only, so `readAndReport` (the one function that touches the filesystem) is exercised directly
// in tests against a fixture written under `cloud/eval/` — `run_eval.ts`'s own `main(args):
// Promise<number>` / `if (import.meta.main) Deno.exit(await main(Deno.args))` shape, reused here.
//
// **Parked, per the T6 brief.** There are no real corrections yet (`calibration_query.sql`'s own
// header), so this CLI has nothing of ours to run against today. It is written and tested on
// synthetic rows now so it is ready the day a real export exists — "do not report this task as
// late; report it as waiting on users."
import {
  buildCalibrationReport,
  type BuildReportOptions,
  type CalibrationGroupReport,
  parseCalibrationRows,
} from "./calibration.ts";

/** Reads `path`, parses it as `CalibrationRow`s (JSON array or JSONL — `parseCalibrationRows`
 * decides which), and returns one report per `(kind, prompt_version, prompt_hash)` group. Thrown
 * errors (a missing file, a malformed row) are the caller's to catch and report; this function
 * does not print anything, so it is safe to call from a test with `console.log` untouched. */
export async function readAndReport(
  path: string,
  options: BuildReportOptions = {},
): Promise<CalibrationGroupReport[]> {
  const text = await Deno.readTextFile(path);
  const rows = parseCalibrationRows(text, path);
  if (rows.length === 0) return [];
  return buildCalibrationReport(rows, options);
}

function usage(): string {
  return "usage: calibration_cli.ts <rows.json|rows.jsonl>\n" +
    "  Reads CalibrationRow[] (JSON array or JSONL, per calibration_query.sql's header) and\n" +
    "  prints one CalibrationGroupReport per (kind, prompt_version, prompt_hash) group as JSON.";
}

/** `0` on a printed report (even an empty one — an empty input file is not an error, it is "no
 * judgments yet", which is the expected state until real corrections exist); `2` on a usage error;
 * `1` on a read or parse failure, naming the file. */
export async function main(args: string[]): Promise<number> {
  const path = args[0];
  if (!path || args.includes("--help") || args.includes("-h")) {
    console.error(usage());
    return path ? 0 : 2;
  }
  try {
    const report = await readAndReport(path);
    console.log(JSON.stringify(report, null, 2));
    return 0;
  } catch (e) {
    console.error(`calibration_cli: ${e instanceof Error ? e.message : String(e)}`);
    return 1;
  }
}

if (import.meta.main) {
  Deno.exit(await main(Deno.args));
}
