// The eval seed's record shape (ruling R-C2-E10) and the scrub rules that keep it safe to commit.
//
// A seed record is one already-judged item, captured as `request` — exactly the body the device
// sends to `/judge-<kind>` today (`engine/src/cloudmodel.rs`'s `task_request` / `event_request` /
// `email_request`) — paired with `theirs`, the human correction the eval suite scores against.
//
// `cloud/eval/` never reads an archive (ruling R-C2-E12, 2026-09-14: P4 was declined — "nothing is
// ever read from the quinn-ops archive"). This file only says what a valid, scrubbed record looks
// like; `cloud/eval/seed/` is empty at merge and fills later, one consented correction at a time
// (a later stream's opt-in) — see `cloud/eval/seed/README.md`.
import {
  EMAIL_ITEM_FIELDS,
  EMAIL_LABELLED_FIELDS,
  EVENT_ITEM_FIELDS,
  EVENT_LABELLED_FIELDS,
  type Kind,
  TASK_ITEM_FIELDS,
  TASK_LABELLED_FIELDS,
} from "../supabase/functions/_shared/judge_validate.ts";

export type { Kind };

export interface SeedRecord {
  id: string;
  kind: Kind;
  request: {
    kind: Kind;
    item: Record<string, unknown>;
    heuristics_seed: Record<string, unknown>;
  };
  theirs: Record<string, unknown>;
}

const KINDS: readonly Kind[] = ["task", "event", "email"];

const ITEM_FIELDS: Record<Kind, readonly string[]> = {
  task: TASK_ITEM_FIELDS,
  event: EVENT_ITEM_FIELDS,
  email: EMAIL_ITEM_FIELDS,
};

const LABELLED_FIELDS: Record<Kind, readonly string[]> = {
  task: TASK_LABELLED_FIELDS,
  event: EVENT_LABELLED_FIELDS,
  email: EMAIL_LABELLED_FIELDS,
};

const TOP_LEVEL_KEYS = ["id", "kind", "request", "theirs"] as const;
const REQUEST_KEYS = ["kind", "item", "heuristics_seed"] as const;

// A UUID (v1-v5 — the shape every `gen_random_uuid()` primary key in `cloud/supabase/migrations/`
// takes): a seed id must never look like one, because a seed id that happens to collide with a
// live table's row id is exactly the "looks synthetic, isn't" mistake this check exists to catch.
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function sameKeySet(actual: string[], expected: readonly string[]): boolean {
  const a = [...actual].sort();
  const e = [...expected].sort();
  return a.length === e.length && a.every((k, i) => k === e[i]);
}

/**
 * The violations in `value`, empty when it is a valid `SeedRecord`. Structural only — see
 * `scrubViolations` for what may not appear inside an otherwise-valid record.
 */
export function validateSeedRecord(value: unknown): string[] {
  const violations: string[] = [];
  if (!isPlainObject(value)) {
    return ["the record is not an object"];
  }

  const keys = Object.keys(value);
  if (!sameKeySet(keys, TOP_LEVEL_KEYS)) {
    violations.push(
      `top-level keys must be exactly ${TOP_LEVEL_KEYS.join(", ")}; got ${keys.join(", ") || "(none)"}`,
    );
  }

  const kind = value.kind;
  if (typeof kind !== "string" || !KINDS.includes(kind as Kind)) {
    violations.push(`kind must be one of ${KINDS.join(", ")}; got ${JSON.stringify(kind)}`);
    // Without a valid kind there is no field list to check request/theirs against.
    return violations;
  }
  const k = kind as Kind;

  const id = value.id;
  if (typeof id !== "string" || id === "") {
    violations.push("id must be a non-empty string");
  } else {
    if (id.includes("@")) violations.push("id must not be an email address");
    if (UUID_RE.test(id)) violations.push("id must not be a UUID");
    if (!id.startsWith("seed-")) violations.push("id must be synthetic: a 'seed-' prefix");
  }

  const request = value.request;
  if (!isPlainObject(request)) {
    violations.push("request must be an object");
  } else {
    const reqKeys = Object.keys(request);
    if (!sameKeySet(reqKeys, REQUEST_KEYS)) {
      violations.push(
        `request keys must be exactly ${REQUEST_KEYS.join(", ")}; got ${reqKeys.join(", ") || "(none)"}`,
      );
    }
    if (request.kind !== k) {
      violations.push(
        `request.kind must equal the record's kind ('${k}'); got ${JSON.stringify(request.kind)}`,
      );
    }
    const item = request.item;
    if (!isPlainObject(item)) {
      violations.push("request.item must be an object");
    } else {
      const allowed = ITEM_FIELDS[k];
      for (const key of Object.keys(item)) {
        if (!allowed.includes(key)) {
          violations.push(
            `request.item has key '${key}', outside the allowed item fields for kind '${k}' (${
              allowed.join(", ")
            })`,
          );
        }
      }
    }
    if (!isPlainObject(request.heuristics_seed)) {
      violations.push("request.heuristics_seed must be an object");
    }
  }

  const theirs = value.theirs;
  if (!isPlainObject(theirs)) {
    violations.push("theirs must be an object");
  } else {
    const theirsKeys = Object.keys(theirs);
    if (theirsKeys.length === 0) {
      violations.push("theirs must carry at least one labelled field");
    }
    const allowedLabels = LABELLED_FIELDS[k];
    for (const key of theirsKeys) {
      if (!allowedLabels.includes(key)) {
        violations.push(
          `theirs has key '${key}', outside the labelled fields for kind '${k}' (${
            allowedLabels.join(", ")
          })`,
        );
      }
    }
  }

  return violations;
}

