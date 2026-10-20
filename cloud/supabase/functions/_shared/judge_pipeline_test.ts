import { assert, assertEquals } from "@std/assert";
import { ModelRefused, ScriptedModel } from "./judge_anthropic.ts";
import type { ChargeOutcome } from "./judge_caps.ts";
import {
  type CapStore,
  DAILY_CAP,
  fieldsOf,
  judge,
  type JudgmentRow,
  type JudgmentSink,
  type ModelRow,
  type RuleTable,
} from "./judge_pipeline.ts";

const ROW: ModelRow = {
  kind: "task",
  provider: "anthropic",
  model_id: "claude-haiku-4-5",
  prompt_version: "task-1",
  grammar_version: "task-1",
  max_tokens: 256,
  sampling: { temperature: 0 },
  route: {},
  precision: "bf16",
  usd_per_m_in: 1.0,
  usd_per_m_out: 5.0,
};

class Sink implements JudgmentSink {
  readonly rows: JudgmentRow[] = [];
  write(row: JudgmentRow): Promise<string | null> {
    this.rows.push(row);
    return Promise.resolve(`judgment-${this.rows.length}`);
  }
}

class Caps implements CapStore {
  charged: Array<[string, string]> = [];
  refunded: Array<[string, string]> = [];
  tokens: Array<[string, string, number, number]> = [];
  allowanceCharged: Array<[string, string]> = [];
  allowanceRefunded: Array<[string, string]> = [];
  /** `capError`: a refused daily charge reads as `"error"`, not `"refused"` (L3). `allowanceAllow`:
   *  what the first-day event allowance answers. Both default off, so every older test keeps its
   *  meaning whether the pipeline calls `charge` or `chargeOutcome`. */
  constructor(
    private readonly allow = true,
    private readonly budget = true,
    private readonly open = false,
    private readonly capError = false,
    private readonly allowanceAllow = false,
  ) {}
  charge(account: string, kind: "task" | "event" | "email"): Promise<boolean> {
    this.charged.push([account, kind]);
    return Promise.resolve(this.allow);
  }
  chargeOutcome(account: string, kind: "task" | "event" | "email"): Promise<ChargeOutcome> {
    this.charged.push([account, kind]);
    return Promise.resolve(this.allow ? "charged" : this.capError ? "error" : "refused");
  }
  chargeAllowance(account: string, kind: "task" | "event" | "email"): Promise<boolean> {
    this.allowanceCharged.push([account, kind]);
    return Promise.resolve(this.allowanceAllow);
  }
  refundAllowance(account: string, kind: "task" | "event" | "email"): Promise<void> {
    this.allowanceRefunded.push([account, kind]);
    return Promise.resolve();
  }
  refund(account: string, kind: "task" | "event" | "email"): Promise<void> {
    this.refunded.push([account, kind]);
    return Promise.resolve();
  }
  tripped(): boolean {
    return this.open;
  }
  withinBudget(): Promise<boolean> {
    return Promise.resolve(this.budget);
  }
  recordTokens(account: string, kind: string, i: number, o: number): Promise<void> {
    this.tokens.push([account, kind, i, o]);
    return Promise.resolve();
  }
}

const NO_RULES: RuleTable = { lookup: () => Promise.resolve(null) };

const ITEM = {
  id: "task-abc123",
  title: "CS 100 HW 01 TRIPWIRE-9f2c",
  body: "Write a program that prints ASCII art. TRIPWIRE-9f2c.",
  source_uid: "zybooks:1839992",
  created_by: "zybooks",
  course: null,
  due: "2026-09-18T23:59",
};
const SEED = {
  course: null,
  effort_hours: null,
  slice_hours: 1.5,
  weights: "Homework 20%",
  preferences: "",
  known_courses: ["cs-100"],
};
const ANSWER = {
  course: "cs-100",
  effort_hours: 2.5,
  importance: 4,
  importance_reason: "homework is 20% of the grade",
  confidence: 0.82,
};

function deps(model: ScriptedModel, log: Sink, caps: Caps, rules: RuleTable = NO_RULES) {
  return { row: ROW, model, rules, caps, log, origin: "device" as const, now: () => 0 };
}

