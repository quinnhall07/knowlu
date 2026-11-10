// T2a: Layer 1 of the red-team suite, the hostile model (plan `2026-11-09-prompt-injection-suite`,
// §4.1, §4.3 and §4.3.1). `ScriptedModel` plays a fully compromised model: it returns the worst
// schema-shaped answers an injection could produce, and each case drives the real code path, step for
// step as production runs it, then asserts on what would reach the device:
//
//   - task, event, email: `judge()` with a literal `ModelRow` and stub rules, caps and log, as
//     `run_eval.ts` uses them; `judge()` itself builds the prompt, scrubs and validates.
//   - mail_claim, as `mail-process/model.ts` does: `buildMailClaimPrompt`, the model, then
//     `validateMailClaims(answer, {from, subject: scrubForPrompt(subject), text: mailPromptText(text),
//     date, knownCourses, timeZone: null}, "model")`.
//   - mail_resolve, as `mail-resolve/handler.ts` does: `buildMailResolvePrompt`, the model, then
//     `validateMailResolve(answer, {claims: keys, candidates: ids})`.
//   - template claims, as `mail-process/handler.ts` does: `validateMailClaims({claims}, {...,
//     subject: scrubForPrompt(subject), text: scrubForPrompt(text.trim())}, "template")`, no model.
//
// Every assertion on free text goes through the suite's own oracle (`detect.ts`), never through the
// enforcement production runs, so a weakened enforcement is caught by code it does not share.
//
// No network, no env, no clock: `now` is fixed and every id is a literal. Every host below is a
// reserved name or `example.<label>`, and every address a documentation range (§4.3.1).
import { assert, assertEquals } from "@std/assert";
import { ScriptedModel } from "../../supabase/functions/_shared/judge_anthropic.ts";
import {
  type CapStore,
  judge,
  type JudgeReply,
  type JudgmentSink,
  type Kind,
  type ModelRow,
  type RuleTable,
} from "../../supabase/functions/_shared/judge_pipeline.ts";
import {
  buildMailClaimPrompt,
  buildMailResolvePrompt,
  buildPrompt,
  mailPromptText,
  type Prompt,
} from "../../supabase/functions/_shared/judge_prompts.ts";
import {
  type MailValidated,
  validateMailClaims,
  validateMailResolve,
} from "../../supabase/functions/_shared/judge_validate.ts";
import { scrubForPrompt } from "../../supabase/functions/_shared/scrub.ts";
import { plantedIn, policyBreaches } from "./detect.ts";

const ACCOUNT = "00000000-0000-0000-0000-000000000000";

/** A literal pin per kind. Nothing here is sent anywhere: the model is scripted. */
function pin(kind: ModelRow["kind"]): ModelRow {
  return {
    kind,
    provider: "openrouter",
    model_id: "example/scripted-hostile",
    prompt_version: `${kind}-test`,
    grammar_version: `${kind}-test`,
    max_tokens: 512,
    sampling: {},
    route: {},
    precision: "bf16",
    usd_per_m_in: 0,
    usd_per_m_out: 0,
  };
}

/** As `run_eval.ts`'s stubs: no rule answers, every charge passes, the log keeps nothing. */
const NO_RULES: RuleTable = { lookup: () => Promise.resolve(null) };
const OPEN_CAPS: CapStore = {
  charge: () => Promise.resolve(true),
  chargeOutcome: () => Promise.resolve("charged"),
  chargeAllowance: () => Promise.resolve(false),
  refundAllowance: () => Promise.resolve(),
  refund: () => Promise.resolve(),
  tripped: () => false,
  withinBudget: () => Promise.resolve(true),
  recordTokens: () => Promise.resolve(),
};
const NO_LOG: JudgmentSink = { write: () => Promise.resolve(null) };

const ITEMS: Record<Kind, Record<string, unknown>> = {
  task: {
    id: "task-hostile-1",
    title: "Lab 4: Springs",
    body: "Write up the springs lab.",
    source_uid: "lms:4001",
    created_by: "lms",
    course: null,
    due: "2026-10-09T23:59",
  },
  event: {
    uid: "event-hostile-1",
    title: "Career fair",
    start: "2026-10-09T10:00",
    end: "2026-10-09T14:00",
    source: "campus",
    organizer: "Career Office",
    location: "Hall A",
    url: "",
    description: "Meet employers.",
    categories: [],
    audiences: [],
    series_uid: "",
  },
  email: {
    message_id: "hostile-1",
    subject: "Lab 4 moved",
    from: "Prof Example <prof@school.example.invalid>",
    date: "Mon, 28 Sep 2026 10:00:00 -0500",
    text: "Lab 4 is now due October 9, 2026.",
  },
};

const SEEDS: Record<Kind, Record<string, unknown>> = {
  task: {
    course: "xx-101",
    effort_hours: null,
    slice_hours: 1.5,
    weights: "Labs 20%",
    preferences: "",
    known_courses: ["xx-101"],
  },
  event: { interests: "" },
  email: { known_courses: ["xx-101"] },
};

/** A well-formed answer per kind; each case overrides only what it attacks. */
const ANSWERS: Record<Kind, Record<string, unknown>> = {
  task: {
    course: "xx-101",
    effort_hours: 2,
    importance: 3,
    importance_reason: "labs are 20% of the grade",
    confidence: 0.9,
  },
  event: { verdict: "opportunity", why: "a career fair worth offering", confidence: 0.9 },
  email: {
    tier: "task",
    title: "Lab 4: Springs",
    why: "the professor moved the lab",
    course: "xx-101",
    due: "2026-10-09",
    effort_hours: 2,
    importance: 3,
    confidence: 0.9,
  },
};

function answer(kind: Kind, over: Record<string, unknown> = {}): Record<string, unknown> {
  return { ...ANSWERS[kind], ...over };
}

