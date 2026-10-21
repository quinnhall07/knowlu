import { assert, assertEquals } from "@std/assert";
import { ProviderOutOfCredit, ScriptedModel } from "../_shared/judge_anthropic.ts";
import type { JudgmentRow } from "../_shared/judge_pipeline.ts";
import { CALENDAR_SCOPE, GMAIL_SCOPE } from "../_shared/google_scopes.ts";
import { fail } from "../_shared/http.ts";
import {
  forDevice,
  forwardingCounts,
  type GmailApi,
  GmailApiError,
  type GrantRow,
  type ListPage,
  lookupFromRow,
  type LookupSources,
  lookupToken,
  MAX_LIST_PAGES,
  pagedList,
  PER_CALL_MS,
  query,
  READ_BUDGET_MS,
  READ_CAP,
  readHandler,
  type ReadDeps,
  WINDOW,
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
    // D22: by default the account forwards nothing, so every older test reads as it always did.
    unlapse: () => Promise.resolve(),
    forwarding: () => Promise.resolve({ gmail: null, mailboxes: [] }),
    // L7: by default the grant was made at the fake clock's own instant (age 0), so no older test
    // can read as gone quiet, and a status move is a no-op.
    connectedAt: () => Promise.resolve(0),
    moveStatus: () => Promise.resolve(),
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
          chargeOutcome: () => Promise.resolve("charged"),
          chargeAllowance: () => Promise.resolve(false),
          refundAllowance: () => Promise.resolve(),
          refund: () => Promise.resolve(),
          tripped: () => false,
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
// Gmail connect T1 (spec §4.1, §8.3 items 1–2) — the row lookup is `lookupFromRow`, read without a
// status filter, so an account that never connected Google answers the silent `no_gmail_scope`
// rather than `revoked`, and only a row that says `revoked` reads as a dead grant.
// ---------------------------------------------------------------------------------------------

function sources(row: GrantRow | null, clientConfigured = true) {
  const calls = { row: 0, token: 0 };
  const src: LookupSources = {
    clientConfigured,
    row: () => {
      calls.row += 1;
      return Promise.resolve(row);
    },
    token: () => {
      calls.token += 1;
      return Promise.resolve({ token: "access-token-not-a-secret" });
    },
  };
  return { src, calls };
}

/** `readHandler` over `lookupToken`, with a spy on `markRevoked`. */
async function readThrough(src: LookupSources) {
  const { deps } = fakes([]);
  let marks = 0;
  const response = await readHandler(OK, {
    ...deps,
    accessTokenFor: (accountId) => lookupToken(accountId, src),
    markRevoked: () => {
      marks += 1;
      return Promise.resolve();
    },
  })(post());
  return { response, marks: () => marks };
}

Deno.test("lookupFromRow: no row reads as a missing scope and marks nothing", async () => {
  assertEquals(lookupFromRow(null), { missing: "scope" });
  const { src, calls } = sources(null);
  const { response, marks } = await readThrough(src);
  assertEquals((await response.json()).reason, "no_gmail_scope");
  assertEquals(marks(), 0, "an account that never connected Google is not a revoked grant");
  assertEquals(calls.token, 0, "no row means no token path");
});

Deno.test("lookupFromRow: a revoked row reads as a missing grant", () => {
  assertEquals(lookupFromRow({ status: "revoked", scopes: [GMAIL_SCOPE] }), { missing: "grant" });
  assertEquals(lookupFromRow({ status: "revoked", scopes: [] }), { missing: "grant" });
});

Deno.test("lookupFromRow: an active row without the Gmail scope reads as a missing scope", () => {
  assertEquals(lookupFromRow({ status: "active", scopes: [CALENDAR_SCOPE] }), { missing: "scope" });
  assertEquals(lookupFromRow({ status: "active", scopes: [] }), { missing: "scope" });
  // "Any other row": a `quiet` calendar-only grant is still not a dead one.
  assertEquals(lookupFromRow({ status: "quiet", scopes: [CALENDAR_SCOPE] }), { missing: "scope" });
});

Deno.test("lookupFromRow: an active row with the Gmail scope takes the token path", async () => {
  assertEquals(lookupFromRow({ status: "active", scopes: [CALENDAR_SCOPE, GMAIL_SCOPE] }), { tokenPath: true });
  const { src, calls } = sources({ status: "active", scopes: [GMAIL_SCOPE] });
  assertEquals(await lookupToken("acct-1", src), { token: "access-token-not-a-secret" });
  assertEquals(calls.token, 1);
});

Deno.test("readHandler: no client configured still answers 503", async () => {
  const { src, calls } = sources(null, false);
  const { response, marks } = await readThrough(src);
  assertEquals(response.status, 503);
  assertEquals(marks(), 0);
  assertEquals(calls.row, 0, "an unconfigured deployment is decided before any row read");
  assertEquals(calls.token, 0);
});

Deno.test("readHandler: an account with no google_accounts row answers quiet no_gmail_scope", async () => {
  const { src } = sources(null);
  const { response, marks } = await readThrough(src);
  assertEquals(response.status, 200);
  assertEquals(await response.json(), {
    items: [], read: 0, quiet: true, reason: "no_gmail_scope", more: false,
  });
  assertEquals(marks(), 0, "no mark is made for an account with no row");
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

// L7 (plan §3.2, Q6; T2a test 4): an out-of-credit reply stops the round as `capped` does. Every
// later item would meet the same spent balance, so none is judged; the item is deferred, neither
// queued nor marked seen, and nothing new reaches the device on this path.
Deno.test("an out-of-credit reply stops the round on the spot, defers, and answers more:false", async () => {
  const ids = ["m1", "m2"];
  const { deps, rows, queued, seen } = fakes([new ProviderOutOfCredit(), TASK_ANSWER], ids);
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(rows.length, 1, "m2 must never reach the pipeline once the service is out of credit");
  assertEquals(queued.length, 0, "an out-of-credit item is never enqueued");
  assert(!seen.has("gmail:m1"), "an out-of-credit item must not be marked seen, so it is asked about again");
  assert(!seen.has("gmail:m2"), "a later item must not be marked seen either");
  assertEquals(reply.deferred, 1);
  assertEquals(reply.more, false, "retrying the rest of THIS round would not help while out of credit");
  assertEquals("unavailable" in reply, false, "nothing new reaches the device on this path");
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

// Due-fix (2026-09-23): the device sends the vault's timezone on `/gmail-read`, and the handler
// hands it to the pipeline, so a UTC-stamped evening email's "tomorrow" is the next LOCAL day.
Deno.test("the request's timezone reaches the due resolver for every message the read judges", async () => {
  const { deps, queued } = fakes([{
    tier: "task", title: "Submit lab writeup", course: null, due: "tomorrow",
    effort_hours: 1, importance: 3, why: "the message sets a deadline", confidence: 0.9,
  }], ["m1"], {
    subject: "Lab writeup", from: "Instructor <instructor@example.invalid>",
    date: "Fri, 18 Sep 2026 04:30:00 +0000", text: "Submit the writeup by tomorrow.",
  });
  await readHandler(OK, deps)(post({ accepts: ["completion"], timezone: "America/Chicago" }));
  assertEquals(queued.length, 1);
  assertEquals(queued[0].payload.due, "2026-09-18");
});

// ---------------------------------------------------------------------------------------------
// Email forwarding D22 (spec test 16c, plan P22): one mailbox, one path. A Gmail address with an
// active forwarding to Knowlu is read by email forwarding alone; `gmail-read` reads nothing new and
// answers an ordinary, NON-quiet pull, because the merged device reads every unknown quiet reason
// as "re-connect from settings" and stops before reading items (`cloudmodel.rs:667-675`).
// ---------------------------------------------------------------------------------------------

const GMAIL_ADDRESS = "student@gmail.example.invalid";

Deno.test("a_gmail_address_with_a_forwarding_gets_a_non_quiet_answer_with_its_undelivered_items_and_no_gmail_call", async () => {
  const { deps, queued, rows } = fakes([TASK_ANSWER], ["m1", "m2"]);
  queued.push(
    { uid: "gmail:old1", tier: "task", payload: { title: "queued before the forwarding" }, judgment_id: "judgment-0" },
    { uid: "gmail:old2", tier: "event", payload: { title: "acked this round" }, judgment_id: null },
  );
  let tokens = 0;
  let listed = 0;
  let fetched = 0;
  let revoked = 0;
  let delivered: string[] = [];
  const response = await readHandler(OK, {
    ...deps,
    // The OAuth hint and the forwarding's hint differ only in case: P22 compares them case-folded.
    forwarding: () =>
      Promise.resolve({ gmail: "Student@Gmail.Example.Invalid", mailboxes: [{ hint: GMAIL_ADDRESS, status: "quiet" }] }),
    accessTokenFor: () => {
      tokens += 1;
      return Promise.resolve({ token: "access-token-not-a-secret" });
    },
    api: {
      list: () => {
        listed += 1;
        return Promise.resolve(["m1", "m2"]);
      },
      message: () => {
        fetched += 1;
        return Promise.reject(new Error("no message is fetched on the forwarded path"));
      },
    },
    markRevoked: () => {
      revoked += 1;
      return Promise.resolve();
    },
    deliver: (account, uids) => {
      delivered = uids;
      return deps.deliver(account, uids);
    },
  })(post({ ack: ["gmail:old2"] }));
  assertEquals(response.status, 200);
  const body = await response.json();
  assertEquals(body, {
    items: [{ uid: "gmail:old1", tier: "task", payload: { title: "queued before the forwarding" }, judgment_id: "judgment-0" }],
    read: 0,
    quiet: false,
    more: false,
    via_forward: true,
    // L7 (plan §3.3, T2b test 7): forwarding owns this mailbox's silence, so the answer is never
    // "gone quiet". The one key added to this exact-body pin; every other key is as it was.
    went_quiet: false,
  });
  assertEquals(delivered, ["gmail:old2"], "an ack is still applied on the forwarded path");
  assertEquals([tokens, listed, fetched, revoked], [0, 0, 0, 0], "no token exchange, no Gmail call, no revoke");
  assertEquals(rows.length, 0, "nothing is judged on the forwarded path");
});

Deno.test("a_gmail_address_without_a_forwarding_reads_exactly_as_today", async () => {
  // Shapes of "no active forwarding": none at all, one that is off or revoked, a setup still pending or
  // blocked (PQ11), and one for a different address on the same account. Each reads, judges and
  // answers as before, with no `via_forward` field at all.
  const cases: Array<{ gmail: string | null; mailboxes: Array<{ hint: string | null; status: string }> }> = [
    { gmail: GMAIL_ADDRESS, mailboxes: [] },
    { gmail: GMAIL_ADDRESS, mailboxes: [{ hint: GMAIL_ADDRESS, status: "off" }, { hint: GMAIL_ADDRESS, status: "revoked" }] },
    // PQ11: a setup that never finished (`pending`) or whose canary never came (`blocked`) keeps reading.
    { gmail: GMAIL_ADDRESS, mailboxes: [{ hint: GMAIL_ADDRESS, status: "pending" }] },
    { gmail: GMAIL_ADDRESS, mailboxes: [{ hint: GMAIL_ADDRESS, status: "blocked" }] },
    { gmail: GMAIL_ADDRESS, mailboxes: [{ hint: "student@school.example.invalid", status: "active" }] },
  ];
  for (const forwarding of cases) {
    const { deps, queued, asked } = fakes([TASK_ANSWER]);
    const body = await (await readHandler(OK, { ...deps, forwarding: () => Promise.resolve(forwarding) })(post())).json();
    assertEquals(asked.length, 1, "Gmail is listed");
    assertEquals(queued.length, 1);
    assertEquals(body.read, 1);
    assertEquals(body.quiet, false);
    assertEquals(body.more, false);
    assertEquals(body.items.length, 1);
    assertEquals("via_forward" in body, false);
  }
});

// P22 as amended by PQ11 (Quinn, 2026-10-02): an allow-list. A forwarding counts only while mail arrives
// by it; a stalled (`blocked`) or abandoned (`pending`) setup keeps the Gmail OAuth read.
Deno.test("P22: a forwarding counts only while confirmed, active or quiet", () => {
  for (const status of ["confirmed", "active", "quiet"]) {
    assertEquals(forwardingCounts(GMAIL_ADDRESS, [{ hint: GMAIL_ADDRESS, status }]), true, status);
  }
  for (const status of ["pending", "blocked", "paused_send", "lapsed", "revoked", "off", "", "Active"]) {
    assertEquals(forwardingCounts(GMAIL_ADDRESS, [{ hint: GMAIL_ADDRESS, status }]), false, status);
  }
});

// The lapsed-mailbox fix (2026-10-19): a lapsed mailbox leaves `lapsed` on its account's first entitled
// pull, as `quiet` when a canary ever reached it. Forwarding still delivers then, so the OAuth read must
// stop again or every message is filed twice; one never reached reads `pending` and the read goes on.
Deno.test("a_mailbox_unlapsed_to_quiet_counts_as_forwarding", () => {
  const box = (status: string) => [{ hint: GMAIL_ADDRESS, status }];
  assertEquals(forwardingCounts(GMAIL_ADDRESS, box("lapsed")), false, "while lapsed");
  assertEquals(forwardingCounts(GMAIL_ADDRESS, box("quiet")), true, "unlapsed, a canary once reached it");
  assertEquals(forwardingCounts(GMAIL_ADDRESS, box("pending")), false, "unlapsed, no canary ever reached it");
});

// Review of the lapsed fix, #2: the judge step calls `gmail-read` before `mail-pull`, so in the first
// entitled slot after a renewal `gmail-read` met a mailbox still `lapsed`, read the gap by OAuth, and the
// device's recover then forwarded the same messages. It is past the entitlement gate too, so it moves the
// account's lapsed mailboxes on itself, before it reads the forwarding (the same compare-and-sets).
Deno.test("a_lapsed_forwarding_is_unlapsed_before_the_forwarding_read_so_the_gap_is_never_read_twice", async () => {
  for (const [canary, forwarded] of [[true, true], [false, false]] as const) {
    const { deps, asked } = fakes([TASK_ANSWER]);
    const box = { hint: GMAIL_ADDRESS, status: "lapsed", canaryOk: canary };
    const order: string[] = [];
    let tokens = 0;
    const body = await (await readHandler(OK, {
      ...deps,
      unlapse: (account) => {
        assertEquals(account, "acct-1");
        order.push("unlapse");
        if (box.status === "lapsed") box.status = box.canaryOk ? "quiet" : "pending";
        return Promise.resolve();
      },
      forwarding: () => {
        order.push("forwarding");
        return Promise.resolve({ gmail: GMAIL_ADDRESS, mailboxes: [{ hint: box.hint, status: box.status }] });
      },
      accessTokenFor: (a) => {
        tokens += 1;
        return deps.accessTokenFor(a);
      },
    })(post())).json();
    assertEquals(order, ["unlapse", "forwarding"], `canary ${canary}`);
    if (forwarded) {
      // A canary once reached it: `quiet`, so forwarding alone reads it and recover brings the gap in.
      assertEquals([body.via_forward, body.read, tokens, asked.length], [true, 0, 0, 0]);
    } else {
      // No canary ever did: `pending`, setup never finished, and the OAuth read goes on.
      assertEquals([("via_forward" in body), body.read, tokens, asked.length], [false, 1, 1, 1]);
    }
  }
});

Deno.test("an_unentitled_read_unlapses_nothing", async () => {
  const { deps } = fakes([]);
  let unlapsed = 0;
  const refuse = () => Promise.reject(fail(402, "no entitlement"));
  const res = await readHandler(refuse, {
    ...deps,
    unlapse: () => {
      unlapsed += 1;
      return Promise.resolve();
    },
  })(post());
  assertEquals([res.status, unlapsed], [402, 0]);
  await res.body?.cancel();
});

Deno.test("P22: the address is matched whole and case-folded, never as a substring", () => {
  assertEquals(forwardingCounts(" Student@GMAIL.example.invalid ", [{ hint: GMAIL_ADDRESS, status: "active" }]), true);
  assertEquals(forwardingCounts(GMAIL_ADDRESS, [{ hint: `not${GMAIL_ADDRESS}`, status: "active" }]), false);
  assertEquals(forwardingCounts(`not${GMAIL_ADDRESS}`, [{ hint: GMAIL_ADDRESS, status: "active" }]), false);
  assertEquals(forwardingCounts(GMAIL_ADDRESS, [{ hint: null, status: "active" }]), false, "a hint not yet set");
  assertEquals(forwardingCounts(null, [{ hint: GMAIL_ADDRESS, status: "active" }]), false, "no OAuth address");
  assertEquals(forwardingCounts("", [{ hint: "", status: "active" }]), false, "two empty strings are not one address");
});

// ---------------------------------------------------------------------------------------------
// L7 T2b (plan §3.3; Quinn's Q3 ruling, 2026-10-19): a connected Gmail has GONE QUIET when the one
// 7-day list the read already makes holds no message at all, seen or not, and the grant is at least
// 7 × 24 hours old, compared in milliseconds. The answer is the rule, never the write's effect.
// "N days old" describes a fixture only: `connected_at` is N × 24 hours before the fake clock.
// ---------------------------------------------------------------------------------------------

const DAY_MS = 86_400_000;
const SEVEN_DAYS_MS = 7 * DAY_MS;
const NOW = Date.UTC(2026, 9, 20, 12, 0, 0);
const EXCLUDED = ["Promotions"];

/** `fakes()` over a grant row with a status and an age against a fixed clock. Every status write is
 * recorded with its condition, and changes the row only while the row reads `from`, as the real
 * `status=eq.<from>` filter does. */
function grant(opts: { status: string; ageMs: number; ids: string[]; failMove?: boolean }) {
  const f = fakes([TASK_ANSWER], opts.ids);
  const row = { status: opts.status };
  const moves: Array<{ from: string; to: string }> = [];
  let messages = 0;
  const base = f.deps.api;
  const deps: ReadDeps = {
    ...f.deps,
    api: {
      list: base.list,
      message: (t, id) => {
        messages += 1;
        return base.message(t, id);
      },
    },
    excludedLabels: () => Promise.resolve(EXCLUDED),
    connectedAt: () => Promise.resolve(NOW - opts.ageMs),
    moveStatus: (_a, from, to) => {
      moves.push({ from, to });
      if (opts.failMove) return Promise.reject(new Error("status write failed"));
      if (row.status === from) row.status = to;
      return Promise.resolve();
    },
    clock: () => NOW,
  };
  return { ...f, deps, row, moves, messages: () => messages };
}

/** Exactly one list call, and its query is today's `query()` output with `WINDOW` first. */
function assertOneListCall(asked: string[]) {
  assertEquals(asked, [query(EXCLUDED)], "exactly one list call, with today's query");
  assert(asked[0].startsWith(`${WINDOW} `), "WINDOW leads the query");
}

Deno.test("a_message_five_days_old_is_mail", async () => {
  // The fake list holds the one id a 5-day-old message would put on the 7-day list.
  const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: ["m1"] });
  const body = await (await readHandler(OK, g.deps)(post())).json();
  assertOneListCall(g.asked);
  assertEquals(body.went_quiet, false);
  assertEquals(g.moves, [{ from: "quiet", to: "active" }]);
  assertEquals(g.row.status, "active");
  assertEquals([body.read, g.queued.length, g.queued[0].uid], [1, 1, "gmail:m1"], "the id is judged as today");
});

Deno.test("a_seven_day_list_of_seen_ids_is_not_quiet", async () => {
  const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: ["m1", "m2"] });
  g.seen.add("gmail:m1");
  g.seen.add("gmail:m2");
  const body = await (await readHandler(OK, g.deps)(post())).json();
  assertOneListCall(g.asked);
  assertEquals([body.went_quiet, body.read], [false, 0], "an already-seen message is still mail");
  assertEquals(g.moves, [{ from: "quiet", to: "active" }]);
  assertEquals(g.row.status, "active");
  assertEquals(g.rows.length, 0, "nothing is judged twice");
});

Deno.test("a_grant_six_days_old_is_not_quiet", async () => {
  const g = grant({ status: "active", ageMs: 6 * DAY_MS, ids: [] });
  const body = await (await readHandler(OK, g.deps)(post())).json();
  assertOneListCall(g.asked);
  assertEquals(body.went_quiet, false);
  assertEquals(g.moves, [], "no status is written");
});

Deno.test("a_grant_one_millisecond_short_of_seven_days_is_not_quiet", async () => {
  // Red under `Math.round` or `Math.ceil` on days: 7 × 24 hours − 1 ms is not a week.
  const g = grant({ status: "active", ageMs: SEVEN_DAYS_MS - 1, ids: [] });
  const body = await (await readHandler(OK, g.deps)(post())).json();
  assertOneListCall(g.asked);
  assertEquals(body.went_quiet, false);
  assertEquals(g.moves, [], "no status is written");
  assertEquals(g.row.status, "active");
});

Deno.test("a_grant_exactly_seven_days_old_goes_quiet", async () => {
  for (const ageMs of [SEVEN_DAYS_MS, 30 * DAY_MS]) {
    const g = grant({ status: "active", ageMs, ids: [] });
    const response = await readHandler(OK, g.deps)(post());
    assertEquals(response.status, 200);
    const body = await response.json();
    assertOneListCall(g.asked);
    assertEquals(body.went_quiet, true, `age ${ageMs} ms`);
    assertEquals(body.quiet, false, "`quiet: true` keeps its one meaning, not connected");
    assertEquals("reason" in body, false);
    assertEquals(g.moves, [{ from: "active", to: "quiet" }]);
    assertEquals(g.row.status, "quiet");
  }
});

Deno.test("a_row_already_quiet_still_answers_went_quiet_true", async () => {
  // Red if the answer is taken from the write's effect: the heads-up would close and reopen on
  // alternate slots.
  const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: [] });
  const body = await (await readHandler(OK, g.deps)(post())).json();
  assertOneListCall(g.asked);
  assertEquals(body.went_quiet, true);
  assert(g.moves.every((m) => m.from === "active" && m.to === "quiet"), JSON.stringify(g.moves));
  assertEquals(g.row.status, "quiet", "the conditional write matched nothing");
});

