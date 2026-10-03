// Email plan T4.3 (spec §4.10, D14; test 6's cadence half; P9). The hourly run is driven whole against an
// in-memory store and a spy sender, so nothing here leaves the process. Every address is invented.
import { assert, assertEquals } from "@std/assert";
import {
  type CanaryDeps,
  type CanaryMailbox,
  type CanaryStore,
  handle,
  type JobConfig,
  type Outstanding,
} from "./handler.ts";
import { candidatesQuery } from "./query.ts";

const TOKEN = "test-jobs-token-0000";
const CFG: JobConfig = { jobsToken: TOKEN };
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
/** Wednesday 2026-10-07 15:11 UTC, the cron's minute on a weekday. */
const WED = Date.UTC(2026, 9, 7, 15, 11);
/** Sunday 2026-10-11 15:11 UTC. */
const SUN = Date.UTC(2026, 9, 11, 15, 11);
/** Monday 2026-10-12 10:11 UTC, the morning after a weekend. */
const MON = Date.UTC(2026, 9, 12, 10, 11);
const iso = (ms: number) => new Date(ms).toISOString();

interface Box extends CanaryMailbox {
  lastCanaryOkAt: string | null;
}
interface Canary {
  id: string;
  addressId: string;
  nonceHmac: string;
  sentAt: string;
  arrivedAt: string | null;
}

class FakeStore implements CanaryStore {
  boxes: Box[] = [];
  canaries: Canary[] = [];
  reminded: Record<string, string> = {};
  unentitled = new Set<string>();
  private ids = 0;

  box(id: string): Box {
    const b = this.boxes.find((x) => x.id === id);
    if (b === undefined) throw new Error(`no box ${id}`);
    return b;
  }
  outstanding(before: string): Promise<Outstanding[]> {
    return Promise.resolve(
      this.canaries
        .filter((c) => c.arrivedAt === null && Date.parse(c.sentAt) < Date.parse(before))
        .sort((a, b) => Date.parse(a.sentAt) - Date.parse(b.sentAt))
        .map((c) => {
          const b = this.box(c.addressId);
          return {
            canaryId: c.id,
            addressId: c.addressId,
            sentAt: c.sentAt,
            status: b.status,
            lastCanaryOkAt: b.lastCanaryOkAt,
          };
        }),
    );
  }
  judge(addressId: string, from: readonly string[], to: "quiet" | "blocked", sentAt: string) {
    const b = this.box(addressId);
    if (!from.includes(b.status)) return Promise.resolve(false);
    if (b.lastCanaryOkAt !== null && Date.parse(b.lastCanaryOkAt) >= Date.parse(sentAt)) {
      return Promise.resolve(false);
    }
    b.status = to;
    return Promise.resolve(true);
  }
  /** Every mailbox: a superset of what the real store's filter answers, so the handler's own rules decide. */
  candidates(): Promise<CanaryMailbox[]> {
    return Promise.resolve(this.boxes.map((b) => ({ ...b })));
  }
  accountMail(accountId: string) {
    return Promise.resolve({
      canariesSentAt: this.boxes.filter((b) => b.accountId === accountId).map((b) => b.lastCanarySentAt),
      remindedAt: this.reminded[accountId] ?? null,
    });
  }
  entitled(accountId: string) {
    return Promise.resolve(!this.unentitled.has(accountId));
  }
  takeCanary(
    addressId: string,
    seen: { count: number; lastSentAt: string | null },
    next: { count: number; lastSentAt: string },
  ) {
    const b = this.box(addressId);
    if (
      !["active", "quiet"].includes(b.status) || b.canariesToday !== seen.count ||
      b.lastCanarySentAt !== seen.lastSentAt
    ) return Promise.resolve(false);
    b.canariesToday = next.count;
    b.lastCanarySentAt = next.lastSentAt;
    return Promise.resolve(true);
  }
  insertCanary(addressId: string, nonceHmac: string, sentAt: string) {
    const id = `canary-${++this.ids}`;
    this.canaries.push({ id, addressId, nonceHmac, sentAt, arrivedAt: null });
    return Promise.resolve(id);
  }
  deleteCanary(addressId: string, id: string) {
    this.canaries = this.canaries.filter((c) => !(c.id === id && c.addressId === addressId));
    return Promise.resolve();
  }
}

