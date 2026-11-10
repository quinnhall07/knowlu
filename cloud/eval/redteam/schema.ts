// The red-team fixture record (plan `2026-11-09-prompt-injection-suite`, §4.2 and §5) and the two checks a
// fixture passes before anything reads it: `validateFixture`, its shape, and `hygieneViolations`, what keeps
// it safe to publish. `loader.ts` runs both on every line of `fixtures/*.jsonl`.
//
// The fixtures are public, so hygiene is enforced here, in code, not by review. Every string anywhere in a
// record, object keys included, is read on `detect.ts`'s matching form (the oracle's own fold), after
// percent and HTML-entity escapes are decoded and spelled-out dots and ats are read as `.` and `@`. A real
// host hidden by case, a zero-width or bidi character, a full-width dot, ` dot `, `[.]` or base64 is read as
// the host it spells. Which final label is a top-level domain is the oracle's decision (`policyBreaches`),
// so the two never disagree on what a host is. The eval seed's scrub rules (`cloud/eval/schema.ts`) do not
// apply: a fixture carries URLs, bodies and senders, and never enters `eval_cases`.
//
// Pure and deterministic: no I/O, no clock, no network. Every regular expression runs in time linear in its
// input: every unbounded quantifier is over one character class, and no quantified group holds one (the one
// quantified group, `URL_TAIL`'s optional port, holds a bounded `{1,5}` and reads at most eight characters).
import { matchingForm, policyBreaches } from "./detect.ts";
import { MAIL_TEXT_CHARS } from "../../supabase/functions/_shared/judge_prompts.ts";
import {
  EMAIL_ITEM_FIELDS,
  EMAIL_TIERS,
  EVENT_ITEM_FIELDS,
  EVENT_VERDICTS,
  TASK_ITEM_FIELDS,
} from "../../supabase/functions/_shared/judge_validate.ts";
import type { MailClaim } from "../../supabase/functions/_shared/mail_templates.ts";

// ---------------------------------------------------------------------------------------------
// The record.
// ---------------------------------------------------------------------------------------------

/** §5: one fixture file per kind, named for it. */
export const FIXTURE_KINDS = ["task", "event", "email", "mail_claim", "mail_resolve"] as const;
export type FixtureKind = (typeof FIXTURE_KINDS)[number];

/** §5's attack classes. */
export const ATTACK_CLASSES = [
  "C1",
  "C2",
  "C3",
  "C4",
  "C5",
  "C6",
  "C7",
  "C8",
  "C9",
  "C10",
  "C11",
  "C12",
  "C13",
] as const;
export type AttackClass = (typeof ATTACK_CLASSES)[number];

/** Classes whose payload always aims at a decision: an attack of one must check that decision, so its goal
 * carries a predicate other than `absent_strings` (T3a ruling I4; `fixtures_test.ts` checks the set too). */
export const DECISION_CLASSES: readonly AttackClass[] = ["C1", "C4", "C5", "C11"];

/** The claim kinds the model can emit: `MAIL_CLAIM_SCHEMA`'s enum. `receipt` is template-only (PQ4) and
 * `validateMailClaims` refuses one from the model, so `claim_kind_absent: receipt` could never fail. */
export const MODEL_CLAIM_KINDS = [
  "new_item",
  "change",
  "course_info",
  "noise",
] as const satisfies readonly Exclude<
  MailClaim["kind"],
  "receipt"
>[];

/** §4.2's goal predicates and the three decision predicates of the T3a rulings: a closed set. */
export const GOAL_PREDICATES = [
  "claim_kind_absent",
  "tier_not_in",
  "verdict_not",
  "target_id",
  "importance_not_in",
  "course_not",
  "due_not",
  "absent_strings",
] as const;
export type GoalPredicate = (typeof GOAL_PREDICATES)[number];

/** The kinds each predicate checks: a decision the kind's validated output really carries. */
export const PREDICATE_KINDS: Readonly<Record<GoalPredicate, readonly FixtureKind[]>> = {
  claim_kind_absent: ["mail_claim"],
  tier_not_in: ["email"],
  verdict_not: ["event"],
  target_id: ["mail_resolve"],
  importance_not_in: ["task", "email"],
  course_not: ["task"],
  due_not: ["email"],
  absent_strings: FIXTURE_KINDS,
};

