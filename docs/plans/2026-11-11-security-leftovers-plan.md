# Security leftovers (advisor gate, client table rights, guard gaps, loopback proxy): plan

**Status: executed.** Plan reviews r1 and r2 (`docs/reports/2026-11-11-security-leftovers-plan-review-r{1,2}.md`)
returned FAIL; §9 maps r1's findings to their answers, and the closing paragraph records r2's. The branch review is
`docs/reports/2026-11-11-security-leftovers-review.md` (PASS). **Date:** 2026-11-11.
**Size: Contract** (a migration). **Branch:** `security/leftovers` (worktree `../knowlu-leftovers`, from `origin/main`
at `0926f0c1`). The branch and this plan stay local until production has the migration (R7).

**Goal and scope** (Quinn's approval, 2026-11-11; all decided, none exploitable today). **(1)** The Supabase Security
Advisor gates `cloud-rls`, and production's first advisor read goes on record. **(2a)** A migration takes TRUNCATE,
REFERENCES, TRIGGER and MAINTAIN from `anon` and `authenticated` on every `public` relation, future ones included, and
EXECUTE on the two trigger functions; pgTAP checks fail on today's schema and pass after it. **(2b)** `NO_PROXY` is
forced for every process cargo starts, so a developer's proxy never captures a loopback test. **(2c)**
`assertDefinerRevoked` closes three gaps: a later re-grant, PUBLIC, and overloads. **(3)** GitHub secret-scanning alert
#1 (a fake `sk_live_` key in the scrubber tests) was resolved by the controller as used-in-tests; it is recorded here
and has no task. The Gmail-label live proof stays deferred. Nothing a student sees changes (§5).

**Read for this plan** (at `0926f0c1`): `CLAUDE.md`, `HANDOFF.md` §1 and §4, `VISION.md`, the RLS plan's §3/§11/§12, the
r1 review, and every file cited `path:line` below, including every `security definer` and `revoke|grant execute` line.

## 1. Facts

The controller's reads (2026-11-11), cited and not re-derived:
- **F-A, the hosted advisor** (`db advisors --linked --type security`). Production (read for the first time) and staging
  are identical: 26 × INFO `rls_enabled_no_policy` (the service-only tables, by design) and 1 × WARN
  `auth_leaked_password_protection` (an Auth setting, not SQL). `handle_new_user`'s finding is gone (`20261109000100`).
- **F-B, the local advisor** (CLI 3.1.0, pinned and checksummed at `ci.yml:302-303`) prints "No issues found" and exits
  0 on main; a planted RLS-off table makes it exit 1. `--local` is the default (v4 removes it). Its lints (0001–0024)
  lack splinter's 0028/0029 (a definer function a client role can execute), which F1 and F2 in `05_functions` cover.
- **F-C, the grants** (staging, production and local PG 17.11 agree). `postgres` owns every `public` table. 26 relations
  give both client roles every table privilege, MAINTAIN included: the 10 owner-read tables, the 15 service tables at
  default grants, and `billing_subscribers`; the rest were `revoke all`'d. `postgres`'s `public` default ACL grants both
  roles `arwdDxtm` on tables and `X` on functions. Only `sync_notes_stamp_rev()` and `sync_usage_bump()`, SECURITY
  INVOKER trigger functions (`20260912000100_sync.sql:84,135`), are client-executable; EXECUTE is not checked when a
  trigger fires (`20261109000100_definer_trigger_execute.sql:9-11`).
- **F-D, the RLS suite.** 858 pgTAP tests, green on main. C6 checks four verbs (`01_coverage_test.sql:119-130`); F2
  exempts trigger functions (`05_functions_test.sql:3-6,65-78`). `09_self_test_test.sql` plants each check's fault and,
  until M6, repeats each check's text; both copies change together (`rls-pgtap-plan.md` §11).
- **F-E, the proxy.** ureq 3.4.0's default config reads ALL_PROXY, HTTPS_PROXY and HTTP_PROXY and honours `NO_PROXY`;
  the engine's cloud agent starts from it (`cloudmodel.rs:251-260`). Cargo's `[env]` reaches what `cargo test` starts
  inside the checkout, never a shipped binary (`.cargo/config.toml:8-11`, `toolchain.md:36-39`).
- **F-F, runs and pushes.** Locally `SUPABASE_DB_PORT=55322` (Hyper-V reserves 54010–54509). Staging gets `db push` from a
  worktree holding a copy of the main checkout's `.temp` link; production gets `db push --linked`, on Quinn's word only.

Read in the code, correcting or adding to the dispatch:
- **F-G.** Five tests name the last migration: `migrations_test.ts:584` (`files.at(-1)` is
  `<stamp>_definer_trigger_execute.sql`), `:1988` (`LATER_LANES`), `:3342-3344`, `:3517-3519` and `:3647-3648`.
- **F-H.** In `assertDefinerRevoked` (`:533-555`), `revokeRe` (`:543`) accepts only `revoke execute on function
  <name>(…)`, skips the arguments, tests the from-list with two substring `includes` (`:546-549`) and reads no `grant`.
  `FUNCTION_DEFINITION` (`:63-66`) reads parameters as `\([^)]*\)`, which stops at the first `)`; four readers take its
  third group as the dollar tag (`:1045, 1661, 3164, 3330`). The search starts at the definition's file (`:541`), so a
  revoke above a re-creation in the same file still counts. No migration has `drop function`.
- **F-I.** The corpus should pass the stricter rule (T1b proves it). The seven definers take simple types,
  `store_google_grant`'s `text[]` included (`20260911000200_google.sql:106-174`,
  `20260911000300_google_privileges.sql:28-30,71`, `20261002000200_mail.sql:371`, two `()` triggers). Each one's last
  revoke follows it and reads `from public, anon, authenticated`; EXECUTE goes only to `service_role` and
  `supabase_auth_admin`.
