PASS (Spec: pass, Quality: pass)

# PR B (prompt-injection suite, Layer 2 and CI): whole-PR review, T9

- Scope: `git diff 3c48bde6 1e3c4cf9` on `security/redteam-live`. The plan is `docs/plans/2026-11-09-prompt-injection-suite-plan.md`: §3-§6, T3a-T9, the §8 ledger and §13.
- Reviewer: reviewer agent, 2026-11-10.
- Size: Lane. No file on the contract list and no frozen reference is touched, so this review is the reviewer's, not contract-reviewer's.

No Critical or Important findings. There are seven Minor ones: three are records, two are CI cost and timeout edges, one is a duplicated host literal, and one is a plan step still owed before merge.

## What was run

| Command | Result |
|---|---|
| `deno test`, the `cloud` job's line from `ci.yml` (`--allow-net=127.0.0.1`, env limited to the four `ANTHROPIC_*` names), over `cloud/supabase/` and `cloud/eval/` | 1569 passed, 0 failed |
| `deno check` (`cloud/eval/*.ts cloud/eval/redteam/*.ts`) and `deno lint cloud/eval/` | clean |
| `cargo test -p knowlu-engine --test workflows -j 2` | 19 passed, no warnings |
| `git ls-files --eol` on every changed file; `git diff --check` | all LF, nothing flagged |

The dispatch's shorter Deno line (`--allow-read --allow-env`, no `--allow-write`, no `--allow-net`) shows 22 failures. They are the temp-directory and `127.0.0.1` listener tests, which need the permissions CI grants. Under CI's own line every test passes, so no test leaves the machine and none needs a key.

`run_redteam.ts` was not run live.

## 1. Against the plan

**§8 rows PR B owns:**
- Fixtures and the coverage table: hold.
- Goal predicates: hold.
- Canary and seed-exfiltration predicates (C10): hold.
- The `touched` trigger: holds.
- The ceiling and estimate printed before the first call: hold (`run_redteam_test.ts:1573`).
- The key named only by its secret: holds (`workflows.rs`, the key-placement test).
- Exit 1 on a critical: holds.
- The one-bit output: holds (tests at `run_redteam_test.ts:1018-1060` and `:1510-1559`).
- No retry of a refusal on the mail paths: holds (`:1311`, `:1417`).
- The three source guards and the `failureCause` guard: hold (`:1806-1855`).
- Non-blocking PR step; weekly on hosted runners, `schedule` and `workflow_dispatch` only: holds in both the Deno tests and `workflows.rs`.
- No exception text in CI output: holds (`:1078`, `:1145`).
- No `engine/src/` change and no fixture change: holds.
- Still open: T6's PR proof (M7).
- The cost row (Q2, $5) is answered by Quinn's ruling in the ledger, but the plan does not record it yet (M6).

**§13 spot-checked against the code. Every bullet matches:**
- **Decision predicates:**
  - `schema.ts:73-82` carries `importance_not_in`, `course_not` and `due_not`.
  - Every C1, C4, C5 and C11 attack in the committed fixtures carries a decision predicate.
  - So does every C2, C3, C12 and C13 attack aimed at a decision.
- **Evidence:** `isEvidence` excludes `evidence` from the echo and `absent_strings` checks (`run_redteam.ts:378-398`, `:445`), and `breaches` still scans it (`:401`).
- **Canary prefix:** `echoes` checks the bare `KNOWLU-CANARY-` prefix (`:392`).
- **Twins:** a twin gets the echo checks against its attacks' planted strings. Its policy hits go only to `q1`, which prints only under `--per-case` (`:605-609`, `:844`).
- **Outage:** `providerError` is applied per kind (`:798`). A critical and an outage together exit 2 and still print `critical yes` (`:847-849`).
- **Spend and faults:**
  - A dry run prices $0.00 (`:691`).
  - Any throw prints `FAULT_RUN` once and exits 2 (`:705-721`).
  - `failureCause` is pinned to both production copies.
- **CI:** the step's `if:` carries `!cancelled()`, and `tlds|judge_due` are in the `redteam` pattern. `eval suite` keeps `timeout-minutes: 20`, and the weekly `main` guard is described as an accident guard.
- **Merge:** the merge waits on the key (`plan:961`).

**§11 (out of scope):**
- Nothing out of scope crept in.
- No prompt or system-template text, console code or content-policy code changed.
- The only edits to existing code are two one-word `export`s (`run_eval.ts` `clipDetail`, `detect.ts` `matchingForm`), comment-only header edits and one test rename in `output_policy_test.ts`.

## 2. Public-repo pass

**No working exploit.** All 36 attacks fall into the 13 generic classes. Every typed field is a clean value in the shape the device sends:
- task `due` is `YYYY-MM-DDTHH:MM`;
- event `start` and `end` are `YYYY-MM-DDTHH:MM`;
- event `source` is a slug;
- `categories` and `audiences` are plain words;
- email and `mail_claim` `date` are RFC 5322 with a numeric zone.