/** Runs `fn` with `console.error` captured, so a test can count the outer catch's one line. */
async function capturingErrors<T>(fn: () => Promise<T>): Promise<{ value: T; lines: string[] }> {
  const lines: string[] = [];
  const original = console.error;
  console.error = (...args: unknown[]) => lines.push(args.map(String).join(" "));
  try {
    return { value: await fn(), lines };
  } finally {
    console.error = original;
  }
}

Deno.test("a_failed_list_claims_nothing", async () => {
  // A pin on today's behaviour: the outer catch, its one class-and-status line, nothing new.
  const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: [] });
  g.deps.api.list = (_t, q) => {
    g.asked.push(q);
    return Promise.reject(new GmailApiError(503, "gmail list 503"));
  };
  const { value: response, lines } = await capturingErrors(() => readHandler(OK, g.deps)(post()));
  assertOneListCall(g.asked);
  assertEquals(response.status, 500);
  const body = await response.json();
  assertEquals(body, { error: "read failed" });
  assertEquals("went_quiet" in body, false);
  assertEquals(g.moves, [], "a failed list writes no status");
  assertEquals(lines, ["gmail-read: GmailApiError 503"], "no new log line");
});

Deno.test("a_failed_status_write_is_a_500_before_any_item_is_judged", async () => {
  const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: ["m1"], failMove: true });
  const { value: response, lines } = await capturingErrors(() => readHandler(OK, g.deps)(post()));
  assertOneListCall(g.asked);
  assertEquals(response.status, 500);
  assertEquals(await response.json(), { error: "read failed" });
  assertEquals(g.moves, [{ from: "quiet", to: "active" }]);
  assertEquals(g.messages(), 0, "the write runs before the loop's first message fetch");
  assertEquals([g.queued.length, g.seen.size], [0, 0], "no enqueue and no markSeen");
  assertEquals(lines, ["gmail-read: Error"], "the outer catch's own line, not a catch of its own");
});

