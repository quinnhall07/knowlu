// POST /gmail-read — the twice-daily read, aligned to the student's slots because the device asks
// for it inside its own slot (cloud design §5.3). Delivery is a pull; nothing is pushed.
//
// The order matters:
//   1. acknowledge what the LAST pull actually wrote (so a crash between reply and write costs a
//      repeat, never a lost task);
//   2. get an access token from the Vault-held refresh token — a revoked grant, an account with no
//      Gmail scope at all, and a deployment with no Google client configured are three different
//      answers, and only one of them (a revoked grant) is ever treated as a failure (R-C2-E41);
//   3. list message ids from the last 7 days across the mailbox, minus the account's excluded
//      labels (the routine's "Crimson label and the personal inbox" rule, generalised), and drop
//      every uid already in `gmail_seen` BEFORE the read cap is applied, so a backlog past
//      `READ_CAP` is still reachable over several rounds (R-C2-E42);
//   4. fetch each remaining message's HEADERS and TEXT PART — never a binary enclosure, ever;
//   5. judge it through the one pipeline with origin 'gmail_api';
//   6. queue the verdict, mark the uid seen, and DISCARD the text — but a verdict the pipeline
//      could not reach (capped, low confidence, a model failure) is NEITHER enqueued nor marked
//      seen, so it is asked about again next time rather than silently dropped; a `capped` outcome
//      stops the whole round on the spot, since nothing later in the same list will fare better;
//   7. stop at the wall-clock budget and say `more`, because an edge function cannot run sixty
//      120-second model calls; the dedup set is the resume cursor.
//
// F-6: the message text exists in this function's memory for the duration of one judgment and
// nowhere else — never in `gmail_queue`, never in `judgments`, never in a log line. What DOES
// reach `gmail_queue` is the model's own WRITING about the message (a title, a rationale) and the
// rest of the verdict's fields, every one of them derived from the text and none of them the text
// itself (the correction beside `gmail_queue`'s own definition, `20260911000200_google.sql`, says
// the same thing from the schema's side).
import { judge, type JudgeReply, type PipelineDeps } from "../_shared/judge_pipeline.ts";
import type { Entitle } from "../_shared/judge_handler.ts";
import { GOOGLE_NOT_CONFIGURED } from "../_shared/google_scopes.ts";

export const WINDOW = "newer_than:7d";
/** A bound on one read, so a mailbox with a thousand unread messages cannot eat a slot. */
export const READ_CAP = 60;
/**
 * F-4: the model timeout the gmail path uses. Not `judge_anthropic.ts`'s own `CALL_TIMEOUT_MS`
 * (120 s): the budget check below has to reserve room for one call already in flight when it
 * decides whether to start another, and 120 s of reservation on top of `READ_BUDGET_MS` would push
 * the round's true worst case well past any edge function's own wall clock (~150 s). The pipeline
 * this function builds (`gmail-read/index.ts`'s `pipeline`) passes this to `liveDeps` so the model
 * client itself is built with the shorter timeout, not only this check.
 */
export const PER_CALL_MS = 60_000;
/**
 * F-4 (regrading m34, "the budget bounds when the last call starts, not ends"): the round's real
 * deadline — when the LAST call must have FINISHED — not merely when it stops accepting new work.
 * The old value here (40 s) only ever bounded the second thing: a call could start at 39.9 s and
 * then run for its own full timeout on top, which at the old 120 s model timeout meant a round
 * could take up to ~160 s — past any edge function's own wall clock (~150 s) and the actual bug.
 *
 * The check below (`elapsed + PER_CALL_MS >= READ_BUDGET_MS`) stops accepting new work at the
 * SAME point as before — `READ_BUDGET_MS - PER_CALL_MS` = 40 s, unchanged — by reserving
 * `PER_CALL_MS` up front rather than discovering the overrun after it already started. What moves
 * is what this constant NAMES: it is now the round's honest total ceiling (40 s of new-work window
 * plus the 60 s the last call it started is allowed to still be running), not the new-work window
 * alone, which is why its value is `PER_CALL_MS` more than it used to be.
 */
export const READ_BUDGET_MS = 40_000 + PER_CALL_MS;

/** A Gmail API call that failed. The status is kept because it reaches a log line; the response
 * body is not, because a Gmail error body is not ours to log (§5.6's rule, applied here too). */
export class GmailApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = "GmailApiError";
  }
}