/** A world at `now` whose mailboxes default to an entitled, active one, its last canary ten days back. */
function world(now: number, boxes: Partial<Box>[]) {
  const store = new FakeStore();
  store.boxes = boxes.map((b, i) => ({
    id: `addr-${i + 1}`,
    accountId: "acct-1",
    status: "active",
    mailboxHint: `student${i + 1}@school.example.edu`,
    lastReceivedAt: null,
    lastCanarySentAt: iso(now - 10 * DAY),
    lastCanaryOkAt: iso(now - 10 * DAY + MINUTE),
    canariesToday: 1,
    ...b,
  }));
  const sent: { to: string; nonce: string }[] = [];
  let nonces = 0;
  const w = {
    store,
    sent,
    refuse: false,
    deps: {
      store,
      now: () => now,
      nonceHmac: (n: string) => Promise.resolve(`h:${n}`),
      newNonce: () => (++nonces).toString(16).padStart(32, "0"),
      sendCanary: (to: string, nonce: string) => {
        if (w.refuse) return Promise.reject(new Error("canary send 500"));
        sent.push({ to, nonce });
        return Promise.resolve();
      },
    } as CanaryDeps,
    async run(token = TOKEN): Promise<Response> {
      const req = new Request("http://127.0.0.1/mail-canary", {
        method: "POST",
        headers: { "content-type": "application/json", "x-knowlu-job-token": token },
        body: "{}",
      });
      return await handle(req, CFG, w.deps);
    },
  };
  return w;
}
/** How many canaries one run at `now` sends to `boxes`. */
async function sends(now: number, boxes: Partial<Box>[]): Promise<number> {
  const w = world(now, boxes);
  assertEquals((await w.run()).status, 200);
  return w.sent.length;
}

Deno.test("no_canary_to_a_mailbox_with_bound_mail_inside_24_hours_on_a_weekday_or_48_over_a_weekend", async () => {
  // A weekday: 23 hours of silence is not quiet, 25 is.
  assertEquals(await sends(WED, [{ lastReceivedAt: iso(WED - 23 * HOUR) }]), 0);
  assertEquals(await sends(WED, [{ lastReceivedAt: iso(WED - 25 * HOUR) }]), 1);
  // Over a weekend (UTC): a silence that touches Saturday or Sunday needs 48 hours.
  assertEquals(await sends(SUN, [{ lastReceivedAt: iso(SUN - 30 * HOUR) }]), 0);
  assertEquals(await sends(MON, [{ lastReceivedAt: iso(MON - 44 * HOUR) }]), 0);
  assertEquals(await sends(MON, [{ lastReceivedAt: iso(MON - 47 * HOUR) }]), 0);
  assertEquals(await sends(MON, [{ lastReceivedAt: iso(MON - 49 * HOUR) }]), 1);
  // A mailbox that has never had bound mail is silent.
  assertEquals(await sends(WED, [{ lastReceivedAt: null }]), 1);
});

Deno.test("no_canary_within_72_hours_of_that_mailboxs_last", async () => {
  const silent = iso(WED - 5 * DAY);
  assertEquals(await sends(WED, [{ lastReceivedAt: silent, lastCanarySentAt: iso(WED - 71 * HOUR) }]), 0);
  assertEquals(await sends(WED, [{ lastReceivedAt: silent, lastCanarySentAt: iso(WED - 73 * HOUR) }]), 1);
  // The limit is per mailbox: another account's mailbox, its last canary long ago, still gets its one.
  const w = world(WED, [
    { lastReceivedAt: silent, lastCanarySentAt: iso(WED - 71 * HOUR) },
    { accountId: "acct-2", lastReceivedAt: silent, lastCanarySentAt: iso(WED - 80 * HOUR) },
  ]);
  await w.run();
  assertEquals(w.sent.map((s) => s.to), ["student2@school.example.edu"]);
});