/** The attacker's intended effect, as data: each predicate holds when the effect did NOT happen. */
export interface Goal {
  /** No validated claim of this kind. */
  claim_kind_absent?: (typeof MODEL_CLAIM_KINDS)[number];
  /** The validated tier is none of these. */
  tier_not_in?: Array<(typeof EMAIL_TIERS)[number]>;
  /** The validated verdict is not this one. */
  verdict_not?: (typeof EVENT_VERDICTS)[number];
  /** The resolve picked no candidate. */
  target_id?: null;
  /** The validated importance is none of these. */
  importance_not_in?: number[];
  /** The validated course is not this slug, one of the record's own `known_courses`. */
  course_not?: string;
  /** The validated due is not on this day (YYYY-MM-DD). */
  due_not?: string;
  /** No output string holds any of these, on the matching form. */
  absent_strings?: string[];
}

/** The body the device or the server builds for the kind (T3a ruling: one shape for every kind). */
export interface FixtureRequest {
  kind: FixtureKind;
  item: Record<string, unknown>;
  heuristics_seed: Record<string, unknown>;
}

/** The clean item. It carries none of an attack's fields. */
export interface TwinFixture {
  id: string;
  kind: FixtureKind;
  request: FixtureRequest;
  class?: never;
  twin_of?: never;
  goal?: never;
  planted?: never;
}

/** The item with a payload: `class` marks it. */
export interface AttackFixture {
  id: string;
  kind: FixtureKind;
  class: AttackClass;
  request: FixtureRequest;
  twin_of: string;
  goal: Goal;
  planted: string[];
}

export type Fixture = TwinFixture | AttackFixture;

export function isAttack(fixture: Fixture): fixture is AttackFixture {
  return "class" in fixture && fixture.class !== undefined;
}

/** `buildMailClaimPrompt`'s item and seed fields. */
export const MAIL_CLAIM_ITEM_FIELDS = ["subject", "from", "date", "text"] as const;
const MAIL_CLAIM_SEED_FIELDS = ["known_courses", "time_zone"] as const;
/** `buildMailResolvePrompt`'s batch: its claims' and candidates' fields. */
export const MAIL_RESOLVE_ITEM_FIELDS = ["claims", "candidates"] as const;
const RESOLVE_CLAIM_FIELDS = ["key", "target", "due"] as const;
const RESOLVE_CANDIDATE_FIELDS = ["id", "title", "due"] as const;

const ITEM_FIELDS: Readonly<Record<FixtureKind, readonly string[]>> = {
  task: TASK_ITEM_FIELDS,
  event: EVENT_ITEM_FIELDS,
  email: EMAIL_ITEM_FIELDS,
  mail_claim: MAIL_CLAIM_ITEM_FIELDS,
  mail_resolve: MAIL_RESOLVE_ITEM_FIELDS,
};

const RECORD_KEYS = ["id", "kind", "class", "request", "twin_of", "goal", "planted"] as const;
const ATTACK_ONLY_KEYS = ["twin_of", "goal", "planted"] as const;
const REQUEST_KEYS = ["kind", "item", "heuristics_seed"] as const;

// ---------------------------------------------------------------------------------------------
// Shared helpers.
// ---------------------------------------------------------------------------------------------

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function includes(list: readonly string[], value: unknown): boolean {
  return typeof value === "string" && list.includes(value);
}

/** A value for a message: JSON, cut so a 2,000-character string does not flood the error. */
function show(value: unknown): string {
  const text = JSON.stringify(value) ?? String(value);
  return text.length > 80 ? `${text.slice(0, 77)}...` : text;
}

/** An eval seed id (`cloud/eval/schema.ts`'s prefix) at the start of a word. */
const SEED_ID = /(?<![\p{L}\p{N}_])seed-/u;
/** A UUID anywhere, the shape of every `gen_random_uuid()` key; tested on the lower-cased matching form. */
const UUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/;

/** The two rules every string obeys, on the matching form; `validateFixture` applies them to `id` too. */
function idRuleProblems(form: string): string[] {
  const problems: string[] = [];
  if (SEED_ID.test(form)) problems.push("holds a 'seed-' id: the eval seed's ids never appear in a fixture");
  if (UUID.test(form)) problems.push("holds a UUID: a real row's id never appears in a fixture");
  return problems;
}

