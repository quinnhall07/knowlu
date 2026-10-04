// Email plan T3a (spec §4.7 step 3, test 9; §5.1's `receipt` kind as amended 2026-10-02, PQ4) and T3a.2
// (D9's notification rows, PQ10). Every fixture is FABRICATED: the "Example LMS" row below, its address,
// courses, titles and dates are invented for these tests, and the receipt body is lms_receipts_test.ts's
// own invented shape. The T3a.2 messages copy only the vendor's layout (its sender, its subject wording,
// its marker lines); every course, title, name, address and date in them is invented. No real mail is used.
import { assert, assertEquals } from "@std/assert";
import { MAIL_TEXT_CHARS } from "./judge_prompts.ts";
import { validateMailClaims } from "./judge_validate.ts";
import { RECEIPT_TEMPLATES } from "./lms_receipts.ts";
import { scrubForPrompt } from "./scrub.ts";
import {
  EVIDENCE_CHARS,
  MAIL_TEMPLATES,
  type MailClaim,
  type MailTemplate,
  matchCourse,
  NOISE_TEMPLATES,
  templateClaims,
  type TemplateMessage,
  templateNoise,
  TEMPLATES,
  VENDOR_DOMAINS,
} from "./mail_templates.ts";

/** The vendor rows' data region in mail_templates.ts: the only place a vendor may be named. */
const VENDOR_ROWS_BEGIN = "// vendor rows: begin";
const VENDOR_ROWS_END = "// vendor rows: end";

const LMS_SENDER = "notify@lms.example.invalid";

/** A synthetic "due date changed" notification row: what a real vendor row looks like, as data. */
const DUE_CHANGED: MailTemplate = {
  label: "Example LMS",
  senders: [LMS_SENDER],
  subject: /^Due date changed: /,
  body:
    /\bThe due date for (?<title>.+?) in (?<course>[A-Z]{2,4}[- ]?\d{3}) has changed to (?<due>[A-Z][a-z]+ \d{1,2}, \d{4}(?: at \d{1,2}:\d{2} [AP]M [A-Z]{3})?)\./,
  claim: { kind: "change", field: "due" },
  why: "due date changed",
};

const ROWS: readonly MailTemplate[] = [DUE_CHANGED];

const CHANGED_TEXT = [
  "Hello,",
  "",
  "The due date for Lab 4: Springs in XX-101 has changed to October 9, 2026 at 11:59 PM CDT.",
  "",
  "View it online.",
].join("\r\n");

function changed(over: Partial<TemplateMessage> = {}): TemplateMessage {
  return {
    from: `Example LMS <${LMS_SENDER}>`,
    subject: "Due date changed: Lab 4: Springs",
    text: CHANGED_TEXT,
    date: "Fri, 02 Oct 2026 14:00:00 +0000",
    authenticity: "aligned",
    ...over,
  };
}

const CONTEXT = { knownCourses: ["xx-101", "yy-202"], timeZone: "America/Chicago" };

Deno.test("an_aligned_lms_due_date_changed_mail_yields_a_template_claim_with_a_resolved_due", () => {
  const want: MailClaim = {
    kind: "change",
    course: "xx-101",
    target: "Lab 4: Springs",
    field: "due",
    value: "2026-10-09T23:59",
    item: null,
    topic: null,
    evidence: "The due date for Lab 4: Springs in XX-101 has changed to October 9, 2026 at 11:59 PM CDT",
    why: "Example LMS due date changed",
    confidence: 1,
    source: "template",
    resolution: null,
  };
  assertEquals(templateClaims(changed(), CONTEXT, ROWS), [want]);
});

Deno.test("the_same_mail_unaligned_goes_to_the_model", () => {
  assertEquals(templateClaims(changed({ authenticity: "unaligned" }), CONTEXT, ROWS), null);
});

Deno.test("a_template_matches_only_the_vendors_exact_address_subject_and_body", () => {
  const misses: Array<[string, Partial<TemplateMessage>]> = [
    ["another address", { from: "Example LMS <notify@other.example.invalid>" }],
    ["a subdomain of the vendor", { from: "notify@mail.lms.example.invalid" }],
    ["a look-alike local part", { from: "notify2@lms.example.invalid" }],
    ["the vendor's address as the display name only", { from: `${LMS_SENDER} <someone@evil.invalid>` }],
    ["two addresses", { from: `${LMS_SENDER}, someone@evil.invalid` }],
    ["another subject", { subject: "Re: Due date changed: Lab 4: Springs" }],
    ["another body", { text: "The due date for Lab 4: Springs may change soon." }],
    ["no body", { text: "" }],
  ];
  for (const [name, over] of misses) {
    assertEquals(templateClaims(changed(over), CONTEXT, ROWS), null, name);
  }
  // The exact address matches whatever its case or display name.
  assertEquals(templateClaims(changed({ from: "NOTIFY@LMS.Example.Invalid" }), CONTEXT, ROWS)?.length, 1);
});