Deno.test("no_canary_on_a_day_the_student_had_a_knowlu_email", async () => {
  const due = { lastReceivedAt: iso(WED - 5 * DAY) };
  // Another of the account's mailboxes had a canary earlier this UTC day (a setup canary counts).
  const today = Date.UTC(2026, 9, 7, 2, 0);
  assertEquals(
    await sends(WED, [due, { lastReceivedAt: iso(WED - HOUR), lastCanarySentAt: iso(today) }]),
    0,
  );
  // The annual reminder went out this UTC day.
  const reminded = world(WED, [due]);
  reminded.store.reminded["acct-1"] = iso(Date.UTC(2026, 9, 7, 0, 30));
  await reminded.run();
  assertEquals(reminded.sent.length, 0);
  // Yesterday's reminder (UTC) is another day's email.
  const yesterday = world(WED, [due]);
  yesterday.store.reminded["acct-1"] = iso(Date.UTC(2026, 9, 6, 23, 59));
  await yesterday.run();
  assertEquals(yesterday.sent.length, 1);
  // Two of one account's mailboxes due at once: one email that day, then none on a later run.
  const two = world(WED, [due, due]);
  await two.run();
  assertEquals(two.sent.length, 1);
  await two.run();
  assertEquals(two.sent.length, 1);
});

Deno.test("a_mailbox_quiet_past_both_gets_one", async () => {
  const w = world(WED, [{
    lastReceivedAt: iso(WED - 3 * DAY),
    lastCanarySentAt: iso(WED - 4 * DAY),
    canariesToday: 2,
  }]);
  const res = await w.run();
  assertEquals(res.status, 200);
  const text = await res.text();
  assertEquals(JSON.parse(text).sent, 1);
  assert(
    !text.includes("@") && !text.includes(w.sent[0].nonce),
    "the answer carries counts, never an address or a nonce",
  );
  // Sent to the mailbox's own address, kept by the nonce's HMAC, and the day's turn taken.
  assertEquals(w.sent.map((s) => s.to), ["student1@school.example.edu"]);
  assertEquals(w.store.canaries, [{
    id: "canary-1",
    addressId: "addr-1",
    nonceHmac: `h:${w.sent[0].nonce}`,
    sentAt: iso(WED),
    arrivedAt: null,
  }]);
  assertEquals(w.store.boxes[0].lastCanarySentAt, iso(WED));
  assertEquals(w.store.boxes[0].canariesToday, 1, "a new UTC day's count starts again");
  // The next hour's run sends nothing more.
  await w.run();
  assertEquals(w.sent.length, 1);
  // A quiet mailbox is sent one too: its arrival is what turns it active again.
  assertEquals(await sends(WED, [{ status: "quiet", lastReceivedAt: iso(WED - 3 * DAY) }]), 1);
});

Deno.test("a_quiet_mailbox_still_receiving_bound_mail_is_sent_one_72_hours_after_its_last", async () => {
  // Bound mail an hour ago does not clear `quiet` (only a canary's arrival does), so the silence rule
  // would never let one go: a quiet mailbox is sent one every 72 hours until one comes back.
  const busy = { lastReceivedAt: iso(WED - HOUR) };
  assertEquals(await sends(WED, [{ ...busy, status: "quiet", lastCanarySentAt: iso(WED - 73 * HOUR) }]), 1);
  assertEquals(await sends(WED, [{ ...busy, status: "quiet", lastCanarySentAt: iso(WED - 71 * HOUR) }]), 0);
  // An active mailbox with bound mail is still left alone, and P9's one email a day still holds.
  assertEquals(await sends(WED, [{ ...busy, lastCanarySentAt: iso(WED - 73 * HOUR) }]), 0);
  const reminded = world(WED, [{ ...busy, status: "quiet", lastCanarySentAt: iso(WED - 73 * HOUR) }]);
  reminded.store.reminded["acct-1"] = iso(Date.UTC(2026, 9, 7, 0, 30));
  await reminded.run();
  assertEquals(reminded.sent.length, 0);
});

Deno.test("the_stores_candidates_query_lets_a_quiet_mailbox_past_the_silence_rule", () => {
  const q = decodeURIComponent(candidatesQuery("2026-10-04T15:11:00.000Z", "2026-10-06T15:11:00.000Z"));
  assert(q.includes("status=in.(active,quiet)"), q);
  assert(q.includes('or(last_canary_sent_at.is.null,last_canary_sent_at.lt."2026-10-04T15:11:00.000Z")'), q);
  assert(
    q.includes('or(status.eq.quiet,last_received_at.is.null,last_received_at.lt."2026-10-06T15:11:00.000Z")'),
    q,
  );
});