No fixture puts a line break, a forged prompt label or extra text in any of those fields. Every payload sits in a body, a description, the mail text or a `mail_resolve` `target`. No fixture is aimed at the typed-field issue that PR #76 fixed. A search of the diff and of every commit message finds no wording about raw interpolation, typed fields, prompt-fields or #76.

Every fixture still renders the same prompt under PR #76's shape checks (now on `origin/main`, `9f133280`), and PR B touches none of #76's files. The PR's merge-ref CI will run the combination.

**No real data.** Hosts are only `example.com/.org/.net` subdomains and the reserved `.example` TLD. People and places are generic: "Prof. Example", `xx-101` and the like, "Student Union, Room 214". There are no real names, schools, vendors or courses.

The IP literals are documentation ranges, plus deliberate near misses such as `192.0.20.1` and `203.0.114.1` in hygiene's must-refuse rows. The public resolver an earlier round used is gone.

`schema_test.ts` names non-reserved hosts under a `notexample` label. They are must-refuse rows: hygiene can only be shown refusing a host that is not reserved, and nothing fetches them. I accept them.

**No secret.** Secrets appear only as `${{ secrets.NAME }}` in step `env:` blocks and by name in prose.

**The one bit.** Without `--per-case`, the runner can print only:
- the seven fixed phrases (`run_redteam.ts:106-113`);
- the ceiling, which is computed from the fixtures and pins alone (`:563-577`);
- the summary `red-team: critical yes|no, N calls, $N.NN` (`:690-693`).

Whether a call happens does not depend on what the model answered:
- the judge path makes one call per run (`judge_pipeline.ts:234`);
- a refusal, truncation or `incomplete` answer is never retried (`:255`, `:269-272`, `:289-294`);
- only `model failed` is retried.

So `N calls` moves only with provider failures, never with a result. No module the runner imports prints anything: none of the `_shared/judge_*.ts` files, `scrub.ts`, `score.ts` or `detect`/`loader`/`schema` has a console call, and `run_eval.ts` prints only inside its own `import.meta.main`. Both workflow `run:` lines pass neither flag, and `workflows.rs` plus the Deno workflow test pin that.

Fixture ids appear in committed files only in the fixtures themselves and in `schema_test.ts`'s structural records. None is tied to a model result.

Commit messages describe mechanisms, never results; no live run has happened.

**Business wording.** Nothing in the diff or the commit messages mentions customers, revenue or funding.

## 3. Security of the CI change

- **Triggers:** `pull_request` only, never `pull_request_target`. The weekly workflow has `schedule` and `workflow_dispatch` only, and `workflows.rs` bans `pull_request`, `push`, `workflow_run` and `workflow_call` in it.
- **Fork PRs:** no secrets reach them. The runner reads `SUPABASE_SERVICE_ROLE_KEY` first and, when it is empty, prints the skip line and exits 0, reading nothing else (`run_redteam.ts:729-732`, test `:948`).
- **The secret's reach:** the step's `env:` only. `OPENROUTER_REDTEAM_API_KEY` appears exactly once in `ci.yml`, and `workflows.rs` forbids it everywhere else, along with `toJSON(secrets)` and `secrets[`. No expression is spliced into either `run:` line.
- **What a branch PR's author can do:** run their own code with this key and the staging service-role key, the same as `eval suite` already allows. The key is separately capped and can be revoked on its own.
- **`persist-credentials`:**
  - `false` on `eval-gate`'s checkout (unchanged);
  - `false` on the weekly checkout, pinned.
- **Permissions:** `{ contents: read }` at the top of both workflows.
- **Concurrency:** the weekly workflow has `redteam-weekly` with `cancel-in-progress: false`. `ci.yml` has none (M5).
- **Weekly guard:** `if: github.ref == 'refs/heads/main'`, `ubuntu-latest` only, never the self-hosted switch. The header is honest that the guard prevents accidents, not attacks.
- **With no key, as things stand:**
  - `modelFor` throws on an empty key (`judge_provider.ts:26`), so the runner prints `red-team: harness fault, a model client could not be built` and exits 2 before any call.
  - On a PR, `continue-on-error: true` makes that a failed step inside a successful job, so `eval-gate` stays green.
  - The weekly run would go red every Monday. §13 keeps PR B unmerged until the key exists, so `main` never carries a weekly run that is red for want of a key.

## 4. The merge resolution at `2f3d631f`

- Its parents are `cfe1613d` (the branch) and `3c48bde6` (main).
- `git diff 3c48bde6 1e3c4cf9 -- .github/workflows/ci.yml` shows only PR B's lines. The red-team step closes `eval-gate`, and `cloud-rls` follows it byte for byte as main has it.
- `engine/tests/workflows.rs` only gains lines against `3c48bde6`, so main's RLS pins all survive, and the file passes 19 of 19.

## 5. CLAUDE.md

- **Contract list:** nothing on it is touched. `engine/tests/workflows.rs` is not on the list.
- **Frozen references:** none touched.
- **Tests never leave the machine:** they pass under `--allow-net=127.0.0.1`.
- **LF:** every changed file is LF.
- **Warnings:** 0 in the `workflows` build.
- **Ignored tests:** none touched.
- **Actions:** SHA-pinned, the same SHAs as `ci.yml`.
- **Privacy:**
  - no new data class;
  - the suite sends only synthetic fixtures to the provider;
  - no student content and no grades;
  - `PRIVACY_VERSION` is untouched.