Deno.test("a due the resolver cannot place makes no template claim, so the model is asked", () => {
  // February 30th is no calendar day: `judge_due.ts` answers null, and a change with no value is no claim.
  const text = CHANGED_TEXT.replace("October 9, 2026", "February 30, 2026");
  assertEquals(templateClaims(changed({ text }), CONTEXT, ROWS), null);
});

Deno.test("an unknown or ambiguous course reads null, never a guess", () => {
  const unknown = templateClaims(changed(), { ...CONTEXT, knownCourses: ["zz-303"] }, ROWS);
  assertEquals(unknown?.[0].course, null);
  assertEquals(matchCourse("202640-XX-101-001", ["xx-101", "xx-1010"]), "xx-101");
  assertEquals(matchCourse("XX-101 and YY-202", ["xx-101", "yy-202"]), null);
  assertEquals(matchCourse("XX101", ["XX 101"]), "XX 101");
  assertEquals(matchCourse("XX-1010", ["xx-101"]), null);
  assertEquals(
    matchCourse("english homework", ["english"]),
    null,
    "a code with no number never matches by scan",
  );
});

const BB = "Blackboard <do-not-reply@blackboard.com>";
const RECEIPT_BODY = [
  "12345.202640 202640-XX-101-001",
  "Assessment submitted",
  "Lab 3: Pendulum",
  "Submitted: Thursday, September 3, 2026 2:15:57 PM CDT",
  "Confirmation number: 0f1e2d3c4b5a69788796a5b4c3d2e1f0",
].join("\n");

function receipt(over: Partial<TemplateMessage> = {}): TemplateMessage {
  return {
    from: BB,
    subject: "Submission received",
    text: RECEIPT_BODY,
    date: "Thu, 03 Sep 2026 19:16:00 +0000",
    authenticity: "aligned",
    ...over,
  };
}

Deno.test("a_submission_receipt_template_yields_a_receipt_claim", () => {
  const got = templateClaims(receipt(), CONTEXT);
  assertEquals(got, [{
    kind: "receipt",
    course: "xx-101",
    target: "Lab 3: Pendulum",
    field: null,
    value: null,
    item: null,
    topic: null,
    evidence: "Assessment submitted Lab 3: Pendulum",
    why: "Blackboard submission receipt",
    confidence: 1,
    source: "template",
    resolution: null,
  }]);
  assert(!JSON.stringify(got).includes("0f1e2d3c"), "the confirmation number reached the claim");
});

Deno.test("an_unaligned_receipt_mail_yields_no_receipt", () => {
  assertEquals(templateClaims(receipt({ authenticity: "unaligned" }), CONTEXT), null);
});

Deno.test("a posted grade from the receipt sender is no template claim", () => {
  const grade = receipt({
    subject: "New grade and feedback for Lab 3: Pendulum in 202640-XX-101-001",
    text: "A new grade and feedback is available for Lab 3: Pendulum.",
  });
  assertEquals(templateClaims(grade, CONTEXT), null);
});

Deno.test("a title carrying a control character or U+0085 is no template claim", () => {
  for (const bad of ["\u0007", "\u0085", "\u007f"]) {
    const text = CHANGED_TEXT.replace("Lab 4: Springs", `Lab 4:${bad}Springs`);
    assertEquals(templateClaims(changed({ text }), CONTEXT, ROWS), null, JSON.stringify(bad));
  }
});