- **F-J.** `assertExecuteRevoked` has the same three gaps (`:159-177`).
- **F-K.** The eval gate runs, and spends on the hosted model, for any changed migration matching `\bmodels\b`
  case-insensitively, comments included (`ci.yml:251-253`).
- **F-L.** `cloud-engineer` never runs the supabase CLI, even locally (`cloud-engineer.md:24`): every pgTAP and advisor
  run here is the controller's. No PR #66 test exists in `engine/tests/`.
- **F-M.** The `at()` needles in `workflows.rs:533-545` (`db start`, `test db`) match one line each; an `advisors` line
  matches neither.

## 2. Rulings

The controller's. "r1 X" marks a revision; *added* marks this plan's additions (§8 O6). Each reads: ruling, why, cost if
wrong.

**R1, the advisor gate (r1 M4).** *Ruling:* one `cloud-rls` step between `pgTAP` (`ci.yml:333-335`) and `nothing ran
that is not locked` (`:338-340`), carrying pgTAP's `if: needs.changes.outputs.code != 'false'`. Its whole `run:` line
is `run: supabase --workdir cloud db advisors --type security --level warn --fail-on warn --output-format text`, with
nothing after the flags; a new `workflows.rs` test pins it, and its comment avoids every string the shape test reads
(T3). *Why:* the job's CLI is already checksummed; `splinter.sql` was rejected because the CLI embeds it; 0028/0029
belong to F1 and F2; a trailing `|| true` or `; exit 0` would fail open. *Cost if wrong:* an unwanted lint fails CI,
and one flag fixes it.

**R2, the migration `cloud/supabase/migrations/20261111000100_client_table_rights.sql`.** *Ruling:* exactly these four
statements, under a header in `20261109000100`'s style that names no table and never the word F-K matches:
```sql
revoke truncate, references, trigger, maintain on all tables in schema public from anon, authenticated;
alter default privileges for role postgres in schema public
  revoke truncate, references, trigger, maintain on tables from anon, authenticated;
revoke execute on function public.sync_notes_stamp_rev() from public, anon, authenticated;
revoke execute on function public.sync_usage_bump() from public, anon, authenticated;
```
The header gives the reasons: TRUNCATE bypasses RLS, so the 10 owner-read tables are the important half; MAINTAIN is
PG17's member of the same class; the default revoke keeps future tables from regaining these rights; the triggers keep
firing (F-C). Select, insert, update and delete are untouched. *Why:* it covers every relation, so no list has to stay
complete. *Cost if wrong:* a client path that needs one of these rights breaks; none exists, and C2 would show a
trigger failure. A REVOKE by a non-grantor only warns, so C10 and C11 read the results, not the statements.

**R3, pgTAP (r1 M2).** *Ruling:* `01_coverage_test.sql` gains two checks. **C10:** no relation in `public` (relkind r,
p, v, m, f) on which `anon` or `authenticated` holds truncate, references, trigger or maintain per `has_table_privilege`
(which counts PUBLIC), each hit named `nspname || '.' || relname || ':' || role || ':' || privilege`. **C11:**
`postgres`'s table default ACL, read with `defaclnamespace in (0, 'public'::regnamespace)`, grants `anon`,
`authenticated` or PUBLIC (*added*) nothing outside SELECT, INSERT, UPDATE and DELETE, each hit named
`grantee:privilege`. F2 in `05` drops `not c.is_trigger`; its header (`05:1-6`) and 09's copy (`09:241-248`) change with
it. Each new or changed check gets 09 plants, and the tests come first, red on main. *Why:* the checks read the
catalog, so a later re-grant fails by name; `oid::regclass::text` follows `search_path` and would print `issues`; a
global default adds to the schema one, and no per-schema revoke removes it. *Cost if wrong:* a platform relation fails
C10 on some image; it is named and the controller rules, never silently excluded. A global entry is §8 O8.

