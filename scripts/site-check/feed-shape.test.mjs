// Tests for feed-shape.mjs. Run: node --test scripts/site-check/
// Every feed here is made up. No test opens a socket: fetch is always a fake.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  parse,
  summarise,
  renderText,
  renderJson,
  uidShape,
  normaliseLink,
  main,
  SITES,
} from "./feed-shape.mjs";

const NOW = "2026-11-21";

function cal(lines, eol = "\r\n") {
  return ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Example//Made Up Feed//EN", ...lines, "END:VCALENDAR", ""].join(eol);
}

const BASIC = [
  "BEGIN:VEVENT",
  "UID:_12345_1-assignment@lms.example",
  "DTSTART:20261120T235900Z",
  "DTEND:20261120T235900Z",
  "SUMMARY:Lab Report 2 - Due",
  "CATEGORIES:BIO 101",
  "DESCRIPTION:Submit at https://www.WebAssign.net/web/Student",
  "END:VEVENT",
  "BEGIN:VEVENT",
  "UID:0f8fad5b-d9cb-469f-a165-70867728950e",
  "DTSTART;VALUE=DATE:20261201",
  "SUMMARY:Reading quiz: Opens",
  "CATEGORIES:HIST 210",
  "URL:https://www.gradescope.com/courses/1",
  "BEGIN:VALARM",
  "ACTION:DISPLAY",
  "TRIGGER:-PT15M",
  "END:VALARM",
  "END:VEVENT",
  "BEGIN:VTODO",
  "UID:todo-77@lms.example",
  "DUE;TZID=America/Chicago:20261215T170000",
  "SUMMARY:Chapter 4 MyLab homework",
  "CATEGORIES:BIO 101",
  "END:VTODO",
  "BEGIN:VEVENT",
  "UID:trip-3@lms.example",
  "DTSTART:20261001T090000",
  "DTEND:20261001T110000",
  "SUMMARY:Field trip to Nortonville",
  "LOCATION:Nortonville Hall",
  "X-LMS-COURSE:HIST 210",
  "END:VEVENT",
];

function io(overrides = {}) {
  const out = { stdout: "", stderr: "", fetchCalls: [] };
  const base = {
    stdout: (s) => { out.stdout += s; },
    stderr: (s) => { out.stderr += s; },
    readLine: async () => { throw new Error("readLine not expected"); },
    readFile: async () => { throw new Error("readFile not expected"); },
    fetch: async () => { throw new Error("fetch not expected"); },
    today: () => NOW,
  };
  return { out, io: { ...base, ...overrides } };
}

function report(text, opts = {}) {
  return renderText(summarise(parse(text), { now: NOW, ...opts }));
}

test("folded lines are unfolded (space and tab)", () => {
  const text = cal(["BEGIN:VEVENT", "UID:a-1", "SUMMARY:Lab Rep", " ort 2 - D", "\tue", "END:VEVENT"]);
  const c = parse(text);
  assert.equal(c.isCalendar, true);
  assert.equal(c.items.length, 1);
  assert.equal(c.items[0].props.find((p) => p.name === "SUMMARY").value, "Lab Report 2 - Due");
});

test("CRLF and LF input give the same report", () => {
  assert.equal(report(cal(BASIC, "\r\n")), report(cal(BASIC, "\n")));
});

test("text values are unescaped", () => {
  const text = cal(["BEGIN:VEVENT", "UID:a-1", "DESCRIPTION:one\\ntwo\\Nthree\\, four\\; five \\\\ six", "END:VEVENT"]);
  const d = parse(text).items[0].props.find((p) => p.name === "DESCRIPTION");
  assert.equal(d.value, "one\ntwo\nthree, four; five \\ six");
});

test("parameters are split from the value, quoted ones included", () => {
  const text = cal(["BEGIN:VEVENT", "UID:a-1", 'DTSTART;TZID="America/New_York";X-A=1,2:20261101T080000', "END:VEVENT"]);
  const p = parse(text).items[0].props.find((q) => q.name === "DTSTART");
  assert.equal(p.params.TZID, "America/New_York");
  assert.equal(p.params["X-A"], "1,2");
  assert.equal(p.value, "20261101T080000");
});