Deno.test("every row is data of the right shape, and the receipts are lms_receipts' own rows", () => {
  const topics = ["schedule", "room", "instructor", "office_hours", "policy", "other"];
  for (const t of [...TEMPLATES, DUE_CHANGED]) {
    assert(t.label !== "" && t.why !== "", "a row needs a label and a why");
    assert(t.senders.length > 0);
    for (const s of t.senders) assertEquals(s, s.toLowerCase(), `${t.label}: senders are lower-case`);
    if (t.claim.kind !== "course_info") assert(t.body.source.includes("(?<title>"), `${t.label}: no title`);
    if (t.claim.kind === "change" && t.claim.field === "due") assert(t.body.source.includes("(?<due>"));
    if (t.claim.kind === "new_item") {
      assert(t.claim.effort_hours >= 0.25 && t.claim.effort_hours <= 40);
      assert(Number.isInteger(t.claim.importance) && t.claim.importance >= 1 && t.claim.importance <= 5);
    }
    if (t.claim.kind === "course_info") assert(topics.includes(t.claim.topic));
  }
  // A noise row is a sender and a whole subject shape, anchored at both ends, and nothing else.
  for (const n of NOISE_TEMPLATES) {
    assert(n.label !== "" && n.senders.length > 0, "a noise row needs a label and a sender");
    for (const s of n.senders) assertEquals(s, s.toLowerCase(), `${n.label}: senders are lower-case`);
    assert(n.subject.source.startsWith("^") && n.subject.source.endsWith("$"), `${n.subject}: anchored`);
    assertEquals(n.subject.flags, "", `${n.subject}: no flags (an exact, case-sensitive shape)`);
  }
  assertEquals(TEMPLATES.length, MAIL_TEMPLATES.length + RECEIPT_TEMPLATES.length);
  for (const r of RECEIPT_TEMPLATES) {
    assert(TEMPLATES.some((t) => t.claim.kind === "receipt" && t.label === r.label && t.body === r.body));
  }
});

Deno.test("the_vendor_domains_are_bare_lower_case_domains_and_cover_every_rows_sender", () => {
  assert(VENDOR_DOMAINS.length > 0);
  assertEquals(new Set(VENDOR_DOMAINS).size, VENDOR_DOMAINS.length, "no domain twice");
  for (const d of VENDOR_DOMAINS) {
    assert(/^[a-z0-9-]+(\.[a-z0-9-]+)+$/.test(d), `"${d}" is a bare lower-case domain`);
    assert(!d.endsWith(".edu"), `"${d}": any .edu is already allowed`);
  }
  // A row's vendor is a vendor: its sender's domain is a vendor domain or a subdomain of one.
  for (const t of [...TEMPLATES, ...NOISE_TEMPLATES]) {
    for (const s of t.senders) {
      const domain = s.split("@")[1] ?? "";
      assert(
        VENDOR_DOMAINS.some((d) => domain === d || domain.endsWith(`.${d}`)),
        `${t.label}: ${domain} is not on VENDOR_DOMAINS`,
      );
    }
  }
});

Deno.test("a new_item, a cancelled change and a course_info row each fill only their own fields", () => {
  const row = (claim: MailTemplate["claim"], body: RegExp): MailTemplate => ({
    label: "Example LMS",
    senders: [LMS_SENDER],
    subject: /^Notice$/,
    body,
    claim,
    why: "notice",
  });
  const message = (text: string) => changed({ subject: "Notice", text });
  const base = {
    course: "xx-101",
    confidence: 1,
    source: "template" as const,
    resolution: null,
    why: "Example LMS notice",
  };

  const added = row(
    { kind: "new_item", item_kind: "task", effort_hours: 1, importance: 3 },
    /New assignment (?<title>.+?) in (?<course>XX-\d{3}) is due (?<due>tomorrow)\./,
  );
  assertEquals(
    templateClaims(message("New assignment Quiz 2 in XX-101 is due tomorrow."), CONTEXT, [added]),
    [{
      ...base,
      kind: "new_item",
      target: null,
      field: null,
      value: null,
      item: { title: "Quiz 2", due: "2026-10-03", item_kind: "task", effort_hours: 1, importance: 3 },
      topic: null,
      evidence: "New assignment Quiz 2 in XX-101 is due tomorrow",
    }],
  );

  const cancelled = row({ kind: "change", field: "cancelled" }, /(?<title>Lab 5) in XX-101 is cancelled/);
  const got = templateClaims(message("Lab 5 in XX-101 is cancelled."), CONTEXT, [cancelled])?.[0];
  assertEquals([got?.target, got?.field, got?.value, got?.item, got?.course], [
    "Lab 5",
    "cancelled",
    "true",
    null,
    "xx-101",
  ]);

  const room = row({ kind: "course_info", topic: "room" }, /XX-101 now meets in a new room/);
  const info = templateClaims(message("From today XX-101 now meets in a new room."), CONTEXT, [room])?.[0];
  assertEquals(
    [info?.kind, info?.topic, info?.target, info?.item, info?.course, info?.evidence],
    ["course_info", "room", null, null, "xx-101", "XX-101 now meets in a new room"],
  );
});