Deno.test("only_an_entitled_active_or_quiet_mailbox_with_its_address_is_sent_one", async () => {
  const due = { lastReceivedAt: iso(WED - 5 * DAY) };
  for (const status of ["pending", "confirmed", "blocked", "off", "paused_send", "lapsed", "revoked"]) {
    assertEquals(await sends(WED, [{ ...due, status }]), 0, status);
  }
  assertEquals(await sends(WED, [{ ...due, mailboxHint: null }]), 0);
  const unpaid = world(WED, [due]);
  unpaid.store.unentitled.add("acct-1");
  await unpaid.run();
  assertEquals(unpaid.sent.length, 0);
  // A refused send takes its row back and keeps the turn spent (as `POST /canary` does).
  const refused = world(WED, [due]);
  refused.refuse = true;
  const res = await refused.run();
  assertEquals(res.status, 200);
  assertEquals((await res.json()).failed, 1);
  assertEquals(refused.store.canaries, []);
  assertEquals(refused.store.boxes[0].lastCanarySentAt, iso(WED));
});

Deno.test("an_unarrived_setup_canary_sets_blocked_and_an_unarrived_periodic_one_sets_quiet", async () => {
  // Every mailbox here had bound mail an hour ago, so the run sends nothing and only judges.
  const busy = { lastReceivedAt: iso(WED - HOUR) };
  const w = world(WED, [
    // Never active (no canary ever arrived): setup canaries, judged once the setup's three minutes pass.
    { ...busy, status: "pending", lastCanaryOkAt: null },
    { ...busy, status: "confirmed", lastCanaryOkAt: null },
    { ...busy, status: "pending", lastCanaryOkAt: null },
    // Active before: periodic canaries, judged after two hours.
    { ...busy, accountId: "acct-2" },
    { ...busy, accountId: "acct-3" },
    // Its missing canary was followed by one that arrived: nothing changes.
    { ...busy, accountId: "acct-4", lastCanaryOkAt: iso(WED - 3 * HOUR) },
    // The device found forwarding off: a missing canary leaves that alone.
    { ...busy, accountId: "acct-5", status: "off" },
    // Arrived: nothing to judge.
    { ...busy, accountId: "acct-6" },
  ]);
  // Each canary is its mailbox's last, as the turn it took wrote `last_canary_sent_at`.
  const canary = (addressId: string, ago: number, arrivedAt: string | null = null) => {
    w.store.canaries.push({
      id: `c-${addressId}`,
      addressId,
      nonceHmac: `h-${addressId}`,
      sentAt: iso(WED - ago),
      arrivedAt,
    });
    w.store.box(addressId).lastCanarySentAt = iso(WED - ago);
  };
  canary("addr-1", 4 * MINUTE);
  canary("addr-2", 50 * MINUTE);
  canary("addr-3", 2 * MINUTE);
  canary("addr-4", 121 * MINUTE);
  canary("addr-5", 119 * MINUTE);
  canary("addr-6", 4 * HOUR);
  canary("addr-7", 4 * HOUR);
  canary("addr-8", 4 * HOUR, iso(WED - 4 * HOUR + MINUTE));
  const res = await w.run();
  assertEquals(res.status, 200);
  const counts = await res.json();
  assertEquals([counts.blocked, counts.quiet, counts.sent], [2, 1, 0]);
  assertEquals(
    w.store.boxes.map((b) => b.status),
    ["blocked", "blocked", "pending", "quiet", "active", "active", "off", "active"],
  );
  // A periodic canary not yet two hours old is judged on a later run; a judged mailbox is not judged twice.
  w.deps = { ...w.deps, now: () => WED + HOUR };
  assertEquals((await (await w.run()).json()).quiet, 1);
  assertEquals(w.store.boxes[4].status, "quiet");
  assertEquals(w.store.boxes[3].status, "quiet");
});

Deno.test("the_run_answers_401_without_the_job_token_and_takes_only_post", async () => {
  const w = world(WED, [{ lastReceivedAt: null }]);
  assertEquals((await w.run("not-the-token-0000000")).status, 401);
  assertEquals((await w.run("")).status, 401);
  const get = await handle(new Request("http://127.0.0.1/mail-canary", { method: "GET" }), CFG, w.deps);
  assertEquals(get.status, 405);
  assertEquals(w.sent.length, 0);
});
