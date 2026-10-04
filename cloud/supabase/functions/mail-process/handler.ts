/**
 * `mail-process` (email plan T3d; spec §4.7 steps 1–3 and step 6's deletes, D8). The worker over
 * `mail_work`: started every minute by the cron and kicked by intake after a hand-off (D8), both with
 * `x-knowlu-job-token`. Per item, oldest arrival first, inside the run's wall-clock budget:
 *
 * 0. **Lease** the item (`next_attempt_at` moved past this run, guarded on its old value), so a kick and
 *    the cron never work one item twice.
 * 1. **Parse** (`mime.ts`): the headers, and the first `text/plain` part, else the HTML part as text. No
 *    attachment is opened.
 * 2. **Screen, no model** (`screen.ts`), in order, the first that fires deciding: bulk or list mail from a
 *    sender off the allow-list; Microsoft's bulk score at 7 or above; the account's promoted email rules
 *    answering noise; the student's own mail outside a batch (From is this mailbox itself; PQ8 (ii) only);
 *    a vendor's notification that carries no D9 item (`NOISE_TEMPLATES`: sender and subject, `aligned`
 *    only). `noise`: the object and the row are deleted.
 *    A live item that is not `aligned` never reaches the screen: intake deletes those, and one that is here
 *    anyway is deleted and counted `unauthenticated`.
 * 3. **Templates, no model** (`mail_templates.ts`): a row's claims, checked by `validateMailClaims` against
 *    the whole text, scrubbed and never cut (a row reads the whole text; only the model's claims are held to
 *    the model's 2,000-character cut, Quinn's ruling of 2026-10-04). Kept: one `mail_claims` row is queued
 *    (`template-only`). All refused: `refused`. Either way the object and the row are deleted.
 * 4. Everything else goes to `deps.model`, §4.7 steps 4–6 (T3e, `model.ts`: charge, model, queue, and its
 *    deletes, or a wait to the next UTC day and, at the third attempt, expiry).
 *
 * An item whose object is gone (the bucket's expiry came first) is expired here (`expire_mail_work`): its
 * work row and its `mail_seen` row are deleted and the mailbox counts one more `expired` (D8). A cron call
 * with `{mode: "sweep"}` runs the nightly sweep instead (`sweep.ts`, plan P7).
 *
 * **Every final path deletes the work object** (step 6). The row goes first and the object after it, so a
 * failure between them leaves an object the bucket's one-day `work/` expiry removes, never a row that would
 * be queued twice.
 *
 * **Privacy:** every log line is a fixed phrase. No address, subject, text, header value, object key or
 * token is logged, put in an error or answered; the text is never written anywhere.
 */
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import type { RuleTable } from "../_shared/judge_rules.ts";
import { validateMailClaims } from "../_shared/judge_validate.ts";
import type { Authenticity } from "../_shared/mail_bind.ts";
import {
  type MailClaim,
  type MailTemplate,
  templateClaims,
  TEMPLATES,
  VENDOR_DOMAINS,
} from "../_shared/mail_templates.ts";
import { bodyText, decodeWords, type MailHeaders, mailUid, parseHeaders } from "../_shared/mime.ts";
import { type S3Client, S3Error } from "../_shared/s3.ts";
import { scrubForPrompt } from "../_shared/scrub.ts";
import { equalHex } from "../_shared/stripe.ts";
import { allowDomains, screen } from "./screen.ts";
import { sweep } from "./sweep.ts";

