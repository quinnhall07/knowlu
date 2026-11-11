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

// ---------------------------------------------------------------------------------------------
// A Date header with a trailing comment (fix/date-comment-due, 2026-11-10). Many mail servers
// write the zone's name after the offset -- "Tue, 10 Nov 2026 14:00:00 +0000 (UTC)" -- which RFC
// 5322 allows (CFWS after the zone). The pattern used to end at the zone, so every such header was
// null and every relative due in that email was lost. Trailing comments are now set aside before
// the match. The zone BEFORE the comment decides the offset, exactly as it does without one; a
// comment is never read as a zone, so "(UTC)" after no zone, or after an unknown one, leaves the
// header on its own wall clock. A comment that is not trailing, an unbalanced or nested one, and a
// header that is only a comment are all still null.
//
// Tue 10 Nov 2026 (Nov 1 is the fall-back Sunday, so Chicago is on CST, UTC-6): "Friday" is the 13th.
// ---------------------------------------------------------------------------------------------

const FRI_PHRASE = "Friday at 11:59pm";
const FRI_DUE = "2026-11-13T23:59";

const COMMENT_ROWS: ZonedRow[] = [
  // -- the three forms from the bug report, on the header's own clock and on a student's clock --
  { category: "+0000 (UTC), no timezone", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 14:00:00 +0000 (UTC)", timeZone: undefined, expected: FRI_DUE },
  { category: "-0500 (EST), no timezone", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (EST)", timeZone: undefined, expected: FRI_DUE },
  { category: "-0800 (PST), no timezone", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0800 (PST)", timeZone: undefined, expected: FRI_DUE },
  { category: "+0000 (UTC), Chicago student", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 14:00:00 +0000 (UTC)", timeZone: CHI, expected: FRI_DUE },
  { category: "-0500 (EST), Chicago student", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (EST)", timeZone: CHI, expected: FRI_DUE },
  { category: "-0800 (PST), Chicago student", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0800 (PST)", timeZone: CHI, expected: FRI_DUE },

  // -- the zone before the comment is the one converted: 04:30 UTC Wed is 22:30 CST Tue --
  { category: "UTC-stamped evening email: the +0000 before the comment puts it on the local Tuesday", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 +0000 (UTC)", timeZone: CHI, expected: "2026-11-10" },
  { category: "UTC-stamped evening email, no timezone: the header's own +0000 is the clock", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 +0000 (UTC)", timeZone: undefined, expected: "2026-11-11" },
  { category: "a named zone (GMT) before the comment is converted as without one", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 GMT (UTC)", timeZone: CHI, expected: "2026-11-10" },

  // -- two comments --
  { category: "two comments: -0800 (PST) (Pacific)", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0800 (PST) (Pacific)", timeZone: undefined, expected: FRI_DUE },
  { category: "two comments, Chicago student", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0800 (PST) (Pacific)", timeZone: CHI, expected: FRI_DUE },
  { category: "two comments: the zone before both still decides the local day", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 +0000 (UTC) (Coordinated Universal Time)", timeZone: CHI, expected: "2026-11-10" },

  // -- a comment with digits and symbols: the numeric zone before it decides --
  { category: "(GMT+05:30) after +0530", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 20:00:00 +0530 (GMT+05:30)", timeZone: undefined, expected: FRI_DUE },
  // 02:00 IST Wed is 20:30 UTC Tue, 14:30 CST Tue.
  { category: "(GMT+05:30) after +0530, Chicago student: converted from +0530", phrase: "today", dateLine: "Wed, 11 Nov 2026 02:00:00 +0530 (GMT+05:30)", timeZone: CHI, expected: "2026-11-10" },
  { category: "(GMT+05:30) after +0530, no timezone: the header's own day", phrase: "today", dateLine: "Wed, 11 Nov 2026 02:00:00 +0530 (GMT+05:30)", timeZone: undefined, expected: "2026-11-11" },

  // -- garbage inside the comment changes nothing (a -0800 inside it is not the offset) --
  { category: "garbage comment after -0500", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (x9 +0800 GMT; ~!@#$%^&*=_ Z)", timeZone: CHI, expected: FRI_DUE },
  { category: "an offset inside the comment is never read: +0000 decides", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 +0000 (-0800 PST)", timeZone: CHI, expected: "2026-11-10" },

  // -- no zone before the comment: the comment is never the offset --
  { category: "no zone, (UTC) comment: the header stays on its own wall clock", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 (UTC)", timeZone: CHI, expected: "2026-11-11" },
  { category: "no zone, (UTC) comment: a weekday still resolves on the header's wall clock", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 (UTC)", timeZone: CHI, expected: FRI_DUE },
  { category: "an unknown zone before a (UTC) comment is still unknown", phrase: "today", dateLine: "Wed, 11 Nov 2026 04:30:00 XYZ (UTC)", timeZone: CHI, expected: "2026-11-11" },

  // -- folding whitespace before, between and after comments --
  { category: "a folded header: CRLF and a tab before the comment", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500\r\n\t(EST)", timeZone: CHI, expected: FRI_DUE },
  { category: "no space before the comment", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500(EST)", timeZone: CHI, expected: FRI_DUE },
  { category: "mixed whitespace between two comments and after them", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0800  (PST)\t(Pacific)\r\n", timeZone: CHI, expected: FRI_DUE },

  // -- still null --
  { category: "a header that is only a comment", phrase: FRI_PHRASE, dateLine: "(Tue, 10 Nov 2026 09:00:00 -0500)", timeZone: CHI, expected: null },
  { category: "a header that is only a zone-name comment", phrase: FRI_PHRASE, dateLine: "(UTC)", timeZone: CHI, expected: null },
  { category: "text after the comment", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (EST) extra", timeZone: CHI, expected: null },
  { category: "a zone after the comment", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (EST) -0800", timeZone: CHI, expected: null },
  { category: "an unclosed comment", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (EST", timeZone: CHI, expected: null },
  { category: "a nested comment is not set aside", phrase: FRI_PHRASE, dateLine: "Tue, 10 Nov 2026 09:00:00 -0500 (EST (Eastern))", timeZone: CHI, expected: null },
  { category: "a comment does not rescue a day that does not exist", phrase: FRI_PHRASE, dateLine: "Mon, 30 Feb 2026 10:00:00 +0000 (UTC)", timeZone: CHI, expected: null },
];

for (const row of COMMENT_ROWS) {
  Deno.test(`trailing comment: ${row.category} (dateLine=${JSON.stringify(row.dateLine)}, tz=${JSON.stringify(row.timeZone)})`, () => {
    assertEquals(resolveDue(row.phrase, row.dateLine, row.timeZone), row.expected, row.category);
  });
}

// The commented header and the same header without its comment give the same answer, on every
// clock and for every kind of phrase: the comment changes nothing but whether the header parses.
const COMMENTED_PAIRS: Array<[string, string]> = [
  ["Tue, 10 Nov 2026 14:00:00 +0000 (UTC)", "Tue, 10 Nov 2026 14:00:00 +0000"],
  ["Tue, 10 Nov 2026 09:00:00 -0500 (EST)", "Tue, 10 Nov 2026 09:00:00 -0500"],
  ["Tue, 10 Nov 2026 09:00:00 -0800 (PST)", "Tue, 10 Nov 2026 09:00:00 -0800"],
  ["Tue, 10 Nov 2026 09:00:00 -0800 (PST) (Pacific)", "Tue, 10 Nov 2026 09:00:00 -0800"],
  ["Wed, 11 Nov 2026 02:00:00 +0530 (GMT+05:30)", "Wed, 11 Nov 2026 02:00:00 +0530"],
  ["Wed, 11 Nov 2026 04:30:00 +0000 (-0800 PST)", "Wed, 11 Nov 2026 04:30:00 +0000"],
  ["Wed, 11 Nov 2026 04:30:00 GMT (UTC)", "Wed, 11 Nov 2026 04:30:00 GMT"],
  ["Wed, 11 Nov 2026 04:30:00 (UTC)", "Wed, 11 Nov 2026 04:30:00"],
];
const PAIR_ZONES = [undefined, CHI, "America/Los_Angeles", "America/New_York", "Asia/Kolkata"];
const RELATIVE_PHRASES = [FRI_PHRASE, "today", "tomorrow", "end of the month at 5pm"];
// A zoned long form needs a clock; a header with no zone and no student timezone gives it none, so
// it is null with or without the comment. It is compared, but not required to resolve.
const ZONED_LONG_FORM = "Friday, November 13, 2026 11:59 PM PST";

for (const [commented, plain] of COMMENTED_PAIRS) {
  Deno.test(`trailing comment: same answer as without it on every clock (dateLine=${JSON.stringify(commented)})`, () => {
    for (const tz of PAIR_ZONES) {
      for (const phrase of RELATIVE_PHRASES) {
        const want = resolveDue(phrase, plain, tz);
        assertEquals(want === null, false, `the uncommented header resolves ${phrase} on ${tz}`);
        assertEquals(resolveDue(phrase, commented, tz), want, `${phrase} on ${tz}`);
      }
      assertEquals(resolveDue(ZONED_LONG_FORM, commented, tz), resolveDue(ZONED_LONG_FORM, plain, tz), `long form on ${tz}`);
    }
  });
}

Deno.test("trailing comment: parseDateLine reads the header's own wall clock, as without the comment", () => {
  assertEquals(parseDateLine("Tue, 10 Nov 2026 14:00:00 +0000 (UTC)"), ref(2026, 11, 10, 14, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500 (EST)"), ref(2026, 11, 10, 9, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0800 (PST)"), ref(2026, 11, 10, 9, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0800 (PST) (Pacific)"), ref(2026, 11, 10, 9, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 20:00:00 +0530 (GMT+05:30)"), ref(2026, 11, 10, 20, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500 (x9 +0800 GMT; ~!@#$%^&*=_ Z)"), ref(2026, 11, 10, 9, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 (UTC)"), ref(2026, 11, 10, 9, 0));
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500\r\n\t(EST)"), ref(2026, 11, 10, 9, 0));
  for (const [commented, plain] of COMMENTED_PAIRS) {
    assertEquals(parseDateLine(commented), parseDateLine(plain), commented);
  }
});

Deno.test("trailing comment: parseDateLine is still null for a comment alone, text after one, or a broken one", () => {
  assertEquals(parseDateLine("(Tue, 10 Nov 2026 09:00:00 -0500)"), null);
  assertEquals(parseDateLine("(UTC)"), null);
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500 (EST) extra"), null);
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500 (EST) -0800"), null);
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500 (EST"), null);
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00 -0500 (EST (Eastern))"), null);
});

// ---------------------------------------------------------------------------------------------
// Length. A Date line and a due phrase are both outside input, so each is answered within a fixed
// time however long it is. Every bound below is far above what a linear pass needs.
// ---------------------------------------------------------------------------------------------

/** Runs `check` over `inputs` and fails when the whole set takes a second or more. */
function withinOneSecond(inputs: string[], check: (input: string) => void): void {
  const started = performance.now();
  for (const input of inputs) check(input);
  const elapsed = performance.now() - started;
  assertEquals(elapsed < 1000, true, `took ${elapsed.toFixed(0)} ms`);
}

Deno.test("length: Date lines of 10k-120k characters (comment runs, whitespace runs, an unclosed comment) are null within one second", () => {
  withinOneSecond([
    "(a)".repeat(40_000) + "x",
    "Tue, 10 Nov 2026 09:00:00 -0500" + " ".repeat(40_000) + "(x",
    "Tue, 10 Nov 2026 09:00:00 -0500 " + "(a) ".repeat(40_000) + "(",
    "Tue, 10 Nov 2026 09:00:00" + " ".repeat(10_000) + "?",
    "Tue, 10 Nov 2026 09:00:00" + " ".repeat(20_000) + "?",
    "Tue, 10 Nov 2026 09:00:00" + " ".repeat(40_000) + "?",
    "Tue, 10 Nov 2026 09:00:00 -0500" + " ".repeat(40_000) + "?",
    "2026-11-10T09:00:00" + " ".repeat(40_000) + "?",
  ], (line) => assertEquals(parseDateLine(line), null, `${line.slice(0, 40)}...`));
});

Deno.test("length: a Date line longer than any real header is null, however well formed", () => {
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00" + " ".repeat(300) + "-0500"), null);
  assertEquals(resolveDue(FRI_PHRASE, "Tue, 10 Nov 2026 09:00:00" + " ".repeat(300) + "-0500", CHI), null);
});

Deno.test("length: the longest real header shapes still parse, long comments included", () => {
  assertEquals(parseDateLine("Wed, 30 September 2026 23:59:59 +0000"), ref(2026, 9, 30, 23, 59));
  assertEquals(
    parseDateLine("Wed, 30 Sep 2026 23:59:59 -0700 (Pacific Daylight Time)" + " (a comment, as some relays write one after the zone name)"),
    ref(2026, 9, 30, 23, 59),
  );
  // The length cap reads the line with its trailing comments set aside, so a long comment never
  // nulls a real header: this whole line is far past the cap, its bare part well under it.
  assertEquals(
    parseDateLine("Wed, 30 Sep 2026 23:59:59 -0700 (Pacific Daylight Time) (" + "c".repeat(200) + ")"),
    ref(2026, 9, 30, 23, 59),
  );
  assertEquals(parseDateLine("Tue, 10 Nov 2026 09:00:00" + " ".repeat(100) + "-0500"), ref(2026, 11, 10, 9, 0));
});

Deno.test("length: due phrases with 10k-40k character whitespace runs around a time clause are null within one second", () => {
  withinOneSecond([
    "today" + " ".repeat(40_000) + "x",
    "tomorrow at 5" + " ".repeat(10_000) + "?",
    "tomorrow at 5" + " ".repeat(20_000) + "?",
    "tomorrow at 5" + " ".repeat(40_000) + "?",
    "tomorrow at 5:00 pm" + " ".repeat(40_000) + "?",
    "tomorrow at 5" + " ".repeat(20_000) + "pm" + " ".repeat(20_000) + "?",
  ], (phrase) => assertEquals(resolveDue(phrase, "Tue, 10 Nov 2026 09:00:00 -0500", CHI), null, `${phrase.slice(0, 40)}...`));
});

Deno.test("length: long-form due phrases with 10k-40k character whitespace runs after the time are null within one second", () => {
  // A Date line that does not parse, so only the long-form reading runs.
  withinOneSecond([
    "Friday, September 18, 2026 11:59" + " ".repeat(10_000) + "?",
    "Friday, September 18, 2026 11:59" + " ".repeat(20_000) + "?",
    "Friday, September 18, 2026 11:59" + " ".repeat(40_000) + "?",
    "Friday, September 18, 2026 11:59 PM" + " ".repeat(40_000) + "?",
    "Friday, September 18, 2026 11:59" + " ".repeat(20_000) + "PM" + " ".repeat(20_000) + "?",
  ], (phrase) => assertEquals(resolveDue(phrase, "garbled, not a date", CHI), null, `${phrase.slice(0, 40)}...`));
});