/** A real calendar day in the form YYYY-MM-DD. */
function isDate(text: string): boolean {
  if (!/^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(text)) return false;
  const [year, month, day] = text.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  return date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day;
}

// ---------------------------------------------------------------------------------------------
// Structure.
// ---------------------------------------------------------------------------------------------

/**
 * The structural violations in `value`, [] for a valid `Fixture`: the keys, the kind and class, the id, the
 * request's shape for its kind, and an attack's twin, goal and planted strings. A goal predicate must belong
 * to the record's kind and take an argument a validated output could contradict, so no goal holds
 * vacuously. What may appear inside the strings is `hygieneViolations`'s business, except the id's.
 */
export function validateFixture(value: unknown): string[] {
  if (!isPlainObject(value)) return ["a fixture must be a JSON object"];
  const violations: string[] = [];

  for (const key of Object.keys(value)) {
    if (!includes(RECORD_KEYS, key)) {
      violations.push(`unknown key ${show(key)}: a fixture holds only ${RECORD_KEYS.join(", ")}`);
    }
  }

  checkId(value.id, violations);

  const kind = includes(FIXTURE_KINDS, value.kind) ? value.kind as FixtureKind : null;
  if (kind === null) violations.push(`kind ${show(value.kind)} is not one of ${FIXTURE_KINDS.join(", ")}`);

  checkRequest(value.request, kind, violations);

  if ("class" in value) {
    checkAttack(value, kind, violations);
  } else {
    for (const key of ATTACK_ONLY_KEYS) {
      if (key in value) {
        violations.push(
          `'${key}' on a record with no 'class': a twin carries none of class, twin_of, goal or planted, ` +
            "and an attack needs a class",
        );
      }
    }
  }
  return violations;
}

function checkId(id: unknown, violations: string[]): void {
  if (typeof id !== "string" || id === "") {
    violations.push("id must be a non-empty string starting 'rt-'");
    return;
  }
  if (!id.startsWith("rt-")) violations.push(`id ${show(id)} must start with the 'rt-' prefix`);
  else if (!/^rt-[a-z0-9-]+$/.test(id)) {
    violations.push(`id ${show(id)} must be 'rt-' then lower-case letters, digits and hyphens`);
  }
  for (const problem of idRuleProblems(matchingForm(id))) violations.push(`id ${show(id)} ${problem}`);
}

function checkRequest(request: unknown, kind: FixtureKind | null, violations: string[]): void {
  if (!isPlainObject(request)) {
    violations.push("request must be an object {kind, item, heuristics_seed}");
    return;
  }
  for (const key of Object.keys(request)) {
    if (!includes(REQUEST_KEYS, key)) {
      violations.push(`request has unknown key ${show(key)}: it holds only ${REQUEST_KEYS.join(", ")}`);
    }
  }
  if (kind !== null && request.kind !== kind) {
    violations.push(`request.kind ${show(request.kind)} must equal the record's kind '${kind}'`);
  }

  const item = request.item;
  if (!isPlainObject(item)) {
    violations.push("request.item must be an object");
  } else if (kind !== null) {
    const allowed = ITEM_FIELDS[kind];
    for (const key of Object.keys(item)) {
      if (!allowed.includes(key)) {
        violations.push(
          `request.item has key ${show(key)}, not a ${kind} item field (${allowed.join(", ")})`,
        );
      }
    }
    if (kind === "mail_claim") {
      for (const field of MAIL_CLAIM_ITEM_FIELDS) {
        if (field in item && typeof item[field] !== "string") {
          violations.push(`request.item.${field} must be a string`);
        }
      }
    }
    if (kind === "mail_resolve") checkResolveBatch(item, violations);
  }

  const seed = request.heuristics_seed;
  if (!isPlainObject(seed)) {
    violations.push("request.heuristics_seed must be an object");
    return;
  }
  if ("known_courses" in seed) {
    const known = seed.known_courses;
    if (
      !Array.isArray(known) || !known.every((c) => typeof c === "string" && c.trim() !== "") ||
      new Set(known).size !== known.length
    ) {
      violations.push("request.heuristics_seed.known_courses must be an array of distinct, non-empty slugs");
    }
  }
  if (kind === "mail_resolve" && Object.keys(seed).length > 0) {
    violations.push(
      "request.heuristics_seed must be {} for mail_resolve: buildMailResolvePrompt reads no seed",
    );
  }
  if (kind === "mail_claim") {
    for (const key of Object.keys(seed)) {
      if (!includes(MAIL_CLAIM_SEED_FIELDS, key)) {
        violations.push(
          `request.heuristics_seed has key ${show(key)}, not a mail_claim seed field ` +
            `(${MAIL_CLAIM_SEED_FIELDS.join(", ")})`,
        );
      }
    }
  }
}

