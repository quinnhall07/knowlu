# E2 review-triage replay — prep report (2026-09-22)

Prepares experiment E2 (`docs/notes/2026-09-22-jev-review-replay-experiment.md`, brief at
`.superpowers/sdd/2026-09-22-judgment-quality-plan/e2-brief.md`): can Jev sort our own graded
C1b code-review findings by severity as well as the graders did, and does it beat a cheap offline
baseline? **This report covers the prep — extraction, the leak check, both baselines, and the
request-building — not the paid run**, because no `OPENROUTER_API_KEY` exists on this machine. The
paid run is one command (§6) once a key is available through any of the three places
`scripts/experiments/e2-review-replay/credentials.ts` tries.

Scripts: `scripts/experiments/e2-review-replay/` (18 files, tests alongside each pure module).
Corpus workspace (git-ignored, not in this repo): `.superpowers/sdd/2026-09-22-judgment-quality-plan/e2/`.
**Paths (final-review fix, 2026-09-22):** no script names a machine path. The C1b task/final
reviews folder and the workspace are given as `--corpus-root <dir>` / `--workspace <dir>` or the
environment variables `E2_CORPUS_ROOT` / `E2_WORKSPACE`, with a clear error when either is absent;
the plan review is read from this repository (`docs/reports/2026-09-17-c1b-sign-in-plan-review.md`).
The corpus-reading tests ignore themselves when `E2_CORPUS_ROOT` is unset.

## 1. Corpus: found vs expected

The brief expects 59 findings (24 plan-review + 35 task/final-review). **Extraction found 60**, and
the discrepancy is real, not a bug: `task-3-review.md` has 6 findings, not the note's table's 5 —
finding 4 (`app/src/account.rs:395-398`, a `(nit)`) is on the page and is not an extraction
artefact. `scripts/experiments/e2-review-replay/parsers_test.ts` pins the corrected per-file counts
against the corpus on disk (10/2/6/1/1/2/2 for tasks 1-7, 12 for the final review, 24 for the plan
review), so a future re-run that disagrees fails loudly rather than silently re-drifting.

