import { assert, assertEquals } from "@std/assert";
import { handle, Mail, pauseDecision, reminderDue, shouldBePaused } from "./handler.ts";

Deno.test("June, July and August are the paused months, and nothing else is", () => {
  assertEquals(shouldBePaused(new Date("2027-05-31T23:59:59Z")), false);
  assertEquals(shouldBePaused(new Date("2027-06-01T00:00:00Z")), true);
  assertEquals(shouldBePaused(new Date("2027-07-15T00:00:00Z")), true);
  assertEquals(shouldBePaused(new Date("2027-08-31T23:59:59Z")), true);
  assertEquals(shouldBePaused(new Date("2027-09-01T00:00:00Z")), false);
});

Deno.test("the job only ever acts on the edge, never every day", () => {
  assertEquals(pauseDecision(new Date("2027-06-01T00:00:00Z"), false), "pause");
  assertEquals(pauseDecision(new Date("2027-07-01T00:00:00Z"), true), "none");
  assertEquals(pauseDecision(new Date("2027-09-01T00:00:00Z"), true), "resume");
  assertEquals(pauseDecision(new Date("2027-10-01T00:00:00Z"), false), "none");
});

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
  const res = await handle(new Request("http://127.0.0.1:1/", { method: "POST" }), {
    token: "a-job-token",
    now: () => new Date("2027-06-01T00:00:00Z"),
    listSubscribers: () => {
      touched = true;
      return Promise.resolve([]);
    },
    stripe: () => Promise.resolve({}),
    sendEmail: () => Promise.resolve(),
    recordReminder: () => Promise.resolve(),
  });
  assertEquals(res.status, 401);
  assertEquals(await res.json(), { error: "not a job caller" });
  assert(!touched);
});

Deno.test("on 1 June every MONTHLY subscription is paused, and the yearly one is not", async () => {
  const calls: [string, Record<string, string>][] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-06-01T03:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: "monthly",
            paused: false,
            started_at: "2026-09-10T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
          {
            account_id: "acc-2",
            email: "b@example.invalid",
            subscription_id: "sub_2",
            plan: "academic_year",
            paused: false,
            started_at: "2026-09-10T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
        ]),
      stripe: (path, form) => {
        calls.push([path, form]);
        return Promise.resolve({ id: "sub_1" });
      },
      sendEmail: () => Promise.resolve(),
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  // One, not two: the academic-year subscriber is exempt, because that price already covers the
  // summer and voiding its renewal invoice would give away a year (R3).
  assertEquals(await res.json(), { paused: 1, resumed: 0, reminded: 0, failed: 0 });
  assertEquals(calls.map((c) => c[0]), ["/v1/subscriptions/sub_1"]);
  assertEquals(calls[0][1], { "pause_collection[behavior]": "void" });
});

Deno.test("on 1 September everything resumes and every resumed student is told before the charge", async () => {
  const mails: Mail[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-09-01T03:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: "monthly",
            paused: true,
            started_at: "2026-09-10T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
        ]),
      stripe: () => Promise.resolve({ id: "sub_1" }),
      sendEmail: (m) => {
        mails.push(m);
        return Promise.resolve();
      },
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 1, reminded: 1, failed: 0 });
  assertEquals(mails.length, 1);
  assertEquals(mails[0].to, "a@example.invalid");
  assert(mails[0].subject.includes("Knowlu"));
  assert(mails[0].text.includes("$9.99"), "the amount is in the resume notice");
  assert(mails[0].text.includes("cancel"), "and so is how to cancel");
  // R-C1-59 (I3): the mail must name the button the app actually has (`app/static/index.html`,
  // `site/terms.html`), never the one it does not.
  assert(mails[0].text.includes("Manage subscription"), "the resume notice must name the real button");
  assert(!mails[0].text.includes("Cancel subscription"), "and never the button the app does not have");
});