test("UID shapes replace digit runs and hex runs", () => {
  assert.equal(uidShape("_12345_1-assignment@lms.example"), "_#_#-assignment@lms.example");
  assert.equal(uidShape("0f8fad5b-d9cb-469f-a165-70867728950e"), "<hex>");
  assert.equal(uidShape("event-ab12cd34ef56@x.example"), "event-<hex>@x.example");
  const s = summarise(parse(cal(BASIC)), { now: NOW });
  assert.deepEqual(s.uidShapes[0], { shape: "<hex>", count: 1 });
  assert.ok(report(cal(BASIC)).includes("_#_#-assignment@lms.example"));
});

test("totals: counts, date range, all-day, times, DTEND, before/after", () => {
  const s = summarise(parse(cal(BASIC)), { now: NOW });
  assert.equal(s.totals.vevent, 3);
  assert.equal(s.totals.vtodo, 1);
  assert.equal(s.totals.earliest, "2026-10-01");
  assert.equal(s.totals.latest, "2026-12-15");
  assert.equal(s.totals.allDay, 1);
  assert.equal(s.totals.timed, 3);
  assert.deepEqual(s.totals.timeForms, { utc: 1, tzid: 1, floating: 1 });
  assert.equal(s.totals.dtendDiffers, 1);
  assert.equal(s.totals.next30, 2);
  assert.equal(s.totals.before, 2);
});

test("property census counts names on items only, sorted", () => {
  const s = summarise(parse(cal(BASIC)), { now: NOW });
  const names = s.properties.map((p) => p.name);
  assert.deepEqual(names, [...names].sort());
  assert.ok(!names.includes("ACTION"), "VALARM properties are not item properties");
  assert.deepEqual(s.properties.find((p) => p.name === "CATEGORIES"), { name: "CATEGORIES", count: 3 });
  assert.ok(names.includes("X-LMS-COURSE"));
});

test("classes are numbered by count, labels hidden by default", () => {
  const s = summarise(parse(cal(BASIC)), { now: NOW });
  assert.deepEqual(s.classes, { property: true, distinct: 2, list: [{ label: "class 1", count: 2 }, { label: "class 2", count: 1 }] });
  const t = report(cal(BASIC), { showTitles: true });
  assert.ok(t.includes("BIO 101"));
  assert.ok(!report(cal(BASIC)).includes("BIO 101"));
});

test("markers: host in DESCRIPTION, host in URL, name in SUMMARY, whole words only", () => {
  const s = summarise(parse(cal(BASIC)), { now: NOW });
  const site = (n) => s.sites.find((x) => x.site === n);
  assert.equal(site("WebAssign").items, 1);
  assert.deepEqual(site("WebAssign").fields, [{ field: "DESCRIPTION", count: 1 }]);
  assert.equal(site("Gradescope").items, 1);
  assert.deepEqual(site("Gradescope").fields, [{ field: "URL", count: 1 }]);
  assert.equal(site("Pearson (MyLab, Mastering, Revel)").items, 1);
  assert.deepEqual(site("Pearson (MyLab, Mastering, Revel)").fields, [{ field: "SUMMARY", count: 1 }]);
  assert.equal(site("Smartwork (Norton)").items, 0, "nortonville must not match norton");
});

test("every site is printed, with 0 when absent", () => {
  const text = cal(["BEGIN:VEVENT", "UID:a-1", "DTSTART:20261101T080000Z", "SUMMARY:Essay 1 - Due", "END:VEVENT"]);
  const t = report(text);
  assert.equal(SITES.length, 13);
  for (const s of SITES) assert.ok(t.includes(`${s.site}: 0`), s.site);
  assert.ok(t.includes("hint, not proof"));
});

test("title endings: marker phrases counted, no other ending printed", () => {
  const sums = ["A - Due", "B: opens", "C - Availability Starts", "D - Graded QXENDING", "E plain", "F - Ends"];
  const lines = sums.flatMap((s, i) => ["BEGIN:VEVENT", `UID:t-${i}`, "DTSTART:20261101T080000Z", `SUMMARY:${s}`, "END:VEVENT"]);
  const s = summarise(parse(cal(lines)), { now: NOW });
  const e = Object.fromEntries(s.endings.phrases.map((p) => [p.phrase, p.count]));
  assert.equal(e["due"], 1);
  assert.equal(e["opens"], 1);
  assert.equal(e["availability starts"], 1);
  assert.equal(e["ends"], 1);
  assert.equal(e["starts"], 0);
  assert.equal(s.endings.none, 2);
  const t = renderText(s);
  assert.ok(t.includes("no marker ending: 2"));
  assert.ok(!/graded/i.test(t));
  assert.ok(!t.includes("QXENDING"));
});

