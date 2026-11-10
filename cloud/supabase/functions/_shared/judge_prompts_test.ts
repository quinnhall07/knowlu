// TDD, per CLAUDE.md: RED until `judge_prompts.ts` gains the clip marker, the email deadline
// rule, the empty-body line and the scrub site (provider swap, Task 3). There was no
// `judge_prompts_test.ts` before this task; the prompt shape was exercised only indirectly, through
// `judge_pipeline_test.ts` and the handlers' own tests, none of which pin prompt TEXT.
import { assert, assertEquals } from "@std/assert";
import {
  buildMailClaimPrompt,
  buildMailResolvePrompt,
  buildPrompt,
  MAX_BODY_CHARS,
  promptHash,
  schemaFor,
  systemTemplate,
} from "./judge_prompts.ts";

// ---------------------------------------------------------------------------------------------
// (a) clipping appends " …[truncated]" when text exceeds MAX_BODY_CHARS, and appends nothing
// otherwise — checked through the one path common to all three kinds: `buildPrompt`'s own use of
// `clip` on the body-shaped field each kind carries (task Body, event Description, email Message).
// ---------------------------------------------------------------------------------------------

Deno.test("task: a body under MAX_BODY_CHARS is not marked truncated", () => {
  const body = "a".repeat(MAX_BODY_CHARS - 1);
  const { user } = buildPrompt("task", { title: "PS3", due: null, body }, {});
  assert(user.includes(body), "the body should reach the prompt whole");
  assert(!user.includes("[truncated]"), "a body under the bound must not be marked truncated");
});

Deno.test("task: a body over MAX_BODY_CHARS is clipped and marked truncated", () => {
  const body = "a".repeat(MAX_BODY_CHARS + 500);
  const { user } = buildPrompt("task", { title: "PS3", due: null, body }, {});
  assert(user.includes(" …[truncated]"), `expected the truncation marker in: ${user}`);
  // The clipped text itself is exactly MAX_BODY_CHARS characters, followed by the marker.
  const clipped = "a".repeat(MAX_BODY_CHARS);
  assert(user.includes(`${clipped} …[truncated]`), `expected the clipped body in: ${user}`);
  assert(!user.includes("a".repeat(MAX_BODY_CHARS + 1)), "the body must actually be cut");
});

// Fix round 1, item 5 (Task 3 review): the marker falls BEYOND the bound, not inside it -- a
// clipped body emits MAX_BODY_CHARS content characters plus the marker's own 13, never
// MAX_BODY_CHARS total. Pinned with an exact count so "the same four bounds" language in the
// header stays honest about what the bound measures (the frame the model is told to trust, not
// the total bytes on the wire).
Deno.test("task: the marker's 13 characters land beyond MAX_BODY_CHARS, not inside it", () => {
  const body = "a".repeat(MAX_BODY_CHARS + 500);
  const { user } = buildPrompt("task", { title: "PS3", due: null, body }, {});
  const marker = " …[truncated]";
  assertEquals([...marker].length, 13, "sanity: the marker itself is 13 characters");
  const emitted = user.slice(user.indexOf("Body:\n") + "Body:\n".length);
  assertEquals([...emitted].length, MAX_BODY_CHARS + 13, "content + marker, not content alone");
});

Deno.test("event: a description over MAX_BODY_CHARS is clipped and marked truncated", () => {
  const description = "e".repeat(MAX_BODY_CHARS + 10);
  const { user } = buildPrompt("event", {
    title: "Fair",
    start: "2026-10-05T10:00",
    end: "2026-10-05T14:00",
    source: "campus-calendar",
    description,
  }, {});
  assert(user.includes(" …[truncated]"), `expected the truncation marker in: ${user}`);
});

Deno.test("event: a description under MAX_BODY_CHARS is not marked truncated", () => {
  const description = "e".repeat(10);
  const { user } = buildPrompt("event", {
    title: "Fair",
    start: "2026-10-05T10:00",
    end: "2026-10-05T14:00",
    source: "campus-calendar",
    description,
  }, {});
  assert(!user.includes("[truncated]"));
});

