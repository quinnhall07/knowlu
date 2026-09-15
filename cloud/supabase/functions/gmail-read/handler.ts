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
// The message text exists in this function's memory for the duration of one judgment and nowhere
// else: not in `gmail_queue`, not in `judgments`, not in a log line.
import { judge, type JudgeReply, type PipelineDeps } from "../_shared/judge_pipeline.ts";
import type { Entitle } from "../_shared/judge_handler.ts";
import { GOOGLE_NOT_CONFIGURED } from "../_shared/google_scopes.ts";

export const WINDOW = "newer_than:7d";
/** A bound on one read, so a mailbox with a thousand unread messages cannot eat a slot. */
export const READ_CAP = 60;
/** A bound on one INVOCATION, well under any edge-function wall clock. */
export const READ_BUDGET_MS = 40_000;

/** A Gmail API call that failed. The status is kept because it reaches a log line; the response
 * body is not, because a Gmail error body is not ours to log (§5.6's rule, applied here too). */
export class GmailApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = "GmailApiError";
  }
}

export interface GmailApi {
  /** Message ids only. `q` is Gmail's own query language. */
  list(accessToken: string, q: string): Promise<string[]>;
  /** Headers plus the first `text/plain` part, decoded. **Never a binary enclosure.** */
  message(accessToken: string, id: string): Promise<{ subject: string; from: string; date: string; text: string }>;
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

export function readHandler(entitle: Entitle, deps: ReadDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const body = await req.json().catch(() => ({})) as { ack?: unknown };
      const ack = Array.isArray(body.ack)
        ? body.ack.filter((u): u is string => typeof u === "string" && ACK_SHAPE.test(u))
        : [];
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
      // round's cap-worth is unseen mail, never partly spent on ids this account already has.
      const ids = (await deps.api.list(token, query(await deps.excludedLabels(account_id))))
        .filter((id) => !already.has(`gmail:${id}`))
        .slice(0, READ_CAP);
      for (const id of ids) {
        const uid = `gmail:${id}`;
        if (deps.clock() - started >= deps.budgetMs) {
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
