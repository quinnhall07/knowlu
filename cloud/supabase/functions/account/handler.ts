/**
 * One function, four routes (spec §5.1 and §4.1):
 *
 *   DELETE /account         the deletion right, and the app's *Delete my data*
 *   GET    /account/export  the access and portability rights
 *   GET    /account/sources what sources are connected (Task 7) — never their URLs
 *   PUT    /account/sources connect one (Task 7)
 *
 * The order inside `DELETE` is the whole of its correctness: Stripe first (a cancelled card is
 * better than a deleted account still being billed), then the rows, then the tombstone, and the
 * login **last** — deleting the auth user first would invalidate the very token the rest of this
 * request is authenticated by.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, subPath } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export interface SourceRow {
  kind: string;
  added_at: string;
}

export interface Deps {
  verify: VerifyToken;
  /** The C2 gate, injected so this handler stays testable: `requireActiveEntitlement`. */
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  getAccount: (accountId: string) => Promise<{ email: string; stripe_customer_id: string | null } | null>;
  getSubscriptionId: (accountId: string) => Promise<string | null>;
  stripe: StripePost;
  /** Rows in sources, telemetry_events, corrections, issues, entitlements, accounts; consents nulled. */
  purge: (accountId: string) => Promise<void>;
  deleteAuthUser: (accountId: string) => Promise<void>;
  tombstone: (emailHash: string, at: string) => Promise<void>;
  exportAll: (accountId: string) => Promise<Record<string, unknown>>;
  hashEmail: (email: string) => Promise<string>;
  getSources: (accountId: string) => Promise<SourceRow[]>;
  putSource: (accountId: string, kind: string, url: string) => Promise<void>;
  now: () => Date;
}

async function deleteAccount(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  const account = await deps.getAccount(user.id);
  if (!account) {
    // The `accounts` row is already gone — a previous DELETE reached it and then failed on the
    // tombstone or the login. There is nothing left to cancel or purge; finish the job from where it
    // stopped rather than answering 404 forever to a login this account can never shed.
    if (user.email) {
      await deps.tombstone(await deps.hashEmail(user.email), deps.now().toISOString());
    }
    await deps.deleteAuthUser(user.id);
    return json(200, { deleted: true });
  }

  const sub = await deps.getSubscriptionId(user.id);
  if (sub) {
    // At period end, not immediately: the student paid for this month and deleting is not a refund.
    // The Stripe *customer* object itself is not deleted here — it stays under Stripe's own
    // billing-record retention, and removing it is not part of this right.
    await deps.stripe(`/v1/subscriptions/${sub}`, { cancel_at_period_end: "true" });
  }
  await deps.purge(user.id);
  await deps.tombstone(await deps.hashEmail(account.email), deps.now().toISOString());
  await deps.deleteAuthUser(user.id);
  return json(200, { deleted: true });
}

async function exportAccount(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  const all = await deps.exportAll(user.id);
  return json(200, { ...all, exported_at: deps.now().toISOString() });
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  const path = subPath(req.url, "account");
  switch (path) {
    case "/":
      if (req.method !== "DELETE") return methodNotAllowed(["DELETE"]);
      return await deleteAccount(req, deps);
    case "/export":
      if (req.method !== "GET") return methodNotAllowed(["GET"]);
      return await exportAccount(req, deps);
    default:
      return fail(404, `no route ${path}`);
  }
}
