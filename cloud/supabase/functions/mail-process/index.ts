/**
 * The wire for `mail-process` (email plan T3d): the environment, the store over PostgREST, S3 and the
 * promoted rules, then `handler.ts`. `verify_jwt = false` (config.toml): the cron and intake's kick send
 * `x-knowlu-job-token`, and that token is the check. Secrets, named only: `MAIL_JOBS_TOKEN`, `MAIL_BUCKET`,
 * `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and the pinned provider's key
 * (`OPENROUTER_API_KEY` for today's `mail_claim` row).
 *
 * The model path (§4.7 steps 4–6, T3e) is `model.ts` over the shared judgment tables: the `mail_claim` pin,
 * the charges (`charge_allowance`, `refund_allowance`, `charge_call`, `enforce_budget`, `record_tokens`)
 * and the judgment sink. The sweep's reads and deletes are below, by age, across every account (P7).
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is
 * cancelled unread, and the handler logs only that a store call failed.
 */
import { type Rest, restFromEnv } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { judgmentSink } from "../_shared/judge_log.ts";
import { modelRow } from "../_shared/judge_models.ts";
import { modelFor } from "../_shared/judge_provider.ts";
import { ruleTable } from "../_shared/judge_rules.ts";
import { s3Client } from "../_shared/s3.ts";
import {
  handle,
  type Mailbox,
  type ModelInput,
  type ProcessConfig,
  type ProcessDeps,
  type WorkItem,
} from "./handler.ts";
import { mailCharges, modelPath, type ModelPathDeps } from "./model.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

/** One PostgREST call; a failure throws a fixed message and its body is never read. */
async function call(rest: Rest, what: string, path: string, init: RequestInit = {}, prefer?: string) {
  const res = await rest.fetch(`${rest.url}/rest/v1/${path}`, {
    ...init,
    headers: {
      apikey: rest.serviceKey,
      authorization: `Bearer ${rest.serviceKey}`,
      "content-type": "application/json",
      ...(prefer === undefined ? {} : { prefer }),
    },
  });
  if (!res.ok) {
    await res.body?.cancel();
    throw new Error(`store ${what} ${res.status}`);
  }
  return res;
}
const rows = async <T>(res: Response) => await res.json() as T[];
const q = (s: string) => encodeURIComponent(s);
/** A delete or patch answered with its rows' ids (`return=representation`), counted. */
const touched = async (res: Response) => (await rows(res)).length;
/** One model call's bound: the run's budget plus one call in flight stays inside the platform's limit. */
const CALL_MS = 30_000;

interface WorkDb {
  id: number;
  account_id: string;
  address_id: string;
  batch_id: string | null;
  s3_key: string;
  phase: WorkItem["phase"];
  authenticity: WorkItem["authenticity"];
  arrived_at: string;
  next_attempt_at: string;
  attempts: number;
  mail_batches: { opened_at: string } | null;
}
const WORK_COLUMNS =
  "id,account_id,address_id,batch_id,s3_key,phase,authenticity,arrived_at,next_attempt_at," +
  "attempts,mail_batches(opened_at)";