Of the 60, the secret/token/session/credential drop filter removes **11** (not the 3-8 a first
manual read of the source text suggested — see §3; fix round 1, §8, added one of the 11 after this
report's first version). **49 findings ship in the corpus**, 19 from Set A (plan review) and 30
from Set B (task + final reviews).

## 2. Extraction bugs found and fixed (RED/GREEN evidence)

TDD was used throughout for the pure functions (`severity.ts`, `secretfilter.ts`, `parsers.ts`,
`baselines.ts`, `jev_request.ts`, `credentials.ts`, `dispositions.ts`); three real bugs surfaced
this way and were fixed before green, not designed around:

1. **`normalizeSeverity`** didn't strip the `"Severity: "` prefix the final review's trailing marker
   uses — RED on `normalizeSeverity("Severity: should-fix.")`, fixed by stripping the prefix before
   the lookup table match.
2. **Item-boundary detection** assumed the finding number always precedes the opening `**`
   (`"1. **should-fix — ...**"`), which is true in six of the seven task reviews but not
   `task-3-review.md`, which bolds the number too (`"**1. \`path\` — ... (should-fix)**"`) — RED on
   the task-3 fixture (0 items found), fixed by allowing an optional leading `**` before the digit.
3. **Section-boundary detection** hardcoded the two spellings of the "verified clean" heading it had
   seen (`"## What I verified clean"`, `"## Verified clean"`) — `task-4-review.md` spells it
   `"## What was verified clean"`, a third variant, and the hardcoded list silently swallowed the
   whole rest of the file into finding 1's text, which then falsely matched the secret filter on
   words from the *Verified clean* section's own "no secret, client id, ..." disclosure line, not
   from the finding itself. RED was `extractCorpus()` reporting 11 dropped findings instead of the
   hand-verified 10, with `B-task4-1` carrying terms (`client id`, `credential`, `secret`, `token`)
   that are nowhere in finding 1's actual text. Fixed by replacing the hardcoded list with
   "read to the next `## ` heading, whatever it says" (`sectionUntilNextH2`), which cannot drift the
   same way again. `extract_test.ts` pins the corrected drop list.

## 3. The secret/token/session/credential drop filter

Mechanical, over every kept finding's leak-stripped text, never a per-item judgment call
(`secretfilter.ts`): a whole-word (underscore/hyphen-normalised) match on session, jwt, anon key,
credential manager, credential, secret, token, client id, api key drops the finding outright.

**11 dropped**, full audit at `.superpowers/sdd/2026-09-22-judgment-quality-plan/e2/dropped.json`:
`B-task1-2` (bearer token / `verify_jwt`), `B-task3-3` (Credential Manager / session), `B-final-F2`
("the port and the tokens" — plural, see §8), `B-final-F7` (anon key / `verify_jwt`), `B-final-F8`
(credential / `token_verifications`), `B-final-F11` (a test name containing
"...becomes_a_session_on_this_machine"), `A-C2` (credential struct / stored session), `A-C5` (the
literal TOML key `secret = "env(GOOGLE_SECRET)"`), `A-C6`, `A-I2`, `A-M4`.

Two read as false positives on a first look and are not, on the mechanical policy this filter
states: **`A-C6`** matches on Stripe's own proper noun "Checkout Session" — still, literally, a
mention of a session, and the policy is written to be over-inclusive rather than to adjudicate
whether a given "session" is ours or a vendor's. **`A-M4`** matches on the literal environment
variable name `KNOWLU_ANON_KEY`, quoted as an example value in the finding's own text. Both are kept
dropped rather than special-cased, per the brief: "the cheapest way to honour it is to drop those
findings."

## 4. The label-leak check (stop rule 1)

**Passes**, by direct proof: `findLeaks()` (`severity.ts`) is a word-boundary scan for
critical/important/minor/blocking/should-fix/nit/severity, and returns empty on every one of the 49
kept findings' text — proven both as a unit test over hand-written fixtures for all four placements
(`severity_test.ts`, `parsers_test.ts`) and as an integration test and a runtime assertion over the
real, extracted corpus (`extract_test.ts`, `extract.ts`, `run.ts`).

A softer, corroborating check (procedure §Step 3's "run once with labels left in"):
`run.ts` also scores the lexical baseline (§5) on the *un-stripped* span. The gap is real but modest
— whole corpus: stripped 42.9% vs raw 46.9%; Set B only (30 items, the only ones whose raw span
literally contains the label word — Set A's severity is positional, a heading above the item, never
inside it) — stripped 50.0% vs raw 53.3%. The modest size is explained, not concerning: a term that
appears in roughly a third to a half of all documents (as `nit`/`should-fix` do) gets a low
IDF weight, so it adds only a little to a cosine score already carrying many other shared words. The
direction is consistently correct (raw ≥ stripped in both comparisons); the load-bearing proof is
the zero-survivors string check, not this magnitude.

## 5. Baselines (leave-one-out over the 49-item corpus)

Majority class and an honest **lexical** (TF-IDF, cosine, 3-nearest-neighbour, leave-one-out — never
called "embeddings") baseline, per the decision context: no embedding model can be downloaded on
this machine, so this stands in for the commercial comparator §4a of the procedure names.

| label | majority class | lexical (TF-IDF 3-NN) |
|---|---|---|
| severity (critical/important/minor) | 53.1% (26/49), 95% CI [39.4%, 66.3%] | 42.9% (21/49), 95% CI [30.0%, 56.7%] |
| disposition (fixed/ruled against/handed off/deferred) | 81.6% (40/49), 95% CI [68.6%, 90.0%] | 85.7% (42/49), 95% CI [73.3%, 92.9%] |

**Stop rule 3 status: not yet evaluated** — it is a comparison against the *model's* accuracy, and
no model has run. What these numbers say on their own: for severity, the lexical baseline **loses**
to the majority class by 10 points — the cheap baseline does not win here, contrary to §4a's
Greptile precedent, and severity is a genuinely hard 3-way call for a bag-of-words method on 49
heterogeneous items. For disposition, majority class is already high (81.6%) because the corpus's
own disposition distribution is skewed — Set A is 100% "fixed" (§7), which the plan review's own
thoroughness explains, not an extraction defect — so the lexical method's 4-point edge over it is a
small win against a strong, unbalanced baseline. Full confusion-adjacent numbers (not a matrix at
n=49 with 3-4 classes; see raw JSON) are at
`.superpowers/sdd/2026-09-22-judgment-quality-plan/e2/baselines.json`.

## 6. Cost, transport, and the one command for the paid run

**Transport is the one open question before trusting the paid run's numbers.** Jev's native
interface (`docs/reports/2026-09-22-jev-system-one-assessment.md`) is `POST
https://api.typesafe.ai/v1/systemone` with body `{state, model, questions}` — no system/user split,
no JSON schema, unlike the product's own `judge_openrouter.ts` adapter. Going through OpenRouter
(this brief's decision, not TypeSafe direct) means OpenRouter's normal `/chat/completions` contract
applies; `jev_request.ts`'s `buildOpenRouterChatBody` carries the native `state`/`questions` intent
as JSON inside one user message, pinned zero-retention the same way `judge_openrouter.ts` pins every
other model (`order: ["typesafe"], allow_fallbacks: false, zdr: true, require_parameters: true`).
**Whether OpenRouter's wrapper for `typesafe/jev-1.13` actually understands this and returns Jev's
calibrated `choice`/`noul` answers, or just free-form chat text, is unverified** — confirm with one
$0.0001 live call before trusting anything past that.

Estimated cost, minimal state (finding + primary file + one-line task context + the three
questions), chars/4 token heuristic: **~12,100 input tokens for all 49 items, ~$0.0005 total**
(output free, $0.042/M input — `jev_request.ts`). Lower than the note's own $0.002-0.009 estimate
for 59 items because the state here carries no whole-review-file context, only the one finding.

**The one command**, from `scripts/experiments/e2-review-replay/`, once `OPENROUTER_API_KEY`
resolves from the process environment, the Windows USER environment variable, or Credential Manager
target `knowlu/dev/openrouter` (tried in that order, `credentials.ts`; never printed or logged):

```
deno run --allow-read --allow-write --allow-run=powershell.exe \
  --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE --allow-net=openrouter.ai run.ts \
  --corpus-root <C1b review folder> --workspace <e2 workspace>
```

Without a key, or with `--dry-run` added, it runs every step through the cost estimate and prints
the first three request bodies, sending nothing.

## 7. Disposition labels: how they were derived

Not parsed — curated (`dispositions.ts`), against every re-review document that exists
(`task-{1,3,7}-rereview.md`, `final-rereview.md`, and the plan review's own "Re-review after fix
round 1/2" sections), because a generic parser over free-form ruling prose would be more fragile
than the 60-item table it replaces. `dispositionFor()` throws on any id the table does not cover.
One consequence worth stating plainly: **every one of Set A's 24 original findings reads as
resolved** in the plan review's own re-review — a true picture of how thorough that particular
review's fix-and-recheck loop was, not a labelling error, but it means Set A alone teaches nothing
about the "ruled against"/"handed off"/"deferred" classes; those come entirely from Set B.

## 8. Fix round 1 (2026-09-22): plural/inflected secret terms

The task review approved the work with one Important finding: every `SECRET_TERMS` pattern in
`secretfilter.ts` was `\bTERM\b` on the singular only, so a plural or inflected mention — "tokens",
"sessions", "secrets", "credentials" — was not caught. Live in the corpus: **`B-final-F2`**'s kept
text read "...is about the URL, the port and the tokens, not the attestation", which named a token
(plural) and should have been dropped under the binding rule ("drop, never redact, any finding that
names a secret, a token or a session"), applied mechanically, not by a human judging each mention.

**Test-first.** Four new rows added to `secretfilter_test.ts` before any fix, covering the plain
plural of every single-word term (token→tokens, session→sessions, secret→secrets,
credential→credentials), the plain plural of every multi-word term (anon key→anon keys, credential
manager→credential managers, client id→client ids, api key→api keys), a plural embedded in a
snake_case identifier, and that `secretMentionTerms` reports the matched plural form itself (not a
normalised singular). RED: 4 of 8 tests in the file failed (`mentionsSecret("about the port and the
tokens, ...")` returned `false`, expected `true`; three more of the same shape).

**Fix.** Every `SECRET_TERMS` pattern's last word gets an optional trailing `s` before the closing
`\b` (`/\bsessions?\b/gi`, `/\btokens?\b/gi`, `/\banon keys?\b/gi`, etc.) — none of the eleven terms
pluralise irregularly (no `-es`, no `-ies`), so this is the whole fix, not a general English-plural
rule. GREEN: all 8 tests in `secretfilter_test.ts` pass; full suite re-run, 54/54 pass.

**Re-extraction.** `run.ts` re-run end to end (`--dry-run`, no network permission needed for this
part): raw findings still 60 (unchanged — the plural fix only affects the drop filter, not parsing);
**11 dropped** (was 10) — `B-final-F2` newly caught; **49 kept** (was 50). `corpus.jsonl`,
`dropped.json`, `summary.json` and `baselines.json` in the workspace directory are all regenerated
with these numbers; every count and baseline number in this report (§1, §3, §4, §5, §6) is updated
to match. `deno check`/`deno lint`/`deno fmt --check` all still clean; no new warning.

```
deno test --config deno.json --allow-read --allow-write --allow-run=powershell.exe \
  --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE .   # with E2_CORPUS_ROOT set; 54 passed at
                                            # this round (57 after the final-review paths fix)
deno check --config deno.json *.ts          # 19 files, clean
deno lint --config deno.json .              # 18 files, clean
deno fmt --config deno.json --check .       # 19 files, clean
deno run --config deno.json --allow-read --allow-write --allow-run=powershell.exe \
  --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE run.ts --dry-run \
  --corpus-root <C1b review folder> --workspace <e2 workspace>   # re-extraction + baselines, no network
```

The paid run was not executed for this fix round, per instruction.

## Files

- `scripts/experiments/e2-review-replay/` — all 9 modules + their test files, `deno.json`.
- `.superpowers/sdd/2026-09-22-judgment-quality-plan/e2/` (git-ignored) — `corpus.jsonl` (49 kept
  findings, leak-free), `dropped.json` (11 audited drops), `summary.json`, `baselines.json`.
- Full report: `.superpowers/sdd/2026-09-22-judgment-quality-plan/e2-report.md`.