Deno.test("the model answer becomes a tier 3 verdict, one logged row, and a judgment id", async () => {
  const log = new Sink();
  const caps = new Caps();
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([ANSWER]), log, caps),
  );
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.tier, 3);
  assertEquals(reply.verdict?.importance, 4);
  assertEquals(reply.model, "claude-haiku-4-5");
  assertEquals(reply.prompt_version, "task-1");
  // The id is returned so a caller CAN name the judgment — the Gmail queue does, and it is what a
  // human debugging one answer asks for. The device does not store it: `corrections.judgment_id`
  // is back-filled by the eval loader's join (Interfaces with C1, contract 4), because the
  // alternative was changing the bytes of every judged note's `judgment:` line.
  assertEquals(reply.judgment_id, "judgment-1");
  assertEquals(log.rows.length, 1);
  assertEquals(log.rows[0].tier, 3);
  assertEquals(log.rows[0].model, "claude-haiku-4-5");
  assert(log.rows[0].prompt_hash !== null && log.rows[0].prompt_hash.length === 64);
  // The tokens are recorded, or the monthly budget is a view over zeroes.
  assertEquals(caps.tokens, [["acct-1", "task", 300, 60]]);
});

Deno.test("a body token reaches no judgment row, and neither does a title beyond its prefix", async () => {
  // The server-side twin of `enrich.rs`'s `the_judgment_log_never_carries_a_notes_title_or_body_text`
  // (ruling R-3a-21). §5.2: "the log holds ids, field values, confidences and the five promotion
  // features — never the body." The tripwire is in the FOURTH word of the title on purpose: the
  // three-word `title_prefix` is a promotion key and does travel, and this proves the cut is where
  // it is claimed to be.
  const log = new Sink();
  await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([ANSWER]), log, new Caps()),
  );
  const rendered = JSON.stringify(log.rows);
  assert(!rendered.includes("TRIPWIRE-9f2c"), "the note's body or full title reached a judgment row");
  assert(rendered.includes("CS 100 HW"), "the three-word title prefix is a promotion key and must travel");
  assertEquals(log.rows[0].fields.created_by, "zybooks");
  assertEquals(log.rows[0].fields.title_prefix, "CS 100 HW");
});

Deno.test("an email's subject and body reach no judgment row — only its sender does", async () => {
  // C2 final review S-6, the twin of the task tripwire above for the kind that carries a person's
  // MAIL. An email item's every text field is message content: the subject as much as the body, so
  // unlike a task there is no prefix that is allowed to travel. What lands in `fields` is the
  // sender (the one promotion feature an email has, S-2) plus the verdict's own non-free-text
  // values — and `fieldsOf` is called directly as well as through `judge`, because the row the
  // pipeline writes and the map the nightly promotion job reads are the same object.
  const item = {
    message_id: "gmail:19c2f",
    from: "registrar@example.edu",
    subject: "Re: your schedule TRIPWIRE-7a31",
    date: "Mon, 14 Sep 2026 09:00:00 -0500",
    text: "Please confirm by Friday. TRIPWIRE-7a31",
  };
  const verdict = {
    tier: "task",
    title: "Confirm schedule TRIPWIRE-7a31",
    course: null,
    due: "2026-09-18",
    effort_hours: 0.5,
    importance: 3,
    why: "the message asks for a reply by Friday TRIPWIRE-7a31",
    confidence: 0.9,
  };
  const fields = fieldsOf(verdict, "email", item);
  assertEquals(JSON.stringify(fields).includes("TRIPWIRE-7a31"), false, JSON.stringify(fields));
  assertEquals(fields.source, "registrar@example.edu");
  assertEquals(fields.tier, "task");
  assertEquals(Object.keys(fields).sort(), ["course", "due", "effort_hours", "importance", "source", "tier"]);

  // And through the pipeline, which is what actually writes the row.
  const log = new Sink();
  await judge("acct-1", { kind: "email", item, heuristics_seed: { known_courses: ["cs-100"] } }, {
    ...deps(new ScriptedModel([verdict]), log, new Caps()),
    row: { ...ROW, kind: "email", prompt_version: "email-1", grammar_version: "email-1" },
  });
  assertEquals(JSON.stringify(log.rows).includes("TRIPWIRE-7a31"), false, JSON.stringify(log.rows));
  assertEquals(log.rows[0].fields.source, "registrar@example.edu");
});