/** A course note's `code:` frontmatter line, unquoted; null without one. */
function courseCode(body: string): string | null {
  const head = /^---\r?\n([\s\S]*?)\r?\n---/.exec(body)?.[1] ?? "";
  const line = /^code:[ \t]*(.+?)[ \t]*$/m.exec(head)?.[1] ?? "";
  const code = line.replace(/^(["'])(.*)\1$/, "$2").trim();
  return code === "" ? null : code;
}

function wire(): { cfg: ProcessConfig; deps: ProcessDeps } {
  const rest = restFromEnv();
  const cfg: ProcessConfig = { jobsToken: env("MAIL_JOBS_TOKEN") };
  const s3 = s3Client({
    bucket: env("MAIL_BUCKET"),
    region: env("AWS_REGION"),
    accessKeyId: env("AWS_ACCESS_KEY_ID"),
    secretAccessKey: env("AWS_SECRET_ACCESS_KEY"),
  }, { fetch: globalThis.fetch });
  const db = sharedDb();
  let pinned: ReturnType<ModelPathDeps["pin"]> | null = null;
  const pin = () => {
    pinned ??= modelRow(db, "mail_claim").then((row) => ({
      row,
      model: modelFor(row, (name) => Deno.env.get(name), { timeoutMs: CALL_MS }),
    }));
    pinned.catch(() => (pinned = null));
    return pinned;
  };
  let path: ((input: ModelInput) => Promise<void>) | null = null;
  const deps: ProcessDeps = {
    s3,
    rules: ruleTable(db),
    now: () => Date.now(),
    // Built at first use, once: it reads `deps.store`, defined below.
    model: (input) =>
      (path ??= modelPath({
        store: deps.store,
        s3,
        now: deps.now,
        charges: mailCharges(db),
        log: judgmentSink(db),
        pin,
      }))(
        input,
      ),
    store: {
      async due(now, limit) {
        const path = `mail_work?next_attempt_at=lte.${q(now)}&select=${WORK_COLUMNS}` +
          `&order=arrived_at.asc,id.asc&limit=${limit}`;
        return (await rows<WorkDb>(await call(rest, "due", path))).map((r) => ({
          id: String(r.id),
          accountId: r.account_id,
          addressId: r.address_id,
          batchId: r.batch_id,
          s3Key: r.s3_key,
          phase: r.phase,
          authenticity: r.authenticity,
          arrivedAt: r.arrived_at,
          nextAttemptAt: r.next_attempt_at,
          attempts: r.attempts,
          batchOpenedAt: r.mail_batches?.opened_at ?? null,
        }));
      },
      async lease(item, until) {
        const path = `mail_work?id=eq.${q(item.id)}&next_attempt_at=eq.${q(item.nextAttemptAt)}&select=id`;
        const init = { method: "PATCH", body: JSON.stringify({ next_attempt_at: until }) };
        return (await rows(await call(rest, "lease", path, init, "return=representation"))).length > 0;
      },
      async mailboxes(account) {
        const path = `mail_addresses?account_id=eq.${q(account)}&select=id,provider,mailbox_hint&limit=20`;
        const got = await rows<{ id: string; provider: Mailbox["provider"]; mailbox_hint: string | null }>(
          await call(rest, "mailboxes", path),
        );
        return got.map((r) => ({ id: r.id, provider: r.provider, mailboxHint: r.mailbox_hint }));
      },
      async knownCourses(account) {
        // §4.7 step 4: the synced `courses/` notes' codes, else the account's judgments, as `gmail-read`.
        const notes = await rows<{ body: string | null }>(
          await call(
            rest,
            "courses",
            `sync_notes?account_id=eq.${
              q(account)
            }&path=like.courses/*&deleted=is.false&select=body&limit=200`,
          ),
        );
        const codes = new Set(
          notes.map((n) => n.body === null ? null : courseCode(n.body)).filter((c) => c !== null),
        );
        if (codes.size > 0) return [...codes] as string[];
        const since = new Date(Date.now() - 120 * 86_400_000).toISOString();
        const judged = await rows<{ fields: Record<string, unknown> | null }>(
          await call(
            rest,
            "judged courses",
            `judgments?account_id=eq.${q(account)}&judged_at=gte.${
              q(since)
            }&select=fields&order=judged_at.desc&limit=1000`,
          ),
        );
        const out = new Set<string>();
        for (const j of judged) {
          const c = j.fields?.course;
          if (typeof c === "string" && c !== "" && c !== "null") out.add(c);
        }
        return [...out];
      },
      async queue(r) {
        const body = {
          account_id: r.accountId,
          address_id: r.addressId,
          batch_id: r.batchId,
          uid: r.uid,
          message_date: r.messageDate,
          ordering_date: r.orderingDate,
          sender_name: r.senderName,
          sender_addr: r.senderAddr,
          phase: r.phase,
          authenticity: r.authenticity,
          claims: r.claims,
          judgment_id: r.judgmentId,
        };
        const init = { method: "POST", body: JSON.stringify(body) };
        await (await call(rest, "queue", "mail_claims", init, "return=minimal")).body?.cancel();
      },
      async finish(id) {
        await (await call(rest, "finish", `mail_work?id=eq.${q(id)}`, { method: "DELETE" })).body?.cancel();
      },
      async count(address, counter) {
        const init = { method: "POST", body: JSON.stringify({ p_address: address, p_counter: counter }) };
        await (await call(rest, "count", "rpc/count_mail_arrival", init)).body?.cancel();
      },
      async wait(id, until, attempts) {
        const init = { method: "PATCH", body: JSON.stringify({ next_attempt_at: until, attempts }) };
        await (await call(rest, "wait", `mail_work?id=eq.${q(id)}`, init, "return=minimal")).body?.cancel();
      },
      async expire(id) {
        const init = { method: "POST", body: JSON.stringify({ p_work: Number(id) }) };
        return await (await call(rest, "expire", "rpc/expire_mail_work", init)).json() === true;
      },
      async expirable(arrivedBy, limit) {
        const path = `mail_work?arrived_at=lte.${
          q(arrivedBy)
        }&select=id,s3_key&order=arrived_at.asc,id.asc&limit=${limit}`;
        const got = await rows<{ id: number; s3_key: string }>(await call(rest, "expirable", path));
        return got.map((r) => ({ id: String(r.id), s3Key: r.s3_key }));
      },
      async closeBatches(now, sentBy, openedBy) {
        const timedOut = q(`(sent_at.lte."${sentBy}",opened_at.lte."${openedBy}")`);
        const path = `mail_batches?closed_at=is.null&or=${timedOut}&select=id`;
        const init = { method: "PATCH", body: JSON.stringify({ closed_at: now }) };
        return await touched(await call(rest, "close batches", path, init, "return=representation"));
      },
      async pruneSeen(seenBy) {
        const path = `mail_seen?seen_at=lte.${q(seenBy)}&select=address_id`;
        return await touched(
          await call(rest, "prune seen", path, { method: "DELETE" }, "return=representation"),
        );
      },
      async pruneClaims(deliveredBy, queuedBy) {
        const del = { method: "DELETE" };
        const delivered = `mail_claims?delivered_at=lte.${q(deliveredBy)}&select=id`;
        const stale = `mail_claims?delivered_at=is.null&queued_at=lte.${q(queuedBy)}&select=id`;
        return {
          delivered: await touched(
            await call(rest, "prune delivered", delivered, del, "return=representation"),
          ),
          undelivered: await touched(
            await call(rest, "prune undelivered", stale, del, "return=representation"),
          ),
        };
      },
      async pruneBatches(closedBy) {
        const path = `mail_batches?closed_at=lte.${q(closedBy)}&select=id`;
        return await touched(
          await call(rest, "prune batches", path, { method: "DELETE" }, "return=representation"),
        );
      },
      async pruneCanaries(sentBy) {
        const path = `mail_canaries?sent_at=lte.${q(sentBy)}&select=id`;
        return await touched(
          await call(rest, "prune canaries", path, { method: "DELETE" }, "return=representation"),
        );
      },
    },
  };
  return { cfg, deps };
}

let wired: { cfg: ProcessConfig; deps: ProcessDeps } | null = null;

Deno.serve(async (req) => {
  try {
    wired ??= wire();
    return await handle(req, wired.cfg, wired.deps);
  } catch (e) {
    wired = null;
    return asResponse(e);
  }
});