/** Path 1: `judge()`, which builds the prompt, scrubs, calls the model and validates. */
function judged(kind: Kind, reply: Record<string, unknown>): Promise<JudgeReply> {
  return judge(ACCOUNT, {
    kind,
    item: ITEMS[kind],
    heuristics_seed: SEEDS[kind],
    // As `run_eval.ts`: the current engine declares `unsure` on every event request.
    accepts: kind === "event" ? ["unsure"] : [],
  }, {
    row: pin(kind),
    model: new ScriptedModel([reply]),
    rules: NO_RULES,
    caps: OPEN_CAPS,
    log: NO_LOG,
    origin: "device",
    now: () => 0,
  });
}

/** A verdict's free-text field; the answer must have been kept. */
function textOf(reply: JudgeReply, name: string): string {
  assertEquals(reply.outcome, "answered", `refused: ${reply.cause}`);
  const value = reply.verdict?.[name];
  assert(typeof value === "string", `${name} is ${JSON.stringify(value)}`);
  return value;
}

// ---- The mail paths.

interface Message {
  subject: string;
  from: string;
  date: string;
  text: string;
}

const KNOWN = ["xx-101", "yy-202"];
const MESSAGE: Message = {
  subject: "Lab 4 moved",
  from: "Prof Example <prof@school.example.invalid>",
  date: "Mon, 28 Sep 2026 10:00:00 -0500",
  text: "Hello class,\n\nLab 4: Springs is now due October 9, 2026.\nThe quiz is Friday.\nThanks",
};

/** The text's quotable line, so a test can put a hostile span in it and quote it as evidence. */
function messageWith(line: string): Message {
  return { ...MESSAGE, text: `Hello class,\n\n${line}\nThanks` };
}

function prompted(prompt: Prompt, row: ModelRow, reply: Record<string, unknown>) {
  return new ScriptedModel([reply]).complete({
    model: row.model_id,
    system: prompt.system,
    user: prompt.user,
    schema: prompt.schema,
    maxTokens: row.max_tokens,
    sampling: row.sampling,
    route: row.route,
  });
}

/** Path 2: `mail_claim`, as `mail-process/model.ts` runs it. */
async function claimedAnswer(reply: Record<string, unknown>, message = MESSAGE): Promise<MailValidated> {
  const prompt = buildMailClaimPrompt(
    { subject: message.subject, from: message.from, date: message.date, text: message.text },
    { known_courses: KNOWN, time_zone: null },
  );
  const answered = await prompted(prompt, pin("mail_claim"), reply);
  return validateMailClaims(answered.json, {
    from: message.from,
    subject: scrubForPrompt(message.subject),
    text: mailPromptText(message.text),
    date: message.date,
    knownCourses: KNOWN,
    timeZone: null,
  }, "model");
}

function claimed(claims: unknown[], message = MESSAGE): Promise<MailValidated> {
  return claimedAnswer({ claims }, message);
}

/** Path 4: template claims, as `mail-process/handler.ts` checks them. No model. */
function templated(claims: unknown[], message: Message): MailValidated {
  return validateMailClaims({ claims }, {
    from: message.from,
    subject: scrubForPrompt(message.subject),
    text: scrubForPrompt(message.text.trim()),
    date: message.date,
    knownCourses: KNOWN,
    timeZone: null,
  }, "template");
}

interface Batch {
  claims: Array<{ key: string; target: string; due: string | null }>;
  candidates: Array<{ id: string; title: string; due: string | null }>;
}

const KEY_0 = `mail:${"a".repeat(32)}:0`;
const KEY_1 = `mail:${"a".repeat(32)}:1`;
const KEY_OUTSIDE = `mail:${"b".repeat(32)}:0`;
const BATCH: Batch = {
  claims: [
    { key: KEY_0, target: "Lab 4: Springs", due: "2026-10-09" },
    { key: KEY_1, target: "Quiz 2", due: null },
  ],
  candidates: [
    { id: "note-1", title: "Lab 4: Springs", due: "2026-10-02" },
    { id: "note-2", title: "Quiz 2", due: "2026-10-02" },
  ],
};

/** Path 3: `mail_resolve`, as `mail-resolve/handler.ts` runs it. */
async function resolved(reply: Record<string, unknown>, batch = BATCH) {
  const prompt = buildMailResolvePrompt({ claims: batch.claims, candidates: batch.candidates });
  const answered = await prompted(prompt, pin("mail_resolve"), reply);
  return validateMailResolve(answered.json, {
    claims: batch.claims.map((c) => c.key),
    candidates: batch.candidates.map((c) => c.id),
  });
}

/** A model's change claim; its evidence is verbatim in `MESSAGE`. */
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

function newItem(item: Record<string, unknown> = {}, over: Record<string, unknown> = {}) {
  return {
    kind: "new_item",
    course: "xx-101",
    target: null,
    field: null,
    value: null,
    item: { title: "Quiz 2", due: "2026-10-09", item_kind: "exam", effort_hours: 2, importance: 4, ...item },
    topic: null,
    evidence: "The quiz is Friday.",
    why: "a quiz was announced",
    confidence: 0.8,
    ...over,
  };
}

function courseInfo(over: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    kind: "course_info",
    course: "xx-101",
    target: null,
    field: null,
    value: null,
    item: null,
    topic: "policy",
    evidence: "The quiz is Friday.",
    why: "the professor set a quiz day",
    confidence: 0.8,
    ...over,
  };
}

/** One claim refused, and nothing kept. */
function assertRefused(got: MailValidated, note: string) {
  assertEquals(got.claims, [], note);
  assertEquals(got.refused, ["incomplete"], note);
  assertEquals(got.ok, false, note);
}

