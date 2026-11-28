import { assert, assertEquals } from "@std/assert";
import { type Deps, handle, type Mail, reminderDue, type Subscriber } from "./handler.ts";
import {
  type CatalogDef,
  type KeptEntry,
  priceVariable,
  resolveCatalog,
  type ResolvedCatalog,
  TERMS_VERSION,
  WORDS,
} from "../_shared/price_catalog.ts";
import { chargeDate } from "../stripe-webhook/handler.ts";

// Every test builds its own catalog from a table here, through the catalog's resolve helper (note
// D13); none reads the real `SELLING`, so a price change edits no test in this file.
const SELLING_TABLE: CatalogDef["selling"] = [{ key: "yearly", cents: 8437 }, {
  key: "four_weekly",
  cents: 1299,
}];

/** Each entry's derived variable holds an invented id that names its key and cents. */
const idOf = (key: string, cents: number) => `price_test_${key}_${cents}`;

function catalogOf(kept: KeptEntry[] = [], selling: CatalogDef["selling"] = SELLING_TABLE): ResolvedCatalog {
  const def: CatalogDef = { termsVersion: TERMS_VERSION, selling, kept, words: WORDS };
  const env = new Map([...selling, ...kept].map((e) => [priceVariable(e), idOf(e.key, e.cents)]));
  return resolveCatalog(def, (name) => env.get(name));
}

const DAY = 24 * 60 * 60;
const NOW = new Date("2027-06-02T12:00:00Z");
const NOW_S = NOW.getTime() / 1000;
const iso = (seconds: number) => new Date(seconds * 1000).toISOString();

/** A row of `billing_subscribers`. Not due an annual reminder unless `started_at` says so. */
function row(n: number, over: Partial<Subscriber> = {}): Subscriber {
  return {
    account_id: `acc-${n}`,
    email: `s${n}@example.invalid`,
    subscription_id: `sub_${n}`,
    plan: "four_weekly",
    paused: false,
    started_at: iso(NOW_S - 30 * DAY),
    last_reminded_at: null,
    current_period_end: null,
    ...over,
  };
}
const DUE = iso(NOW_S - 400 * DAY);

/** The Subscription as Stripe hands it back at the pinned version (the period end on the item). */
function stripeSub(
  n: number,
  a: {
    price: string;
    periodEnd?: number;
    status?: string;
    cancel?: boolean;
    metadata?: Record<string, string>;
  },
) {
  return {
    id: `sub_${n}`,
    status: a.status ?? "active",
    cancel_at_period_end: a.cancel ?? false,
    metadata: { account_id: `acc-${n}`, ...a.metadata },
    items: {
      data: [{
        id: `si_${n}`,
        current_period_end: a.periodEnd ?? NOW_S + 200 * DAY,
        price: { id: a.price },
      }],
    },
  };
}

interface Seen {
  /** Everything in order: `read:<id>`, `mail:<to>`, `post:<path> <form keys>`. */
  events: string[];
  mails: Mail[];
  posts: [string, Record<string, string>][];
  recorded: string[];
}

function fakes(
  rows: Subscriber[],
  subs: Record<string, unknown>,
  over: Partial<Deps> = {},
): { deps: Deps; seen: Seen } {
  const seen: Seen = { events: [], mails: [], posts: [], recorded: [] };
  const deps: Deps = {
    token: "a-job-token",
    now: () => NOW,
    listSubscribers: () => Promise.resolve(rows),
    getSubscription: (id) => (seen.events.push(`read:${id}`), Promise.resolve((subs[id] ?? null) as never)),
    stripe: (path, form) => {
      seen.events.push(`post:${path} ${Object.keys(form).sort().join(",")}`);
      seen.posts.push([path, form]);
      return Promise.resolve({ id: "sub" });
    },
    sendEmail: (m) => (seen.events.push(`mail:${m.to}`), seen.mails.push(m), Promise.resolve()),
    recordReminder: (id) => (seen.recorded.push(id), Promise.resolve()),
    catalog: catalogOf(),
    ...over,
  };
  return { deps, seen };
}

