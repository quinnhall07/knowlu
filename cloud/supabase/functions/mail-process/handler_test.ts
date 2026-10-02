// Email plan T3d (spec §4.7 steps 1–3 and step 6's deletes; tests 8, 15 and 16, the process halves; PQ8
// (ii) only). The handler runs whole: S3 against a fake bucket on one listener at 127.0.0.1:0 reached
// through an injected fetch that keeps the real-shaped AWS URLs; the database is an in-memory store; the
// model path (T3e's) is a spy that counts the items it was handed. Every address, domain, course and
// message here is invented for these tests: no real capture is used.
import { assert, assertEquals } from "@std/assert";
import type { RuleTable } from "../_shared/judge_rules.ts";
import { mailUid, parseHeaders } from "../_shared/mime.ts";
import { s3Client } from "../_shared/s3.ts";
import {
  type ClaimsRow,
  handle,
  type Mailbox,
  type ModelInput,
  type ProcessConfig,
  type ProcessDeps,
  type ProcessStore,
  type WorkItem,
} from "./handler.ts";

const BUCKET = "mail-test-bucket";
const NOW = Date.UTC(2026, 9, 2, 15, 0, 0);
const MARKER = "zq-marker-5528";
const TOKEN = "test-jobs-token-0000";
const CFG: ProcessConfig = { jobsToken: TOKEN };
const enc = new TextEncoder();

// ---------------------------------------------------------------- the store, the rules, the bucket

class FakeStore implements ProcessStore {
  work: WorkItem[] = [];
  claims: ClaimsRow[] = [];
  done: string[] = [];
  counters: Record<string, number> = {};
  /** Jane's account: her Gmail and her school's M365 mailbox. */
  boxes: Record<string, Mailbox[]> = {
    "acct-1": [
      { id: "addr-1", provider: "google", mailboxHint: "jane@gmail.com" },
      { id: "addr-4", provider: "m365", mailboxHint: "jdoe@school.test" },
    ],
  };
  courses: Record<string, string[]> = { "acct-1": ["xx-101", "yy-220"] };
  calls: string[] = [];
  due(nowIso: string, limit: number) {
    this.calls.push("due");
    const ready = this.work.filter((w) =>
      !this.done.includes(w.id) && Date.parse(w.nextAttemptAt) <= Date.parse(nowIso)
    );
    return Promise.resolve(
      ready.sort((a, b) => Date.parse(a.arrivedAt) - Date.parse(b.arrivedAt)).slice(0, limit),
    );
  }
  lease(item: WorkItem, untilIso: string) {
    this.calls.push("lease");
    const row = this.work.find((w) => w.id === item.id);
    if (row === undefined || row.nextAttemptAt !== item.nextAttemptAt) return Promise.resolve(false);
    row.nextAttemptAt = untilIso;
    return Promise.resolve(true);
  }
  mailboxes(accountId: string) {
    this.calls.push("mailboxes");
    return Promise.resolve(this.boxes[accountId] ?? []);
  }
  knownCourses(accountId: string) {
    this.calls.push("knownCourses");
    return Promise.resolve(this.courses[accountId] ?? []);
  }
  queue(row: ClaimsRow) {
    this.calls.push("queue");
    this.claims.push(row);
    return Promise.resolve();
  }
  finish(workId: string) {
    this.calls.push("finish");
    this.done.push(workId);
    return Promise.resolve();
  }
  count(addressId: string, counter: "unauthenticated") {
    this.calls.push("count");
    this.counters[`${addressId}:${counter}`] = (this.counters[`${addressId}:${counter}`] ?? 0) + 1;
    return Promise.resolve();
  }
}

/** Promoted rules, by the sender as `judge_rules.ts` keys an email rule (its `from`). */
class FakeRules implements RuleTable {
  verdicts: Record<string, Record<string, unknown>> = {};
  asked: Array<[string, string, Record<string, unknown>]> = [];
  lookup(account: string, kind: string, item: Record<string, unknown>) {
    this.asked.push([account, kind, item]);
    return Promise.resolve(this.verdicts[String(item.from)] ?? null);
  }
}

interface World {
  store: FakeStore;
  rules: FakeRules;
  bucket: Map<string, Uint8Array>;
  /** What the model path (T3e) was handed. */
  modelled: ModelInput[];
  logs: string[];
  deps: ProcessDeps;
  /** Put `raw` under `work/` as intake would, with its `mail_work` row. */
  put(raw: string, o?: Partial<WorkItem>): WorkItem;
  /** One cron call. */
  run(token?: string): Promise<Response>;
}

