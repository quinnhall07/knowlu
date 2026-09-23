// Table-driven tests for the T4 email `due` resolver (`judge_due.ts`). TDD, per CLAUDE.md: this
// file is written before `judge_due.ts` exists — RED because the import resolves to nothing —
// until the module does.
import { assertEquals } from "@std/assert";
import { parseDateLine, type ReferenceDate, resolveDue } from "./judge_due.ts";

// ---------------------------------------------------------------------------------------------
// resolveDue: one table, one assertion per row. Every reference date below was computed with
// `Date.UTC(...).getUTCDay()` (the same function `judge_due.ts` uses), not guessed by hand, and
// each row names the category it exercises from the T4 brief so a failure says which promise
// broke, not just which line.
// ---------------------------------------------------------------------------------------------

interface Row {
  category: string;
  phrase: string | null | undefined;
  dateLine: string;
  expected: string | null;
}

const ROWS: Row[] = [
  // -- weekday names, bare vs "this" vs "next" -- reference: Wed 2026-09-16.
  { category: "bare weekday = this-week semantics (Wed -> Fri)", phrase: "Friday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-18" },
  { category: "explicit 'this <weekday>' matches bare weekday", phrase: "this Friday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-18" },
  // Final review item 6: "next Friday" said on a Wednesday means the 18th to some readers and the
  // 25th to others -- two candidate days, so the same rule as "next week": null, never a guess.
  { category: "'next <weekday>' is ambiguous between two days, so null", phrase: "next Friday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "'next <weekday>' when today IS that weekday is still null", phrase: "next Wednesday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "'next <weekday>' with a time clause is still null", phrase: "next Friday at 9am", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "case-insensitive weekday name", phrase: "FRIDAY", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-18" },

  // -- same-day --
  { category: "same-day: 'today'", phrase: "today", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-16" },
  { category: "same-day: bare weekday equal to today's weekday resolves to today", phrase: "Wednesday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-16" },
  { category: "same-day: 'this <weekday>' equal to today resolves to today", phrase: "this Wednesday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-16" },
  { category: "'tomorrow'", phrase: "tomorrow", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-17" },

  // -- month ends --
  { category: "month end: 30-day month", phrase: "end of the month", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-30" },
  { category: "month end: 'end of month' (no 'the') reads the same", phrase: "end of month", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-30" },
  { category: "month end: non-leap February (28 days)", phrase: "end of the month", dateLine: "Tue, 10 Feb 2026 08:00:00 +0000", expected: "2026-02-28" },
  { category: "month end: leap February (29 days)", phrase: "end of the month", dateLine: "Thu, 10 Feb 2028 08:00:00 +0000", expected: "2028-02-29" },

  // -- year rollover --
  { category: "year rollover: 'tomorrow' crosses New Year's Eve", phrase: "tomorrow", dateLine: "Thu, 31 Dec 2026 09:00:00 +0000", expected: "2027-01-01" },
  { category: "year rollover: 'this <weekday>' crosses into January", phrase: "this Friday", dateLine: "Tue, 29 Dec 2026 09:00:00 +0000", expected: "2027-01-01" },
  { category: "year rollover: 'next <weekday>' is still null", phrase: "next Friday", dateLine: "Tue, 29 Dec 2026 09:00:00 +0000", expected: null },

  // -- times with am/pm --
  { category: "time: 'tomorrow at 5pm'", phrase: "tomorrow at 5pm", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-17T17:00" },
  { category: "time: weekday with minutes and am", phrase: "Friday at 9:30am", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-18T09:30" },
  { category: "time: 12pm is noon, not midnight", phrase: "tomorrow at 12pm", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-17T12:00" },
  { category: "time: 12am is midnight, not noon", phrase: "tomorrow at 12am", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-17T00:00" },
  { category: "time: 24-hour clock, no am/pm", phrase: "tomorrow at 17:00", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-17T17:00" },
  { category: "time: 'end of the month' can carry a time too", phrase: "end of the month at 11:59pm", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-30T23:59" },

  // -- malformed 12-hour times: a value with am/pm outside 1-12, or minutes >= 60, is not sure --
  // fix round 1, finding 2: these used to yield a wrong-but-in-range time instead of null.
  { category: "malformed time: '13pm' has no 12-hour meaning", phrase: "tomorrow at 13pm", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "malformed time: '0am' has no 12-hour meaning (valid hours are 1-12)", phrase: "tomorrow at 0am", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "malformed time: minutes >= 60, even with a valid 12-hour value", phrase: "tomorrow at 5:75pm", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "malformed time: '25pm' is doubly out of range", phrase: "Friday at 25pm", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },

  // -- timezone offsets on the Date line: same naive wall clock, different offsets, same answer --
  { category: "offset: -0400 (EDT)", phrase: "tomorrow", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-17" },
  { category: "offset: +0530 (IST) — same naive time, same answer", phrase: "tomorrow", dateLine: "Wed, 16 Sep 2026 14:23:00 +0530", expected: "2026-09-17" },
  { category: "offset: named zone GMT — same naive time, same answer", phrase: "tomorrow", dateLine: "Wed, 16 Sep 2026 14:23:00 GMT", expected: "2026-09-17" },
  { category: "offset: +0000", phrase: "tomorrow", dateLine: "Wed, 16 Sep 2026 14:23:00 +0000", expected: "2026-09-17" },

  // -- a DST boundary: resolving across it changes no arithmetic, because the offset is never
  // re-applied (see judge_due.ts's module doc) -- Nov 1 2026 is the US fall-back Sunday, and
  // 'this Friday' from the Saturday before it lands 6 days later, past the crossing.
  { category: "DST boundary: 'this Friday' from just before fall-back", phrase: "this Friday", dateLine: "Sat, 31 Oct 2026 22:00:00 -0400", expected: "2026-11-06" },
  { category: "DST boundary: a time survives the crossing unchanged", phrase: "Friday at 9am", dateLine: "Sat, 31 Oct 2026 22:00:00 -0400", expected: "2026-11-06T09:00" },

  // -- an absolute date the email stated explicitly: the model's own extraction, passed through --
  { category: "absolute passthrough: date only", phrase: "2026-10-01", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-10-01" },
  { category: "absolute passthrough: date and time", phrase: "2026-10-01T17:00", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-10-01T17:00" },
  { category: "absolute passthrough survives an unparseable Date line", phrase: "2026-10-01", dateLine: "not a real date header", expected: "2026-10-01" },

  // -- unresolvable phrases -> null, never a guess --
  { category: "unresolvable: vague urgency word", phrase: "ASAP", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: 'soon'", phrase: "soon", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  // fix round 1, finding 1: "next week" names a SPAN (seven candidate days), not one calendar day.
  // Picking today+7 out of that span was a guess wearing a resolved date's clothes -- the same
  // failure "next month" (below) was already refused for. Same treatment, near the reference date
  // and across a year rollover, so the null branch is proven to fire before any date arithmetic
  // ever runs, not just to happen to produce a date nobody checked.
  { category: "unresolvable: 'next week' names a span, not one calendar day", phrase: "next week", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: 'next week' is still a span across a year rollover", phrase: "next week", dateLine: "Tue, 29 Dec 2026 09:00:00 +0000", expected: null },
  { category: "unresolvable: 'next week' with a time clause is still a span", phrase: "next week at 9am", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: a month name alone, not a calendar date", phrase: "next month", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: a day-of-month with no month named", phrase: "the 13th", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: empty string", phrase: "", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: whitespace only", phrase: "   ", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: null phrase", phrase: null, dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: undefined phrase", phrase: undefined, dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
  { category: "unresolvable: a relative phrase with an unparseable Date line", phrase: "Friday", dateLine: "garbled, not a date", expected: null },
  { category: "unresolvable: an out-of-range time clause is left unparsed and fails every pattern", phrase: "tomorrow at 25:00", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: null },
];

for (const row of ROWS) {
  Deno.test(`resolveDue: ${row.category} (phrase=${JSON.stringify(row.phrase)})`, () => {
    assertEquals(resolveDue(row.phrase, row.dateLine), row.expected, row.category);
  });
}

// ---------------------------------------------------------------------------------------------
// parseDateLine directly: the RFC 5322 / ISO 8601 parsing `resolveDue` depends on, including the
// zone-letter and two-digit-year forms RFC 5322 still allows in the wild, and its failure cases.
// ---------------------------------------------------------------------------------------------

function ref(year: number, month: number, day: number, hour: number, minute: number): ReferenceDate {
  return { year, month, day, hour, minute };
}

Deno.test("parseDateLine: full RFC 5322 with a numeric offset", () => {
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:00 -0400"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: no leading day-of-week name (optional per RFC 5322)", () => {
  assertEquals(parseDateLine("16 Sep 2026 14:23:00 -0400"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: seconds are accepted and ignored", () => {
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:45 -0400"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: obsolete US zone letters (EST/EDT/etc.) parse — the letters are consumed, not applied", () => {
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:00 EDT"), ref(2026, 9, 16, 14, 23));
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:00 PST"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: UT, GMT and Z all parse", () => {
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:00 UT"), ref(2026, 9, 16, 14, 23));
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:00 GMT"), ref(2026, 9, 16, 14, 23));
  assertEquals(parseDateLine("Wed, 16 Sep 2026 14:23:00 Z"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: a two-digit year is normalised (RFC 5322 obsolete form)", () => {
  assertEquals(parseDateLine("Wed, 16 Sep 26 14:23:00 -0400"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: ISO 8601 fallback, with a numeric offset", () => {
  assertEquals(parseDateLine("2026-09-16T14:23:00-04:00"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: ISO 8601 fallback, with Z", () => {
  assertEquals(parseDateLine("2026-09-16T14:23:00Z"), ref(2026, 9, 16, 14, 23));
});

Deno.test("parseDateLine: ISO 8601 fallback, date only", () => {
  assertEquals(parseDateLine("2026-09-16"), ref(2026, 9, 16, 0, 0));
});

Deno.test("parseDateLine: an impossible calendar date (Feb 30) is rejected, not clamped", () => {
  assertEquals(parseDateLine("Mon, 30 Feb 2026 10:00:00 +0000"), null);
});

Deno.test("parseDateLine: an out-of-range hour is rejected", () => {
  assertEquals(parseDateLine("Wed, 16 Sep 2026 25:23:00 +0000"), null);
});

Deno.test("parseDateLine: an unparseable string is null, not a thrown error", () => {
  assertEquals(parseDateLine("not a date at all"), null);
});

Deno.test("parseDateLine: the empty string is null", () => {
  assertEquals(parseDateLine(""), null);
});

// ---------------------------------------------------------------------------------------------
// Due-fix (2026-09-23, from scoring the pinned email model against the labelled set): two bugs.
//
// 1. The model often writes an explicit date in LONG FORM, copied from the email --
//    "Friday, September 18, 2026 11:59:00 PM CDT" -- and the resolver used to return null for it
//    (6 of 50 real task emails lost their deadline). A long form is now accepted, its named US
//    zone converted to the right instant and expressed on the student's own clock; an unknown
//    zone, or a weekday that names a different day than the date, is null -- never a guess.
// 2. Relative words were resolved against the Date header's own wall clock, which for mail a
//    server stamped in UTC is the UTC calendar date -- a 23:30 CDT email's "tomorrow" came out a
//    day late. They now resolve against the email's local date in the student's timezone (the
//    vault's `config/ingest.yaml` `timezone`, sent on the request).
// ---------------------------------------------------------------------------------------------

interface ZonedRow extends Row {
  timeZone: string | null | undefined;
}

// Wed 16 Sep 2026, 14:23 CDT -- the email that carries the long form.
const WED = "Wed, 16 Sep 2026 14:23:00 -0500";
const CHI = "America/Chicago";

const ZONED_ROWS: ZonedRow[] = [
  // -- long forms, on a Chicago clock (CDT, UTC-5, in September) --
  { category: "long form: weekday, seconds, 12-hour, CDT (the form the model writes)", phrase: "Friday, September 18, 2026 11:59:00 PM CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },
  { category: "long form: no weekday", phrase: "September 18, 2026 11:59:00 PM CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },
  { category: "long form: no seconds", phrase: "Friday, September 18, 2026 11:59 PM CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },
  { category: "long form: 'at' before the time", phrase: "Friday, September 18, 2026 at 11:59 PM CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },
  { category: "long form: lower-case am/pm", phrase: "Friday, September 18, 2026 9:05 am CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T09:05" },
  { category: "long form: 24-hour clock", phrase: "Friday, September 18, 2026 23:59 CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },
  { category: "long form: date only, with weekday", phrase: "Friday, September 18, 2026", dateLine: WED, timeZone: CHI, expected: "2026-09-18" },
  { category: "long form: date only, no weekday", phrase: "September 18, 2026", dateLine: WED, timeZone: CHI, expected: "2026-09-18" },
  { category: "long form: abbreviated month 'Sept'", phrase: "Sept 18, 2026", dateLine: WED, timeZone: CHI, expected: "2026-09-18" },
  { category: "long form: abbreviated weekday and month", phrase: "Fri, Sep 18, 2026 11:59 PM CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },
  { category: "long form: a time with no zone is the email's own wall clock, as stated", phrase: "Friday, September 18, 2026 11:59 PM", dateLine: WED, timeZone: CHI, expected: "2026-09-18T23:59" },

  // -- each US zone, 10:00 AM on Fri 18 Sep 2026, onto a Chicago (CDT) clock --
  { category: "zone CDT (UTC-5)", phrase: "September 18, 2026 10:00 AM CDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T10:00" },
  { category: "zone CST (UTC-6)", phrase: "September 18, 2026 10:00 AM CST", dateLine: WED, timeZone: CHI, expected: "2026-09-18T11:00" },
  { category: "zone EDT (UTC-4)", phrase: "September 18, 2026 10:00 AM EDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T09:00" },
  { category: "zone EST (UTC-5)", phrase: "September 18, 2026 10:00 AM EST", dateLine: WED, timeZone: CHI, expected: "2026-09-18T10:00" },
  { category: "zone MDT (UTC-6)", phrase: "September 18, 2026 10:00 AM MDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T11:00" },
  { category: "zone MST (UTC-7)", phrase: "September 18, 2026 10:00 AM MST", dateLine: WED, timeZone: CHI, expected: "2026-09-18T12:00" },
  { category: "zone PDT (UTC-7)", phrase: "September 18, 2026 10:00 AM PDT", dateLine: WED, timeZone: CHI, expected: "2026-09-18T12:00" },
  { category: "zone PST (UTC-8)", phrase: "September 18, 2026 10:00 AM PST", dateLine: WED, timeZone: CHI, expected: "2026-09-18T13:00" },
  { category: "zone conversion crosses midnight onto the next local day", phrase: "Friday, September 18, 2026 11:59 PM PDT", dateLine: WED, timeZone: CHI, expected: "2026-09-19T01:59" },
  { category: "zone conversion onto an Eastern student's clock", phrase: "Friday, September 18, 2026 11:59 PM CDT", dateLine: WED, timeZone: "America/New_York", expected: "2026-09-19T00:59" },
  { category: "zone CST in winter onto a Chicago (CST) clock", phrase: "Friday, December 4, 2026 11:59 PM CST", dateLine: WED, timeZone: CHI, expected: "2026-12-04T23:59" },

  // -- long forms that are not sure -> null --
  { category: "long form: unknown zone abbreviation", phrase: "Friday, September 18, 2026 11:59 PM XYZ", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: an ambiguous non-US abbreviation (BST) is unknown", phrase: "Friday, September 18, 2026 11:59 PM BST", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: weekday names a different day than the date", phrase: "Thursday, September 18, 2026 11:59 PM CDT", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: weekday inconsistent, date only", phrase: "Monday, September 18, 2026", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: not a real calendar day", phrase: "February 30, 2026", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: 13 PM is not a 12-hour time", phrase: "September 18, 2026 13:00 PM CDT", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: no year is not one calendar day", phrase: "Friday, September 18 11:59 PM CDT", dateLine: WED, timeZone: CHI, expected: null },
  { category: "long form: a zoned time with no clock to put it on is null", phrase: "Friday, September 18, 2026 11:59 PM CDT", dateLine: "garbled, not a date", timeZone: null, expected: null },
  { category: "long form: a date-only long form needs no clock", phrase: "Friday, September 18, 2026", dateLine: "garbled, not a date", timeZone: null, expected: "2026-09-18" },
  { category: "long form: no student timezone falls back to the Date header's own offset (-0400)", phrase: "Friday, September 18, 2026 11:59 PM CDT", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", timeZone: undefined, expected: "2026-09-19T00:59" },

  // -- relative words against the student's local date: the late-evening email --
  // 23:30 CDT on Thu 17 Sep 2026 is 04:30 UTC on Fri 18 Sep. "tomorrow" is Friday the 18th.
  { category: "evening email, CDT-stamped header: 'tomorrow' is the next local day", phrase: "tomorrow", dateLine: "Thu, 17 Sep 2026 23:30:00 -0500", timeZone: CHI, expected: "2026-09-18" },
  { category: "evening email, UTC-stamped header: 'tomorrow' is the next LOCAL day, not UTC's", phrase: "tomorrow", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-09-18" },
  { category: "evening email, ISO Z date line: 'tomorrow'", phrase: "tomorrow", dateLine: "2026-09-18T04:30:00Z", timeZone: CHI, expected: "2026-09-18" },
  { category: "evening email, UTC-stamped: 'today' is the local Thursday", phrase: "today", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-09-17" },
  { category: "evening email, UTC-stamped: 'tonight' is the local Thursday", phrase: "tonight", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-09-17" },
  { category: "evening email, UTC-stamped: 'tonight at 11:59pm'", phrase: "tonight at 11:59pm", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-09-17T23:59" },
  { category: "evening email, UTC-stamped: bare weekday equal to the local day is today", phrase: "Thursday", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-09-17" },
  { category: "evening email, UTC-stamped: 'this Friday' from the local Thursday", phrase: "this Friday", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-09-18" },
  { category: "evening email, UTC-stamped: 'next Friday' is still null", phrase: "next Friday", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: null },
  { category: "evening email, UTC-stamped: 'next week' is still null", phrase: "next week", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: null },

  // -- the UTC-midnight boundary --
  { category: "UTC midnight exactly is still the previous local evening: 'tomorrow'", phrase: "tomorrow", dateLine: "Fri, 18 Sep 2026 00:00:00 +0000", timeZone: CHI, expected: "2026-09-18" },
  { category: "local midnight (05:00 UTC in CDT) is the new local day: 'today'", phrase: "today", dateLine: "Fri, 18 Sep 2026 05:00:00 +0000", timeZone: CHI, expected: "2026-09-18" },
  { category: "one minute before local midnight: 'today' is still Thursday", phrase: "today", dateLine: "Fri, 18 Sep 2026 04:59:00 +0000", timeZone: CHI, expected: "2026-09-17" },
  { category: "east of UTC: 20:00 UTC Thursday is already Friday in Kolkata", phrase: "today", dateLine: "Thu, 17 Sep 2026 20:00:00 +0000", timeZone: "Asia/Kolkata", expected: "2026-09-18" },
  { category: "'end of the month' on the local date, across a UTC month boundary", phrase: "end of the month", dateLine: "Thu, 1 Oct 2026 03:00:00 +0000", timeZone: CHI, expected: "2026-09-30" },
  { category: "a named header zone (EDT) is converted too", phrase: "tomorrow", dateLine: "Fri, 18 Sep 2026 00:30:00 EDT", timeZone: CHI, expected: "2026-09-18" },

  // -- fallbacks when no student timezone is usable --
  { category: "no timezone: the Date header's own offset is the clock (UTC-stamped -> UTC date)", phrase: "tomorrow", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: undefined, expected: "2026-09-19" },
  { category: "null timezone reads as none", phrase: "tomorrow", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: null, expected: "2026-09-19" },
  { category: "an unknown IANA name reads as none, never a thrown error", phrase: "tomorrow", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: "Not/AZone", expected: "2026-09-19" },
  { category: "a header with no offset at all stays on its own wall clock", phrase: "tomorrow", dateLine: "2026-09-17T23:30:00", timeZone: CHI, expected: "2026-09-18" },
  { category: "absolute passthrough is untouched by the timezone", phrase: "2026-10-01T17:00", dateLine: "Fri, 18 Sep 2026 04:30:00 +0000", timeZone: CHI, expected: "2026-10-01T17:00" },
];

for (const row of ZONED_ROWS) {
  Deno.test(`resolveDue (zoned): ${row.category} (phrase=${JSON.stringify(row.phrase)}, tz=${JSON.stringify(row.timeZone)})`, () => {
    assertEquals(resolveDue(row.phrase, row.dateLine, row.timeZone), row.expected, row.category);
  });
}