const SENTINEL = "ZQXSENTINEL";
const PLANTED = cal([
  "BEGIN:VEVENT",
  "UID:s-1",
  "DTSTART:20261101T080000Z",
  `SUMMARY:${SENTINEL} essay - Due`,
  `DESCRIPTION:${SENTINEL} details`,
  `LOCATION:${SENTINEL} room`,
  `CATEGORIES:${SENTINEL} 101`,
  `X-LMS-NOTE:${SENTINEL}`,
  "END:VEVENT",
  "BEGIN:VEVENT",
  "UID:s-2",
  "DTSTART:20261102T080000Z",
  `SUMMARY:${SENTINEL} webassign set`,
  "END:VEVENT",
]);

test("titles never appear by default, in text or JSON; they do with --show-titles", async () => {
  const sum = summarise(parse(PLANTED), { now: NOW });
  assert.ok(!renderText(sum).includes(SENTINEL));
  assert.ok(!renderJson(sum).includes(SENTINEL));
  const shown = summarise(parse(PLANTED), { now: NOW, showTitles: true });
  assert.ok(renderText(shown).includes(SENTINEL));
  assert.ok(renderJson(shown).includes(SENTINEL));

  for (const flags of [[], ["--json"]]) {
    const { out, io: x } = io({ readFile: async () => Buffer.from(PLANTED) });
    assert.equal(await main(["--file", "made-up.ics", ...flags], x), 0);
    assert.ok(!out.stdout.includes(SENTINEL), flags.join(" "));
    assert.ok(!out.stderr.includes(SENTINEL));
  }
  const { out, io: x } = io({ readFile: async () => Buffer.from(PLANTED) });
  assert.equal(await main(["--file", "made-up.ics", "--show-titles"], x), 0);
  assert.ok(out.stdout.includes(`${SENTINEL} webassign set`), "matching SUMMARY listed under its site");
  assert.ok(out.stdout.includes(`${SENTINEL} essay - Due`), "unmatched SUMMARY listed at the end");
  assert.ok(!out.stdout.includes(`${SENTINEL} details`), "DESCRIPTION is never printed");
  assert.ok(!out.stdout.includes(`${SENTINEL} room`), "LOCATION is never printed");
});

test("--json prints one JSON object", async () => {
  const { out, io: x } = io({ readFile: async () => Buffer.from(cal(BASIC)) });
  assert.equal(await main(["--file", "f.ics", "--json", "--now", NOW], x), 0);
  const o = JSON.parse(out.stdout);
  assert.equal(o.totals.vevent, 3);
  assert.equal(o.sites.length, 13);
});

test("the same input renders byte-identical twice", () => {
  assert.equal(report(cal(BASIC)), report(cal(BASIC)));
  const a = renderJson(summarise(parse(cal(BASIC)), { now: NOW }));
  const b = renderJson(summarise(parse(cal(BASIC)), { now: NOW }));
  assert.equal(a, b);
});

test("--now changes only the before/after counts", async () => {
  const run = async (now) => {
    const { out, io: x } = io({ readFile: async () => Buffer.from(cal(BASIC)) });
    assert.equal(await main(["--file", "f.ics", "--now", now], x), 0);
    return out.stdout.split("\n");
  };
  const a = await run("2026-11-21");
  const b = await run("2026-09-01");
  assert.equal(a.length, b.length);
  const differing = a.filter((line, i) => line !== b[i]);
  assert.ok(differing.length > 0);
  for (const line of differing) assert.match(line, /^(in the 30 days from|before|later than)/, line);
});

test("default --now is today's date from io", async () => {
  const { out, io: x } = io({ readFile: async () => Buffer.from(cal(BASIC)), today: () => "2026-09-01" });
  assert.equal(await main(["--file", "f.ics"], x), 0);
  assert.ok(out.stdout.includes("before 2026-09-01: 0"));
});

test("input without BEGIN:VCALENDAR exits 2", async () => {
  const { out, io: x } = io({ readFile: async () => Buffer.from("<html><body>Sign in</body></html>") });
  assert.equal(await main(["--file", "f.ics"], x), 2);
  assert.match(out.stderr, /not a calendar/);
  assert.equal(out.stdout, "");
});

