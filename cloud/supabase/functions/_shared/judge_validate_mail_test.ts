// Email plan T3b (spec §4.7 step 4, §5.1's claim as amended 2026-10-02, D9, D12; tests 10 and 13's
// validation half; §10 R11). Every fixture is FABRICATED: the sender, courses, titles and dates are invented.
import { assert, assertEquals } from "@std/assert";
import {
  CONFIDENCE_FLOOR,
  type Kind,
  MAIL_KINDS,
  MAIL_TOPICS,
  type MailValidateContext,
  MAX_MAIL_CLAIMS,
  validateMailClaims,
  validateMailResolve,
} from "./judge_validate.ts";

const CTX: MailValidateContext = {
  from: "Prof Example <prof@school.example.invalid>",
  subject: "Lab 4 moved",
  text: "Hello class,\n\nLab 4: Springs is now due\nOctober 9, 2026.\n\tThe quiz is Friday.\nThanks",
  date: "Mon, 28 Sep 2026 10:00:00 -0500",
  knownCourses: ["xx-101", "yy-202"],
  timeZone: "America/Chicago",
};

/** A model's change claim. Its evidence crosses the text's line break, so it holds only after folding. */
function change(over: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    kind: "change",
    course: "xx-101",
    target: "Lab 4: Springs",
    field: "due",
    value: "2026-10-09",
    item: null,
    topic: null,
    evidence: "Lab 4: Springs is now due October 9, 2026.",
    why: "the professor moved the lab",
    confidence: 0.9,
    ...over,
  };
}

function newItem(
  item: Record<string, unknown> = {},
  over: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    kind: "new_item",
    course: "xx-101",
    target: null,
    field: null,
    value: null,
    item: {
      title: "Quiz 2",
      due: "October 9, 2026",
      item_kind: "exam",
      effort_hours: 2,
      importance: 4,
      ...item,
    },
    topic: null,
    evidence: "The quiz is Friday.",
    why: "a quiz was announced",
    confidence: 0.8,
    ...over,
  };
}

function receipt(over: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    kind: "receipt",
    course: "xx-101",
    target: "Lab 4: Springs",
    field: null,
    value: null,
    item: null,
    topic: null,
    evidence: "Lab 4: Springs",
    why: "Example LMS submission receipt",
    confidence: 1,
    ...over,
  };
}

function one(claim: Record<string, unknown>, source: "model" | "template" = "model", ctx = CTX) {
  return validateMailClaims({ claims: [claim] }, ctx, source);
}

Deno.test("a well-formed change is kept in §5.1's shape, marked with the caller's source", () => {
  const got = one(change());
  assertEquals(got.ok, true);
  assertEquals(got.refused, []);
  assertEquals(got.claims, [{
    kind: "change",
    course: "xx-101",
    target: "Lab 4: Springs",
    field: "due",
    value: "2026-10-09",
    item: null,
    topic: null,
    evidence: "Lab 4: Springs is now due October 9, 2026.",
    why: "the professor moved the lab",
    confidence: 0.9,
    source: "model",
    resolution: null,
  }]);
});

Deno.test("evidence_not_verbatim_after_whitespace_folding_is_refused_incomplete", () => {
  // Folding is whitespace only: a tab and a line break in the text match one space in the evidence.
  assertEquals(one(newItem({}, { evidence: "2026. The quiz is Friday." })).ok, true);
  assertEquals(one(change({ evidence: "  Lab 4:   Springs  " })).claims[0]?.evidence, "Lab 4: Springs");
  // The subject is part of the message the model saw.
  assertEquals(one(change({ evidence: "Lab 4 moved" })).ok, true);
  for (
    const evidence of [
      "Lab 4: Springs is now due October 10, 2026.", // invented
      "lab 4: springs is now due", // case is not folded
      "Lab 4: Springs is now due October 9, 2026. Thanks!", // runs past the text
      "Lab 4 moved Hello class", // across the subject and the text
      "", // none
    ]
  ) {
    const got = one(change({ evidence }));
    assertEquals(got.ok, false, evidence);
    assertEquals(got.claims, []);
    assertEquals(got.refused, ["incomplete"]);
    assertEquals(got.cause, "incomplete");
  }
  assertEquals(one(change({ evidence: 42 })).cause, "incomplete");
});

Deno.test("evidence longer than 140 characters is cut to a verbatim prefix", () => {
  const long = "word ".repeat(60).trim();
  const got = one(change({ evidence: long }), "model", { ...CTX, text: `${long} end` });
  assertEquals(got.ok, true);
  const cut = got.claims[0].evidence;
  assert([...cut].length <= 140 && [...cut].length >= 135, cut);
  assert(long.startsWith(cut) && cut === cut.trim());
});

