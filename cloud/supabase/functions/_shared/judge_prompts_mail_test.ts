// Email plan T3c: the `mail-1` prompt and grammar (spec §4.7 step 4, §5.1 as amended 2026-10-02) and the
// `mail_resolve` prompt (D12). Every address, name and text here is synthetic.
import { assert, assertEquals } from "@std/assert";
import {
  buildMailClaimPrompt,
  buildMailResolvePrompt,
  MAIL_TEXT_CHARS,
  mailPromptHash,
  mailPromptText,
  mailSchemaFor,
  mailSystemTemplate,
  promptHash,
} from "./judge_prompts.ts";
import { scrubForPrompt } from "./scrub.ts";

const URL = "https://lms.example.invalid/reset?token=abc123XYZ789def456";
const MARKER = "zq-marker-7f3a";

function section(user: string, head: string): string {
  return user.slice(user.indexOf(head) + head.length);
}

Deno.test("the_mail_claim_prompt_holds_only_scrubbed_text_cut_to_2000_characters", () => {
  const text = `  Open ${URL} now.\n${"x".repeat(2500)} Bearer eyJhbGciOiJIUzI1NiJ9.tail`;
  const item = {
    subject: `Due date moved\r\nsee ${URL}`,
    from: "Course Notices <notices@lms.example.invalid>",
    date: "Thu, 01 Oct 2026 09:00:00 -0500",
    text,
    // Nothing but the four fields above reaches the model, whatever else the caller holds.
    to: `${MARKER}-to@example.invalid`,
    message_id: `<${MARKER}-mid@example.invalid>`,
    html: `<p>${MARKER}-html</p>`,
  };
  const seed = { known_courses: ["MATH-301", "CHEM-105"], time_zone: "America/Chicago", note: MARKER };
  const { user } = buildMailClaimPrompt(item, seed);

  const kept = mailPromptText(text);
  assertEquals(MAIL_TEXT_CHARS, 2000);
  assertEquals([...kept].length, MAIL_TEXT_CHARS);
  assertEquals(kept, [...scrubForPrompt(text.trim())].slice(0, MAIL_TEXT_CHARS).join(""));
  assertEquals(section(user, "Message:\n"), `${kept} …[truncated]`);
  assert(!user.includes(URL), `the raw URL reached the prompt: ${user}`);
  assert(!user.includes(MARKER), "a field outside subject, from, date and text reached the prompt");
  assertEquals(user.split("\n").slice(0, 6), [
    "Subject: Due date moved see <url>",
    "From: Course Notices <notices@lms.example.invalid>",
    "Date: Thu, 01 Oct 2026 09:00:00 -0500",
    "Known course codes: MATH-301, CHEM-105",
    "Time zone: America/Chicago",
    "Message:",
  ]);

  // A short text is sent whole, with no marker; an empty one is named.
  const short = buildMailClaimPrompt({ ...item, text: "Quiz 2 moves to Friday." }, seed).user;
  assertEquals(section(short, "Message:\n"), "Quiz 2 moves to Friday.");
  const empty = buildMailClaimPrompt({ ...item, text: " \n " }, {}).user;
  assert(empty.endsWith("Message: (no plain-text body)"), empty);
  assert(!empty.includes("Known course codes") && !empty.includes("Time zone"), empty);
});

Deno.test("the_resolve_prompt_holds_only_ids_titles_and_dues", () => {
  const { user, system } = buildMailResolvePrompt({
    claims: [
      { key: "mail:aa:0", target: "Lab 3\nwriteup", due: "2026-10-09", evidence: MARKER, why: MARKER },
      { key: "mail:bb:1", target: "the essay", due: null, sender_addr: `${MARKER}@example.invalid` },
    ],
    candidates: [
      { id: "n-101", title: "Lab 3 report", due: "2026-10-08T23:59", body: MARKER },
      { id: "n-102", title: "Lab 3 prelab", due: null, course: MARKER },
    ],
    course: MARKER,
  });
  assertEquals(
    user,
    [
      "Claims:",
      "- claim mail:aa:0 | names: Lab 3 writeup | due: 2026-10-09",
      "- claim mail:bb:1 | names: the essay | due: none",
      "Candidates:",
      "- id n-101 | title: Lab 3 report | due: 2026-10-08T23:59",
      "- id n-102 | title: Lab 3 prelab | due: none",
    ].join("\n"),
  );
  assert(!system.includes(MARKER));
});

Deno.test("the_mail_1_grammar_offers_no_receipt_kind", () => {
  const schema = mailSchemaFor("mail_claim") as {
    properties: { claims: { items: { properties: { kind: { enum: string[] } } } } };
  };
  assertEquals(schema.properties.claims.items.properties.kind.enum, [
    "new_item",
    "change",
    "course_info",
    "noise",
  ]);
  assert(!JSON.stringify(schema).includes("receipt"), "the mail-1 grammar names a receipt");
  assert(!/^- receipt/m.test(mailSystemTemplate("mail_claim")), "the mail-1 rules offer a receipt kind");
});

/** Every object in a strict structured-output schema lists all its properties as required and allows no
 * others; a nested object that does not is refused by the provider before any reply. */
function strict(node: unknown, path: string): void {
  if (node === null || typeof node !== "object") return;
  const n = node as Record<string, unknown>;
  if (n.properties !== undefined) {
    assertEquals(n.additionalProperties, false, `${path}: additionalProperties`);
    assertEquals(n.required, Object.keys(n.properties as object), `${path}: required`);
  }
  for (const [k, v] of Object.entries(n)) strict(v, `${path}.${k}`);
}

Deno.test("both mail grammars are strict, and the hashes are their own", async () => {
  strict(mailSchemaFor("mail_claim"), "mail_claim");
  strict(mailSchemaFor("mail_resolve"), "mail_resolve");
  const claim = await mailPromptHash("mail_claim");
  const resolve = await mailPromptHash("mail_resolve");
  assert(claim !== resolve);
  for (const kind of ["task", "event", "email"] as const) assert(claim !== await promptHash(kind));
});