/** A resolve batch with no claim or no candidate gives the model nothing to pick, so `target_id: null`
 * would hold whatever it did. Each entry carries its prompt's fields only, its key or id non-empty. */
function checkResolveBatch(item: Record<string, unknown>, violations: string[]): void {
  const lists: Array<[string, readonly string[], string]> = [
    ["claims", RESOLVE_CLAIM_FIELDS, "key"],
    ["candidates", RESOLVE_CANDIDATE_FIELDS, "id"],
  ];
  for (const [list, fields, idField] of lists) {
    const entries = item[list];
    if (!Array.isArray(entries) || entries.length === 0) {
      violations.push(`request.item.${list} must be a non-empty array`);
      continue;
    }
    entries.forEach((entry, i) => {
      const where = `request.item.${list}[${i}]`;
      if (!isPlainObject(entry)) {
        violations.push(`${where} must be an object of ${fields.join(", ")}`);
        return;
      }
      for (const key of Object.keys(entry)) {
        if (!fields.includes(key)) {
          violations.push(`${where} has key ${show(key)}, not one of ${fields.join(", ")}`);
        }
      }
      if (typeof entry[idField] !== "string" || (entry[idField] as string).trim() === "") {
        violations.push(`${where}.${idField} must be a non-empty string`);
      }
      for (const key of fields) {
        if (key !== idField && key in entry && entry[key] !== null && typeof entry[key] !== "string") {
          violations.push(`${where}.${key} must be a string or null`);
        }
      }
    });
  }
}

function checkAttack(record: Record<string, unknown>, kind: FixtureKind | null, violations: string[]): void {
  const cls = includes(ATTACK_CLASSES, record.class) ? record.class as AttackClass : null;
  if (cls === null) violations.push(`class ${show(record.class)} is not one of C1 to C13`);

  const twinOf = record.twin_of;
  if (typeof twinOf !== "string" || twinOf === "") {
    violations.push("an attack needs twin_of: the id of its clean twin");
  } else if (twinOf === record.id) {
    violations.push("twin_of names the attack itself, not its twin");
  } else if (!twinOf.startsWith("rt-")) {
    violations.push(`twin_of ${show(twinOf)} is not a fixture id ('rt-')`);
  }

  const planted = record.planted;
  if (!Array.isArray(planted) || planted.length === 0) {
    violations.push("an attack needs planted: a non-empty array of the strings its payload carries");
  } else {
    planted.forEach((p, i) => {
      if (typeof p !== "string" || p.trim() === "") {
        violations.push(`planted[${i}] must be a non-empty string`);
      } else if (planted.indexOf(p) !== i) violations.push(`planted[${i}] repeats ${show(p)}`);
    });
  }

  const goal = record.goal;
  if (!isPlainObject(goal) || Object.keys(goal).length === 0) {
    violations.push("an attack needs a goal: a non-empty object of goal predicates");
    return;
  }
  const request = isPlainObject(record.request) ? record.request : {};
  const seed = isPlainObject(request.heuristics_seed) ? request.heuristics_seed : {};
  const known = Array.isArray(seed.known_courses) ? seed.known_courses : [];

  for (const [name, argument] of Object.entries(goal)) {
    if (!includes(GOAL_PREDICATES, name)) {
      violations.push(`goal predicate ${show(name)} is not one of ${GOAL_PREDICATES.join(", ")}`);
      continue;
    }
    const predicate = name as GoalPredicate;
    if (kind !== null && !PREDICATE_KINDS[predicate].includes(kind)) {
      violations.push(
        `goal.${predicate} does not apply to a ${kind} fixture: it checks ${
          PREDICATE_KINDS[predicate].join(", ")
        } output`,
      );
      continue;
    }
    const problem = argumentProblem(predicate, argument, known);
    if (problem !== null) violations.push(`goal.${predicate}: ${problem}`);
  }

  const decisions = Object.keys(goal).filter((k) => includes(GOAL_PREDICATES, k) && k !== "absent_strings");
  if (cls !== null && DECISION_CLASSES.includes(cls) && decisions.length === 0) {
    violations.push(
      `a ${cls} attack's goal needs a decision predicate, not absent_strings alone: its payload aims at a decision`,
    );
  }
}

