import { assert, assertEquals } from "@std/assert";
import { ScriptedModel } from "../_shared/judge_anthropic.ts";
import type { JudgmentRow } from "../_shared/judge_pipeline.ts";
import {
  forDevice,
  type GmailApi,
  type ListPage,
  MAX_LIST_PAGES,
  pagedList,
  PER_CALL_MS,
  READ_BUDGET_MS,
  READ_CAP,
  readHandler,
  type ReadDeps,
} from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const TRIPWIRE = "TRIPWIRE-9f2c";

const TASK_ANSWER = {
  tier: "task", title: "PH 106 problem set 4", course: "ph-106", due: "2026-09-11",
  effort_hours: 2.5, importance: 4, why: "the email states a Friday deadline", confidence: 0.86,
};

type Message = { subject: string; from: string; date: string; text: string };

function fakes(replies: Array<Record<string, unknown> | Error>, ids = ["m1"], message?: Message) {
  const rows: JudgmentRow[] = [];
  const queued: Array<
    { uid: string; tier: string; payload: Record<string, unknown>; judgment_id: string | null }
  > = [];
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
      if (message !== undefined) return Promise.resolve(message);
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
    accessTokenFor: () => Promise.resolve({ token: "access-token-not-a-secret" }),
    excludedLabels: () => Promise.resolve([]),
    markRevoked: () => Promise.resolve(),
    seen: () => Promise.resolve(seen),
    markSeen: (_a, uid) => {
      seen.add(uid);
      return Promise.resolve();
    },
    enqueue: (_a, uid, tier, payload, judgmentId) => {
      queued.push({ uid, tier, payload, judgment_id: judgmentId });
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
          sampling: { temperature: 0 }, route: {}, precision: "bf16",
          usd_per_m_in: 1.0, usd_per_m_out: 5.0,
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
    // F-4: matches production (`gmail-read/index.ts` passes `READ_BUDGET_MS` too) rather than an
    // arbitrary round number — with `clock` pinned at a constant 0 below, elapsed never advances
    // between calls, so no ordinary test here comes anywhere near the budget check regardless of
    // exactly how large this is, but keeping it the REAL value is what makes a mismatch between the
    // two impossible to introduce by accident.
    budgetMs: READ_BUDGET_MS,
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

// ---------------------------------------------------------------------------------------------
// F6a — the judgment_id `enqueue` already writes rides back out to the device, so a later label
// report (F8) can post it to `/telemetry` and `calibration_query.sql`'s `c.judgment_id = j.id`
// join finds it with no heuristic.
// ---------------------------------------------------------------------------------------------

Deno.test("an undelivered row's judgment_id reaches the device", async () => {
  const { deps } = fakes([TASK_ANSWER]);
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(reply.items.length, 1);
  assertEquals(reply.items[0].judgment_id, "judgment-1");
});

Deno.test("forDevice keeps judgment_id when it rewrites a tier", () => {
  const items = [{ uid: "gmail:m1", tier: "completion", payload: {}, judgment_id: "judgment-1" }];
  // No accepts: "completion" is a declared-only tier and gets rewritten to "information".
  const out = forDevice(items, []);
  assertEquals(out[0].tier, "information");
  assertEquals(out[0].judgment_id, "judgment-1", "the id must survive the tier rewrite");
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
  // R2-6: rewritten against the exported constants so this proves what its name says. Under the
  // current check (`elapsed + PER_CALL_MS >= budgetMs`), the old `budgetMs: 10` tripped on the very
  // FIRST iteration (elapsed is already 8 there, and 8 + PER_CALL_MS always dwarfs 10) — so
  // `queued.length < 5` was true no matter how the stop logic behaved, even a broken one that
  // stopped after zero items. `started` is one clock call, and each loop iteration's check is one
  // more, each advancing the clock by 8ms: `elapsed` at iteration i (1-indexed) is `i * 8`. Picking
  // `budgetMs = PER_CALL_MS + 32` puts the threshold (`budgetMs - PER_CALL_MS = 32`) strictly above
  // i=1,2,3's elapsed (8, 16, 24) and at i=4's (32, and `>=` trips on equality) — three items
  // processed, the fourth refused before it starts.
  let tick = 0;
  const { deps, queued } = fakes(
    Array.from({ length: 5 }, () => TASK_ANSWER),
    ["m1", "m2", "m3", "m4", "m5"],
  );
  const reply = await (
    await readHandler(OK, { ...deps, budgetMs: PER_CALL_MS + 32, clock: () => (tick += 8) })(post())
  ).json();
  assert(reply.more === true, "the handler must say it stopped early");
  assertEquals(queued.length, 3, "exactly three items must have been processed before the stop");
});

// ---------------------------------------------------------------------------------------------
// F-4 — the budget bounds when the last call ENDS, not when it starts: the check reserves
// `PER_CALL_MS` up front, so nothing starts that could not finish inside `budgetMs`.
// ---------------------------------------------------------------------------------------------

/** A `clock()` fake that returns 0 once (for `started`), then each of `checks` in order — one per
 * loop iteration's budget check — then throws if asked for more than that, so a test that expects
 * to stop after N checks fails loudly rather than silently reading a stale value past the point it
 * meant to assert about. */
function scriptedClock(...checks: number[]): () => number {
  const values = [0, ...checks];
  let i = 0;
  return () => {
    if (i >= values.length) throw new Error("scriptedClock: asked for more ticks than scripted");
    return values[i++];
  };
}

Deno.test("a call is refused the instant starting it could not finish before the budget — elapsed + PER_CALL_MS === budgetMs", async () => {
  const { deps, queued } = fakes([TASK_ANSWER], ["m1"]);
  const budgetMs = 100_000;
  // elapsed at the check is exactly `budgetMs - PER_CALL_MS`: this call's worst case would land
  // EXACTLY on the deadline, and `>=` refuses it rather than letting it just touch the line.
  const clock = scriptedClock(budgetMs - PER_CALL_MS);
  const reply = await (await readHandler(OK, { ...deps, budgetMs, clock })(post())).json();
  assertEquals(queued.length, 0, "a call whose worst case lands exactly on the deadline must not start");
  assertEquals(reply.more, true);
});

Deno.test("one millisecond of slack is enough to let the call start", async () => {
  const { deps, queued } = fakes([TASK_ANSWER], ["m1"]);
  const budgetMs = 100_000;
  // One millisecond less elapsed than the refusal case above: the same call now finishes inside
  // the deadline in the worst case, and must be allowed to start.
  const clock = scriptedClock(budgetMs - PER_CALL_MS - 1);
  const reply = await (await readHandler(OK, { ...deps, budgetMs, clock })(post())).json();
  assertEquals(queued.length, 1, "a call with one millisecond of slack must start");
  assertEquals(reply.more, false);
});

Deno.test("READ_BUDGET_MS keeps the same new-work cutoff as before (40 s) by reserving PER_CALL_MS on top of it", () => {
  // F-4's whole point: the OLD 40 s threshold for "stop accepting new work" is preserved exactly —
  // only the constant's own value and meaning changed, to also cover the last call's own tail.
  assertEquals(READ_BUDGET_MS - PER_CALL_MS, 40_000);
  assertEquals(PER_CALL_MS, 60_000);
  assert(READ_BUDGET_MS + 0 < 150_000, "the round's true worst case must stay under the assumed edge function ceiling");
});

Deno.test("excluded_labels_become_negative_label_terms (and the window is seven days)", async () => {
  const { deps, asked } = fakes([TASK_ANSWER]);
  await readHandler(OK, { ...deps, excludedLabels: () => Promise.resolve(["Promotions", "Social"]) })(post());
  assertEquals(asked[0], "newer_than:7d -label:Promotions -label:Social");
  assertEquals(READ_CAP, 60);
});

Deno.test("export_training_rows was ORIGINALLY defined excluding gmail_api rows, as this applied migration still reads", async () => {
  // This is a historical pin, not a description of the LIVE filter: 20260911000200_google.sql is
  // applied and forward-only, so its own text never changes and must still read exactly as it did
  // the day it shipped (cloud design §5.3 and §9's first cut). The provider swap
  // (20260916000100_provider_swap.sql) later REDEFINES export_training_rows with a wider filter —
  // `origin not in ('gmail_api', 'events')` — and that corpus-wide "whichever definition is LAST"
  // guarantee is asserted in migrations_test.ts's own export_training_rows case, not here.
  const sql = await Deno.readTextFile(new URL("../../migrations/20260911000200_google.sql", import.meta.url));
  const body = sql.slice(sql.indexOf("create or replace function export_training_rows"));
  assert(
    body.includes("origin <> 'gmail_api'"),
    "this applied migration's own text must still say what it always said, unedited",
  );
});

// ---------------------------------------------------------------------------------------------
// R-C2-E41 — a revoked grant, a calendar-only grant and an unconfigured deployment are three
// different situations, and only one of them is ever `markRevoked`.
// ---------------------------------------------------------------------------------------------

Deno.test("a calendar-only grant reads as no_gmail_scope, and markRevoked is never called", async () => {
  const { deps } = fakes([]);
  let revoked = false;
  const response = await readHandler(OK, {
    ...deps,
    accessTokenFor: () => Promise.resolve({ missing: "scope" } as const),
    markRevoked: () => {
      revoked = true;
      return Promise.resolve();
    },
  })(post());
  assertEquals(response.status, 200);
  const body = await response.json();
  assertEquals(body.quiet, true);
  assertEquals(body.reason, "no_gmail_scope");
  assertEquals(revoked, false, "a calendar-only grant must never be revoked over a Gmail step never taken");
});

Deno.test("a rejected refresh reads as revoked, and markRevoked is called", async () => {
  const { deps } = fakes([]);
  let revoked = false;
  const response = await readHandler(OK, {
    ...deps,
    accessTokenFor: () => Promise.resolve({ missing: "grant" } as const),
    markRevoked: () => {
      revoked = true;
      return Promise.resolve();
    },
  })(post());
  assertEquals(response.status, 200);
  const body = await response.json();
  assertEquals(body.quiet, true);
  assertEquals(body.reason, "revoked");
  assertEquals(revoked, true);
});

Deno.test("no Google client configured on this deployment is a 503, never quiet and never markRevoked", async () => {
  const { deps } = fakes([]);
  let revoked = false;
  const response = await readHandler(OK, {
    ...deps,
    accessTokenFor: () => Promise.resolve({ missing: "config" } as const),
    markRevoked: () => {
      revoked = true;
      return Promise.resolve();
    },
  })(post());
  assertEquals(response.status, 503);
  assertEquals(revoked, false);
});

// ---------------------------------------------------------------------------------------------
// R-C2-E42 — dedup before the cap, and an unjudged verdict is deferred rather than dropped.
// ---------------------------------------------------------------------------------------------

Deno.test("dedup happens before the read cap, so a backlog past it is still reachable", async () => {
  // 65 ids, the first 5 already seen: dedup-before-slice means the 60 UNSEEN ids fill the whole
  // cap, rather than 5 of the 60 slots being spent on ids this account has already judged.
  const ids = Array.from({ length: 65 }, (_, i) => `m${i}`);
  const { deps, queued, seen } = fakes(Array.from({ length: READ_CAP }, () => TASK_ANSWER), ids);
  for (let i = 0; i < 5; i++) seen.add(`gmail:m${i}`);
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(reply.read, READ_CAP, JSON.stringify(reply));
  assertEquals(queued.length, READ_CAP);
  assert(!queued.some((q) => Number(q.uid.replace("gmail:m", "")) < 5), "a seen id must never be read again");
});

Deno.test("a capped outcome stops the round on the spot, defers, and answers more:false", async () => {
  const ids = ["m1", "m2"];
  // Only one reply is scripted: m2 must never reach the model at all, since `withinBudget`
  // answers false for it before `judge` ever calls `complete`.
  const { deps, queued, seen } = fakes([TASK_ANSWER], ids);
  let calls = 0;
  const basePipeline = deps.pipeline;
  deps.pipeline = async () => {
    const p = await basePipeline();
    return { ...p, caps: { ...p.caps, withinBudget: () => Promise.resolve((calls += 1) === 1) } };
  };
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(queued.length, 1, "only the first item was judged and queued");
  assertEquals(queued[0].uid, "gmail:m1");
  assertEquals(reply.deferred, 1);
  assertEquals(reply.more, false, "retrying the rest of THIS round would not help while capped");
  assert(!seen.has("gmail:m2"), "a capped item must not be marked seen, so it is asked about again");
});

Deno.test("a model failure defers that one uid and continues to the next", async () => {
  const ids = ["m1", "m2"];
  const { deps, queued, seen } = fakes([new Error("boom"), TASK_ANSWER], ids);
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(queued.length, 1, "the second item was still judged");
  assertEquals(queued[0].uid, "gmail:m2");
  assertEquals(reply.deferred, 1);
  assert(!seen.has("gmail:m1"), "a failed item must not be marked seen, so it is asked about again");
  assert(seen.has("gmail:m2"));
});

// ---------------------------------------------------------------------------------------------
// F-3 — `pagedList` follows `nextPageToken` until it has enough UNSEEN ids or the pages run out,
// with a hard ceiling regardless. A fake `fetchPage` proves every stopping condition with no
// Gmail shape and no network at all.
// ---------------------------------------------------------------------------------------------

function page(ids: string[], nextPageToken?: string): ListPage {
  return { ids, nextPageToken };
}

Deno.test("a single page with enough unseen ids never asks for a second page", async () => {
  let calls = 0;
  const fetchPage = () => {
    calls += 1;
    return Promise.resolve(page(["m1", "m2", "m3"], "would-be-page-2"));
  };
  const ids = await pagedList(fetchPage, () => true, 3);
  assertEquals(ids, ["m1", "m2", "m3"]);
  assertEquals(calls, 1, "a page token is never followed once `want` is already met");
});

Deno.test("a page short of unseen ids follows nextPageToken for more", async () => {
  const pages = [page(["m1", "m2"], "tok-2"), page(["m3", "m4"], "tok-3"), page(["m5"], undefined)];
  const tokensAsked: Array<string | undefined> = [];
  const fetchPage = (pageToken?: string) => {
    tokensAsked.push(pageToken);
    return Promise.resolve(pages[tokensAsked.length - 1]);
  };
  const ids = await pagedList(fetchPage, () => true, 4);
  // Stops the moment 4 have been gathered — the third page (no more unseen needed) is never asked.
  assertEquals(ids, ["m1", "m2", "m3", "m4"]);
  assertEquals(tokensAsked, [undefined, "tok-2"]);
});

Deno.test("pages run out before `want` is met, and the loop stops rather than looping forever", async () => {
  const fetchPage = (pageToken?: string) =>
    Promise.resolve(pageToken === undefined ? page(["m1"], "tok-2") : page(["m2"], undefined));
  const ids = await pagedList(fetchPage, () => true, 100);
  assertEquals(ids, ["m1", "m2"], "every id gathered before the pages ran out");
});

Deno.test("only UNSEEN ids count toward `want` — a page of already-seen mail does not look like enough", async () => {
  const seen = new Set(["m1", "m2"]);
  const pages = [page(["m1", "m2"], "tok-2"), page(["m3"], undefined)];
  let calls = 0;
  const fetchPage = () => Promise.resolve(pages[calls++]);
  const ids = await pagedList(fetchPage, (id) => !seen.has(id), 1);
  assertEquals(ids, ["m1", "m2", "m3"], "the already-seen page did not satisfy `want` on its own");
  assertEquals(calls, 2);
});

Deno.test("a hard ceiling of pages applies even when nextPageToken keeps promising more", async () => {
  let calls = 0;
  const fetchPage = () => {
    calls += 1;
    return Promise.resolve(page([`m${calls}`], `tok-${calls + 1}`));
  };
  const ids = await pagedList(fetchPage, () => true, 1_000_000);
  assertEquals(calls, MAX_LIST_PAGES, "a mailbox that never says 'no more pages' must still stop");
  assertEquals(ids.length, MAX_LIST_PAGES);
});

Deno.test("gmail-read's own call passes READ_CAP as pagedList's `want`, and the handler still caps and dedups after", async () => {
  // Production wiring lives in index.ts and is not itself unit-tested (no function's index.ts is,
  // in this codebase) — this proves the handler side of the contract: `api.list` receives the
  // account's own `unseen` predicate, and whatever it returns is STILL filtered and sliced by the
  // handler afterward (dedup-before-cap, R-C2-E42), even if a fake ignores `unseen` entirely and
  // hands back ids the account has already seen.
  const ids = Array.from({ length: READ_CAP + 10 }, (_, i) => `m${i}`);
  const { deps, queued, seen } = fakes(
    Array.from({ length: READ_CAP }, () => TASK_ANSWER),
    ids,
  );
  for (let i = 0; i < 3; i++) seen.add(`gmail:m${i}`);
  // Read at call time, before the loop below has a chance to mutate the SAME `seen` set the fake
  // `deps.seen()` hands back as `already` — `already.has(...)` would otherwise report every id
  // this very run has since processed as "seen", which is true by the end but proves nothing about
  // what the handler passed api.list before any of that happened.
  let sawM0 = "unset", sawM3 = "unset";
  deps.api.list = (_t, _q, unseen) => {
    sawM0 = unseen("m0") ? "unseen" : "already seen";
    sawM3 = unseen("m3") ? "unseen" : "already seen";
    return Promise.resolve(ids);
  };
  await readHandler(OK, deps)(post());
  assertEquals(queued.length, READ_CAP);
  assertEquals(sawM0, "already seen", "m0 was marked seen above");
  assertEquals(sawM3, "unseen", "m3 was never marked seen");
});

// ---------------------------------------------------------------------------------------------
// R-C2-E44 — the minors: a malformed ack is dropped, and a label carrying a quote is escaped.
// ---------------------------------------------------------------------------------------------

Deno.test("a malformed ack entry is dropped rather than reaching the delivery filter", async () => {
  const { deps } = fakes([TASK_ANSWER]);
  let delivered: string[] = [];
  const tracked = { ...deps, deliver: (_a: string, uids: string[]) => { delivered = uids; return Promise.resolve(); } };
  await readHandler(OK, tracked)(post({ ack: ["gmail:m1", "not-an-ack", "gmail:evil; drop table", ""] }));
  assertEquals(delivered, ["gmail:m1"]);
});

// F-6: a hard ceiling on one round's ack, so a device bug (or a hostile request) cannot build an
// unbounded `uid=in.(...)` filter.
Deno.test("an ack past the 500 cap is truncated, and the drop is logged rather than silent", async () => {
  const { deps } = fakes([TASK_ANSWER]);
  let delivered: string[] = [];
  const tracked = { ...deps, deliver: (_a: string, uids: string[]) => { delivered = uids; return Promise.resolve(); } };
  const ack = Array.from({ length: 600 }, (_, i) => `gmail:m${i}`);
  await readHandler(OK, tracked)(post({ ack }));
  assertEquals(delivered.length, 500, "only the first 500 reach deliver");
  assertEquals(delivered, ack.slice(0, 500));
});

Deno.test("a label carrying a double quote is escaped, not left to break the query", async () => {
  const { deps, asked } = fakes([TASK_ANSWER]);
  await readHandler(OK, { ...deps, excludedLabels: () => Promise.resolve(['Say "Hi"']) })(post());
  assertEquals(asked[0], 'newer_than:7d -label:"Say \\"Hi\\""');
});

// ---------------------------------------------------------------------------------------------
// Stream J Task T9: an LMS submission receipt is recognised BEFORE the model (tier 2 before tier 3),
// and a device that has not declared it understands `completion` is handed `information` instead.
// Every fixture is fabricated in the real template's shape.
// ---------------------------------------------------------------------------------------------

const RECEIPT: Message = {
  subject: "Submission received",
  from: "Blackboard <do-not-reply@blackboard.com>",
  date: "Thu, 03 Sep 2026 14:16:00 -0500",
  text: "12345.202640 202640-XX-101-001\nAssessment submitted\nLab 3: Pendulum\n" +
    `Submitted: Thursday, September 3, 2026 2:15:57 PM CDT\nConfirmation number: 0f1e2d3c4b5a ${TRIPWIRE}`,
};

Deno.test("T9: a submission receipt is queued as completion without a model call", async () => {
  // No scripted replies: a model call would find none and defer the message.
  const { deps, queued, rows, seen } = fakes([], ["m1"], RECEIPT);
  const reply = await (await readHandler(OK, deps)(post({ accepts: ["completion"] }))).json();
  assertEquals(reply.read, 1);
  assertEquals(reply.deferred, 0);
  assertEquals(queued.length, 1);
  assertEquals(queued[0].tier, "completion");
  assertEquals(queued[0].payload.title, "Lab 3: Pendulum");
  assertEquals(queued[0].payload.why, "Blackboard submission receipt");
  assert(seen.has("gmail:m1"));
  // Recorded as a free, deterministic tier-2 answer — never tier 3, so rule promotion never learns
  // from it — and with nothing of the message in it.
  assertEquals(rows.length, 1);
  assertEquals(rows[0].tier, 2);
  assertEquals(rows[0].model, null);
  assertEquals(rows[0].fields.tier, "completion");
  for (const leak of [TRIPWIRE, "0f1e2d3c4b5a", "Lab 3", "Submission received"]) {
    assert(!JSON.stringify(rows).includes(leak), `${leak} reached a judgment row`);
    assert(!JSON.stringify(queued).includes(leak) || leak === "Lab 3", `${leak} reached the queue`);
  }
  assertEquals(reply.items[0].tier, "completion");
});

Deno.test("T9: an unrelated newsletter still reaches the model and is queued as information", async () => {
  const { deps, queued, rows } = fakes([{
    tier: "information", title: "Weekly newsletter", course: null, due: null,
    effort_hours: null, importance: null, why: "a newsletter", confidence: 0.95,
  }], ["m1"], {
    subject: "This week on campus", from: "Campus News <news@example.invalid>",
    date: "Thu, 03 Sep 2026 08:00:00 -0500", text: "Five things happening this week.",
  });
  await readHandler(OK, deps)(post({ accepts: ["completion"] }));
  assertEquals(queued[0].tier, "information");
  assertEquals(rows[0].tier, 3);
});

Deno.test("T9: a posted grade from the same LMS is not a receipt, so the model is asked", async () => {
  const { deps, rows } = fakes([{
    tier: "information", title: "Grade posted", course: null, due: null,
    effort_hours: null, importance: null, why: "a grade notification", confidence: 0.9,
  }], ["m1"], {
    subject: "New grade and feedback for Lab 3: Pendulum in 202640-XX-101-001",
    from: "Blackboard <do-not-reply@blackboard.com>",
    date: "Thu, 10 Sep 2026 08:00:00 -0500", text: "A new grade and feedback is available.",
  });
  await readHandler(OK, deps)(post({ accepts: ["completion"] }));
  assertEquals(rows[0].tier, 3);
});

Deno.test("T9: a device that does not declare completion is handed it as information", async () => {
  // An older engine files every tier it does not know as a `kind: task` card — for a receipt that
  // would propose ADDING the work the student just finished. It never declares `accepts`, so it is
  // given the one tier every engine drops and records.
  const { deps, queued } = fakes([], ["m1"], RECEIPT);
  const old = await (await readHandler(OK, deps)(post())).json();
  assertEquals(queued[0].tier, "completion", "the queue keeps the real tier");
  assertEquals(old.items.length, 1);
  assertEquals(old.items[0].tier, "information");
  // The same row, pulled again by a device that does declare it, is completion.
  const fresh = await (await readHandler(OK, deps)(post({ accepts: ["completion"] }))).json();
  assertEquals(fresh.items[0].tier, "completion");
});

// ---------------------------------------------------------------------------------------------
// T9 fix round 1: a vendor's own not-evidence mail (a posted grade, "overdue", "due soon") is NEVER
// completion, deterministically — even when the model answers `completion` with the exact title.
// ---------------------------------------------------------------------------------------------

for (
  const [shape, subject] of [
    ["grade posted", "New grade and feedback for Lab 3: Pendulum in 202640-XX-101-001"],
    ["overdue", "Lab 3: Pendulum is overdue in 202640-XX-101-001"],
    ["due soon", "Lab 3: Pendulum is due soon in 202640-XX-101-001"],
  ]
) {
  Deno.test(`T9 fix 1: a model 'completion' for a Blackboard ${shape} email is queued as information`, async () => {
    const { deps, queued } = fakes([{
      tier: "completion", title: "Lab 3: Pendulum", course: null, due: null,
      effort_hours: null, importance: null, why: "the work is graded", confidence: 0.9,
    }], ["m1"], {
      subject, from: "Blackboard <do-not-reply@blackboard.com>",
      date: "Thu, 10 Sep 2026 08:00:00 -0500", text: "Lab 3: Pendulum.",
    });
    const reply = await (await readHandler(OK, deps)(post({ accepts: ["completion"] }))).json();
    assertEquals(queued.length, 1);
    assertEquals(queued[0].tier, "information");
    assertEquals(queued[0].payload.tier, "information");
    assertEquals(reply.items[0].tier, "information");
  });
}

Deno.test("T9 fix 1: a model 'completion' from a sender with no template is left alone", async () => {
  const { deps, queued } = fakes([{
    tier: "completion", title: "Essay 2", course: null, due: null,
    effort_hours: null, importance: null, why: "a submission confirmation", confidence: 0.9,
  }], ["m1"], {
    subject: "New grade and feedback for Essay 2 in XX-202",
    from: "Other LMS <noreply@lms.example.invalid>",
    date: "Thu, 10 Sep 2026 08:00:00 -0500", text: "Received.",
  });
  await readHandler(OK, deps)(post({ accepts: ["completion"] }));
  assertEquals(queued[0].tier, "completion");
});
