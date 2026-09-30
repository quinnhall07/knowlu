// One sequential pass over the corpus against the decisions endpoint (procedure §Step 4: one pass,
// logging the request, the full answer, the latency and the usage for each item). The transport is
// injected as `send`, so the loop — per-item error recording, and the stop after five consecutive
// failures — is tested without a network. `run.ts` supplies the real `fetch`; the key lives only in
// that closure's headers and never in anything this module records.

import { buildDecisionsBody } from "./jev_request.ts";
import { type ItemResult, parseDecisionReply } from "./score.ts";
import type { CorpusFinding } from "./types.ts";

export type SendFn = (body: unknown) => Promise<{ status: number; text: string }>;

export const MAX_CONSECUTIVE_FAILURES = 5;

/** Belt and braces: an error string never carries anything shaped like an OpenRouter key. */
export function redactKeys(text: string): string {
  return text.replace(/sk-or-[A-Za-z0-9_-]+/g, "[redacted]");
}

export interface PassResult {
  results: ItemResult[];
  stoppedEarly: boolean;
}

export async function runPass(
  findings: CorpusFinding[],
  send: SendFn,
  opts: { log?: (line: string) => void; maxConsecutiveFailures?: number } = {},
): Promise<PassResult> {
  const log = opts.log ?? console.log;
  const limit = opts.maxConsecutiveFailures ?? MAX_CONSECUTIVE_FAILURES;
  const results: ItemResult[] = [];
  let consecutive = 0;
  for (const f of findings) {
    const request = buildDecisionsBody(f);
    const base = { id: f.id, set: f.set, severity: f.severity, disposition: f.disposition, request };
    const started = performance.now();
    let item: ItemResult;
    try {
      const { status, text } = await send(request);
      const elapsedMs = performance.now() - started;
      const parsed = parseDecisionReply(status, text);
      item = parsed.ok
        ? { ...base, status, elapsedMs, reply: parsed.reply }
        : { ...base, status, elapsedMs, error: redactKeys(parsed.error) };
    } catch (e) {
      item = {
        ...base,
        status: null,
        elapsedMs: performance.now() - started,
        error: redactKeys(`network: ${e instanceof Error ? e.message : String(e)}`),
      };
    }
    results.push(item);
    if (item.reply) {
      consecutive = 0;
      log(
        `${f.id}: HTTP ${item.status} in ${
          item.elapsedMs.toFixed(0)
        }ms — grade ${item.reply.grade.choice}, ` +
          `disposition ${item.reply.disposition.choice}, blocks ${item.reply.blocks.toFixed(2)}`,
      );
    } else {
      consecutive++;
      log(`${f.id}: FAILED — ${item.error}`);
      if (consecutive >= limit) {
        log(`stopping: ${limit} consecutive failures.`);
        return { results, stoppedEarly: true };
      }
    }
  }
  return { results, stoppedEarly: false };
}
