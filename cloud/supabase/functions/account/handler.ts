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
import { fail, json, methodNotAllowed, readJson, subPath } from "../_shared/http.ts";
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

/** The whole `sources.kind` vocabulary, matching the check constraint in `20260910000100_accounts.sql`
 * and `SOURCE_KINDS` in `app/src/lms_link.rs` — three copies, pinned to each other by
 * `app/tests/lms_link.rs::the_source_kind_vocabulary_is_one_list_in_three_places`.
 *
 * C1's wizard writes `lms_ics` and `calendar_ics` (spec §11a, one panel, both calendars).
 * `google_calendar` is a reserved value nobody writes (R-X-9): a Google grant has no URL and lives in
 * C2's `google_accounts`. It is listed here from the start so C2 never has to edit a C1-owned file. */
const SOURCE_KINDS = ["lms_ics", "calendar_ics", "google_calendar"];
const MAX_URL = 2048;

async function putSource(req: Request, deps: Deps): Promise<Response> {
  // The gate C2 imports, exercised here in C1 so the contract is proved by something that ships.
  const { account_id } = await deps.requireEntitled(req);
  const body = await readJson<{ kind?: string; url?: string }>(req);
  const kind = String(body.kind ?? "");
  const url = String(body.url ?? "");
  if (!SOURCE_KINDS.includes(kind)) throw fail(400, `unknown source kind; use ${SOURCE_KINDS.join(", ")}`);
  // `google_calendar` is a reserved kind **nobody writes** (R-X-9): a Google grant has no URL and
  // lives in C2's `google_accounts`, so no row of this kind ever exists here; the value is in the
  // vocabulary only so the constraint never needs a C2 migration.
  // The device is already narrowed by `lms_link::DEVICE_KINDS`; this is the same rule on the end that
  // a patched client actually talks to, so a forged row cannot become an iCal URL C2 then fetches.
  if (kind === "google_calendar") {
    throw fail(403, "that calendar is connected by signing in, not by pasting a link");
  }
  if (!url.startsWith("https://")) throw fail(400, "the feed link must start with https://");
  if (url.length > MAX_URL) throw fail(400, `the feed link is longer than ${MAX_URL} characters`);
  await deps.putSource(account_id, kind, url);
  // The kind, and nothing else: the reply must not echo a capability URL back over the wire.
  return json(200, { kind });
}

async function getSources(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  return json(200, { sources: await deps.getSources(user.id) });
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
    case "/sources":
      if (req.method === "PUT") return await putSource(req, deps);
      if (req.method === "GET") return await getSources(req, deps);
      return methodNotAllowed(["GET", "PUT"]);
    default:
      return fail(404, `no route ${path}`);
  }
}
