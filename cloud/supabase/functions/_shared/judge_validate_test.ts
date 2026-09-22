import { assertEquals } from "@std/assert";
import { CONFIDENCE_FLOOR, EVENT_RULE_WHY, MAX_REASON_CHARS, oneLine, validate } from "./judge_validate.ts";

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

// ---------------------------------------------------------------------------------------------
// Stream J Task T3 (prompt event-4): the two drop rules are their own schema fields, answered by
// the model and combined HERE, in code. A drop that fires on a named rule carries a templated why
// naming the rule. The reply's wire shape is unchanged: verdict, why, confidence — nothing else.
// ---------------------------------------------------------------------------------------------

Deno.test("event-4: an audience the event excludes the student from is a drop, whatever the model's own verdict", () => {
  const got = validate("event", {
    audience_excludes_student: true,
    standing_or_drop_in: false,
    verdict: "opportunity",
    why: "a colloquium worth attending",
    confidence: 0.9,
  }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict, { verdict: "drop", why: EVENT_RULE_WHY.audience, confidence: 0.9 });
});

Deno.test("event-4: a standing exhibit, office-hours block or recurring drop-in is a drop with the rule's why", () => {
  const got = validate("event", {
    audience_excludes_student: false,
    standing_or_drop_in: true,
    verdict: "obligation",
    why: "weekly office hours",
    confidence: 0.8,
  }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict, { verdict: "drop", why: EVENT_RULE_WHY.standing, confidence: 0.8 });
});

Deno.test("event-4: when both rules fire the audience rule names the drop", () => {
  const got = validate("event", {
    audience_excludes_student: true,
    standing_or_drop_in: true,
    verdict: "drop",
    why: "x",
    confidence: 0.9,
  }, {});
  assertEquals(got.verdict?.why, EVENT_RULE_WHY.audience);
});

Deno.test("event-4: with neither rule firing, the model's own verdict and why pass through", () => {
  const got = validate("event", {
    audience_excludes_student: false,
    standing_or_drop_in: false,
    verdict: "unsure",
    why: "audience not stated",
    confidence: 0.5,
  }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict, { verdict: "unsure", why: "audience not stated", confidence: 0.5 });
});

Deno.test("event-4: a rule that fires needs no why of the model's own — the rule's why is the reason", () => {
  const got = validate("event", {
    audience_excludes_student: true,
    standing_or_drop_in: false,
    verdict: "drop",
    why: "",
    confidence: 0.9,
  }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.why, EVENT_RULE_WHY.audience);
});

Deno.test("event-4: a rule-fired drop still clears the confidence floor like any other drop", () => {
  const got = validate("event", {
    audience_excludes_student: true,
    standing_or_drop_in: false,
    verdict: "unsure",
    why: "x",
    confidence: CONFIDENCE_FLOOR - 0.01,
  }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "below floor");
});

Deno.test("event-4: a rule field that is not a boolean is incomplete, never read as truthy", () => {
  const got = validate("event", {
    audience_excludes_student: "true",
    standing_or_drop_in: false,
    verdict: "opportunity",
    why: "x",
    confidence: 0.9,
  }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("event-4: one rule field without the other is incomplete — the pair is answered together or not at all", () => {
  const got = validate("event", {
    standing_or_drop_in: false,
    verdict: "opportunity",
    why: "x",
    confidence: 0.9,
  }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("event-4: a verdict without the rule fields (a promoted rule's shape) is judged as before", () => {
  // `judge_pipeline.ts` validates a tier-2 rule's stored verdict through this same function, and a
  // promoted rule carries only verdict/why/confidence — it must keep answering.
  const got = validate("event", { verdict: "drop", why: "promoted rule", confidence: 1 }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict, { verdict: "drop", why: "promoted rule", confidence: 1 });
});

Deno.test("event-4: the templated whys are ledger-safe one-liners under the reason bound", () => {
  for (const why of Object.values(EVENT_RULE_WHY)) {
    assertEquals(oneLine(why, MAX_REASON_CHARS), why);
    assertEquals(why.startsWith("drop rule:"), true, why);
  }
});