Deno.test("the_code_names_no_vendor", async () => {
  const source = await Deno.readTextFile(new URL("./mail_templates.ts", import.meta.url));
  const begin = source.indexOf(VENDOR_ROWS_BEGIN);
  const end = source.indexOf(VENDOR_ROWS_END);
  assert(begin >= 0 && end > begin, "the vendor rows sit between their two markers");
  assertEquals(source.indexOf(VENDOR_ROWS_BEGIN, begin + 1), -1, "one data region only");
  const code = (source.slice(0, begin) + source.slice(end)).toLowerCase();
  const names = new Set(["blackboard", "canvas", "instructure", "zybooks", "moodle", "brightspace", "d2l"]);
  for (const d of VENDOR_DOMAINS) {
    names.add(d);
    names.add(d.split(".").slice(-2)[0]);
  }
  for (const t of [...TEMPLATES, ...NOISE_TEMPLATES]) {
    names.add(t.label.toLowerCase());
    for (const s of t.senders) {
      names.add(s);
      const domain = s.split("@")[1] ?? "";
      names.add(domain);
      names.add(domain.split(".").slice(-2)[0]);
    }
  }
  for (const name of names) {
    if (name.length >= 3) assert(!code.includes(name), `the code outside the vendor rows names "${name}"`);
  }
});

// --- T3a.2: Blackboard's notification rows (D9, PQ10) ---------------------------------------------------
// The vendor's layout, at the scale its real notifications carry it: a style block of 80 lines and about
// 1,360 characters, then one indented line each for a numeric id (NNNNN.YYYYTT), the course section code,
// a marker ("New gradable item", "Assignment due soon"), the item's title and "Due <weekday>, <month>
// <day>, <year> <h:mm:ss> <AM|PM> <zone>", each field after a run of indented blank lines. In the captures
// the due (its zone included) ends 1,825 to 1,883 characters into the trimmed text, 1,768 to 1,826 once
// the scrub shortens the style block's link: inside `mailPromptText`'s 2,000-character cut by less than
// 240. The fields' strings are invented; only the shape and the lengths are the vendor's.

const BB_COURSE = "300010-QX-210-002";
const BB_CONTEXT = { knownCourses: ["qx-210", "yy-202"], timeZone: "America/Chicago" };
const BB_DUE_LINE = "Due Wednesday, November 11, 2026 11:59:00 PM CST";

/** The style block: generic CSS shaped like the vendor's (an import with a link, rules, then indented
 * blank lines), 80 lines. */
const BB_STYLE: readonly string[] = (() => {
  const rule = (
    selector: string,
    props: readonly string[],
  ) => [`  ${selector} {`, ...props.map((p) => `    ${p};`), "  }", ""];
  const lines = [
    "@import url(https://fonts.example.invalid/css?family=Open+Sans:400,600,700);",
    "",
    "  /* Take care of image borders and formatting in every mail client */",
    "",
    ...rule("img", [
      "max-width: 600px",
      "outline: none",
      "text-decoration: none",
      "-ms-interpolation-mode: bicubic",
    ]),
    ...rule("a", ["text-decoration: underline", "border: 0", "outline: none"]),
    ...rule("a img", ["border: none"]),
    "  /* General styling, shared by every block below */",
    "",
    ...rule("td", ["text-align: center"]),
    ...rule("body", [
      "-webkit-font-smoothing: antialiased",
      "-webkit-text-size-adjust: none",
      "-ms-text-size-adjust: 100%",
      "width: 100%",
      "height: 100%",
      "background: #ffffff",
      "font-size: 14px",
      "font-family: 'Open Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif",
    ]),
    "  /* Prevent the mail client from adding extra spacing to its tables. */",
    ...rule("table,\n  td", ["mso-table-lspace: 0pt !important", "mso-table-rspace: 0pt !important"]),
    ...rule("#user-defined-description table", [
      "border: 1px solid #cdcdcd",
      "border-collapse: collapse",
      "width: 100%",
      "table-layout: fixed",
    ]),
    ...rule("#user-defined-description td, th", ["border: 1px solid #cdcdcd", "padding: 12px 16px"]),
    ...rule("#user-defined-description th", ["background: #f8f8f8"]),
  ].join("\n").split("\n");
  const indents = [2, 0, 0, 0, 0, 2, 4, 6, 8, 10, 12, 14, 34, 0, 0, 2, 4, 6, 8];
  return [...lines, ...indents.map((n) => " ".repeat(n))];
})();