/** A non-empty list of distinct members of `vocabulary`, leaving at least one member out (a predicate
 * naming every value could never hold). */
function subsetProblem(argument: unknown, vocabulary: readonly unknown[], what: string): string | null {
  if (!Array.isArray(argument) || argument.length === 0) return `must be a non-empty array of ${what}`;
  const off = argument.filter((a) => !vocabulary.includes(a));
  if (off.length > 0) return `${off.map(show).join(", ")} ${off.length === 1 ? "is" : "are"} not ${what}`;
  if (new Set(argument).size !== argument.length) return `${show(argument)} repeats a value`;
  if (argument.length >= vocabulary.length) {
    return `${show(argument)} names every value, so it could never hold`;
  }
  return null;
}

const IMPORTANCES = [1, 2, 3, 4, 5] as const;

function argumentProblem(
  predicate: GoalPredicate,
  argument: unknown,
  known: readonly unknown[],
): string | null {
  switch (predicate) {
    case "claim_kind_absent":
      return includes(MODEL_CLAIM_KINDS, argument)
        ? null
        : `${show(argument)} is not a claim kind the model ` +
          `can emit (${MODEL_CLAIM_KINDS.join(", ")}; a receipt is template-only)`;
    case "tier_not_in":
      return subsetProblem(argument, EMAIL_TIERS, `an email tier (${EMAIL_TIERS.join(", ")})`);
    case "verdict_not":
      return includes(EVENT_VERDICTS, argument)
        ? null
        : `${show(argument)} is not an event verdict (${EVENT_VERDICTS.join(", ")})`;
    case "target_id":
      return argument === null
        ? null
        : `${show(argument)} must be null: the resolve should pick no candidate`;
    case "importance_not_in":
      return subsetProblem(argument, IMPORTANCES, "a whole number 1 to 5");
    case "course_not":
      if (typeof argument !== "string" || argument.trim() === "") return "must be a non-empty course slug";
      return known.includes(argument) ? null : `${show(argument)} is not in this record's ` +
        "heuristics_seed.known_courses; validation nulls any other slug, so the goal could never fail";
    case "due_not":
      return typeof argument === "string" && isDate(argument)
        ? null
        : `${show(argument)} must be a real calendar day, YYYY-MM-DD`;
    case "absent_strings":
      if (!Array.isArray(argument) || argument.length === 0) return "must be a non-empty array of strings";
      if (!argument.every((a) => typeof a === "string" && a.trim() !== "")) {
        return "every entry must be a non-empty string";
      }
      return new Set(argument).size === argument.length ? null : `${show(argument)} repeats a string`;
  }
}

// ---------------------------------------------------------------------------------------------
// Hygiene.
// ---------------------------------------------------------------------------------------------

const RESERVED_TEXT = "example, example.com, example.org or example.net, a name under one, or a name under " +
  ".test or .invalid";
const IP_RANGES = "192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24 or 2001:db8::/32";

/** §5's reserved names (RFC 2606): `example`, `example.com`, `example.org`, `example.net`, a name under one
 * of them, or a name under `.test` or `.invalid`. */
function reserved(host: string): boolean {
  if (
    ["example", "example.com", "example.org", "example.net"].some((b) => host === b || host.endsWith(`.${b}`))
  ) {
    return true;
  }
  return host.endsWith(".test") || host.endsWith(".invalid");
}

/** The documentation IPv4 ranges (RFC 5737). */
function documentationV4(octets: readonly number[]): boolean {
  const net = octets.slice(0, 3).join(".");
  return net === "192.0.2" || net === "198.51.100" || net === "203.0.113";
}

function codePoint(n: number): string | null {
  return n <= 0x10ffff && (n < 0xd800 || n > 0xdfff) ? String.fromCodePoint(n) : null;
}

/** Percent escapes (byte by byte), numeric HTML entities and the named ones for a dot, an at and a slash,
 * decoded. */
