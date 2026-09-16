# C2 — the whole-branch review of `c2-judge` (2026-09-15/16)

The final review of the C2 stream (the judgment service: `docs/plans/2026-09-09-c2-judge-plan.md`,
Tasks 0–15, executed 2026-09-14/16 on branch `c2-judge`, draft PR #4 → `main`). It ran after Task 15
closed the branch and `main` was merged in (`8e2a945`), as four area passes on the most capable model
available, each over its own diff package of the whole range `main..c2-judge`: the engine
(`engine/`), the cloud's shared modules, migrations and eval (`cloud/supabase/functions/_shared/`,
`cloud/supabase/migrations/`, `cloud/eval/`), the eleven C2 functions, and the app, CI and docs. Every
finding was ruled by the controller into ONE fix brief (ruling R-C2-E52), executed as one fix wave of
six commits, then re-reviewed once against the brief's items. The reviewers' transcripts did not
survive on disk; this report is condensed from the SDD ledger (preserved beside it as
`2026-09-16-c2-judge-sdd-ledger.md`), which carries every finding as the reviewer stated it.

**Verdict of the four passes:** *With fixes* in every area — three Criticals and fifteen Importants,
none of which the per-task reviews could have seen (each is cross-cutting: a view created by the
plan's own SQL, a scope the plan's consent never asked for, an app rule the plan's H5 broke).

**Outcome:** every Critical and Important is fixed on the branch (the six commits below), verified
on staging where the fix is observable, and re-reviewed. The residuals are minors, each named with
where it goes.

## The three Criticals

| # | Where | What the reviewer found | Fix | Commit |
|---|---|---|---|---|
| C-1 | `20260911000100_judgment_service.sql:188` | `monthly_spend` was created with no `security_invoker` and no revoke, so it was a definer view that Supabase's default grants made SELECT-able by `anon` — **verified live**: `GET /rest/v1/monthly_spend` answered 200 to the anon key shipped in every install (C1's revoked `telemetry_daily` answered 401), returning every account's id, month and spend. The migration guard scanned RLS on tables and never looked at a view. | `20260911000900_final_review_fixes.sql`: `revoke all on public.monthly_spend from anon, authenticated` (it stays a definer view because `enforce_budget` reads it through the service role); `migrations_test.ts` gains a view guard — every `create [or replace] view` in any migration declares `security_invoker = true` or is revoked from `anon, authenticated` in that or a later migration (RED against a scratch copy without the migration, then GREEN). **Verified live after the push: the anon key now gets 401.** | `34b1d86` |
| C-2 | `google-callback/index.ts:37` | `body.id_token.split(".")` unconditionally, but Google returns `id_token` only when an identity scope was requested and `google-connect` asked for exactly one API scope (pinned by a test named "one scope per consent") — every real connect would have thrown the day P2 was set. Nothing caught it because the callback test injected `exchange` with `sub` present and `index.ts` was never executed. | The consent now requests `openid email` beside the one API scope (the test became "exactly one Google API scope per consent"); the token exchange moved out of `index.ts` into an exported function tested three ways with a fake fetch (an `id_token`; none, so `sub`/`email` come from the userinfo endpoint; neither, a named error and no `TypeError`). `google_accounts.scopes` records Google's answer verbatim, so it now includes the identity scopes. **P2 must know:** the consent screen's scopes are `openid`, `email` and the API scope. | `4081b70` |
| A-1 | `app/src/scheduler.rs:323` + `engine/src/ingest.rs` | H5 runs `ingest` for every vault with a `config/cloud.yaml`; a student who left the optional LMS feed empty has no `ics_url` and no `lms_ics` row, `/ingest-ics` answers 404, and the engine exited 1 — a run-and-failed step (amber tray, retry backoff) every slot forever, the exact invariant CLAUDE.md and the scheduler's own doc name. | The engine distinguishes a 404 from every other service failure: with an empty local url a 404 is a named exit-0 skip (`ingest: no LMS feed on this account — skipped`); any other failure keeps exit 1 and names the HTTP status honestly. The scheduler pushes the `ingest (skipped: …)` line only when it actually leaves the step out, so a cloud vault's run summary names `ingest` exactly once (A-2). | `df9b91a` |

## The Importants, by area

### Engine (`engine/`) — commit `73588c2`

Token discipline, exit codes, every note write through `write`, `write_mapping` byte-exact, the
oracle provably out of reach (no fixture carries `cloud.yaml`; the oracle spawns only `rank`) and
the wire contracts field-by-field were all clean. Three Importants:

- **E-1 — the cloud arm could outrun the app's 20-minute child timeout.** `enrich_with` started its
  own clock and `pull_rules` took no budget (probe 120 s + 15 min + a call in flight + one GET ≈ 21
  min). Now both `enrich_with` calls receive the budget remaining since the arm started and
  `pull_rules` takes a budget, checked before each request, with a "left for the next slot" line.
- **E-2 — `outcome: "capped"` read as fifty model failures** (regrades m5). The three verdict-null
  arms read only `cause`, so the daily cap, the monthly ceiling and an unreachable cap store each
  became `no verdict` → `model failed` per item, fifty calls deep, with the wrong cause in the judge
  log. Now `outcome` is read first: `capped` is the closed-set fatal label `the daily judgment cap`
  and stops the batch after one call exactly as a 402 does; any other null verdict uses `outcome`
  as the cause.
- **E-3 — sixteen new note-writing tests omitted `journal::DEVICE_ENV_MUTEX`**, the getenv/setenv
  race the crate has paid for twice (nine in `enrich.rs`, seven in `coursework.rs`). Each now takes
  the lock first.
- **E-4 (folded minors m102, m104, m106, m107, and the device's `deferred` count m128):** server
  `warnings` are clipped and scrubbed before entering a run record; the comment at the roster
  re-read corrected; `pull_rules`' doc states its predicate instead of "runs on every cloud arm";
  the `gmail: 0 queued` line suppressed when nothing was queued; the dead `CloudClient.account_id`
  deleted; `pull_gmail` prints `N deferred by the service` when non-zero.

### Cloud shared, migrations, eval — commits `34b1d86` (C-1) and `1a8fec2`

- **S-1 — `gmail_queue` was never swept.** Delivered rows kept a model-written, subject-derived
  title forever. A nightly `knowlu-sweep-gmail-queue` (07:31 UTC) deletes delivered rows older than
  seven days; undelivered rows are never touched (a laptop off for a fortnight must still get its
  tasks). The `…000200` comment that understated the payload's contents is corrected beside the
  table.
- **S-2 — tier 2 was structurally dead for `kind: "email"`.** `features()`/`featureMap()` keyed on
  organizer, source, series and title, none of which a Gmail item carries, so no email rule could
  be looked up or promoted. Both twins now map the sender into the existing `source` feature (no
  migration: `source` is in the check constraint); the file states which email fields are
  deliberately not features (the subject text).
- **S-3 — the account-scoping guard was narrower than its header claimed.** It scanned `.select(`
  only, in template literals only, in two fixed filenames. Now `.select(` and `.update(` in both
  quote styles, `p_account` required on every `.rpc(` to an account-keyed function, every non-test
  `*.ts` in each function directory, and the header says exactly that (RED cases for an unscoped
  update and for a helper file).
- **S-4 — the "not configured" 503 string was a five-copy exact-match contract** (regrades m83).
  The engine matched `gmail-read`'s copy by equality and no test pinned that copy; a wording edit
  would have turned a named skip into a failed slot step. One exported constant in
  `_shared/google_scopes.ts`, imported by both handlers, and the engine's literal pinned against
  the TypeScript source from the Rust side.
- **S-5 — the eval gate could spend unbounded money once cases exist.** A `MAX_CASES` (200) limit
  per kind in `load()`, a printed `N cases, ≤ N model calls` line before the loop, and the job
  comment stating that fork PRs get no secrets and therefore no real run.
- **S-6 (folded minors m111, m112, m116–m122):** the superseded `create extension` line and its
  wrong cross-reference marked as comments (applied SQL untouched); `monthly_spend`'s comment says
  it prices every month at the current pin; the SQL twin's subtracted field keys pinned against
  `featureMap`'s keys; the `judgments.fields` privacy tripwire extended to `kind: "email"`; a test
  that every directory under `functions/` has its `verify_jwt = false` entry; `judge_log.ts` logs
  the error's class; "four features" → five; the `CALL_TIMEOUT_MS` sentence repaired; `fieldsOf`
  merges the verdict first and the feature map last, asserted.

### The eleven functions — commit `888e8b8`

- **F-1 — `ingest-ics` and `ingest-calendar` fetched the student's stored URL unguarded**
  (regrades m36): no host or DNS check, redirects followed unvalidated, no body cap, no timeout,
  and a reachability oracle through two distinguishable 502s — while `/events` guarded the same URL
  class four ways. `guardedFetch` and its helpers moved from `events/handler.ts` to
  `_shared/guarded_fetch.ts`; both `fetchText`s call it with the same limits; a refusal answers the
  same body as an unfetchable feed. Each handler test adds a case where the injected fetch is the real guard against a loopback host: the same 502 body as an unfetchable feed, and neither the host nor the path reflected.
- **F-2 — `gmail_seen` was unbounded and its read truncated silently.** Past 1,000 rows (PostgREST's
  silent default) a uid was re-judged (cap spent) and `markSeen` 409'd, so the whole request 500'd
  forever; `knownCourses` had the same unbounded read. A nightly `knowlu-prune-gmail-seen` (30
  days), a windowed and explicitly limited seen read, an idempotent `markSeen` (`on_conflict` +
  ignore-duplicates, through a tested `onConflict` option on the shared insert), and a limit on
  `knownCourses`. The shared insert's request shape became a pure, tested function (the first draft of the test needed environment access the CI command does not grant, and was caught by running the exact command); the seen window is 14 days, ordered, limited to 5,000.
- **F-3 — `gmail-read` listed one page of 100 and dropped `nextPageToken`**, so a mailbox past
  ~100 messages a week was permanently partly read. The list now pages until `READ_CAP` unseen ids
  are collected or the pages run out, with a hard ceiling of five pages. A `pagedList` with six tests, including the five-page ceiling and that only unseen ids count toward the want; the handler still filters and caps the result itself.
- **F-4 — the 40-second model budget bounded when the last call started, not ended** (worst ≈ 160 s
  against the 150-second edge wall clock). The check now counts the call's own timeout, and the
  Gmail path uses a shorter per-call timeout. Ruling R-C2-E53: `READ_BUDGET_MS` became 40 s plus the 60 s per-call timeout (100 s) with the check counting the call's own timeout, pinned by a test that the two differ by exactly 40 s; the brief's literal arithmetic against an unchanged 40 s would have refused every call.
- **F-5 — a deleted account left a live Google grant, and a reconnect to a different Google account
  never revoked the old one.** Account deletion (C1's `account/index.ts`; C2 owns the grant
  lifecycle) now reads the grant, revokes it at Google and deletes the Vault row before the
  `accounts` delete — a failed revoke never blocks the deletion; the callback revokes the previous
  token when Google's `sub` changes. Two new pure modules (`account/google_delete.ts`, `google-callback/reconnect.ts`) over one shared `_shared/google_revoke.ts`, thirteen tests, six of them proving a failed or throwing revoke never blocks the deletion or the new grant; a RED run without the guards failed exactly those six.
- **F-6 (folded minors m127, m131, m132, m134, m135):** the consumer-less `courses` field dropped;
  the `gmail-read` privacy comment reworded (a model-written title and rationale are stored, never
  the text); `deliver` caps the `ack` array at 500; the judge-rules test renamed to what it proves;
  the callback's pages carry `X-Content-Type-Options: nosniff` and a CSP. Tested: 600 acks in, exactly 500 delivered and the drop logged; both headers asserted on all four page paths.

### App, CI, docs — commit `df9b91a`

- **A-2 — `run_slot_inner` recorded two `ingest` lines for a cloud vault** (the skip line on
  `IcsState::NoUrl`, then the real step because `cloud.yaml` exists). One decision point now; a cloud
  vault's `RunSummary.steps` names `ingest` exactly once. `ingest_included(vault)` is the one predicate both `slot_argv` and the run summary read; the RED run showed the double entry exactly as reported.
- **A-3 — stale sentences.** CLAUDE.md's `ingest` line and the scheduler's `ics_state` doc described
  pre-H5 behaviour; HANDOFF claimed `set_google_calendar` "exists with no caller" (it does not, ruling
  R-C2-E6: C4 writes the command and the row together); three plan claims (phase (b)'s console
  command, "C2 never edits `app/`" — m144) carry override notes in the plan's existing style.
  A fifth stale claim the implementer noticed ("C2 never edits `app/src/scheduler.rs`", plan ~313) got its note in the docs commit under ruling R-C2-E54.
- **A-4 — the eval gate could never wake up.** Its vacuity check read only the local seed
  directory, but consented cases (C4's opt-in) land in `eval_cases`. Ruling: the local seed first;
  if empty and a service-role key is in the environment, count `eval_cases` and run when any exist;
  if empty and no key (a fork PR, a secretless run), print the named line and exit 0 as today —
  and the no-key path reads no other environment variable. `anyEvalCasesExist` asks the database once; the no-key test's environment fake throws on any read other than the key's; the woken-gate test scores one database case with no local seed and writes three `eval_runs` rows.
- **A-5 — the wizard's Google flow lost state** (regrades m59 + m60). Its messages lived in the DOM
  and a consent that finished after the last poll could be lost at Finish with no recovery path.
  The state lives in `WIZ` and renders like every other wizard field; the button is disabled during
  the poll; `wizGo` cancels a running poll; `wizFinish` re-reads `google_connected` so the vault is
  born with the `cloud:google` entry. `scripts/wizard-check.py` was not re-run on this branch (no
  Playwright on the machine) — recorded in HANDOFF. The controller then ran `scripts/wizard-check.py` at `df9b91a` in a fresh Playwright venv: `ok`, exit 0 (it drives the C1 wizard's nine panels with a fake `__TAURI__`, so it is a regression check on the rewritten `console.js`, not a drive of the Google button itself).
- **A-6 (folded minors m136–m142, m145):** the wizard's 401 ("sign in again") and 503 ("Google
  sign-in is not available right now — use the secret address below") copy; a stale assertion
  message relabelled; `engine/tests/workflows.rs` pins that the `eval-gate` job exists, is
  `pull_request`-only and names its two secrets; the touched-files regex covers
  `judge_pipeline.ts`; the staging URL is one job-level `env` key; the job comment states that
  PR-authored code runs with the staging service-role key and that fork PRs get no secrets; a
  `calfeed.rs` note that the `cloud:` contract is enforced in `cli.rs`. A pure `google_error_for_status` with three tests; the eval-gate job pinned by a workflows test; the workflow re-parsed as YAML.

## The fix wave's gate and the staging verification

The gates, exactly as CI runs them, on the branch's final head `74da6a3` (the fix wave, round 2 and its two follow-ups):

| Gate | Result |
|---|---|
| Deno (`cloud/supabase/` + `cloud/eval/`) | 347 passed, 0 failed (298 before the wave, 343 after its six commits); `deno check` and `deno lint` clean |
| Rust (`cargo test --workspace`) | green at 0 warnings beyond the accepted `.rsrc` line; engine lib 931 passed, 3 ignored; app `scheduler` 25 passed, 1 ignored (the four by-design ignores, none touched); oracles unchanged; nothing under `engine/tests/fixtures/` |
| CI on PR #4 (run 35068063610 on the wave's head; run 35072171074 on `74da6a3`) | `test`, `cloud`, `eval-gate` all green on both; the eval gate still vacuous at 0 cases with no secrets set |
| `scripts/wizard-check.py` at `df9b91a` and again after round 2's Finish change | `ok`, exit 0 both times |

On staging (`brvhgbihxevrudqpulcm`), all by the controller:

- `20260911000900_final_review_fixes.sql` pushed (the dry run listed exactly it); eleven functions
  redeployed (`--use-api`): the ten C2 functions whose bundles changed plus `account`;
  `ingest-coursework` verified untouched by grep and left alone. Round 2 redeployed `account` and
  `google-callback` (the latter twice); both boot and answer their designed refusals.
- `GET /rest/v1/monthly_spend` with the anon key: **200 before the push, 401 after** (C-1 closed
  live); `cron.job` carries `knowlu-sweep-gmail-queue` (07:31 UTC) and `knowlu-prune-gmail-seen`
  (07:33 UTC) with their exact commands; `google-connect?scope=calendar` with a session still answers
  the named 503 (P2 unset; the S-4 constant is what it answers with).
- **The device side, end to end:** a scratch copy of the s1 fixture with a `config/cloud.yaml`
  (`api_base` ending `/functions/v1`) and no `ics_url`, the session credential written as the
  JSON blob under the vault's `session_credential_target`, `PUT /account/sources` with a public
  iCal — `knowlu-engine ingest` exited 0: `ingest: 317 events, 318 action(s)`, 165 past items
  archived on the first run, 152 tasks created. The feed was fetched server-side through
  `_shared/guarded_fetch.ts` (F-1) and delivered to a device that holds no URL at all (the H5/A-1
  cloud arm's success path). The scratch source row, credential and vault were removed after; no
  token was written anywhere but the credential store.

## The re-review

One scoped re-review of the whole fix wave (the most capable model, seven passes over the six
commits' 424 KB package plus reads of the worktree at the head), with the contract "ADDRESSED or
NOT ADDRESSED, the specific defect gone, not attempted": **every brief item and every folded
sub-fix addressed; no new Critical or Important breakage.** Sections A–C, whose implementer was cut
off before writing a report, were verdicted from the diff alone and hold. It confirmed the
invariants directly: nothing under `engine/tests/fixtures/` changed, no surface reference was
regenerated, every touched file is LF, applied migrations carry comment-only corrections, and no
secret appears in a file, a log line, a prompt or a test name.

It found nine Minors in the fix diff itself. Seven were fixed in a small second round (ruling
R-C2-E55) because each changed behaviour on a real product path or weakened a test the round
touched anyway:

- the account function importing the model SDK through a shared module (now the plain-fetch
  database client);
- a reconnect that could fail a good consent when its grant lookup failed transiently (now logged
  and stored without the stale revoke);
- Finish disabling itself only after an IPC round trip (now first);
- the engine's "no LMS feed" skip matching any 404, a gateway 404 included (now gated on the
  handler's own body, pinned across the language boundary as the Gmail string is);
- the view guard blind to a materialized view (now seen; none exists);
- the old wall-clock budget test made near-vacuous by the new check (rewritten against the
  constants, exact count asserted);
- the workflows test slicing to end of file (now bounded at the next job).

Round 2 landed as one commit (`7279f96`; 13 files; Deno 347 passed; the workspace green at 0
warnings), with `account` and `google-callback` redeployed and no migration. The same reviewer verdicted round 2 and its two follow-up commits: every item addressed, no new
Critical or Important breakage; its one cosmetic note (a doc comment attached to the wrong item)
was fixed in the last commit.

Two were parked with rulings: the scoping guard has no fail-loud count for a `.select(` with a
variable path (every call today is a literal; m146), and a failed `eval_cases` count with a key
present now fails the gate instead of passing vacuously (right for a PR-only gate during a staging
outage; m147, stated in the job comment). One is F-1's ruled intent on the record: stored LMS and
calendar URLs are https on port 443 only.

## Residuals — deferred with a home

Every one is a minor. Numbers are the ledger's.

- **Found by the controller's live smoke, not by any review (m148):** Supabase's functions relay
  rewrites an HTML response to `text/plain` on the shared `*.supabase.co` domain (its docs: HTML is
  served only from a custom domain, a paid add-on), so the Google callback's consent pages reached
  the browser as raw source. Ruling R-C2-E56: the pages are plain sentences now (`7279f96`'s
  successor commit); a custom functions domain or a redirect to a page on `knowlu.com` is a C4
  decision for Quinn once the site is public.
- **To C4 (the local runtime's removal), because it owns the files:** the `enrich.rs` split (the
  file is ~2,500 lines; a `gmail.rs` is the natural home — m77); a product caller for `DELETE
  /google-connect` (the settings row — m123); the wizard's Google commands being pending-target
  only (m124).
- **Cloud, product-side monitoring is ops, not code:** nothing reads `cron.job_run_details` (m86);
  `/events` is an unmetered relay (m129); `charge_call` counts refused attempts (m113) and a charge
  is lost to a pre-model throw (m130) — both bounded by the cap; `course_exact` credits a dead
  model on a null-course label (m114, sibling of m95); unclipped `categories`/`audiences`/
  `known_courses`/dates into the prompt (m115); a few reserved-IP literal forms the guard's parser
  does not normalise (m133); `ingest-calendar` reads one page (m61); the scoping guard has no
  fail-loud count for a `.select(` with a variable path (m146); a failed `eval_cases` count with a
  key present fails the gate on purpose (m147).
- **Engine:** the probe's GET duplicated by `pull_rules` (m100); the final ack flush returns items
  the device discards (m103); `resolve(vault).ok()` silent at three sites and named at one (m105);
  a mixed-indent block can get a duplicate `courses:` (m108, m32's branch); `config/ingest.yaml`
  edits are unjournalled (m109 — stated in HANDOFF); the failure-marker test proves classification
  only (m110).
- **App:** one `async () =>` with no await (m143).
- **Earlier tasks' minors (m1–m99)** are listed per task in the preserved ledger; none was
  regraded by the final review except m5, m36, m59, m60 and m83, all fixed above.

## What the branch needs from Quinn (unchanged by the review)

- **P1** `ANTHROPIC_API_KEY` on staging — then the controller smokes `judge-task`, `judge-event`
  and `judge-email` live.
- **P2** the Google OAuth client and consent screen (Web application client; redirect URI
  `https://brvhgbihxevrudqpulcm.supabase.co/functions/v1/google-callback`; secrets
  `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`; Testing mode) — **the consent now requests `openid`,
  `email` and the API scope**, calendar first, Gmail incrementally. **P3** the first test user.
- The two GitHub secrets for the eval gate: `ANTHROPIC_API_KEY`, `SUPABASE_STAGING_SERVICE_ROLE_KEY`.
- The merge of PR #4 (`--no-ff`) on Quinn's word.
