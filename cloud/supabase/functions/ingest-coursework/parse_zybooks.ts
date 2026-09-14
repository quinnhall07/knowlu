// zyBooks' assignment payload -> task items. A faithful port of `engine/src/zybooks.rs`'s
// `parse_assignments`, `category_of` and `route_zybook`, measured against the same frozen
// Python-written reference the Rust is measured against (cloud design §4.3).
//
// **Pure.** No fetch, no credential, no clock: the device signs in and fetches with the student's
// own password (D11) and posts the payload here. This half is what a vendor markup change moves,
// and moving it is why a fix becomes a deploy instead of an app release.
import { dueLocal, pyInt, pyStr, round2, slugify, strip, truthy } from "./pyshims.ts";

export const DEFAULT_IMPORTANCE = 3;
export const DEFAULT_FLOOR_HOURS = 0.25;

export interface Assignment {
  uid: string;
  slug: string;
  title: string;
  /** `%Y-%m-%dT%H:%M`, already local and naive — `ingest::format_due`'s spelling. */
  due: string;
  course: string | null;
  effort_hours: number;
  effort_confidence: string;
  effort_source: string;
  importance: number;
  importance_reason: string;
  progress: number;
  created_by: string;
  body: string;
}

/** A 200 whose `success` is falsy is a dead session, not an empty semester. */
export class NotLoggedIn extends Error {}

type Obj = Record<string, unknown>;

function obj(value: unknown): Obj {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Obj : {};
}

function arrayField(map: Obj, key: string): unknown[] {
  const value = map[key];
  if (!truthy(value)) return [];
  if (!Array.isArray(value)) throw new Error(`${key} is not a list`);
  return value;
}

function mappingField(cfg: Obj, key: string): Obj {
  const value = cfg[key];
  if (!truthy(value)) return {};
  if (Array.isArray(value) || typeof value !== "object") {
    throw new Error(`config key "${key}" is not a mapping`);
  }
  return value as Obj;
}

export function requireSuccess(payload: unknown, what: string): void {
  const map = obj(payload);
  if (!("success" in map)) return;
  if (truthy(map.success)) return;
  const detail = truthy(map.error)
    ? pyStr(map.error)
    : truthy(map.message)
    ? pyStr(map.message)
    : "no detail";
  throw new NotLoggedIn(`zybooks ${what} reported success=false (${detail})`);
}

/** First configured prefix the lowercased title starts with, in the config's own order. */
export function categoryOf(title: string, categories: Obj): string | null {
  const lowered = title.toLowerCase();
  for (const [prefix, kind] of Object.entries(categories)) {
    if (lowered.startsWith(prefix.toLowerCase())) return pyStr(kind);
  }
  return null;
}

export type Routing =
  | { kind: "mapped"; mapping: Obj }
  | { kind: "ignored" }
  | { kind: "unmapped" };

/**
 * `ignored` is silent on purpose — `HowToUseZyBooks2` is zyBooks' own onboarding book and a
 * warning there would fire on every healthy run. `unmapped` is a warning nobody should miss: a
 * genuinely new course appearing is something the student must be told about.
 */
export function routeZybook(code: string, courses: Obj, ignore: string[]): Routing {
  const mapping = courses[code];
  if (truthy(mapping) && typeof mapping === "object" && !Array.isArray(mapping)) {
    return { kind: "mapped", mapping: mapping as Obj };
  }
  if (ignore.includes(code)) return { kind: "ignored" };
  return { kind: "unmapped" };
}

export function parseAssignments(
  payload: unknown,
  courseSlug: string,
  courseLabel: string,
  cfg: Obj,
  timeZone: string,
  warnings: string[],
): Assignment[] {
  requireSuccess(payload, "assignment payload");
  // slugify, not the raw config value: the slug becomes tasks/<slug>.md on the device, and a
  // config typo carrying a path separator would otherwise write outside tasks/.
  const slugPrefix = slugify(courseSlug);
  const categories = mappingField(cfg, "categories");
  const effortCfg = mappingField(cfg, "effort");
  const perSection = truthy(effortCfg.minutes_per_section) || effortCfg.minutes_per_section === 0
    ? Number(effortCfg.minutes_per_section)
    : 6;
  const floors = mappingField(effortCfg, "floors");
  const importanceTable = mappingField(cfg, "importance");

  const out: Assignment[] = [];
  for (const rawItem of arrayField(obj(payload), "assignments")) {
    const raw = obj(rawItem);
    const title = strip(truthy(raw.title) ? pyStr(raw.title) : "");
    if (title === "") {
      warnings.push("assignment with no title skipped");
      continue;
    }
    // `is False`, not falsiness: a missing or null `visible` leaves the assignment visible.
    if (raw.visible === false) continue;

    const stamps = arrayField(raw, "due_dates")
      .map((d) => obj(d).date)
      .filter((d) => truthy(d));
    if (stamps.length === 0) {
      warnings.push(`${title}: no due date; skipped`);
      continue;
    }
    // Earliest is the only defensible pick from a plural due_dates[] the vendor does not order:
    // it is the deadline that binds first, and deferring one is far cheaper than missing one.
    let earliest: string | null = null;
    let unreadable: string | null = null;
    for (const stamp of stamps) {
      try {
        const parsed = dueLocal(stamp, timeZone);
        if (earliest === null || parsed < earliest) earliest = parsed;
      } catch (e) {
        unreadable = e instanceof Error ? e.message : String(e);
        break;
      }
    }
    if (unreadable !== null) {
      warnings.push(`${title}: unreadable due date (${unreadable}); skipped`);
      continue;
    }
    if (earliest === null) continue;
    if (stamps.length > 1) {
      warnings.push(`${title}: ${stamps.length} due dates; using the earliest (${earliest})`);
    }

    const sections = arrayField(raw, "sections");
    let points = 0;
    for (const section of sections) {
      try {
        points += pyInt(obj(section).total_points);
      } catch (e) {
        throw new Error(`${title}: ${e instanceof Error ? e.message : String(e)}`);
      }
    }

    const kind = categoryOf(title, categories);
    if (kind === null) warnings.push(`${title}: uncategorised; using default importance`);
    const importance = kind !== null && kind in importanceTable
      ? pyInt(importanceTable[kind])
      : DEFAULT_IMPORTANCE;
    const floor = kind !== null && kind in floors ? Number(floors[kind]) : DEFAULT_FLOOR_HOURS;
    const effort = round2(Math.max(sections.length * perSection / 60, floor));

    const assignmentId = "assignment_id" in raw ? pyStr(raw.assignment_id) : "None";
    const listing = sections.map((s) => {
      const section = obj(s);
      const field = (key: string) => key in section ? pyStr(section[key]) : "None";
      return `- ${field("chapter_number")}.${field("section_number")} ${field("title")}`;
    });
    const body = `${sections.length} zyBooks section(s), ${points} points.\n\n${listing.join("\n")}`;

    out.push({
      uid: `zybooks:${assignmentId}`,
      slug: `${slugPrefix}-${slugify(title)}`,
      title: `${courseLabel} ${title}`,
      due: earliest,
      course: courseSlug,
      effort_hours: effort,
      effort_confidence: "low",
      effort_source: "inferred",
      importance,
      importance_reason:
        `zyBooks ${kind !== null && kind !== "" ? kind : "item"} worth ${points} points across ` +
        `${sections.length} sections; per-category importance from config`,
      progress: 0,
      created_by: "zybooks",
      body,
    });
  }
  return out;
}