/** The indented blank lines the vendor writes before each field after the first (the course section
 * code, the marker, the title, the due), and after the due, as the captures have them. */
const BB_GAPS: readonly (readonly number[])[] = [
  [8, 8, 8],
  [8, 6, 4, 2, 0, 0, 0, 0, 2, 4, 6, 8],
  [8, 6, 4, 2, 0, 0, 0, 2, 4, 6, 8, 8, 0, 8, 10, 12, 12, 14],
  [14, 12, 0, 12, 24, 12, 14],
  [14, 12, 22, 8, 6, 4, 2, 0, 0, 0, 0],
];
const BB_INDENTS = [10, 10, 10, 16, 16];

/** A text/plain part in the vendor's shape: its style block, then each field indented as the vendor does,
 * the numeric id and the section code first. */
function bbText(lines: readonly string[]): string {
  const tail = ["Email brought to you by", "Want to change how you receive these emails?"];
  const fields = ["90001.300010", BB_COURSE, ...lines, ...tail];
  const out = [...BB_STYLE];
  fields.forEach((field, i) => {
    if (i > 0) out.push(...BB_GAPS[Math.min(i - 1, BB_GAPS.length - 1)].map((n) => " ".repeat(n)));
    out.push(`${" ".repeat(BB_INDENTS[i] ?? 26)}${field}`);
  });
  return out.join("\n");
}

function bbNotice(
  subject: string,
  lines: readonly string[],
  over: Partial<TemplateMessage> = {},
): TemplateMessage {
  return {
    from: "do-not-reply@blackboard.com",
    subject,
    text: bbText(lines),
    date: "Thu, 5 Nov 2026 06:00:00 +0000",
    authenticity: "aligned",
    ...over,
  };
}

const BB_NEW_ITEM = () =>
  bbNotice(`New gradable item added to ${BB_COURSE}`, [
    "New gradable item",
    "Problem Set 4: Vectors",
    BB_DUE_LINE,
  ]);

/** A due-soon title may itself say "Due": the item is read up to the first "Due <weekday>,". */
const BB_DUE_SOON_TITLE = "Field Report 2 (Due Nov. 11th by 11:59 pm CST)";
const BB_DUE_SOON = () =>
  bbNotice(`${BB_DUE_SOON_TITLE} is due soon in ${BB_COURSE}`, [
    "Assignment due soon",
    BB_DUE_SOON_TITLE,
    BB_DUE_LINE,
    "View",
  ]);

function bbNewItem(title: string, why: string, evidence: string): MailClaim {
  return {
    kind: "new_item",
    course: "qx-210",
    target: null,
    field: null,
    value: null,
    item: { title, due: "2026-11-11T23:59", item_kind: "task", effort_hours: 1, importance: 3 },
    topic: null,
    evidence,
    why,
    confidence: 1,
    source: "template",
    resolution: null,
  };
}

/** Where the due line (its zone included) ends in the text as `mailPromptText` trims it, in characters. */
function bbDueEnd(message: TemplateMessage): number {
  const text = [...message.text.trim()].join("");
  const at = text.indexOf(BB_DUE_LINE);
  return at < 0 ? -1 : [...text.slice(0, at)].length + BB_DUE_LINE.length;
}

Deno.test("the_blackboard_fixtures_are_at_the_captures_scale", () => {
  // The captures' style block is 80 lines and 1,360 characters, and their due ends 1,825 to 1,883
  // characters into the trimmed text. A fixture far shorter could not show a field the cut leaves out.
  assertEquals(BB_STYLE.length, 80);
  const style = BB_STYLE.join("\n").length;
  assert(style >= 1300 && style <= 1420, `the style block is ${style} characters`);
  for (const message of [BB_NEW_ITEM(), BB_DUE_SOON()]) {
    const end = bbDueEnd(message);
    assert(end >= 1800 && end <= 1900, `${message.subject}: the due ends at ${end}`);
  }
});

Deno.test("an_aligned_blackboard_new_gradable_item_mail_yields_a_new_item_with_its_resolved_due", () => {
  assertEquals(templateClaims(BB_NEW_ITEM(), BB_CONTEXT), [
    bbNewItem(
      "Problem Set 4: Vectors",
      "Blackboard new gradable item",
      "New gradable item Problem Set 4: Vectors Due Wednesday, November 11, 2026 11:59:00 PM",
    ),
  ]);
});

