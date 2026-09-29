// TDD, per CLAUDE.md: written against `calibration_cli.ts`'s intended shape first.
//
// `Deno.makeTempDir({ dir: here, prefix: ".tmp-" })`, landed inside `cloud/eval/` — the same
// pattern `scrub_test.ts` uses — so this stays inside the `--allow-write=cloud/eval` grant
// `lane-rules.md`'s exact `deno test` command carries, rather than reaching for an unportable OS
// temp root. `.gitignore`'s `.tmp-*` keeps the directories this creates out of git.
import { assert, assertEquals } from "@std/assert";
import { main, readAndReport } from "./calibration_cli.ts";
import type { CalibrationRow } from "./calibration.ts";

function fromFileUrl(url: URL): string {
  const decoded = decodeURIComponent(url.pathname);
  return /^\/[A-Za-z]:\//.test(decoded) ? decoded.slice(1) : decoded;
}

async function withTempFile(
  name: string,
  contents: string,
  fn: (path: string) => Promise<void>,
): Promise<void> {
  const here = fromFileUrl(new URL("./", import.meta.url));
  const dir = await Deno.makeTempDir({ dir: here, prefix: ".tmp-" });
  const path = `${dir}/${name}`;
  try {
    await Deno.writeTextFile(path, contents);
    await fn(path);
  } finally {
    await Deno.remove(dir, { recursive: true });
  }
}

function row(overrides: Partial<CalibrationRow> = {}): CalibrationRow {
  return {
    kind: "event",
    prompt_version: "event-1",
    prompt_hash: "hash-a",
    confidence: 0.9,
    verdict: "obligation",
    wrong: false,
    ...overrides,
  };
}

function captureConsole(): { log: string[]; error: string[]; restore: () => void } {
  const originalLog = console.log;
  const originalError = console.error;
  const log: string[] = [];
  const error: string[] = [];
  console.log = (...args: unknown[]) => {
    log.push(args.map(String).join(" "));
  };
  console.error = (...args: unknown[]) => {
    error.push(args.map(String).join(" "));
  };
  return {
    log,
    error,
    restore: () => {
      console.log = originalLog;
      console.error = originalError;
    },
  };
}

Deno.test("readAndReport on a JSONL fixture returns one report per group", async () => {
  await withTempFile(
    "rows.jsonl",
    [
      row({ confidence: 0.6, wrong: true }),
      row({ confidence: 0.7, wrong: false }),
      row({ confidence: 0.8, wrong: false }),
      row({ confidence: 0.9, wrong: false }),
    ].map((r) => JSON.stringify(r)).join("\n") + "\n",
    async (path) => {
      const reports = await readAndReport(path);
      assertEquals(reports.length, 1);
      assertEquals(reports[0].kind, "event");
      assertEquals(reports[0].n, 4);
    },
  );
});

Deno.test("readAndReport on an empty file returns []", async () => {
  await withTempFile("empty.jsonl", "", async (path) => {
    assertEquals(await readAndReport(path), []);
  });
});

Deno.test("readAndReport on a malformed row throws naming the file and the line", async () => {
  await withTempFile("bad.jsonl", "not json\n", async (path) => {
    let threw: unknown;
    try {
      await readAndReport(path);
    } catch (e) {
      threw = e;
    }
    assert(threw instanceof Error && threw.message.includes(":1:"), String(threw));
  });
});

Deno.test("main() with no arguments prints usage and exits 2", async () => {
  const capture = captureConsole();
  try {
    const code = await main([]);
    assertEquals(code, 2);
    assert(capture.error.some((line) => line.includes("usage:")));
  } finally {
    capture.restore();
  }
});

Deno.test("main() on a missing file prints an error naming the path and exits 1", async () => {
  const capture = captureConsole();
  try {
    const code = await main(["does-not-exist.jsonl"]);
    assertEquals(code, 1);
    assert(capture.error.some((line) => line.includes("calibration_cli:")));
  } finally {
    capture.restore();
  }
});

Deno.test("main() on a valid fixture prints a JSON report and exits 0", async () => {
  await withTempFile(
    "rows.jsonl",
    [
      row({ confidence: 0.6, wrong: true }),
      row({ confidence: 0.7, wrong: false }),
      row({ confidence: 0.8, wrong: false }),
    ].map((r) => JSON.stringify(r)).join("\n") + "\n",
    async (path) => {
      const capture = captureConsole();
      try {
        const code = await main([path]);
        assertEquals(code, 0);
        assertEquals(capture.log.length, 1);
        const parsed = JSON.parse(capture.log[0]);
        assertEquals(parsed.length, 1);
        assertEquals(parsed[0].kind, "event");
      } finally {
        capture.restore();
      }
    },
  );
});

// F7 (engine follow-ups plan, item (b)): the device now reports id-bearing label rows through
// `/telemetry`. A static pin on the query text, the way `app/tests/telemetry.rs` pins `ACTIONS`:
// an abstention is no calibration point, an approval is no correction, an email label never reaches
// the cross-account aggregate (Gmail Limited Use), and a correction marks only its own account's
// judgment wrong.
Deno.test("the calibration query excludes unsure verdicts, approved decisions and email labels, and joins on the account", async () => {
  const sql = (await Deno.readTextFile(fromFileUrl(new URL("./calibration_query.sql", import.meta.url))))
    .replace(/\s+/g, " ");
  for (
    const clause of [
      "and coalesce(j.fields ->> 'verdict', '') <> 'unsure'",
      "and c.account_id = j.account_id",
      "and not (c.field in ('verdict','decision') and c.judgment_kind = 'email')",
      "and not (c.field = 'decision' and c.theirs = 'approved')",
    ]
  ) {
    assert(sql.includes(clause), `calibration_query.sql lacks: ${clause}`);
  }
});
