// The deterministic email `due` resolver (stream J Task T4).
//
// The 2026-09-16 scoping note called the old prompt rule — "resolve this relative deadline
// yourself" — the single most dangerous line in the email prompt: a mis-resolved "by Friday"
// comes back as a well-formed, entirely wrong `YYYY-MM-DD`, and `judge_validate.ts` has no way to
// tell a wrong date from a right one. The corroborated fix (independently found this week, and
// consistent with every vendor's own guidance for this model class) is to stop asking the model to
// do arithmetic at all: the model now does EXTRACTION ONLY (`judge_prompts.ts`'s email due bullet),
// returning the deadline exactly as the email states it — a phrase ("Friday", "the end of the
// month", "tomorrow at 5pm") or, only when the email itself gives an explicit calendar date, that
// date. This module is the "ordinary code" that does the arithmetic instead.
//
// **Ambiguity resolves to null, never a guess.** A wrong date silently written into a student's
// vault is the one failure this module exists to prevent, so every branch below that is not sure
// returns `null` rather than its best guess — and that includes a phrase that names a SPAN of days
// rather than one of them: "next week" is seven candidate days, and picking one (fix round 1,
// finding 1: this module used to pick today+7) is exactly the kind of guess this module exists to
// refuse. Only a phrase this module can place on exactly one calendar day is ever resolved —
// "today", "tonight", "tomorrow", a named weekday (bare or "this <weekday>"), and "end of the
// month" (which names the month's own last day, a single day, not a range) qualify; "next week"
// and "next month" do not, and neither does "next <weekday>" (final review item 6): "next Friday"
// said on a Monday means that Friday to some readers and the one after to others -- two candidate
// days, one guess.
//
// **Explicit dates in long form (due-fix, 2026-09-23).** Scoring the pinned email model against a
// real labelled set showed it often copies the email's own date verbatim —
// "Friday, September 18, 2026 11:59:00 PM CDT" — and this module used to return null for it (6 of
// 50 real task emails lost their deadline). `parseLongForm` accepts `[Weekday, ]Month D, YYYY`
// optionally followed by `[at ]H:MM[:SS][ AM|PM][ ZONE]`, with ZONE one of the US abbreviations
// in `NAMED_ZONES` (plus UTC/GMT). A named zone is converted to the instant it names and expressed
// on the student's clock (below). An unknown abbreviation (including an ambiguous non-US one such
// as BST or IST), a weekday that names a different day than the date, a year left out, or a date
// or time that does not exist is null — never a guess.
//
// **Which clock: the student's timezone, from the request (due-fix, 2026-09-23).** A relative
// phrase means a day on the STUDENT's calendar, not on the clock of whatever server stamped the
// email. This module used to resolve against the Date header's own wall clock, and mail a server
// stamps in UTC (common for LMS notifications, and the form the collector recorded) carries the
// UTC calendar date: a 23:30 CDT email's "tomorrow" came out a day late. The timezone sources the
// pipeline has, most reliable first:
//   1. the vault's own `config/ingest.yaml` `timezone` (an IANA name, the zone the whole engine
//      already treats as the student's clock — `cli.rs`'s `vault_zone`). The device sends it on
//      the request as `timezone`, the same field `/ingest-coursework` already takes, and
//      `judge_pipeline.ts` hands it here. This is the one that is used whenever it is present and
//      names a zone `Intl` knows;
//   2. the Date header's own UTC offset — the sender's clock, right for a student who mails
//      from their own zone and wrong for a UTC-stamping server. Used only when no usable
//      `timezone` arrived (an engine older than this fix, or a vault without one);
//   there is no timezone on the account row, and none elsewhere in the request.
// With a timezone: the header is converted to the instant it names (its numeric offset, or one of
// the RFC 5322 named zones), and that instant's local date and time in the timezone is the
// reference. A header whose offset cannot be known (no zone, or an unknown one) stays on its own
// wall clock, the only clock it gives. Once the reference date is fixed, the arithmetic is pure
// day/month/year arithmetic — a resolution that crosses a DST boundary re-derives no offset.
//
// The output format is unchanged: `YYYY-MM-DD`, or `YYYY-MM-DDTHH:MM` on the student's clock
// (naive, as every due in the vault is).
import { ABSOLUTE_DUE_RE } from "./judge_validate.ts";