const jobReq = () =>
  new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } });

const silently = async <T>(f: () => Promise<T>): Promise<T> => {
  const original = console.error;
  console.error = () => {};
  try {
    return await f();
  } finally {
    console.error = original;
  }
};

Deno.test("an annual reminder is due a year after the last one, or a year after the start", () => {
  // California's ARL wants an annual reminder for a monthly subscription, stating the product, the
  // amount, the cadence and how to cancel.
  assert(
    reminderDue({
      lastSentAt: null,
      startedAt: "2026-09-10T00:00:00Z",
      now: new Date("2027-09-10T00:00:01Z"),
    }),
  );
  assert(
    !reminderDue({
      lastSentAt: null,
      startedAt: "2026-09-10T00:00:00Z",
      now: new Date("2027-09-09T00:00:00Z"),
    }),
  );
  assert(
    !reminderDue({
      lastSentAt: "2027-09-10T00:00:00Z",
      startedAt: "2026-09-10T00:00:00Z",
      now: new Date("2027-10-10T00:00:00Z"),
    }),
  );
  assert(
    reminderDue({
      lastSentAt: "2027-09-10T00:00:00Z",
      startedAt: "2026-09-10T00:00:00Z",
      now: new Date("2028-09-11T00:00:00Z"),
    }),
  );
});

Deno.test("the job refuses a caller with no job token, and writes nothing", async () => {
  let touched = false;
  const { deps } = fakes([], {}, {
    listSubscribers: () => {
      touched = true;
      return Promise.resolve([]);
    },
  });
  const res = await handle(new Request("http://127.0.0.1:1/", { method: "POST" }), deps);
  assertEquals(res.status, 401);
  assertEquals(await res.json(), { error: "not a job caller" });
  assert(!touched);
});

Deno.test("no_subscriber_is_ever_paused", async () => {
  const fw = idOf("four_weekly", 1299), yr = idOf("yearly", 8437);
  const { deps, seen } = fakes(
    [row(1, { plan: "four_weekly", started_at: DUE }), row(2, { plan: "yearly", started_at: DUE })],
    { sub_1: stripeSub(1, { price: fw }), sub_2: stripeSub(2, { price: yr }) },
  );
  const res = await handle(jobReq(), deps);
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(body, { reminded: 2, moved: 0, failed: 0 });
  assert(!("paused" in body) && !("resumed" in body));
  assertEquals(seen.posts, [], "no Stripe write at all, so no pause_collection");
});

Deno.test("one subscriber's failure does not stop the run", async () => {
  const fw = idOf("four_weekly", 1299);
  const { deps, seen } = fakes(
    [row(1, { started_at: DUE }), row(2, { started_at: DUE })],
    { sub_1: stripeSub(1, { price: fw }), sub_2: stripeSub(2, { price: fw }) },
    {
      sendEmail: (m) =>
        m.to === "s1@example.invalid" ? Promise.reject(new Error("smtp down")) : Promise.resolve(),
    },
  );
  const res = await silently(() => handle(jobReq(), deps));
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { reminded: 1, moved: 0, failed: 1 });
  assertEquals(seen.recorded, ["acc-2"]);
});