// T4: the model now answers `due` with the phrase as written, and the pipeline — not the model —
// resolves it against the email's own Date line before `validate` ever sees it. This is the one
// test that exercises the wiring in `judge_pipeline.ts` itself (`judge_due_test.ts` covers the
// resolver's own logic exhaustively); RED before `judge_pipeline.ts` called `resolveDue` was: the
// verdict's `due` came back as the literal string "Friday", which is not `ABSOLUTE_DUE_RE`-shaped,
// so `validate` would have dropped it to `null` — the wiring is what turns a correct extraction
// into a correct verdict instead of a silently discarded one.
Deno.test("a relative due phrase from the model is resolved against the email's Date line before validate sees it", async () => {
  const item = {
    message_id: "gmail:9f31c",
    from: "registrar@example.edu",
    subject: "Re: your schedule",
    date: "Mon, 14 Sep 2026 09:00:00 -0500", // a Monday; "Friday" is 2026-09-18.
    text: "Please confirm by Friday.",
  };
  const verdict = {
    tier: "task",
    title: "Confirm schedule",
    course: null,
    due: "Friday", // the model's extraction, not a resolved date.
    effort_hours: 0.5,
    importance: 3,
    why: "the message asks for a reply by Friday",
    confidence: 0.9,
  };
  const log = new Sink();
  const reply = await judge("acct-1", { kind: "email", item, heuristics_seed: { known_courses: ["cs-100"] } }, {
    ...deps(new ScriptedModel([verdict]), log, new Caps()),
    row: { ...ROW, kind: "email", prompt_version: "email-3", grammar_version: "email-1" },
  });
  assertEquals(reply.verdict?.due, "2026-09-18");
  assertEquals(log.rows[0].fields.due, "2026-09-18");
});

// T4, the failure side of the same wiring: a phrase the resolver cannot place on one calendar day
// must reach `validate` as `null`, not as the literal phrase (which would otherwise fail
// `ABSOLUTE_DUE_RE` and still end up `null` today — but only by accident of that regex, not by the
// resolver's own design; this pins the intended path, not just the accidental outcome).
Deno.test("an unresolvable due phrase from the model becomes null, never a guess, before validate sees it", async () => {
  const item = {
    message_id: "gmail:1a2b3",
    from: "registrar@example.edu",
    subject: "Reminder",
    date: "Mon, 14 Sep 2026 09:00:00 -0500",
    text: "Please respond soon.",
  };
  const verdict = {
    tier: "borderline",
    title: "Respond to registrar",
    course: null,
    due: "soon",
    effort_hours: null,
    importance: null,
    why: "vague timing, needs a human",
    confidence: 0.9,
  };
  const log = new Sink();
  const reply = await judge("acct-1", { kind: "email", item, heuristics_seed: {} }, {
    ...deps(new ScriptedModel([verdict]), log, new Caps()),
    row: { ...ROW, kind: "email", prompt_version: "email-3", grammar_version: "email-1" },
  });
  assertEquals(reply.verdict?.due, null);
});

Deno.test("fieldsOf merges the verdict first and the feature map last", () => {
  // C2 final review S-6: a promotion feature can never be overwritten by a verdict field of the
  // same name. `promote_rules` subtracts the feature keys from `fields` to build a rule's verdict,
  // so a feature the verdict had clobbered would be subtracted as if it were still the key it was
  // looked up by — and the promoted rule would never fire.
  const fields = fieldsOf({ source: "the verdict's own value" }, "event", { source: "engage" });
  assertEquals(fields.source, "engage");
});

Deno.test("a rule answers without a model call and without charging the day's cap", async () => {
  // M2: a promoted rule must not spend the account's allowance. "Rules retire model calls" and
  // "rules still consume the model budget" cannot both be true, and the first one is the design.
  const model = new ScriptedModel([]);
  const log = new Sink();
  const caps = new Caps();
  const rules: RuleTable = {
    lookup: () =>
      Promise.resolve({
        course: "cs-100",
        effort_hours: 2,
        importance: 2,
        importance_reason: "promoted rule",
        confidence: 1,
      }),
  };
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, log, caps, rules),
  );
  assertEquals(reply.tier, 2);
  assertEquals(reply.outcome, "answered");
  assertEquals(model.seen.length, 0, "tier 2 answered, so tier 3 must never be reached");
  assertEquals(caps.charged, [], "a rule costs nothing, so it charges nothing");
  assertEquals(log.rows[0].model, null);
});