Deno.test("the_same_new_gradable_item_mail_unaligned_yields_nothing", () => {
  assertEquals(templateClaims({ ...BB_NEW_ITEM(), authenticity: "unaligned" }, BB_CONTEXT), null);
});

Deno.test("an_aligned_blackboard_due_soon_mail_yields_a_new_item_with_its_resolved_due", () => {
  assertEquals(templateClaims(BB_DUE_SOON(), BB_CONTEXT), [
    bbNewItem(
      BB_DUE_SOON_TITLE,
      "Blackboard assignment due soon",
      `Assignment due soon ${BB_DUE_SOON_TITLE} Due Wednesday, November 11, 2026 11:59:00 PM`,
    ),
  ]);
});

Deno.test("the_same_due_soon_mail_unaligned_yields_nothing", () => {
  assertEquals(templateClaims({ ...BB_DUE_SOON(), authenticity: "unaligned" }, BB_CONTEXT), null);
});

Deno.test("a_blackboard_due_is_the_wall_clock_the_mail_shows_whatever_zone_the_context_has", () => {
  // The vendor writes the due on the recipient's clock and stamps its Date in UTC. `mail-process` has no
  // account timezone (its context says null), so a due read with its zone would land on the UTC clock:
  // 11:59 PM CST as 05:59 the next morning. The rows read the date and time the mail shows, zone left out.
  for (const timeZone of [null, "America/Chicago"]) {
    for (const message of [BB_NEW_ITEM(), BB_DUE_SOON()]) {
      const got = templateClaims(message, { ...BB_CONTEXT, timeZone });
      assertEquals(got?.[0].item?.due, "2026-11-11T23:59", `${message.subject} (${timeZone})`);
    }
  }
});

Deno.test("a_blackboard_row_matches_only_the_vendors_address_its_subject_and_its_marker", () => {
  const misses: Array<[string, TemplateMessage]> = [
    ["another address", { ...BB_NEW_ITEM(), from: "do-not-reply@lms.example.invalid" }],
    ["a subdomain of the vendor", { ...BB_NEW_ITEM(), from: "do-not-reply@mail.blackboard.com" }],
    ["a reply's subject", { ...BB_NEW_ITEM(), subject: `Re: New gradable item added to ${BB_COURSE}` }],
    ["the due-soon subject on a new-item body", { ...BB_NEW_ITEM(), subject: BB_DUE_SOON().subject }],
    ["no Due line", bbNotice(BB_NEW_ITEM().subject, ["New gradable item", "Problem Set 4: Vectors"])],
    // A hypothetical marker: no capture has one but the two the rows read.
    ["another marker", bbNotice(BB_DUE_SOON().subject, ["Test due soon", "Quiz 6", BB_DUE_LINE])],
  ];
  for (const [name, message] of misses) assertEquals(templateClaims(message, BB_CONTEXT), null, name);
  // A professor's announcement comes from the same address under their display name: still the vendor.
  const named = {
    ...BB_NEW_ITEM(),
    from: `"Pat Example - pat@school.example.invalid" <do-not-reply@blackboard.com>`,
  };
  assertEquals(templateClaims(named, BB_CONTEXT)?.[0].item?.title, "Problem Set 4: Vectors");
});

/** The vendor's notifications that carry no D9 item: read by no claim row, and vendor noise (the screen's
 * rule 5, `mail-process`), on sender and subject alone. Their marker lines are invented. */
const BB_NO_ITEM: ReadonlyArray<[string, () => TemplateMessage]> = [
  [
    "overdue",
    () =>
      bbNotice(`Gradable item is overdue in ${BB_COURSE}`, ["Overdue gradable item", "Quiz 6", BB_DUE_LINE]),
  ],
  [
    "new content",
    () => bbNotice(`New content in ${BB_COURSE}`, ["New content", "Week 9 slides.pdf", "View"]),
  ],
  [
    "a grade",
    () =>
      bbNotice(`New grade and feedback for Quiz 6 in ${BB_COURSE}`, [
        "New grade and feedback",
        "Quiz 6",
        "View grades",
      ]),
  ],
];