/** A Date header's own local wall-clock date and time — the offset, if any, is not part of it
 * (`parseDateLine`); `resolveDue` reads it separately to place the header on the student's clock. */
export interface ReferenceDate {
  year: number;
  month: number; // 1-12
  day: number; // 1-31
  hour: number; // 0-23
  minute: number; // 0-59
}

const MONTHS = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
const WEEKDAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const DAYS_IN_MONTH = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/**
 * UTC offsets in minutes for the zone names this module will convert: RFC 5322's own named zones
 * (UT/GMT/Z and the eight obsolete US names) plus UTC. Deliberately US-only past UTC: a student's
 * mail names these, and the common non-US abbreviations are ambiguous (IST is India, Israel or
 * Ireland; BST is British Summer or Bangladesh), so any other name is unknown and resolves to
 * null rather than to a guessed offset.
 */
const NAMED_ZONES: Record<string, number> = {
  UT: 0,
  UTC: 0,
  GMT: 0,
  Z: 0,
  EDT: -240,
  EST: -300,
  CDT: -300,
  CST: -360,
  MDT: -360,
  MST: -420,
  PDT: -420,
  PST: -480,
};

function isLeapYear(year: number): boolean {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

function daysInMonth(year: number, month1: number): number {
  if (month1 === 2 && isLeapYear(year)) return 29;
  return DAYS_IN_MONTH[month1 - 1];
}

function validDate(year: number, month: number, day: number): boolean {
  return month >= 1 && month <= 12 && day >= 1 && day <= daysInMonth(year, month);
}

/** A parsed Date line: its own wall clock, and its UTC offset in minutes when the line names one
 * this module knows (`null` for no zone, or a zone name outside `NAMED_ZONES`). */
interface Stamp {
  local: ReferenceDate;
  offsetMinutes: number | null;
}

function numericOffset(sign: string, hh: string, mm: string): number {
  const minutes = Number(hh) * 60 + Number(mm);
  return sign === "-" ? -minutes : minutes;
}

function parseStamp(dateLine: string): Stamp | null {
  const s = dateLine.trim();
  const rfc =
    /^(?:[A-Za-z]{3},\s*)?(\d{1,2})\s+([A-Za-z]{3})[A-Za-z]*\s+(\d{2,4})\s+(\d{1,2}):(\d{2})(?::\d{2})?\s*([+-]\d{4}|[A-Za-z]{1,5})?\s*$/;
  const m1 = rfc.exec(s);
  if (m1) {
    const day = Number(m1[1]);
    const monthIdx = MONTHS.indexOf(m1[2].toLowerCase());
    if (monthIdx === -1) return null;
    let year = Number(m1[3]);
    if (m1[3].length <= 2) year += year < 50 ? 2000 : 1900;
    const hour = Number(m1[4]);
    const minute = Number(m1[5]);
    if (!validDate(year, monthIdx + 1, day) || hour > 23 || minute > 59) return null;
    const zone = m1[6];
    let offsetMinutes: number | null = null;
    if (zone !== undefined) {
      const numeric = /^([+-])(\d{2})(\d{2})$/.exec(zone);
      offsetMinutes = numeric
        ? numericOffset(numeric[1], numeric[2], numeric[3])
        : (NAMED_ZONES[zone.toUpperCase()] ?? null);
    }
    return { local: { year, month: monthIdx + 1, day, hour, minute }, offsetMinutes };
  }

  const iso = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::\d{2})?)?(Z|([+-])(\d{2}):?(\d{2}))?\s*$/;
  const m2 = iso.exec(s);
  if (m2) {
    const year = Number(m2[1]);
    const month = Number(m2[2]);
    const day = Number(m2[3]);
    const hour = m2[4] ? Number(m2[4]) : 0;
    const minute = m2[5] ? Number(m2[5]) : 0;
    if (!validDate(year, month, day) || hour > 23 || minute > 59) return null;
    const offsetMinutes = m2[6] === undefined ? null : m2[6] === "Z" ? 0 : numericOffset(m2[7], m2[8], m2[9]);
    return { local: { year, month, day, hour, minute }, offsetMinutes };
  }

  return null;
}

