/**
 * `POST /mail-resolve` (email spec D12, §4.8 step 3, §6.3; plan T4.5): one course's ambiguous claims, the
 * course's candidate items, and the reasoning model's answer to which candidate each claim means, or none.
 * Session plus entitlement, like `mail-pull`.
 *
 * **The wire** (plan T4.5). `{course, claims: [{uid, index}, ...], candidates: [{id, title, due}, ...]}`:
 * the course (one line, 1 to 80 characters), read and bounded only, since each stored claim holds its own;
 * each claim by its message's uid (`mail:` and 32 hex) and its 0-based index below D9's five, at most
 * `MAX_CLAIMS`, once each; each candidate a note's opaque id (one line, at most 80 characters, once each), its
 * title (at most 500) and its due (a string or null), at most `MAX_CANDIDATES`. Nothing else is accepted.
 * The answer is `{answers: [{uid, index, target_id}]}`, one per claim, in the request's order, `target_id`
 * null for none. Inside, a claim is D20's key `mail:<h>:<n>`, the form the prompt and the model answer in.
 *
 * **What is asked.** The server reads each claim from the account's own `mail_claims` row, never from the
 * request: what it names (a change's or a receipt's `target`, a new item's title) and, for a new item, its
 * due. A key that names no claim of the account's, or a claim with nothing to resolve (`course_info`,
 * `noise`), reads none and is never asked or written. A claim already holding a stored answer replays it.
 *
 * **The order** is the one pipeline's (`judge_pipeline.ts`): the pin, then the monthly budget, then the day's
 * `mail_resolve` cap (`charge_call`), then one model call for the claims left, then its tokens, then the
 * check (`validateMailResolve`: an id outside the batch, an unanswered claim or a claim answered two ways
 * reads none), then one `judgments` row (kind `mail_resolve`, origin `forward`, counts only), then the
 * answers stored on their claims. Nothing left to ask, or no candidate: no call, no charge, no row.
 *
 * **What is stored** (D12: the model's answer is data): `resolution: {target_id}` on each asked claim of an
 * answered call, the model's nones included. A capped, over-budget, failed, refused or truncated call stores
 * nothing (429 `capped` / `over budget`, 502 with the cause), so the device asks again at a later slot. A
 * `model failed` call gives its day's-cap charge back by main's F1 `refund_call`; a refusal or a truncation
 * reached the model and stays charged (F1's rule, plan §9.5). A stored answer is never overwritten or
 * erased: the write carries the whole claims array, so it lands only while every claim unanswered at its
 * read is still unanswered, and an answer another call stored first, on any claim, wins and is kept.
 *
 * **Privacy:** nothing here logs; the judgment row holds counts, never a title, target or id.
 */
import type { Entitle } from "../_shared/judge_handler.ts";
import { type JudgeModel, ModelRefused } from "../_shared/judge_anthropic.ts";
import type { JudgmentRow, JudgmentSink } from "../_shared/judge_log.ts";
import type { ModelRow } from "../_shared/judge_models.ts";
import { buildMailResolvePrompt, mailPromptHash } from "../_shared/judge_prompts.ts";
import { type Cause, MAX_MAIL_CLAIMS, validateMailResolve } from "../_shared/judge_validate.ts";
import { asResponse, fail, json, methodNotAllowed } from "../_shared/http.ts";

/** D20's claim key: a message uid and the claim's index, below D9's five. */
export const CLAIM_KEY = new RegExp(`^(mail:[0-9a-f]{32}):([0-${MAX_MAIL_CLAIMS - 1}])$`);
/** The most claims one call asks: T18b's 15-claim batch outran 640 output tokens, and the pin allows 1280. */
export const MAX_CLAIMS = 15;
/** The most candidates one call carries: a course's active items. */
export const MAX_CANDIDATES = 200;
const ID_CHARS = 80;
const COURSE_CHARS = 80;
/** A message uid on the wire, as `mail-pull` acks it. */
const UID_SHAPE = /^mail:[0-9a-f]{32}$/;
const TITLE_CHARS = 500;
const BODY_LIMIT = 256 * 1024;