export interface ProcessConfig {
  /** `MAIL_JOBS_TOKEN`: the cron's and the kick's `x-knowlu-job-token`. */
  jobsToken: string;
}
export type Phase = "live" | "backfill" | "recover";
/** A `mail_work` row as the worker reads it; times are ISO strings, as PostgREST answers them. */
export interface WorkItem {
  id: string;
  accountId: string;
  addressId: string;
  batchId: string | null;
  s3Key: string;
  phase: Phase;
  authenticity: Authenticity;
  arrivedAt: string;
  /** The lease's guard: the value a run must see to take the item. */
  nextAttemptAt: string;
  attempts: number;
  /** The batch's `opened_at`, for an original with no delivery stamp (§4.8's ordering date). */
  batchOpenedAt: string | null;
}
/** One of the account's mailboxes. */
export interface Mailbox {
  id: string;
  provider: "m365" | "google" | "other";
  mailboxHint: string | null;
}
/** A `mail_claims` row (§5.1, §4.7 step 6): the claims and the message's metadata, never its text. */
export interface ClaimsRow {
  accountId: string;
  addressId: string;
  batchId: string | null;
  uid: string;
  messageDate: string | null;
  orderingDate: string;
  senderName: string | null;
  senderAddr: string | null;
  phase: Phase;
  authenticity: Authenticity;
  claims: MailClaim[];
  judgmentId: string | null;
}
export interface ProcessStore {
  /** Work whose `next_attempt_at` has come, oldest arrival first, at most `limit`. */
  due(nowIso: string, limit: number): Promise<WorkItem[]>;
  /** Moves the item's `next_attempt_at` to `untilIso` only while it still reads `item.nextAttemptAt`;
   * false, with nothing written, when another run took it first. */
  lease(item: WorkItem, untilIso: string): Promise<boolean>;
  mailboxes(accountId: string): Promise<Mailbox[]>;
  /** The account's known course codes (§4.7 step 4). */
  knownCourses(accountId: string): Promise<string[]>;
  queue(row: ClaimsRow): Promise<void>;
  /** Deletes the `mail_work` row. */
  finish(workId: string): Promise<void>;
  count(addressId: string, counter: "unauthenticated"): Promise<void>;
  /** After a capped, over-budget or failed attempt (§4.7 step 5): the item's next attempt and its count. */
  wait(workId: string, untilIso: string, attempts: number): Promise<void>;
  /** `expire_mail_work`: the work row, its `mail_seen` row and one `expired` on its mailbox, in one
   * transaction; false, counting nothing, when the row is already gone. */
  expire(workId: string): Promise<boolean>;
}
/** The nightly sweep's reads and deletes (plan P7). Every bound is an ISO time, at or before which a row
 * qualifies; each answers how many rows it touched. */
export interface SweepStore {
  /** Work that arrived at or before the bound, oldest first, at most `limit`. */
  expirable(arrivedByIso: string, limit: number): Promise<Array<{ id: string; s3Key: string }>>;
  /** Stamps `closed_at` on open batches sent at or before `sentByIso` or opened at or before `openedByIso`. */
  closeBatches(nowIso: string, sentByIso: string, openedByIso: string): Promise<number>;
  pruneSeen(seenByIso: string): Promise<number>;
  /** Delivered claims delivered at or before the first bound; undelivered ones queued at or before the second. */
  pruneClaims(
    deliveredByIso: string,
    queuedByIso: string,
  ): Promise<{ delivered: number; undelivered: number }>;
  pruneBatches(closedByIso: string): Promise<number>;
  pruneCanaries(sentByIso: string): Promise<number>;
}
/** The message as step 1 parsed it. Held in memory only. */
export interface MailMessage {
  headers: MailHeaders;
  /** The From header's value, decoded (the sender a promoted email rule is keyed on). */
  from: string;
  subject: string;
  /** The Date header as written: `judge_due.ts`'s reference. */
  date: string;
  /** The text before any cut or scrub. */
  text: string;
}
/** What the model path (§4.7 steps 4–6, T3e) is handed for an item no screen or template decided. */
export interface ModelInput {
  item: WorkItem;
  message: MailMessage;
  knownCourses: readonly string[];
}
export interface ProcessDeps {
  store: ProcessStore & SweepStore;
  s3: S3Client;
  rules: RuleTable;
  now(): number;
  /** §4.7 steps 4–6 (T3e). It owns the item from here: its charge, its call, its queue and its deletes. */
  model(input: ModelInput): Promise<void>;
  /** The template rows; `mail_templates.ts`'s `TEMPLATES` by default. */
  templates?: readonly MailTemplate[];
  /** The vendors' sending domains (the screen's allow-list); `mail_templates.ts`'s `VENDOR_DOMAINS` by default. */
  vendorDomains?: readonly string[];
}

/** What finishing or expiring one item needs. */
export interface ItemDeps {
  store: ProcessStore;
  s3: S3Client;
}

/** How long a run holds an item it took: past any one item's work, short of the next attempt. */
export const LEASE_MS = 5 * 60_000;
/** The run's own wall-clock budget, inside the platform's limit, and the work read per round. */
export const RUN_BUDGET_MS = 50_000;
const ROUND = 25;
/** At most this many items a run, whatever the budget. */
const MAX_ITEMS = 500;
const SENDER_CHARS = 200;

/** The only thing the worker logs: a fixed phrase. */
const log = (phrase: string) => console.log(`mail-process: ${phrase}`);
const iso = (ms: number) => new Date(ms).toISOString();

