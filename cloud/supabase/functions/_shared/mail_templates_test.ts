// Email plan T3a (spec §4.7 step 3, test 9; §5.1's `receipt` kind as amended 2026-10-02, PQ4) and T3a.2
// (D9's notification rows, PQ10). Every fixture is FABRICATED: the "Example LMS" row below, its address,
// courses, titles and dates are invented for these tests, and the receipt body is lms_receipts_test.ts's
// own invented shape. The T3a.2 messages copy only the vendor's layout (its sender, its subject wording,
// its marker lines); every course, title, name, address and date in them is invented. No real mail is used.
import { assert, assertEquals } from "@std/assert";
import { mailPromptText } from "./judge_prompts.ts";
import { validateMailClaims } from "./judge_validate.ts";
import { RECEIPT_TEMPLATES } from "./lms_receipts.ts";
import { scrubForPrompt } from "./scrub.ts";
import {
  MAIL_TEMPLATES,
  type MailClaim,
  type MailTemplate,
  matchCourse,
  templateClaims,
  type TemplateMessage,
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
  for (const t of TEMPLATES) {
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
  for (const t of TEMPLATES) {
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
// The vendor's layout, as its real notifications carry it: a style block first, then one indented line
// each for a course id, the course name, a marker ("New gradable item", "Assignment due soon"), the item's
// title, and "Due <weekday>, <month> <day>, <year> <h:mm:ss> <AM|PM> <zone>". Everything named is invented.

const BB_COURSE = "300010-QX-210-002";
const BB_CONTEXT = { knownCourses: ["qx-210", "yy-202"], timeZone: "America/Chicago" };
const BB_DUE_LINE = "Due Wednesday, November 11, 2026 11:59:00 PM CST";

/** A text/plain part in the vendor's shape: its style block, then each line indented as the vendor does. */
function bbText(lines: readonly string[]): string {
  const style = ["  body {", "    font-size: 14px;", "  }", "  #user-defined-description th {", "  }"];
  const tail = ["Email brought to you by", "Want to change how you receive these emails?"];
  return [...style, "", ...["90001.300010", BB_COURSE, ...lines, ...tail].map((l) => `          ${l}`)]
    .join("\n");
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

Deno.test("the_vendors_other_notifications_stay_on_the_model_path", () => {
  // D9's rows are new assignment and due soon; these carry no item a row could read without a guess (an
  // announcement is free text, and may itself move a due), or are not D9 kinds at all.
  const others: Array<[string, TemplateMessage]> = [
    [
      "overdue",
      bbNotice(`Gradable item is overdue in ${BB_COURSE}`, ["Overdue gradable item", "Quiz 6", BB_DUE_LINE]),
    ],
    ["new content", bbNotice(`New content in ${BB_COURSE}`, ["New content", "Week 9 slides.pdf", "View"])],
    [
      "a grade",
      bbNotice(`New grade and feedback for Quiz 6 in ${BB_COURSE}`, [
        "New grade and feedback",
        "Quiz 6",
        "View grades",
      ]),
    ],
    [
      "an announcement",
      bbNotice(`New announcement in ${BB_COURSE}`, [
        "New announcement",
        "Room change",
        "We now meet in Hall 4.",
      ]),
    ],
    [
      "a professor's announcement",
      bbNotice(`${BB_COURSE}: Problem Set 4 moved`, [
        "New announcement from Pat Example",
        "Problem Set 4 moved",
        "Problem Set 4 is now due November 13 at 11:59 PM.",
      ], { from: `"Pat Example - pat@school.example.invalid" <do-not-reply@blackboard.com>` }),
    ],
  ];
  for (const [name, message] of others) assertEquals(templateClaims(message, BB_CONTEXT), null, name);
});

Deno.test("a_blackboard_rows_claim_passes_the_handlers_check_against_the_cut_scrubbed_text", () => {
  // `mail-process` holds a template's claims to the model's checks against `mailPromptText` (§4.7 step 1).
  for (const message of [BB_NEW_ITEM(), BB_DUE_SOON()]) {
    const claims = templateClaims(message, BB_CONTEXT);
    assert(claims !== null, message.subject);
    const checked = validateMailClaims({ claims }, {
      from: message.from,
      subject: scrubForPrompt(message.subject),
      text: mailPromptText(message.text),
      date: message.date,
      ...BB_CONTEXT,
    }, "template");
    assertEquals(checked.refused, [], message.subject);
    assertEquals(checked.claims, claims, message.subject);
  }
});
