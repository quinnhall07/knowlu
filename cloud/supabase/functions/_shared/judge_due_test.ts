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
  { category: "'next <weekday>' skips the immediate occurrence", phrase: "next Friday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-25" },
  { category: "'next <weekday>' when today IS that weekday jumps a full week", phrase: "next Wednesday", dateLine: "Wed, 16 Sep 2026 14:23:00 -0400", expected: "2026-09-23" },
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
  { category: "year rollover: 'next <weekday>' crosses into January", phrase: "next Friday", dateLine: "Tue, 29 Dec 2026 09:00:00 +0000", expected: "2027-01-08" },

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
  // 'next Friday' from the Saturday before it lands 13 days later, well past the crossing.
  { category: "DST boundary: 'next Friday' from just before fall-back", phrase: "next Friday", dateLine: "Sat, 31 Oct 2026 22:00:00 -0400", expected: "2026-11-13" },
  { category: "DST boundary: a time survives the crossing unchanged", phrase: "next Friday at 9am", dateLine: "Sat, 31 Oct 2026 22:00:00 -0400", expected: "2026-11-13T09:00" },

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