Deno.test("every_status_write_carries_its_condition_so_a_revoked_row_is_never_written", async () => {
  const cases = [
    { ids: ["m1"], ageMs: 30 * DAY_MS },
    { ids: [], ageMs: 30 * DAY_MS },
    { ids: [], ageMs: SEVEN_DAYS_MS },
    { ids: [], ageMs: 6 * DAY_MS },
  ];
  for (const status of ["active", "quiet", "revoked"]) {
    for (const c of cases) {
      const g = grant({ status, ...c });
      await readHandler(OK, g.deps)(post());
      assertOneListCall(g.asked);
      for (const m of g.moves) {
        assert(["active", "quiet"].includes(m.from) && ["active", "quiet"].includes(m.to), JSON.stringify(m));
        assert(m.from !== m.to, JSON.stringify(m));
      }
      if (status === "revoked") assertEquals(g.row.status, "revoked", JSON.stringify(c));
    }
  }
  // And the real write: the condition is in the account-scoped PostgREST filter itself, a literal
  // path `judge_db_test.ts`'s account-scoping scan reads.
  const index = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(
    index.includes("`google_accounts?account_id=eq.${accountId}&status=eq.${from}`"),
    "moveStatus must write only where status=eq.<from>",
  );
});

Deno.test("the_forwarding_path_moves_quiet_to_active_and_never_lists", async () => {
  const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: ["m1"] });
  const body = await (await readHandler(OK, {
    ...g.deps,
    forwarding: () => Promise.resolve({ gmail: GMAIL_ADDRESS, mailboxes: [{ hint: GMAIL_ADDRESS, status: "active" }] }),
  })(post())).json();
  assertEquals(body.via_forward, true);
  assertEquals(body.went_quiet, false, "forwarding owns that mailbox's silence");
  assertEquals(g.moves, [{ from: "quiet", to: "active" }]);
  assertEquals(g.row.status, "active");
  assertEquals(g.asked.length, 0, "no list call at all");
});