Deno.test("a_line_separator_or_control_character_in_title_from_or_evidence_is_refused", () => {
  // §10 R11: a value that breaks a line could forge a seen-ledger or frontmatter line on the device.
  const breaks = [
    "\r",
    "\n",
    "\u0085",
    String.fromCodePoint(0x2028),
    String.fromCodePoint(0x2029),
    "\u0000",
    "\u000b",
    "\u001b",
    "\u001f",
    "\u007f",
  ];
  for (const ch of breaks) {
    const name = JSON.stringify(ch);
    const text = `${CTX.text} Lab 4:${ch}Springs`;
    assertEquals(one(newItem({ title: `Quiz${ch}2` })).cause, "incomplete", `title ${name}`);
    assertEquals(
      one(change({ evidence: `Lab 4:${ch}Springs` }), "model", { ...CTX, text }).ok,
      false,
      `evidence ${name}`,
    );
    assertEquals(one(change({ target: `Lab${ch}4` })).ok, false, `target ${name}`);
    assertEquals(one(change({ why: `moved${ch}the lab` })).ok, false, `why ${name}`);
    const forged = validateMailClaims({ claims: [change(), newItem()] }, {
      ...CTX,
      from: `Prof${ch}<p@x.invalid>`,
    }, "model");
    assertEquals([forged.ok, forged.claims, forged.cause], [false, [], "incomplete"], `from ${name}`);
  }
  // A tab is not a line break: a title keeps it as a space, and the from passes.
  assertEquals(one(newItem({ title: "Quiz\t2" })).claims[0]?.item?.title, "Quiz 2");
  assertEquals(one(change(), "model", { ...CTX, from: "Prof\tExample <p@x.invalid>" }).ok, true);
});

Deno.test("an_unknown_course_reads_null", () => {
  for (const course of ["phys-999", "XX-101", " ", 7, null]) {
    const got = one(change({ course }));
    assertEquals(got.ok, true, String(course));
    assertEquals(got.claims[0].course, null, String(course));
  }
  assertEquals(one(change({ course: "yy-202" })).claims[0].course, "yy-202");
});

Deno.test("an_unresolvable_due_reads_null", () => {
  // Through judge_due.ts against the Date header, then held to a real calendar day and time: a shaped
  // but impossible due (the engine's T5.3 finding) is null here too, so the server and device agree.
  const resolved: [unknown, string | null][] = [
    ["October 9, 2026", "2026-10-09"],
    ["tomorrow", "2026-09-29"],
    ["2026-10-09T23:59", "2026-10-09T23:59"],
    ["2028-02-29", "2028-02-29"],
    ["sometime soon", null],
    ["next week", null],
    ["2026-09-31", null],
    ["2026-02-29", null],
    ["2026-13-01", null],
    ["2026-00-10", null],
    ["2026-10-09T24:00", null],
    ["2026-10-09T23:60", null],
    [`${String.fromCodePoint(0x0662, 0x0660, 0x0662, 0x0666)}-10-09`, null],
    [17, null],
    [null, null],
  ];
  for (const [due, want] of resolved) {
    const item = one(newItem({ due }));
    assertEquals(item.ok, true, String(due));
    assertEquals(item.claims[0].item?.due, want, `new_item ${due}`);
    const moved = one(change({ value: due }));
    assertEquals(moved.ok, true, String(due));
    assertEquals(moved.claims[0].value, want, `change ${due}`);
  }
});

Deno.test('a cancellation\'s value is "true" and nothing else', () => {
  assertEquals(one(change({ field: "cancelled", value: "true" })).claims[0]?.value, "true");
  for (const value of ["false", true, null, "2026-10-09"]) {
    assertEquals(one(change({ field: "cancelled", value })).cause, "incomplete", String(value));
  }
  for (const field of ["title", "status", null, "completed"]) {
    assertEquals(one(change({ field })).cause, "incomplete", String(field));
  }
  assertEquals(one(change({ target: " " })).cause, "incomplete");
});

Deno.test("a_sixth_claim_is_dropped", () => {
  const six = [1, 2, 3, 4, 5, 6].map((n) => change({ target: `Lab ${n}` }));
  const got = validateMailClaims({ claims: six }, CTX, "model");
  assertEquals(MAX_MAIL_CLAIMS, 5);
  assertEquals(got.claims.map((c) => c.target), ["Lab 1", "Lab 2", "Lab 3", "Lab 4", "Lab 5"]);
  assertEquals(got.refused, []);
});

Deno.test("a_claim_below_point_six_is_refused", () => {
  assertEquals(CONFIDENCE_FLOOR, 0.6);
  const low = one(change({ confidence: 0.59 }));
  assertEquals([low.ok, low.claims, low.cause], [false, [], "below floor"]);
  assertEquals(one(change({ confidence: 0.6 })).ok, true);
  assertEquals(one(change({ confidence: 1.4 })).claims[0].confidence, 1);
  assertEquals(one(change({ confidence: "0.9" })).cause, "incomplete");
  // Shape is judged before the floor, as in validate().
  assertEquals(one(change({ confidence: 0.1, evidence: "invented" })).cause, "incomplete");
});