async function withWorld(fn: (w: World) => Promise<void>): Promise<void> {
  const store = new FakeStore();
  const rules = new FakeRules();
  const bucket = new Map<string, Uint8Array>();
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    const key = decodeURIComponent(new URL(req.url).pathname.slice(1));
    const body = new Uint8Array(await req.arrayBuffer());
    if (req.method === "GET") {
      const got = bucket.get(key);
      return got ? new Response(new Uint8Array(got)) : new Response("<Error/>", { status: 404 });
    }
    if (req.method === "PUT") {
      bucket.set(key, body);
      return new Response(null, { status: 200 });
    }
    if (req.method === "DELETE") {
      bucket.delete(key);
      return new Response(null, { status: 204 });
    }
    return new Response("<Error/>", { status: 405 });
  });
  const local = (raw: string, init?: RequestInit) => {
    const u = new URL(raw);
    return fetch(`http://127.0.0.1:${server.addr.port}${u.pathname}${u.search}`, init);
  };
  const s3 = s3Client(
    { bucket: BUCKET, region: "us-east-2", accessKeyId: "AKIDTESTONLY0000000", secretAccessKey: "test-only" },
    { fetch: local, now: () => new Date(NOW) },
  );
  const w = { store, rules, bucket, modelled: [] as ModelInput[], logs: [] as string[] } as World;
  w.deps = {
    store,
    s3,
    rules,
    now: () => NOW,
    model: (input) => {
      w.modelled.push(input);
      return Promise.resolve();
    },
  };
  let n = 0;
  w.put = (raw, o = {}) => {
    const item: WorkItem = {
      id: String(++n),
      accountId: "acct-1",
      addressId: "addr-1",
      batchId: null,
      s3Key: `work/item-${n}`,
      phase: "live",
      authenticity: "aligned",
      arrivedAt: new Date(NOW - 3_600_000 + n * 1000).toISOString(),
      nextAttemptAt: new Date(NOW - 60_000).toISOString(),
      attempts: 0,
      batchOpenedAt: null,
      ...o,
    };
    bucket.set(item.s3Key, enc.encode(raw));
    store.work.push(item);
    return item;
  };
  w.run = (token = TOKEN) =>
    handle(
      new Request("http://127.0.0.1/mail-process", {
        method: "POST",
        headers: { "content-type": "application/json", "x-knowlu-job-token": token },
        body: "{}",
      }),
      CFG,
      w.deps,
    );
  const saved = { ...console };
  const capture = (...args: unknown[]) => w.logs.push(args.map(String).join(" "));
  Object.assign(console, { log: capture, info: capture, warn: capture, error: capture, debug: capture });
  try {
    await fn(w);
  } finally {
    Object.assign(console, saved);
    await server.shutdown();
  }
}

const keysUnder = (w: World, prefix: string) =>
  [...w.bucket.keys()].filter((k) => k.startsWith(prefix)).sort();

// ---------------------------------------------------------------- the mail

let mid = 0;
/** One original as the provider delivered it; the marker is in its Message-ID, Subject and text unless the
 * caller sets those. */
function mail(o: { from: string; subject?: string; extra?: string[]; body?: string }): string {
  return [
    `From: ${o.from}`,
    "To: jane@gmail.com",
    `Subject: ${o.subject ?? `Lab 5 moved ${MARKER}`}`,
    "Date: Fri, 02 Oct 2026 09:00:00 -0500",
    `Message-ID: <${MARKER}.${++mid}@origin.test>`,
    ...(o.extra ?? []),
  ].join("\r\n") + `\r\n\r\n${o.body ?? `Lab 5 is now due Friday at noon. ${MARKER}`}\r\n`;
}
const PROF = `Prof ${MARKER} <prof@school.test>`;
const UNSUB = "List-Unsubscribe: <mailto:leave@lists.test>";
/** A Blackboard submission receipt (`lms_receipts.ts`'s row), the marker after the confirmation number. */
const receipt = (title: string) =>
  mail({
    from: "Blackboard <do-not-reply@blackboard.com>",
    subject: "Submission received",
    body:
      `202640.XX XX-101-001 Assessment submitted ${title} Submitted: Friday, October 2, 2026 9:00 AM CDT ` +
      `Confirmation number: 0a1b2c3d ${MARKER}`,
  });
const modelledFroms = (w: World) => w.modelled.map((m) => m.message.from);

// ---------------------------------------------------------------- test 8 and the screen's rules