Deno.test("a model that throws is a low-confidence outcome, never a write", async () => {
  const log = new Sink();
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([new Error("upstream 529")]), log, new Caps()),
  );
  assertEquals(reply.outcome, "low confidence");
  assertEquals(reply.cause, "model failed");
  assertEquals(reply.verdict, null);
  assertEquals(log.rows[0].outcome, "low confidence");
  assertEquals(log.rows[0].cause, "model failed");
});

// F1 (live proof 2026-10-02): a dead provider key failed all 160 event judgments of a first day,
// every one was charged, and the account was `capped` for the rest of the day. A call the model
// never answered is refunded; a call it did answer (a verdict, a refusal, a truncation) is not.
for (
  const [label, failure] of [
    ["an auth error", new Error("the model service answered HTTP 401")],
    ["a transport error", new TypeError("error sending request: connection refused")],
    ["a timeout", new Error("the model call timed out after 120000 ms")],
    ["a 2xx error envelope", new Error("the model service answered a 2xx with an error envelope")],
    ["an unparseable reply", new Error("the model's reply did not parse as JSON (12 chars)")],
  ] as const
) {
  Deno.test(`F1: ${label} is refunded, so a failed call leaves the day's cap untouched`, async () => {
    const caps = new Caps();
    const reply = await judge(
      "acct-1",
      { kind: "event", item: ITEM, heuristics_seed: SEED },
      deps(new ScriptedModel([failure]), new Sink(), caps),
    );
    assertEquals(reply.cause, "model failed");
    assertEquals(caps.charged, [["acct-1", "event"]], "still charged BEFORE the call: the hot-loop guard");
    assertEquals(caps.refunded, [["acct-1", "event"]], "and given back once the call failed");
    assertEquals(caps.tokens, [], "no reply, no tokens: the monthly ceiling sees nothing");
  });
}

Deno.test("F1: a refusal or a truncation reached the model, so it stays charged", async () => {
  for (const failure of [new ModelRefused("the model declined"), new Error("the reply hit max_tokens (256)")]) {
    const caps = new Caps();
    const reply = await judge(
      "acct-1",
      { kind: "task", item: ITEM, heuristics_seed: SEED },
      deps(new ScriptedModel([failure]), new Sink(), caps),
    );
    assert(reply.cause === "refused" || reply.cause === "truncated", `got ${reply.cause}`);
    assertEquals(caps.charged.length, 1);
    assertEquals(caps.refunded, []);
  }
});

Deno.test("F1: a success still charges and is never refunded", async () => {
  const caps = new Caps();
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([ANSWER]), new Sink(), caps),
  );
  assertEquals(reply.outcome, "answered");
  assertEquals(caps.charged, [["acct-1", "task"]]);
  assertEquals(caps.refunded, []);
});

Deno.test("F1: an answer the model gave that fails validation stays charged", async () => {
  const caps = new Caps();
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([{ ...ANSWER, confidence: 0.1 }]), new Sink(), caps),
  );
  assertEquals(reply.outcome, "low confidence");
  assertEquals(caps.charged.length, 1);
  assertEquals(caps.refunded, [], "the model answered and billed; only an unanswered call is refunded");
});

Deno.test("F1: concurrent failures each charge once and refund once, never twice", async () => {
  const caps = new Caps();
  const failures = Array.from({ length: 8 }, () => new Error("the model service answered HTTP 401"));
  const model = new ScriptedModel(failures);
  const replies = await Promise.all(
    failures.map(() =>
      judge("acct-1", { kind: "event", item: ITEM, heuristics_seed: SEED }, deps(model, new Sink(), caps))
    ),
  );
  assert(replies.every((r) => r.cause === "model failed"));
  assertEquals(caps.charged.length, 8);
  assertEquals(caps.refunded.length, 8);
});

