/**
 * The two things a subscription needs that neither Stripe nor a user does on their own:
 *
 * 1. **The June–August pause** (R3). Nobody else does this, and it halves summer revenue — which is
 *    exactly why it has to be visible: a `pause_collection` window that a student is not told about
 *    ending looks like a free-to-pay conversion to a regulator and like a surprise charge to them.
 *    So resuming always sends the notice first, and only then resumes at Stripe.
 * 2. **The annual reminder** California's ARL requires for a monthly subscription: what it is, what
 *    it costs, how often, and how to cancel.
 *
 * Driven by `pg_cron` once a day, authenticated by a shared job token — not by a user's JWT, since
 * there is no user, and not by the service-role key, which has no business in a database's config.
 *
 * **One subscriber's failure never stops the run**: each is wrapped in its own `try/catch`, logged
 * by account id only (never an email address or a token), and counted in `failed`.
 */
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { equalHex, StripePost } from "../_shared/stripe.ts";

export interface Subscriber {
  account_id: string;
  email: string;
  subscription_id: string;
  plan: string | null;
  paused: boolean;
  started_at: string;
  last_reminded_at: string | null;
  current_period_end: string | null;
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
const FOURTEEN_DAYS_MS = 14 * 24 * 60 * 60 * 1000;

export function reminderDue(a: { lastSentAt: string | null; startedAt: string; now: Date }): boolean {
  const since = Date.parse(a.lastSentAt ?? a.startedAt);
  if (!Number.isFinite(since)) return false;
  return a.now.getTime() - since > YEAR_MS;
}

/** The one place a plan becomes a price. `null` for anything that is not `monthly` or
 * `academic_year` — a subscriber whose plan we do not recognize is never mailed a guessed price. */
function priceOf(plan: string | null): string | null {
  if (plan === "academic_year") return "$69.99 per academic year";
  if (plan === "monthly") return "$9.99 per month";
  return null;
}

function annualReminder(s: Subscriber, price: string): Mail {
  return {
    to: s.email,
    subject: "Your Knowlu subscription",
    text: [
      "This is your yearly reminder that you have a Knowlu subscription.",
      "",
      `Knowlu, ${price}, renewing automatically until you cancel.`,
      "",
      "To cancel, open Knowlu, click the gear, and choose Manage subscription — or",
      "reply to this message and we will cancel it for you.",
    ].join("\n"),
  };
}

function resumeNotice(s: Subscriber, price: string): Mail {
  return {
    to: s.email,
    subject: "Knowlu billing starts again on 1 September",
    text: [
      "Knowlu does not bill over the summer, and the summer is over.",
      "",
      `Your subscription starts charging again: ${price}.`,
      "",
      "If you do not want it back, cancel before your next charge: open Knowlu,",
      "click the gear, and choose Manage subscription.",
    ].join("\n"),
  };
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  // R-C1-59 (M9): the same length-then-XOR compare `_shared/stripe.ts` uses for a webhook signature.
  if (!equalHex(req.headers.get("x-knowlu-job-token") ?? "", deps.token)) {
    return fail(401, "not a job caller");
  }

  const now = deps.now();
  let paused = 0, resumed = 0, reminded = 0, failed = 0;

  for (const s of await deps.listSubscribers()) {
    try {
      const price = priceOf(s.plan);
      if (price === null) {
        console.error(`billing-jobs: ${s.account_id}: unrecognized plan ${JSON.stringify(s.plan)}`);
        failed++;
        continue;
      }

      // **Monthly only** (R3, as ruled). The academic-year price already prices the summer in — that
      // is most of why it exists — and `pause_collection[behavior] = "void"` on a yearly subscription
      // whose renewal invoice falls in June, July or August voids that invoice and gives away a year.
      // The annual reminder below still runs for every plan.
      if (s.plan !== "monthly") {
        if (reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now })) {
          await deps.sendEmail(annualReminder(s, price));
          await deps.recordReminder(s.account_id, now.toISOString());
          reminded++;
        }
        continue;
      }

      switch (pauseDecision(now, s.paused)) {
        case "pause":
          await deps.stripe(`/v1/subscriptions/${s.subscription_id}`, {
            "pause_collection[behavior]": "void",
          });
          paused++;
          break;
        case "resume": {
          // **Tell the student before resuming, not after.** If the mail throws, this subscriber's
          // `catch` below fires and Stripe is never touched — the subscription stays paused and
          // tomorrow's run tries again. And skip the notice (never the resume) when one already went
          // out in the last 14 days: a Stripe failure that leaves `paused: true` must retry the
          // resume every day without mailing the student twice, and two runs in one day cannot
          // double-mail either.
          const recentlyReminded = s.last_reminded_at !== null &&
            now.getTime() - Date.parse(s.last_reminded_at) < FOURTEEN_DAYS_MS;
          if (!recentlyReminded) {
            await deps.sendEmail(resumeNotice(s, price));
            await deps.recordReminder(s.account_id, now.toISOString());
            reminded++;
          }
          await deps.stripe(`/v1/subscriptions/${s.subscription_id}`, { "pause_collection": "" });
          resumed++;
          continue;
        }
        case "none":
          break;
      }
      if (reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now })) {
        await deps.sendEmail(annualReminder(s, price));
        await deps.recordReminder(s.account_id, now.toISOString());
        reminded++;
      }
    } catch (e) {
      // Never an email address or a token: only the account id, and either the thrown Response's
      // status or the error's own string.
      console.error(`billing-jobs: ${s.account_id}: ${e instanceof Response ? e.status : String(e)}`);
      failed++;
    }
  }

  return json(200, { paused, resumed, reminded, failed });
}
