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

Deno.test("T9: email-3's hash moved off T4's draft; task and event hashes did not move", async () => {
  assert((await promptHash("email")) !== T4_EMAIL_DRAFT_HASH, "the email prompt hash did not change");
  // email-3 as shipped (T4 + T9). Any edit to the email template or schema moves this and must
  // come with a new prompt_version in a migration.
  assertEquals(await promptHash("email"), "ad890b53c5e984a8bd7ac03eff7170085cbb8df17e316e7d8d48ad6c15fa2405");
  assertEquals(await promptHash("task"), "ee59545ad9910231b22debe10ce9ed9c34f4d0fe043dc6aeb3fe11a4f9e02ca6");
  assertEquals(await promptHash("event"), "0faedc21498a0f4127aee5095f4dcae8700e0c7e3dbec079d1b4cc6b96f61928");
});