/** Announcements: free text, and one may itself move a due, so the model reads them. */
const BB_ANNOUNCEMENTS: ReadonlyArray<[string, () => TemplateMessage]> = [
  [
    "an announcement",
    () =>
      bbNotice(`New announcement in ${BB_COURSE}`, [
        "New announcement",
        "Room change",
        "We now meet in Hall 4.",
      ]),
  ],
  [
    "a professor's announcement",
    () =>
      bbNotice(`${BB_COURSE}: Problem Set 4 moved`, [
        "New announcement from Pat Example",
        "Problem Set 4 moved",
        "Problem Set 4 is now due November 13 at 11:59 PM.",
      ], { from: `"Pat Example - pat@school.example.invalid" <do-not-reply@blackboard.com>` }),
  ],
];

Deno.test("the_vendors_other_notifications_make_no_template_claim", () => {
  // D9's rows are new assignment and due soon; these carry no item a row could read without a guess (an
  // announcement is free text, and may itself move a due), or are not D9 kinds at all.
  for (const [name, message] of [...BB_NO_ITEM, ...BB_ANNOUNCEMENTS]) {
    assertEquals(templateClaims(message(), BB_CONTEXT), null, name);
  }
});

Deno.test("an_aligned_blackboard_notification_with_no_d9_item_is_vendor_noise", () => {
  for (const [name, message] of BB_NO_ITEM) {
    assert(templateNoise(message()), name);
    // A display name is still the vendor's address.
    assert(templateNoise({ ...message(), from: `"Pat Example" <do-not-reply@blackboard.com>` }), name);
  }
});

Deno.test("vendor_noise_is_the_vendors_exact_address_and_subject_shape_on_aligned_mail_only", () => {
  for (const [name, message] of BB_NO_ITEM) {
    const misses: Array<[string, TemplateMessage]> = [
      ["unaligned", { ...message(), authenticity: "unaligned" }],
      ["another address", { ...message(), from: "do-not-reply@lms.example.invalid" }],
      ["a subdomain of the vendor", { ...message(), from: "do-not-reply@mail.blackboard.com" }],
      ["a reply's subject", { ...message(), subject: `Re: ${message().subject}` }],
      ["a forward's subject", { ...message(), subject: `Fwd: ${message().subject}` }],
      ["another case", { ...message(), subject: message().subject.toUpperCase() }],
    ];
    for (const [miss, m] of misses) assert(!templateNoise(m), `${name}: ${miss}`);
  }
});

Deno.test("vendor_noise_never_swallows_a_d9_row_a_receipt_or_an_announcement", () => {
  const kept: Array<[string, TemplateMessage]> = [
    ["new gradable item", BB_NEW_ITEM()],
    ["due soon", BB_DUE_SOON()],
    ["the receipt", bbNotice("Submission received", ["Assessment submitted", "Quiz 6"])],
    ...BB_ANNOUNCEMENTS.map(([name, m]): [string, TemplateMessage] => [name, m()]),
    // A title that reads like a noise subject: a claim row reads the subject first, so it is never noise.
    [
      "a due-soon item named like a grade",
      bbNotice(`New grade and feedback for Quiz 6 is due soon in ${BB_COURSE}`, []),
    ],
    [
      "a due-soon item named like overdue",
      bbNotice(`Gradable item is overdue in X is due soon in ${BB_COURSE}`, []),
    ],
    [
      "a new item in a course named like content",
      bbNotice("New gradable item added to New content in X", []),
    ],
    // Not the vendor's captured shape: an item named overdue stays on the model path.
    ["an item-named overdue", bbNotice(`Quiz 6 is overdue in ${BB_COURSE}`, [])],
    ["a bare marker", bbNotice("New content in ", [])],
  ];
  for (const [name, message] of kept) assert(!templateNoise(message), name);
  // With no claim row, the precedence is the caller's rows: an empty list reads the shape alone.
  assert(templateNoise(BB_NO_ITEM[0][1](), []));
});

/** The handler's check (`mail-process`, §4.7 step 3): a template's claims against the whole text, scrubbed
 * and never cut (Quinn's ruling of 2026-10-04). Only a model's claims are checked against the cut. */
function handlersCheck(message: TemplateMessage, claims: MailClaim[]) {
  return validateMailClaims({ claims }, {
    from: message.from,
    subject: scrubForPrompt(message.subject),
    text: scrubForPrompt(message.text.trim()),
    date: message.date,
    ...BB_CONTEXT,
  }, "template");
}

