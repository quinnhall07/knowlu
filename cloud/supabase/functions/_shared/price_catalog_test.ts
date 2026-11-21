/**
 * The price catalog's own rules (v0.4.1 note D13, plan T1a).
 *
 * **Only `the_catalog_sells_two_plans_yearly_first` names the selling amounts**, so a price change
 * edits one test, on purpose and in view (note D16 step 2). Every other test here builds its own
 * entries from a table in the test and never reads the real `SELLING`, except the terms tie (T5),
 * which reads the real catalog against the published terms page and names no amount of its own.
 */
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import { inMoveWindow } from "../billing-jobs/handler.ts";
import {
  CADENCE,
  type CatalogDef,
  catalogProblems,
  type Entry,
  figuresFor,
  KEPT,
  type KeptEntry,
  lookupPrice,
  money,
  MOVE_NOTICE_MAX_DAYS,
  MOVE_NOTICE_MIN_DAYS,
  offer,
  priceText,
  priceVariable,
  resolveCatalog,
  SELLING,
  TERMS_VERSION,
  unknownPriceText,
  WORDS,
} from "./price_catalog.ts";

/** The repository root, from `cloud/supabase/functions/_shared/`. */
const ROOT = new URL("../../../../", import.meta.url);

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

// ---------------------------------------------------------------------------------------------
// The terms tie (plan T5): the published terms page states the catalog's prices, its version and
// its notice bounds, and the app records the same version.

const DAY = 24 * 60 * 60 * 1000;

/**
 * The days ahead a price-change notice can go out, from the catalog's bounds on N and
 * `billing-jobs`' own window: the earliest is N = `MOVE_NOTICE_MIN_DAYS` at exactly N days; the
 * latest is under `max` days, the first whole day past the window at N = `MOVE_NOTICE_MAX_DAYS`.
 */
function noticeBounds(): { min: number; max: number } {
  let max = MOVE_NOTICE_MAX_DAYS;
  while (max < 366 && inMoveWindow(max * DAY, 0, MOVE_NOTICE_MAX_DAYS)) max++;
  return { min: MOVE_NOTICE_MIN_DAYS, max };
}

const FIGURE = /\$\d[\d,]*(?:\.\d+)?/g;
/** A dollar sign the figure scan cannot read: an entity, or `$` not followed by a digit. */
const OTHER_DOLLAR = /&#0*36;|&#x0*24;|&dollar;|\$(?!\d)/gi;

/** The markup with its comments, scripts and styles gone: what can render. */
function withoutComments(html: string): string {
  return html.replace(/<!--[\s\S]*?(?:-->|$)/g, " ").replace(
    /<(script|style)\b[\s\S]*?(?:<\/\1\s*>|$)/gi,
    " ",
  );
}

/** What a reader sees: no comments, no tags (so no attributes), whitespace collapsed. */
function visibleText(html: string): string {
  return withoutComments(html).replace(/<[^>]*>/g, " ").replace(/\s+/g, " ");
}

/**
 * What a terms page fails to say for a catalog, one sentence each; `[]` when it says it all.
 *
 * Two readings (review M1, M2). The **raw** page, comments and attributes included, is scanned for
 * anything the catalog does not write: a `$` figure no entry gives, or a dollar sign written any
 * other way, which the scan cannot read. **What a reader sees** is where every presence is checked:
 * the selling figures, `Effective <v>.`, `These terms are version <v>`, and the notice bounds in the
 * price-change paragraph. The version line's markup, `version <strong><v></strong>`, is checked on
 * the page with its comments removed, so it is the rendered markup and never a comment's.
 */
function termsProblems(
  page: string,
  c: { termsVersion: string; selling: readonly Entry[]; kept: readonly KeptEntry[] },
  notice: { min: number; max: number },
): string[] {
  const out: string[] = [];
  const written = new Set([...c.selling, ...c.kept].flatMap(figuresFor));
  for (const f of page.match(FIGURE) ?? []) {
    if (!written.has(f)) out.push(`the page states ${f}, which the catalog writes for no entry`);
  }
  for (const m of page.matchAll(OTHER_DOLLAR)) {
    const around = page.slice(m.index, m.index + 12).replace(/\s+/g, " ");
    out.push(`the page has a dollar figure written another way, which the check cannot read: "${around}"`);
  }
  const rendered = withoutComments(page);
  const seen = visibleText(page);
  const stated: string[] = seen.match(FIGURE) ?? [];
  for (const e of c.selling) {
    for (const f of figuresFor(e)) {
      if (!stated.includes(f)) {
        out.push(`the page lacks ${f}, which the catalog writes for ${e.key} ${e.cents}`);
      }
    }
  }
  const v = c.termsVersion;
  for (const s of [`Effective ${v}.`, `These terms are version ${v}`]) {
    if (!seen.includes(s)) out.push(`the page lacks "${s}" where a reader sees it`);
  }
  const marked = `These terms are version <strong>${v}</strong>`;
  if (!rendered.includes(marked)) out.push(`the page lacks "${marked}" outside a comment`);
  const bounds = `between ${notice.min} and ${notice.max} days ahead`;
  const paragraph = rendered.split("</p>").find((p) => visibleText(p).includes("If we ever change a price"));
  if (paragraph === undefined) out.push("the page has no price-change paragraph");
  else if (!visibleText(paragraph).includes(bounds)) out.push(`the price-change paragraph lacks "${bounds}"`);
  return out;
}