Deno.test("if the resume notice fails to send, Stripe is never called and the run continues to the next subscriber", async () => {
  const stripeCalls: string[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-09-01T03:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: "monthly",
            paused: true,
            started_at: "2026-09-10T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
          {
            account_id: "acc-2",
            email: "b@example.invalid",
            subscription_id: "sub_2",
            plan: "monthly",
            paused: true,
            started_at: "2026-09-10T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
        ]),
      stripe: (path) => {
        stripeCalls.push(path);
        return Promise.resolve({ id: "sub" });
      },
      sendEmail: (m) =>
        m.to === "a@example.invalid" ? Promise.reject(new Error("smtp down")) : Promise.resolve(),
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 1, reminded: 1, failed: 1 });
  // acc-1's Stripe resume must never fire: the mail came first, and it failed.
  assertEquals(stripeCalls, ["/v1/subscriptions/sub_2"]);
});

Deno.test("a resume within 14 days of the last reminder skips the notice but still resumes", async () => {
  let mailed = false;
  let recorded = false;
  const stripeCalls: string[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-09-01T03:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: "monthly",
            paused: true,
            started_at: "2026-09-10T00:00:00Z",
            // Two days ago: within the 14-day window, so no second mail — but the resume still
            // proceeds, because a Stripe failure yesterday must not block a retry today.
            last_reminded_at: "2027-08-30T03:00:00Z",
            current_period_end: null,
          },
        ]),
      stripe: (path) => {
        stripeCalls.push(path);
        return Promise.resolve({ id: "sub_1" });
      },
      sendEmail: () => {
        mailed = true;
        return Promise.resolve();
      },
      recordReminder: () => {
        recorded = true;
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 1, reminded: 0, failed: 0 });
  assertEquals(stripeCalls, ["/v1/subscriptions/sub_1"]);
  assert(!mailed, "no second mail inside the 14-day window");
  assert(!recorded, "no second record inside the 14-day window");
});

Deno.test("one subscriber's failure does not stop the run", async () => {
  const remindedFor: string[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-06-15T00:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: "academic_year",
            paused: false,
            started_at: "2025-01-01T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
          {
            account_id: "acc-2",
            email: "b@example.invalid",
            subscription_id: "sub_2",
            plan: "academic_year",
            paused: false,
            started_at: "2025-01-01T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
        ]),
      stripe: () => Promise.resolve({}),
      sendEmail: (m) =>
        m.to === "a@example.invalid" ? Promise.reject(new Error("smtp down")) : Promise.resolve(),
      recordReminder: (accountId) => {
        remindedFor.push(accountId);
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 0, reminded: 1, failed: 1 });
  assertEquals(remindedFor, ["acc-2"]);
});

Deno.test("the annual reminder is driven through handle for a subscriber over a year old", async () => {
  const mails: Mail[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-06-15T00:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: "academic_year",
            paused: false,
            started_at: "2025-01-01T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
        ]),
      stripe: () => Promise.resolve({}),
      sendEmail: (m) => {
        mails.push(m);
        return Promise.resolve();
      },
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 0, reminded: 1, failed: 0 });
  assertEquals(mails.length, 1);
  assertEquals(mails[0].subject, "Your Knowlu subscription");
  assert(mails[0].text.includes("Knowlu"), "the product name is in the annual reminder");
  assert(mails[0].text.includes("$69.99 per academic year"), "the amount is in the annual reminder");
  assert(mails[0].text.includes("cancel"), "and so is how to cancel");
  // R-C1-59 (I3): California's ARL notice must name the button the app actually has, never one it
  // does not — the same pin `handler_test.ts`'s resume-notice test carries.
  assert(mails[0].text.includes("Manage subscription"), "the annual reminder must name the real button");
  assert(!mails[0].text.includes("Cancel subscription"), "and never the button the app does not have");
});

Deno.test("a subscriber with no recognized plan is never mailed a guessed price", async () => {
  let mailed = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-06-15T00:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          {
            account_id: "acc-1",
            email: "a@example.invalid",
            subscription_id: "sub_1",
            plan: null,
            paused: false,
            started_at: "2025-01-01T00:00:00Z",
            last_reminded_at: null,
            current_period_end: null,
          },
        ]),
      stripe: () => Promise.resolve({}),
      sendEmail: () => {
        mailed = true;
        return Promise.resolve();
      },
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 0, reminded: 0, failed: 1 });
  assert(!mailed);
});
