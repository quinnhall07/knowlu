/**
 * **The price catalog** (v0.4.1 note D13): the one place in the billing code where a price amount,
 * the name of a price variable, or the terms version that states the prices is written. Every
 * billing function (`billing-checkout`, `stripe-webhook`, `billing-jobs`) reads it through its
 * `Deps` as a resolved catalog, so a test can hand it an invented one.
 *
 * Pure: it writes no file and calls nothing. The Stripe price ids are not here. Each entry's id is
 * read from an environment variable whose name the catalog derives from the entry
 * (`STRIPE_PRICE_` + the key in capitals + `_` + the cents), so a variable names one Stripe price
 * forever and a price change adds a variable rather than re-pointing one.
 *
 * A price change (note D16) edits `SELLING` (and moves the old entry to `KEPT` with its rule) in the
 * same reviewed commit as the terms page that states it; `price_catalog_test.ts`'s first test is the
 * one test that names the selling amounts.
 */

/** A plan key outlives its price (D1): a price change is a new entry under the same key. */
export type PlanKey = "four_weekly" | "yearly";
export type Interval = "day" | "week" | "month" | "year";

export interface Cadence {
  interval: Interval;
  count: number;
}

/** One price: its key and its amount in US cents. */
export interface Entry {
  key: PlanKey;
  cents: number;
}

/**
 * A price no longer sold that a subscriber may still pay (D14). `stay`: they keep it until they
 * cancel. `move`: `billing-jobs` emails them `noticeDays` before a renewal, then moves them to the
 * key's selling price for that renewal.
 */
export type KeptEntry =
  | { key: PlanKey; cents: number; rule: "stay" }
  | { key: PlanKey; cents: number; rule: "move"; noticeDays: number };

/** The words every surface uses beside a price, so none of them can drift from another. */
export interface Words {
  /** Lower case, no stop: it follows a price inside a sentence. */
  tax: string;
  /** No stop: the app's line adds one, Stripe's page adds the terms link. */
  moneyBack: string;
}

export interface CatalogDef {
  termsVersion: string;
  selling: readonly Entry[];
  kept: readonly KeptEntry[];
  words: Words;
}

export interface ResolvedEntry extends Entry {
  interval: Interval;
  count: number;
  /** The environment variable this entry's Stripe price id was read from. */
  variable: string;
  /** `null` when the variable is unset or blank: the entry then matches no id. */
  priceId: string | null;
}

export type ResolvedKept = ResolvedEntry & KeptEntry;

export interface ResolvedCatalog {
  termsVersion: string;
  words: Words;
  selling: ResolvedEntry[];
  kept: ResolvedKept[];
  /** The variables that were unset, by name, for a caller to log once. */
  missing: string[];
}

export interface OfferPlan {
  plan: PlanKey;
  amount_cents: number;
  lines: string[];
}

/** What `GET /billing-checkout` answers (D9), and what the app renders unchanged. */
export interface Offer {
  terms_version: string;
  plans: OfferPlan[];
  note: string;
}

// ---------------------------------------------------------------------------------------------
// The catalog itself.

/** The version of the terms page that states the selling prices. A quoted literal on one line: the
 * app's terms tie reads this file as text. */
export const TERMS_VERSION = "2026-11-21";

/** Fixed forever: a key's cadence never changes with its price (D13). */
export const CADENCE: Record<PlanKey, Cadence> = {
  four_weekly: { interval: "week", count: 4 },
  yearly: { interval: "year", count: 1 },
};

/** What is sold, in display order, Yearly first (ruling 2). */
export const SELLING: readonly Entry[] = [
  { key: "yearly", cents: 8437 },
  { key: "four_weekly", cents: 1299 },
];

/** Empty at v0.4.1: no subscriber pays anything else (ruling 6). */
export const KEPT: readonly KeptEntry[] = [];

export const WORDS: Words = {
  tax: "plus tax where it applies",
  moneyBack: "30-day money-back guarantee on each yearly charge",
};

export const CATALOG: CatalogDef = {
  termsVersion: TERMS_VERSION,
  selling: SELLING,
  kept: KEPT,
  words: WORDS,
};

// ---------------------------------------------------------------------------------------------
// The rules (checked by `price_catalog_test.ts`).

const KEYS: readonly PlanKey[] = ["yearly", "four_weekly"];

/** The terms promise a price-change notice 7 to 30 days ahead; a week-wide window from N days must
 * close before 30, so N runs 7 to 24 (D14). */
export const MOVE_NOTICE_MIN_DAYS = 7;
export const MOVE_NOTICE_MAX_DAYS = 24;

/** Every rule the catalog must keep, as one sentence per broken rule; `[]` when it keeps them all. */
export function catalogProblems(c: CatalogDef): string[] {
  const out: string[] = [];
  if (!/^\d{4}-\d{2}-\d{2}$/.test(c.termsVersion)) {
    out.push(`terms version ${c.termsVersion} is not YYYY-MM-DD`);
  }
  for (const k of KEYS) {
    const n = c.selling.filter((e) => e.key === k).length;
    if (n !== 1) out.push(`${k} is sold ${n} times; every key is sold exactly once`);
  }
  const seen = new Set<string>();
  for (const e of [...c.selling, ...c.kept]) {
    const id = `${e.key} ${e.cents}`;
    if (seen.has(id)) out.push(`${id} appears twice`);
    seen.add(id);
    if (!Number.isInteger(e.cents) || e.cents <= 0) out.push(`${id}: cents must be a positive whole number`);
    if (e.key === "yearly" && e.cents % 13 !== 0) {
      out.push(`${id}: a yearly amount must be 13 whole-cent charges every 4 weeks`);
    }
  }
  for (const e of c.kept) {
    if (e.rule !== "move") continue;
    const n = e.noticeDays;
    if (!Number.isInteger(n) || n < MOVE_NOTICE_MIN_DAYS || n > MOVE_NOTICE_MAX_DAYS) {
      out.push(
        `${e.key} ${e.cents}: move notice ${n} is outside ${MOVE_NOTICE_MIN_DAYS} to ${MOVE_NOTICE_MAX_DAYS} days`,
      );
    }
  }
  return out;
}

