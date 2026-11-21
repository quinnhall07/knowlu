// feed-shape: a read-only look at what a school calendar feed (iCalendar) carries.
//
// It reads a feed from --file or from ONE pasted line on stdin (never argv, so the link stays out
// of shell history and the process list), and prints the feed's shape: counts, property names, UID
// shapes, class counts, outside-site markers and title endings. It never writes a file, and never
// prints, logs or keeps the link. Without --show-titles no text from SUMMARY, DESCRIPTION, LOCATION,
// CATEGORIES or an X- value reaches the output.
//
// Node 20 or later, no dependencies. Usage: see README.md, or run with --help.
import { readFile } from "node:fs/promises";
import { createInterface } from "node:readline";
import { Writable } from "node:stream";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const TIMEOUT_MS = 20_000;
export const MAX_BYTES = 20 * 1024 * 1024;
const TOP_TIMES = 5;
const TOP_UID_SHAPES = 12;
const MAX_UNMATCHED_TITLES = 40;
const WINDOW_DAYS = 30;
const ZERO_ITEMS = "0 items: the link works but lists nothing";

// Each site: host fragments (matched anywhere, case-insensitive) and names (whole words,
// case-insensitive). A name match is a hint, not proof.
export const SITES = [
  { site: "WebAssign", hosts: ["webassign.net", "webassign.com"], names: ["webassign"] },
  { site: "Cengage / MindTap", hosts: ["cengage.com"], names: ["mindtap", "cengage"] },
  { site: "Pearson (MyLab, Mastering, Revel)", hosts: ["pearson.com", "pearsoncmg.com", "pearsoned.com"], names: ["mylab", "mastering", "pearson"] },
  { site: "McGraw Hill Connect", hosts: ["mheducation.com"], names: ["mcgraw"] },
  { site: "ALEKS", hosts: ["aleks.com"], names: ["aleks"] },
  { site: "Gradescope", hosts: ["gradescope.com", "gradescope.ca"], names: ["gradescope"] },
  { site: "Ed", hosts: ["edstem.org"], names: ["edstem"] },
  { site: "Aktiv", hosts: ["aktiv.com", "101edu.co"], names: ["aktiv", "chem101"] },
  { site: "Expert TA", hosts: ["theexpertta.com"], names: ["expertta"] },
  { site: "Edfinity", hosts: ["edfinity.com"], names: ["edfinity"] },
  { site: "Smartwork (Norton)", hosts: ["wwnorton.com"], names: ["smartwork", "norton"] },
  { site: "zyBooks", hosts: ["zybooks.com"], names: ["zybooks", "zybook"] },
  { site: "VHL", hosts: ["vhlcentral.com"], names: ["vhl", "vhlcentral"] },
];

export const ENDING_PHRASES = [
  "due", "available", "availability starts", "availability ends", "opens", "closes",
  "starts", "ends", "unlocks", "locks",
];

const MARKER_FIELDS = ["SUMMARY", "DESCRIPTION", "URL", "LOCATION"];

// ---------------------------------------------------------------------------------------------
// Parsing

function unescapeText(raw) {
  return raw.replace(/\\([nN,;\\])/g, (_, c) => (c === "n" || c === "N" ? "\n" : c));
}

// Split a TEXT list (CATEGORIES) on unescaped commas, then unescape each part.
function splitTextList(raw) {
  const parts = [];
  let cur = "";
  for (let i = 0; i < raw.length; i++) {
    const c = raw[i];
    if (c === "\\" && i + 1 < raw.length) {
      cur += c + raw[i + 1];
      i++;
    } else if (c === ",") {
      parts.push(cur);
      cur = "";
    } else {
      cur += c;
    }
  }
  parts.push(cur);
  return parts.map((p) => unescapeText(p).trim()).filter((p) => p !== "");
}

// A property name is an iana-token or x-name (RFC 5545 3.1). A line whose "name" is anything else
// (for example the rest of a text value after an unescaped line break) is not a property line: it
// is counted, and its text is never kept.
const PROPERTY_NAME = /^[A-Za-z0-9-]+$/;
const NOT_A_PROPERTY = { notProperty: true };
export const NOT_A_PROPERTY_LABEL = "(not a property line)";

