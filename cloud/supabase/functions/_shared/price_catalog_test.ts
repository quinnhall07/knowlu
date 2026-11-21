/**
 * The price catalog's own rules (v0.4.1 note D13, plan T1a).
 *
 * **Only `the_catalog_sells_two_plans_yearly_first` names the selling amounts**, so a price change
 * edits one test, on purpose and in view (note D16 step 2). Every other test here builds its own
 * entries from a table in the test and never reads the real `SELLING`.
 */
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import {
  CADENCE,
  type CatalogDef,
  catalogProblems,
  figuresFor,
  KEPT,
  type KeptEntry,
  lookupPrice,
  money,
  offer,
  priceText,
  priceVariable,
  resolveCatalog,
  SELLING,
  TERMS_VERSION,
  unknownPriceText,
  WORDS,
} from "./price_catalog.ts";

/** A catalog built in the test: the same shape and words as the real one, its own amounts. */
function def(over: Partial<CatalogDef> = {}): CatalogDef {
  return {
    termsVersion: "2030-01-01",
    selling: [{ key: "yearly", cents: 9087 }, { key: "four_weekly", cents: 1399 }],
    kept: [],
    words: WORDS,
    ...over,
  };
}

Deno.test("the_catalog_sells_two_plans_yearly_first", async () => {
  assertEquals(SELLING.map((e) => [e.key, e.cents]), [["yearly", 8437], ["four_weekly", 1299]]);
  assertEquals(KEPT.length, 0, "v0.4.1 keeps no old price (ruling 6)");
  assert(/^\d{4}-\d{2}-\d{2}$/.test(TERMS_VERSION), TERMS_VERSION);
  // Quoted, on one line, as a literal: the app's terms tie (D11) reads this file as text.
  const text = await Deno.readTextFile(new URL("./price_catalog.ts", import.meta.url));
  const m = text.match(/^export const TERMS_VERSION = "(\d{4}-\d{2}-\d{2})";$/m);
  assert(m, "TERMS_VERSION must be a quoted YYYY-MM-DD literal on its own line");
  assertEquals(m[1], TERMS_VERSION);
  assertEquals(
    catalogProblems({ termsVersion: TERMS_VERSION, selling: SELLING, kept: KEPT, words: WORDS }),
    [],
  );
});

Deno.test("every_entry_keeps_its_keys_cadence", () => {
  assertEquals(CADENCE.four_weekly, { interval: "week", count: 4 });
  assertEquals(CADENCE.yearly, { interval: "year", count: 1 });
  const resolved = resolveCatalog(
    def({
      kept: [{ key: "four_weekly", cents: 1299, rule: "stay" }, { key: "yearly", cents: 8437, rule: "stay" }],
    }),
    () => undefined,
  );
  for (const e of [...resolved.selling, ...resolved.kept]) {
    assertEquals({ interval: e.interval, count: e.count }, CADENCE[e.key], `${e.key} ${e.cents}`);
  }
  // The real lists too, whatever they hold.
  const real = resolveCatalog(
    { termsVersion: TERMS_VERSION, selling: SELLING, kept: KEPT, words: WORDS },
    () => undefined,
  );
  for (const e of [...real.selling, ...real.kept]) {
    assertEquals({ interval: e.interval, count: e.count }, CADENCE[e.key], `${e.key} ${e.cents}`);
  }
  // No key and cents twice, across selling and kept.
  assertEquals(catalogProblems(def()), []);
  assertNotEquals(catalogProblems(def({ kept: [{ key: "four_weekly", cents: 1399, rule: "stay" }] })), []);
  assertNotEquals(
    catalogProblems(def({ selling: [{ key: "yearly", cents: 9087 }, { key: "yearly", cents: 9087 }] })),
    [],
  );
  // Exactly the two selling keys, each once.
  assertNotEquals(catalogProblems(def({ selling: [{ key: "yearly", cents: 9087 }] })), []);
  assertNotEquals(
    catalogProblems(def({ selling: [{ key: "yearly", cents: 9087 }, { key: "yearly", cents: 9100 }] })),
    [],
  );
});

Deno.test("the_yearly_amount_is_thirteen_whole_cent_charges", () => {
  assertEquals(8437 / 13, 649);
  assertEquals(
    catalogProblems(def({ selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1299 }] })),
    [],
  );
  const bad = catalogProblems(
    def({ selling: [{ key: "yearly", cents: 8999 }, { key: "four_weekly", cents: 1299 }] }),
  );
  assertEquals(bad.length, 1, JSON.stringify(bad));
  assert(bad[0].includes("13"), bad[0]);
  // A kept yearly price holds to it as well.
  assertNotEquals(catalogProblems(def({ kept: [{ key: "yearly", cents: 8999, rule: "stay" }] })), []);
});

Deno.test("a_move_entry_notices_inside_seven_to_thirty_days", () => {
  const withMove = (n: number) =>
    catalogProblems(def({ kept: [{ key: "four_weekly", cents: 1299, rule: "move", noticeDays: n }] }));
  for (let n = 7; n <= 24; n++) assertEquals(withMove(n), [], `N = ${n}`);
  for (const n of [6, 25, 0, -1, 7.5]) assertNotEquals(withMove(n), [], `N = ${n}`);
});