Deno.test("the_annual_reminder_states_the_subscribers_own_price", async () => {
  // The subscription is read, and its price id looked up: with an invented catalog entry at 1399 for
  // that id, the mail says so.
  const id = idOf("four_weekly", 1299);
  const invented = resolveCatalog(
    {
      termsVersion: TERMS_VERSION,
      selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1399 }],
      kept: [],
      words: WORDS,
    },
    (name) => (name === priceVariable({ key: "four_weekly", cents: 1399 }) ? id : undefined),
  );
  {
    const { deps, seen } = fakes([row(1, { started_at: DUE })], { sub_1: stripeSub(1, { price: id }) }, {
      catalog: invented,
    });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 1, moved: 0, failed: 0 });
    assertEquals(
      seen.events.filter((e) => e.startsWith("read:")),
      ["read:sub_1"],
      "the subscription is read",
    );
    assertEquals(seen.mails.length, 1);
    const m = seen.mails[0];
    assertEquals(m.to, "s1@example.invalid");
    assertEquals(m.subject, "Your Knowlu subscription");
    assert(m.text.includes("$13.99 every 4 weeks, plus tax where it applies"), m.text);
    assert(!m.text.includes("per month") && !m.text.includes("a month"), m.text);
    assert(m.text.includes("Knowlu"), "the product name is in the annual reminder");
    assert(m.text.includes("cancel"), "and so is how to cancel");
    // R-C1-59 (I3): California's ARL notice must name the button the app actually has, never one it
    // does not.
    assert(m.text.includes("Manage subscription"), "the annual reminder must name the real button");
    assert(!m.text.includes("Cancel subscription"), "and never the button the app does not have");
    assertEquals(seen.recorded, ["acc-1"]);
  }
  {
    const { deps, seen } = fakes([row(1, { plan: "yearly", started_at: DUE })], {
      sub_1: stripeSub(1, { price: idOf("yearly", 8437) }),
    });
    await handle(jobReq(), deps);
    assert(seen.mails[0].text.includes("$84.37 once a year, plus tax where it applies"), seen.mails[0].text);
  }
});

Deno.test("a_kept_price_is_reminded_at_its_own_amount", async () => {
  const catalog = catalogOf([{ key: "four_weekly", cents: 999, rule: "stay" }]);
  const { deps, seen } = fakes([row(1, { started_at: DUE })], {
    sub_1: stripeSub(1, { price: idOf("four_weekly", 999) }),
  }, { catalog });
  const res = await handle(jobReq(), deps);
  assertEquals(await res.json(), { reminded: 1, moved: 0, failed: 0 });
  assert(seen.mails[0].text.includes("$9.99 every 4 weeks"), seen.mails[0].text);
  assert(!seen.mails[0].text.includes("$12.99"), "never the selling price");
  assertEquals(seen.posts, [], "a stay entry is never moved");
});

Deno.test("an_unknown_price_is_never_mailed", async () => {
  const { deps, seen } = fakes(
    [row(1, { started_at: DUE }), row(2, { started_at: DUE }), row(3, { started_at: DUE })],
    {
      sub_1: stripeSub(1, { price: "price_the_catalog_lacks" }),
      // sub_2: the read fails (Stripe answers non-2xx, `stripeGetFrom` gives null).
      sub_3: stripeSub(3, { price: idOf("four_weekly", 1299) }),
    },
  );
  const logged: string[] = [];
  const original = console.error;
  console.error = (...a: unknown[]) => void logged.push(a.map(String).join(" "));
  try {
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 1, moved: 0, failed: 2 });
  } finally {
    console.error = original;
  }
  assertEquals(seen.mails.map((m) => m.to), ["s3@example.invalid"], "only the known price is mailed");
  assertEquals(seen.recorded, ["acc-3"], "and the next subscriber is still reached");
  assert(
    logged.some((l) => l.includes("acc-1")) && logged.some((l) => l.includes("acc-2")),
    logged.join("\n"),
  );
  for (const l of logged) assert(!l.includes("@"), `logs name the account id only: ${l}`);
});

Deno.test("the_cancel_path_names_settings_and_membership", async () => {
  const { deps, seen } = fakes([row(1, { started_at: DUE })], {
    sub_1: stripeSub(1, { price: idOf("four_weekly", 1299) }),
  });
  await handle(jobReq(), deps);
  const text = seen.mails[0].text;
  for (const w of ["Settings", "Membership", "Manage subscription"]) {
    assert(text.includes(w), `${w}: ${text}`);
  }
  assert(!text.includes("gear"), text);
});

// ---------------------------------------------------------------------------------------------
// The move (note D14): a KEPT entry at `move` N = 10, `now` fixed.

const OLD = idOf("four_weekly", 999);
const NEW = idOf("four_weekly", 1299);
const MOVE10 = () => catalogOf([{ key: "four_weekly", cents: 999, rule: "move", noticeDays: 10 }]);