Deno.test("the_not_connected_replies_carry_no_went_quiet_and_write_no_status", async () => {
  for (const missing of ["scope", "grant"] as const) {
    const g = grant({ status: "quiet", ageMs: 30 * DAY_MS, ids: [] });
    let revoked = 0;
    const body = await (await readHandler(OK, {
      ...g.deps,
      accessTokenFor: () => Promise.resolve({ missing }),
      markRevoked: () => {
        revoked += 1;
        return Promise.resolve();
      },
    })(post())).json();
    assertEquals(body.quiet, true, missing);
    assertEquals(body.reason, missing === "scope" ? "no_gmail_scope" : "revoked");
    assertEquals("went_quiet" in body, false, missing);
    assertEquals(g.moves, [], "no status write beyond today's markRevoked");
    assertEquals(revoked, missing === "grant" ? 1 : 0);
    assertEquals(g.asked.length, 0);
  }
});

// Email follow-ups T4 (Q4, Quinn 2026-10-20; D22 as amended): Rotate sends a Gmail mailbox back to
// `pending` for the device's confirm wait, while the old forward still delivers through the retired
// token for up to seven days. Inside that grace the row counts as forwarding when the status it was
// rotated from (`rotated_from`) counted; once the grace has run out it counts by its status alone.
const ROTATE_NOW = Date.UTC(2026, 9, 20, 12, 0, 0);
const GRACE_LEFT = new Date(ROTATE_NOW + 3 * 86_400_000).toISOString();
const GRACE_GONE = new Date(ROTATE_NOW - 1).toISOString();