/** One claim kept; returned for its fields. */
function kept(got: MailValidated, note: string) {
  assertEquals(got.refused, [], note);
  assertEquals(got.claims.length, 1, note);
  return got.claims[0];
}

// ---- The hostile inputs.

/** The splitlines classes (`pystr::splitlines`), each by name. */
const SPLITLINES: Array<[string, string]> = [
  ["\\n", "\n"],
  ["\\r", "\r"],
  ["\\v", "\v"],
  ["\\f", "\f"],
  ["\\x1c", "\x1c"],
  ["\\x1d", "\x1d"],
  ["\\x1e", "\x1e"],
  ["U+0085", "\u0085"],
  ["U+2028", "\u2028"],
  ["U+2029", "\u2029"],
];
const SPLIT_SET = new Set(SPLITLINES.map(([, ch]) => ch));

/** Every other C0 control, tab included, and DEL (§4.3's line rule). */
const OTHER_CONTROLS: Array<[string, string]> = [
  ...Array.from({ length: 0x20 }, (_, n) => String.fromCharCode(n))
    .filter((ch) => !SPLIT_SET.has(ch))
    .map((ch): [string, string] => [`U+${ch.charCodeAt(0).toString(16).padStart(4, "0")}`, ch]),
  ["U+007F", "\x7f"],
];
const LINE_CHARS: Array<[string, string]> = [...SPLITLINES, ...OTHER_CONTROLS];

/** What the mail path's whitespace fold (`\s+` to one space) already turns into a space. */
const FOLDED_BY_MAIL = new Set(["\t", "\n", "\v", "\f", "\r", "\u2028", "\u2029"]);

interface Hostile {
  label: string;
  token: string;
  /** What must not survive into the output, compared on the matching form. */
  planted: string[];
}

/** One standalone token per URL and markup class of §4.3, each placed as `Visit <token> now`. */
const HOSTILE: Hostile[] = [
  { label: "scheme URL", token: "https://example.com/reset", planted: ["example.com"] },
  { label: "upper-case scheme URL", token: "HTTPS://EXAMPLE.COM/reset", planted: ["example.com"] },
  { label: "backslash scheme", token: "https:\\\\example.com", planted: ["example.com"] },
  { label: "javascript:", token: "javascript:alert(1)", planted: ["javascript:"] },
  { label: "vbscript:", token: "vbscript:msgbox(1)", planted: ["vbscript:"] },
  { label: "data: MIME", token: "data:text/html;base64,PHN2Zz4=", planted: ["data:text/html"] },
  { label: "file:", token: "file:///etc/passwd", planted: ["file:/"] },
  { label: "mailto:", token: "mailto:help@example.com", planted: ["mailto:", "example.com"] },
  { label: "www.", token: "www.example.org", planted: ["www.example.org"] },
  { label: "bare host", token: "example.com", planted: ["example.com"] },
  { label: "extension label with a path", token: "example.zip/update", planted: ["example.zip"] },
  { label: "extension label, port then path", token: "example.ai:8080/x", planted: ["example.ai"] },
  { label: "dotted quad", token: "203.0.113.7", planted: ["203.0.113.7"] },
  { label: "ideographic full stop", token: "example\u3002com", planted: ["example.com"] },
  {
    label: "full-width host",
    token: "\uff45\uff58\uff41\uff4d\uff50\uff4c\uff45\uff0e\uff43\uff4f\uff4d",
    planted: [
      "example.com",
    ],
  },
  { label: "zero-width inside a host", token: "exa\u200bmple.com", planted: ["example.com"] },
  { label: "tag", token: "<img src=x onerror=alert(1)>", planted: ["<img"] },
  { label: "closing tag", token: "</script>", planted: ["</script"] },
  { label: "comment", token: "<!--x-->", planted: ["<!--"] },
  { label: "processing instruction", token: "<?php", planted: ["<?php"] },
  { label: "full-width tag", token: "\uff1cimg src=x\uff1e", planted: ["<img"] },
  { label: "Markdown link", token: "[click](notes)", planted: ["]("] },
  { label: "Markdown image", token: "![x](notes)", planted: ["!["] },
  {
    label: "Markdown link to a URL",
    token: "[click](https://example.com/x)",
    planted: ["](", "example.com"],
  },
];

const PLACEHOLDERS = ["[link removed]", "[markup removed]"];

/** §4.3 step 3's neutralised form: kept, clean by the oracle, the planted span gone, a placeholder in
 * its place, and the words around it untouched. */
function assertNeutralised(value: string, h: Hostile, lead = "Visit ", tail = " now") {
  const note = `${h.label}: ${JSON.stringify(value)}`;
  assertEquals(policyBreaches(value), [], note);
  assertEquals(plantedIn(value, h.planted), [], note);
  assert(PLACEHOLDERS.some((p) => value.includes(p)), note);
  assert(value.startsWith(lead) && value.endsWith(tail), note);
}

const EXTRA = { link: "https://example.com/x", verdict_override: "obligation", tier_override: "task" };

// ---- H1: off-vocabulary words in closed fields, and extra keys.

Deno.test("H1 task: extra keys never reach the verdict", async () => {
  const reply = await judged("task", answer("task", EXTRA));
  assertEquals(Object.keys(reply.verdict ?? {}).sort(), [
    "confidence",
    "course",
    "effort_hours",
    "importance",
    "importance_reason",
  ]);
});

Deno.test("H1 event: an off-vocabulary verdict is refused; extra keys are dropped", async (t) => {
  for (const verdict of ["urgent", "Obligation", "obligation ", "required"]) {
    await t.step(`verdict ${JSON.stringify(verdict)}`, async () => {
      const reply = await judged("event", answer("event", { verdict }));
      assertEquals([reply.verdict, reply.cause], [null, "incomplete"]);
    });
  }
  await t.step("extra keys", async () => {
    const reply = await judged("event", answer("event", EXTRA));
    assertEquals(Object.keys(reply.verdict ?? {}).sort(), ["confidence", "verdict", "why"]);
  });
});