/** A subscriber on the old price whose renewal (in the view and at Stripe) is `days` away. */
function onOld(n: number, days: number, sub: Partial<Parameters<typeof stripeSub>[1]> = {}, view?: number) {
  const end = NOW_S + days * DAY;
  return {
    row: row(n, { current_period_end: iso(view === undefined ? end : NOW_S + view * DAY) }),
    sub: stripeSub(n, { price: OLD, periodEnd: end, ...sub }),
  };
}

const moveCalls = (n: number) => [
  `mail:s${n}@example.invalid`,
  `post:/v1/subscriptions/sub_${n} metadata[knowlu_price_notice]`,
  `post:/v1/subscriptions/sub_${n} items[0][id],items[0][price],proration_behavior`,
];

Deno.test("a_move_notices_then_marks_then_changes", async () => {
  const s = onOld(1, 12);
  const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
  const res = await handle(jobReq(), deps);
  assertEquals(await res.json(), { reminded: 0, moved: 1, failed: 0 });
  assertEquals(seen.events, ["read:sub_1", ...moveCalls(1)], "notice, then mark, then change, in that order");
  const m = seen.mails[0];
  assertEquals(m.to, "s1@example.invalid");
  assert(m.text.includes("$9.99 every 4 weeks"), `the old amount: ${m.text}`);
  assert(m.text.includes("$12.99 every 4 weeks"), `the new amount: ${m.text}`);
  assert(m.text.includes(chargeDate(NOW_S + 12 * DAY)), `the UTC-10 date: ${m.text}`);
  assert(m.text.includes("plus tax where it applies"), m.text);
  assert(m.text.includes("Manage subscription"), m.text);
  assert(!m.text.includes("per month") && !m.text.includes("a month"), m.text);
  // (r5: m6) The mark is the only key written: `metadata[account_id]` is never touched.
  const [markPath, mark] = seen.posts[0];
  assertEquals(markPath, "/v1/subscriptions/sub_1");
  assertEquals(Object.keys(mark), ["metadata[knowlu_price_notice]"]);
  assert(
    mark["metadata[knowlu_price_notice]"].includes(String(NOW_S + 12 * DAY)),
    "the mark names this renewal",
  );
  const [changePath, change] = seen.posts[1];
  assertEquals(changePath, "/v1/subscriptions/sub_1");
  assertEquals(change, { "items[0][id]": "si_1", "items[0][price]": NEW, "proration_behavior": "none" });
  assertEquals(seen.recorded, [], "a price notice is not the annual reminder");
});

Deno.test("the_window_is_exact", async () => {
  for (
    const [name, seconds, moved] of [
      ["exactly 10 x 24h ahead", 10 * DAY, true],
      ["10 x 24h less one second", 10 * DAY - 1, false],
      ["exactly 16 x 24h ahead", 16 * DAY, false],
      ["16 x 24h less one second", 16 * DAY - 1, true],
    ] as const
  ) {
    const end = NOW_S + seconds;
    const { deps, seen } = fakes(
      [row(1, { current_period_end: iso(end) })],
      { sub_1: stripeSub(1, { price: OLD, periodEnd: end }) },
      { catalog: MOVE10() },
    );
    const res = await handle(jobReq(), deps);
    assertEquals((await res.json()).moved, moved ? 1 : 0, name);
    assertEquals(seen.mails.length, moved ? 1 : 0, name);
    assertEquals(seen.posts.length, moved ? 2 : 0, name);
  }
});

Deno.test("the_window_and_the_date_are_stripes", async () => {
  // The view says 30 days, Stripe says 12. The view alone would not choose this subscriber, but it is
  // read for its annual reminder, and then the decision and the date follow Stripe's value.
  {
    const s = onOld(1, 12, {}, 30);
    s.row.started_at = DUE;
    const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 1, moved: 1, failed: 0 });
    assert(seen.mails[0].text.includes(chargeDate(NOW_S + 12 * DAY)), seen.mails[0].text);
    assert(!seen.mails[0].text.includes(chargeDate(NOW_S + 30 * DAY)), "never the view's date");
  }
  // The reverse: the view says 11 days (inside its one-day margin, so it is read), Stripe says 30.
  {
    const s = onOld(1, 30, {}, 11);
    const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 });
    assertEquals(seen.events, ["read:sub_1"], "read, and nothing else");
  }
  // The margin itself: a view copy 9 days out is read (one day either side of the 10-to-16 window).
  {
    const s = onOld(1, 12, {}, 9);
    const { deps } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    assertEquals((await (await handle(jobReq(), deps)).json()).moved, 1);
  }
});