/** A `mail_claims` row as the resolve reads it: its id, its message uid and its claims array. */
export interface ClaimRowRef {
  id: number;
  uid: string;
  claims: unknown[];
}
export interface ResolveStore {
  /** The account's rows whose uid is listed, in id order. */
  rows(accountId: string, uids: string[]): Promise<ClaimRowRef[]>;
  /** Writes `claims` over the account's row `id` only while each index in `open` (every claim that had no
   * resolution when the row was read) still has none; false, with nothing written, otherwise. */
  answer(accountId: string, id: number, open: number[], claims: unknown[]): Promise<boolean>;
}
/** The pipeline's guards for `mail_resolve`, each one statement in SQL; an unreachable one refuses. */
export interface ResolveCharges {
  /** `enforce_budget` at `MONTHLY_CEILING_USD`. */
  withinBudget(account: string): Promise<boolean>;
  /** `charge_call` for `mail_resolve` at its daily cap. */
  charge(account: string): Promise<boolean>;
  /** `record_tokens` for `mail_resolve`; never throws. */
  recordTokens(account: string, inTokens: number, outTokens: number): Promise<void>;
  /** Main's F1 `refund_call` for `mail_resolve`: one call back after a call the model never answered. Not in
   * this branch's base, so until main is merged it fails and refuses, keeping the charge (§9.5, T17). */
  refundDaily(account: string): Promise<boolean>;
}
export interface ResolveDeps {
  /** A session and an active entitlement (`requireActiveEntitlement`). */
  entitle: Entitle;
  store: ResolveStore;
  charges: ResolveCharges;
  log: JudgmentSink;
  /** The `mail_resolve` pin and its client. */
  pin(): Promise<{ row: ModelRow; model: JudgeModel }>;
  now(): number;
}

interface Candidate {
  id: string;
  title: string;
  due: string | null;
}
interface Batch {
  claims: string[];
  candidates: Candidate[];
}
type Answer = { target_id: string | null };

const isRecord = (v: unknown): v is Record<string, unknown> =>
  v !== null && typeof v === "object" && !Array.isArray(v);
/** No C0 control, DEL, U+0085, U+2028 or U+2029: a candidate id goes on one prompt line and back to a note. */
function oneLine(s: string): boolean {
  for (const ch of s) {
    const c = ch.codePointAt(0) ?? 0;
    if (c < 0x20 || c === 0x7f || c === 0x85 || c === 0x2028 || c === 0x2029) return false;
  }
  return true;
}
const bad = (why: string) => fail(400, why);

function readCandidate(v: unknown): Candidate {
  if (!isRecord(v) || Object.keys(v).some((k) => !["id", "title", "due"].includes(k))) {
    throw bad("a candidate is {id, title, due}");
  }
  const { id, title, due } = v;
  if (typeof id !== "string" || id === "" || id.length > ID_CHARS || !oneLine(id)) {
    throw bad(`a candidate id is one line of 1 to ${ID_CHARS} characters`);
  }
  if (typeof title !== "string" || title.length > TITLE_CHARS) {
    throw bad(`a candidate title is a string of at most ${TITLE_CHARS} characters`);
  }
  if (due !== null && typeof due !== "string") throw bad("a candidate due is a string or null");
  return { id, title, due: due ?? null };
}

/** The body: exactly `course`, `claims` and `candidates`, each bounded and without repeats; the claims as D20
 * keys. Throws a 400 or a 413. */
async function readBatch(req: Request): Promise<Batch> {
  const text = await req.text();
  if (text.length > BODY_LIMIT) throw fail(413, `body over ${BODY_LIMIT} characters`);
  let body: unknown;
  try {
    body = JSON.parse(text);
  } catch {
    throw bad("body is not JSON");
  }
  if (!isRecord(body) || Object.keys(body).some((k) => !["course", "claims", "candidates"].includes(k))) {
    throw bad("the body is {course, claims, candidates}");
  }
  const { course, claims: refs, candidates } = body;
  if (typeof course !== "string" || course === "" || course.length > COURSE_CHARS || !oneLine(course)) {
    throw bad(`course is one line of 1 to ${COURSE_CHARS} characters`);
  }
  if (!Array.isArray(refs) || refs.length === 0 || refs.length > MAX_CLAIMS) {
    throw bad(`claims lists 1 to ${MAX_CLAIMS} claims`);
  }
  const claims = refs.map((r) => {
    if (
      !isRecord(r) || Object.keys(r).length !== 2 || Object.keys(r).some((k) => k !== "uid" && k !== "index")
    ) {
      throw bad("a claim is {uid, index}");
    }
    const { uid, index } = r;
    if (typeof uid !== "string" || !UID_SHAPE.test(uid)) throw bad("a claim is {uid, index}");
    if (!Number.isInteger(index) || (index as number) < 0 || (index as number) > MAX_MAIL_CLAIMS - 1) {
      throw bad("a claim is {uid, index}");
    }
    return `${uid}:${index}`;
  });
  if (new Set(claims).size !== claims.length) throw bad("a claim is listed once");
  if (!Array.isArray(candidates) || candidates.length > MAX_CANDIDATES) {
    throw bad(`candidates lists at most ${MAX_CANDIDATES} items`);
  }
  const read = candidates.map(readCandidate);
  if (new Set(read.map((c) => c.id)).size !== read.length) throw bad("a candidate id is listed once");
  return { claims, candidates: read };
}