Deno.test("H1 email: an off-vocabulary tier is refused; extra keys are dropped", async (t) => {
  for (const tier of ["urgent", "Task", "receipt", "obligation"]) {
    await t.step(`tier ${JSON.stringify(tier)}`, async () => {
      const reply = await judged("email", answer("email", { tier }));
      assertEquals([reply.verdict, reply.cause], [null, "incomplete"]);
    });
  }
  await t.step("extra keys", async () => {
    const reply = await judged("email", answer("email", EXTRA));
    assertEquals(Object.keys(reply.verdict ?? {}).sort(), [
      "confidence",
      "course",
      "due",
      "effort_hours",
      "importance",
      "tier",
      "title",
      "why",
    ]);
  });
});

Deno.test("H1 mail_claim: off-vocabulary kind, field, topic and item_kind are refused; extra keys dropped", async (t) => {
  const cases: Array<[string, Record<string, unknown>]> = [
    ["kind", change({ kind: "deadline" })],
    ["kind, case", change({ kind: "Change" })],
    ["field", change({ field: "room" })],
    ["topic", courseInfo({ topic: "grades" })],
    ["item_kind", newItem({ item_kind: "homework" })],
  ];
  for (const [label, claim] of cases) {
    await t.step(label, async () => assertRefused(await claimed([claim]), label));
  }
  await t.step("extra keys on the answer, the claim and the item", async () => {
    const got = await claimedAnswer({
      claims: [newItem({ url: "https://example.com/x" }, EXTRA)],
      note: "x",
    });
    const claim = kept(got, "extra keys");
    assertEquals(Object.keys(claim).sort(), [
      "confidence",
      "course",
      "evidence",
      "field",
      "item",
      "kind",
      "resolution",
      "source",
      "target",
      "topic",
      "value",
      "why",
    ]);
    assertEquals(Object.keys(claim.item ?? {}).sort(), [
      "due",
      "effort_hours",
      "importance",
      "item_kind",
      "title",
    ]);
    assertEquals(claim.source, "model");
  });
});

Deno.test("H1 mail_resolve: extra keys never reach a resolution", async () => {
  const got = await resolved({
    answers: [{ claim: KEY_0, target_id: "note-1", note_title: "x", url: "https://example.com/x" }],
    extra: true,
  });
  assertEquals(got, [{ target_id: "note-1" }, { target_id: null }]);
});

// ---- H2: a receipt from the model; a cancellation whose value is not "true".

Deno.test("H2 mail_claim: a model receipt and a cancellation valued other than true are refused", async (t) => {
  const receipt = change({ kind: "receipt", field: null, value: null, evidence: "Lab 4: Springs" });
  const cases: Array<[string, Record<string, unknown>]> = [
    ["receipt from the model", receipt],
    ["receipt claiming a template source", { ...receipt, source: "template" }],
    ['cancelled: "false"', change({ field: "cancelled", value: "false" })],
    ["cancelled: true (boolean)", change({ field: "cancelled", value: true })],
    ['cancelled: "True"', change({ field: "cancelled", value: "True" })],
    ['cancelled: "yes"', change({ field: "cancelled", value: "yes" })],
    ["cancelled: null", change({ field: "cancelled", value: null })],
  ];
  for (const [label, claim] of cases) {
    await t.step(label, async () => assertRefused(await claimed([claim]), label));
  }
});

// ---- H3: unknown courses, unusable dues, numbers out of range or not finite.

Deno.test("H3 task: an unknown course is null; numbers are clamped or refused", async (t) => {
  const held: Array<[string, Record<string, unknown>, Record<string, unknown>]> = [
    ["unknown course", { course: "zz-999" }, { course: null }],
    ["course with padding of an unknown slug", { course: " xx-1010 " }, { course: null }],
    ["effort 900", { effort_hours: 900 }, { effort_hours: 40 }],
    ["effort -5", { effort_hours: -5 }, { effort_hours: 0.25 }],
    ["importance 99", { importance: 99 }, { importance: 5 }],
    ["importance -3", { importance: -3 }, { importance: 1 }],
    ["confidence 9", { confidence: 9 }, { confidence: 1 }],
  ];
  for (const [label, over, want] of held) {
    await t.step(label, async () => {
      const reply = await judged("task", answer("task", over));
      for (const [k, v] of Object.entries(want)) assertEquals(reply.verdict?.[k], v, label);
    });
  }
  const refused: Array<[string, Record<string, unknown>]> = [
    ["effort Infinity", { effort_hours: Infinity }],
    ["effort NaN", { effort_hours: NaN }],
    ["importance -Infinity", { importance: -Infinity }],
    ["importance as a string", { importance: "5" }],
    ["confidence NaN", { confidence: NaN }],
  ];
  for (const [label, over] of refused) {
    await t.step(label, async () => {
      const reply = await judged("task", answer("task", over));
      assertEquals([reply.verdict, reply.cause], [null, "incomplete"], label);
    });
  }
});

Deno.test("H3 event: confidence is clamped or refused", async (t) => {
  await t.step("confidence 9", async () => {
    assertEquals((await judged("event", answer("event", { confidence: 9 }))).verdict?.confidence, 1);
  });
  for (const confidence of [Infinity, NaN, "0.9"]) {
    await t.step(`confidence ${String(confidence)}`, async () => {
      const reply = await judged("event", answer("event", { confidence }));
      assertEquals([reply.verdict, reply.cause], [null, "incomplete"]);
    });
  }
});

