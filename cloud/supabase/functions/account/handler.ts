/**
 * One function, five routes (spec §5.1 and §4.1):
 *
 *   DELETE /account         the deletion right, and the app's *Delete my data*
 *   GET    /account/export  the access and portability rights
 *   GET    /account/sources what sources are connected (Task 7) — never their URLs
 *   PUT    /account/sources connect one (Task 7)
 *   POST   /account/consent record the terms/privacy/18+ attestation (Task 1, C1b) — idempotent
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

export interface ConsentWrite {
  account_id: string;
  email: string;
  tos_version: string;
  privacy_version: string;
  at: string;
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
  /** True when this account already has a `tos` consent row — the idempotence test. */
  hasConsent: (accountId: string) => Promise<boolean>;
  /** Fills the four null columns on `accounts` and inserts the three `consents` rows. */
  recordAccountConsent: (c: ConsentWrite) => Promise<void>;
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
// The kinds a client may actually send. `google_calendar` stays in `SOURCE_KINDS` (it is a known,
// reserved value — refused below with its own 403, not folded into "unknown") but the 400 message
// for a genuinely unknown kind must not advertise it as something worth trying.
const CLIENT_SOURCE_KINDS = SOURCE_KINDS.filter((k) => k !== "google_calendar");
const MAX_URL = 2048;
// A version string (`tos_version`, `privacy_version`) is a date stamp like "2026-09-10", never a
// document — this is a defensive bound on the consent log, not a real format check (nit 7).
const MAX_VERSION = 64;

async function putSource(req: Request, deps: Deps): Promise<Response> {
  // The gate C2 imports, exercised here in C1 so the contract is proved by something that ships.
  const { account_id } = await deps.requireEntitled(req);
  const body = await readJson<{ kind?: string; url?: string } | null>(req);
  // A body of `null` is valid JSON, so `readJson` does not throw — without this, `body.kind` below
  // throws a bare TypeError that reaches no `fail()` call and answers 500, not the 400 a bad request
  // should get.
  const fields = (body ?? {}) as { kind?: string; url?: string };
  const kind = String(fields.kind ?? "");
  const url = String(fields.url ?? "");
  if (!SOURCE_KINDS.includes(kind)) {
    throw fail(400, `unknown source kind; use ${CLIENT_SOURCE_KINDS.join(", ")}`);
  }
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

/**
 * **The attestation, after the fact — and the only place a `tos`, `privacy` or `age_18` consent row
 * is written.** (`billing-checkout` still writes its own `auto_renew` row; the two never collide —
 * `hasConsent`'s `kind=eq.tos` filter cannot see a checkout's row.) Migration `20260917000100`
 * stopped the auth trigger reading the sign-up's own metadata at all, on BOTH paths, because `/otp`
 * with `create_user: true` is reachable by anyone holding the public anon key: an attestation taken
 * out of that request would be an `age_18` row the address's owner never made. So every new account
 * arrives here with its four consent columns null, and the app calls this route after EVERY
 * sign-in — `google_sign_in` and `verify_email_code` alike — which is why a second call must be
 * silent rather than a conflict.
 *
 * The 18+ gate has not moved off the server: `billing-checkout` refuses an account whose
 * `age_attested_at` is still null, so a client that skips this route gets an account that can never
 * subscribe. That refusal is the whole of the tooth the migration's `raise` used to be.
 */
async function recordConsent(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  // A body of `null` is valid JSON, so `readJson` does not throw — without the `?? {}` and the
  // `String(...)` coercions below, a null body or a non-string version reaches `.trim()` and throws
  // a bare TypeError that `asResponse` turns into a 500, not the 400 a bad request should get.
  // `putSource` at `:107-111` carries this same guard for the same reason.
  const body =
    await readJson<{ tos_version?: string; privacy_version?: string; age_attested?: boolean } | null>(
      req,
    ) ?? {};
  if (body.age_attested !== true) {
    throw fail(400, "age attestation required: Knowlu is for people 18 or older");
  }
  const tos = String(body.tos_version ?? "").trim();
  const priv = String(body.privacy_version ?? "").trim();
  if (!tos || !priv) throw fail(400, "the terms and the privacy policy must be accepted at sign-up");
  // A version string is a record kept as evidence, so a value too long to have come from our own
  // client is refused rather than silently truncated into something that still looks valid.
  if (tos.length > MAX_VERSION || priv.length > MAX_VERSION) {
    throw fail(400, `a version string is longer than ${MAX_VERSION} characters`);
  }
  if (await deps.hasConsent(user.id)) return json(200, { ok: true });
  const account = await deps.getAccount(user.id);
  if (!account) throw fail(404, "no such account");
  await deps.recordAccountConsent({
    account_id: user.id,
    email: account.email,
    tos_version: tos,
    privacy_version: priv,
    at: deps.now().toISOString(),
  });
  return json(200, { ok: true });
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
    case "/consent":
      if (req.method !== "POST") return methodNotAllowed(["POST"]);
      return await recordConsent(req, deps);
    case "/sources":
      if (req.method === "PUT") return await putSource(req, deps);
      if (req.method === "GET") return await getSources(req, deps);
      return methodNotAllowed(["GET", "PUT"]);
    default:
      return fail(404, `no route ${path}`);
  }
}
