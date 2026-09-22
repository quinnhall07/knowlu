import { assert, assertEquals } from "@std/assert";
import { ScriptedModel } from "./judge_anthropic.ts";
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
  tokens: Array<[string, string, number, number]> = [];
  constructor(private readonly allow = true, private readonly budget = true) {}
  charge(account: string, kind: "task" | "event" | "email"): Promise<boolean> {
    this.charged.push([account, kind]);
    return Promise.resolve(this.allow);
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
