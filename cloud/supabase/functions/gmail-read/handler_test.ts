import { assert, assertEquals } from "@std/assert";
import { ScriptedModel } from "../_shared/judge_anthropic.ts";
import type { JudgmentRow } from "../_shared/judge_pipeline.ts";
import { type GmailApi, READ_CAP, readHandler, type ReadDeps } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const TRIPWIRE = "TRIPWIRE-9f2c";

const TASK_ANSWER = {
  tier: "task", title: "PH 106 problem set 4", course: "ph-106", due: "2026-09-11",
  effort_hours: 2.5, importance: 4, why: "the email states a Friday deadline", confidence: 0.86,
};

function fakes(replies: Array<Record<string, unknown>>, ids = ["m1"]) {
  const rows: JudgmentRow[] = [];
  const queued: Array<{ uid: string; tier: string; payload: Record<string, unknown> }> = [];
  const delivered: string[] = [];
  const seen = new Set<string>();
  const asked: string[] = [];
  let attachments = 0;
  const api: GmailApi = {
    list: (_t, q) => {
      asked.push(q);
      return Promise.resolve(ids);
    },
    message: (_t, id) => {
      if (id === "attachment") attachments += 1;
      return Promise.resolve({
        subject: "PH 106 problem set 4 is posted",
        from: "noreply@lms.example.invalid",
        date: "Wed, 09 Sep 2026 08:00:00 -0500",
        text: `Problem set 4 is due Friday. ${TRIPWIRE}`,
      });
    },
  };
  const deps: ReadDeps = {
    api,
    accessTokenFor: () => Promise.resolve("access-token-not-a-secret"),
    excludedLabels: () => Promise.resolve([]),
    markRevoked: () => Promise.resolve(),
    seen: () => Promise.resolve(seen),
    markSeen: (_a, uid) => {
      seen.add(uid);
      return Promise.resolve();
    },
    enqueue: (_a, uid, tier, payload) => {
      queued.push({ uid, tier, payload });
      return Promise.resolve();
    },
    undelivered: () => Promise.resolve(queued.filter((q) => !delivered.includes(q.uid))),
    deliver: (_a, uids) => {
      delivered.push(...uids);
      return Promise.resolve();
    },
    knownCourses: () => Promise.resolve(["ph-106"]),
    pipeline: () =>
      Promise.resolve({
        row: {
          kind: "email", provider: "anthropic", model_id: "claude-haiku-4-5",
          prompt_version: "email-1", grammar_version: "email-1", max_tokens: 640,
          sampling: { temperature: 0 }, usd_per_m_in: 1.0, usd_per_m_out: 5.0,
        },
        model: new ScriptedModel(replies),
        rules: { lookup: () => Promise.resolve(null) },
        caps: {
          charge: () => Promise.resolve(true),
          withinBudget: () => Promise.resolve(true),
          recordTokens: () => Promise.resolve(),
        },
        log: {
          write: (row: JudgmentRow) => {
            rows.push(row);
            return Promise.resolve(`judgment-${rows.length}`);
          },
        },
        origin: "gmail_api",
        now: () => 0,
      }),
    budgetMs: 60_000,
    clock: () => 0,
  };
  return { deps, rows, queued, seen, asked, attachmentsRead: () => attachments };
}

function post(body: unknown = {}): Request {
  return new Request("http://127.0.0.1/gmail-read", { method: "POST", body: JSON.stringify(body) });
}

Deno.test("the message text reaches no queue row and no judgment row", async () => {
  const { deps, rows, queued } = fakes([TASK_ANSWER]);
  await readHandler(OK, deps)(post());
  assertEquals(queued.length, 1);
  assert(!JSON.stringify(queued).includes(TRIPWIRE), "the message body reached the queue");
  assert(!JSON.stringify(rows).includes(TRIPWIRE), "the message body reached a judgment row");
  assert(!JSON.stringify(rows).includes("PH 106 problem set 4 is posted"), "the subject reached a judgment row");
});

Deno.test("every derived row is flagged gmail_api, which is what the export filter keys on", async () => {
  const { deps, rows } = fakes([TASK_ANSWER]);
  await readHandler(OK, deps)(post());
  assertEquals(rows.length, 1);
  assertEquals(rows[0].origin, "gmail_api");
  assertEquals(rows[0].item_id, "gmail:m1");
});

