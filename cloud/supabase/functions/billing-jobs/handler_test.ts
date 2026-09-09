import { assert, assertEquals } from "@std/assert";
import { handle, pauseDecision, reminderDue, shouldBePaused } from "./handler.ts";

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
          },
          {
            account_id: "acc-2",
            email: "b@example.invalid",
            subscription_id: "sub_2",
            plan: "academic_year",
            paused: false,
            started_at: "2026-09-10T00:00:00Z",
            last_reminded_at: null,
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
  assertEquals(await res.json(), { paused: 1, resumed: 0, reminded: 0 });
  assertEquals(calls.map((c) => c[0]), ["/v1/subscriptions/sub_1"]);
  assertEquals(calls[0][1], { "pause_collection[behavior]": "void" });
});

Deno.test("on 1 September everything resumes and every resumed student is told before the charge", async () => {
  const mails: { to: string; subject: string; text: string }[] = [];
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
  assertEquals(await res.json(), { paused: 0, resumed: 1, reminded: 1 });
  assertEquals(mails.length, 1);
  assertEquals(mails[0].to, "a@example.invalid");
  assert(mails[0].subject.includes("Knowlu"));
  assert(mails[0].text.includes("$9.99"), "the amount is in the reminder");
  assert(mails[0].text.includes("cancel"), "and so is how to cancel");
});
