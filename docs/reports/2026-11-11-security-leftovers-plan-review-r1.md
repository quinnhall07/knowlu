FAIL

# Plan review r1: security leftovers (advisor gate, client table rights, guard gaps, loopback proxy)

**Document:** `docs/plans/2026-11-11-security-leftovers-plan.md` (r0), worktree `knowlu-leftovers`, branch
`security/leftovers` at `0926f0c1`. **Mode:** spec/plan, round 1. **Reviewer:** `reviewer`, 2026-11-11.

**Verdict: FAIL.** There are no Critical findings. The migration is safe as written. Both triggers keep firing,
because Postgres does not check EXECUTE when a trigger fires, and the existing 09 unmask cases would go red if it did
(see "Checked and sound"). The pgTAP count of 870 is right. The CI step fits the existing shape pin. Three Important
findings remain. R4's argument matching (2c) cannot be built from the plan as written. R4's re-grant matching
misses grant forms that Postgres accepts. R6 would put an unverified reason into a public note.

## Important

**I1. R4 (b) does not say how argument lists are read, and the obvious ways break.** Locations: plan §2 R4, §3 T1a
synthetic cases 4 and 5, T1b. Conflicts with `migrations_test.ts:63-66, 1045, 1661, 3164, 3330`.
- `FUNCTION_DEFINITION` reads the parameter list as `\([^)]*\)`, so it stops at the first `)`. Case 4's
  `p_at timestamptz default now()` stops after `now(`, and case 5's `numeric(10,2)` stops after `10,2`. If T1b
  adds a capture group there, the list is cut short: in case 4, `out r text` is lost. Worse, any parameter after a
  parenthesised default is dropped. Say `f(a int default g(), b uuid)` has an overload `f(int)`. The cut list
  normalises to `integer`, so `revoke … f(integer)` wrongly satisfies the guard for the two-argument function.
- A new group placed before the dollar tag also renumbers it. The `\3` back-reference then points at the
  argument text, and the six other readers of `FUNCTION_DEFINITION` break: four read `m[3]` as the tag, and the
  51-function pin depends on it.
- "Names dropped" has no rule for telling a name from the first word of a multi-word type. Postgres also allows
  names (and modes) in a revoke's own list. Here is the failure: an implementer drops the first word of every
  argument that has two or more words. The revoke's `timestamp with time zone` becomes `with time zone`, and
  case 4 fails although it should pass. The rule also leaves `inout` out.
- "Plain types" is not defined either. The corpus's `store_google_grant(… text[])`
  (`20260911000300_google_privileges.sql:71`) needs `[]` to normalise. A normaliser that accepts only words
  fails closed on a real definer, and that costs a round trip through the controller.
- *Fix:* say that T1b leaves `FUNCTION_DEFINITION` unchanged. It reads each list with a scan that balances
  parentheses, starting at the `(` after the name in `d[0]`, and the revoke's list the same way. Then state the
  rule: split at top-level commas; drop the default (`default …` or `= …`) first; drop `in`/`variadic`; skip `out`;
  keep `inout`; the name is the first word unless the argument is a known multi-word type (`double precision`,
  `character varying`, `timestamp|time with[out] time zone`, `bit varying`) or a single word; keep `[]`; any
  parenthesis left over fails closed. Add a synthetic case where a parameter follows a parenthesised default, and
  one where the revoke list carries names.

**I2. R4 (c)'s re-grant match misses grant forms that Postgres accepts, so gap (c) stays open for them.**
Locations: plan §2 R4 (c), §3 T1a case 6. A matcher built like today's `revokeRe` would require `f\s*\(args\)` then
`to`. Postgres also accepts each of these:
- `grant execute on function f to anon` (no list; valid when the name is unique);
- `grant execute on function g(text), f(uuid) to authenticated` (a list of functions);
- `grant execute on routine f(uuid) to anon`;
- `grant all privileges on function f(uuid) to public`.