function parseLine(line) {
  const n = line.length;
  let i = 0;
  while (i < n && line[i] !== ";" && line[i] !== ":") i++;
  if (i >= n) return NOT_A_PROPERTY;
  const rawName = line.slice(0, i).trim();
  if (!PROPERTY_NAME.test(rawName)) return NOT_A_PROPERTY;
  const name = rawName.toUpperCase();
  const params = {};
  while (line[i] === ";") {
    i++;
    let j = i;
    while (j < n && line[j] !== "=" && line[j] !== ":" && line[j] !== ";") j++;
    const pname = line.slice(i, j).trim().toUpperCase();
    const vals = [];
    if (line[j] === "=") {
      i = j + 1;
      for (;;) {
        if (line[i] === '"') {
          const k = line.indexOf('"', i + 1);
          if (k < 0) {
            vals.push(line.slice(i + 1));
            i = n;
          } else {
            vals.push(line.slice(i + 1, k));
            i = k + 1;
          }
        } else {
          let k = i;
          while (k < n && line[k] !== "," && line[k] !== ";" && line[k] !== ":") k++;
          vals.push(line.slice(i, k));
          i = k;
        }
        if (line[i] === ",") {
          i++;
          continue;
        }
        break;
      }
    } else {
      i = j;
    }
    if (pname) params[pname] = vals.join(",");
  }
  if (line[i] !== ":") return NOT_A_PROPERTY;
  const raw = line.slice(i + 1);
  return { name, params, raw, value: unescapeText(raw) };
}

// Returns { isCalendar, items }, items being the VEVENT and VTODO components in input order, each
// { type, props: [{ name, params, raw, value }], notProperty }, notProperty counting the lines in it
// that are not property lines. Properties of nested components (VALARM) are not the item's.
export function parse(text) {
  const physical = String(text).replace(/^\uFEFF/, "").split(/\r\n|\n|\r/);
  const lines = [];
  for (const l of physical) {
    if ((l.startsWith(" ") || l.startsWith("\t")) && lines.length > 0) lines[lines.length - 1] += l.slice(1);
    else lines.push(l);
  }
  let isCalendar = false;
  const items = [];
  const stack = [];
  for (const line of lines) {
    if (line.trim() === "") continue;
    const p = parseLine(line);
    if (p.notProperty) {
      if (stack.length > 0) stack[stack.length - 1].notProperty++;
      continue;
    }
    if (p.name === "BEGIN") {
      const type = p.value.trim().toUpperCase();
      if (type === "VCALENDAR") isCalendar = true;
      stack.push({ type, props: [], notProperty: 0 });
    } else if (p.name === "END") {
      const type = p.value.trim().toUpperCase();
      let idx = stack.length - 1;
      while (idx >= 0 && stack[idx].type !== type) idx--;
      if (idx < 0) continue;
      const c = stack[idx];
      stack.length = idx;
      if (c.type === "VEVENT" || c.type === "VTODO") items.push(c);
    } else if (stack.length > 0) {
      stack[stack.length - 1].props.push(p);
    }
  }
  return { isCalendar, items };
}

// ---------------------------------------------------------------------------------------------
// Summarising

const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

function first(item, name) {
  return item.props.find((p) => p.name === name);
}

function dateOf(prop) {
  const m = prop && /^(\d{4})(\d{2})(\d{2})/.exec(prop.value.trim());
  return m ? `${m[1]}-${m[2]}-${m[3]}` : null;
}

function isAllDay(prop) {
  return String(prop.params.VALUE || "").toUpperCase() === "DATE" || /^\d{8}$/.test(prop.value.trim());
}

function addDays(ymd, days) {
  const [y, m, d] = ymd.split("-").map(Number);
  const t = new Date(Date.UTC(y, m - 1, d) + days * 86_400_000);
  const pad = (x) => String(x).padStart(2, "0");
  return `${t.getUTCFullYear()}-${pad(t.getUTCMonth() + 1)}-${pad(t.getUTCDate())}`;
}

// The tool's own structural words: the only words a UID shape keeps (compared case-insensitively,
// kept as written). Every other run of letters becomes <w>.
export const UID_WORDS = new Set([
  "assignment", "blackboard", "brightspace", "calendar", "canvas", "course", "d2l", "discussion",
  "due", "event", "gradableitem", "gradebook", "ics", "instructure", "item", "lms", "moodle",
  "platform", "quiz", "todo", "uid",
]);
const PLAIN_HOST = /^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?)+$/;