Deno.test("email: a message over MAX_BODY_CHARS is clipped and marked truncated", () => {
  const text = "m".repeat(MAX_BODY_CHARS + 10);
  const { user } = buildPrompt("email", {
    subject: "Reminder",
    from: "lms@example.invalid",
    date: "2026-10-02",
    text,
  }, {});
  assert(user.includes(" …[truncated]"), `expected the truncation marker in: ${user}`);
});

Deno.test("email: a message under MAX_BODY_CHARS is not marked truncated", () => {
  const text = "m".repeat(10);
  const { user } = buildPrompt("email", {
    subject: "Reminder",
    from: "lms@example.invalid",
    date: "2026-10-02",
    text,
  }, {});
  assert(!user.includes("[truncated]"));
});

// ---------------------------------------------------------------------------------------------
// (b) T4: the model is EXTRACTION ONLY for `due` — it must not resolve a relative deadline
// itself (`judge_due.ts` does that, between the model and `validate`). The old assertion here
// pinned the opposite instruction ("resolve a relative deadline... against the Date line") and is
// replaced, not kept alongside the new one: that sentence must be GONE from the prompt, not merely
// supplemented, or the model would see both instructions at once.
// ---------------------------------------------------------------------------------------------

Deno.test("email rules: the due bullet asks for the phrase as written, not a resolved date", () => {
  const rules = systemTemplate("email");
  assert(rules.includes("exactly as the email states it"), `missing sentence in: ${rules}`);
  assert(rules.includes("do NOT compute or resolve it yourself"), `missing sentence in: ${rules}`);
  assert(!rules.includes("Resolve a relative deadline"), `the old resolve-it-yourself rule is still present: ${rules}`);
});

// ---------------------------------------------------------------------------------------------
// (c) an empty email body: the user message names it, and the rules say how to judge one.
// ---------------------------------------------------------------------------------------------

Deno.test("email: an empty text yields 'Message: (no plain-text body)' in the user message", () => {
  const { user } = buildPrompt("email", {
    subject: "Reminder",
    from: "lms@example.invalid",
    date: "2026-10-02",
    text: "",
  }, {});
  assert(user.includes("Message: (no plain-text body)"), `expected the empty-body line in: ${user}`);
});

// Fix round 1, item 4 (Task 3 review): a body of nothing but whitespace reads as empty too. This
// works because `buildPrompt` trims BEFORE it scrubs (`str(item.text).trim()`, then
// `scrubForPrompt`, then `clip`) -- `"   \n  ".trim()` is `""`, so the same empty-string branch
// that (a) above exercises fires here, with no separate whitespace-detection code needed.
Deno.test("email: a whitespace-only text also yields 'Message: (no plain-text body)'", () => {
  const { user } = buildPrompt("email", {
    subject: "Reminder",
    from: "lms@example.invalid",
    date: "2026-10-02",
    text: "   \n  ",
  }, {});
  assert(user.includes("Message: (no plain-text body)"), `expected the empty-body line in: ${user}`);
});

Deno.test("email rules: an empty body is named, with a confidence ceiling", () => {
  const rules = systemTemplate("email");
  assert(rules.includes("with no message body"), `missing sentence in: ${rules}`);
});

// ---------------------------------------------------------------------------------------------
// (d) the three prompt_version strings. `prompt_version` is recorded from the `models` row
// (`judge_pipeline.ts:165`), not a code constant — there is nothing to read or assert here in
// `judge_prompts.ts` itself. `cloud/supabase/migrations/migrations_test.ts` already pins
// `task-2` / `event-2` / `email-2` (Tasks 1-2's re-pin, `20260916000100_provider_swap.sql`); this
// suite does not duplicate that assertion, it only points at it.
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// Step 3: the email item's `subject` and `text` reach `buildPrompt` scrubbed. `From` is NEVER
// scrubbed — `judge_rules.ts`'s `featureMap` reads the sender for promotion, and `scrub` replaces
// email addresses, which would break that lookup silently.
// ---------------------------------------------------------------------------------------------