Deno.test("bulk_mail_from_an_off_list_sender_is_noise_with_zero_model_calls", async () => {
  await withWorld(async (w) => {
    w.put(mail({ from: `Club ${MARKER} <news@club.test>`, extra: [UNSUB] }));
    w.put(mail({ from: "deals@shop.test", extra: ["Precedence: bulk"] }));
    w.put(mail({ from: "digest@forum.test", extra: ["Precedence: List"] }));
    assertEquals((await w.run()).status, 200);
    assertEquals(w.modelled, []);
    // The screen decided before any rule was read, and each object and work row is gone.
    assertEquals(w.rules.asked, []);
    assertEquals(keysUnder(w, "work/"), []);
    assertEquals(w.store.done, ["1", "2", "3"]);
    assertEquals(w.store.claims, []);
    // The control: the same sender without a list header goes on to the model.
    w.put(mail({ from: "news@club.test" }));
    await w.run();
    assertEquals(modelledFroms(w), ["news@club.test"]);
  });
});

Deno.test("an_edu_sender_with_list_unsubscribe_passes_on", async () => {
  await withWorld(async (w) => {
    w.put(mail({ from: "Registrar <registrar@univ.edu>", extra: [UNSUB] }));
    w.put(mail({ from: "notices@mail.univ.edu", extra: ["Precedence: bulk"] }));
    // The school's own domain (the account's school mailbox), and a vendor's (a template row's sender).
    w.put(mail({ from: "dean@school.test", extra: [UNSUB] }));
    w.put(mail({ from: "do-not-reply@blackboard.com", subject: "Weekly digest", extra: [UNSUB] }));
    // Not on the list: a personal Gmail domain, and `.edu` anywhere but the end.
    w.put(mail({ from: "promo@gmail.com", extra: [UNSUB] }));
    w.put(mail({ from: "news@edu.club.test", extra: [UNSUB] }));
    w.put(mail({ from: "news@clubedu", extra: [UNSUB] }));
    await w.run();
    assertEquals(modelledFroms(w), [
      "Registrar <registrar@univ.edu>",
      "notices@mail.univ.edu",
      "dean@school.test",
      "do-not-reply@blackboard.com",
    ]);
    assertEquals(w.store.done, ["5", "6", "7"]);
    // The model path holds what it was handed; the noise objects are gone.
    assertEquals(keysUnder(w, "work/"), ["work/item-1", "work/item-2", "work/item-3", "work/item-4"]);
  });
});

Deno.test("a_microsoft_bulk_score_of_seven_or_more_is_noise", async () => {
  await withWorld(async (w) => {
    const edu = "registrar@univ.edu";
    w.put(mail({ from: edu, extra: ["X-Microsoft-Antispam: BCL:7;"] }));
    w.put(mail({ from: edu, extra: ["X-Microsoft-Antispam: BCL:9; ARA:13230040"] }));
    w.put(mail({ from: edu, extra: ["X-Microsoft-Antispam: BCL:6;"] }));
    w.put(mail({ from: edu, extra: ["X-Microsoft-Antispam: ARA:13230040"] }));
    w.put(mail({ from: edu }));
    await w.run();
    assertEquals(w.store.done, ["1", "2"]);
    assertEquals(w.modelled.map((m) => m.item.id), ["3", "4", "5"]);
    assertEquals(keysUnder(w, "work/"), ["work/item-3", "work/item-4", "work/item-5"]);
  });
});

Deno.test("a_promoted_rule_answering_noise_is_noise", async () => {
  await withWorld(async (w) => {
    w.rules.verdicts["Club <news@club.test>"] = { tier: "information", why: "a newsletter" };
    w.rules.verdicts[PROF] = { tier: "task" };
    w.put(mail({ from: "Club <news@club.test>" }));
    w.put(mail({ from: PROF }));
    await w.run();
    assertEquals(w.store.done, ["1"]);
    assertEquals(modelledFroms(w), [PROF]);
    // An email rule, keyed on the sender alone: nothing of the subject or text is offered to the lookup.
    assertEquals(w.rules.asked, [
      ["acct-1", "email", { from: "Club <news@club.test>" }],
      ["acct-1", "email", { from: PROF }],
    ]);
  });
});

Deno.test("the_students_own_mail_outside_a_batch_is_noise", async () => {
  await withWorld(async (w) => {
    w.put(mail({ from: "Jane <jane@gmail.com>" }));
    w.put(mail({ from: "JANE@GMAIL.COM" }));
    // Inside a batch the student's own mail is what she sent us: it goes on.
    w.put(mail({ from: "Jane <jane@gmail.com>" }), { batchId: "b-1", phase: "backfill" });
    await w.run();
    assertEquals(w.store.done, ["1", "2"]);
    assertEquals(w.modelled.map((m) => m.item.id), ["3"]);
  });
});