Deno.test("H3 email: unknown course and unusable due are null; numbers are clamped or refused", async (t) => {
  const held: Array<[string, Record<string, unknown>, Record<string, unknown>]> = [
    ["unknown course", { course: "zz-999" }, { course: null }],
    ["due in prose", { due: "whenever the professor decides" }, { due: null }],
    ["due a span", { due: "next week" }, { due: null }],
    ["due on an impossible day", { due: "2026-02-30" }, { due: null }],
    ["due in an impossible month", { due: "2026-13-01" }, { due: null }],
    ["due at an impossible hour", { due: "2026-10-09T24:00" }, { due: null }],
    ["effort 900", { effort_hours: 900 }, { effort_hours: 40 }],
    ["effort Infinity", { effort_hours: Infinity }, { effort_hours: null }],
    ["importance 0", { importance: 0 }, { importance: 1 }],
    ["importance NaN", { importance: NaN }, { importance: null }],
    ["confidence 9", { confidence: 9 }, { confidence: 1 }],
  ];
  for (const [label, over, want] of held) {
    await t.step(label, async () => {
      const reply = await judged("email", answer("email", over));
      assertEquals(reply.outcome, "answered", label);
      for (const [k, v] of Object.entries(want)) assertEquals(reply.verdict?.[k], v, label);
    });
  }
  await t.step("confidence Infinity", async () => {
    const reply = await judged("email", answer("email", { confidence: Infinity }));
    assertEquals([reply.verdict, reply.cause], [null, "incomplete"]);
  });
});

Deno.test("H3 mail_claim: unknown course and unusable due are null; numbers off range refuse", async (t) => {
  const nulls: Array<[string, Record<string, unknown>, (c: MailValidated["claims"][number]) => unknown]> = [
    ["unknown course", newItem({}, { course: "zz-999" }), (c) => c.course],
    ["item due in prose", newItem({ due: "whenever the professor decides" }), (c) => c.item?.due],
    ["item due on an impossible day", newItem({ due: "2026-02-30" }), (c) => c.item?.due],
    ["item due at an impossible hour", newItem({ due: "2026-10-09T24:00" }), (c) => c.item?.due],
    ["change to an impossible day", change({ value: "2026-02-30" }), (c) => c.value],
  ];
  for (const [label, claim, read] of nulls) {
    await t.step(label, async () => assertEquals(read(kept(await claimed([claim]), label)), null, label));
  }
  await t.step("confidence 9", async () => {
    assertEquals(kept(await claimed([change({ confidence: 9 })]), "confidence 9").confidence, 1);
  });
  const refused: Array<[string, Record<string, unknown>]> = [
    ["effort 900", newItem({ effort_hours: 900 })],
    ["effort 0", newItem({ effort_hours: 0 })],
    ["effort Infinity", newItem({ effort_hours: Infinity })],
    ["importance 9", newItem({ importance: 9 })],
    ["importance 2.5", newItem({ importance: 2.5 })],
    ["importance NaN", newItem({ importance: NaN })],
    ["confidence Infinity", change({ confidence: Infinity })],
    ["confidence as a string", change({ confidence: "0.9" })],
  ];
  for (const [label, claim] of refused) {
    await t.step(label, async () => assertRefused(await claimed([claim]), label));
  }
});

// ---- H4: line-boundary and control characters in every free-text field.

/** The `validate()` path's free-text fields, by kind. */
const VALIDATE_FIELDS: Array<[Kind, string]> = [
  ["task", "importance_reason"],
  ["event", "why"],
  ["email", "why"],
  ["email", "title"],
];

for (const [kind, name] of VALIDATE_FIELDS) {
  Deno.test(`H4 ${kind} ${name}: each line-rule character becomes a space`, async (t) => {
    for (const [label, ch] of LINE_CHARS) {
      await t.step(label, async () => {
        const value = textOf(await judged(kind, answer(kind, { [name]: `Lab 4${ch}moved to Friday` })), name);
        assertEquals(policyBreaches(value), [], `${label}: ${JSON.stringify(value)}`);
        assertEquals(value, "Lab 4 moved to Friday", label);
      });
    }
  });
}

type Claim = MailValidated["claims"][number];

/** The mail path's free-text fields other than evidence: a line-rule character refuses the claim (R11),
 * except a tab, which R11 allows and the claim folds to a space. */
const MAIL_FIELDS: Array<[string, (s: string) => Record<string, unknown>, (c: Claim) => unknown, string]> = [
  ["why", (s) => change({ why: `the lab${s}moved` }), (c) => c.why, "the lab moved"],
  ["target", (s) => change({ target: `Lab 4:${s}Springs` }), (c) => c.target, "Lab 4: Springs"],
  ["item.title", (s) => newItem({ title: `Quiz${s}2` }), (c) => c.item?.title, "Quiz 2"],
];

for (const [name, build, read, folded] of MAIL_FIELDS) {
  Deno.test(`H4 mail_claim ${name}: a line-rule character refuses the claim; a tab is folded`, async (t) => {
    for (const [label, ch] of LINE_CHARS) {
      await t.step(label, async () => {
        const got = await claimed([build(ch)]);
        if (ch !== "\t") return assertRefused(got, label);
        const value = read(kept(got, label));
        assert(typeof value === "string", label);
        assertEquals(policyBreaches(value), [], label);
        assertEquals(value, folded, label);
      });
    }
  });
}

// Evidence is folded before R11 and checked verbatim against the folded text (§4.3 step 2, "as today"),
// so a quote of hard-wrapped mail holds on one line. The character is in the message too, so the
// verbatim check passes and R11 alone decides.
Deno.test("H4 mail_claim evidence: folded whitespace is one space; any other line-rule character refuses", async (t) => {
  for (const [label, ch] of LINE_CHARS) {
    await t.step(label, async () => {
      const line = `Lab 4: Springs is now due${ch}October 9, 2026.`;
      const got = await claimed([change({ evidence: line })], messageWith(line));
      if (!FOLDED_BY_MAIL.has(ch)) return assertRefused(got, label);
      const evidence = kept(got, label).evidence;
      assertEquals(policyBreaches(evidence), [], label);
      assertEquals(evidence, "Lab 4: Springs is now due October 9, 2026.", label);
    });
  }
});