Deno.test("F1: an open breaker answers model failed without charging and without calling the model", async () => {
  const caps = new Caps(true, true, true);
  const model = new ScriptedModel([ANSWER]);
  const log = new Sink();
  const reply = await judge("acct-1", { kind: "email", item: ITEM, heuristics_seed: SEED }, deps(model, log, caps));
  assertEquals(reply.verdict, null);
  assertEquals(reply.outcome, "low confidence");
  assertEquals(reply.cause, "model failed");
  assertEquals(reply.tier, 0, "tier 0: nothing was asked of the model");
  assertEquals(model.seen.length, 0);
  assertEquals(caps.charged, []);
  assertEquals(caps.refunded, []);
  assertEquals(log.rows.length, 1);
  assertEquals(log.rows[0].tier, 0);
  assertEquals(log.rows[0].cause, "model failed");
});

Deno.test("the daily cap refuses before the model is reached and says so", async () => {
  const model = new ScriptedModel([]);
  const log = new Sink();
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, log, new Caps(false, true)),
  );
  assertEquals(reply.outcome, "capped");
  assertEquals(model.seen.length, 0);
  assertEquals(log.rows[0].outcome, "capped");
  assertEquals(DAILY_CAP.task, 60);
  assertEquals(DAILY_CAP.event, 80);
  assertEquals(DAILY_CAP.email, 120);
});

Deno.test("the monthly budget refuses too, and before the day's cap is charged", async () => {
  const model = new ScriptedModel([]);
  const caps = new Caps(true, false);
  const reply = await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, new Sink(), caps),
  );
  assertEquals(reply.outcome, "capped");
  assertEquals(model.seen.length, 0);
  assertEquals(caps.charged, [], "no call was made, so nothing is charged");
});

Deno.test("the prompt carries the note and its grounding and nothing else", async () => {
  // The server-side twin of `judge.rs`'s test of the same name (plan 3a fidelity row R6).
  const model = new ScriptedModel([ANSWER]);
  await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, new Sink(), new Caps()),
  );
  const sent = model.seen[0].system + "\n" + model.seen[0].user;
  for (const forbidden of ["ingest.yaml", "credential", "acct-1", "C:\\", "http"]) {
    assert(!sent.includes(forbidden), `the prompt carried '${forbidden}'`);
  }
  assert(sent.includes("CS 100 HW 01"));
  assert(sent.includes("Homework 20%"));
  assert(sent.length < 6000, "the prompt is bounded however long the note is");
  // The pinned row's sampling reaches the client; the client decides nothing.
  assertEquals(model.seen[0].sampling, { temperature: 0 });
  assertEquals(model.seen[0].maxTokens, 256);
});

Deno.test("the prompt hash is stable across accounts with different planner slices", async () => {
  // M1: it hashes the TEMPLATE and the SCHEMA, and `slice_hours` is a per-account value that used
  // to be rendered into the hashed text — two accounts then sent different system prompts under
  // one hash, which is the one thing the hash exists to prevent.
  const a = new Sink(), b = new Sink();
  await judge(
    "acct-1",
    { kind: "task", item: ITEM, heuristics_seed: { ...SEED, slice_hours: 1.5 } },
    deps(new ScriptedModel([ANSWER]), a, new Caps()),
  );
  await judge(
    "acct-2",
    { kind: "task", item: ITEM, heuristics_seed: { ...SEED, slice_hours: 3 } },
    deps(new ScriptedModel([ANSWER]), b, new Caps()),
  );
  assertEquals(a.rows[0].prompt_hash, b.rows[0].prompt_hash);
});

// Final review item 2: the capability gate for `unsure`. A pre-T1 engine rejects the fourth word
// (its VALID_VERDICTS has three) and re-asks — and pays — every slot, so a request that does not
// declare `accepts: ["unsure"]` gets exactly the pre-T1 shape instead: no verdict, `below floor`.
const EVENT_ITEM = { uid: "engage:1", title: "AI Club Kickoff", start: "2026-08-29T18:00", end: "2026-08-29T19:30" };
const UNSURE = { verdict: "unsure", why: "the text does not say who it is for", confidence: 0.3 };