Deno.test("email: a URL with a token-looking query string in subject and text is scrubbed before buildPrompt", () => {
  const url = "https://portal.example.invalid/reset?token=abc123XYZ789def456";
  const { user } = buildPrompt("email", {
    subject: `Reset your account: ${url}`,
    from: "noreply@example.invalid",
    date: "2026-10-02",
    text: `Click ${url} to continue.`,
  }, {});
  assert(!user.includes(url), `the raw URL must not reach the prompt: ${user}`);
  const occurrences = user.split("<url>").length - 1;
  assert(occurrences >= 2, `expected the URL placeholder at least twice in: ${user}`);
});

Deno.test("email: a long bearer-shaped string in subject and text is scrubbed before buildPrompt", () => {
  const bearer = "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdef1234567890";
  const { user } = buildPrompt("email", {
    subject: `Your session: ${bearer}`,
    from: "noreply@example.invalid",
    date: "2026-10-02",
    text: `Authorization: ${bearer}`,
  }, {});
  assert(!user.includes(bearer), `the raw bearer token must not reach the prompt: ${user}`);
  assert(user.includes("<secret>"), `expected the credential placeholder in: ${user}`);
});

// ---------------------------------------------------------------------------------------------
// CHECKPOINT J-1 (ruled 2026-09-22): the event schema and prompt teach `unsure` as a real answer.
// ---------------------------------------------------------------------------------------------

Deno.test("event schema: the verdict enum carries unsure as a fourth word", () => {
  const schema = schemaFor("event") as { properties: { verdict: { enum: string[] } } };
  assertEquals(schema.properties.verdict.enum, ["obligation", "opportunity", "drop", "unsure"]);
});

Deno.test("event rules: unsure is named and told not to guess the audience", () => {
  const rules = systemTemplate("event");
  assert(rules.includes("unsure"), `missing 'unsure' in: ${rules}`);
  assert(rules.includes("do not guess"), `missing the no-guessing rule in: ${rules}`);
});

Deno.test("email: From is never scrubbed, even though it is an email address", () => {
  const { user } = buildPrompt("email", {
    subject: "Reminder",
    from: "registrar@example.invalid",
    date: "2026-10-02",
    text: "See you Friday.",
  }, {});
  assert(user.includes("From: registrar@example.invalid"), `From must survive unscrubbed: ${user}`);
});

// ---------------------------------------------------------------------------------------------
// Stream J Task T9: the sixth email tier. Folded into T4's `email-3` (nothing deployed between
// the two), so `email-3`'s hash is the one computed here, not T4's draft.
// ---------------------------------------------------------------------------------------------

/** T4's `email-3` draft hash, before T9 added the tier. Pinned so the fold is provably a change. */
const T4_EMAIL_DRAFT_HASH = "c4bb4925b8104c38b064ad4d3b8535181838a6dfcb433a33fe43a6f361a7094d";

Deno.test("T9: the email schema's tier enum names completion", () => {
  const tier = (schemaFor("email").properties as Record<string, { enum: string[] }>).tier;
  assertEquals(tier.enum, ["task", "borderline", "event", "opportunity", "information", "completion"]);
});

Deno.test("T9: the email rules define completion, name the work as its title, and exclude a posted grade", () => {
  const rules = systemTemplate("email");
  assert(rules.includes("into exactly one of six tiers"), `tier count not updated in: ${rules}`);
  assert(
    rules.includes("- completion: this email confirms the student already submitted or finished a specific piece of work."),
    `missing completion tier line in: ${rules}`,
  );
  assert(rules.includes("for completion, the name of that piece of work"), `missing completion title rule in: ${rules}`);
  assert(rules.includes("A grade or feedback being posted is not completion"), `missing grade exclusion in: ${rules}`);
  // `information` no longer claims receipts wholesale, or the two tiers would contradict each other.
  assert(!rules.includes("including receipts,"), `information still swallows every receipt: ${rules}`);
});

Deno.test("T9: email-3's hash moved off T4's draft", async () => {
  assert((await promptHash("email")) !== T4_EMAIL_DRAFT_HASH, "the email prompt hash did not change");
  // email-3 as shipped (T4 + T9). Any edit to the email template or schema moves this and must
  // come with a new prompt_version in a migration. The task hash is pinned in the task-3 section
  // below (effort anchors, 2026-10-23), which moved it.
  assertEquals(await promptHash("email"), "ad890b53c5e984a8bd7ac03eff7170085cbb8df17e316e7d8d48ad6c15fa2405");
  // event-3 (stream J T1, the `unsure` verdict), pinned at integration: T9 was written on a branch
  // without T1, where the event prompt was still event-2 (0faedc21...). T9 does not touch it.
  assertEquals(await promptHash("event"), "b6324351b0a70976f818930a3dd2e1e66478c804f4a8ff01d4a9123b674b5e92");
});