Deno.test("the_variable_names_are_derived", () => {
  assertEquals(priceVariable({ key: "four_weekly", cents: 1299 }), "STRIPE_PRICE_FOUR_WEEKLY_1299");
  assertEquals(priceVariable({ key: "yearly", cents: 8437 }), "STRIPE_PRICE_YEARLY_8437");
  const table = def({ selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1299 }] });
  const env: Record<string, string> = { STRIPE_PRICE_YEARLY_8437: "price_test_yearly" };
  const asked: string[] = [];
  const c = resolveCatalog(table, (name) => {
    asked.push(name);
    return env[name];
  });
  assertEquals(asked.sort(), ["STRIPE_PRICE_FOUR_WEEKLY_1299", "STRIPE_PRICE_YEARLY_8437"]);
  assertEquals(c.selling.map((e) => [e.key, e.variable, e.priceId]), [
    ["yearly", "STRIPE_PRICE_YEARLY_8437", "price_test_yearly"],
    ["four_weekly", "STRIPE_PRICE_FOUR_WEEKLY_1299", null],
  ]);
  assertEquals(c.missing, ["STRIPE_PRICE_FOUR_WEEKLY_1299"]);
  assertEquals(lookupPrice(c, "price_test_yearly")?.key, "yearly");
  // The unset entry matches no id: not the empty string, not null's text, not anything.
  assertEquals(lookupPrice(c, ""), null);
  assertEquals(lookupPrice(c, "null"), null);
  assertEquals(lookupPrice(c, "undefined"), null);
  assertEquals(lookupPrice(c, "price_unknown"), null);
  // A blank value is unset too.
  const blank = resolveCatalog(table, (name) => (name === "STRIPE_PRICE_YEARLY_8437" ? "  " : undefined));
  assertEquals(blank.selling[0].priceId, null);
  assertEquals(lookupPrice(blank, "  "), null);
  // Kept entries resolve the same way and are found by id.
  const kept: KeptEntry[] = [{ key: "four_weekly", cents: 999, rule: "stay" }];
  const k = resolveCatalog(
    def({ kept }),
    (n) => (n === "STRIPE_PRICE_FOUR_WEEKLY_999" ? "price_old" : undefined),
  );
  assertEquals(lookupPrice(k, "price_old")?.cents, 999);
  assertEquals(lookupPrice(k, "price_old")?.key, "four_weekly");
});

Deno.test("the_formatter_writes_every_4_weeks_and_once_a_year", () => {
  assertEquals(priceText({ key: "four_weekly", cents: 1299 }), "$12.99 every 4 weeks");
  assertEquals(priceText({ key: "yearly", cents: 8437 }), "$84.37 once a year");
  assertEquals(priceText({ key: "four_weekly", cents: 1399 }), "$13.99 every 4 weeks");
  assertEquals(money(500), "$5.00");
  assertEquals(money(1), "$0.01");
  // The P9 lines, word for word (note §1), and the line below both.
  const o = offer(def({ selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1299 }] }));
  assertEquals(o, {
    terms_version: "2030-01-01",
    plans: [
      {
        plan: "yearly",
        amount_cents: 8437,
        lines: [
          "$6.49 every 4 weeks, billed $84.37 once a year. Renews yearly.",
          "30-day money-back guarantee on each yearly charge.",
        ],
      },
      { plan: "four_weekly", amount_cents: 1299, lines: ["$12.99 every 4 weeks."] },
    ],
    note: "Plus tax where it applies.",
  });
  // The tax phrase and the money-back words, as the emails and Stripe's page use them.
  assertEquals(WORDS.tax, "plus tax where it applies");
  assertEquals(WORDS.moneyBack, "30-day money-back guarantee on each yearly charge");
  // An invented entry at 1399.
  assertEquals(offer(def()).plans[1].lines, ["$13.99 every 4 weeks."]);
  // Generic cadences, for a price the catalog lacks (D6).
  const usd = (cents: number, interval: string, count: number) =>
    unknownPriceText({ cents, currency: "usd", interval, count });
  assertEquals(usd(1500, "month", 2), "$15.00 every 2 months");
  assertEquals(usd(300, "week", 1), "$3.00 every week");
  assertEquals(usd(9900, "year", 1), "$99.00 every year");
  assertEquals(usd(1299, "week", 4), "$12.99 every 4 weeks");
  assertEquals(usd(1299, "fortnight", 1), null, "an interval nobody knows is never guessed");
  assertEquals(usd(1299, "week", 0), null);
  assertEquals(usd(12.5, "week", 4), null);
  assertEquals(unknownPriceText({ cents: 1299, currency: "eur", interval: "week", count: 4 }), null);
  // (r5: I1) the figures it writes for an entry: the list the terms tie (T5) reads.
  assertEquals(figuresFor({ key: "four_weekly", cents: 1299 }), ["$12.99"]);
  assertEquals(figuresFor({ key: "yearly", cents: 8437 }), ["$84.37", "$6.49"]);
  for (const e of [{ key: "four_weekly", cents: 1299 }, { key: "yearly", cents: 8437 }] as const) {
    assert(!figuresFor(e).includes("$6.50"), `${e.key}: nothing gives $6.50`);
  }
});
