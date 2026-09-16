// The provider seam: which JudgeModel a pinned row's `provider` column builds, and which secret
// it names when the key is missing. No network, no real key — `modelFor` only ever reaches the
// `env` function it is handed.
import { assertEquals, assertThrows } from "@std/assert";
import { AnthropicModel } from "./judge_anthropic.ts";
import { modelFor } from "./judge_provider.ts";
import { OpenRouterModel } from "./judge_openrouter.ts";

Deno.test("an openrouter row builds an OpenRouterModel when OPENROUTER_API_KEY answers", () => {
  const model = modelFor({ provider: "openrouter" }, (name: string) => name === "OPENROUTER_API_KEY" ? "test-key-not-a-secret" : undefined);
  assertEquals(model instanceof OpenRouterModel, true);
});

Deno.test("an openrouter row with no key throws, naming OPENROUTER_API_KEY", () => {
  assertThrows(
    () => modelFor({ provider: "openrouter" }, () => undefined),
    Error,
    "no API key for provider 'openrouter' (set OPENROUTER_API_KEY)",
  );
});

Deno.test("an anthropic row builds an AnthropicModel when ANTHROPIC_API_KEY answers", () => {
  const model = modelFor({ provider: "anthropic" }, (name: string) => name === "ANTHROPIC_API_KEY" ? "test-key-not-a-secret" : undefined);
  assertEquals(model instanceof AnthropicModel, true);
});

Deno.test("an anthropic row with no key throws, naming ANTHROPIC_API_KEY", () => {
  assertThrows(
    () => modelFor({ provider: "anthropic" }, () => undefined),
    Error,
    "no API key for provider 'anthropic' (set ANTHROPIC_API_KEY)",
  );
});

Deno.test("an unknown provider throws, naming itself, without reading any env var", () => {
  const spy = (name: string) => {
    throw new Error(`must not read '${name}' for an unknown provider`);
  };
  assertThrows(
    () => modelFor({ provider: "x" }, spy),
    Error,
    "unknown model provider 'x'",
  );
});

Deno.test("the env function is called only for the chosen provider's own secret name", () => {
  const spy = (name: string) => {
    if (name === "OPENROUTER_API_KEY") return "test-key-not-a-secret";
    throw new Error(`must not read '${name}' when the row is pinned to openrouter`);
  };
  const model = modelFor({ provider: "openrouter" }, spy);
  assertEquals(model instanceof OpenRouterModel, true);
});

Deno.test("modelFor passes timeoutMs through to the built model", () => {
  // Only observable indirectly (the constructed model has no public timeout getter), so this just
  // proves the call does not throw when opts.timeoutMs is supplied.
  const model = modelFor(
    { provider: "anthropic" },
    (name: string) => name === "ANTHROPIC_API_KEY" ? "test-key-not-a-secret" : undefined,
    { timeoutMs: 5_000 },
  );
  assertEquals(model instanceof AnthropicModel, true);
});