Deno.test("an event request that does not declare accepts unsure never sees unsure", async () => {
  for (const accepts of [undefined, [], ["completion"]]) {
    const log = new Sink();
    const reply = await judge(
      "acct-1",
      { kind: "event", item: EVENT_ITEM, heuristics_seed: {}, ...(accepts === undefined ? {} : { accepts }) },
      deps(new ScriptedModel([UNSURE]), log, new Caps()),
    );
    assertEquals(reply.verdict, null, `accepts ${JSON.stringify(accepts)}`);
    assertEquals(reply.outcome, "low confidence");
    assertEquals(reply.cause, "below floor");
    assertEquals(reply.tier, 3);
    assertEquals(reply.judgment_id, "judgment-1");
  }
});

Deno.test("an event request that declares accepts unsure gets unsure as a verdict", async () => {
  const reply = await judge(
    "acct-1",
    { kind: "event", item: EVENT_ITEM, heuristics_seed: {}, accepts: ["unsure"] },
    deps(new ScriptedModel([UNSURE]), new Sink(), new Caps()),
  );
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.verdict?.verdict, "unsure");
});

Deno.test("an undeclared event request still gets the three old words untouched", async () => {
  const reply = await judge(
    "acct-1",
    { kind: "event", item: EVENT_ITEM, heuristics_seed: {} },
    deps(new ScriptedModel([{ verdict: "drop", why: "a club social", confidence: 0.9 }]), new Sink(), new Caps()),
  );
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.verdict?.verdict, "drop");
});

// Due-fix (2026-09-23): the request's `timezone` (the vault's own, sent by the device) reaches the
// resolver, so a relative phrase resolves against the email's LOCAL date. The Date header here is
// stamped in UTC, as server-sent mail often is: 04:30 UTC Friday is 23:30 CDT Thursday, so the
// student's "tomorrow" is Friday the 18th -- the UTC calendar would have said the 19th.
Deno.test("the request's timezone reaches the due resolver, so 'tomorrow' is the student's next local day", async () => {
  const item = {
    message_id: "gmail:7e0d1",
    from: "instructor@example.edu",
    subject: "Lab writeup",
    date: "Fri, 18 Sep 2026 04:30:00 +0000",
    text: "Submit the writeup by tomorrow.",
  };
  const verdict = {
    tier: "task",
    title: "Submit lab writeup",
    course: null,
    due: "tomorrow",
    effort_hours: 1,
    importance: 3,
    why: "the message sets a deadline",
    confidence: 0.9,
  };
  const row = { ...ROW, kind: "email" as const, prompt_version: "email-3", grammar_version: "email-1" };
  const zoned = await judge("acct-1", { kind: "email", item, heuristics_seed: {}, timezone: "America/Chicago" }, {
    ...deps(new ScriptedModel([verdict]), new Sink(), new Caps()),
    row,
  });
  assertEquals(zoned.verdict?.due, "2026-09-18");
  // No timezone on the request: the Date header's own offset (UTC here) is the only clock left.
  const bare = await judge("acct-1", { kind: "email", item, heuristics_seed: {} }, {
    ...deps(new ScriptedModel([verdict]), new Sink(), new Caps()),
    row,
  });
  assertEquals(bare.verdict?.due, "2026-09-19");
});

// L3 (spec 2026-10-06 §3, §5; plan T2b): the first-day event allowance. The daily cap is charged
// first; only a real refusal of an `event` call asks the allowance, and a failed call is refunded to
// whichever store charged it. `Caps(allow, budget, open, capError, allowanceAllow)`.
const DROP = { verdict: "drop", why: "a club social", confidence: 0.9 };
const EMAIL_ITEM = {
  message_id: "gmail:4c11d",
  from: "registrar@example.edu",
  subject: "Reminder",
  date: "Mon, 14 Sep 2026 09:00:00 -0500",
  text: "Please respond.",
};

Deno.test("an_event_the_cap_refuses_is_charged_to_the_allowance_and_judged", async () => {
  const caps = new Caps(false, true, false, false, true);
  const model = new ScriptedModel([DROP]);
  const reply = await judge("acct-1", { kind: "event", item: EVENT_ITEM, heuristics_seed: {} }, deps(model, new Sink(), caps));
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.tier, 3);
  assertEquals(model.seen.length, 1);
  assertEquals(caps.charged, [["acct-1", "event"]]);
  assertEquals(caps.allowanceCharged, [["acct-1", "event"]]);
});