/** Step 1: the headers and the text, in memory only. */
function parse(raw: Uint8Array): MailMessage {
  const headers = parseHeaders(raw);
  const fromField = headers.headers.find((f) => f.name.trim().toLowerCase() === "from");
  return {
    headers,
    from: fromField === undefined ? "" : decodeWords(fromField.value).trim(),
    subject: headers.subject ?? "",
    date: headers.date ?? "",
    text: bodyText(raw),
  };
}

/** A sender's display name as the device may show it: one line (§10, R11), cut. */
function senderName(name: string | null): string | null {
  if (name === null) return null;
  const breaks = (c: number) => c < 0x20 || c === 0x7f || c === 0x85 || c === 0x2028 || c === 0x2029;
  const one = [...name].map((ch) => breaks(ch.codePointAt(0) ?? 0) ? " " : ch).join("").replace(/\s+/g, " ")
    .trim();
  return one === "" ? null : [...one].slice(0, SENDER_CHARS).join("");
}

/** §4.7 step 6's row: the claims and the metadata, never the text. The ordering date (§4.8) is the earlier
 * of the Date header and the received time: intake's arrival for live mail; the provider's delivery
 * stamp, else the batch's opening, for a backfilled or recovered original. */
export async function claimsRow(
  item: WorkItem,
  m: MailMessage,
  claims: MailClaim[],
  judgmentId: string | null,
): Promise<ClaimsRow> {
  const arrived = Date.parse(item.arrivedAt);
  const opened = item.batchOpenedAt === null ? NaN : Date.parse(item.batchOpenedAt);
  const received = item.phase === "live"
    ? arrived
    : m.headers.receivedAt ?? (Number.isFinite(opened) ? opened : arrived);
  const dated = m.headers.dateAt;
  return {
    accountId: item.accountId,
    addressId: item.addressId,
    batchId: item.batchId,
    uid: await mailUid(m.headers),
    messageDate: dated === null ? null : iso(dated),
    orderingDate: iso(dated === null ? received : Math.min(dated, received)),
    senderName: senderName(m.headers.fromName),
    senderAddr: m.headers.fromAddress,
    phase: item.phase,
    authenticity: item.authenticity,
    claims,
    judgmentId,
  };
}

/** Step 6's deletes: the row, then the object. A failed object delete is left to the bucket's one-day
 * `work/` expiry; the row is already gone, so nothing is processed twice. */
export async function finishItem(item: WorkItem, deps: ItemDeps): Promise<void> {
  await deps.store.finish(item.id);
  await deps.s3.delete(item.s3Key).catch(() => log("work object delete failed, left to the bucket's expiry"));
}

/** Expiry (D8): the work row and its seen row in one transaction, counted, then the object if it is still
 * there. A failed object delete is left to the bucket's expiry, as `finishItem`'s is. */
export async function expireItem(item: { id: string; s3Key: string }, deps: ItemDeps): Promise<boolean> {
  const expired = await deps.store.expire(item.id);
  await deps.s3.delete(item.s3Key).catch(() => log("work object delete failed, left to the bucket's expiry"));
  return expired;
}

type Outcome = "noise" | "template" | "refused" | "unauthenticated" | "model" | "expired" | "taken" | "left";

/** The account's mailboxes, allow-list and known courses, read once a run. */
interface Account {
  mailboxes: Mailbox[];
  allow: string[];
  knownCourses: string[];
}