// ---------------------------------------------------------------------------------------------
// task-3 (effort anchors, 2026-10-23). task-2's one effort line ("the realistic time in hours to
// finish this one assignment, 0.25 to 40") gave the model no scale: a fresh vault's twenty
// Blackboard items, judged from a title, a course slug and a due date, came back at a median of
// 8 hours and a maximum of 24. task-3 adds anchors by type, a ceiling rule for "above 8", a rule
// for a vague title with nothing else given, and the points/weight rule. The schema, the
// 0.25-40 clamp and the `{slice_hours}` placeholder do not move.
// ---------------------------------------------------------------------------------------------

/** task-2's hash (`20260916000100_provider_swap.sql`'s re-pin), pinned so task-3 provably moved it. */
const TASK_2_HASH = "ee59545ad9910231b22debe10ce9ed9c34f4d0fe043dc6aeb3fe11a4f9e02ca6";

// Revised after PR #61's per-case eval (task-3 draft vs task-2, granite-4.2-8b): given RANGES the
// model answered at or past the top (homework 1-3 -> 2.5-3.5, milestone 3-8 -> 8, exam 3-6 ->
// 12 and 24, a 12-page paper at 1-2 per page -> 40), so each type now has one typical value.
Deno.test("task-3: the effort rule gives one typical value per assignment type", () => {
  const rules = systemTemplate("task");
  assert(rules.includes("for a typical undergraduate"), `missing the typical-student frame in: ${rules}`);
  for (
    const anchor of [
      "a reading, discussion post or quiz 0.5",
      "a homework, problem set, worksheet or lab 2",
      "a short essay or report 3",
      "a longer paper 1.5 per page",
      "a project milestone 5",
      "an exam, midterm or final exam 4 of study",
    ]
  ) {
    assert(rules.includes(anchor), `missing anchor '${anchor}' in: ${rules}`);
  }
});

Deno.test("task-3: above 8 needs a named large project or long paper, an exam never passes 6, a vague title is 2", () => {
  const rules = systemTemplate("task");
  assert(rules.includes("answer above 8 only for a large project or a paper of more than 5 pages"), rules);
  assert(rules.includes("An exam is never above 6."), `missing the exam ceiling in: ${rules}`);
  assert(rules.includes("a vague title with nothing else given: answer 2, never a large number."), rules);
  assert(rules.includes("points, when given, scale effort_hours only: a 5-point item is small."), rules);
});

// The draft's "points or a grade weight ... scale the estimate" line and its size anchors leaked
// into importance: with no weights given, task-2 answered the rule's 3 on 16 of 19 cases and the
// draft on 9 of 19 — 2 for every quiz-sized item, 4-5 for exams, papers and projects.
Deno.test("task-3: importance comes from the grade weights only, never from size or effort", () => {
  const rules = systemTemplate("task");
  assert(
    rules.includes(
      "- importance: 1 to 5, grounded only in the grade weights given, never in the assignment's size or effort. With no weights given, answer 3 and say so in importance_reason.",
    ),
    rules,
  );
  assert(!rules.includes("grade weight, when given, scale"), `weights must not scale effort: ${rules}`);
});

Deno.test("task-3: the vague-title rule never asks for a confidence under the 0.6 floor", () => {
  // A task answer under CONFIDENCE_FLOOR is `below floor`: no verdict at all, and the note is
  // re-asked every slot. The device already writes every model estimate as `effort_confidence:
  // low`, so the prompt must not push an honest middle-of-range answer under the floor.
  const rules = systemTemplate("task");
  assert(!/confidence (at or )?below/i.test(rules), `the task rules must not ask for a sub-floor confidence: ${rules}`);
  assert(rules.includes("- confidence: 0 to 1, how sure you are of effort_hours and importance."), rules);
});

