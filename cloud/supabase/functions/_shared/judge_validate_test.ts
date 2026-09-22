import { assert, assertEquals } from "@std/assert";
import { CONFIDENCE_FLOOR, EMAIL_TIERS, validate } from "./judge_validate.ts";

Deno.test("a task answer is clamped, one-lined and accepted", () => {
  const got = validate("task", {
    course: "cs-100",
    effort_hours: 900,
    importance: 9,
    importance_reason: "  worth 20%\nof the grade  ",
    confidence: 1.4,
  }, { known_courses: ["cs-100"] });
  assertEquals(got.ok, true);
  assertEquals(got.verdict, {
    course: "cs-100",
    effort_hours: 40,
    importance: 5,
    importance_reason: "worth 20% of the grade",
    confidence: 1,
  });
});

Deno.test("a slug the vault does not know is dropped, not refused", () => {
  // Mirrors `judge::judge_task`: the effort and importance answers are still good, and a course
  // nothing renders would put the note in a group no course note explains. The DEVICE re-applies
  // `knows_course` after the reply arrives, so this check is a convenience and that one is the
  // guarantee — except on the Gmail path, where there is no device-side `judge_task` and this is
  // the only check (Task 11 says so where the list comes from).
  const got = validate("task", {
    course: "phys-999",
    effort_hours: 2,
    importance: 3,
    importance_reason: "no weights given",
    confidence: 0.9,
  }, { known_courses: ["cs-100"] });
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.course, null);
});

Deno.test("under the floor is refused and says which refusal it was", () => {
  const got = validate("task", {
    course: null,
    effort_hours: 2,
    importance: 3,
    importance_reason: "x",
    confidence: CONFIDENCE_FLOOR - 0.01,
  }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "below floor");
});

Deno.test("a missing required field is incomplete, not a bad answer", () => {
  const got = validate("task", { course: null, effort_hours: 2, confidence: 0.9 }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("an event why is shaped so the device's ledger can actually write it", () => {
  // `eventledger::why_problem` refuses a double quote, a newline and the field separator ' - '
  // written with a middle dot. A verdict the service produced and the device then cannot record
  // is a judgment thrown away.
  const got = validate("event", {
    verdict: "opportunity",
    why: 'she said "yes" \u00b7 maybe\nnext week',
    confidence: 0.8,
  }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.why, "she said 'yes' - maybe next week");
});

Deno.test("an event verdict outside the ledger's three words is refused", () => {
  const got = validate("event", { verdict: "maybe", why: "unsure", confidence: 0.9 }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("an email tier outside the five is refused", () => {
  const got = validate("email", { tier: "spam", why: "junk", confidence: 0.9 }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

// ---------------------------------------------------------------------------------------------
// Stream J Task T9: the sixth email tier, `completion` — "this email confirms the student already
// submitted or finished a specific piece of work". Its `title` is the work's name, the only thing
// the device matches a task on, so a completion without one is not a usable answer.
// ---------------------------------------------------------------------------------------------

Deno.test("T9: completion is an email tier and carries the work's name as its title", () => {
  assert((EMAIL_TIERS as readonly string[]).includes("completion"));
  const got = validate("email", {
    tier: "completion", title: 'Office Space "Quiz"', course: null, due: null,
    effort_hours: null, importance: null, why: "a submission receipt", confidence: 0.9,
  }, { known_courses: [] });
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.tier, "completion");
  // `oneLine` turns a double quote into an apostrophe; the device folds quotes when it matches.
  assertEquals(got.verdict?.title, "Office Space 'Quiz'");
});

Deno.test("T9: a completion with no title is incomplete, so a stale title-less rule falls through to the model", () => {
  const got = validate("email", {
    tier: "completion", title: "   ", course: null, due: null,
    effort_hours: null, importance: null, why: "a submission receipt", confidence: 0.9,
  }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("T9: gmail_queue's tier check, at its latest definition, admits every email tier", async () => {
  // `gmail_queue.tier` is the one database constraint that names the email tiers
  // (`20260911000200_google.sql`). A tier the pipeline can answer but the queue refuses would fail
  // the whole read with a 23514, so the latest (re)definition across all migrations must list
  // exactly `EMAIL_TIERS`.
  const dir = new URL("../../migrations/", import.meta.url);
  const names: string[] = [];
  for await (const entry of Deno.readDir(dir)) if (entry.name.endsWith(".sql")) names.push(entry.name);
  names.sort();
  let latest: string | null = null;
  for (const name of names) {
    const sql = await Deno.readTextFile(new URL(name, dir));
    const inline = sql.match(/\btier\s+text\s+not\s+null\s+check\s*\(tier in \(([^)]*)\)\)/);
    const altered = sql.match(/gmail_queue_tier_check\s+check\s*\(tier in \(([^)]*)\)\)/);
    if (inline) latest = inline[1];
    if (altered) latest = altered[1];
  }
  assert(latest !== null, "no gmail_queue tier check found");
  const listed = latest!.split(",").map((s) => s.trim().replace(/^'|'$/g, "")).sort();
  assertEquals(listed, [...EMAIL_TIERS].sort());
});