**R4, the static guard (r1 I1, I2).** *Ruling:* `assertDefinerRevoked` keeps `FUNCTION_DEFINITION` and its groups
unchanged (F-H) and changes in four ways.
- **(a) PUBLIC.** A qualifying revoke's from-list holds `public`, `anon` and `authenticated` as whole tokens.
- **(b) Signatures (I1).** A definition's argument list is read by a separate balanced-parenthesis scan from the match's
  opening `(` (the first after the name in `d[0]`), skipping `'…'` literals; a list item in a revoke or grant is read
  the same way from its own `(`. Each list is normalised: (1) split on top-level commas, where an empty list `()` is
  zero arguments, never one empty argument (r2 N1: `handle_new_user` and `trim_user_metadata` are `()`); (2) drop a
  top-level `default …` or `= …` tail; (3) lower-case and collapse whitespace, none kept beside `(`, `)`, `[`, `]` or a
  comma (before any token split, r2 M-a); (4) strip a leading mode word, where an `in`, `inout` or `variadic` argument
  keeps its type in the signature and an `out` argument is skipped; (5) if the first token can start a multi-word type
  (`timestamp`, `time`, `character`, `double`, `bit`, `interval`, `national`), or the argument is one token, there is
  no name, otherwise the first token is the name and is dropped; (6) keep `[]`, drop every type modifier (each
  parenthesised group inside a type: Postgres ignores them in a signature, so `numeric(10,2)` is `numeric`, r2 M-a), then
  map int/int4→integer, int8→bigint, bool→boolean, timestamptz→timestamp with time zone, varchar→character varying,
  float8→double precision. A scan that does not close before the body, an empty argument inside a non-empty list, or an
  argument left with no type fails closed, naming the function.
- **(c) Statement forms (I2).** Each statement runs to its next `;`, as `revokeRe` does today. A *qualifying revoke* is
  `revoke (execute | all [privileges]) on (function | routine) <list> from …` where one item names the function with a
  matching signature (an item with no argument list matches every overload) and (a) holds. A *re-grant* is any later
  `grant` whose privileges include `execute` or `all`, whose grantees (read before any `with grant option` or
  `granted by`) include public, anon or authenticated, and whose target is `function`, `routine` or `procedure`
  `<list>` with an item naming the function (matching signature, or no argument list), or `all functions` or
  `all routines in schema public`. The guard fails with no qualifying revoke, or with a re-grant after the last one.
  *Grantees* (r2 N2) are the comma list after the top-level `to` (grant) or `from` (revoke) that follows the object
  list, read up to `with grant option`, `granted by` or the `;`; each has its double quotes removed and is compared
  lower-cased as a whole token. A schema qualifier in the object list (`public.x`) is never a grantee, so a qualified
  grant to `service_role` alone is no re-grant.
- **(d, *added*)** The search starts at the definition's offset (F-H), the ordering (c) needs. Each form gets a
  synthetic red case first, and `leaky_trigger` (`:565-576`) keeps its intent.

*Why:* `\([^)]*\)` cuts a list at its first `)`, and a new group would renumber the tag four readers use; Postgres
accepts grants and revokes by routine, in lists, without argument lists and as `all privileges`, which a regex shaped
like `revokeRe` misses or miscounts. *Cost if wrong:* a real definer fails the stricter rule (a finding for the
controller, never papered over), or an unlisted form slips past the static guard; F1 and F2 still read real privileges.

**R5, NO_PROXY (r1 M5, M6).** *Ruling:* T2 is the controller's, written out below: a test that (i) the test process's
`NO_PROXY` names both loopback hosts and (ii, *added*) the config line has `force = true` (CI sets no `NO_PROXY`, so (i)
alone passes with `force = false`), and a manual proof on two loopback tests, the last run with `NO_PROXY=example.org`.
*Why:* the line is forced like its neighbours, and that run shows a developer's own `NO_PROXY` losing to it. *Cost if
wrong:* none to the shipped app.