Deno.test("no_move_outside_the_window", async () => {
  for (const days of [9, 17]) {
    const s = onOld(1, days);
    const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 }, `${days} days`);
    assertEquals(seen.mails, [], `${days} days`);
    assertEquals(seen.posts, [], `${days} days`);
  }
  // Another price, and a `stay` entry, inside the window: nothing sent, nothing written.
  for (
    const [name, catalog, price] of [
      ["the selling price", MOVE10(), NEW],
      ["a stay entry", catalogOf([{ key: "four_weekly", cents: 999, rule: "stay" }]), OLD],
    ] as const
  ) {
    const end = NOW_S + 12 * DAY;
    const { deps, seen } = fakes(
      [row(1, { current_period_end: iso(end) })],
      { sub_1: stripeSub(1, { price, periodEnd: end }) },
      { catalog },
    );
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 }, name);
    assertEquals(seen.mails, [], name);
    assertEquals(seen.posts, [], name);
  }
  // (r5: m10) Set to cancel at period end, read from Stripe: no mail and no write; the read is the
  // one call.
  {
    const s = onOld(1, 12, { cancel: true });
    const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 });
    assertEquals(seen.events, ["read:sub_1"]);
  }
});

Deno.test("a_raise_leaves_a_trials_first_charge", async () => {
  // Selling 1299 is above the kept entry's 999: a raise.
  {
    const s = onOld(1, 12, { status: "trialing" });
    const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 });
    assertEquals(seen.mails, [], "no mail before the charge that ends the trial");
    assertEquals(seen.posts, [], "and no write");
  }
  {
    const s = onOld(1, 12, { status: "active" });
    const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 1, failed: 0 });
    assertEquals(seen.events, ["read:sub_1", ...moveCalls(1)], "once active, it is moved");
  }
});

Deno.test("the_raise_rule_skips_a_trial_before_the_selling_price_is_needed", async () => {
  // A raise whose new selling variable is not set yet: a trialing subscriber in the window is skipped
  // by the raise rule, so it is neither mailed, written nor counted as failed on any day of the window.
  const def: CatalogDef = {
    termsVersion: TERMS_VERSION,
    selling: SELLING_TABLE,
    kept: [{ key: "four_weekly", cents: 999, rule: "move", noticeDays: 10 }],
    words: WORDS,
  };
  const unsetSelling = priceVariable({ key: "four_weekly", cents: 1299 });
  const catalog = resolveCatalog(
    def,
    (
      name,
    ) => (name === unsetSelling
      ? undefined
      : name === priceVariable({ key: "four_weekly", cents: 999 })
      ? OLD
      : idOf("yearly", 8437)),
  );
  assertEquals(catalog.missing, [unsetSelling]);
  const s = onOld(1, 12, { status: "trialing" });
  const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog });
  const res = await silently(() => handle(jobReq(), deps));
  assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 });
  assertEquals(seen.mails, []);
  assertEquals(seen.posts, []);
});

Deno.test("a_cut_moves_a_trial", async () => {
  // Selling 1299 is below the kept entry's 1499: a cut, so the trial is moved like any other.
  const catalog = catalogOf([{ key: "four_weekly", cents: 1499, rule: "move", noticeDays: 10 }]);
  const end = NOW_S + 12 * DAY;
  const { deps, seen } = fakes(
    [row(1, { current_period_end: iso(end) })],
    { sub_1: stripeSub(1, { price: idOf("four_weekly", 1499), periodEnd: end, status: "trialing" }) },
    { catalog },
  );
  const res = await handle(jobReq(), deps);
  assertEquals(await res.json(), { reminded: 0, moved: 1, failed: 0 });
  assertEquals(seen.events, ["read:sub_1", ...moveCalls(1)]);
  assert(
    seen.mails[0].text.includes("$14.99 every 4 weeks") &&
      seen.mails[0].text.includes("$12.99 every 4 weeks"),
  );
  assertEquals(seen.posts[1][1]["items[0][price]"], NEW);
});