Each one restores client EXECUTE on a definer and passes the static guard. `cloud-rls` is not a required check
yet (§7), so the Deno guard is the gate that actually blocks a merge, and these forms get past it. Going the other
way, `revoke all on function f(uuid) from public, anon, authenticated` is a full revoke that the guard will not
count, which is a false failure. *Fix:* match a grant by statement: from `grant … on (function|routine)` to the
semicolon, split the object list, and allow a bare name to match a definition of the same name. Treat
`revoke (execute|all [privileges])` the same way. Add the first two forms to case 6.

**I3. R6 records a reason for the hosted-only `auth_leaked_password_protection` WARN that nothing in the plan
verifies.** Locations: plan §1 F-A, §2 R6, §3 T4, §7. R6 writes that reason into
`docs/notes/2026-11-09-security-tooling.md`, which is public, and nothing in this lane checks it against the hosted
Auth API the advisor reads. *Fix:* the public record names the WARN only as an Auth setting, and the controller
handles anything further under the note's own rule (`:46-47`), outside this lane's scope. *(Passage shortened by the
controller before commit: the original pointed at an item tracked privately.)*

## Minor

**M1. The red-run predictions are off in three places** (§3 T1a "Expected red"; "The controller's red run").
- With `not c.is_trigger` gone, `09:249` (expects `{public.sync_prune}`) and `09:251` (expects `{}`) also go red on
  main, not only the controls at `:255-256`.
- Adding `client_table_rights` to `LATER_LANES` (`:1988`) leaves that test green while the file is absent: an
  extra exclusion changes nothing. So four of the five F-G tests go red, not five.
- The halves of synthetic cases 3, 4 and 6 that should pass already pass on the unchanged guard.

The controller's run should expect these results, not stop on them.

**M2. C10 and C11 need two details stated** (§2 R3).
- C10 must name each relation as `nspname || '.' || relname`. `oid::regclass::text` depends on `search_path` and
  would print `issues`, so the `{public.issues:…}` plants fail.
- C11 should read the `postgres` default ACL with `defaclnamespace in (0, 'public'::regnamespace)`. A global
  default (no `in schema`) adds to the schema one, and a per-schema revoke cannot remove it. So a later global
  `grant truncate on tables to anon` would leave C11 green. C10 would catch it only once a table exists.

**M3. The no-`models` assertion should be `/\bmodels\b/i`** (§3 T1a corpus tests). It has to match
`ci.yml:252`'s `grep -qiE`. The testers test already does (`migrations_test.ts:2926`). Without the flag, a
capitalised `Models` in the header passes the test and still starts the paid eval.

**M4. The advisor step can fail open or break the shape pin** (§3 T3).
- T3's pin accepts `… --fail-on warn || true` and `…; exit 0`. Assert that nothing follows the flags on the line.
- The two-line comment is part of the `job` text that `workflows.rs:471-479, 492-494, 524-529` search. A natural
  comment that mentions `--linked`, `db push`, `secrets.` or `--project-ref` fails the shape test that is meant to
  stay green unedited. So does one that names `scripts/ci/supabase-images.sh` without `bash ` in front of it.
  Name these strings in T3.

**M5. T2's manual proof names two different tests** (§3 T2). The command runs `cloud_contract`'s
`the_task_request_carries…`, but the "before" result is `mail_pass.rs:355`, and "the after-run uses the same
test". Run both, before and after. R5's own failure case is a developer whose `NO_PROXY` lacks `127.0.0.1`. To
prove what `force = true` adds, add one run with `$env:NO_PROXY = "example.invalid"`.

**M6. T2's agent label does not fit** (§3 table). `mechanical.md` excludes "tests that encode new behavior" and
takes a plan task only when "the plan writes out in full" its code. T2 is a red-first test that the plan only
describes. Either write the ~15 lines into the plan or label T2 `test-writer`, then `implementer`.