Deno.test("a_new_item_without_effort_or_importance_in_range_is_refused", () => {
  // Refused, never clamped: D9's fixed ranges are the device's only effort and importance (§5.2).
  const bad: Record<string, unknown>[] = [
    { effort_hours: undefined },
    { effort_hours: 0.1 },
    { effort_hours: 41 },
    { effort_hours: "2" },
    { effort_hours: Number.NaN },
    { importance: undefined },
    { importance: 0 },
    { importance: 6 },
    { importance: 2.5 },
    { title: " " },
    { item_kind: "meeting" },
  ];
  for (const over of bad) {
    assertEquals(one(newItem(over)).cause, "incomplete", JSON.stringify(over));
  }
  assertEquals(one(newItem({}, { item: null })).cause, "incomplete");
  for (const over of [{ effort_hours: 0.25, importance: 1 }, { effort_hours: 40, importance: 5 }]) {
    assertEquals(one(newItem(over)).claims[0]?.item, {
      title: "Quiz 2",
      due: "2026-10-09",
      item_kind: "exam",
      ...over,
    });
  }
});

Deno.test("a_topic_outside_the_fixed_set_is_refused", () => {
  assertEquals([...MAIL_TOPICS], ["schedule", "room", "instructor", "office_hours", "policy", "other"]);
  const info = (topic: unknown) =>
    change({
      kind: "course_info",
      target: null,
      field: null,
      value: null,
      topic,
      evidence: "The quiz is Friday.",
    });
  for (const topic of ["grades", "Room", null, 3]) {
    assertEquals(one(info(topic)).cause, "incomplete", String(topic));
  }
  const got = one(info("office_hours"));
  assertEquals([got.claims[0]?.kind, got.claims[0]?.topic], ["course_info", "office_hours"]);
});

Deno.test("an unknown kind is refused, and noise is kept for the worker to read", () => {
  assertEquals(one(change({ kind: "update" })).cause, "incomplete");
  const noise = one(change({ kind: "noise", target: null, field: null, value: null }));
  assertEquals([noise.ok, noise.claims[0]?.kind, noise.claims[0]?.field], [true, "noise", null]);
});

Deno.test("a_receipt_claim_from_the_model_is_refused", () => {
  // PQ4: only a template emits a receipt. The source is the caller's word, never the answer's.
  for (const claim of [receipt(), receipt({ source: "template" })]) {
    const got = one(claim, "model");
    assertEquals([got.ok, got.claims, got.cause], [false, [], "incomplete"]);
  }
  const kept = one(receipt(), "template");
  assertEquals(kept.claims, [{ ...receipt(), source: "template", resolution: null }] as unknown[]);
  assertEquals(one(change(), "model").claims[0].source, "model");
  assertEquals(one(change({ source: "template" }), "model").claims[0].source, "model");
});

Deno.test("a_receipt_carrying_a_field_value_item_or_topic_is_refused", () => {
  const item = { title: "Lab 4", due: null, item_kind: "task", effort_hours: 1, importance: 3 };
  for (const over of [{ field: "due" }, { value: "true" }, { item }, { topic: "other" }, { target: null }]) {
    assertEquals(one(receipt(over), "template").cause, "incomplete", JSON.stringify(over));
  }
});

Deno.test("a reply keeps its good claims, names each refusal, and a reply with none is incomplete", () => {
  const got = validateMailClaims(
    { claims: [change({ evidence: "invented" }), newItem(), change({ confidence: 0.2 })] },
    CTX,
    "model",
  );
  assertEquals([got.ok, got.claims.length, got.refused, got.cause], [
    true,
    1,
    ["incomplete", "below floor"],
    undefined,
  ]);
  for (const answer of [{}, { claims: [] }, { claims: "x" }, { claims: [null, 7] }]) {
    const none = validateMailClaims(answer, CTX, "model");
    assertEquals([none.ok, none.claims, none.cause], [false, [], "incomplete"], JSON.stringify(answer));
  }
});

Deno.test("the mail kinds are their own type, apart from Kind (P6)", () => {
  assertEquals([...MAIL_KINDS], ["mail_claim", "mail_resolve", "mail_digest"]);
  const judged: readonly Kind[] = ["task", "event", "email"];
  assert(MAIL_KINDS.every((k) => !(judged as readonly string[]).includes(k)));
});

Deno.test("a_resolve_answer_naming_an_id_outside_its_batch_reads_none", () => {
  const batch = { claims: ["mail:a:0", "mail:a:1", "mail:b:0", "mail:c:0"], candidates: ["n-1", "n-2"] };
  const got = validateMailResolve({
    answers: [
      { claim: "mail:a:0", target_id: "n-2" },
      { claim: "mail:a:1", target_id: "n-9" }, // outside the batch's candidates
      { claim: "mail:b:0", target_id: null },
      { claim: "mail:z:0", target_id: "n-1" }, // a claim outside the batch
      { claim: "mail:c:0", target_id: "n-1" },
      { claim: "mail:c:0", target_id: "n-2" }, // two answers for one claim: none, never a pick
    ],
  }, batch);
  assertEquals(got, [{ target_id: "n-2" }, { target_id: null }, { target_id: null }, { target_id: null }]);
  for (const answer of [{}, { answers: "x" }, { answers: [null, { claim: 1, target_id: "n-1" }] }]) {
    assertEquals(validateMailResolve(answer, batch), batch.claims.map(() => ({ target_id: null })));
  }
});