function maskUidPart(s) {
  return s
    .replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, "\u0000")
    .replace(/[\p{L}\p{N}]+/gu, (tok) => (UID_WORDS.has(tok.toLowerCase())
      ? tok
      : tok
        .replace(/[0-9a-f]{8,}/gi, (m) => (/^\d+$/.test(m) ? m : "\u0000"))
        .replace(/\p{N}+/gu, "#")
        .replace(/\p{L}+/gu, (w) => (UID_WORDS.has(w.toLowerCase()) ? w : "<w>"))))
    .replace(/\u0000/g, "<hex>");
}

// UUIDs and runs of 8+ hex characters become <hex>, digit runs #, and other words <w> unless they
// are in UID_WORDS. Punctuation stays. The part after the last "@" is kept as written when it is a
// plain host name, and masked like the rest when it is not.
export function uidShape(uid) {
  const at = uid.lastIndexOf("@");
  if (at < 0) return maskUidPart(uid);
  const host = uid.slice(at + 1);
  return `${maskUidPart(uid.slice(0, at))}@${PLAIN_HOST.test(host) ? host : maskUidPart(host)}`;
}

function countsSorted(map, keyName) {
  return [...map.entries()]
    .sort((a, b) => b[1] - a[1] || cmp(a[0], b[0]))
    .map(([k, count]) => ({ [keyName]: k, count }));
}

function bump(map, key, by = 1) {
  map.set(key, (map.get(key) || 0) + by);
}

function oneLine(s) {
  return s.replace(/[\u0000-\u001f\u007f]+/g, " ").replace(/\s+/g, " ").trim();
}

const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

// A host fragment matches only at a host boundary: not preceded or followed by a letter, digit,
// "_" or "-" (so "praktiv.com" is not "aktiv.com"). A name matches only as a whole word, any
// Unicode letter or digit being a word character (so "émastering" is not "mastering").
const SITE_MATCHERS = SITES.map((s) => ({
  site: s.site,
  hosts: s.hosts.map((h) => new RegExp(`(?<![\\p{L}\\p{N}_-])${escapeRe(h)}(?![\\p{L}\\p{N}_-])`, "iu")),
  names: s.names.map((n) => new RegExp(`(?<![\\p{L}\\p{N}_])${escapeRe(n)}(?![\\p{L}\\p{N}_])`, "iu")),
}));

function siteMatches(m, text) {
  return m.hosts.some((re) => re.test(text)) || m.names.some((re) => re.test(text));
}

function fieldOrder(a, b) {
  const ia = MARKER_FIELDS.indexOf(a);
  const ib = MARKER_FIELDS.indexOf(b);
  if (ia >= 0 || ib >= 0) return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib);
  return cmp(a, b);
}

function ending(summary) {
  const d = summary.lastIndexOf(" - ");
  const c = summary.lastIndexOf(": ");
  if (d < 0 && c < 0) return null;
  const tail = d > c ? summary.slice(d + 3) : summary.slice(c + 2);
  return oneLine(tail).toLowerCase();
}