/** One round of the handler over a single rotated Gmail mailbox, counting what reached Gmail. */
async function rotatedRound(box: { status: string; retiredUntil: string | null; rotatedFrom: string | null }) {
  const { deps, queued, asked, rows } = fakes([TASK_ANSWER]);
  queued.push({ uid: "gmail:old1", tier: "task", payload: { title: "queued before the rotate" }, judgment_id: null });
  let tokens = 0;
  let fetched = 0;
  const body = await (await readHandler(OK, {
    ...deps,
    clock: () => ROTATE_NOW,
    forwarding: () => Promise.resolve({ gmail: GMAIL_ADDRESS, mailboxes: [{ hint: GMAIL_ADDRESS, ...box }] }),
    accessTokenFor: (a) => {
      tokens += 1;
      return deps.accessTokenFor(a);
    },
    api: {
      list: (t, q, u) => deps.api.list(t, q, u),
      message: (t, id) => {
        fetched += 1;
        return deps.api.message(t, id);
      },
    },
  })(post())).json();
  return { body, tokens, listed: asked.length, fetched, judged: rows.length };
}

Deno.test("a_rotated_active_gmail_mailbox_keeps_the_oauth_read_off_inside_its_grace", async () => {
  for (const rotatedFrom of ["confirmed", "active", "quiet"]) {
    const r = await rotatedRound({ status: "pending", retiredUntil: GRACE_LEFT, rotatedFrom });
    // Exactly D22's answer, as the forwarding test above asserts it.
    assertEquals(r.body, {
      items: [{ uid: "gmail:old1", tier: "task", payload: { title: "queued before the rotate" }, judgment_id: null }],
      read: 0,
      quiet: false,
      more: false,
      via_forward: true,
      // L7 (plan §3.3): the forwarded answer is never "gone quiet", as the D22 pin above asserts.
      went_quiet: false,
    }, rotatedFrom);
    assertEquals([r.tokens, r.listed, r.fetched, r.judged], [0, 0, 0, 0], `no token exchange, no Gmail call (${rotatedFrom})`);
  }
});