Deno.test("a_task_or_email_the_cap_refuses_is_capped_and_never_asks_the_allowance", async () => {
  for (const req of [
    { kind: "task" as const, item: ITEM, heuristics_seed: SEED },
    { kind: "email" as const, item: EMAIL_ITEM, heuristics_seed: {} },
  ]) {
    const caps = new Caps(false, true, false, false, true);
    const model = new ScriptedModel([ANSWER]);
    const reply = await judge("acct-1", req, deps(model, new Sink(), caps));
    assertEquals(reply.outcome, "capped", req.kind);
    assertEquals(reply.tier, 0);
    assertEquals(caps.allowanceCharged, [], `${req.kind} never reaches the allowance`);
    assertEquals(model.seen.length, 0);
  }
});

Deno.test("an_event_both_refuse_is_capped_as_today", async () => {
  const caps = new Caps(false, true, false, false, false);
  const log = new Sink();
  const model = new ScriptedModel([DROP]);
  const reply = await judge("acct-1", { kind: "event", item: EVENT_ITEM, heuristics_seed: {} }, deps(model, log, caps));
  assertEquals(reply, { verdict: null, tier: 0, outcome: "capped" });
  assertEquals(caps.allowanceCharged, [["acct-1", "event"]], "the refused event asked the allowance");
  assertEquals(model.seen.length, 0);
  assertEquals(log.rows.length, 1);
  assertEquals(log.rows[0].outcome, "capped");
  assertEquals(log.rows[0].tier, 0);
});

Deno.test("an_event_whose_cap_charge_errors_is_capped_and_never_asks_the_allowance", async () => {
  // Green before the pipeline knew the allowance, and must stay green after: a cap store that could
  // not answer has not refused, so it neither opens nor spends the one-time allowance (Q2).
  const caps = new Caps(false, true, false, true, true);
  const log = new Sink();
  const model = new ScriptedModel([DROP]);
  const reply = await judge("acct-1", { kind: "event", item: EVENT_ITEM, heuristics_seed: {} }, deps(model, log, caps));
  assertEquals(reply, { verdict: null, tier: 0, outcome: "capped" });
  assertEquals(model.seen.length, 0);
  assertEquals(caps.allowanceCharged, []);
  // Logged exactly as a refused charge logs (the both-refuse case's row).
  const refusedLog = new Sink();
  await judge(
    "acct-1",
    { kind: "event", item: EVENT_ITEM, heuristics_seed: {} },
    deps(new ScriptedModel([]), refusedLog, new Caps(false, true, false, false, false)),
  );
  assertEquals(log.rows, refusedLog.rows);
});

import { capStore } from "./judge_caps.ts";
import type { Db } from "./judge_db.ts";

/** The live `capStore`'s database, scripted per rpc name: a value to resolve, or a rejection. */
class RpcDb implements Db {
  calls: string[] = [];
  constructor(private readonly script: Record<string, { value: unknown } | { reject: string }>) {}
  select(): Promise<unknown[]> {
    return Promise.resolve([]);
  }
  insert(): Promise<Record<string, unknown> | null> {
    return Promise.resolve(null);
  }
  update(): Promise<void> {
    return Promise.resolve();
  }
  rpc(fn: string): Promise<unknown> {
    this.calls.push(fn);
    const answer = this.script[fn];
    if (answer === undefined) return Promise.reject(new Error(`unscripted rpc ${fn}`));
    return "reject" in answer ? Promise.reject(new Error(answer.reject)) : Promise.resolve(answer.value);
  }
}

