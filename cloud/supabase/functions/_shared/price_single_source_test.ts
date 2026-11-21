/**
 * **One source for every price** (v0.4.1 note D13, §5): no amount, price-variable name or monthly
 * wording is written in the billing functions outside `_shared/price_catalog.ts`. Its own file, so
 * the task that adds it (T3) touches nothing T1a owns.
 *
 * It does not match bare cents (r5: m3): such a rule would fail on `fail(500, …)` the day a price is
 * $5.00. Test files are not read: they build invented catalogs on purpose.
 */
import { assert, assertEquals } from "@std/assert";

const FOLDERS = ["billing-checkout", "stripe-webhook", "billing-jobs", "billing-portal"];

const RULES: [string, RegExp][] = [
  ["a dollar figure", /\$\d/],
  ["a decimal money figure", /\b\d+\.\d{2}\b/],
  ["a price variable's name", /STRIPE_PRICE_/],
  ['"per month"', /per month/i],
  ['"a month"', /\ba month\b/i],
];

/** Every rule a text breaks, as `rule: the line`. */
function amountsWritten(text: string): string[] {
  const out: string[] = [];
  for (const line of text.split("\n")) {
    for (const [name, re] of RULES) if (re.test(line)) out.push(`${name}: ${line.trim()}`);
  }
  return out;
}

async function sources(): Promise<{ path: string; text: string }[]> {
  const out: { path: string; text: string }[] = [];
  for (const folder of FOLDERS) {
    const dir = new URL(`../${folder}/`, import.meta.url);
    for await (const entry of Deno.readDir(dir)) {
      if (!entry.isFile || !entry.name.endsWith(".ts") || entry.name.endsWith("_test.ts")) continue;
      out.push({ path: `${folder}/${entry.name}`, text: await Deno.readTextFile(new URL(entry.name, dir)) });
    }
  }
  return out;
}

Deno.test("no_amount_is_written_outside_the_catalog", async () => {
  // The rules themselves, on text the test supplies, so a rule that can never fire cannot pass.
  assertEquals(amountsWritten('return "$9.99 per month";').length, 3);
  assertEquals(amountsWritten("const price = 12.99;").length, 1);
  assertEquals(amountsWritten('env("STRIPE_PRICE_MONTHLY")').length, 1);
  assertEquals(amountsWritten("billed a month at a time").length, 1);
  assertEquals(amountsWritten("throw fail(500, `${x} is not set`); // 1299 cents, 13 charges").length, 0);

  const files = await sources();
  for (const folder of FOLDERS) {
    assert(files.some((f) => f.path.startsWith(`${folder}/`)), `${folder} has source files`);
  }
  const found = files.flatMap((f) => amountsWritten(f.text).map((p) => `${f.path}: ${p}`));
  assertEquals(found, [], "every amount, price variable and price wording lives in _shared/price_catalog.ts");
});