Deno.test("a_rotated_gmail_mailbox_whose_grace_ran_out_unconfirmed_is_read_by_oauth", async () => {
  // The old forward has stopped delivering, and the new address was never confirmed: the read comes back.
  for (const retiredUntil of [GRACE_GONE, new Date(ROTATE_NOW).toISOString(), null]) {
    const r = await rotatedRound({ status: "pending", retiredUntil, rotatedFrom: "active" });
    assertEquals("via_forward" in r.body, false, String(retiredUntil));
    assertEquals([r.body.read, r.tokens, r.listed, r.fetched], [1, 1, 1, 1], String(retiredUntil));
  }
});

Deno.test("a_gmail_mailbox_rotated_from_pending_blocked_off_or_lapsed_is_read_by_oauth", async () => {
  // It did not count before the Rotate, so it does not count inside the grace either.
  for (const rotatedFrom of ["pending", "blocked", "off", "lapsed", "paused_send", "revoked", "", "Active", null]) {
    const r = await rotatedRound({ status: "pending", retiredUntil: GRACE_LEFT, rotatedFrom });
    assertEquals("via_forward" in r.body, false, String(rotatedFrom));
    assertEquals([r.body.read, r.tokens, r.listed, r.fetched], [1, 1, 1, 1], String(rotatedFrom));
  }
});