**R6, the records (r1 I3, M10).** *Ruling:* the security-tooling note gains the advisor row, production's first read
(2026-11-11, F-A), and 2a–2c under "Fixed". It records the hosted-only `auth_leaked_password_protection` WARN only as an
Auth setting: not SQL, not covered by the local gate, and left to HANDOFF's Pilot row "C1b's open sign-in settings"
(`HANDOFF.md:573`), with no other reason. `cloud/supabase/README.md` §Testing names the advisor step; lines in HANDOFF
§1 and `toolchain.md` are the controller's call and claim the proxy fix for cargo only. *Why:* nothing in this lane
checks a reason for that WARN against the hosted Auth API, and the note is public. *Cost if wrong:* the note says less
than it could.

**R7, order and gates (r1 M7).** *Ruling:* tasks run one after another in the worktree: T1a, red run, T1b, green run,
T2, T3, T4, T5, close. Every commit names its paths (`git commit -- <paths>`). The plan and the branch stay local until
production has the migration. *Why:* one worktree is one git index, and a bare commit can sweep in another task's
staged files and blur the red-first commits; staying local is the house rule for security leads in a public
repository. *Cost if wrong:* some lost parallel time.

## 3. Tasks

| Task | Agent, and why | Files |
|---|---|---|
| T1a | `cloud-engineer`: `cloud/`; a false green here hides a client right | `cloud/supabase/tests/{01_coverage,05_functions,09_self_test}_test.sql`, `cloud/supabase/migrations/migrations_test.ts` |
| T1b | `cloud-engineer`: the migration and the guard that checks it | `…/migrations/20261111000100_client_table_rights.sql` (new), `migrations_test.ts` |
| T2 | the main session (controller): code written out below; the proof needs a shell (r1 M6) | `.cargo/config.toml`, `engine/tests/dependency_boundary.rs` |
| T3 | `cloud-engineer` (cargo slot): a `cloud-rls` step and its pin | `.github/workflows/ci.yml`, `engine/tests/workflows.rs` |
| T4 | `docs-keeper`: the records | the security note, `cloud/supabase/README.md`; optionally `HANDOFF.md`, `toolchain.md` |
| T5 | `reviewer`: the whole diff before any push | `docs/reports/2026-11-11-security-leftovers-review.md` |

**Gates.** Deno tasks run CI's line from the root (`ci.yml:172`), then `deno check` and `deno lint` as `ci.yml:155,158`
run them, and record pass and fail counts. Files are LF; commits use `git commit -- <paths>`; nothing is pushed before C5.
`deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`

**T1a. Tests first (R3, R4).** In `01`, add C10 and C11, each with a comment naming its 09 plant; `plan(21 + …)` becomes
`plan(23 + …)` (`01:46`). In `05`, F2 loses `not c.is_trigger`; its header says there is no trigger exemption and that
`<stamp>_client_table_rights.sql` revokes both functions. In `09`, F2's copy changes the same way, and `plan(164)`
becomes `plan(174)` for the ten cases below. **Unverified:** whether the session may plant C11's faults (that needs
`postgres` or a member of it); if not, each fault starts `set local role postgres;` and the ledger says so.

| Check | Planted fault | The check must report |
|---|---|---|
| C10 | `grant truncate on public.issues to authenticated` | `{public.issues:authenticated:truncate}` |
| C10 | `grant maintain on public.mail_accounts to anon` | `{public.mail_accounts:anon:maintain}` |
| C10 | `grant references, trigger on public.judgments to public` | four names: both roles × both rights (PUBLIC counts) |
| C10 | none (control) | `{}` |
| C11 | `alter default privileges for role postgres in schema public grant truncate on tables to anon` | `{anon:TRUNCATE}` |
| C11 | `alter default privileges for role postgres grant trigger on tables to public` (global) | `{PUBLIC:TRIGGER}` |
| C11 | none (control) | `{}` |
| F2 | `grant execute on function public.sync_notes_stamp_rev() to authenticated` | authenticated's list: `{public.sync_notes_stamp_rev}` |
| F2 | `grant execute on function public.sync_usage_bump() to public` | anon's and authenticated's lists, each `{public.sync_usage_bump}` (two cases) |

**Synthetic cases** in `migrations_test.ts`, each through the exported guard. A *full revoke* is
`revoke execute on function f(uuid) from public, anon, authenticated`.
1. `from anon, authenticated` throws, naming the function and `public`.
2. `from public, anon_reader, authenticated` throws.
3. `f(p uuid)` revoked as `f(text)` throws, as `f(uuid)` passes; `f(a int default g(), b uuid)` revoked as `f(integer)`
   throws, as `f(integer, uuid)` passes.