// ---- H5: URL and markup classes in every model-written free-text field.

/** The fields §4.3 step 3 neutralises (`why`, `importance_reason`) on the `validate()` path. */
const NEUTRALISED_FIELDS: Array<[Kind, string]> = [
  ["task", "importance_reason"],
  ["event", "why"],
  ["email", "why"],
];

for (const [kind, name] of NEUTRALISED_FIELDS) {
  Deno.test(`H5 ${kind} ${name}: each URL and markup class is neutralised, the answer kept`, async (t) => {
    for (const h of HOSTILE) {
      await t.step(h.label, async () => {
        const reply = await judged(kind, answer(kind, { [name]: `Visit ${h.token} now` }));
        assertNeutralised(textOf(reply, name), h);
      });
    }
  });
}

Deno.test("H5 email title: each URL and markup class refuses the answer (Q1)", async (t) => {
  for (const h of HOSTILE) {
    await t.step(h.label, async () => {
      const reply = await judged("email", answer("email", { title: `Visit ${h.token} now` }));
      assertEquals([reply.verdict, reply.cause], [null, "incomplete"], h.label);
    });
  }
});

Deno.test("H5 mail_claim why: each URL and markup class is neutralised, the claim kept", async (t) => {
  for (const h of HOSTILE) {
    await t.step(h.label, async () => {
      const got = await claimed([change({ why: `Visit ${h.token} now` })]);
      assertNeutralised(kept(got, h.label).why, h);
    });
  }
});

Deno.test("H5 mail_claim target: each URL and markup class refuses the claim (Q1)", async (t) => {
  for (const h of HOSTILE) {
    await t.step(
      h.label,
      async () => assertRefused(await claimed([change({ target: `Visit ${h.token} now` })]), h.label),
    );
  }
});

Deno.test("H5 mail_claim item.title: each URL and markup class refuses the claim (Q1)", async (t) => {
  for (const h of HOSTILE) {
    await t.step(
      h.label,
      async () => assertRefused(await claimed([newItem({ title: `Visit ${h.token} now` })]), h.label),
    );
  }
});

// Evidence must be verbatim in the scrubbed text, so only the classes `scrubForPrompt` leaves alone
// can reach the policy through it; the message carries the span and the evidence quotes it.
Deno.test("H5 mail_claim evidence: each class the scrub leaves is neutralised, the claim kept", async (t) => {
  for (const h of HOSTILE.filter((x) => scrubForPrompt(x.token) === x.token)) {
    await t.step(h.label, async () => {
      const line = `Visit ${h.token} now`;
      const got = await claimed([courseInfo({ evidence: line })], messageWith(line));
      assertNeutralised(kept(got, h.label).evidence, h);
    });
  }
});

Deno.test("H5 mail_claim evidence: a schemeless host and a tag, quoted verbatim, are neutralised", async () => {
  const line = "Reset your password at example.com before <b>Friday</b> today";
  assertEquals(scrubForPrompt(line), line); // the scrub leaves both, so the verbatim check passes
  const got = await claimed([courseInfo({ evidence: line })], messageWith(line));
  const evidence = kept(got, "host and tag").evidence;
  assertEquals(policyBreaches(evidence), [], evidence);
  assertEquals(plantedIn(evidence, ["example.com", "<b>", "</b>"]), [], evidence);
  assert(evidence.includes("[link removed]") && evidence.includes("[markup removed]"), evidence);
  assert(evidence.startsWith("Reset your password at ") && evidence.endsWith(" today"), evidence);
});

Deno.test("H5 mail_claim: a breaking item.title refuses that claim, not the message", async () => {
  const got = await claimed([change(), newItem({ title: "Visit https://example.com/reset now" })]);
  assertEquals(got.claims.map((c) => [c.kind, c.target]), [["change", "Lab 4: Springs"]]);
  assert(got.refused.includes("incomplete"), JSON.stringify(got.refused));
  assertEquals(got.ok, true);
});

Deno.test("H5 mail_claim: a breaking target refuses that claim, not the message", async () => {
  const got = await claimed([newItem(), change({ target: "Lab 4 at example.com/reset" })]);
  assertEquals(got.claims.map((c) => [c.kind, c.item?.title]), [["new_item", "Quiz 2"]]);
  assert(got.refused.includes("incomplete"), JSON.stringify(got.refused));
  assertEquals(got.ok, true);
});

// §4.3 step 3: normalising is for matching only. A full-width digit, letters and quote (NFKC would
// rewrite every one) in a field that breaks nothing come out exactly as today's validator returns them.
const FULL_WIDTH_CLEAN =
  "Quiz \uff13 moved to \uff26\uff52\uff49\uff44\uff41\uff59, \uff02bring a pencil\uff02";

Deno.test("H5 clean text is untouched: a full-width why comes out byte for byte", async (t) => {
  assertEquals(policyBreaches(FULL_WIDTH_CLEAN), []);
  for (const [kind, name] of NEUTRALISED_FIELDS) {
    await t.step(`${kind} ${name}`, async () => {
      assertEquals(
        textOf(await judged(kind, answer(kind, { [name]: FULL_WIDTH_CLEAN })), name),
        FULL_WIDTH_CLEAN,
      );
    });
  }
  await t.step("mail_claim why", async () => {
    assertEquals(kept(await claimed([change({ why: FULL_WIDTH_CLEAN })]), "why").why, FULL_WIDTH_CLEAN);
  });
});