// summarise(calendar, { now: "YYYY-MM-DD", showTitles }) -> a plain object. Without showTitles it
// holds no text from SUMMARY, DESCRIPTION, LOCATION, CATEGORIES or X- values.
export function summarise(cal, { now, showTitles = false } = {}) {
  const items = cal.items;
  const windowEnd = addDays(now, WINDOW_DAYS - 1);
  const later = addDays(now, WINDOW_DAYS);

  const totals = {
    vevent: 0, vtodo: 0, earliest: null, latest: null, allDay: 0, timed: 0, noDate: 0,
    timeForms: { utc: 0, tzid: 0, floating: 0 }, topTimes: [], dtendDiffers: 0,
    now, windowEnd, next30: 0, before: 0, later: 0,
  };
  const times = new Map();
  const propCounts = new Map();
  const shapes = new Map();
  const classCounts = new Map();
  let hasCategories = false;
  const sites = SITE_MATCHERS.map((m) => ({ m, items: 0, fields: new Map(), titles: [] }));
  const endingCounts = new Map(ENDING_PHRASES.map((p) => [p, 0]));
  let noEnding = 0;
  const unmatched = [];

  for (const item of items) {
    if (item.type === "VEVENT") totals.vevent++;
    else totals.vtodo++;

    for (const p of item.props) {
      if (p.name === "DTSTART" || p.name === "DUE") {
        const d = dateOf(p);
        if (d && (totals.earliest === null || d < totals.earliest)) totals.earliest = d;
        if (d && (totals.latest === null || d > totals.latest)) totals.latest = d;
      }
    }

    const primary = first(item, "DTSTART") || first(item, "DUE");
    const pdate = dateOf(primary);
    if (!primary || !pdate) {
      totals.noDate++;
    } else {
      if (isAllDay(primary)) {
        totals.allDay++;
      } else {
        totals.timed++;
        const v = primary.value.trim();
        const t = /T(\d{2})(\d{2})/.exec(v);
        if (t) bump(times, `${t[1]}:${t[2]}`);
        if (/z$/i.test(v)) totals.timeForms.utc++;
        else if (primary.params.TZID) totals.timeForms.tzid++;
        else totals.timeForms.floating++;
      }
      if (pdate < now) totals.before++;
      else if (pdate < later) totals.next30++;
      else totals.later++;
    }
    const start = first(item, "DTSTART");
    const end = first(item, "DTEND");
    if (end && (!start || end.value.trim() !== start.value.trim())) totals.dtendDiffers++;

    for (const name of new Set(item.props.map((p) => p.name))) bump(propCounts, name);
    if (item.notProperty > 0) bump(propCounts, NOT_A_PROPERTY_LABEL);

    const uid = first(item, "UID");
    bump(shapes, uid ? uidShape(uid.value.trim()) : "(no UID)");

    const labels = new Set();
    for (const p of item.props) {
      if (p.name !== "CATEGORIES") continue;
      hasCategories = true;
      for (const l of splitTextList(p.raw)) labels.add(l);
    }
    for (const l of labels) bump(classCounts, l);

    const summaryProp = first(item, "SUMMARY");
    const summary = summaryProp ? summaryProp.value : "";
    const markerProps = item.props.filter((p) => MARKER_FIELDS.includes(p.name) || p.name.startsWith("X-"));
    let matchedAny = false;
    for (const s of sites) {
      const hitFields = new Set();
      for (const p of markerProps) if (siteMatches(s.m, p.value)) hitFields.add(p.name);
      if (hitFields.size === 0) continue;
      matchedAny = true;
      s.items++;
      for (const f of hitFields) bump(s.fields, f);
      s.titles.push(oneLine(summary));
    }
    if (!matchedAny && unmatched.length < MAX_UNMATCHED_TITLES) unmatched.push(oneLine(summary));

    const e = summaryProp ? ending(summary) : null;
    if (e !== null && endingCounts.has(e)) bump(endingCounts, e);
    else noEnding++;
  }

  totals.topTimes = countsSorted(times, "time").slice(0, TOP_TIMES);

  const classList = countsSorted(classCounts, "name").map((c, i) => {
    const o = { label: `class ${i + 1}`, count: c.count };
    if (showTitles) o.name = c.name;
    return o;
  });

  const out = {
    tool: "feed-shape",
    items: items.length,
    titlesShown: Boolean(showTitles),
    totals,
    properties: [...propCounts.entries()].sort((a, b) => cmp(a[0], b[0])).map(([name, count]) => ({ name, count })),
    uidShapes: countsSorted(shapes, "shape").slice(0, TOP_UID_SHAPES),
    classes: { property: hasCategories, distinct: classCounts.size, list: classList },
    sites: sites.map((s) => {
      const o = {
        site: s.m.site,
        items: s.items,
        fields: [...s.fields.entries()].sort((a, b) => fieldOrder(a[0], b[0])).map(([field, count]) => ({ field, count })),
      };
      if (showTitles) o.titles = s.titles;
      return o;
    }),
    endings: { phrases: ENDING_PHRASES.map((phrase) => ({ phrase, count: endingCounts.get(phrase) })), none: noEnding },
  };
  if (showTitles) out.unmatchedTitles = unmatched;
  if (items.length === 0) out.note = ZERO_ITEMS;
  return out;
}

// ---------------------------------------------------------------------------------------------
// Rendering