4. `(p_n int4, p_at timestamptz default now(), inout p_io bool, variadic p_v text[], out r text)` revoked as
   `(p_n integer, timestamp with time zone, inout boolean, text[])` passes (the revoke list carries a name).
5. `store_google_grant`'s shape `(p_account uuid, p_scopes text[])` revoked as `(uuid, text[])` passes;
   `(p_amount numeric(10,2))` revoked as `(numeric)` passes; `(p uuid,)` throws, naming the function; `f()` revoked as
   `f()` passes (r2 N1).
6. After a full revoke, each throws: `grant execute on function f to anon`; `grant execute on function g(text), f(uuid)
   to authenticated`; `grant execute on routine f(uuid) to anon`; `grant all privileges on function f(uuid) to public`;
   `grant all on procedure f(uuid) to anon`; `grant execute on all functions in schema public to public`;
   `grant execute on all routines in schema public to authenticated`; `grant execute on function f(uuid) to "anon"`
   (r2 N2). A later grant to `service_role` passes, `grant execute on function public.f(uuid) to service_role` included,
   as does a client grant followed by a full revoke, and a full revoke written `from "public", "anon", "authenticated"`
   qualifies (r2 N2).
7. Each qualifies as the revoke: `revoke all on function f(uuid) from public, anon, authenticated`; `revoke all
   privileges on routine f(uuid) from …`; `revoke execute on function g(text), f(uuid) from …`; `revoke execute on
   function f from …`.
8. A revoke above the definition in the same file throws.

**Corpus tests.** The new migration's own test: found by suffix, it is the last file; its statements are exactly R2's
four (house style `:1998-2008`: strip comments, split on `;`, collapse whitespace, lower-case); it creates no function
and grants nothing; its raw text fails `/\bmodels\b/i` (r1 M3). The five F-G tests gain `<stamp>_client_table_rights.sql`,
and `:584`'s `.at(-1)` becomes "the later files are exactly `["<stamp>_client_table_rights.sql"]`".

**Expected red (r1 M1).** On the unchanged guard, cases 1 and 2, 3's throwing halves, 5's `(p uuid,)`, 6's seven grants,
all of 7, and 8 are red; case 4, the passing halves of 3 and 5, and 6's two passing cases are already green. The
migration test and four of the five F-G tests are red; `:1988` stays green, because an extra exclusion changes nothing
while the file is absent. Commit the tests alone. Then **the controller's red run**: `$env:SUPABASE_DB_PORT = "55322"`,
`supabase --workdir cloud db start` (or a local `db reset`), `supabase --workdir cloud test db`. Expect, and carry on:
C10 names F-C's 26 relations; C11 names both roles' four rights (a global entry goes to §8 O8); 05's F2 names the two
trigger functions; in 09 the C10 and C11 controls and exact-set cases fail, as do F2's `:249` (expects
`{public.sync_prune}`), `:251` (expects `{}`), the controls at `:255-256` and the three new F2 cases; the rest pass, and
870 are planned (858 + 12). The ledger records the failing descriptions.

**T1b. The migration and the guard (R2, R4).** Write the migration; add the balanced scan, the normaliser and the
statement matcher as helpers (so §8 O5 is one call later); `FUNCTION_DEFINITION` stays untouched. *Green:* the Deno line
passes; the 51-function (`:347`) and 10-view (`:368`) pins are unchanged; `:557-563` still finds its four names. A real
definer that fails goes to the controller only, never into a commit, a PR or a ledger line. Then **the controller's
green run (r1 M9)**: a local `supabase --workdir cloud db reset` (never `--linked`), `test db`, and `docker images`
before and after R1's advisor command. *Expected:* 870 of 870; the advisor prints "No issues found" and exits 0; the two
image lists are equal (if not, the new image joins `images.lock` in T3, or the controller rules).