Deno.test("a_yearly_subscriber_is_moved_before_the_yearly_renewal", async () => {
  // A yearly kept entry (7787 is 13 x 599) at move N = 20.
  const catalog = catalogOf([{ key: "yearly", cents: 7787, rule: "move", noticeDays: 20 }]);
  const old = idOf("yearly", 7787);
  {
    // Started a year ago less 22 days, so its renewal is 22 days out and no annual reminder is due.
    const end = NOW_S + 22 * DAY;
    const { deps, seen } = fakes(
      [row(1, { plan: "yearly", started_at: iso(end - 365 * DAY), current_period_end: iso(end) })],
      { sub_1: stripeSub(1, { price: old, periodEnd: end }) },
      { catalog },
    );
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 1, failed: 0 });
    assertEquals(seen.events, ["read:sub_1", ...moveCalls(1)]);
    assert(
      seen.mails[0].text.includes("$77.87 once a year") && seen.mails[0].text.includes("$84.37 once a year"),
    );
    assertEquals(seen.posts[1][1]["items[0][price]"], idOf("yearly", 8437), "the yearly selling id");
  }
  {
    const end = NOW_S + 40 * DAY;
    const { deps, seen } = fakes(
      [row(1, { plan: "yearly", started_at: iso(end - 365 * DAY), current_period_end: iso(end) })],
      { sub_1: stripeSub(1, { price: old, periodEnd: end }) },
      { catalog },
    );
    const res = await handle(jobReq(), deps);
    assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 });
    assertEquals(seen.events, [], "40 days out is not read, so not moved");
  }
});

Deno.test("a_failed_notice_changes_nothing", async () => {
  const s = onOld(1, 12);
  const { deps, seen } = fakes([s.row], { sub_1: s.sub }, {
    catalog: MOVE10(),
    sendEmail: () => Promise.reject(new Error("smtp down")),
  });
  const res = await silently(() => handle(jobReq(), deps));
  assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 1 });
  assertEquals(seen.posts, [], "no metadata write and no price change");
});

Deno.test("a_noticed_subscription_is_changed_without_a_second_notice", async () => {
  // Day one: the notice goes and the mark is written, then the price change fails.
  const s = onOld(1, 12);
  const first = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
  first.deps.stripe = (path, form) => {
    first.seen.posts.push([path, form]);
    return "proration_behavior" in form ? Promise.reject(new Error("stripe down")) : Promise.resolve({});
  };
  assertEquals(await (await silently(() => handle(jobReq(), first.deps))).json(), {
    reminded: 0,
    moved: 0,
    failed: 1,
  });
  assertEquals(first.seen.mails.length, 1);
  const mark = first.seen.posts[0][1]["metadata[knowlu_price_notice]"];
  assert(mark, "the mark was written before the change was tried");
  // Day two: the metadata names this renewal, so the change is made and no second notice goes.
  const noticed = stripeSub(1, {
    price: OLD,
    periodEnd: NOW_S + 12 * DAY,
    metadata: { knowlu_price_notice: mark },
  });
  const second = fakes([s.row], { sub_1: noticed }, { catalog: MOVE10() });
  const res = await handle(jobReq(), second.deps);
  assertEquals(await res.json(), { reminded: 0, moved: 1, failed: 0 });
  assertEquals(second.seen.mails, [], "no second notice");
  assertEquals(second.seen.events, ["read:sub_1", moveCalls(1)[2]], "only the change");
  // A mark for another renewal is not this renewal's: that one is noticed again.
  const stale = stripeSub(1, {
    price: OLD,
    periodEnd: NOW_S + 12 * DAY,
    metadata: { knowlu_price_notice: mark.replace(String(NOW_S + 12 * DAY), String(NOW_S - 16 * DAY)) },
  });
  const third = fakes([s.row], { sub_1: stale }, { catalog: MOVE10() });
  await handle(jobReq(), third.deps);
  assertEquals(third.seen.events, ["read:sub_1", ...moveCalls(1)]);
});