test("a feed with zero items says so and exits 0", async () => {
  const { out, io: x } = io({ readFile: async () => Buffer.from(cal([])) });
  assert.equal(await main(["--file", "f.ics"], x), 0);
  assert.equal(out.stdout, "0 items: the link works but lists nothing\n");
});

const LINK_HOST = "calendar.lms.example";
const LINK_TOKEN = "s3cr3tT0kenAbc";
const LINK = `webcal://${LINK_HOST}/feed/${LINK_TOKEN}.ics`;

function assertNoLink(s) {
  assert.ok(!s.includes(LINK_HOST), "host leaked");
  assert.ok(!s.includes(LINK_TOKEN), "token leaked");
  assert.ok(!s.includes("/feed/"), "path leaked");
}

test("webcal:// is rewritten to https://, http:// and others are refused", () => {
  assert.deepEqual(normaliseLink(`  ${LINK}  `), { ok: true, url: `https://${LINK_HOST}/feed/${LINK_TOKEN}.ics` });
  assert.deepEqual(normaliseLink("WEBCAL://x.example/a.ics"), { ok: true, url: "https://x.example/a.ics" });
  assert.equal(normaliseLink("http://x.example/a.ics").ok, false);
  assert.equal(normaliseLink("ftp://x.example/a.ics").ok, false);
  assert.equal(normaliseLink("").ok, false);
});

test("the pasted link is read from stdin and fetched as https", async () => {
  const { out, io: x } = io({
    readLine: async () => LINK,
    fetch: async (url, init) => {
      out.fetchCalls.push(url);
      assert.ok(init && init.signal, "fetch gets an abort signal");
      return new Response(cal(BASIC), { status: 200 });
    },
  });
  assert.equal(await main(["--now", NOW], x), 0);
  assert.deepEqual(out.fetchCalls, [`https://${LINK_HOST}/feed/${LINK_TOKEN}.ics`]);
  assert.match(out.stderr, /Paste the calendar link, then Enter:/);
  assertNoLink(out.stdout + out.stderr);
});

test("an http:// link is refused with exit 2 and never fetched", async () => {
  const { out, io: x } = io({ readLine: async () => `http://${LINK_HOST}/feed/${LINK_TOKEN}.ics` });
  assert.equal(await main([], x), 2);
  assert.match(out.stderr, /refused/);
  assertNoLink(out.stdout + out.stderr);
});

test("a 404 gives exit 1 and a message without the link", async () => {
  const { out, io: x } = io({ readLine: async () => LINK, fetch: async () => new Response("nope", { status: 404 }) });
  assert.equal(await main([], x), 1);
  assert.match(out.stderr, /HTTP 404/);
  assertNoLink(out.stdout + out.stderr);
});

test("a network error gives exit 1 and never echoes the error text", async () => {
  const fetch = async () => {
    const e = new TypeError(`fetch failed for https://${LINK_HOST}/feed/${LINK_TOKEN}.ics`);
    e.cause = Object.assign(new Error(`getaddrinfo ENOTFOUND ${LINK_HOST}`), { code: "ENOTFOUND" });
    throw e;
  };
  const { out, io: x } = io({ readLine: async () => LINK, fetch });
  assert.equal(await main([], x), 1);
  assert.match(out.stderr, /ENOTFOUND/);
  assertNoLink(out.stdout + out.stderr);
});

test("a timeout gives exit 1 and names the timeout", async () => {
  const fetch = async () => { throw new DOMException("The operation was aborted due to timeout", "TimeoutError"); };
  const { out, io: x } = io({ readLine: async () => LINK, fetch });
  assert.equal(await main([], x), 1);
  assert.match(out.stderr, /timed out/);
});

test("a body over the size cap gives exit 1", async () => {
  const { out, io: x } = io({
    readLine: async () => LINK,
    fetch: async () => new Response("x".repeat(5000), { status: 200 }),
    maxBytes: 1000,
  });
  assert.equal(await main([], x), 1);
  assert.match(out.stderr, /too large/);
});

test("an unknown argument is refused without echoing it", async () => {
  const { out, io: x } = io();
  assert.equal(await main([`https://${LINK_HOST}/feed/${LINK_TOKEN}.ics`], x), 2);
  assertNoLink(out.stdout + out.stderr);
});

test("a bad --now is refused", async () => {
  const { io: x } = io();
  assert.equal(await main(["--file", "f.ics", "--now", "21/11/2026"], x), 2);
});