// Keys that never belong in `request.item`, wherever they appear in it — each one is either an
// address (`email`, `from`, `to`, `organizer_email`), a list of addresses (`attendees`), or free
// text a person wrote (`body`, `snippet`, `raw`) that no character-level filter makes safe. `body`
// and `from` really are fields the device sends today (`task_request`, `email_request`) — the rule
// is that a SEED item never carries them, not that the real request never does.
const BANNED_ITEM_KEYS = ["email", "from", "to", "organizer_email", "attendees", "body", "snippet", "raw"];

const EMAIL_TOKEN_RE = /\S+@\S+/;
const URL_RE = /https?:\/\//;
const WINDOWS_PATH_RE = /[A-Za-z]:\\/;
const POSIX_HOME_RE = /\/(home|Users)\//;
const LONG_NUMBER_RE = /\d{7,}/;
const MAX_STRING_CHARS = 200;

function walkStrings(value: unknown, path: string, cb: (s: string, path: string) => void): void {
  if (typeof value === "string") {
    cb(value, path);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((v, i) => walkStrings(v, `${path}[${i}]`, cb));
    return;
  }
  if (isPlainObject(value)) {
    for (const [k, v] of Object.entries(value)) walkStrings(v, path === "" ? k : `${path}.${k}`, cb);
  }
}

function walkKeys(value: unknown, cb: (key: string) => void): void {
  if (Array.isArray(value)) {
    for (const v of value) walkKeys(v, cb);
    return;
  }
  if (isPlainObject(value)) {
    for (const [k, v] of Object.entries(value)) {
      cb(k);
      walkKeys(v, cb);
    }
  }
}

/**
 * What may never appear inside `request` or `theirs`, checked independently of
 * `validateSeedRecord` so a record can be schema-valid and still fail the scrub (that combination
 * is exactly what `loadSeed` refuses to load silently past).
 */
export function scrubViolations(record: SeedRecord): string[] {
  const violations: string[] = [];
  const scopes: Array<[string, unknown]> = [
    ["request", record.request],
    ["theirs", record.theirs],
  ];

  for (const [label, scope] of scopes) {
    walkStrings(scope, "", (s, path) => {
      const where = `${label}.${path}`;
      if (EMAIL_TOKEN_RE.test(s)) violations.push(`${where} carries an '@' token`);
      if (URL_RE.test(s)) violations.push(`${where} carries a URL`);
      if (WINDOWS_PATH_RE.test(s)) violations.push(`${where} carries a Windows path`);
      if (POSIX_HOME_RE.test(s)) violations.push(`${where} carries a POSIX home path`);
      if (LONG_NUMBER_RE.test(s)) violations.push(`${where} carries a long number`);
      if ([...s].length > MAX_STRING_CHARS) {
        violations.push(`${where} is over ${MAX_STRING_CHARS} characters`);
      }
    });
  }

  const item = isPlainObject(record.request) ? record.request.item : undefined;
  walkKeys(item, (key) => {
    if (BANNED_ITEM_KEYS.includes(key)) violations.push(`request.item carries a banned key '${key}'`);
  });

  return violations;
}
