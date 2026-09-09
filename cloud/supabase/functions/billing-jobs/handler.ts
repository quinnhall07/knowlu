/**
 * The two things a subscription needs that neither Stripe nor a user does on their own:
 *
 * 1. **The June–August pause** (R3). Nobody else does this, and it halves summer revenue — which is
 *    exactly why it has to be visible: a `pause_collection` window that a student is not told about
 *    ending looks like a free-to-pay conversion to a regulator and like a surprise charge to them.
 *    So resuming always sends the notice, in the same pass.
 * 2. **The annual reminder** California's ARL requires for a monthly subscription: what it is, what
 *    it costs, how often, and how to cancel.
 *
 * Driven by `pg_cron` once a day, authenticated by a shared job token — not by a user's JWT, since
 * there is no user, and not by the service-role key, which has no business in a database's config.
 */
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export interface Subscriber {
  account_id: string;
  email: string;
  subscription_id: string;
  plan: string | null;
  paused: boolean;
  started_at: string;
  last_reminded_at: string | null;
}

export interface Mail {
  to: string;
  subject: string;
  text: string;
}

export interface Deps {
  token: string;
  now: () => Date;
  listSubscribers: () => Promise<Subscriber[]>;
  stripe: StripePost;
  sendEmail: (m: Mail) => Promise<void>;
  recordReminder: (accountId: string, at: string) => Promise<void>;
}

/** June, July and August, by UTC month. A summer that starts on a local date nobody agrees on is
 * worse than one that starts a few hours early for somebody. */
export function shouldBePaused(d: Date): boolean {
  const m = d.getUTCMonth();
  return m === 5 || m === 6 || m === 7;
}

export function pauseDecision(d: Date, paused: boolean): "pause" | "resume" | "none" {
  const want = shouldBePaused(d);
  if (want && !paused) return "pause";
  if (!want && paused) return "resume";
  return "none";
}

const YEAR_MS = 365 * 24 * 60 * 60 * 1000;

export function reminderDue(a: { lastSentAt: string | null; startedAt: string; now: Date }): boolean {
  const since = Date.parse(a.lastSentAt ?? a.startedAt);
  if (!Number.isFinite(since)) return false;
  return a.now.getTime() - since > YEAR_MS;
}

function annualReminder(s: Subscriber): Mail {
  const price = s.plan === "academic_year" ? "$69.99 per academic year" : "$9.99 per month";
  return {
    to: s.email,
    subject: "Your Knowlu subscription",
    text: [
      "This is your yearly reminder that you have a Knowlu subscription.",
      "",
      `Knowlu, ${price}, renewing automatically until you cancel.`,
      "",
      "To cancel, open Knowlu, click the gear, and choose Cancel subscription — or",
      "reply to this message and we will cancel it for you.",
    ].join("\n"),
  };
}

function resumeNotice(s: Subscriber): Mail {
  const price = s.plan === "academic_year" ? "$69.99 per academic year" : "$9.99 per month";
  return {
    to: s.email,
    subject: "Knowlu billing starts again on 1 September",
    text: [
      "Knowlu does not bill over the summer, and the summer is over.",
      "",
      `Your subscription starts charging again: ${price}.`,
      "",
      "If you do not want it back, cancel before your next charge: open Knowlu,",
      "click the gear, and choose Cancel subscription.",
    ].join("\n"),
  };
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  if (req.headers.get("x-knowlu-job-token") !== deps.token) return fail(401, "not a job caller");

  const now = deps.now();
  let paused = 0, resumed = 0, reminded = 0;

  for (const s of await deps.listSubscribers()) {
    // **Monthly only** (R3, as ruled). The academic-year price already prices the summer in — that is
    // most of why it exists — and `pause_collection[behavior] = "void"` on a yearly subscription
    // whose renewal invoice falls in June, July or August voids that invoice and gives away a year.
    // The annual reminder below still runs for every plan.
    if (s.plan !== "monthly") {
      if (reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now })) {
        await deps.sendEmail(annualReminder(s));
        await deps.recordReminder(s.account_id, now.toISOString());
        reminded++;
      }
      continue;
    }
    switch (pauseDecision(now, s.paused)) {
      case "pause":
        await deps.stripe(`/v1/subscriptions/${s.subscription_id}`, { "pause_collection[behavior]": "void" });
        paused++;
        break;
      case "resume":
        await deps.stripe(`/v1/subscriptions/${s.subscription_id}`, { "pause_collection": "" });
        resumed++;
        // Always, and in the same pass: the charge comes back and the student hears it first.
        await deps.sendEmail(resumeNotice(s));
        await deps.recordReminder(s.account_id, now.toISOString());
        reminded++;
        continue;
      case "none":
        break;
    }
    if (reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now })) {
      await deps.sendEmail(annualReminder(s));
      await deps.recordReminder(s.account_id, now.toISOString());
      reminded++;
    }
  }

  return json(200, { paused, resumed, reminded });
}