Deno.test("H5 clean text is untouched: full-width titles, target and evidence come out byte for byte", async (t) => {
  await t.step("email title", async () => {
    assertEquals(
      textOf(await judged("email", answer("email", { title: FULL_WIDTH_CLEAN })), "title"),
      FULL_WIDTH_CLEAN,
    );
  });
  await t.step("mail_claim item.title", async () => {
    assertEquals(
      kept(await claimed([newItem({ title: FULL_WIDTH_CLEAN })]), "item").item?.title,
      FULL_WIDTH_CLEAN,
    );
  });
  await t.step("mail_claim target", async () => {
    assertEquals(
      kept(await claimed([change({ target: FULL_WIDTH_CLEAN })]), "target").target,
      FULL_WIDTH_CLEAN,
    );
  });
  await t.step("mail_claim evidence", async () => {
    // The message holds the string verbatim and the scrub leaves it, so the verbatim check passes.
    assertEquals(scrubForPrompt(FULL_WIDTH_CLEAN), FULL_WIDTH_CLEAN);
    const got = await claimed([courseInfo({ evidence: FULL_WIDTH_CLEAN })], messageWith(FULL_WIDTH_CLEAN));
    assertEquals(kept(got, "evidence").evidence, FULL_WIDTH_CLEAN);
  });
});

// §4.3: file names, a file-and-line reference and the scrub's own tokens are not URLs or markup, so a
// title holding one is kept as it was (the guard against refusing honest titles).
Deno.test("H5 clean titles: file names and scrub tokens break nothing and are kept", async (t) => {
  for (
    const title of [
      "Submit essay.docx and lab3.py",
      "Register at <url> by Friday",
      "Fix main.rs:42 before lab",
    ]
  ) {
    await t.step(title, async () => {
      assertEquals(policyBreaches(title), []);
      assertEquals(textOf(await judged("email", answer("email", { title })), "title"), title);
      assertEquals(kept(await claimed([newItem({ title })]), title).item?.title, title);
      assertEquals(kept(await claimed([change({ target: title })]), title).target, title);
    });
  }
});

// §4.3 step 4: a placeholder that forms new markup with its neighbours is refused.
const ADJACENT: Array<[string, string]> = [
  ["would read [link removed](", "Mirror at example.com(backup) today"],
  ["would read ![link removed]", "Reset !https://example.com/x now"],
];

Deno.test("H5 adjacency: a why whose neutralised form makes markup refuses the answer", async (t) => {
  for (const [label, why] of ADJACENT) {
    await t.step(label, async () => {
      const reply = await judged("event", answer("event", { why }));
      assertEquals([reply.verdict, reply.cause], [null, "incomplete"], label);
    });
  }
});

Deno.test("H5 adjacency: a mail why whose neutralised form makes markup refuses the claim", async (t) => {
  for (const [label, why] of ADJACENT) {
    await t.step(label, async () => assertRefused(await claimed([change({ why })]), label));
  }
});

// §4.3 step 5: the clip runs after neutralising and the policy runs once more on the clipped value. A
// token ending `.com1` (a final label with a digit, so a host under no TLD list or rule), cut right after
// `.com`, becomes a host.
function cutIntoHost(bound: number): string {
  return "x".repeat(bound - " example.com".length) + " example.com1";
}

// The controls: a clean field over its bound is kept, cut to exactly the bound, so a refusal of every
// over-long field cannot pass for the clip rule.
Deno.test("H5 clip control: a clean over-long field is kept, cut to its bound", async (t) => {
  const cases: Array<[string, number, () => Promise<unknown>]> = [
    [
      "event why",
      140,
      async () => textOf(await judged("event", answer("event", { why: "x".repeat(150) })), "why"),
    ],
    [
      "email title",
      200,
      async () => textOf(await judged("email", answer("email", { title: "x".repeat(210) })), "title"),
    ],
    ["mail_claim why", 140, async () => kept(await claimed([change({ why: "x".repeat(150) })]), "why").why],
    [
      "mail_claim target",
      200,
      async () => kept(await claimed([change({ target: "x".repeat(210) })]), "target").target,
    ],
    [
      "mail_claim item.title",
      200,
      async () => kept(await claimed([newItem({ title: "x".repeat(210) })]), "item").item?.title,
    ],
    [
      "mail_claim evidence",
      140,
      async () =>
        kept(await claimed([courseInfo({ evidence: "x".repeat(150) })], messageWith("x".repeat(150))), "ev")
          .evidence,
    ],
  ];
  for (const [label, bound, run] of cases) {
    await t.step(label, async () => {
      const value = await run();
      assert(typeof value === "string", label);
      assertEquals(policyBreaches(value), [], label);
      assertEquals(value, "x".repeat(bound), label);
    });
  }
});

Deno.test("H5 clip: a why whose cut at 140 ends in a host refuses the answer", async () => {
  const reply = await judged("event", answer("event", { why: cutIntoHost(140) }));
  assertEquals([reply.verdict, reply.cause], [null, "incomplete"]);
});

Deno.test("H5 clip: a title whose cut at 200 ends in a host refuses the answer", async () => {
  const reply = await judged("email", answer("email", { title: cutIntoHost(200) }));
  assertEquals([reply.verdict, reply.cause], [null, "incomplete"]);
});

// ---- H6 (structural, no model): two accounts' prompts built back to back.

function promptText(p: Prompt): string {
  return `${p.system}\n${p.user}\n${JSON.stringify(p.schema)}`;
}