**M7. T2 and T3 run in parallel in one worktree, which means one git index** (§2 R7, §3 Gates). If either agent
does `git add` and then a bare `git commit`, it can sweep in the other's staged files. That blurs the red-first
commits the ledger cites. Require `git commit -- <paths>`, or run the two tasks one after the other.

**M8. Steps C2 and C4 need two practical details** (§3 close).
- The migration exists only on this branch, which stays local. "The production-linked worktree" must hold the
  branch's file. If it is on main, `db push --linked` reports that the remote is up to date, and C4's re-read
  shows the old grants.
- C2's "text of … F2" calls `tests.client_functions`, which exists only inside a pgTAP transaction. The staging
  read needs the inline catalog query.

**M9. The advisor's image check can close before production** (§3 T3 "Unverified until C5"). At the green run,
the controller can record `docker images` before and after the advisor, or run `supabase-images.sh baseline` and
then `verify` locally. That settles it before C4 rather than after.

**M10. Deno's loopback tests are not covered** (§7). `deno test --allow-net=127.0.0.1`
(`judge_anthropic_test.ts`) also honours `HTTP_PROXY`, and cargo's `[env]` never reaches Deno. That is outside
(2b) as approved. §7 should list it so that the README and `toolchain.md` lines claim no more than cargo.

## Checked and sound

- **The triggers keep firing.** Postgres checks EXECUTE on a trigger function only at `create trigger`. The
  09 unmask cases (`09_self_test_test.sql:780-807`) insert under `anon` and `authenticated`, which fires
  `sync_notes_stamp_rev` (BEFORE, never unmasked) and `sync_usage_bump`. If EXECUTE were checked when a trigger
  fires, the "reads 1 through `tests.unmasked`" and "-2 bare" cases would read -1. §4's claim holds.
  `sync_usage_bump`'s upsert needs only select, insert and update, and the migration leaves those alone. Foreign-key
  checks run as the table's owner, so REFERENCES is never needed at write time.
- **What `revoke … on all tables` covers.** It reaches tables, views, materialized views, foreign tables and
  partitioned tables, never sequences. PostgREST, pg_graphql and the security-invoker `billing_subscribers` need
  only the four verbs. `compRightsProblems`, `routeDailyCorpusProblems`, the grant, address and uuid tripwires, and
  the `\btrigger\b` check (which is scoped to `definer_trigger_execute`) all pass the new file as specified.
- **`for role postgres` is correct.** Locally, migrations run as `postgres` (F-C: it owns every table). On hosted
  projects, the explicit role is right whether the CLI logs in as `postgres` or as a member of it.
  `major_version = 17` (`config.toml:17`) covers the `maintain` keyword.
- **The pgTAP count: 858 + 12 = 870.** 01's fixed cases go from 21 to 23. 09 gains 10 cases (C10 4, C11 3, F2 3).
  05's plan is unchanged at `7 + 2·definers`. Every planted fault runs as the session owner inside `under_fault`,
  which rolls back, and `alter default privileges` is transactional. The C11 fallback is flagged honestly.
- **The CI pin.** `at("db start")` and `at("test db")` do not match an `advisors` line (F-M). The new step meets
  the "every later step has one `if:` holding `code != 'false'`" rule, and the `--workdir cloud` rule. O7 is
  reasonable.
- **The NO_PROXY test is sound.** Check (ii), `force = true`, is what makes it go red on CI, which sets no
  `NO_PROXY`. Check (i) catches a value missing either host. `include_str!("../../.cargo/config.toml")` resolves
  the same way as the existing `WORKSPACE` path.
- **Order and gates.** Staging comes before production, production waits for Quinn's go (C3), the branch stays
  local (R7), `sync-push` is checked live, and no subagent runs the CLI (O3). The plan adds no single-user
  assumption. `SUPABASE_DB_PORT` is documented as a per-machine override. The plan touches no frozen reference,
  no contract-list file, no new data class and no `PRIVACY_VERSION`.