/** What a stored claim names and its due, when it is a claim a resolve can answer; else null. */
function askable(claim: unknown): { names: string; due: string | null } | null {
  if (!isRecord(claim)) return null;
  const { kind, target, item } = claim;
  if ((kind === "change" || kind === "receipt") && typeof target === "string" && target.trim() !== "") {
    // A change's `value` is the new due, which names nothing about the item it moves.
    return { names: target, due: null };
  }
  if (kind === "new_item" && isRecord(item) && typeof item.title === "string" && item.title.trim() !== "") {
    return { names: item.title, due: typeof item.due === "string" ? item.due : null };
  }
  return null;
}

/** True while a stored claim has no resolution at all, as the SQL's `claims->i->>resolution is null` reads
 * it (a JSON null, a missing key, or an element that is no object). */
function unresolved(claim: unknown): boolean {
  return !isRecord(claim) || claim.resolution === null || claim.resolution === undefined;
}

/** A claim's stored answer, `{target_id: <id> | null}`; undefined while it has none. */
function storedAnswer(claim: unknown): Answer | undefined {
  if (!isRecord(claim) || !isRecord(claim.resolution)) return undefined;
  const id = claim.resolution.target_id;
  return typeof id === "string" || id === null ? { target_id: id } : undefined;
}

function failureCause(e: unknown): Cause {
  if (e instanceof ModelRefused) return "refused";
  if (e instanceof Error && e.message.includes("max_tokens")) return "truncated";
  return "model failed";
}

/** The most writes one message's answers try before reading what another call stored as final. */
const WRITE_TRIES = 3;

/**
 * Stores `fills` (claim index -> answer) on every row of one message, each into a claim still unanswered,
 * and answers what each claim holds afterwards: ours, or one another call stored first.
 */
async function storeAnswers(
  deps: ResolveDeps,
  account: string,
  uid: string,
  read: ClaimRowRef[],
  fills: Map<number, Answer>,
): Promise<Map<number, Answer>> {
  let rows = read;
  for (let tries = 0; tries < WRITE_TRIES; tries++) {
    let lost = false;
    for (const row of rows) {
      const claims = structuredClone(row.claims);
      // Every claim unanswered at this read, filled here or not: the write carries the whole array, so an
      // answer another call stores on any of them in between must refuse it, never be erased by it.
      const open = claims.flatMap((c, i) => unresolved(c) ? [i] : []);
      let filled = false;
      for (const [i, answer] of fills) {
        if (!isRecord(claims[i]) || !unresolved(claims[i])) continue;
        claims[i] = { ...claims[i], resolution: answer };
        filled = true;
      }
      if (filled && !await deps.store.answer(account, row.id, open, claims)) lost = true;
    }
    if (!lost) break;
    rows = await deps.store.rows(account, [uid]);
  }
  // The first row read is the message's; a stored answer there wins over ours.
  const held = rows[0]?.claims ?? [];
  return new Map([...fills].map(([i, answer]) => [i, storedAnswer(held[i]) ?? answer]));
}

interface Ask {
  key: string;
  uid: string;
  index: number;
  names: string;
  due: string | null;
}