export interface GmailApi {
  /**
   * Message ids only, matching `q` — `unseen` is the account's own dedup filter (`!already.has(…)`
   * below), handed down so an implementation that pages (F-3) knows when it has gathered enough
   * NEW mail to stop asking Gmail for more, rather than paging to exhaustion or stopping at
   * Gmail's own one-page default of 100. This is an early-stop HINT only: the handler still
   * filters and caps the result itself below (dedup-before-cap, R-C2-E42), unconditionally.
   */
  list(accessToken: string, q: string, unseen: (id: string) => boolean): Promise<string[]>;
  /** Headers plus the first `text/plain` part, decoded. **Never a binary enclosure.** */
  message(accessToken: string, id: string): Promise<{ subject: string; from: string; date: string; text: string }>;
}

/** One raw page of ids from a paged list API, and the token for the next page, if any. */
export interface ListPage {
  ids: string[];
  nextPageToken?: string;
}

/** `list`'s own hard ceiling (F-3): whatever `want` asks for, and whatever `nextPageToken` keeps
 * promising, a mailbox is never walked past this many pages in one invocation. */
export const MAX_LIST_PAGES = 5;

/**
 * Follows `nextPageToken` until it has gathered `want` ids `unseen` accepts, or the pages run out
 * — capped at `maxPages` regardless, so a mailbox that never stops promising a next page cannot
 * turn one read into an unbounded loop (F-3, regrading m34). Before this, the caller asked for
 * exactly one page of 100 and threw the token away: a backlog whose truly-unseen mail lived on
 * page two was invisible no matter how large `want` was, because dedup-before-cap can only dedup
 * what it was handed.
 *
 * `fetchPage` is the one thing this needs injected — no accessToken, no query string, no Gmail
 * shape — so a test can prove the paging and the two stopping conditions with a fake that returns
 * canned pages, and the production wiring (`gmail-read/index.ts`) supplies the one that actually
 * calls Gmail.
 */
export async function pagedList(
  fetchPage: (pageToken?: string) => Promise<ListPage>,
  unseen: (id: string) => boolean,
  want: number,
  maxPages: number = MAX_LIST_PAGES,
): Promise<string[]> {
  const out: string[] = [];
  let pageToken: string | undefined;
  for (let page = 0; page < maxPages; page++) {
    const { ids, nextPageToken } = await fetchPage(pageToken);
    out.push(...ids);
    if (out.filter(unseen).length >= want || nextPageToken === undefined) break;
    pageToken = nextPageToken;
  }
  return out;
}

/** R-C2-E41: a revoked grant, an account that never granted the Gmail scope, and a deployment
 * with no Google client configured are three different situations — only the first is ever
 * treated as a failure. The device's `cloud:google` marker is a CALENDAR grant; every account that
 * has one but never took the later, incremental Gmail step answers `missing: "scope"`, and that
 * must never call `markRevoked` on a perfectly good calendar grant (R-C2-E41's whole point). */
export type TokenLookup = { token: string } | { missing: "scope" | "grant" | "config" };

export interface ReadDeps {
  api: GmailApi;
  /** A fresh access token, or which of the three reasons there is none. */
  accessTokenFor(accountId: string): Promise<TokenLookup>;
  excludedLabels(accountId: string): Promise<string[]>;
  markRevoked(accountId: string): Promise<void>;
  seen(accountId: string): Promise<Set<string>>;
  markSeen(accountId: string, uid: string): Promise<void>;
  enqueue(
    accountId: string,
    uid: string,
    tier: string,
    payload: Record<string, unknown>,
    judgmentId: string | null,
  ): Promise<void>;
  undelivered(accountId: string): Promise<Array<{ uid: string; tier: string; payload: Record<string, unknown> }>>;
  deliver(accountId: string, uids: string[]): Promise<void>;
  knownCourses(accountId: string): Promise<string[]>;
  pipeline(): Promise<PipelineDeps>;
  budgetMs: number;
  clock: () => number;
}