export function renderText(s) {
  if (s.items === 0) return `${ZERO_ITEMS}\n`;
  const t = s.totals;
  const L = [];
  L.push("feed-shape: what this calendar feed carries");
  L.push(s.titlesShown
    ? "Item titles ARE shown (--show-titles): for eyes at the sitting only; do not save or share this output."
    : "No item titles, descriptions, locations or class names are shown.");
  L.push("");
  L.push("== Totals ==");
  L.push(`items: ${s.items} (VEVENT ${t.vevent}, VTODO ${t.vtodo})`);
  L.push(t.earliest ? `dates (DTSTART, DUE): ${t.earliest} to ${t.latest}` : "dates (DTSTART, DUE): none");
  L.push(`all-day: ${t.allDay}`);
  L.push(`timed: ${t.timed}`);
  L.push(`no date: ${t.noDate}`);
  L.push(`timed values: ${t.timeForms.utc} end in Z, ${t.timeForms.tzid} carry a TZID, ${t.timeForms.floating} floating`);
  L.push(`most common times of day, as written (top ${TOP_TIMES}):`);
  if (t.topTimes.length === 0) L.push("  none");
  for (const x of t.topTimes) L.push(`  ${x.time}  ${x.count}`);
  L.push(`DTEND different from DTSTART: ${t.dtendDiffers}`);
  L.push(`in the 30 days from ${t.now} (to ${t.windowEnd}): ${t.next30}`);
  L.push(`before ${t.now}: ${t.before}`);
  L.push(`later than ${t.windowEnd}: ${t.later}`);
  L.push("");
  L.push("== Properties on items (items carrying each) ==");
  for (const p of s.properties) L.push(`${p.name}  ${p.count}`);
  L.push("");
  L.push(`== UID shapes (digits as #, hex runs as <hex>, other words as <w>; top ${TOP_UID_SHAPES}) ==`);
  for (const u of s.uidShapes) L.push(`${u.count}  ${u.shape}`);
  L.push("");
  L.push("== Classes (CATEGORIES) ==");
  if (!s.classes.property) {
    L.push("no CATEGORIES property");
  } else {
    L.push(`distinct: ${s.classes.distinct}`);
    for (const c of s.classes.list) L.push(s.titlesShown ? `${c.label}: ${c.count}  ${oneLine(c.name)}` : `${c.label}: ${c.count}`);
  }
  L.push("");
  L.push("== Outside-site markers (items whose SUMMARY, DESCRIPTION, URL, LOCATION or X- value matched) ==");
  L.push("A name match is a hint, not proof. 0 here does not prove a site is absent: its items may carry no marker.");
  for (const site of s.sites) {
    L.push(`${site.site}: ${site.items}`);
    for (const f of site.fields) L.push(`  ${f.field}: ${f.count}`);
    if (s.titlesShown) for (const title of site.titles) L.push(`    - ${title}`);
  }
  L.push("");
  L.push('== Title endings (SUMMARY after its last " - " or ": ") ==');
  for (const e of s.endings.phrases) L.push(`${e.phrase}: ${e.count}`);
  L.push(`no marker ending: ${s.endings.none}`);
  if (s.titlesShown) {
    L.push("");
    L.push(`== Titles of items that matched no site (up to ${MAX_UNMATCHED_TITLES}) ==`);
    if (s.unmatchedTitles.length === 0) L.push("none");
    for (const title of s.unmatchedTitles) L.push(`- ${title}`);
  }
  return `${L.join("\n")}\n`;
}

export function renderJson(s) {
  return `${JSON.stringify(s, null, 2)}\n`;
}

// ---------------------------------------------------------------------------------------------
// Input: the link and the fetch. No message here ever carries the link or any part of it.

