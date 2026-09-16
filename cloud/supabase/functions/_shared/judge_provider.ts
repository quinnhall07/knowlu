// The provider seam: one place that turns a pinned row's `provider` column into a `JudgeModel`.
// Everything else (`judge_deps.ts`'s `liveDeps`, `cloud/eval/run_eval.ts`) calls `modelFor` instead
// of naming a provider's client directly, so adding a provider is a case here, not a search over
// every caller.
import { AnthropicModel, type JudgeModel } from "./judge_anthropic.ts";
import { OpenRouterModel } from "./judge_openrouter.ts";

/// The row's own secret name, per provider — named in the thrown error so an operator setting up
/// a new project knows exactly which secret is missing, never a value.
const API_KEY_ENV: Record<string, string> = {
  anthropic: "ANTHROPIC_API_KEY",
  openrouter: "OPENROUTER_API_KEY",
};

export function modelFor(
  row: { provider: string },
  env: (name: string) => string | undefined,
  opts?: { timeoutMs?: number },
): JudgeModel {
  const envName = API_KEY_ENV[row.provider];
  if (envName === undefined) {
    throw new Error(`unknown model provider '${row.provider}'`);
  }
  const apiKey = env(envName);
  if (apiKey === undefined || apiKey === "") {
    throw new Error(`no API key for provider '${row.provider}' (set ${envName})`);
  }
  if (row.provider === "openrouter") {
    return new OpenRouterModel({ apiKey, timeoutMs: opts?.timeoutMs });
  }
  // `baseURL` explicit, always: leaving it out makes the Anthropic SDK's own constructor read
  // `ANTHROPIC_BASE_URL` itself (a default-parameter fallback baked into the SDK, not this
  // codebase's own env discipline) — nothing here sets that variable, so the value is identical
  // either way, but the bare read needs an env permission this service's Deno test sandbox does
  // not grant. Passing the SDK's own default explicitly sidesteps the read entirely.
  return new AnthropicModel({ apiKey, baseURL: "https://api.anthropic.com", timeoutMs: opts?.timeoutMs });
}