Deno.test("the_terms_state_the_catalogs_prices", async () => {
  // The bounds, from the catalog and the window that uses them: every N the catalog allows sends
  // its notice inside them, and they are tight.
  const notice = noticeBounds();
  assertEquals(notice.min, MOVE_NOTICE_MIN_DAYS);
  for (let n = MOVE_NOTICE_MIN_DAYS; n <= MOVE_NOTICE_MAX_DAYS; n++) {
    assert(!inMoveWindow(notice.min * DAY - 1, 0, n), `N = ${n}: nothing earlier than ${notice.min} days`);
    assert(!inMoveWindow(notice.max * DAY, 0, n), `N = ${n}: nothing ${notice.max} days or more ahead`);
  }
  assert(inMoveWindow(notice.min * DAY, 0, MOVE_NOTICE_MIN_DAYS));
  assert(inMoveWindow(notice.max * DAY - 1, 0, MOVE_NOTICE_MAX_DAYS));

  // The check itself, on text the test supplies, against a table of its own.
  const table = {
    termsVersion: "2030-01-01",
    selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1299 }] as Entry[],
    kept: [] as KeptEntry[],
  };
  const page = (
    prices: string,
    o: { version?: string; notice?: string; noticeElsewhere?: boolean } = {},
  ) => {
    const v = o.version ?? "2030-01-01";
    const n = o.notice ?? "between 7 and 30 days ahead";
    return [
      `<p class="date">Effective ${v}. These terms are version <strong>${v}</strong>.</p>`,
      `<p><strong>Two plans. ${prices}</strong></p>`,
      o.noticeElsewhere
        ? `<p>If we ever change a price, we email you first.</p><p>Notices go ${n}.</p>`
        : `<p>If we ever change a price, we email you before the change takes effect, ${n}.</p>`,
    ].join("\n");
  };
  const bounds = { min: 7, max: 30 };
  const good = "The yearly plan: $6.49 every 4 weeks, billed $84.37 once a year. Or $12.99 every 4 weeks.";
  assertEquals(termsProblems(page(good), table, bounds), []);
  const p650 = termsProblems(page(good.replace("$6.49", "$6.50")), table, bounds);
  assert(p650.some((p) => p.includes("states $6.50")), JSON.stringify(p650));
  const no8437 = termsProblems(page(good.replace("billed $84.37 ", "billed ")), table, bounds);
  assertEquals(no8437, ["the page lacks $84.37, which the catalog writes for yearly 8437"]);
  assertNotEquals(termsProblems(page(good, { version: "2030-01-02" }), table, bounds), []);
  assertNotEquals(termsProblems(page(good, { notice: "between 7 and 31 days ahead" }), table, bounds), []);
  assertNotEquals(termsProblems(page(good, { noticeElsewhere: true }), table, bounds), []);
  // (review M1, M2) What a reader cannot see does not count as stated, and a figure written any
  // other way than `$<digit>` is a problem: each case names the problem it must raise.
  const hidden: [string, string, string][] = [
    [
      "$84.37 only in a comment",
      page(good.replace("billed $84.37 ", "billed <!-- $84.37 --> ")),
      "lacks $84.37",
    ],
    [
      "$84.37 only in a meta attribute",
      `<meta name="description" content="$84.37 a year">\n${page(good.replace("billed $84.37 ", "billed "))}`,
      "lacks $84.37",
    ],
    [
      "the Effective line only in a comment",
      page(good).replace("Effective 2030-01-01.", "<!-- Effective 2030-01-01. -->Effective 2030-01-09."),
      'lacks "Effective 2030-01-01."',
    ],
    [
      "the version line only in a comment",
      page(good).replace(
        "These terms are version <strong>2030-01-01</strong>",
        "<!-- These terms are version <strong>2030-01-01</strong> -->",
      ),
      "These terms are version",
    ],
    ["a stray figure in a comment", `${page(good)}\n<!-- $9.99 -->`, "states $9.99"],
    ["a stray figure in an attribute", `<a title="$9.99">x</a>\n${page(good)}`, "states $9.99"],
  ];
  for (const s of ["&#36;9.99", "&#x24;9.99", "&dollar;9.99", "$ 9.99", "$.99", "$\n9.99"]) {
    hidden.push([
      `a figure written as ${JSON.stringify(s)}`,
      page(`${good} Or ${s}.`),
      "written another way",
    ]);
  }
  const missed: string[] = [];
  for (const [name, text, want] of hidden) {
    const problems = termsProblems(text, table, bounds);
    if (!problems.some((p) => p.includes(want))) missed.push(`${name}: ${JSON.stringify(problems)}`);
  }
  assertEquals(missed, [], "every hidden or respelled case raises its problem");

  // The published page and the real catalog.
  const terms = await Deno.readTextFile(new URL("site/terms.html", ROOT));
  assertEquals(
    termsProblems(terms, { termsVersion: TERMS_VERSION, selling: SELLING, kept: KEPT }, notice),
    [],
    "site/terms.html states the catalog's prices, version and notice bounds",
  );
});

Deno.test("the_catalogs_terms_version_is_the_apps", async () => {
  const account = await Deno.readTextFile(new URL("app/src/account.rs", ROOT));
  const lines = account.split("\n").map((l) => l.replace(/\r$/, ""))
    .filter((l) => l.startsWith("pub const TOS_VERSION: &str = "));
  assertEquals(lines.length, 1, "app/src/account.rs declares TOS_VERSION once");
  const m = lines[0].match(/^pub const TOS_VERSION: &str = "([^"]*)";$/);
  assert(m, `TOS_VERSION is a quoted literal: ${lines[0]}`);
  assertEquals(TERMS_VERSION, m[1], "the catalog's TERMS_VERSION is the app's TOS_VERSION");
});
