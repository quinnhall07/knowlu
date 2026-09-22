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
// "today", "tomorrow", a named weekday, and "end of the month" (which names the month's own last
// day, a single day, not a range) qualify; "next week" and "next month" do not.
//
// **Everything happens in the email's own Date header's wall-clock values, never converted.** RFC
// 5322's `Date:` header already records local time plus a UTC offset — the date/time fields ARE
// the local wall clock the mail client (or server) that stamped it was using, and the offset only
// says which UTC instant that wall clock names. Every relative phrase this module resolves
// ("Friday", "tomorrow") means a day on THAT wall clock, so resolution never needs to leave it:
// `parseDateLine` reads the offset (so a header that carries one, in any of the forms RFC 5322
// allows, is never misparsed) but the arithmetic below never applies it. That is also why a
// timezone offset on the Date line, and a resolution that crosses a DST boundary, cannot change
// the answer: the calendar math is pure day/month/year arithmetic on the header's own local
// values, with no re-derivation of what the offset would be on the resolved day.
import { ABSOLUTE_DUE_RE } from "./judge_validate.ts";

/** A Date header's own local wall-clock date and time — the offset, if any, has already been
 * consumed by `parseDateLine` and is never applied here (see the module doc above). */
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

/**
 * Parses an RFC 5322 `Date:` header — `"[Day, ]DD Mon YYYY HH:MM[:SS] <zone>"`, the zone being a
 * numeric `[+-]HHMM`, `UT`/`GMT`/`Z`, or one of the obsolete US zone names — or, defensively, a
 * plain ISO 8601 stamp (`YYYY-MM-DD[THH:MM[:SS]][zone]`). The zone is matched so it is never
 * mistaken for part of the date or time, then discarded: see the module doc for why. Returns
 * `null` for anything else, or for a date that is not a real calendar day (`31 Feb`, an hour past
 * 23, a minute past 59) — a reference this module cannot trust is a phrase it cannot resolve.
 */
export function parseDateLine(dateLine: string): ReferenceDate | null {
  const s = dateLine.trim();
  const zone = "(?:[+-]\\d{4}|UT|GMT|Z|[A-Z]{2,5})?";
  const rfc = new RegExp(
    `^(?:[A-Za-z]{3},\\s*)?(\\d{1,2})\\s+([A-Za-z]{3})[A-Za-z]*\\s+(\\d{2,4})\\s+(\\d{1,2}):(\\d{2})(?::\\d{2})?\\s*${zone}\\s*$`,
    "i",
  );
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
    return { year, month: monthIdx + 1, day, hour, minute };
  }

  const iso = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::\d{2})?)?(?:Z|[+-]\d{2}:?\d{2})?\s*$/;
  const m2 = iso.exec(s);
  if (m2) {
    const year = Number(m2[1]);
    const month = Number(m2[2]);
    const day = Number(m2[3]);
    const hour = m2[4] ? Number(m2[4]) : 0;
    const minute = m2[5] ? Number(m2[5]) : 0;
    if (!validDate(year, month, day) || hour > 23 || minute > 59) return null;
    return { year, month, day, hour, minute };
  }

  return null;
}

interface CalendarDay {
  year: number;
  month: number;
  day: number;
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
  let hour = Number(m[1]);
  const minute = m[2] ? Number(m[2]) : 0;
  const ampm = m[3]?.toLowerCase();
  if (minute > 59) return { rest: phrase, time: null };
  if (ampm !== undefined) {
    if (hour < 1 || hour > 12) return { rest: phrase, time: null };
    if (ampm === "pm" && hour < 12) hour += 12;
    if (ampm === "am" && hour === 12) hour = 0;
  } else if (hour > 23) {
    return { rest: phrase, time: null };
  }
  return { rest: phrase.slice(0, m.index).trim(), time: { hour, minute } };
}

const WEEKDAY_ALT = WEEKDAYS.join("|");
const THIS_WEEKDAY_RE = new RegExp(`^(?:this\\s+)?(${WEEKDAY_ALT})$`);
const NEXT_WEEKDAY_RE = new RegExp(`^next\\s+(${WEEKDAY_ALT})$`);

/**
 * Resolves `phrase` — the model's extracted `due` text — against `dateLine` — the email's own
 * `Date:` header — into `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`, or `null` when it cannot be resolved
 * to exactly one calendar day.
 *
 * An absolute date the model already gave verbatim (the email stated one explicitly) passes
 * through unchanged: there is nothing to compute, and re-deriving it from `dateLine` would risk
 * disagreeing with the email's own words. `judge_validate.ts`'s `validate()` still checks the
 * final shape before it reaches a verdict (defense in depth — this function is the only caller
 * today, but a caller that skipped it must not be able to smuggle an unresolved phrase through).
 */
export function resolveDue(phrase: string | null | undefined, dateLine: string): string | null {
  const raw = typeof phrase === "string" ? phrase.trim() : "";
  if (raw === "") return null;
  if (ABSOLUTE_DUE_RE.test(raw)) return raw;

  const ref = parseDateLine(dateLine);
  if (ref === null) return null;

  const { rest, time } = splitTime(raw.toLowerCase());
  const today: CalendarDay = { year: ref.year, month: ref.month, day: ref.day };

  if (rest === "today") return format(today, time);
  if (rest === "tomorrow") return format(addDays(today, 1), time);
  // "next week" names a SPAN of seven candidate days, not one of them — fix round 1, finding 1.
  // Deliberately NOT a branch here: it falls through to the final `return null` below, the same
  // path "next month" and every other unmatched phrase already takes.
  if (rest === "end of month" || rest === "end of the month") {
    return format({ ...today, day: daysInMonth(today.year, today.month) }, time);
  }

  const next = NEXT_WEEKDAY_RE.exec(rest);
  if (next !== null) {
    const target = WEEKDAYS.indexOf(next[1]);
    const delta = ((target - weekdayOf(today) + 7) % 7) + 7;
    return format(addDays(today, delta), time);
  }

  const bareOrThis = THIS_WEEKDAY_RE.exec(rest);
  if (bareOrThis !== null) {
    const target = WEEKDAYS.indexOf(bareOrThis[1]);
    const delta = (target - weekdayOf(today) + 7) % 7;
    return format(addDays(today, delta), time);
  }

  return null;
}