Deno.test("each of the five tiers becomes a queue row with that tier", async () => {
  for (const tier of ["task", "borderline", "event", "opportunity", "information"]) {
    const { deps, queued } = fakes([{ ...TASK_ANSWER, tier }]);
    await readHandler(OK, deps)(post());
    assertEquals(queued[0].tier, tier);
  }
  // `information` is queued too — the device records the uid and stops asking — but it carries no
  // task fields to write.
  const { deps, queued } = fakes([{
    tier: "information", title: "Weekly newsletter", course: null, due: null,
    effort_hours: null, importance: null, why: "a newsletter", confidence: 0.95,
  }]);
  await readHandler(OK, deps)(post());
  assertEquals(queued[0].payload.effort_hours, null);
});

Deno.test("a uid already seen is neither fetched nor judged", async () => {
  const { deps, rows, seen } = fakes([]);
  seen.add("gmail:m1");
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(reply.read, 0);
  assertEquals(rows.length, 0);
});

Deno.test("an acknowledged row is delivered and never returned twice", async () => {
  const { deps } = fakes([TASK_ANSWER]);
  const first = await (await readHandler(OK, deps)(post())).json();
  assertEquals(first.items.length, 1);
  const second = await (await readHandler(OK, deps)(post({ ack: ["gmail:m1"] }))).json();
  assertEquals(second.items.length, 0);
});

Deno.test("attachments are never fetched, because there is no code path that could", async () => {
  const { deps, attachmentsRead } = fakes([TASK_ANSWER]);
  await readHandler(OK, deps)(post());
  assertEquals(attachmentsRead(), 0);
  // And structurally: the handler's whole Gmail surface is `list` and `message`. A third method
  // would have to be added to `GmailApi` before an attachment could be reached.
  const source = await Deno.readTextFile(new URL("./handler.ts", import.meta.url));
  assertEquals(source.includes("attachment"), false, "cloud design §5.3: attachments are never fetched");
});

Deno.test("the read stops at its wall-clock budget and says there is more", async () => {
  // An edge function's wall clock is far under sixty model calls at 120 s each. The dedup set is
  // the resume cursor: everything judged is marked seen, so the next round starts where this one
  // stopped and nothing is judged twice.
  let tick = 0;
  const { deps, queued } = fakes(
    Array.from({ length: 5 }, () => TASK_ANSWER),
    ["m1", "m2", "m3", "m4", "m5"],
  );
  const reply = await (await readHandler(OK, { ...deps, budgetMs: 10, clock: () => (tick += 8) })(post())).json();
  assert(reply.more === true, "the handler must say it stopped early");
  assert(queued.length < 5, "it must actually have stopped early");
});

Deno.test("excluded_labels_become_negative_label_terms (and the window is seven days)", async () => {
  const { deps, asked } = fakes([TASK_ANSWER]);
  await readHandler(OK, { ...deps, excludedLabels: () => Promise.resolve(["Promotions", "Social"]) })(post());
  assertEquals(asked[0], "newer_than:7d -label:Promotions -label:Social");
  assertEquals(READ_CAP, 60);
});

Deno.test("a revoked grant is quiet, not an error, and never a failed slot", async () => {
  const { deps } = fakes([]);
  let revoked = false;
  const response = await readHandler(OK, {
    ...deps,
    accessTokenFor: () => Promise.resolve(null),
    markRevoked: () => {
      revoked = true;
      return Promise.resolve();
    },
  })(post());
  assertEquals(response.status, 200);
  assertEquals((await response.json()).quiet, true);
  assertEquals(revoked, true);
});

Deno.test("gmail_rows_are_excluded_from_the_training_export", async () => {
  // A static test over the function's own text: the filter is one predicate and losing it is
  // silent, so it is pinned where it cannot be lost by an edit that looks like a refactor.
  const sql = await Deno.readTextFile(new URL("../../migrations/20260911000200_google.sql", import.meta.url));
  const body = sql.slice(sql.indexOf("create or replace function export_training_rows"));
  assert(
    body.includes("origin <> 'gmail_api'"),
    "cloud design §5.3 and §9: gmail-derived rows are excluded by the export filter",
  );
});
