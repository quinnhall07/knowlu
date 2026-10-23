// TDD, per CLAUDE.md: RED until `judge_prompts.ts` gains the clip marker, the email deadline
// rule, the empty-body line and the scrub site (provider swap, Task 3). There was no
// `judge_prompts_test.ts` before this task; the prompt shape was exercised only indirectly, through
// `judge_pipeline_test.ts` and the handlers' own tests, none of which pin prompt TEXT.
import { assert, assertEquals } from "@std/assert";
import { buildPrompt, MAX_BODY_CHARS, promptHash, schemaFor, systemTemplate } from "./judge_prompts.ts";

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