Deno.test("task-3: the clamp, the slice placeholder and the schema are unchanged", () => {
  const rules = systemTemplate("task");
  assert(rules.includes("0.25 to 40."), `the 0.25-40 bound left the effort rule: ${rules}`);
  assertEquals(rules.split("{slice_hours}").length - 1, 1, "exactly one {slice_hours} placeholder");
  const rendered = buildPrompt("task", { title: "Homework 4" }, { slice_hours: 2 }).system;
  assert(rendered.includes("is 2 hours."), `the slice did not render: ${rendered}`);
  assert(!rendered.includes("{slice_hours}"), "the placeholder survived rendering");
  assertEquals(schemaFor("task"), {
    type: "object",
    properties: {
      course: { type: ["string", "null"] },
      effort_hours: { type: "number" },
      importance: { type: "integer", enum: [1, 2, 3, 4, 5] },
      importance_reason: { type: "string" },
      confidence: { type: "number" },
    },
    required: ["course", "effort_hours", "importance", "importance_reason", "confidence"],
    additionalProperties: false,
  });
});

Deno.test("task-3: the task hash moved off task-2 and is pinned", async () => {
  assert((await promptHash("task")) !== TASK_2_HASH, "the task prompt hash did not change");
  // task-3 as shipped, `<stamp>_task_effort_anchors.sql`. Any edit to the task template or schema
  // moves this and must come with a new prompt_version in a migration.
  assertEquals(await promptHash("task"), "03a48a2da8b6d7523eb9176b049bb00977f81fdba3f7e4b538343e8a3faa927f");
});

// ---------------------------------------------------------------------------------------------
// Every single-line item field reaches the user message as one line, cut to its own bound. A line
// break inside a field (LF, CRLF, U+2028, U+2029, U+0085) becomes a space, so the field's text
// never starts a line of its own. The multi-line blocks (Body, Description, Message, the seed's
// preferences, weights and interests) are out of scope here and keep their `clip` bounds.
// ---------------------------------------------------------------------------------------------

/** Every code point a reader of the prompt may take as a line boundary (`str.splitlines`'s set). */
const LINE_BOUNDARY = new Set([0x0a, 0x0b, 0x0c, 0x0d, 0x1c, 0x1d, 0x1e, 0x85, 0x2028, 0x2029]);
const BREAKS = ["\n", "\r\n", "\u2028", "\u2029", "\u0085"];

/** The prompt split at every line boundary, CRLF counted once. */
function promptLines(user: string): string[] {
  const lines = [""];
  for (let i = 0; i < user.length; i++) {
    const c = user.charCodeAt(i);
    if (!LINE_BOUNDARY.has(c)) {
      lines[lines.length - 1] += user[i];
      continue;
    }
    if (c === 0x0d && user.charCodeAt(i + 1) === 0x0a) i++;
    lines.push("");
  }
  return lines;
}

const EVENT_ITEM = {
  title: "Career fair",
  start: "2026-10-05T10:00",
  end: "2026-10-05T14:00",
  source: "department-calendar",
};
const EMAIL_ITEM = {
  subject: "Quiz 2 moved",
  from: "Course Notices <notices@lms.example.invalid>",
  date: "Thu, 01 Oct 2026 09:00:00 -0500",
  text: "Quiz 2 moves to Friday.",
};

interface FieldCase {
  name: string;
  bound: number;
  /** The user message with `value` in the field under test. */
  build: (value: string) => string;
  /** The line that field should produce once its value is `value`. */
  line: (value: string) => string;
}