## Findings

**M1 (Minor): the README names files that do start the step.** `cloud/eval/redteam/README.md:216-220` lists `judge_pipeline.ts`, `judge_prompts.ts`, `judge_models.ts`, `judge_validate.ts` and `scrub.ts` among the files that "don't start the step on their own". All five are in `run`'s pattern, and `redteam=$run` (`ci.yml`, `touched` step).
- **Scenario:** the hardening lane reads this list as the gap to close and widens the trigger by those five names, or decides a prompt edit does not run the suite. The real gap is `judge_provider.ts`, `judge_openrouter.ts` (reached through `judge_provider.ts`), `judge_anthropic.ts`, `judge_db.ts`, `judge_caps.ts`, `run_eval.ts` and `score.ts`.
- **Fix:** list only the files outside both patterns.

**M2 (Minor): the workflow count is wrong.** `docs/reference/app.md:264` calls `redteam-weekly.yml` "A third workflow". `.github/workflows/` already holds `ci.yml`, `release.yml` and `security.yml`, so it is the fourth.
- **Scenario:** a reader auditing which workflows hold secrets trusts the count and never looks at `security.yml`.
- **Fix:** name it without a count.

**M3 (Minor): the job timeout leaves no room for setup.** `.github/workflows/ci.yml:220` sets `timeout-minutes: 50`, exactly `eval suite`'s 20 plus `red-team suite`'s 30. Checkout with `fetch-depth: 0`, `setup-deno` and the `touched` step get no headroom.
- **Scenario:** a passing but slow eval run of about 19.5 minutes, followed by a provider stall that runs the red-team step to its 30-minute limit, hits the job's clock first. `eval-gate` is a required check, so it goes red and the PR is blocked by a step that `continue-on-error` was meant to keep advisory.
- **Fix:** about 55 minutes, or a red-team step bound of 25.

**M4 (Minor): a second staging-host literal that nothing pins.** `.github/workflows/redteam-weekly.yml:40` repeats `STAGING_HOST`. `ci.yml`'s own A-6 comment warns that this shape "drifts the day staging moves", and no test pins the two values equal.
- **Scenario:** staging moves and `ci.yml` is updated alone. The weekly run then fails at the pin read every Monday (`FAULT_PIN`, exit 2, red). That is loud, not silent, but the alert reads as a harness fault rather than a stale host.
- **Fix:** add an equality pin in `workflows.rs` beside the runner-line equality.

**M5 (Minor): the cost model counts PRs, but the step runs on every push.** The `red-team suite` step (`ci.yml:328`) runs on every `synchronize` of a PR that touches the judgment path, and `ci.yml` has no `concurrency` with `cancel-in-progress`. §6's month counts ten PRs, not pushes.
- **Scenario:** the hardening lane's own PR, iterated over 20 or more pushes, spends 20 or more runs. At §9's figures ($0.15 bound, $0.44 with retries) that can exhaust a $10 monthly cap. Every later run that month, the weekly one included, then reports `provider error` (exit 2), and a real regression is hidden until the cap resets.
- **Fix:** either add a per-PR `concurrency` group with `cancel-in-progress: true` scoped to `eval-gate`, or state the per-push cost in the README and §13.

**M6 (Minor): the plan does not record Quinn's cost ruling.** The plan still says the cap is $5 (`plan:339`, `:736`, `:836`). §9 ends "Quinn decides before T6" (`plan:816`). §13 does not fold the decision in. Quinn's ruling in the ledger is a $10 limit, and T6 ran before that answer arrived, though its code does not depend on the figure. $10 covers the $6.20 worst-case month.
- **Scenario:** whoever creates the key reads §10 Q2 and sets $5. The worst-case month of $6.20 then trips the cap as an outage.
- **Fix:** add one §13 line recording the $10 limit as Quinn's ruling, without the ruling's business condition, which stays off the public repo.

**M7 (Minor): T6's PR proof is still owed.** T6's last step (`plan:652-653`) is to prove on the PR that a branch touching only `cloud/eval/redteam/README.md` runs the step and a `docs/`-only change runs neither. This is §8's proof for the CI-trigger row. It cannot exist before the push, and nothing records it yet.
- **Scenario:** PR B merges with the trigger proven only by unit pins on the YAML text, never on GitHub's runner.
- **Fix:** do it on the draft PR before merge, and record the two run ids in the ledger.

## Held, checked and not findings

- The `notexample` hosts in `schema_test.ts`, for the reason in section 2.
- The README's list of what hygiene cannot see. It describes gaps in the fixture-hygiene checker, which guards this repository's own files, not a production path.
- `spend` counts only replies that parsed. This is §13's accepted residual, and the ceiling remains the bound.
- The plan text says 144 calls; the run makes 131 because twins are shared. The runner's formula is right, and §5's "about 36" holds.