Deno.test("one_move_failure_does_not_stop_the_run", async () => {
  const a = onOld(1, 12), b = onOld(2, 12);
  const { deps, seen } = fakes([a.row, b.row], { sub_1: a.sub, sub_2: b.sub }, { catalog: MOVE10() });
  deps.stripe = (path, form) => {
    seen.events.push(`post:${path} ${Object.keys(form).sort().join(",")}`);
    return path === "/v1/subscriptions/sub_1" && "proration_behavior" in form
      ? Promise.reject(new Error("stripe down"))
      : Promise.resolve({});
  };
  const res = await silently(() => handle(jobReq(), deps));
  assertEquals(await res.json(), { reminded: 0, moved: 1, failed: 1 });
  assertEquals(seen.events.slice(-4), ["read:sub_2", ...moveCalls(2)], "the next subscriber is still moved");
});

Deno.test("the_move_comes_before_the_annual_reminder", async () => {
  const s = onOld(1, 12);
  s.row.started_at = DUE;
  const { deps, seen } = fakes([s.row], { sub_1: s.sub }, { catalog: MOVE10() });
  const res = await handle(jobReq(), deps);
  assertEquals(await res.json(), { reminded: 1, moved: 1, failed: 0 });
  assertEquals(seen.mails.length, 2);
  assert(seen.mails[0].text.includes("$9.99 every 4 weeks"), "the notice first");
  assertEquals(seen.mails[1].subject, "Your Knowlu subscription");
  assert(
    seen.mails[1].text.includes("$12.99 every 4 weeks"),
    `the reminder states the new amount: ${seen.mails[1].text}`,
  );
  assert(!seen.mails[1].text.includes("$9.99"), seen.mails[1].text);
  assertEquals(
    seen.events.indexOf(moveCalls(1)[2]) < seen.events.lastIndexOf("mail:s1@example.invalid"),
    true,
  );
});

Deno.test("an unset kept variable never answers 500 for the run", async () => {
  // A KEPT entry whose variable is unset matches no id: its subscribers are reminded as unknown prices
  // (not mailed, counted), and the run goes on.
  const def: CatalogDef = {
    termsVersion: TERMS_VERSION,
    selling: SELLING_TABLE,
    kept: [{ key: "four_weekly", cents: 999, rule: "move", noticeDays: 10 }],
    words: WORDS,
  };
  const catalog = resolveCatalog(
    def,
    (name) => (name === priceVariable({ key: "four_weekly", cents: 999 }) ? undefined : idOf("x", 1)),
  );
  const s = onOld(1, 12);
  const { deps } = fakes([s.row], { sub_1: s.sub }, { catalog });
  const res = await handle(jobReq(), deps);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { reminded: 0, moved: 0, failed: 0 });
});

Deno.test("index.ts resolves every price variable as optional, never through the throwing env", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(/from "\.\.\/_shared\/price_catalog\.ts"/.test(text), "index.ts imports the catalog");
  assert(
    /resolveCatalog\(\s*CATALOG,\s*\(name\)\s*=>\s*Deno\.env\.get\(name\)\s*\)/.test(text),
    "the catalog is resolved through Deno.env.get, every variable optional",
  );
  assert(!/resolveCatalog\([^)]*\benv\(/.test(text), "never through the file's throwing env()");
  assert(!/STRIPE_PRICE_/.test(text), "no price variable is named here");
  assert(/\bcatalog[,:]/.test(text), "the catalog is handed to the handler");
  assert(
    /getSubscription: \(id\) =>\s*stripeGetFrom\(/.test(text),
    "the subscription read goes through stripeGetFrom",
  );
  assert(text.includes("`/v1/subscriptions/${encodeURIComponent(id)}`"), "one subscription, path-encoded");
});