const FIELD_CASES: FieldCase[] = [
  {
    name: "task Due",
    bound: 40,
    build: (v) => buildPrompt("task", { title: "PS3", due: v }, {}).user,
    line: (v) => `Due: ${v}`,
  },
  {
    name: "task Title",
    bound: 200,
    build: (v) => buildPrompt("task", { title: v }, {}).user,
    line: (v) => `Title: ${v}`,
  },
  {
    name: "task Course slug",
    bound: 40,
    build: (v) => buildPrompt("task", { title: "PS3" }, { course: v }).user,
    line: (v) => `Course slug: ${v}`,
  },
  {
    name: "event When start",
    bound: 40,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, start: v }, {}).user,
    line: (v) => `When: ${v} to 2026-10-05T14:00`,
  },
  {
    name: "event When end",
    bound: 40,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, end: v }, {}).user,
    line: (v) => `When: 2026-10-05T10:00 to ${v}`,
  },
  {
    name: "event Source",
    bound: 120,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, source: v }, {}).user,
    line: (v) => `Source: ${v}`,
  },
  {
    name: "event Categories entry",
    bound: 60,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, categories: [v, "career"] }, {}).user,
    line: (v) => `Categories: ${v}, career`,
  },
  {
    name: "event Audiences entry",
    bound: 60,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, audiences: ["Undergraduate Students", v] }, {}).user,
    line: (v) => `Audiences: Undergraduate Students, ${v}`,
  },
  {
    name: "event Title",
    bound: 200,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, title: v }, {}).user,
    line: (v) => `Title: ${v}`,
  },
  {
    name: "event Organizer",
    bound: 120,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, organizer: v }, {}).user,
    line: (v) => `Organizer: ${v}`,
  },
  {
    name: "event Location",
    bound: 120,
    build: (v) => buildPrompt("event", { ...EVENT_ITEM, location: v }, {}).user,
    line: (v) => `Location: ${v}`,
  },
  {
    name: "email Date",
    bound: 80,
    build: (v) => buildPrompt("email", { ...EMAIL_ITEM, date: v }, {}).user,
    line: (v) => `Date: ${v}`,
  },
  {
    name: "email Subject",
    bound: 200,
    build: (v) => buildPrompt("email", { ...EMAIL_ITEM, subject: v }, {}).user,
    line: (v) => `Subject: ${v}`,
  },
  {
    name: "email From",
    bound: 200,
    build: (v) => buildPrompt("email", { ...EMAIL_ITEM, from: v }, {}).user,
    line: (v) => `From: ${v}`,
  },
  {
    name: "email Known course slugs entry",
    bound: 40,
    build: (v) => buildPrompt("email", EMAIL_ITEM, { known_courses: ["math-221", v] }).user,
    line: (v) => `Known course slugs: math-221, ${v}`,
  },
  {
    name: "mail_claim Subject",
    bound: 200,
    build: (v) => buildMailClaimPrompt({ ...EMAIL_ITEM, subject: v }, {}).user,
    line: (v) => `Subject: ${v}`,
  },
  {
    name: "mail_claim From",
    bound: 200,
    build: (v) => buildMailClaimPrompt({ ...EMAIL_ITEM, from: v }, {}).user,
    line: (v) => `From: ${v}`,
  },
  {
    name: "mail_claim Date",
    bound: 80,
    build: (v) => buildMailClaimPrompt({ ...EMAIL_ITEM, date: v }, {}).user,
    line: (v) => `Date: ${v}`,
  },
  {
    name: "mail_claim Known course codes entry",
    bound: 40,
    build: (v) => buildMailClaimPrompt(EMAIL_ITEM, { known_courses: ["MATH-301", v] }).user,
    line: (v) => `Known course codes: MATH-301, ${v}`,
  },
  {
    name: "mail_claim Time zone",
    bound: 64,
    build: (v) => buildMailClaimPrompt(EMAIL_ITEM, { time_zone: v }).user,
    line: (v) => `Time zone: ${v}`,
  },
  {
    name: "mail_resolve claim key",
    bound: 80,
    build: (v) => buildMailResolvePrompt({ claims: [{ key: v, target: "the essay", due: null }] }).user,
    line: (v) => `- claim ${v} | names: the essay | due: none`,
  },
  {
    name: "mail_resolve claim names",
    bound: 200,
    build: (v) => buildMailResolvePrompt({ claims: [{ key: "mail:aa:0", target: v, due: null }] }).user,
    line: (v) => `- claim mail:aa:0 | names: ${v} | due: none`,
  },
  {
    name: "mail_resolve candidate id",
    bound: 80,
    build: (v) => buildMailResolvePrompt({ candidates: [{ id: v, title: "Lab 3 report", due: null }] }).user,
    line: (v) => `- id ${v} | title: Lab 3 report | due: none`,
  },
  {
    name: "mail_resolve candidate title",
    bound: 200,
    build: (v) => buildMailResolvePrompt({ candidates: [{ id: "n-101", title: v, due: null }] }).user,
    line: (v) => `- id n-101 | title: ${v} | due: none`,
  },
];