/** The pipeline for the claims left to ask: an answer per key, or the Response a refusal answers. */
async function askModel(
  deps: ResolveDeps,
  account: string,
  ask: Ask[],
  candidates: Candidate[],
): Promise<Map<string, Answer> | Response> {
  const started = deps.now();
  // The pin before any charge: a missing row or key is a deployment fault that must not spend a call.
  const { row, model } = await deps.pin();
  const base: JudgmentRow = {
    account_id: account,
    kind: "mail_resolve",
    item_id: ask[0].key,
    tier: 0,
    outcome: "capped",
    cause: null,
    confidence: 0,
    fields: {},
    model: null,
    prompt_version: null,
    grammar_version: null,
    prompt_hash: null,
    ms: 0,
    origin: "forward",
  };
  const record = (r: Partial<JudgmentRow>) => deps.log.write({ ...base, ...r, ms: deps.now() - started });
  if (!await deps.charges.withinBudget(account)) {
    await record({});
    return fail(429, "over budget");
  }
  if (!await deps.charges.charge(account)) {
    await record({});
    return fail(429, "capped");
  }
  const prompt = buildMailResolvePrompt({
    claims: ask.map((a) => ({ key: a.key, target: a.names, due: a.due })),
    candidates,
  });
  const pinned = {
    model: row.model_id,
    prompt_version: row.prompt_version,
    grammar_version: row.grammar_version,
    prompt_hash: await mailPromptHash("mail_resolve"),
    tier: 3,
  };
  let reply;
  try {
    reply = await model.complete({
      model: row.model_id,
      system: prompt.system,
      user: prompt.user,
      schema: prompt.schema,
      maxTokens: row.max_tokens,
      sampling: row.sampling,
      route: row.route,
    });
  } catch (e) {
    // The error's text is never carried anywhere: a provider's message is where a prompt can come back.
    const cause = failureCause(e);
    await record({ ...pinned, outcome: "low confidence", cause });
    // No answer: the day's-cap charge is given back (main's F1 rule); a refusal or truncation keeps it.
    if (cause === "model failed") await deps.charges.refundDaily(account);
    return fail(502, cause);
  }
  await deps.charges.recordTokens(account, reply.inputTokens, reply.outputTokens);
  const said = validateMailResolve(reply.json, {
    claims: ask.map((a) => a.key),
    candidates: candidates.map((c) => c.id),
  });
  // A resolve answer carries no confidence of its own; the row records the counts.
  await record({
    ...pinned,
    outcome: "answered",
    fields: { claims: String(ask.length), named: String(said.filter((s) => s.target_id !== null).length) },
  });
  return new Map(ask.map((a, i) => [a.key, said[i]]));
}

export function resolveHandler(deps: ResolveDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      if (req.method !== "POST") return methodNotAllowed(["POST"]);
      const { account_id } = await deps.entitle(req);
      const batch = await readBatch(req);
      const keys = batch.claims.map((key) => {
        const [, uid, n] = CLAIM_KEY.exec(key) ?? [];
        return { key, uid, index: Number(n) };
      });
      const rows = await deps.store.rows(account_id, [...new Set(keys.map((k) => k.uid))]);
      const rowsOf = (uid: string) => rows.filter((r) => r.uid === uid);
      // Each claim: none when it is not the account's or names nothing; its stored answer; else asked.
      const answers = new Map<string, Answer>();
      const ask: Ask[] = [];
      for (const k of keys) {
        const claim = rowsOf(k.uid)[0]?.claims[k.index];
        const named = askable(claim);
        const kept = storedAnswer(claim);
        if (named === null) answers.set(k.key, { target_id: null });
        else if (kept !== undefined) answers.set(k.key, kept);
        else ask.push({ ...k, ...named });
      }
      // With no candidate there is nothing to choose: none, unasked and unstored.
      if (ask.length > 0 && batch.candidates.length > 0) {
        const said = await askModel(deps, account_id, ask, batch.candidates);
        if (said instanceof Response) return said;
        for (const uid of new Set(ask.map((a) => a.uid))) {
          const fills = new Map(ask.filter((a) => a.uid === uid).map((a) => [a.index, said.get(a.key)!]));
          const held = await storeAnswers(deps, account_id, uid, rowsOf(uid), fills);
          for (const a of ask) if (a.uid === uid) answers.set(a.key, held.get(a.index)!);
        }
      }
      return json(200, {
        answers: keys.map(({ key, uid, index }) => ({
          uid,
          index,
          target_id: answers.get(key)?.target_id ?? null,
        })),
      });
    } catch (e) {
      return asResponse(e);
    }
  };
}