/**
 * Parses an RFC 5322 `Date:` header — `"[Day, ]DD Mon YYYY HH:MM[:SS] <zone>"`, the zone being a
 * numeric `[+-]HHMM`, `UT`/`GMT`/`Z`, or one of the obsolete US zone names — or, defensively, a
 * plain ISO 8601 stamp (`YYYY-MM-DD[THH:MM[:SS]][zone]`), into the header's own wall clock. The
 * zone is matched so it is never mistaken for part of the date or time; `resolveDue` reads it
 * separately. Returns `null` for anything else, or for a date that is not a real calendar day
 * (`31 Feb`, an hour past 23, a minute past 59) — a reference this module cannot trust is a
 * phrase it cannot resolve.
 */
export function parseDateLine(dateLine: string): ReferenceDate | null {
  return parseStamp(dateLine)?.local ?? null;
}

interface CalendarDay {
  year: number;
  month: number;
  day: number;
}

interface WallClock extends CalendarDay {
  hour: number;
  minute: number;
}

/** `0` (Sunday) to `6` (Saturday) for a pure proleptic-Gregorian calendar date. `Date.UTC` is used
 * only as a day-of-week calculator here — the values fed in are already local, and the UTC
 * conversion this performs is never observed, so no timezone enters the answer. */
function weekdayOf(d: CalendarDay): number {
  return new Date(Date.UTC(d.year, d.month - 1, d.day)).getUTCDay();
}