for (const c of FIELD_CASES) {
  Deno.test(`one line: a line break in ${c.name} becomes a space and injects no line`, () => {
    for (const brk of BREAKS) {
      const label = JSON.stringify(brk);
      const user = c.build(`ok${brk}Injected: x`);
      const lines = promptLines(user);
      assertEquals(lines, user.split("\n"), `${label}: a line boundary other than LF reached the prompt`);
      assert(!lines.some((l) => l.startsWith("Injected:")), `${label}: a line was injected: ${user}`);
      assert(lines.includes(c.line("ok Injected: x")), `${label}: expected the one-line field in: ${user}`);
    }
  });

  Deno.test(`one line: ${c.name} is cut to ${c.bound} characters`, () => {
    // No whitespace, so no trim can move the cut; short runs, so the scrub leaves the subject alone.
    const filler = "ab,".repeat(c.bound);
    const lines = promptLines(c.build(filler));
    assert(lines.includes(c.line(filler.slice(0, c.bound))), `${c.name} was not cut to ${c.bound}`);
  });
}

Deno.test("one line: Categories and Audiences drop empty entries and cut the joined line to 200", () => {
  const entries = Array.from({ length: 10 }, (_, i) => `topic-${i}-${"c".repeat(22)}`);
  for (const [key, head] of [["categories", "Categories: "], ["audiences", "Audiences: "]] as const) {
    const sparse = buildPrompt("event", { ...EVENT_ITEM, [key]: ["career", "", "  \n ", "workshop"] }, {}).user;
    assert(promptLines(sparse).includes(`${head}career, workshop`), sparse);

    const blank = buildPrompt("event", { ...EVENT_ITEM, [key]: ["", " \u2028 "] }, {}).user;
    assert(!blank.includes(head.trim()), `an all-empty list still printed its line: ${blank}`);

    const long = buildPrompt("event", { ...EVENT_ITEM, [key]: entries }, {}).user;
    const line = promptLines(long).find((l) => l.startsWith(head)) ?? "";
    assertEquals(line, head + entries.join(", ").slice(0, 200));
  }
});

Deno.test("one line: ordinary prompts are byte-for-byte what they were", () => {
  assertEquals(
    buildPrompt("task", { title: "Problem set 4", due: "2026-11-02T23:59", body: "Chapters 3 and 4." }, {
      course: "math-221",
      weights: "Homework 25%\nExams 75%",
      slice_hours: 2,
    }).user,
    [
      "Course slug: math-221",
      "Grade weights:",
      "Homework 25%",
      "Exams 75%",
      "Title: Problem set 4",
      "Due: 2026-11-02T23:59",
      "Body:",
      "Chapters 3 and 4.",
    ].join("\n"),
  );
  assertEquals(buildPrompt("task", { title: "PS3", due: null }, {}).user, "Course slug: null\nTitle: PS3");
  assertEquals(
    buildPrompt("event", {
      ...EVENT_ITEM,
      organizer: "Career Center",
      location: "Union ballroom",
      categories: ["career", "networking"],
      audiences: ["Undergraduate Students"],
      description: "Bring a resume.",
    }, { interests: "internships" }).user,
    [
      "Title: Career fair",
      "When: 2026-10-05T10:00 to 2026-10-05T14:00",
      "Source: department-calendar",
      "Organizer: Career Center",
      "Location: Union ballroom",
      "Categories: career, networking",
      "Audiences: Undergraduate Students",
      "Description:",
      "Bring a resume.",
      "The student's stated interests:",
      "internships",
    ].join("\n"),
  );
  assertEquals(
    buildPrompt("email", EMAIL_ITEM, { known_courses: ["math-221", "chem-101"] }).user,
    [
      "Subject: Quiz 2 moved",
      "From: Course Notices <notices@lms.example.invalid>",
      "Date: Thu, 01 Oct 2026 09:00:00 -0500",
      "Known course slugs: math-221, chem-101",
      "Message:",
      "Quiz 2 moves to Friday.",
    ].join("\n"),
  );
});