function decodeEscapes(text: string): string {
  return text
    .replace(/%([0-9a-fA-F]{2})/g, (_, hex: string) => String.fromCharCode(parseInt(hex, 16)))
    .replace(/&#([0-9]{1,7});/g, (whole, dec: string) => codePoint(Number(dec)) ?? whole)
    .replace(/&#[xX]([0-9a-fA-F]{1,6});/g, (whole, hex: string) => codePoint(parseInt(hex, 16)) ?? whole)
    .replace(/&period;/gi, ".")
    .replace(/&commat;/gi, "@")
    .replace(/&sol;/gi, "/");
}

/**
 * The form hosts are read on: escapes decoded, then the matching form, then whitespace runs folded to one
 * space, then the spelled-out dots (`[dot]`, `(dot)`, `{dot}`, `<dot>`, `[.]` and the like, ` dot `, ` . `)
 * read as `.` and the bracketed ats (`[at]`, `(at)`) as `@`. Case is already folded, so `DOT` is ` dot `.
 */
function hostForm(text: string): string {
  return matchingForm(decodeEscapes(text))
    .replace(/\s+/g, " ")
    .replace(/ ?[\[({<] ?(?:dot|\.) ?[\])}>] ?/g, ".")
    .replace(/ ?[\[({<] ?at ?[\])}>] ?/g, "@")
    .replace(/ (?:dot|\.) /g, ".");
}

/** Whether the oracle reads `token` alone as a host: its TLD rule and extension list, not a copy of them. */
function oracleHost(token: string): boolean {
  return policyBreaches(token).includes("url");
}

const extensionCache = new Map<string, boolean>();

/** A label on the oracle's file-extension list: a host before a path, a file name without one. */
function isExtensionLabel(label: string): boolean {
  let known = extensionCache.get(label);
  if (known === undefined) {
    known = !oracleHost(`a.${label}`) && oracleHost(`a.${label}/`);
    extensionCache.set(label, known);
  }
  return known;
}

/** A path, query or port right after a host. */
const URL_TAIL = /^(?::[0-9]{1,5})?(?:[\/\\]|[?#]\S)/;

/**
 * Whether the dot-joined `labels` name a host. A final label the oracle reads as a TLD makes one. A final
 * label on the extension list makes one in a URL's host position, after `www`, before a path or port, or
 * when the token is longer than `name.ext` or `name.ext.ext` (a file name such as `essay.docx` or
 * `notes.tar.gz` is not a host).
 */
function isDomain(labels: readonly string[], urlHost: boolean, rest: string): boolean {
  if (oracleHost(labels.join("."))) return true;
  if (!isExtensionLabel(labels[labels.length - 1])) return false;
  if (urlHost || labels[0] === "www" || URL_TAIL.test(rest)) return true;
  const fileName = labels.length === 2 || (labels.length === 3 && isExtensionLabel(labels[1]));
  return !fileName;
}

/** Dot-separated labels of host characters (detect.ts's HOST_SHAPED class), dots included in the run. */
const LABEL_RUN = /[\p{L}\p{M}\p{N}_.\-]+/gu;
const LABEL_RUN_AT = /[\p{L}\p{M}\p{N}_.\-]+/uy;
const NUMERIC_LABEL = /^(?:[0-9]+|0x[0-9a-f]*)$/;
const CANONICAL_OCTET = /^(?:0|[1-9][0-9]{0,2})$/;
/** A character an address's local part may end with. */
const LOCAL_PART_END = /[\p{L}\p{M}\p{N}_.%+\-]/u;
const HEX_RUN = /[0-9a-f:.]+/g;
const WORD_CHAR = /[\p{L}\p{N}_]/u;

function labelsOf(run: string): string[] {
  return run.split(".").filter((label) => label !== "");
}

/**
 * An all-numeric host: a canonical dotted quad must be in a documentation range; any other numeric form
 * (a leading zero, an octet over 255, a hex or bare-integer host) is refused where a resolver would read it
 * as an address, which is in a URL's host position or with four parts anywhere.
 */
function ipv4Problem(labels: readonly string[], urlHost: boolean): string | null {
  const ip = labels.join(".");
  if (labels.length === 4 && labels.every((l) => CANONICAL_OCTET.test(l) && Number(l) <= 255)) {
    return documentationV4(labels.map(Number))
      ? null
      : `'${ip}' is an IP address outside the documentation ranges (${IP_RANGES})`;
  }
  if (labels.length === 4 || urlHost) {
    return `'${ip}' is shaped like an IP address but is not a canonical dotted quad in ${IP_RANGES}`;
  }
  return null;
}

/** The eight 16-bit groups of an IPv6 address, an embedded IPv4 tail included, or null if `text` is not one. */
function parseIPv6(text: string): number[] | null {
  let body = text;
  const lastColon = text.lastIndexOf(":");
  const tail = text.slice(lastColon + 1);
  if (tail.includes(".")) {
    const quad = tail.split(".");
    if (quad.length !== 4 || !quad.every((q) => CANONICAL_OCTET.test(q) && Number(q) <= 255)) return null;
    const [a, b, c, d] = quad.map(Number);
    body = `${text.slice(0, lastColon + 1)}${((a << 8) | b).toString(16)}:${((c << 8) | d).toString(16)}`;
  }
  const halves = body.split("::");
  if (halves.length > 2) return null;
  const groups = (part: string) => part === "" ? [] : part.split(":");
  const head = groups(halves[0]);
  const rest = halves.length === 2 ? groups(halves[1]) : [];
  if (![...head, ...rest].every((g) => /^[0-9a-f]{1,4}$/.test(g))) return null;
  const count = head.length + rest.length;
  if (halves.length === 1 ? count !== 8 : count > 7) return null;
  return [...head, ...Array(8 - count).fill("0"), ...rest].map((g) => parseInt(g, 16));
}

/** Every host, address and IP literal in `text` that is not reserved, as messages. */
function hostProblems(text: string): string[] {
  const form = hostForm(text);
  const problems: string[] = [];

  for (const run of form.matchAll(LABEL_RUN)) {
    const labels = labelsOf(run[0]);
    if (labels.length === 0) continue;
    const start = run.index;
    const urlHost = /[\/\\]{2}$/.test(form.slice(Math.max(0, start - 2), start)) || form[start - 1] === "@";
    if (labels.every((l) => NUMERIC_LABEL.test(l))) {
      const problem = ipv4Problem(labels, urlHost);
      if (problem !== null) problems.push(problem);
      continue;
    }
    const end = start + run[0].length;
    if (labels.length >= 2 && isDomain(labels, urlHost, form.slice(end, end + 8))) {
      const host = labels.join(".");
      if (!reserved(host)) problems.push(`'${host}' is not a reserved domain (${RESERVED_TEXT})`);
    }
  }

  // `local@domain`, however short its domain: `prof@mailhost` is at no reserved name either.
  for (let at = form.indexOf("@"); at !== -1; at = form.indexOf("@", at + 1)) {
    if (at === 0 || !LOCAL_PART_END.test(form[at - 1])) continue;
    LABEL_RUN_AT.lastIndex = at + 1;
    const match = LABEL_RUN_AT.exec(form);
    if (match === null) continue;
    const labels = labelsOf(match[0]);
    // A numeric domain is an IP literal: the IPv4 rule above judges it.
    if (labels.length === 0 || labels.every((l) => NUMERIC_LABEL.test(l))) continue;
    const domain = labels.join(".");
    if (!reserved(domain)) {
      problems.push(`an address at '${domain}' is not at a reserved domain (${RESERVED_TEXT})`);
    }
  }

  for (const run of form.matchAll(HEX_RUN)) {
    let candidate = run[0];
    while (candidate.endsWith(".")) candidate = candidate.slice(0, -1);
    if (candidate.split(":").length < 3 || !/[0-9a-f]/.test(candidate)) continue;
    const before = form[run.index - 1];
    const after = form[run.index + run[0].length];
    if ((before !== undefined && WORD_CHAR.test(before)) || (after !== undefined && WORD_CHAR.test(after))) {
      continue;
    }
    const groups = parseIPv6(candidate);
    if (groups !== null && !(groups[0] === 0x2001 && groups[1] === 0x0db8)) {
      problems.push(`'${candidate}' is an IP address outside the documentation ranges (${IP_RANGES})`);
    }
  }
  return problems;
}

/** A base64 or base64url token long enough to hide a host. */
const BASE64_TOKEN = /[A-Za-z0-9+\/_-]{12,}={0,2}/g;

/** The token's decoding when every byte is printable ASCII (text, not a random identifier), else null. */
function decodedBase64(token: string): string | null {
  const body = token.replace(/=+$/, "").replace(/-/g, "+").replace(/_/g, "/");
  if (body.length % 4 === 1) return null;
  let binary: string;
  try {
    binary = atob(body + "=".repeat((4 - (body.length % 4)) % 4));
  } catch {
    return null;
  }
  for (let i = 0; i < binary.length; i++) {
    const c = binary.charCodeAt(i);
    if (!(c === 0x09 || c === 0x0a || c === 0x0d || (c >= 0x20 && c <= 0x7e))) return null;
  }
  return binary;
}

const CANARY_MENTION = /knowlu-canary/gi;
const CANARY_AT = /KNOWLU-CANARY-[A-Z0-9]+(?![A-Za-z0-9_-])/y;
const FORM_CANARY_AT = /knowlu-canary-[a-z0-9]+(?![a-z0-9_-])/y;
const FORM_CANARY = /knowlu-canary-[a-z0-9]+/g;
const TOKEN_CHAR = /[A-Za-z0-9_-]/;

/** Whether every mention of the canary prefix in `text` starts a whole token of the generated format. */
function canariesWellFormed(text: string, at: RegExp): boolean {
  for (const mention of text.matchAll(CANARY_MENTION)) {
    if (mention.index > 0 && TOKEN_CHAR.test(text[mention.index - 1])) return false;
    at.lastIndex = mention.index;
    if (!at.test(text)) return false;
  }
  return true;
}

/** Every canary in `record`, on the matching form and upper-cased, sorted: the loader's uniqueness key. */
export function canariesIn(record: unknown): string[] {
  const found = new Set<string>();
  walk(record, "", (text) => {
    for (const m of matchingForm(text).matchAll(FORM_CANARY)) found.add(m[0].toUpperCase());
  });
  return [...found].sort();
}

/** Calls `visit` on every string inside `value`, object keys included, with where it sits. */
function walk(value: unknown, where: string, visit: (text: string, where: string) => void): void {
  if (typeof value === "string") {
    visit(value, where === "" ? "the record" : where);
  } else if (Array.isArray(value)) {
    value.forEach((entry, i) => walk(entry, `${where}[${i}]`, visit));
  } else if (isPlainObject(value)) {
    for (const [key, entry] of Object.entries(value)) {
      const name = /^[A-Za-z0-9_]{1,40}$/.test(key) ? key : `[${show(key)}]`;
      const path = where === "" ? name : `${where}.${name}`;
      visit(key, `the key ${path}`);
      walk(entry, path, visit);
    }
  }
}

function stringProblems(text: string): string[] {
  const problems: string[] = [];
  // Code points, as `mailPromptText` cuts the mail text.
  const length = [...text].length;
  if (length > MAIL_TEXT_CHARS) {
    problems.push(
      `${length} characters, over the ${MAIL_TEXT_CHARS.toLocaleString("en-US")}-character bound (the mail ` +
        "text cut)",
    );
  }
  const form = matchingForm(text);
  problems.push(...idRuleProblems(form));
  if (!canariesWellFormed(text, CANARY_AT) || !canariesWellFormed(form, FORM_CANARY_AT)) {
    problems.push(
      "'KNOWLU-CANARY-' must start a whole token with a generated suffix of A-Z and 0-9 " +
        "(KNOWLU-CANARY-[A-Z0-9]+)",
    );
  }
  problems.push(...hostProblems(text));
  for (const token of text.matchAll(BASE64_TOKEN)) {
    const decoded = decodedBase64(token[0]);
    if (decoded === null) continue;
    for (const problem of hostProblems(decoded)) {
      problems.push(`base64 ${show(token[0])} decodes to text where ${problem}`);
    }
  }
  return problems;
}

/**
 * §5's publishing rules over every string anywhere in `record`, object keys included, [] when clean: every
 * host, address and IP literal reserved, every canary of the generated format, no string over the mail
 * text cut, no `seed-` id and no UUID. A twin carries no canary: a canary is payload.
 */
export function hygieneViolations(record: Fixture): string[] {
  const problems = new Set<string>();
  walk(record, "", (text, where) => {
    for (const problem of stringProblems(text)) problems.add(`${where}: ${problem}`);
  });
  const value: unknown = record;
  if (isPlainObject(value) && !("class" in value)) {
    const canaries = canariesIn(value);
    if (canaries.length > 0) {
      problems.add(`a twin carries ${canaries.join(", ")}: a canary is payload, and a twin carries none`);
    }
  }
  return [...problems];
}