function addDays(d: CalendarDay, n: number): CalendarDay {
  let { year, month, day } = d;
  day += n;
  while (day > daysInMonth(year, month)) {
    day -= daysInMonth(year, month);
    month += 1;
    if (month > 12) {
      month = 1;
      year += 1;
    }
  }
  while (day < 1) {
    month -= 1;
    if (month < 1) {
      month = 12;
      year -= 1;
    }
    day += daysInMonth(year, month);
  }
  return { year, month, day };
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

function format(d: CalendarDay, time: { hour: number; minute: number } | null): string {
  const date = `${String(d.year).padStart(4, "0")}-${pad(d.month)}-${pad(d.day)}`;
  return time === null ? date : `${date}T${pad(time.hour)}:${pad(time.minute)}`;
}

// -- the student's clock ------------------------------------------------------------------------

/** Where a UTC instant is read as a wall clock: an IANA zone (the request's `timezone`), or a
 * fixed offset in minutes (the Date header's own, when no usable `timezone` arrived). */
type Clock = { zone: Intl.DateTimeFormat } | { offsetMinutes: number };

/** An `Intl` formatter for `timeZone`, or `null` when it is absent or not a zone `Intl` knows —
 * an unknown name is treated as no timezone at all, never a thrown error. */
function zoneFormatter(timeZone: string | null | undefined): Intl.DateTimeFormat | null {
  if (typeof timeZone !== "string" || timeZone.trim() === "") return null;
  try {
    return new Intl.DateTimeFormat("en-US", {
      timeZone: timeZone.trim(),
      hourCycle: "h23",
      year: "numeric",
      month: "numeric",
      day: "numeric",
      hour: "numeric",
      minute: "numeric",
    });
  } catch {
    return null;
  }
}

/** The wall clock `clock` shows at the UTC instant `utcMs`. */
function wallAt(utcMs: number, clock: Clock): WallClock {
  if ("offsetMinutes" in clock) {
    const d = new Date(utcMs + clock.offsetMinutes * 60_000);
    return {
      year: d.getUTCFullYear(),
      month: d.getUTCMonth() + 1,
      day: d.getUTCDate(),
      hour: d.getUTCHours(),
      minute: d.getUTCMinutes(),
    };
  }
  const parts: Record<string, number> = {};
  for (const p of clock.zone.formatToParts(new Date(utcMs))) {
    if (p.type !== "literal") parts[p.type] = Number(p.value);
  }
  return { year: parts.year, month: parts.month, day: parts.day, hour: parts.hour, minute: parts.minute };
}

/** The UTC instant a wall clock at `offsetMinutes` from UTC names. */
function instantOf(w: WallClock, offsetMinutes: number): number {
  return Date.UTC(w.year, w.month - 1, w.day, w.hour, w.minute) - offsetMinutes * 60_000;
}

// -- time clauses -------------------------------------------------------------------------------

/** A 12- or 24-hour clock reading to `{hour, minute}`, or `null` when it is not a real time: with
 * am/pm the hour must be 1-12 ("13pm" and "0am" are not sure), without it 0-23; minutes 0-59. */
function clockTime(h: string, m: string | undefined, ampm: string | undefined): { hour: number; minute: number } | null {
  let hour = Number(h);
  const minute = m ? Number(m) : 0;
  if (minute > 59) return null;
  const half = ampm?.replace(/\./g, "").toLowerCase();
  if (half !== undefined) {
    if (hour < 1 || hour > 12) return null;
    if (half === "pm" && hour < 12) hour += 12;
    if (half === "am" && hour === 12) hour = 0;
  } else if (hour > 23) {
    return null;
  }
  return { hour, minute };
}

/**
 * Pulls a trailing `"at H[:MM][am|pm]"` clause off `phrase` (already lower-cased), returning the
 * day words that remain and the parsed time, if any. `null` for the no-match case, for a minute
 * past 59, and for an hour that is not sure for the clock it was written on — fix round 1, finding
 * 2: `am`/`pm` is a 12-hour clock, whose only real hours are 1-12 ("13pm" and "0am" are not sure,
 * not calendar times, the same as an "at 25:00" with no am/pm); bare 24-hour digits (no am/pm) are
 * checked against 0-23 instead. Either way, a time clause this cannot parse is left whole in
 * `rest`, where it will fail every day-word pattern below and the phrase resolves to `null` rather
 * than silently dropping the time a student actually wrote.
 */
function splitTime(phrase: string): { rest: string; time: { hour: number; minute: number } | null } {
  const m = /\s*\bat\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\s*$/i.exec(phrase);
  if (!m) return { rest: phrase, time: null };
  const time = clockTime(m[1], m[2], m[3]);
  if (time === null) return { rest: phrase, time: null };
  return { rest: phrase.slice(0, m.index).trim(), time };
}

// -- long-form explicit dates -------------------------------------------------------------------

const MONTH_NAMES: Record<string, number> = {
  january: 1, jan: 1, february: 2, feb: 2, march: 3, mar: 3, april: 4, apr: 4, may: 5,
  june: 6, jun: 6, july: 7, jul: 7, august: 8, aug: 8, september: 9, sept: 9, sep: 9,
  october: 10, oct: 10, november: 11, nov: 11, december: 12, dec: 12,
};
const WEEKDAY_NAMES: Record<string, number> = {
  sunday: 0, sun: 0, monday: 1, mon: 1, tuesday: 2, tues: 2, tue: 2, wednesday: 3, wed: 3,
  thursday: 4, thurs: 4, thur: 4, thu: 4, friday: 5, fri: 5, saturday: 6, sat: 6,
};
const LONG_FORM_RE = new RegExp(
  "^(?:([a-z]+)\\.?,?\\s+)?([a-z]+)\\.?\\s+(\\d{1,2})(?:st|nd|rd|th)?,?\\s+(\\d{4})" +
    "(?:,?\\s+(?:at\\s+)?(\\d{1,2}):(\\d{2})(?::(\\d{2}))?\\s*([ap]\\.?m\\.?)?(?:\\s+([a-z]{1,5}))?)?$",
  "i",
);

type LongForm =
  | { date: CalendarDay; time: { hour: number; minute: number } | null; offsetMinutes: number | null }
  | "unsure";

/**
 * `phrase` as a long-form explicit date, `"unsure"` when it has that shape but something in it is
 * not sure (an unknown weekday, month or zone name, a weekday that names a different day than the
 * date, a date or time that does not exist), or `null` when it is not a long form at all.
 */
function parseLongForm(phrase: string): LongForm | null {
  const m = LONG_FORM_RE.exec(phrase);
  if (m === null) return null;
  const [, weekdayWord, monthWord, dayText, yearText, h, min, sec, ampm, zoneWord] = m;
  const month = MONTH_NAMES[monthWord.toLowerCase()];
  if (month === undefined) return null; // not a month name: some other phrase, not a long form.
  const date = { year: Number(yearText), month, day: Number(dayText) };
  if (!validDate(date.year, date.month, date.day)) return "unsure";
  if (weekdayWord !== undefined) {
    const weekday = WEEKDAY_NAMES[weekdayWord.toLowerCase()];
    if (weekday === undefined || weekday !== weekdayOf(date)) return "unsure";
  }
  if (h === undefined) return { date, time: null, offsetMinutes: null };
  if (sec !== undefined && Number(sec) > 59) return "unsure";
  const time = clockTime(h, min, ampm);
  if (time === null) return "unsure";
  if (zoneWord === undefined) return { date, time, offsetMinutes: null };
  const offsetMinutes = NAMED_ZONES[zoneWord.toUpperCase()];
  if (offsetMinutes === undefined) return "unsure";
  return { date, time, offsetMinutes };
}

const WEEKDAY_ALT = WEEKDAYS.join("|");
const THIS_WEEKDAY_RE = new RegExp(`^(?:this\\s+)?(${WEEKDAY_ALT})$`);

/**
 * Resolves `phrase` — the model's extracted `due` text — against `dateLine` — the email's own
 * `Date:` header — on the student's clock (`timeZone`, an IANA name; see the module doc for where
 * it comes from and what happens without it), into `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`, or `null`
 * when it cannot be resolved to exactly one calendar day.
 *
 * An absolute date the model already gave verbatim (the email stated one explicitly) passes
 * through unchanged: there is nothing to compute, and re-deriving it from `dateLine` would risk
 * disagreeing with the email's own words. A long-form explicit date is read the same way, except
 * that a named zone on it is converted onto the student's clock. `judge_validate.ts`'s
 * `validate()` still checks the final shape before it reaches a verdict (defense in depth — this
 * function is the only caller today, but a caller that skipped it must not be able to smuggle an
 * unresolved phrase through).
 */
export function resolveDue(
  phrase: string | null | undefined,
  dateLine: string,
  timeZone?: string | null,
): string | null {
  const raw = typeof phrase === "string" ? phrase.trim() : "";
  if (raw === "") return null;
  if (ABSOLUTE_DUE_RE.test(raw)) return raw;

  const stamp = parseStamp(dateLine);
  const zone = zoneFormatter(timeZone);
  // The student's clock: their timezone when one arrived, else the Date header's own offset,
  // else none (a zoned long form then has nowhere to land, and resolves to null).
  const clock: Clock | null = zone !== null
    ? { zone }
    : stamp !== null && stamp.offsetMinutes !== null
    ? { offsetMinutes: stamp.offsetMinutes }
    : null;

  const long = parseLongForm(raw);
  if (long === "unsure") return null;
  if (long !== null) {
    if (long.time === null || long.offsetMinutes === null) return format(long.date, long.time);
    if (clock === null) return null;
    const local = wallAt(instantOf({ ...long.date, ...long.time }, long.offsetMinutes), clock);
    return format(local, local);
  }

  if (stamp === null) return null;
  // The email's local date: the header's instant on the student's clock when both are known,
  // otherwise the header's own wall clock (the only clock it gives).
  const reference = zone !== null && stamp.offsetMinutes !== null
    ? wallAt(instantOf(stamp.local, stamp.offsetMinutes), { zone })
    : stamp.local;

  const { rest, time } = splitTime(raw.toLowerCase());
  const today: CalendarDay = { year: reference.year, month: reference.month, day: reference.day };

  if (rest === "today" || rest === "tonight") return format(today, time);
  if (rest === "tomorrow") return format(addDays(today, 1), time);
  // "next week" names a SPAN of seven candidate days, not one of them — fix round 1, finding 1.
  // Deliberately NOT a branch here: it falls through to the final `return null` below, the same
  // path "next month" and every other unmatched phrase already takes.
  if (rest === "end of month" || rest === "end of the month") {
    return format({ ...today, day: daysInMonth(today.year, today.month) }, time);
  }

  // "next <weekday>" is ambiguous between two days (final review item 6) -- deliberately NOT a
  // branch here: `THIS_WEEKDAY_RE` does not match it, so it falls through to `return null`, the
  // same rule as "next week".
  const bareOrThis = THIS_WEEKDAY_RE.exec(rest);
  if (bareOrThis !== null) {
    const target = WEEKDAYS.indexOf(bareOrThis[1]);
    const delta = (target - weekdayOf(today) + 7) % 7;
    return format(addDays(today, delta), time);
  }

  return null;
}
