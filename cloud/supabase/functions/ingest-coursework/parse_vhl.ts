// The VHL dashboard page -> one task item per due-date bucket. A faithful port of
// `engine/src/vhl.rs`'s `parse_dashboard` and `coursework::parse_duration_hours`, measured against
// the same frozen Python-written reference (cloud design §4.3).
//
// The device still does the whole credentialed half — the CAS login on www.vhlcentral.com, the
// one-time `lt` ticket, the cookie jar that has to survive the host change to m3a.vhlcentral.com
// — and posts the page here (D11).
import { type Assignment, NotLoggedIn } from "./parse_zybooks.ts";
import { pyInt, pyStr, round2, slugify, truthy } from "./pyshims.ts";

export const MOUNT_MARKER = "js-student-dashboard-app";

const SUMMARIES = /data-assignment-summaries=(?:"([\s\S]*?)"|'([\s\S]*?)')/;
const SECTION = /\/sections\/(\d+)\//;
const DURATION = /^\s*(?:(\d+)\s*h)?\s*(?:(\d+)\s*m)?\s*$/i;

type Obj = Record<string, unknown>;

function obj(value: unknown): Obj {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Obj : {};
}

/**
 * `html_escape::decode_html_entities` over an attribute value.
 *
 * An HTML attribute can only carry the five XML entities and numeric references, and the captured
 * dashboard carries exactly one of them (`&quot;`, 490 times). An unrecognised named entity is
 * left as it was rather than guessed at, and the frozen reference is what proves this is enough
 * on the real capture.
 */
function decodeEntities(text: string): string {
  return text
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) => String.fromCodePoint(parseInt(hex, 16)))
    .replace(/&#(\d+);/g, (_, dec) => String.fromCodePoint(Number(dec)))
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

/** `"1h 22m"` -> 1.37. Throws on anything it cannot read: a zero-effort task fits any gap. */
export function parseDurationHours(text: string): number {
  const m = DURATION.exec(text);
  const unreadable = `unparseable duration: '${text}'`;
  if (m === null) throw new Error(unreadable);
  const hours = m[1] === undefined ? null : Number(m[1]);
  const minutes = m[2] === undefined ? null : Number(m[2]);
  // Both groups are optional, so the pattern also matches the empty string.
  if (hours === null && minutes === null) throw new Error(unreadable);
  return round2((hours ?? 0) + (minutes ?? 0) / 60);
}

/** `str(source.get(key) or "")`. */
function fieldStr(source: Obj, key: string): string {
  return truthy(source[key]) ? pyStr(source[key]) : "";
}

const WEEKDAY = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function parseDashboard(html: string, cfg: Obj, warnings: string[]): Assignment[] {
  if (!html.includes(MOUNT_MARKER)) {
    throw new NotLoggedIn("dashboard mount element absent (login page or redirect?)");
  }
  const captured = SUMMARIES.exec(html);
  if (captured === null) throw new NotLoggedIn("data-assignment-summaries attribute absent");
  const raw = decodeEntities(captured[1] ?? captured[2] ?? "");
  let summaries: unknown;
  try {
    summaries = JSON.parse(raw);
  } catch (e) {
    throw new NotLoggedIn(`data-assignment-summaries not valid JSON (${e instanceof Error ? e.message : e})`);
  }
  if (!Array.isArray(summaries)) {
    throw new NotLoggedIn(
      `data-assignment-summaries is ${summaries === null ? "NoneType" : typeof summaries}, not a list`,
    );
  }

  const sections = truthy(cfg.sections) ? obj(cfg.sections) : {};
  const importance = "importance" in cfg ? pyInt(cfg.importance) : 3;
  const importanceReason = truthy(cfg.importance_reason) ? pyStr(cfg.importance_reason) : "";

  const out: Assignment[] = [];
  for (const entry of summaries) {
    if (entry === null || typeof entry !== "object" || Array.isArray(entry)) {
      throw new Error("a data-assignment-summaries entry is not an object");
    }
    const item = entry as Obj;
    const rawDate = fieldStr(item, "due_date");
    const detail = fieldStr(item, "detail_url");
    const sectionMatch = SECTION.exec(detail);
    if (sectionMatch === null) {
      warnings.push(`${rawDate}: no section id in detail_url; skipped`);
      continue;
    }
    const sectionId = sectionMatch[1];
    const mapping = sections[sectionId];
    if (!truthy(mapping) || typeof mapping !== "object" || Array.isArray(mapping)) {
      warnings.push(`section ${sectionId} not in config; skipped`);
      continue;
    }
    const map = mapping as Obj;
    // VHL has only ever emitted YYYY-MM-DD; the narrower reading warns and skips where Python's
    // `date.fromisoformat` would also accept the basic form and ISO week dates.
    if (!/^\d{4}-\d{2}-\d{2}$/.test(rawDate)) {
      warnings.push(`unreadable due_date '${rawDate}'; skipped`);
      continue;
    }
    let remaining: number;
    try {
      remaining = parseDurationHours(fieldStr(item, "estimated_time"));
    } catch (e) {
      warnings.push(`${rawDate}: ${e instanceof Error ? e.message : e}; skipped`);
      continue;
    }

    const progress = pyInt(item.percentage_complete);
    // estimated_time covers what is LEFT; effort_hours is the whole bucket, because
    // `Task.remaining_hours` is effort_hours * (1 - progress/100).
    const effort = progress < 100 ? round2(remaining / (1 - progress / 100)) : remaining;
    const count = pyInt(item.assignment_count);
    const label = truthy(map.label) ? pyStr(map.label) : "Hausaufgaben";
    const course = truthy(map.course) ? pyStr(map.course) : null;

    // `%a` and `%m-%d` over the naive local date. Built from the parts rather than from a Date, so
    // no time zone can shift the weekday of a date that never had a time.
    const [y, mo, d] = rawDate.split("-").map(Number);
    const weekday = WEEKDAY[new Date(Date.UTC(y, mo - 1, d)).getUTCDay()];

    out.push({
      uid: `vhl:${sectionId}:${rawDate}`,
      slug: `${slugify(course ?? "task")}-hausaufgaben-${rawDate}`,
      title: `${label} — due ${weekday} ${String(mo).padStart(2, "0")}-${
        String(d).padStart(2, "0")
      } (${count} activities)`,
      due: `${rawDate}T23:59`,
      course,
      effort_hours: effort,
      effort_confidence: "high",
      effort_source: "vendor",
      importance,
      importance_reason: importanceReason,
      progress,
      created_by: "vhl",
      body:
        `${count} VHL activities, ${
          "activities_remaining" in item ? pyStr(item.activities_remaining) : "None"
        } outstanding.\n` +
        `VHL's own estimate for what remains: **${
          "estimated_time" in item ? pyStr(item.estimated_time) : "None"
        }**.\n\n` +
        `Detail: \`${detail}\``,
    });
  }
  return out;
}