**T2. NO_PROXY (R5), the controller's.** Append to `engine/tests/dependency_boundary.rs`:
```rust
/// Loopback tests never go through a developer's proxy: ureq reads ALL_PROXY, HTTPS_PROXY and HTTP_PROXY, so
/// `.cargo/config.toml` forces NO_PROXY for every process cargo starts, over a developer's own (plan R5).
#[test]
fn every_cargo_test_process_bypasses_a_proxy_for_loopback() {
    let no_proxy = std::env::var("NO_PROXY").unwrap_or_default();
    let hosts: Vec<&str> = no_proxy.split(',').map(str::trim).collect();
    for host in ["127.0.0.1", "localhost"] {
        assert!(hosts.contains(&host), "NO_PROXY must name {host}, got `{no_proxy}`");
    }
    let line = r#"NO_PROXY = { value = "127.0.0.1,localhost", force = true }"#;
    let config = include_str!("../../.cargo/config.toml");
    assert!(config.lines().any(|l| l.trim() == line), "`.cargo/config.toml` must force NO_PROXY: {line}");
}
```
`cargo test -p knowlu-engine --test dependency_boundary -j 2` is red (on (ii), and on (i) unless the shell sets
`NO_PROXY`). Commit. Then append to `.cargo/config.toml`'s `[env]`, and commit:
```toml
# Loopback tests bind 127.0.0.1:0; ureq would otherwise send them through a developer's HTTP_PROXY or ALL_PROXY.
NO_PROXY = { value = "127.0.0.1,localhost", force = true }
```
**The manual proof (r1 M5).** With `$env:HTTP_PROXY = "http://127.0.0.1:9"`, run both of
`cargo test -p knowlu-engine --test mail_pass seen_records_mail_uids_and_the_ack_sends_them -j 2` (`mail_pass.rs:355`),
`cargo test -p knowlu-engine --test cloud_contract the_task_request_carries_the_note_and_its_grounding_and_nothing_else -j 2` (`cloud_contract.rs:128`).
Between the two commits both fail; after the config commit both pass, and still pass with `$env:NO_PROXY = "example.org"`
also set (the forced line wins). Then `Remove-Item Env:HTTP_PROXY, Env:NO_PROXY`. If the "before" runs pass, ureq
already bypasses loopback: the ledger says so, and the line stays as a defence.

**T3. The advisor step (R1).** *Red:* append `the_rls_job_runs_the_security_advisor_after_the_suite_and_fails_on_warn`.
In `job_block(&c, "cloud-rls")` it asserts that exactly one code line contains `db advisors` and, trimmed, equals R1's
`run:` line (so no `||`, `;` or `&&` follows the flags); that it comes after the `test db` line and before
`supabase-images.sh verify`; that its step's one `if:` equals pgTAP's; and that the job has no `continue-on-error`.
`cargo test -p knowlu-engine --test workflows -j 2` is red. *Green:* add the step with a two-line comment on what it
gates and why 0028/0029 are pgTAP's; `the_rls_job_is_shaped_to_be_required_runs_hosted_linux_and_holds_no_secret` stays
green, unedited. The comment lands in the job text the shape test searches (`workflows.rs:471-500, 524-529, 546-547`),
so it contains none of: `== 'true'`, `!= 'true'`, `runs-on:`, `CI_SELF_HOSTED`, `secrets.`, `SUPABASE_ACCESS_TOKEN`,
`supabase link`, `db push`, `--project-ref`, `--linked`, `functions deploy`, `SMTP_PASSWORD`, `github.token`,
`GITHUB_TOKEN`; a line starting `permissions:` or `env:`; `scripts/ci/` without `bash ` before it, or `scripts\ci`;
`supabase-images.sh list` or `supabase-images.sh verify` (`:547` finds the verify step by that text); and, for the new
pin, `db advisors` and `continue-on-error`. *Planted edits*, each seen red then reverted: `--fail-on error`, a trailing
`|| true`, `continue-on-error: true`, the step moved above `pgTAP`. The image question is settled at the green run.

**T4. The records (R6).** The README shows the local command and explains `SUPABASE_DB_PORT` as a per-machine override.
**T5. The review.** `reviewer` reads the whole branch against this plan and §6, and must PASS before the close.