export function normaliseLink(line) {
  const s = String(line ?? "").trim();
  if (s === "") return { ok: false, error: "refused: no link was pasted" };
  let url = s;
  if (/^webcal:\/\//i.test(url)) url = `https://${url.slice("webcal://".length)}`;
  if (!/^https:\/\//i.test(url)) return { ok: false, error: "refused: only https:// or webcal:// links are fetched" };
  try {
    new URL(url);
  } catch {
    return { ok: false, error: "refused: that is not a valid link" };
  }
  return { ok: true, url };
}

function sizeLabel(bytes) {
  return bytes % (1024 * 1024) === 0 ? `${bytes / (1024 * 1024)} MB` : `${bytes} bytes`;
}

function failureKind(e, timeoutMs) {
  if (e && (e.name === "TimeoutError" || e.name === "AbortError")) return `timed out after ${timeoutMs / 1000} s`;
  const code = (e && e.cause && e.cause.code) || (e && e.code);
  if (typeof code === "string" && /^[A-Z][A-Z0-9_]*$/.test(code)) return `network error (${code})`;
  return "network error";
}

export const MAX_REDIRECTS = 5;
const REDIRECT_STATUSES = new Set([301, 302, 303, 307, 308]);

// Redirects are followed by hand, at most MAX_REDIRECTS of them, and only to https: a redirect to
// any other scheme is refused, so the link and the feed never cross the network in cleartext. The
// one timeout covers the whole chain and the body. A refused redirect is a failed fetch (exit 1).
export async function fetchFeed(url, fetchFn, { timeoutMs = TIMEOUT_MS, maxBytes = MAX_BYTES } = {}) {
  const tooLarge = { ok: false, kind: `the feed is too large (over ${sizeLabel(maxBytes)})` };
  const signal = AbortSignal.timeout(timeoutMs);
  let current = url;
  let res;
  for (let hops = 0; ; hops++) {
    try {
      res = await fetchFn(current, { signal, redirect: "manual", headers: { accept: "text/calendar, */*" } });
    } catch (e) {
      return { ok: false, kind: failureKind(e, timeoutMs) };
    }
    if (!REDIRECT_STATUSES.has(res.status)) break;
    try {
      if (res.body && res.body.cancel) res.body.cancel().catch(() => {});
    } catch {
      // the redirect's body is not needed
    }
    if (hops >= MAX_REDIRECTS) return { ok: false, kind: `too many redirects (over ${MAX_REDIRECTS})` };
    const location = res.headers && res.headers.get ? res.headers.get("location") : null;
    if (!location) return { ok: false, kind: "a redirect gave no location" };
    let next;
    try {
      next = new URL(location, current);
    } catch {
      return { ok: false, kind: "a redirect gave a location that cannot be read" };
    }
    if (next.protocol !== "https:") return { ok: false, kind: "refused a redirect to a non-https address" };
    current = next.href;
  }
  if (!res.ok) return { ok: false, kind: `HTTP ${res.status}` };
  const declared = Number(res.headers && res.headers.get ? res.headers.get("content-length") : NaN);
  if (Number.isFinite(declared) && declared > maxBytes) return tooLarge;
  try {
    const chunks = [];
    let total = 0;
    if (res.body && res.body.getReader) {
      const reader = res.body.getReader();
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        total += value.byteLength;
        if (total > maxBytes) {
          await reader.cancel().catch(() => {});
          return tooLarge;
        }
        chunks.push(value);
      }
    }
    const bytes = new Uint8Array(total);
    let at = 0;
    for (const c of chunks) {
      bytes.set(c, at);
      at += c.byteLength;
    }
    return { ok: true, text: new TextDecoder("utf-8").decode(bytes) };
  } catch (e) {
    return { ok: false, kind: failureKind(e, timeoutMs) };
  }
}

// ---------------------------------------------------------------------------------------------
// Command line

const USAGE = `feed-shape: a read-only look at what a school calendar feed carries.

  node scripts/site-check/feed-shape.mjs [--file <path>] [--now YYYY-MM-DD] [--show-titles] [--json]

With no --file it asks for the link on stdin (paste, then Enter). The link is never printed or kept.
  --file <path>    read an .ics file instead of fetching
  --now <date>     the day the before/after counts use (default: today)
  --show-titles    also show item titles and class names: for eyes at the sitting only
  --json           print the summary as one JSON object
Never put the link on the command line: it would land in your shell history, and it is refused.
Exit codes: 0 report printed, 2 bad input or refused link, 1 fetch failed (a refused redirect included).
`;

const LINK_ON_COMMAND_LINE =
  "that looks like a link, so it was not used and is not shown. A link typed on the command line is now in " +
  "your shell history: reset the feed link in your school's site, then run this again with no link and " +
  "paste the new one when asked";

function parseArgs(argv) {
  const o = { file: null, now: null, showTitles: false, json: false, help: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--file") {
      if (i + 1 >= argv.length) return { error: "--file needs a path" };
      o.file = argv[++i];
    } else if (a === "--now") {
      const v = argv[++i];
      if (!v || !/^\d{4}-\d{2}-\d{2}$/.test(v) || addDays(v, 0) !== v) return { error: "--now needs a date as YYYY-MM-DD" };
      o.now = v;
    } else if (a === "--show-titles") {
      o.showTitles = true;
    } else if (a === "--json") {
      o.json = true;
    } else if (a === "--help" || a === "-h") {
      o.help = true;
    } else {
      // Never echo it: a pasted link may have landed here.
      return { error: "unknown argument (not shown, in case it is a link); see --help" };
    }
  }
  return o;
}