Deno.test("a_charge_call_error_on_the_live_store_never_opens_the_allowance", async () => {
  const live = (chargeCall: { value: unknown } | { reject: string }) =>
    new RpcDb({
      enforce_budget: { value: true },
      charge_call: chargeCall,
      charge_judge_allowance: { value: true },
      record_tokens: { value: null },
      refund_call: { value: null },
    });
  const req = { kind: "event" as const, item: EVENT_ITEM, heuristics_seed: {} };

  // An error from `charge_call` is not a refusal: capped, and the allowance is never asked.
  const erroring = live({ reject: "PostgREST 503" });
  const model = new ScriptedModel([DROP]);
  const capped = await judge("acct-1", req, { ...deps(model, new Sink(), new Caps()), caps: capStore(erroring) });
  assertEquals(capped.outcome, "capped");
  assertEquals(model.seen.length, 0);
  assert(erroring.calls.includes("charge_call"));
  assertEquals(erroring.calls.includes("charge_judge_allowance"), false, erroring.calls.join(","));

  // A real `false` is a refusal: the allowance is asked, and the event is judged.
  const refusing = live({ value: false });
  const judged = await judge("acct-1", req, {
    ...deps(new ScriptedModel([DROP]), new Sink(), new Caps()),
    caps: capStore(refusing),
  });
  assertEquals(judged.outcome, "answered");
  assertEquals(refusing.calls, ["enforce_budget", "charge_call", "charge_judge_allowance", "record_tokens"]);
});

Deno.test("over_the_monthly_ceiling_neither_charge_is_asked", async () => {
  const caps = new Caps(false, false, false, false, true);
  const reply = await judge(
    "acct-1",
    { kind: "event", item: EVENT_ITEM, heuristics_seed: {} },
    deps(new ScriptedModel([DROP]), new Sink(), caps),
  );
  assertEquals(reply.outcome, "capped");
  assertEquals(caps.charged, []);
  assertEquals(caps.allowanceCharged, []);
});

Deno.test("a_failed_call_on_the_allowance_is_refunded_to_the_allowance", async () => {
  const caps = new Caps(false, true, false, false, true);
  const reply = await judge(
    "acct-1",
    { kind: "event", item: EVENT_ITEM, heuristics_seed: {} },
    deps(new ScriptedModel([new TypeError("error sending request: connection refused")]), new Sink(), caps),
  );
  assertEquals(reply.cause, "model failed");
  assertEquals(caps.allowanceRefunded, [["acct-1", "event"]]);
  assertEquals(caps.refunded, [], "the cap never charged this call, so it is never refunded");
});

Deno.test("a_failed_call_on_the_cap_is_refunded_to_the_cap", async () => {
  const caps = new Caps();
  const reply = await judge(
    "acct-1",
    { kind: "event", item: EVENT_ITEM, heuristics_seed: {} },
    deps(new ScriptedModel([new TypeError("error sending request: connection refused")]), new Sink(), caps),
  );
  assertEquals(reply.cause, "model failed");
  assertEquals(caps.refunded, [["acct-1", "event"]]);
  assertEquals(caps.allowanceRefunded, []);
});

Deno.test("a_refusal_truncation_or_invalid_answer_on_the_allowance_refunds_nothing", async () => {
  for (
    const [want, reply] of [
      ["refused", new ModelRefused("the model declined")],
      ["truncated", new Error("the reply hit max_tokens (256)")],
      ["incomplete", { verdict: "maybe", why: "not a verdict word", confidence: 0.9 }],
    ] as const
  ) {
    const caps = new Caps(false, true, false, false, true);
    const got = await judge(
      "acct-1",
      { kind: "event", item: EVENT_ITEM, heuristics_seed: {} },
      deps(new ScriptedModel([reply]), new Sink(), caps),
    );
    assertEquals(got.tier, 3, `${want}: the allowance charged, so the model was reached`);
    assertEquals(got.cause, want);
    assertEquals(caps.allowanceCharged, [["acct-1", "event"]]);
    assertEquals(caps.allowanceRefunded, [], want);
    assertEquals(caps.refunded, [], want);
  }
});

Deno.test("the_breaker_skips_both_charges", async () => {
  const caps = new Caps(false, true, true, false, true);
  const model = new ScriptedModel([DROP]);
  const reply = await judge("acct-1", { kind: "event", item: EVENT_ITEM, heuristics_seed: {} }, deps(model, new Sink(), caps));
  assertEquals(reply.cause, "model failed");
  assertEquals(reply.tier, 0);
  assertEquals(model.seen.length, 0);
  assertEquals(caps.charged, []);
  assertEquals(caps.allowanceCharged, []);
});