**The controller's close (r1 M8).** **C1:** `cargo build --workspace` and `cargo test --workspace`, dev profile, no
warning but the accepted `.rsrc` line. **C2, staging:** `db push`; the advisor (F-A unchanged); the three read-only
queries below return no rows; then, with a staging session, push a scratch note through the deployed `sync-push`,
confirm its `rev` is stamped and `sync_usage` bumped, and remove it. **C3:** Quinn's go. **C4, production:**
`db push --linked` from a worktree cut from this branch's head and linked to production (a main checkout would report
the remote up to date), then the advisor and the same queries, recorded in the security note. **C5:** push; the PR says
what §5 says and reports C2 and C4; the advisor and `verify` steps are green; merge on green CI and the reviewer's PASS;
commit messages record the fixed leftovers (security note, `:46-47`). **Unverified:** the queries are written, not run;
the controller runs them locally at the red run (they must return rows) and the green run (none) before C2.
```sql
-- C10's equal: client table rights outside the four verbs
select n.nspname || '.' || c.relname || ':' || r.role || ':' || p.priv
  from pg_class c join pg_namespace n on n.oid = c.relnamespace
 cross join (values ('anon'), ('authenticated')) r(role)
 cross join (values ('truncate'), ('references'), ('trigger'), ('maintain')) p(priv)
 where n.nspname = 'public' and c.relkind in ('r', 'p', 'v', 'm', 'f') and has_table_privilege(r.role::name, c.oid, p.priv);
-- C11's equal: postgres's table defaults, global ('-') and in public
select d.defaclnamespace::regnamespace::text || ':' || coalesce(nullif(a.grantee::regrole::text, '-'), 'PUBLIC')
       || ':' || a.privilege_type
  from pg_default_acl d cross join lateral aclexplode(d.defaclacl) a
 where d.defaclrole = 'postgres'::regrole and d.defaclobjtype = 'r' and d.defaclnamespace in (0, 'public'::regnamespace)
   and (a.grantee = 0 or a.grantee in ('anon'::regrole, 'authenticated'::regrole))
   and a.privilege_type not in ('SELECT', 'INSERT', 'UPDATE', 'DELETE');
-- F2's equal: functions in public or admin a client role may execute (the RPC allowlist is empty)
select n.nspname || '.' || p.proname || '(' || pg_get_function_identity_arguments(p.oid) || '):' || r.role
  from pg_proc p join pg_namespace n on n.oid = p.pronamespace
 cross join (values ('anon'), ('authenticated')) r(role)
 where n.nspname in ('public', 'admin') and has_function_privilege(r.role::name, p.oid, 'execute')
   and not exists (select 1 from pg_depend d where d.classid = 'pg_proc'::regclass and d.objid = p.oid and d.deptype = 'e');
```

## 4. What must not break

- **C6's four verbs:** C6 (`01:119-130`) and its 09 plants (`09:560-581`) stay green unedited.
- **The masking-trigger machinery in 02, 03 and 09** (`tests.unmasked`, from `09:771`): the usage triggers still fire
  under a client role and RLS refuses their `sync_usage` upsert (-2); were EXECUTE checked at fire time these cases would
  read -1, so they detect F-C's last fact.
- **`sync_usage_bump` and C9's `md5(prosrc)` pin** (`01:167-189`): the migration creates and replaces no function (T1a
  asserts it), and C2's live check proves the triggers fire in the deployed service.
- **The rest:** 858 tests green (870 after); the 51 and 10 corpus pins hold; the shape pin needs no edit; the eval gate
  stays asleep (asserted with `/\bmodels\b/i`); `[env]` never reaches a shipped binary.

## 5. What a student sees

Nothing. No client issues TRUNCATE, REFERENCES, TRIGGER or MAINTAIN or calls a trigger function, and the triggers keep
stamping `rev` and counting usage. The PR says no student can see the change; C2's check through `sync-push` is the live
run, and it proves sync still stamps and counts.

## 6. Fidelity ledger

Each line, once closed, names the run its red and green came from: the controller's pgTAP run, the Deno counts or cargo.

| Approved item | Ruling | Task | What closes it |
|---|---|---|---|
| (1) The advisor's lints in CI | R1 | T3, C5 | the pin red, green, then red on four planted edits; equal image lists at the green run; the PR's advisor and `verify` steps green |
| (1) Production's first advisor read | R6 | T4, C4 | F-A in the security note, dated 2026-11-11; C4's re-read |
| (2a) Table rights and their defaults | R2, R3 | T1a, T1b | C10 and C11 red on main, then green; their 09 plants, one global; the migration's test; the queries at C2 and C4 |
| (2a) EXECUTE on the trigger functions | R2, R3 | T1a, T1b | F2 without its exemption, red then green; three F2 plants; C2's live sync check |
| (2b) Loopback tests and a proxy | R5 | T2 | the `dependency_boundary.rs` test red then green; both loopback tests before and after, and again with `NO_PROXY=example.org` |
| (2c) A later re-grant | R4 (c), (d) | T1a, T1b | cases 6 and 8, red then green |
| (2c) PUBLIC not required | R4 (a) | T1a, T1b | cases 1 and 2; `leaky_trigger` still throws |
| (2c) Overloads ignored | R4 (b), (c) | T1a, T1b | cases 3 to 5 (signatures) and 7 (revoke forms) |
| (2c) The real corpus under the stricter rule | R4 | T1b | the Deno line green with its pins unchanged (F-I), or a private finding |
| (3) Secret-scanning alert #1 | none | none | recorded in scope; resolved by the controller |
| Production only on Quinn's word | R7 | C3, C4 | Quinn's go, recorded before `db push --linked` |

## 7. Out of scope

