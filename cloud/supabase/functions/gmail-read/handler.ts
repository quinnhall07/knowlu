// POST /gmail-read — the twice-daily read, aligned to the student's slots because the device asks
// for it inside its own slot (cloud design §5.3). Delivery is a pull; nothing is pushed.
//
// The order matters:
//   1. acknowledge what the LAST pull actually wrote (so a crash between reply and write costs a
//      repeat, never a lost task);
//   2. get an access token from the Vault-held refresh token — a revoked grant is `quiet`, never
//      an error;
//   3. list message ids from the last 7 days across the mailbox, minus the account's excluded
//      labels (the routine's "Crimson label and the personal inbox" rule, generalised);
//   4. skip every `gmail:<message-id>` already in `gmail_seen`;
//   5. fetch each remaining message's HEADERS and TEXT PART — never a binary enclosure, ever;
//   6. judge it through the one pipeline with origin 'gmail_api';
//   7. queue the verdict, mark the uid seen, and DISCARD the text;
//   8. stop at the wall-clock budget and say `more`, because an edge function cannot run sixty
//      120-second model calls; the dedup set is the resume cursor.
//
// The message text exists in this function's memory for the duration of one judgment and nowhere
// else: not in `gmail_queue`, not in `judgments`, not in a log line.
import { judge, type JudgeReply, type PipelineDeps } from "../_shared/judge_pipeline.ts";
import type { Entitle } from "../_shared/judge_handler.ts";

export const WINDOW = "newer_than:7d";
/** A bound on one read, so a mailbox with a thousand unread messages cannot eat a slot. */
export const READ_CAP = 60;
/** A bound on one INVOCATION, well under any edge-function wall clock. */
export const READ_BUDGET_MS = 40_000;

export interface GmailApi {
  /** Message ids only. `q` is Gmail's own query language. */
  list(accessToken: string, q: string): Promise<string[]>;
  /** Headers plus the first `text/plain` part, decoded. **Never a binary enclosure.** */
  message(accessToken: string, id: string): Promise<{ subject: string; from: string; date: string; text: string }>;
}

export interface ReadDeps {
  api: GmailApi;
  /** A fresh access token from the Vault-held refresh token, or null when the grant is gone. */
  accessTokenFor(accountId: string): Promise<string | null>;
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
  // A label with a space is quoted the way Gmail's own search does it.
  const terms = excluded.map((l) => `-label:${l.includes(" ") ? `"${l}"` : l}`);
  return [WINDOW, ...terms].join(" ");
}

export function readHandler(entitle: Entitle, deps: ReadDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const body = await req.json().catch(() => ({})) as { ack?: unknown };
      const ack = Array.isArray(body.ack) ? body.ack.filter((u): u is string => typeof u === "string") : [];
      if (ack.length > 0) await deps.deliver(account_id, ack);

      const token = await deps.accessTokenFor(account_id);
      if (token === null) {
        // A revoked or expired grant is not an error the slot should fail on — while the Google
        // project is in Testing the token dies every 7 days by design (§5.3, §9). The DEVICE turns
        // `quiet` into a line the student can act on; here it is one status change.
        await deps.markRevoked(account_id);
        return Response.json({ items: await deps.undelivered(account_id), read: 0, quiet: true, more: false });
      }

      const already = await deps.seen(account_id);
      const known = await deps.knownCourses(account_id);
      const pipeline = await deps.pipeline();
      const started = deps.clock();
      let read = 0;
      let more = false;
      for (const id of (await deps.api.list(token, query(await deps.excludedLabels(account_id)))).slice(0, READ_CAP)) {
        const uid = `gmail:${id}`;
        if (already.has(uid)) continue;
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
        const verdict = reply.verdict ?? { tier: "information", why: "not judged", confidence: 0 };
        const tier = typeof verdict.tier === "string" ? verdict.tier : "information";
        await deps.enqueue(account_id, uid, tier, verdict, reply.judgment_id ?? null);
        await deps.markSeen(account_id, uid);
        read += 1;
      }
      return Response.json({ items: await deps.undelivered(account_id), read, quiet: false, more });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`gmail-read: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "read failed" }, { status: 500 });
    }
  };
}