// ---------------------------------------------------------------------------------------------
// Resolving and looking up.

/** The environment variable that holds this entry's Stripe price id; computed, never written. */
export function priceVariable(e: Entry): string {
  return `STRIPE_PRICE_${e.key.toUpperCase()}_${e.cents}`;
}

/**
 * Reads every entry's price id through `get` (in a function, `Deno.env.get`). An unset or blank
 * variable leaves its entry with `priceId: null`, which matches no id, and is listed in `missing`.
 * Never throws: whether a missing selling id is fatal is the caller's to decide (checkout: yes, D4).
 */
export function resolveCatalog(c: CatalogDef, get: (name: string) => string | undefined): ResolvedCatalog {
  const missing: string[] = [];
  const resolve = <E extends Entry>(e: E): E & Omit<ResolvedEntry, keyof Entry> => {
    const variable = priceVariable(e);
    const raw = get(variable);
    const priceId = typeof raw === "string" && raw.trim() !== "" ? raw.trim() : null;
    if (priceId === null) missing.push(variable);
    return { ...e, ...CADENCE[e.key], variable, priceId };
  };
  return {
    termsVersion: c.termsVersion,
    words: c.words,
    selling: c.selling.map(resolve),
    kept: c.kept.map(resolve) as ResolvedKept[],
    missing,
  };
}

/** The selling or kept entry whose Stripe price id is `priceId`, or `null`. */
export function lookupPrice(c: ResolvedCatalog, priceId: string): ResolvedEntry | null {
  if (typeof priceId !== "string" || priceId === "") return null;
  return c.selling.find((e) => e.priceId !== null && e.priceId === priceId) ??
    c.kept.find((e) => e.priceId !== null && e.priceId === priceId) ?? null;
}

/** The selling entry for a key, if the catalog sells it. */
export function sellingFor(c: ResolvedCatalog, key: PlanKey): ResolvedEntry | undefined {
  return c.selling.find((e) => e.key === key);
}

// ---------------------------------------------------------------------------------------------
// The words: the one formatter every email, the offer and the terms tie use.

/** US cents as dollars: 1299 is "$12.99". */
export function money(cents: number): string {
  return `$${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}

/** A key's cadence words, which never change with its price. */
export function cadenceWords(key: PlanKey): string {
  return key === "yearly" ? "once a year" : "every 4 weeks";
}

/** "$12.99 every 4 weeks" or "$84.37 once a year". */
export function priceText(e: Entry): string {
  return `${money(e.cents)} ${cadenceWords(e.key)}`;
}

/** A yearly price as its 4-weekly figure (P11): 13 charges a year, so 8437 is 649. */
function perFourWeeks(yearlyCents: number): number {
  if (yearlyCents % 13 !== 0) throw new Error("a yearly amount must be 13 whole-cent charges");
  return yearlyCents / 13;
}

/**
 * A price the catalog lacks (D6), from Stripe's own fields: "every 2 months", "every week", "every
 * year"; N = 1 is written without a number. `null` when it cannot be stated truthfully: not US
 * dollars, not whole cents, or an interval nobody knows. Nothing Knowlu sells is any of those (D4).
 */
export function unknownPriceText(
  p: { cents: number; currency: string; interval: string; count: number },
): string | null {
  if (p.currency !== "usd" || !Number.isInteger(p.cents) || p.cents < 0) return null;
  if (!["day", "week", "month", "year"].includes(p.interval)) return null;
  if (!Number.isInteger(p.count) || p.count < 1) return null;
  const cadence = p.count === 1 ? `every ${p.interval}` : `every ${p.count} ${p.interval}s`;
  return `${money(p.cents)} ${cadence}`;
}

/** Every dollar figure the catalog writes for an entry: the list the terms tie reads (r5: I1). */
export function figuresFor(e: Entry): string[] {
  return e.key === "yearly" ? [money(e.cents), money(perFourWeeks(e.cents))] : [money(e.cents)];
}

/** The P9 lines the app shows for a selling entry (note §1, D9). */
function offerLines(e: Entry, words: Words): string[] {
  if (e.key === "yearly") {
    return [
      `${money(perFourWeeks(e.cents))} every 4 weeks, billed ${money(e.cents)} once a year. Renews yearly.`,
      `${words.moneyBack}.`,
    ];
  }
  return [`${priceText(e)}.`];
}

/** "Plus tax where it applies." */
export function taxNote(words: Words): string {
  return `${words.tax.charAt(0).toUpperCase()}${words.tax.slice(1)}.`;
}

/** The offer: the selling plans in the catalog's order, each with its cents and lines (D9). */
export function offer(c: Pick<CatalogDef, "termsVersion" | "selling" | "words">): Offer {
  return {
    terms_version: c.termsVersion,
    plans: c.selling.map((e) => ({ plan: e.key, amount_cents: e.cents, lines: offerLines(e, c.words) })),
    note: taxNote(c.words),
  };
}