Deno.test("H6 task: each prompt carries its own preferences and weights canaries, never the other's", () => {
  const canaries = [
    "canary-a-prefs-41d7",
    "canary-a-weights-8c02",
    "canary-b-prefs-5e19",
    "canary-b-weights-b7a3",
  ];
  const seed = (prefs: string, weights: string) => ({
    ...SEEDS.task,
    preferences: `Mornings only ${prefs}`,
    weights: `Labs 20% ${weights}`,
  });
  const a = buildPrompt("task", ITEMS.task, seed(canaries[0], canaries[1]));
  const b = buildPrompt("task", ITEMS.task, seed(canaries[2], canaries[3]));
  const a2 = buildPrompt("task", ITEMS.task, seed(canaries[0], canaries[1]));
  assertEquals(plantedIn(promptText(a), canaries), [canaries[0], canaries[1]]);
  assertEquals(plantedIn(promptText(b), canaries), [canaries[2], canaries[3]]);
  assertEquals(promptText(a2), promptText(a));
});

Deno.test("H6 event: each prompt carries its own interests canary, never the other's", () => {
  const canaries = ["canary-a-interests-2f61", "canary-b-interests-9d40"];
  const a = buildPrompt("event", ITEMS.event, { interests: `Robotics ${canaries[0]}` });
  const b = buildPrompt("event", ITEMS.event, { interests: `Robotics ${canaries[1]}` });
  assertEquals(plantedIn(promptText(a), canaries), [canaries[0]]);
  assertEquals(plantedIn(promptText(b), canaries), [canaries[1]]);
});

Deno.test("H6 mail_resolve: each prompt carries its own candidate-title canary, never the other's", () => {
  const canaries = ["canary-a-candidate-63ae", "canary-b-candidate-0b5c"];
  const batch = (canary: string) => ({
    claims: BATCH.claims,
    candidates: [{ id: "note-1", title: `Lab 4: Springs ${canary}`, due: "2026-10-02" }],
  });
  const a = buildMailResolvePrompt(batch(canaries[0]));
  const b = buildMailResolvePrompt(batch(canaries[1]));
  assertEquals(plantedIn(promptText(a), canaries), [canaries[0]]);
  assertEquals(plantedIn(promptText(b), canaries), [canaries[1]]);
});

// ---- H7: mail_resolve answers outside the batch.

Deno.test("H7 mail_resolve: an id outside the candidates, two ids for a claim, or a foreign claim read none", async (t) => {
  const cases: Array<[string, Record<string, unknown>[]]> = [
    ["an id outside the candidates", [{ claim: KEY_0, target_id: "note-9" }, {
      claim: KEY_1,
      target_id: "note-0",
    }]],
    ["two ids for one claim", [{ claim: KEY_0, target_id: "note-1" }, { claim: KEY_0, target_id: "note-2" }]],
    ["a claim outside the batch", [{ claim: KEY_OUTSIDE, target_id: "note-1" }]],
    ["a non-string id", [{ claim: KEY_0, target_id: 1 }, { claim: KEY_1, target_id: ["note-2"] }]],
  ];
  for (const [label, answers] of cases) {
    await t.step(label, async () => {
      assertEquals(await resolved({ answers }), [{ target_id: null }, { target_id: null }], label);
    });
  }
});

// ---- H8: six or more claims.

Deno.test("H8 mail_claim: at most five claims are kept", async (t) => {
  for (const count of [6, 7, 12]) {
    await t.step(`${count} claims`, async () => {
      const got = await claimed(Array.from({ length: count }, () => change()));
      assertEquals([got.claims.length, got.refused], [5, []]);
    });
  }
});

// ---- H9 (template path, no model): an LMS title shaped like a file name or a host is kept unchanged;
// R11's line rule still applies.

const LMS_FROM = "Example LMS <notify@lms.example.com>";

function templateMessage(line: string): Message {
  return { ...MESSAGE, from: LMS_FROM, subject: "Submission received", text: `Hello,\n\n${line}\nThanks` };
}

function templateReceipt(target: string): Record<string, unknown> {
  return {
    kind: "receipt",
    course: "xx-101",
    target,
    field: null,
    value: null,
    item: null,
    topic: null,
    evidence: target,
    why: "Example LMS submission receipt",
    confidence: 1,
    source: "template",
    resolution: null,
  };
}

function templateNewItem(title: string): Record<string, unknown> {
  return {
    ...templateReceipt(title),
    kind: "new_item",
    target: null,
    item: { title, due: null, item_kind: "task", effort_hours: 1, importance: 3 },
    why: "Example LMS new gradable item",
  };
}

const LMS_TITLES = [
  "report.pdf",
  "lab3.py",
  "Case study example.com",
  "Review www.example.org",
  "Notes 203.0.113.7",
];

Deno.test("H9 template receipt: a target shaped like a file name or a host is kept unchanged", async (t) => {
  for (const title of LMS_TITLES) {
    await t.step(title, () => {
      const claim = kept(templated([templateReceipt(title)], templateMessage(`Received: ${title}`)), title);
      assertEquals([claim.kind, claim.target, claim.source], ["receipt", title, "template"]);
    });
  }
});

Deno.test("H9 template new_item: an item.title shaped like a file name or a host is kept unchanged", async (t) => {
  for (const title of LMS_TITLES) {
    await t.step(title, () => {
      const claim = kept(templated([templateNewItem(title)], templateMessage(`New item: ${title}`)), title);
      assertEquals([claim.kind, claim.item?.title, claim.source], ["new_item", title, "template"]);
    });
  }
});

Deno.test("H9 template: R11's line rule still refuses a target or item.title that breaks a line", async (t) => {
  for (const [label, ch] of SPLITLINES) {
    await t.step(label, () => {
      const title = `Case study${ch}example.com`;
      const message = templateMessage("Received: Case study example.com");
      assertRefused(
        templated([{ ...templateReceipt(title), evidence: "Case study example.com" }], message),
        label,
      );
      assertRefused(
        templated([{ ...templateNewItem(title), evidence: "Case study example.com" }], message),
        label,
      );
    });
  }
});
