// The handful of Python semantics both moved parsers depend on. Named for what they are: this is
// a port of a port, and the frozen references are what say whether it is faithful.

/** Python's `round(x, 2)` — half to EVEN, matching `zybooks::round2` and `ranking::round2`. */
export function round2(x: number): number {
  const scaled = x * 100;
  const floor = Math.floor(scaled);
  const diff = scaled - floor;
  let n: number;
  if (diff > 0.5) n = floor + 1;
  else if (diff < 0.5) n = floor;
  else n = floor % 2 === 0 ? floor : floor + 1;
  return n / 100;
}

/** `ingest::slugify`: lowercase, collapse non-alphanumeric runs to `-`, trim, cap at 60 CHARACTERS, trim again. */
export function slugify(text: string): string {
  const dashed = text.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const trimmed = dashed.replace(/^-+/, "").replace(/-+$/, "");
  const capped = [...trimmed].slice(0, 60).join("").replace(/-+$/, "");
  return capped === "" ? "item" : capped;
}

/** Python's truthiness over a JSON value — `pystr::json_truthy`. */
export function truthy(value: unknown): boolean {
  if (value === null || value === undefined) return false;
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value !== 0;
  if (typeof value === "string") return value !== "";
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") return Object.keys(value).length > 0;
  return true;
}

/**
 * Python's `str(x)` over a JSON value — `pystr::json_str`. `None`, `True`, `False` spelled
 * Python's way. Python's `str(2.0)` is `"2.0"` — a whole float keeps its `.0` — but `JSON.parse`
 * collapses a source `2` and a source `2.0` into the same JavaScript `number`, so nothing built on
 * its output can tell which one a JSON literal was, and this function cannot reproduce that `.0`.
 * Every numeric field the frozen fixtures route through `pyStr` today is a bare integer; if a
 * future capture ever carried a whole-number float through a text field, the structural comparison
 * against the frozen reference is what would catch the drift.
 */
export function pyStr(value: unknown): string {
  if (value === null || value === undefined) return "None";
  if (value === true) return "True";
  if (value === false) return "False";
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  if (typeof value === "bigint") return String(value);
  return JSON.stringify(value);
}

/**
 * A JSON *float* as Python spells it. Used only where the source value is known to be a float in
 * Python's sense — `Assignment.effort_hours` on the device's side of the wire, never here.
 */
export function pyFloat(value: number): string {
  return Number.isInteger(value) ? `${value}.0` : String(value);
}

/** Python's `int(x or 0)` — `pystr::json_int`. Absent, null, false, 0 and "" are all 0. */
export function pyInt(value: unknown): number {
  if (!truthy(value)) return 0;
  if (value === true) return 1;
  if (typeof value === "number") return Math.trunc(value);
  if (typeof value === "string") {
    const n = Number(value.trim().replaceAll("_", ""));
    if (!Number.isFinite(n) || !Number.isInteger(n)) {
      throw new Error(`invalid literal for int() with base 10: '${value}'`);
    }
    return n;
  }
  throw new Error(`int() argument must be a number, not ${JSON.stringify(value)}`);
}

/**
 * Python's `str.strip()` — **whitespace only, both ends**.
 *
 * Python strips the ASCII whitespace set plus a few Unicode separators; JavaScript's `trim()`
 * strips its own `WhiteSpace` set plus line terminators, which is the same set for every character
 * either engine will meet in a course title. The class is written out rather than left implicit so
 * that the one place the two could diverge is visible — and note it does **not** strip a dash: a
 * title legitimately starts or ends with one, and stripping it would change a uid's slug.
 */
const PY_SPACE = /^\s+|\s+$/g;
export function strip(text: string): string {
  return text.replace(PY_SPACE, "");
}

/**
 * `datetime.strptime(raw, "%Y-%m-%dT%H:%M:%SZ")` in UTC, converted to `timeZone`, made naive, and
 * rendered as `ingest::format_due` renders it — `%Y-%m-%dT%H:%M`.
 *
 * `Intl.DateTimeFormat` carries the IANA database Deno already ships, so this needs no dependency.
 * The `hour12: false` "24" quirk is real: at local midnight some runtimes report hour `24`.
 */
export function dueLocal(raw: unknown, timeZone: string): string {
  if (typeof raw !== "string") {
    throw new Error(`strptime() argument 1 must be str, not ${typeof raw}`);
  }
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})Z$/.exec(raw);
  if (m === null) {
    throw new Error(`time data '${raw}' does not match format '%Y-%m-%dT%H:%M:%SZ'`);
  }
  const at = Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);
  const fmt = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
  const parts: Record<string, string> = {};
  for (const part of fmt.formatToParts(new Date(at))) parts[part.type] = part.value;
  const hour = parts.hour === "24" ? "00" : parts.hour;
  return `${parts.year}-${parts.month}-${parts.day}T${hour}:${parts.minute}`;
}
