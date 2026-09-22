import { assertEquals } from "@std/assert";
import { CONFIDENCE_FLOOR, validate } from "./judge_validate.ts";

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

Deno.test("an event verdict outside the ledger's four words is refused", () => {
  const got = validate("event", { verdict: "maybe", why: "unsure", confidence: 0.9 }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

// CHECKPOINT J-1: `unsure` is now a genuine fourth verdict word.

Deno.test("an event verdict of unsure is accepted", () => {
  const got = validate("event", { verdict: "unsure", why: "audience not stated", confidence: 0.9 }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict, { verdict: "unsure", why: "audience not stated", confidence: 0.9 });
});

Deno.test("an event verdict of unsure is accepted even below the confidence floor — the floor guards a guess, not an honest decline", () => {
  const got = validate("event", { verdict: "unsure", why: "audience not stated", confidence: CONFIDENCE_FLOOR - 0.01 }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.verdict, "unsure");
});

Deno.test("an event verdict of obligation below the floor is still refused — the unsure exemption is not a general floor bypass", () => {
  const got = validate("event", { verdict: "obligation", why: "x", confidence: CONFIDENCE_FLOOR - 0.01 }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "below floor");
});

Deno.test("an email tier outside the five is refused", () => {
  const got = validate("email", { tier: "spam", why: "junk", confidence: 0.9 }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});