Deno.test("mail_from_another_of_the_accounts_mailboxes_is_not_screened_as_own_mail", async () => {
  await withWorld(async (w) => {
    // PQ8 (ii) only: the school mailbox's address arriving on Jane's Gmail, and the reverse, pass.
    w.put(mail({ from: "J Doe <jdoe@school.test>" }));
    w.put(mail({ from: "Jane <jane@gmail.com>" }), { addressId: "addr-4" });
    await w.run();
    assertEquals(w.store.done, []);
    assertEquals(w.modelled.map((m) => m.item.id), ["1", "2"]);
  });
});

// ---------------------------------------------------------------- tests 15 and 16: logs and objects

Deno.test("no_process_log_line_carries_the_messages_marker", async () => {
  await withWorld(async (w) => {
    w.rules.verdicts[PROF] = { tier: "information" };
    w.put(mail({ from: `Club ${MARKER} <news@club.test>`, extra: [UNSUB] })); // noise: bulk
    w.put(mail({ from: "registrar@univ.edu", extra: ["X-Microsoft-Antispam: BCL:8;"] })); // noise: score
    w.put(mail({ from: PROF })); // noise: a rule
    w.put(mail({ from: `Jane ${MARKER} <jane@gmail.com>` })); // noise: own mail
    w.put(receipt(`Lab 5 ${MARKER}`)); // template
    w.put(receipt(`Lab 6 for jdoe@school.test ${MARKER}`)); // refused
    w.put(mail({ from: `Dean ${MARKER} <dean@school.test>` }), { authenticity: "unaligned" }); // unauthenticated
    w.put(mail({ from: `TA ${MARKER} <ta@univ.edu>` })); // the model's
    assertEquals((await w.run()).status, 200);
    assert(w.logs.length >= 7, `the paths log their outcome (${w.logs.length} lines)`);
    for (const line of w.logs) {
      assert(!line.includes(MARKER), "a log line carries the message's marker");
      assert(!line.includes("@"), "a log line carries an address");
      assert(!line.includes("work/"), "a log line carries an object key");
      assert(!line.includes(TOKEN), "a log line carries the job token");
    }
  });
});

Deno.test("no_object_is_left_after_noise_refused_template_only_or_unauthenticated", async () => {
  await withWorld(async (w) => {
    w.put(mail({ from: "news@club.test", extra: [UNSUB] })); // noise
    w.put(receipt("Lab 6 for jdoe@school.test")); // refused: its evidence names an address the scrub removes
    const raw = receipt("Lab 5 Report");
    const template = w.put(raw); // template-only
    w.put(mail({ from: PROF }), { authenticity: "unaligned" }); // live and unaligned: unauthenticated
    assertEquals((await w.run()).status, 200);
    assertEquals(keysUnder(w, "work/"), []);
    assertEquals(w.store.done, ["1", "2", "3", "4"]);
    assertEquals(w.modelled, []);
    assertEquals(w.store.counters, { "addr-1:unauthenticated": 1 });
    // The template's claim is queued, with the message's metadata and no text.
    assertEquals(w.store.claims.length, 1);
    const row = w.store.claims[0];
    assertEquals(row, {
      accountId: "acct-1",
      addressId: "addr-1",
      batchId: null,
      uid: await mailUid(parseHeaders(raw)),
      messageDate: "2026-10-02T14:00:00.000Z",
      orderingDate: "2026-10-02T14:00:00.000Z",
      senderName: "Blackboard",
      senderAddr: "do-not-reply@blackboard.com",
      phase: "live",
      authenticity: "aligned",
      claims: row.claims,
      judgmentId: null,
    });
    assertEquals(template.arrivedAt > row.orderingDate, true);
    assertEquals(row.claims.map((c) => [c.kind, c.source, c.target, c.course]), [
      ["receipt", "template", "Lab 5 Report", "xx-101"],
    ]);
    assert(!JSON.stringify(row).includes(MARKER), "the queued row carries text past the claim");
  });
});

Deno.test("a_call_without_the_job_token_is_refused_and_reads_nothing", async () => {
  await withWorld(async (w) => {
    w.put(mail({ from: "news@club.test", extra: [UNSUB] }));
    assertEquals((await w.run("not-the-token-0000000")).status, 401);
    assertEquals((await w.run("")).status, 401);
    assertEquals(w.store.calls, []);
    assertEquals(keysUnder(w, "work/"), ["work/item-1"]);
  });
});