/** Steps 0–3 for one item, then the model path for what is left. */
async function processItem(
  item: WorkItem,
  deps: ProcessDeps,
  account: (id: string) => Promise<Account>,
): Promise<Outcome> {
  if (!(await deps.store.lease(item, iso(deps.now() + LEASE_MS)))) return "taken";
  if (item.phase === "live" && item.authenticity !== "aligned") {
    await deps.store.count(item.addressId, "unauthenticated");
    await finishItem(item, deps);
    log("unauthenticated, deleted");
    return "unauthenticated";
  }
  let raw: Uint8Array;
  try {
    raw = await deps.s3.get(item.s3Key);
  } catch (e) {
    // A missing object expired first (D8): the item is counted `expired` and loses its seen row. Anything
    // else waits out the lease.
    if (e instanceof S3Error && e.status === 404) {
      await expireItem(item, deps);
      log("work object gone, expired");
      return "expired";
    }
    log(`work object unreadable (${e instanceof S3Error ? `s3 get ${e.status}` : "s3"}), left`);
    return "left";
  }
  const message = parse(raw);
  const acct = await account(item.accountId);
  const mailbox = acct.mailboxes.find((m) => m.id === item.addressId) ?? { mailboxHint: null };
  const templates = deps.templates ?? TEMPLATES;
  const reason = await screen({
    headers: message.headers,
    from: message.from,
    accountId: item.accountId,
    phase: item.phase,
    mailbox,
    allow: acct.allow,
    subject: message.subject,
    authenticity: item.authenticity,
    templates,
  }, deps.rules);
  if (reason !== null) {
    await finishItem(item, deps);
    log(`noise (${reason}), deleted`);
    return "noise";
  }
  const context = { knownCourses: acct.knownCourses, timeZone: null };
  const fromTemplate = templateClaims({ ...message, authenticity: item.authenticity }, context, templates);
  if (fromTemplate === null) {
    await deps.model({ item, message, knownCourses: acct.knownCourses });
    return "model";
  }
  // A template's claims meet the model's checks against the whole text, scrubbed: a row reads the whole
  // text, so the model's cut never refuses one (Quinn, 2026-10-04), and an evidence span the scrub would
  // change (an address, a link, a token) is still refused, so none reaches the device. The model path keeps
  // its claims to the cut it was shown (`model.ts`).
  const checked = validateMailClaims({ claims: fromTemplate }, {
    from: message.from,
    subject: scrubForPrompt(message.subject),
    text: scrubForPrompt(message.text.trim()),
    date: message.date,
    ...context,
  }, "template");
  const kept = checked.claims.filter((c) => c.kind !== "noise");
  if (kept.length > 0) await deps.store.queue(await claimsRow(item, message, kept, null));
  await finishItem(item, deps);
  log(kept.length > 0 ? "queued (template), deleted" : "template refused, deleted");
  return kept.length > 0 ? "template" : "refused";
}

/** One run: due work, oldest arrival first, round by round, inside the budget. */
async function run(deps: ProcessDeps): Promise<Record<Outcome | "failed", number>> {
  const counts: Record<Outcome | "failed", number> = {
    noise: 0,
    template: 0,
    refused: 0,
    unauthenticated: 0,
    model: 0,
    expired: 0,
    taken: 0,
    left: 0,
    failed: 0,
  };
  const started = deps.now();
  const accounts = new Map<string, Promise<Account>>();
  const account = (id: string) => {
    let got = accounts.get(id);
    if (got === undefined) {
      got = Promise.all([deps.store.mailboxes(id), deps.store.knownCourses(id)]).then((
        [mailboxes, known],
      ) => ({
        mailboxes,
        allow: allowDomains(mailboxes, deps.vendorDomains ?? VENDOR_DOMAINS, deps.templates ?? TEMPLATES),
        knownCourses: known,
      }));
      got.catch(() => accounts.delete(id));
      accounts.set(id, got);
    }
    return got;
  };
  let seen = 0;
  while (seen < MAX_ITEMS && deps.now() - started < RUN_BUDGET_MS) {
    const due = await deps.store.due(iso(deps.now()), ROUND);
    if (due.length === 0) break;
    let took = 0;
    for (const item of due) {
      if (seen >= MAX_ITEMS || deps.now() - started >= RUN_BUDGET_MS) break;
      seen++;
      try {
        const outcome = await processItem(item, deps, account);
        counts[outcome]++;
        if (outcome !== "taken") took++;
      } catch (e) {
        counts.failed++;
        took++;
        log(`item failed (${e instanceof S3Error ? `s3 ${e.op} ${e.status}` : "store"}), left`);
      }
    }
    // A round that took nothing (every item leased by another run) ends the run rather than spin.
    if (took === 0) break;
  }
  return counts;
}

/** One cron call or kick. 401 without the job token; `{mode: "sweep"}` runs the nightly sweep (P7). */
export async function handle(req: Request, cfg: ProcessConfig, deps: ProcessDeps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const token = req.headers.get("x-knowlu-job-token") ?? "";
  if (cfg.jobsToken === "" || !equalHex(token, cfg.jobsToken)) return fail(401, "not a job caller");
  const body = await req.json().catch(() => ({})) as { mode?: unknown };
  try {
    if (body?.mode === "sweep") return json(200, { ok: true, ...(await sweep(deps)) });
    return json(200, { ok: true, ...(await run(deps)) });
  } catch (e) {
    log(`run failed (${e instanceof S3Error ? `s3 ${e.op} ${e.status}` : "store"})`);
    return fail(500, "the run failed");
  }
}
