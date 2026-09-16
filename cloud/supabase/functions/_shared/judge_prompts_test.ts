// TDD, per CLAUDE.md: RED until `judge_prompts.ts` gains the clip marker, the email deadline
// rule, the empty-body line and the scrub site (provider swap, Task 3). There was no
// `judge_prompts_test.ts` before this task; the prompt shape was exercised only indirectly, through
// `judge_pipeline_test.ts` and the handlers' own tests, none of which pin prompt TEXT.
import { assert } from "@std/assert";
import { buildPrompt, MAX_BODY_CHARS, systemTemplate } from "./judge_prompts.ts";

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
// (b) the email rules text names how to resolve a relative deadline, against the Date line.
// ---------------------------------------------------------------------------------------------

Deno.test("email rules: the due bullet resolves a relative deadline against the Date line", () => {
  const rules = systemTemplate("email");
  assert(rules.includes("Resolve a relative deadline"), `missing sentence in: ${rules}`);
  assert(rules.includes("against the Date line above"), `missing sentence in: ${rules}`);
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

Deno.test("email: From is never scrubbed, even though it is an email address", () => {
  const { user } = buildPrompt("email", {
    subject: "Reminder",
    from: "registrar@example.invalid",
    date: "2026-10-02",
    text: "See you Friday.",
  }, {});
  assert(user.includes("From: registrar@example.invalid"), `From must survive unscrubbed: ${user}`);
});