- The performance advisor's 10 × `auth_rls_initplan`: performance, not security; C3 pins `qual` as `(<key> = auth.uid())`.
- `auth_leaked_password_protection`: an Auth setting, not SQL; R6 records it as left to HANDOFF's Pilot row.
- Deno's loopback tests (`deno test --allow-net=127.0.0.1`, e.g. `judge_anthropic_test.ts`; r1 M10) also honour
  `HTTP_PROXY`, and cargo's `[env]` never reaches them.
- Sequences; `service_role`; the `admin` schema (no client USAGE); `supabase_admin`'s default ACL; function default ACLs
  (F2 guards new functions). Splinter's 0028/0029 as SQL (F1 and F2 cover them); `assertExecuteRevoked`'s gaps (§8 O5);
  the Gmail-label live proof; making `cloud-rls` required; the RLS lane's M5 and M6; Q5's staging run.

## 8. Open for the controller (each with a recommendation)

O1–O3 and O7 of r0 were accepted at review r1 and are closed.
- **O4.** R4 (d) closes a fourth, same-file form of the re-grant gap. *Recommended:* keep it; (c) needs the ordering,
  and the corpus passes it (F-I).
- **O5 (F-J).** *Recommended:* carry `assertExecuteRevoked`'s gaps to a later lane; F2 reads every function's real
  rights, and T1b's helpers make the change one call.
  *Carried with it (branch review m1):* the definer guard still reads `to group anon` as no grantee, misses a re-grant
  whose list also names a function `\w+` cannot read, drops schema qualifiers (T1b's ruling), assumes `public` for an
  unqualified definition, and folds `"F"` into `f`. No migration uses these forms, and F1 and F2 read real rights.
- **O6.** This plan's additions, each to strike if unwanted: PUBLIC in C11, R5's `force = true` check, the no-`models`
  assertion, R4 (d), the queries' local runs before C2, and the `toolchain.md` line.
- **O8 (new, r1 M2).** If the red run's C11 names a global (`defaclnamespace = 0`) `postgres` table default granting a
  client role one of the four rights, R2's per-schema revoke cannot remove it. *Recommended:* R2 gains a fifth
  statement, the same `alter default privileges for role postgres revoke … on tables from anon, authenticated` without
  `in schema public`, and the migration test expects five. F-C names only the schema entry, so the red run decides.

## 9. Fidelity notes

**r1 → r2** (plan review r1, 2026-11-11, FAIL: 0 Critical, 3 Important, 10 Minor). Every finding is folded in under the
controller's rulings; each changed ruling now reads ruling, why, cost if wrong.

| Finding | Answer | Where |
|---|---|---|
| I1, argument lists | `FUNCTION_DEFINITION` unchanged; a separate balanced scan; the six-step normaliser, failing closed; cases cover `text[]`, `numeric(10,2)`, `default now()`, a parameter after a parenthesised default, and a revoke list with names | F-H, R4 (b), T1a cases 3–5, T1b |
| I2, grant and revoke forms | the qualifying revoke and the re-grant are defined by statement, with one red case per form | R4 (c), T1a cases 6 and 7 |
| I3, the WARN's reason | recorded only as an Auth setting and left to HANDOFF's Pilot row; no other reason | F-A, R6, §7 |
| M1, red predictions | 09 `:249` and `:251`; four F-G tests, not five; which cases already pass | T1a's expected red |
| M2, C10 and C11 | `nspname.relname`; the global default read, with a global plant | R3, T1a, the close's queries, §8 O8 |
| M3, case | `/\bmodels\b/i` | T1a's corpus tests, §4 |
| M4, the pin | the exact `run:` line, nothing after the flags; the comment's forbidden strings listed; `\|\| true` planted | R1, T3 |
| M5, the manual proof | both tests, before and after; one more run with `NO_PROXY=example.org` | R5, T2 |
| M6, T2's agent | the controller's, with the test and the config line written out | T2 |
| M7, one git index | tasks in sequence; `git commit -- <paths>` | R7, Gates |
| M8, close details | C4 runs from a worktree cut from this branch's head; the queries are inline | the close |
| M9, the image question | `docker images` before and after the advisor, at the green run | T1b's green run |
| M10, Deno loopback | out of scope; the records claim the fix for cargo only | §7, R6 |

**After plan review r2 (FAIL 0C/2I/2M, the last round), 2026-11-11.** The controller applied the four fixes the report
writes out, with no third round: Ruling: N1, N2, M-a and M-b are folded into R4 (b), R4 (c) and T1a's cases 5 and 6 as
written in `docs/reports/2026-11-11-security-leftovers-plan-review-r2.md` — each is one clause, no design choice, and the
whole diff is reviewed before any push — cost if wrong: T1b's red-first cases and the corpus run show it at once.