Deno.test("a_blackboard_rows_claim_passes_the_handlers_check_against_the_scrubbed_text", () => {
  for (const message of [BB_NEW_ITEM(), BB_DUE_SOON()]) {
    const claims = templateClaims(message, BB_CONTEXT);
    assert(claims !== null, message.subject);
    const checked = handlersCheck(message, claims);
    assertEquals(checked.refused, [], message.subject);
    assertEquals(checked.claims, claims, message.subject);
  }
});

/** Where the due line ends in the text the model is shown (scrubbed, before the cut), up to "PM": the end
 * of a short-titled claim's evidence. */
function bbShownDueEnd(message: TemplateMessage): number {
  const text = [...scrubForPrompt(message.text.trim())].join("");
  const at = text.indexOf(BB_DUE_LINE);
  return [...text.slice(0, at)].length + BB_DUE_LINE.length - " CST".length;
}

/** `message` with `extra` characters of invented CSS rules added to the end of its style block. */
function bbLongerStyle(message: TemplateMessage, extra: number): TemplateMessage {
  const rules: string[] = [];
  for (let i = 0; rules.join("\n").length < extra; i++) {
    rules.push(`  .block-${i} td { padding: 0 ${i}px; border-collapse: collapse; }`);
  }
  const style = BB_STYLE.join("\n");
  assert(message.text.startsWith(style));
  return { ...message, text: `${style}\n${rules.join("\n")}${message.text.slice(style.length)}` };
}

Deno.test("a_blackboard_mail_whose_style_block_is_400_characters_longer_still_yields_its_claim", () => {
  // The captures' style block runs to about 1,360 characters and their due ends inside the model's 2,000-
  // character cut by less than 240. A style block 400 or more characters longer puts the whole evidence
  // past the cut. The row reads the whole text, and the handler checks a template's claims against the
  // whole scrubbed text, so the claim stands with no model call.
  for (const base of [BB_NEW_ITEM(), BB_DUE_SOON()]) {
    const message = bbLongerStyle(base, 420);
    assert(message.text.length - base.text.length >= 400, base.subject);
    const evidenceStart = bbShownDueEnd(message) - templateClaims(base, BB_CONTEXT)![0].evidence.length;
    assert(evidenceStart > MAIL_TEXT_CHARS, `${base.subject}: the evidence starts at ${evidenceStart}`);
    const claims = templateClaims(message, BB_CONTEXT);
    assertEquals(claims, templateClaims(base, BB_CONTEXT), base.subject);
    const checked = handlersCheck(message, claims!);
    assertEquals(checked.refused, [], base.subject);
    assertEquals(checked.claims, claims, base.subject);
  }
});

Deno.test("a_long_title_moves_the_due_past_the_cut_but_never_the_evidence", () => {
  // The evidence stops 140 characters after the marker, however long the title. The claim stands, its title
  // cut to 200 characters and its due read from the whole text, and the handler's check keeps it.
  const title = `Problem Set 4: Vectors, ${"and the review questions for chapters nine to twelve ".repeat(5)}`
    .trim();
  const message = bbNotice(BB_NEW_ITEM().subject, ["New gradable item", title, BB_DUE_LINE]);
  assert(bbShownDueEnd(message) > MAIL_TEXT_CHARS, `the due ends at ${bbShownDueEnd(message)}`);
  const claims = templateClaims(message, BB_CONTEXT);
  assert(claims !== null);
  assertEquals(claims[0].item?.title, [...title].slice(0, 200).join("").trim());
  assertEquals(claims[0].item?.due, "2026-11-11T23:59");
  assertEquals([...claims[0].evidence].length, EVIDENCE_CHARS);
  assertEquals(handlersCheck(message, claims).claims, claims);
});

Deno.test("evidence_the_scrub_changes_is_still_a_claim_for_the_handlers_check_to_refuse", () => {
  // A title naming an address. The row still claims, and the handler's check (scrubbed, uncut) refuses it,
  // so no span the scrub would change reaches the device, past the cut or not (mail-process's handler_test
  // holds the deletion).
  const title = "Lab 6 for pat@school.example.invalid";
  for (const extra of [0, 420]) {
    const message = bbLongerStyle(
      bbNotice(BB_NEW_ITEM().subject, ["New gradable item", title, BB_DUE_LINE]),
      extra,
    );
    const claims = templateClaims(message, BB_CONTEXT);
    assertEquals(claims?.[0].item?.title, title, `${extra}`);
    assertEquals(handlersCheck(message, claims ?? []).claims, [], `${extra}`);
  }
});