export function query(excluded: string[]): string {
  // A label with a space OR a double quote is quoted the way Gmail's own search does it, and an
  // embedded quote is escaped so it cannot close the term early.
  const terms = excluded.map((l) => {
    const escaped = l.replace(/"/g, '\\"');
    return `-label:${l.includes(" ") || l.includes('"') ? `"${escaped}"` : l}`;
  });
  return [WINDOW, ...terms].join(" ");
}

/** `ack` entries the device could have produced: `gmail:` plus the id characters Gmail actually
 * uses. Anything else is dropped rather than handed to a `uid=in.(...)` filter. */
const ACK_SHAPE = /^gmail:[A-Za-z0-9_-]+$/;
/** F-6: a hard ceiling on one round's acknowledgement. The device never queues more than
 * `READ_CAP` uids in a round it could later ack, so a legitimate `ack` is always small — anything
 * past this is either a device bug or a degenerate request, and the excess is dropped (and
 * counted, in the log line below) rather than building an unbounded `uid=in.(...)` filter. */
const ACK_CAP = 500;

export function readHandler(entitle: Entitle, deps: ReadDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const body = await req.json().catch(() => ({})) as { ack?: unknown };
      const shaped = Array.isArray(body.ack)
        ? body.ack.filter((u): u is string => typeof u === "string" && ACK_SHAPE.test(u))
        : [];
      const ack = shaped.slice(0, ACK_CAP);
      if (shaped.length > ACK_CAP) {
        console.error(`gmail-read: ack dropped ${shaped.length - ACK_CAP} entries past the ${ACK_CAP} cap`);
      }
      if (ack.length > 0) await deps.deliver(account_id, ack);

      const lookup = await deps.accessTokenFor(account_id);
      if ("missing" in lookup) {
        if (lookup.missing === "config") {
          // Not a per-account situation at all — P2 was never set on this deployment — so it is
          // never `quiet` (which the device reads as "reconnect from settings", a UI Quinn cannot
          // act on) and never `markRevoked` (there may be no grant to revoke in the first place).
          return Response.json({ error: GOOGLE_NOT_CONFIGURED }, { status: 503 });
        }
        if (lookup.missing === "scope") {
          // A calendar-only grant. Not a failure and not `markRevoked` — that would kill the
          // calendar reader over a Gmail step the student never took (R-C2-E41).
          return Response.json({
            items: await deps.undelivered(account_id), read: 0, quiet: true, reason: "no_gmail_scope", more: false,
          });
        }
        // `missing === "grant"`: a revoked or expired grant, and while the Google project is in
        // Testing the token dies every 7 days by design (§5.3, §9). The DEVICE turns `quiet` into
        // a line the student can act on; here it is one status change.
        await deps.markRevoked(account_id);
        return Response.json({
          items: await deps.undelivered(account_id), read: 0, quiet: true, reason: "revoked", more: false,
        });
      }
      const token = lookup.token;

      const already = await deps.seen(account_id);
      const known = await deps.knownCourses(account_id);
      const pipeline = await deps.pipeline();
      const started = deps.clock();
      let read = 0;
      let more = false;
      let deferred = 0;
      // R-C2-E42: dedup BEFORE the cap, so a backlog past `READ_CAP` is still reachable — each
      // round's cap-worth is unseen mail, never partly spent on ids this account already has. The
      // `unseen` predicate also reaches `api.list` itself (F-3): a paging implementation uses it
      // to know when it has gathered enough NEW mail to stop, but this filter-then-slice is the
      // real cap and runs regardless of what the API already did.
      const unseen = (id: string) => !already.has(`gmail:${id}`);
      const ids = (await deps.api.list(token, query(await deps.excludedLabels(account_id)), unseen))
        .filter(unseen)
        .slice(0, READ_CAP);
      for (const id of ids) {
        const uid = `gmail:${id}`;
        // F-4: bounds when the call about to start would FINISH, not when it starts. The old check
        // (`clock() - started >= budgetMs`) let a call begin a moment before the budget ran out and
        // then run for its own full timeout on top — `PER_CALL_MS` is reserved up front instead, so
        // nothing starts that could not finish inside the budget it was given.
        if (deps.clock() - started + PER_CALL_MS >= deps.budgetMs) {
          more = true;
          break;
        }
        const message = await deps.api.message(token, id);
        const reply: JudgeReply = await judge(account_id, {
          kind: "email",
          item: { message_id: uid, subject: message.subject, from: message.from, date: message.date, text: message.text },
          heuristics_seed: { known_courses: known },
        }, pipeline);
        // The text is out of scope from here: nothing below this line can reach it.
        if (reply.verdict === null) {
          // R-C2-E42: a verdict the pipeline could not reach is neither enqueued nor marked seen
          // — `WINDOW` bounds how long it can keep coming back, so this is a deferral, not a leak.
          deferred += 1;
          if (reply.outcome === "capped") {
            // Every later item in this same round would answer the same way; stop rather than
            // spend the rest of the list finding that out one call at a time.
            more = false;
            break;
          }
          continue;
        }
        const tier = typeof reply.verdict.tier === "string" ? reply.verdict.tier : "information";
        await deps.enqueue(account_id, uid, tier, reply.verdict, reply.judgment_id ?? null);
        await deps.markSeen(account_id, uid);
        read += 1;
      }
      return Response.json({ items: await deps.undelivered(account_id), read, quiet: false, more, deferred });
    } catch (e) {
      if (e instanceof Response) return e;
      // The class and, for a Gmail API failure, its status — never the message, which for a
      // provider's error is the one place a request detail could come back out (§5.6).
      const label = e instanceof GmailApiError
        ? `GmailApiError ${e.status}`
        : e instanceof Error ? e.constructor.name : "unknown";
      console.error(`gmail-read: ${label}`);
      return Response.json({ error: "read failed" }, { status: 500 });
    }
  };
}