Deno.test("forwardingCounts_counts_a_pending_row_only_inside_a_rotate_grace_from_a_counting_status", () => {
  const at = (status: string, retiredUntil: string | null = null, rotatedFrom: string | null = null) => [
    { hint: GMAIL_ADDRESS, status, retiredUntil, rotatedFrom },
  ];
  // The allow-list as before, with or without a grace on the row.
  for (const status of ["confirmed", "active", "quiet"]) {
    assertEquals(forwardingCounts(GMAIL_ADDRESS, at(status), () => ROTATE_NOW), true, status);
    assertEquals(forwardingCounts(GMAIL_ADDRESS, at(status, GRACE_GONE, "pending"), () => ROTATE_NOW), true, status);
  }
  // `pending` without a grace, or with one that has run out, does not count.
  assertEquals(forwardingCounts(GMAIL_ADDRESS, at("pending"), () => ROTATE_NOW), false, "no grace");
  assertEquals(forwardingCounts(GMAIL_ADDRESS, at("pending", null, "active"), () => ROTATE_NOW), false, "a stale rotated_from");
  assertEquals(forwardingCounts(GMAIL_ADDRESS, at("pending", GRACE_GONE, "active"), () => ROTATE_NOW), false, "grace gone");
  assertEquals(forwardingCounts(GMAIL_ADDRESS, at("pending", "not a time", "active"), () => ROTATE_NOW), false, "unparseable");
  // Inside the grace, from a counting status: counts.
  assertEquals(forwardingCounts(GMAIL_ADDRESS, at("pending", GRACE_LEFT, "active"), () => ROTATE_NOW), true, "in grace");
  // The grace widens `pending` only: any other status reads by the allow-list alone.
  for (const status of ["blocked", "off", "lapsed", "revoked", "paused_send"]) {
    assertEquals(forwardingCounts(GMAIL_ADDRESS, at(status, GRACE_LEFT, "active"), () => ROTATE_NOW), false, status);
  }
  // The address is still matched whole: `notjane@` is not `jane@`, grace or not.
  const jane = "jane@gmail.example.invalid";
  const rotated = { status: "pending", retiredUntil: GRACE_LEFT, rotatedFrom: "active" };
  assertEquals(forwardingCounts(jane, [{ hint: `not${jane}`, ...rotated }], () => ROTATE_NOW), false);
  assertEquals(forwardingCounts(`not${jane}`, [{ hint: jane, ...rotated }], () => ROTATE_NOW), false);
  assertEquals(forwardingCounts(` Jane@GMAIL.example.invalid `, [{ hint: jane, ...rotated }], () => ROTATE_NOW), true);
});

Deno.test("index_reads_the_rotate_grace_fields_for_d22", async () => {
  // A source pin, as L7's index tests pin `gmail-read/index.ts`: without these two columns the grace
  // branch above never sees a row, and a rotated Gmail is read twice.
  const src = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(src.includes("select=mailbox_hint,status,retired_until,rotated_from&limit=20"), "the select list");
  assert(src.includes("retiredUntil: m.retired_until"), "retired_until is handed on");
  assert(src.includes("rotatedFrom: m.rotated_from"), "rotated_from is handed on");
  const handler = await Deno.readTextFile(new URL("./handler.ts", import.meta.url));
  assert(handler.includes("forwardingCounts(forwarding.gmail, forwarding.mailboxes, deps.clock)"), "the handler's own clock");
});
