# Inference Provider Swap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every judgment from Anthropic Haiku 4.5 to two open-weights models on zero-retention endpoints reached through OpenRouter, pinned per kind, at roughly one seventh of the cost, and ship the four fixes the scoping study found regardless of provider.

**Architecture:** The pipeline already speaks to a model through one interface (`JudgeModel.complete(ModelRequest) → ModelReply`) and reads the pin from a `models` row per kind. This plan adds a second implementation of that interface (an OpenAI-compatible client with OpenRouter's provider-pinning and reasoning controls), lets the dependency builder and the eval runner choose the implementation by the row's `provider`, re-pins the three rows by migration, and bumps the prompts. No schema of any verdict changes; the validator, the caps, the logging and the device are untouched.

**Tech Stack:** Deno edge functions (plain `fetch`, no new npm dependency), Postgres migrations (Supabase CLI), Rust tests that pin the workflow, the static site.

**Spec:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (ruling R8 as amended by Quinn on 2026-09-16 — option 1 of `docs/notes/2026-09-16-inference-provider-and-model-scoping.md` §8), with the four research reports `docs/reports/2026-09-16-inference-scoping-*.md` and the host check preserved beside them.

**Status:** written 2026-09-16 by the controller from the scoping study; executes on branch `provider-swap` in a worktree; merges on Quinn's word.

## Global Constraints

- Every rule in `CLAUDE.md`: 0 warnings is part of green; LF everywhere; nothing under `engine/tests/fixtures/`; no single-user assumptions; no secret in the repo, a log, a prompt or a test name; every child process `.no_console()`; applied migrations are never edited (comment-only corrections allowed); every schema change is a NEW migration; the migration guard (`cloud/supabase/migrations/migrations_test.ts`) and the account-scoping guard (`_shared/judge_db_test.ts`) stay green with their pins moved only with a stated reason.
- The two pins, exactly: **task** and **event** → OpenRouter model id `ibm-granite/granite-4.2-8b`, upstream `CoreWeave` (bf16, `structured_outputs` supported, `reasoning_effort` supported), list price **$0.10 in / $0.15 out per million tokens**; **email** → `qwen/qwen3.5-35b-a3b`, upstream `DeepInfra` (fp8, zero retention), **$0.14 / $1.00**. Every request carries `provider: { order: [<that one upstream>], allow_fallbacks: false, zdr: true, require_parameters: true }` and `reasoning: { enabled: false }`. The fallback email pin, used only if the live check in Task 5 fails on Qwen, is `deepseek/deepseek-v4-flash-0731` at `DeepInfra` (fp8, zero retention, $0.06 / $0.18).
- Secret names: the functions read **`OPENROUTER_API_KEY`**; the eval gate reads the GitHub secret **`OPENROUTER_API_KEY`**. `ANTHROPIC_API_KEY` stays readable only when a row's `provider` is `anthropic` (no row is, after Task 2). No task sets, reads or prints a secret value; Quinn sets both.
- OpenRouter's endpoint is `https://openrouter.ai/api/v1/chat/completions`, OpenAI-compatible, called with plain `fetch`; every request carries `HTTP-Referer: https://knowlu.com` and `X-Title: Knowlu`.
- `max_tokens` stays 256 / 256 / 640. `MONTHLY_CEILING_USD` becomes **2.0** (the heavy persona on the new pins is about $0.87).
- Prompt versions bump to `task-2`, `event-2`, `email-2` (Task 3 changes every prompt's clip marker; email also gains the date rule and the empty-body line).
- The training export (`export_training_rows(p_since)` in `20260911000200_google.sql`) must exclude every `gmail_api` row and every `events` row: the `origin` vocabulary (`device`, `gmail_api`, `events`) cannot tell a Google-Calendar event from an ICS one, so the conservative exclusion is the ruled one until an origin split exists (a C4 refinement, noted in HANDOFF).
- Tests: TDD (the failing test first); the Deno gate exactly as CI runs it plus `deno check` and `deno lint`; `cargo test --workspace` at 0 warnings beyond the accepted `.rsrc` line.
- Commit messages from a file outside the worktree, specific `git add`, ending with the two trailers `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z`. No push by an implementer; the controller pushes and watches CI.

## File structure

- Create `cloud/supabase/functions/_shared/judge_openrouter.ts` — `OpenRouterModel implements JudgeModel` and its request builder (pure, exported for tests).
- Create `cloud/supabase/functions/_shared/judge_openrouter_test.ts` — the adapter's tests against a fake `fetch`.
- Create `cloud/supabase/functions/_shared/judge_provider.ts` — `modelFor(row, env)`: the one place that maps a row's `provider` to a client and its secret name. Used by `judge_deps.ts` and `cloud/eval/run_eval.ts`.
- Modify `cloud/supabase/functions/_shared/judge_anthropic.ts` — `Sampling` widens to `Record<string, unknown>`; `ModelRequest` gains `route: Record<string, unknown>`; nothing else.
- Modify `cloud/supabase/functions/_shared/judge_models.ts` — the row gains `route` and `precision`.
- Modify `cloud/supabase/functions/_shared/judge_deps.ts` — builds the model through `judge_provider.ts`.
- Modify `cloud/supabase/functions/_shared/judge_pipeline.ts` — passes `route: deps.row.route` into the request.
- Modify `cloud/supabase/functions/_shared/judge_caps.ts` — the ceiling.
- Create `cloud/supabase/migrations/20260916000100_provider_swap.sql` — two columns, three re-pinned rows, the export filter, the comments.
- Modify `cloud/supabase/functions/_shared/judge_prompts.ts` (+ its test) — the clip marker, the due-date rule, the empty-body line, the versions.
- Modify `cloud/supabase/functions/judge-email/handler.ts` or `_shared/judge_pipeline.ts` (whichever builds the email item) — `scrub()` over the email's `subject` and `text` before the prompt (+ test).
- Modify `cloud/eval/run_eval.ts` (+ test), `.github/workflows/ci.yml`, `engine/tests/workflows.rs` — the secret rename and the allow-net host.
- Modify `site/privacy.html`, `app/src/account.rs` (`PRIVACY_VERSION`), `app/tests/static_assets.rs` if it pins the sentence, `HANDOFF.md`, `docs/specs/2026-09-09-knowlu-cloud-design.md` (an amendment section, the body untouched), `CLAUDE.md` (the model sentence if it names Anthropic).

---

### Task 1: The OpenRouter adapter and the provider seam

**Files:**
- Create: `cloud/supabase/functions/_shared/judge_openrouter.ts`, `cloud/supabase/functions/_shared/judge_openrouter_test.ts`, `cloud/supabase/functions/_shared/judge_provider.ts`, `cloud/supabase/functions/_shared/judge_provider_test.ts`
- Modify: `cloud/supabase/functions/_shared/judge_anthropic.ts` (types only), `judge_models.ts`, `judge_deps.ts`, `judge_pipeline.ts`, `cloud/eval/run_eval.ts`

**Interfaces:**
- Consumes: `JudgeModel`, `ModelRequest`, `ModelReply`, `ModelRefused` from `judge_anthropic.ts`; `ModelRow` from `judge_models.ts`.
- Produces: `export class OpenRouterModel implements JudgeModel` with `constructor(opts: { apiKey: string; baseURL?: string; timeoutMs?: number; fetchImpl?: typeof fetch })`; `export function openRouterBody(req: ModelRequest): Record<string, unknown>` (pure); `export function modelFor(row: { provider: string }, env: (name: string) => string | undefined, opts?: { timeoutMs?: number }): JudgeModel` in `judge_provider.ts`, which throws `new Error("no API key for provider '<p>' (set <SECRET NAME>)")` naming the secret, never a value; `ModelRequest.route` (the row's `route` jsonb, spread into the request's `provider` object by the OpenRouter adapter and ignored by the Anthropic one).

- [ ] **Step 1: Widen the types.** In `judge_anthropic.ts` replace
  `export type Sampling = { temperature: number } | Record<string, never>;` with
  `export type Sampling = Record<string, unknown>;` and add to `ModelRequest`:
  ```ts
  /** Provider-routing preferences from the pinned row (`models.route`); the OpenRouter adapter
   *  sends them as its `provider` object, the Anthropic adapter ignores them. */
  route: Record<string, unknown>;
  ```
  Fix every construction site the compiler names (`judge_pipeline.ts` passes `route: deps.row.route`; the tests' fakes pass `route: {}`).

- [ ] **Step 2: Write the failing adapter tests** in `judge_openrouter_test.ts` (Deno.test, `assertEquals`/`assertRejects` from `jsr:@std/assert` as the sibling tests import it):
  1. `openRouterBody builds an OpenAI-compatible request with the schema, the pin and reasoning off` — given `{ model: "qwen/qwen3.5-35b-a3b", system: "S", user: "U", schema: { type: "object" }, maxTokens: 640, sampling: { temperature: 0, reasoning: { enabled: false } }, route: { order: ["DeepInfra"], allow_fallbacks: false, zdr: true, require_parameters: true } }` the body equals
     ```json
     { "model": "qwen/qwen3.5-35b-a3b",
       "messages": [{"role":"system","content":"S"},{"role":"user","content":"U"}],
       "max_tokens": 640,
       "response_format": {"type":"json_schema","json_schema":{"name":"verdict","strict":true,"schema":{"type":"object"}}},
       "provider": {"order":["DeepInfra"],"allow_fallbacks":false,"zdr":true,"require_parameters":true},
       "temperature": 0,
       "reasoning": {"enabled": false} }
     ```
     (the sampling map is spread at the top level; `route` becomes `provider`).
  2. `a reply's content parses and its usage is mapped` — a fake fetch answering 200 with `{ choices: [{ finish_reason: "stop", message: { content: "{\"tier\":\"task\"}" } }], usage: { prompt_tokens: 810, completion_tokens: 80 } }` → `{ json: { tier: "task" }, inputTokens: 810, outputTokens: 80 }`; the test also asserts the fake saw `Authorization: Bearer <the key>`, `HTTP-Referer: https://knowlu.com`, `X-Title: Knowlu`, `Content-Type: application/json`, and the URL `https://openrouter.ai/api/v1/chat/completions`.
  3. `a length stop is the max_tokens error, not bad JSON` — `finish_reason: "length"` → rejects with a message containing `hit max_tokens (640)` and the model id.
  4. `a refusal is ModelRefused` — `message: { refusal: "no" , content: null }` → rejects with `ModelRefused`.
  5. `a non-2xx answer names the status and never the body` — 429 with a body `{"error":{"message":"SECRET-LOOKING-TEXT"}}` → rejects with a message containing `HTTP 429` and not containing `SECRET-LOOKING-TEXT`.
  6. `a reply that is not a JSON object is named without quoting it` — content `"[1,2]"` → rejects with `did not parse as JSON: not an object`; content `"nope"` → message contains `did not parse as JSON (4 chars)` and not `nope`.
  7. `the request times out at timeoutMs` — a fake fetch that never resolves until aborted; `timeoutMs: 10` → rejects (an `AbortError` or the adapter's own `timed out after 10 ms` message).

- [ ] **Step 3: Run them to see them fail** (`deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_openrouter_test.ts` from the repository root): module not found.

- [ ] **Step 4: Write `judge_openrouter.ts`:**
  ```ts
  // The OpenRouter adapter: one OpenAI-compatible POST per judgment, pinned to ONE named upstream
  // with zero retention, reasoning disabled, the reply constrained to the kind's JSON schema.
  //
  // Why a router and not the upstream directly: OpenRouter enforces the zero-retention filter and
  // the single-upstream pin per request (`provider.zdr`, `provider.order`, `allow_fallbacks: false`),
  // so a policy change at the upstream fails the request instead of silently routing elsewhere. The
  // upstream is named in the row's `route` and on the privacy page; OpenRouter's own policy is not to
  // retain prompts unless logging is opted into (never on this account).
  import { type JudgeModel, type ModelReply, type ModelRequest, ModelRefused } from "./judge_anthropic.ts";

  export const OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions";
  export const CALL_TIMEOUT_MS = 120_000;

  /** The request body, pure: the row's sampling spreads at the top level (temperature, reasoning),
   *  the row's route becomes OpenRouter's `provider` object, the schema is strict. */
  export function openRouterBody(req: ModelRequest): Record<string, unknown> {
    return {
      model: req.model,
      messages: [{ role: "system", content: req.system }, { role: "user", content: req.user }],
      max_tokens: req.maxTokens,
      response_format: { type: "json_schema", json_schema: { name: "verdict", strict: true, schema: req.schema } },
      provider: req.route,
      ...req.sampling,
    };
  }

  export class OpenRouterModel implements JudgeModel {
    #apiKey: string; #url: string; #timeoutMs: number; #fetch: typeof fetch;
    constructor(opts: { apiKey: string; baseURL?: string; timeoutMs?: number; fetchImpl?: typeof fetch }) {
      this.#apiKey = opts.apiKey;
      this.#url = opts.baseURL ? `${opts.baseURL.replace(/\/$/, "")}/chat/completions` : OPENROUTER_URL;
      this.#timeoutMs = opts.timeoutMs ?? CALL_TIMEOUT_MS;
      this.#fetch = opts.fetchImpl ?? fetch;
    }
    async complete(req: ModelRequest): Promise<ModelReply> {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.#timeoutMs);
      let response: Response;
      try {
        response = await this.#fetch(this.#url, {
          method: "POST",
          headers: {
            "Authorization": `Bearer ${this.#apiKey}`,
            "Content-Type": "application/json",
            "HTTP-Referer": "https://knowlu.com",
            "X-Title": "Knowlu",
          },
          body: JSON.stringify(openRouterBody(req)),
          signal: controller.signal,
        });
      } catch (e) {
        if (e instanceof DOMException && e.name === "AbortError") throw new Error(`the model call timed out after ${this.#timeoutMs} ms`);
        throw e;
      } finally { clearTimeout(timer); }
      if (!response.ok) {
        await response.body?.cancel();
        throw new Error(`the model service answered HTTP ${response.status}`);
      }
      const data = await response.json() as {
        choices?: { finish_reason?: string; message?: { content?: string | null; refusal?: string | null } }[];
        usage?: { prompt_tokens?: number; completion_tokens?: number };
      };
      const choice = data.choices?.[0];
      if (choice?.message?.refusal) throw new ModelRefused("the model declined");
      if (choice?.finish_reason === "length") {
        throw new Error(`the reply hit max_tokens (${req.maxTokens}) for model ${req.model}: raise the row's max_tokens`);
      }
      const text = choice?.message?.content ?? "";
      let json: unknown;
      try { json = JSON.parse(text); } catch {
        throw new Error(`the model's reply did not parse as JSON (${text.length} chars)`);
      }
      if (json === null || typeof json !== "object" || Array.isArray(json)) {
        throw new Error("the model's reply did not parse as JSON: not an object");
      }
      return {
        json: json as Record<string, unknown>,
        inputTokens: data.usage?.prompt_tokens ?? 0,
        outputTokens: data.usage?.completion_tokens ?? 0,
      };
    }
  }
  ```
  (Keep the comments; adjust the exact error texts only if the sibling Anthropic adapter's tests pin the same texts, in which case match them.)

- [ ] **Step 5: Run the adapter tests to green.**

- [ ] **Step 6: The provider seam, test first** (`judge_provider_test.ts`): `modelFor({provider:"openrouter"}, env)` returns an `OpenRouterModel` when `env("OPENROUTER_API_KEY")` answers and throws `no API key for provider 'openrouter' (set OPENROUTER_API_KEY)` when it does not; `modelFor({provider:"anthropic"}, env)` returns an `AnthropicModel` and names `ANTHROPIC_API_KEY`; an unknown provider throws `unknown model provider 'x'`; the env function is called only for the chosen provider's name (a spy env that throws on any other name).

- [ ] **Step 7: Write `judge_provider.ts`** accordingly; then change `judge_deps.ts`'s `liveDeps` to `model: modelFor(row, Deno.env.get, { timeoutMs: modelTimeoutMs })` after the row is read (the row read moves before the model build), and `cloud/eval/run_eval.ts` to build its live model through `modelFor(row, envGet)` per kind (the `--dry-run` path keeps `ScriptedModel`); the runner's "API key is required unless --dry-run" check names the secret from the row's provider. Keep the runner's zero-case short-circuit reading no key.

- [ ] **Step 8: Update `judge_models.ts`:** the row type gains `route: Record<string, unknown>` and `precision: string`, and the PostgREST select adds `route,precision`; its test (if any) pins the select string.

- [ ] **Step 9: The full Deno gate** (exact CI command from `.github/workflows/ci.yml`, plus `deno check` and `deno lint` over `cloud/supabase/` and `cloud/eval/`) and `cargo test --workspace` (the eval-gate workflow test still passes: nothing in CI changed yet). Report counts.

- [ ] **Step 10: Commit** — `cloud: an OpenRouter adapter behind the judge seam, the provider chosen by the pinned row (provider swap, Task 1)`.

### Task 2: The migration that re-pins, the ceiling, the export filter

**Files:**
- Create: `cloud/supabase/migrations/20260916000100_provider_swap.sql`
- Modify: `cloud/supabase/functions/_shared/judge_caps.ts`, `cloud/supabase/migrations/migrations_test.ts` (only if a pin moves), `cloud/supabase/functions/_shared/judge_rules_test.ts` or whichever test reads `export_training_rows`

**Interfaces:**
- Consumes: the `models` table (`kind` primary key; `provider`, `model_id`, `prompt_version`, `grammar_version`, `max_tokens`, `sampling jsonb`, `usd_per_m_in`, `usd_per_m_out`, `since`); `export_training_rows(p_since)` from `20260911000200_google.sql`.
- Produces: `models.route jsonb not null default '{}'`, `models.precision text not null default 'unspecified'`; the three rows re-pinned; `export_training_rows` excluding `origin in ('gmail_api','events')`.

- [ ] **Step 1: The failing test.** In the migration test that pins the models seed (find it: grep `claude-haiku-4-5` under `cloud/supabase/`), add a case that reads every `*provider_swap*` migration and asserts the LAST definition of the three rows: `provider = 'openrouter'`, the two model ids, `precision` values `bf16 (CoreWeave)` / `bf16 (CoreWeave)` / `fp8 (DeepInfra)`, prices `0.10/0.15`, `0.10/0.15`, `0.14/1.00`, prompt versions `task-2`/`event-2`/`email-2`, `max_tokens` 256/256/640, sampling `{"temperature": 0, "reasoning": {"enabled": false}}`, route `{"order": ["CoreWeave"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}` (and `DeepInfra` for email). And a case that the last definition of `export_training_rows` contains `origin not in ('gmail_api', 'events')`.

- [ ] **Step 2: Write the migration:**
  ```sql
  -- Knowlu — the inference provider swap (Quinn's ruling of 2026-09-16 on the cloud design's R8:
  -- option 1 of docs/notes/2026-09-16-inference-provider-and-model-scoping.md).
  --
  -- Forward-only: 20260911000100…000900 are applied and never edited. Two new columns on `models`
  -- and the three rows re-pinned; the training export narrowed. No function here is definer or
  -- writing, so nothing to revoke; the guard's counts do not move.
  --
  -- `route`: the provider-routing object the adapter sends verbatim (OpenRouter's `provider`
  -- preferences: one named upstream, no fallback, zero-retention endpoints only, and every
  -- requested parameter honoured or the request refused). `precision`: what the pinned endpoint
  -- declares, recorded beside the model id because a quantized build under a full-precision name
  -- is the failure the scoping study warned about.
  alter table models add column if not exists route jsonb not null default '{}'::jsonb;
  alter table models add column if not exists precision text not null default 'unspecified';

  -- The pins. Granite 4.2 8B for the two classification kinds (instruction following at 8B, an
  -- explicit non-thinking mode, German supported); Qwen3.5-35B-A3B for email (the top open-weights
  -- value accuracy on the Structured Output Benchmark from 3B active parameters). Prices are the
  -- endpoints' list prices read 2026-09-16; `usage_daily` and `monthly_spend` price every call at
  -- the row's numbers.
  update models set
    provider = 'openrouter', model_id = 'ibm-granite/granite-4.2-8b', precision = 'bf16 (CoreWeave)',
    prompt_version = 'task-2', usd_per_m_in = 0.10, usd_per_m_out = 0.15,
    sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
    route = '{"order": ["CoreWeave"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
    since = current_date
  where kind = 'task';
  update models set
    provider = 'openrouter', model_id = 'ibm-granite/granite-4.2-8b', precision = 'bf16 (CoreWeave)',
    prompt_version = 'event-2', usd_per_m_in = 0.10, usd_per_m_out = 0.15,
    sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
    route = '{"order": ["CoreWeave"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
    since = current_date
  where kind = 'event';
  update models set
    provider = 'openrouter', model_id = 'qwen/qwen3.5-35b-a3b', precision = 'fp8 (DeepInfra)',
    prompt_version = 'email-2', usd_per_m_in = 0.14, usd_per_m_out = 1.00,
    sampling = '{"temperature": 0, "reasoning": {"enabled": false}}'::jsonb,
    route = '{"order": ["DeepInfra"], "allow_fallbacks": false, "zdr": true, "require_parameters": true}'::jsonb,
    since = current_date
  where kind = 'email';

  -- The training export excluded Gmail rows only. Google's Limited Use policy binds data from a
  -- sensitive scope (the Calendar API) exactly as it binds a restricted one, and `origin` cannot
  -- tell a Google-Calendar event from an ICS one, so every `events` row is excluded too until an
  -- origin split exists (a C4 refinement).
  create or replace function export_training_rows(p_since timestamptz)
  returns setof judgments
  language sql
  stable
  security invoker
  set search_path = public, extensions
  as $$
    select * from judgments where origin not in ('gmail_api', 'events') and judged_at >= p_since;
  $$;
  ```
  Match the original function's exact signature and header (read it in `20260911000200_google.sql` first; if it is `security invoker` and non-writing, the guard demands nothing; if the guard's per-file parse count for the new file needs a pin, add it with the reason).

- [ ] **Step 3: `judge_caps.ts`:** `MONTHLY_CEILING_USD = 2.0` with the comment restated (the heavy persona on the new pins is about $0.87; the ceiling is a runaway guard, not a budget). Update its test if it pins the number.

- [ ] **Step 4: Gates green; `db push --linked --dry-run` lists exactly `20260916000100_provider_swap.sql`; push it** (the CLI's keyring supplies the password; BLOCKED on any prompt).

- [ ] **Step 5: Commit** — `cloud: the three kinds re-pinned to Granite 4.2 8B and Qwen3.5-35B-A3B on zero-retention endpoints, the ceiling at $2, the training export excludes every Google-derived row (provider swap, Task 2)`.

### Task 3: The prompts and the email scrub

**Files:**
- Modify: `cloud/supabase/functions/_shared/judge_prompts.ts`, `judge_prompts_test.ts`, the email handler or pipeline where the email item is built, `_shared/scrub.ts` only if a narrower export is needed

**Interfaces:**
- Consumes: `clip`, `buildPrompt`, `systemTemplate` in `judge_prompts.ts`; `scrub(text)` in `_shared/scrub.ts`.
- Produces: prompts whose `prompt_version` strings are `task-2`, `event-2`, `email-2` (wherever the version is emitted — find `prompt_version` in the pipeline/log); the email item's `subject` and `text` scrubbed before the prompt.

- [ ] **Step 1: Failing prompt tests:** (a) clipping appends ` …[truncated]` when text exceeds `MAX_BODY_CHARS` and appends nothing otherwise (all three kinds); (b) the email rules text contains the sentence `Resolve a relative deadline` and `against the Date line above`; (c) an email item with `text: ""` yields a user message containing `Message: (no plain-text body)` and the rules contain `with no message body`; (d) the three version strings.
- [ ] **Step 2: Change `judge_prompts.ts`:** `clip` returns the clipped text plus ` …[truncated]` when it cut; the email `due` rule becomes `- due: the deadline as YYYY-MM-DD, or YYYY-MM-DDTHH:MM when a time is given. Resolve a relative deadline ("Friday", "next week", "end of the month") against the Date line above, in that line's own timezone. If you cannot resolve it to one calendar day, answer null.`; the email builder pushes `Message: (no plain-text body)` when the text is empty and the rules gain `- with no message body, judge from the subject and sender alone and answer a confidence at or below 0.5.`; bump the version constants.
- [ ] **Step 3: Failing scrub test:** an email item whose subject and text carry a URL with a token-looking query string and a long bearer-shaped string reaches `buildPrompt` with both replaced by `scrub()`'s placeholders (assert the placeholder appears and the original does not, in the built user message).
- [ ] **Step 4: Apply `scrub()`** to the email item's `subject` and `text` at the one place the email item is built for the prompt (state the file and line in the report). Task bodies are not scrubbed (a ruling: LMS text is the student's own; revisit if a body ever carries a credential).
- [ ] **Step 5: Gates green. Commit** — `cloud: the prompts say when they clipped, the email deadline resolves against its own Date line, an empty body is named, the email text is scrubbed before it travels (provider swap, Task 3)`.

