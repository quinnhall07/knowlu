// Stream J Task T9: the deterministic LMS-receipt recogniser. Every fixture here is FABRICATED in
// the shape of the real templated mail (course codes, titles, ids and confirmation numbers are
// invented) — never a real student's course or assignment.
import { assert, assertEquals } from "@std/assert";
import { RECEIPT_TEMPLATES, receiptVerdict, recogniseReceipt } from "./lms_receipts.ts";

const BB = "Blackboard <do-not-reply@blackboard.com>";

const RECEIPT_BODY = [
  "12345.202640 202640-XX-101-001",
  "Assessment submitted",
  "Lab 3: Pendulum",
  "Submitted: Thursday, September 3, 2026 2:15:57 PM CDT",
  "Confirmation number: 0f1e2d3c4b5a69788796a5b4c3d2e1f0",
].join("\n");

interface Row {
  name: string;
  subject: string;
  from: string;
  text: string;
  /** The extracted title, or null when the message is not a receipt this recogniser knows. */
  want: string | null;
}

const TABLE: Row[] = [
  { name: "shape 1, multi-line body", subject: "Submission received", from: BB, text: RECEIPT_BODY, want: "Lab 3: Pendulum" },
  {
    name: "shape 1, one-line snippet form",
    subject: "Submission received",
    from: BB,
    text:
      "12345.202640 202640-XX-101-001 Assessment submitted Lab 3: Pendulum Submitted: Thursday, September 3, 2026 2:15:57 PM CDT Confirmation number: 0f1e2d3c",
    want: "Lab 3: Pendulum",
  },
  {
    name: "shape 1, course code twice and a title carrying double quotes",
    subject: "Submission received",
    from: BB,
    text: 'XX-202-002 XX-202-002\r\nAssessment submitted\r\nOffice Space "Quiz"\r\n' +
      "Submitted: Monday, September 14, 2026 11:59:01 PM CDT\r\nConfirmation number: abc123def456",
    want: 'Office Space "Quiz"',
  },
  {
    name: "shape 1, a bare sender address in another case",
    subject: "Submission received",
    from: "DO-NOT-REPLY@Blackboard.com",
    text: RECEIPT_BODY,
    want: "Lab 3: Pendulum",
  },
  {
    name: "shape 1, a resubmission reads the same",
    subject: "Submission received",
    from: BB,
    text: RECEIPT_BODY.replace("2:15:57 PM", "4:02:10 PM"),
    want: "Lab 3: Pendulum",
  },
  // Ruling: shapes 2-4 are NOT completion evidence. A grade email is sent for a zero on missing work.
  {
    name: "shape 2, grade posted",
    subject: "New grade and feedback for Lab 3: Pendulum in 202640-XX-101-001",
    from: BB,
    text: "A new grade and feedback is available for Lab 3: Pendulum.",
    want: null,
  },
  {
    name: "shape 3, overdue",
    subject: "Lab 3: Pendulum is overdue in 202640-XX-101-001",
    from: BB,
    text: "Lab 3: Pendulum was due on Thursday.",
    want: null,
  },
  {
    name: "shape 4, due soon",
    subject: "Lab 3: Pendulum is due soon in 202640-XX-101-001",
    from: BB,
    text: "Lab 3: Pendulum is due Thursday.",
    want: null,
  },
  {
    name: "a grade-posted body pasted under the receipt subject is still not a receipt",
    subject: "Submission received",
    from: BB,
    text: "A new grade and feedback is available for Lab 3: Pendulum.",
    want: null,
  },
  {
    name: "the receipt shape from any other sender is not trusted",
    subject: "Submission received",
    from: "Helpdesk <do-not-reply@lms.example.invalid>",
    text: RECEIPT_BODY,
    want: null,
  },
  {
    name: "the sender's name alone is not the sender",
    subject: "Submission received",
    from: "do-not-reply@blackboard.com <someone@example.invalid>",
    text: RECEIPT_BODY,
    want: null,
  },
  {
    name: "a newsletter",
    subject: "This week on campus",
    from: "Campus News <news@example.invalid>",
    text: "Five things happening this week.",
    want: null,
  },
  { name: "no body at all", subject: "Submission received", from: BB, text: "", want: null },
  {
    name: "no confirmation number",
    subject: "Submission received",
    from: BB,
    text: RECEIPT_BODY.replace(/Confirmation number: .*/, ""),
    want: null,
  },
  {
    name: "an empty title",
    subject: "Submission received",
    from: BB,
    text: "Assessment submitted\nSubmitted: Thursday, September 3, 2026 2:15:57 PM CDT\nConfirmation number: 0f1e",
    want: null,
  },
];

for (const row of TABLE) {
  Deno.test(`recogniseReceipt: ${row.name}`, () => {
    const got = recogniseReceipt({ subject: row.subject, from: row.from, text: row.text });
    assertEquals(got === null ? null : got.title, row.want);
  });
}

Deno.test("each template is data: a label, lower-case sender addresses, a subject, and a body with a title group", () => {
  assert(RECEIPT_TEMPLATES.length >= 1);
  for (const t of RECEIPT_TEMPLATES) {
    assert(t.label !== "", "a template needs a label for its why");
    assert(t.senders.length > 0, `${t.label}: a template must name its senders`);
    for (const s of t.senders) assertEquals(s, s.toLowerCase(), `${t.label}: senders are compared lower-cased`);
    assert(t.body.source.includes("(?<title>"), `${t.label}: the body pattern must capture a named title`);
  }
});

Deno.test("receiptVerdict: a receipt is a validated completion verdict with a templated why and no body", () => {
  const got = receiptVerdict({ subject: "Submission received", from: BB, text: RECEIPT_BODY }, { known_courses: ["xx-101"] });
  assertEquals(got, {
    tier: "completion",
    why: "Blackboard submission receipt",
    title: "Lab 3: Pendulum",
    course: null,
    due: null,
    effort_hours: null,
    importance: null,
    confidence: 1,
  });
  // No confirmation number and no body text in anything the verdict carries.
  assert(!JSON.stringify(got).includes("0f1e2d3c"), "the confirmation number reached the verdict");
  assert(!JSON.stringify(got).includes("12345.202640"), "the body reached the verdict");
});

Deno.test("receiptVerdict: anything that is not a receipt answers null, so the model is asked", () => {
  assertEquals(
    receiptVerdict({ subject: "This week on campus", from: "news@example.invalid", text: "Hi." }, {}),
    null,
  );
});