// main(argv, io) -> exit code. io: { stdout(s), stderr(s), readLine(), readFile(path), fetch,
// today(), timeoutMs?, maxBytes? }.
export async function main(argv, io) {
  if (argv.some((a) => /^(https?|webcal):/i.test(a))) {
    io.stderr(`feed-shape: ${LINK_ON_COMMAND_LINE}\n`);
    return 2;
  }
  const o = parseArgs(argv);
  if (o.error) {
    io.stderr(`feed-shape: ${o.error}\n`);
    return 2;
  }
  if (o.help) {
    io.stdout(USAGE);
    return 0;
  }
  let text;
  if (o.file !== null) {
    let buf;
    try {
      buf = await io.readFile(o.file);
    } catch (e) {
      io.stderr(`feed-shape: cannot read the file (${(e && e.code) || "error"})\n`);
      return 2;
    }
    text = typeof buf === "string" ? buf : new TextDecoder("utf-8").decode(buf);
  } else {
    io.stderr("Paste the calendar link, then Enter: ");
    const line = await io.readLine();
    io.stderr("\n");
    const link = normaliseLink(line);
    if (!link.ok) {
      io.stderr(`feed-shape: ${link.error}\n`);
      return 2;
    }
    const got = await fetchFeed(link.url, io.fetch, {
      timeoutMs: io.timeoutMs ?? TIMEOUT_MS,
      maxBytes: io.maxBytes ?? MAX_BYTES,
    });
    if (!got.ok) {
      io.stderr(`feed-shape: fetch failed: ${got.kind}\n`);
      return 1;
    }
    text = got.text;
  }
  const cal = parse(text);
  if (!cal.isCalendar) {
    io.stderr("feed-shape: not a calendar (no BEGIN:VCALENDAR); a sign-in page or a wrong link looks like this\n");
    return 2;
  }
  const summary = summarise(cal, { now: o.now ?? io.today(), showTitles: o.showTitles });
  io.stdout(o.json ? renderJson(summary) : renderText(summary));
  return 0;
}

// One line from stdin. On a terminal, typed or pasted characters are not echoed.
function readOneLine() {
  return new Promise((resolve) => {
    const tty = Boolean(process.stdin.isTTY);
    const muted = new Writable({ write(_chunk, _enc, cb) { cb(); } });
    const rl = createInterface({ input: process.stdin, output: tty ? muted : undefined, terminal: tty });
    let done = false;
    const finish = (v) => {
      if (done) return;
      done = true;
      rl.close();
      resolve(v);
    };
    rl.once("line", (l) => finish(l));
    rl.once("SIGINT", () => finish(null));
    rl.once("close", () => finish(null));
  });
}

function localToday() {
  const d = new Date();
  const pad = (x) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function isMainModule() {
  if (!process.argv[1]) return false;
  const a = path.resolve(process.argv[1]);
  const b = fileURLToPath(import.meta.url);
  return process.platform === "win32" ? a.toLowerCase() === b.toLowerCase() : a === b;
}

if (isMainModule()) {
  main(process.argv.slice(2), {
    stdout: (s) => process.stdout.write(s),
    stderr: (s) => process.stderr.write(s),
    readLine: readOneLine,
    readFile: (p) => readFile(p),
    fetch: (url, init) => globalThis.fetch(url, init),
    today: localToday,
  }).then(
    (code) => { process.exitCode = code; },
    (e) => {
      process.stderr.write(`feed-shape: internal error (${(e && e.name) || "error"})\n`);
      process.exitCode = 1;
    },
  );
}