### Task 4: CI, the eval gate, the docs, the privacy page

**Files:**
- Modify: `.github/workflows/ci.yml`, `engine/tests/workflows.rs`, `HANDOFF.md`, `docs/specs/2026-09-09-knowlu-cloud-design.md`, `CLAUDE.md` (if it names Anthropic), `site/privacy.html`, `app/src/account.rs`, `app/tests/static_assets.rs` (if it pins the page's sentence)

- [ ] **Step 1: Failing Rust test:** `engine/tests/workflows.rs`'s eval-gate test asserts `secrets.OPENROUTER_API_KEY` and `secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY`, and that `secrets.ANTHROPIC_API_KEY` no longer appears in the job; the "appended job" regression fixture follows.
- [ ] **Step 2: `ci.yml`:** the eval-gate step's env and `--allow-env` list read `OPENROUTER_API_KEY` instead of `ANTHROPIC_API_KEY` (keep the four SDK names only if the Anthropic adapter is still compiled into the runner; it is, so keep them); `--allow-net` adds `openrouter.ai`; the job comment restated (the two secrets by name). The `cloud` job's test `--allow-env`/`--allow-net` lists do not change unless `judge_openrouter_test.ts` needs `127.0.0.1` (it uses an injected fetch, so no).
- [ ] **Step 3: The spec amendment.** Append to `docs/specs/2026-09-09-knowlu-cloud-design.md` a dated section `## Amendment 2026-09-16 — the inference provider (ruling R8 amended by Quinn)` stating: the launch provider is OpenRouter pinned to one named zero-retention upstream per kind (CoreWeave for task and event, DeepInfra for email), models Granite 4.2 8B and Qwen3.5-35B-A3B, prices as pinned; that §5.2's "zero-retention" claim was not true of the Anthropic self-serve tier it named (30-day retention) and is true of the pinned endpoints; the reference to the scoping note. The body of the spec is not rewritten; a one-line pointer goes beside the R8 row and beside the §11 provider row in the plan's existing override-note style.
- [ ] **Step 4: The privacy page.** Replace the Anthropic bullet with the legal report's single-provider draft adapted to a router plus two named upstreams (`docs/reports/2026-09-16-inference-scoping-legal-floor.md` §5.1): OpenRouter as the company that carries the request with zero retention and no training, CoreWeave and DeepInfra as the two places a model reads it, each on zero-retention endpoints; the "one place" bullet becomes "two places, named above"; the count sentence ("Four companies, each doing one job") becomes the right number; the "No note bodies" bullet takes the report's §5.6 replacement. `PRIVACY_VERSION` in `app/src/account.rs` becomes `"2026-09-16"` and `TOS_VERSION` stays. Any static test that pins the page's sentences follows. Record in HANDOFF that the page's own notice-and-ask clause is satisfied for the one existing account by Quinn's ruling and that a re-consent screen for a `privacy_version` change is a C4 item before other accounts exist.
- [ ] **Step 5: HANDOFF and CLAUDE.md.** HANDOFF's C2 block "The pinned model" paragraph restated for the new pins and prices; Quinn's queue: P1 becomes `OPENROUTER_API_KEY` on staging (set), the GitHub secret `OPENROUTER_API_KEY`; the C4 list gains the origin split for Calendar-API rows and the re-consent screen; CLAUDE.md's Direction line ("Anthropic API") gets the new provider in one clause.
- [ ] **Step 6: Gates green (Deno, Rust). Commit** — `ci+docs+site: the eval gate reads OPENROUTER_API_KEY, the spec's R8 amended on Quinn's ruling, the privacy page names the router and the two zero-retention hosts, version 2026-09-16 (provider swap, Task 4)`.

### Task 5: Deploy and the live check (the controller, with Quinn's key on staging)

- [ ] **Step 1:** redeploy every function whose bundle changed (`judge-task`, `judge-event`, `judge-email`, `judge-rules`, `gmail-read`, and any other importer of `judge_deps.ts`/`judge_prompts.ts` — grep), `--use-api`.
- [ ] **Step 2:** the ten-request JSON check, through the deployed functions with a staging session (never with the key in hand): ten `judge-task` calls with fixture-shaped items → every reply a valid verdict, `judgments.model_id = 'ibm-granite/granite-4.2-8b'`, `usage_daily` charged at the row's price; three `judge-email` calls with fixture-shaped emails (no real mail) → valid verdicts from `qwen/qwen3.5-35b-a3b` with `reasoning.enabled: false` honoured (no truncation, no reasoning text in the content). The same ten fixture-shaped emails also go through the fallback pin (`deepseek/deepseek-v4-flash-0731` at DeepInfra, fp8, zero retention, $0.06 / $0.18) by a temporary one-row change, and the two sets of verdicts are compared field by field (Quinn's question of 2026-09-16: DeepSeek is about a third of Qwen's price for the email kind but has no structured-output measurement); a tie goes to the cheaper model, a difference in `tier`, `due` or `course` correctness goes to the better one, and the winning row is the one that stays. If Qwen answers invalid JSON or truncates, the fallback is the pin and the check repeats.
- [ ] **Step 3:** Quinn's own vault after P3: the old pin and the new one judged side by side on the same items (the eval runner's `--dry-run` shape with real calls to both providers is out of scope; a manual comparison of ten items each is the gate). Record in HANDOFF.

## Self-review

Spec coverage: option 1 of the note (per-kind pins, one zero-retention host) → Tasks 1–2; the four provider-independent fixes (`due`, scrub, export filter, spec sentence) → Tasks 2–4; the seam's two blocking gaps (thinking off, the JSON check) → Task 1's `sampling.reasoning` and Task 5's check; the privacy page's notice clause → Task 4. Placeholders: none. Types: `route` is `Record<string, unknown>` on `ModelRequest` and on the row; `precision` is `string`; `modelFor` takes `(row, env, opts)` everywhere it is named.
