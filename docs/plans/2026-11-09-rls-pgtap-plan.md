# Row-level security isolation tests with pgTAP: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: r0, §4 answered, awaiting plan review r1.** No task starts before the review passes (§10).

**Date:** 2026-11-09. **Lane:** security, RLS behaviour tests. **Branch:** `security/rls-pgtap`, worktree
`../knowlu-rls`, cut from `origin/main` at `c61c7cec` (it carries `20261109000100_definer_trigger_execute.sql`).

**Why.** The October ship-readiness report rates row-level security as the only authorization boundary
between one student's grades, synced notes and email-derived rows and another's, and notes that the
Supabase Security Advisor reads catalogs, not intent: "RLS enabled, no policy" and "a policy exists" are
both green there whether or not the policy says what we mean. Today's staging advisor reports RLS on every
public table, with a policy on the ten owner-readable ones and none on the rest. Nothing executes a query
as one student and checks it cannot see another's row. `migrations_test.ts`, `migrations_sync_test.ts`
and `migrations/migrations_test.ts` are static Deno tests over the SQL text; they stay as they are, and
this lane adds the behavioural layer under them.

**Goal.** A pgTAP suite, run in CI against a local Supabase database built from `cloud/supabase/migrations/`,
that proves for every relation in `public` (and the closed `admin` schema):
1. `anon` reads nothing and writes nothing;
2. on each table with an owner-read policy, student A reads exactly A's rows, never B's, and cannot
   insert, update, delete, upsert, or move a row to B's `account_id`, whether the row is A's or B's;
3. on each table with no policy, an authenticated student reads and writes nothing, A's own rows included;
4. no SECURITY DEFINER function, and no other non-trigger function, is executable by `anon` or
   `authenticated`;
5. no storage bucket, storage policy or realtime publication exists that the matrix does not name;
6. a relation added by a later migration and missing from the matrix fails the suite, in the pgTAP job
   and, statically, in the required `cloud` job.

**Architecture.**
- `cloud/supabase/tests/`: the pgTAP files (`NN_<name>_test.sql`) and three psql includes
  (`_setup.psql`, `_matrix.psql`, `_fixtures.psql`) that pg_prove does not run as tests.
- `cloud/supabase/rls_matrix_test.ts`: the static twin of the coverage guard, in Deno, run by the
  existing `cloud` job.
- `.github/workflows/ci.yml`: one new job, `cloud-rls`, shaped like the three required jobs.
- `engine/tests/workflows.rs`: one new test pinning that job's shape.
- `scripts/ci/supabase-images.sh` and `cloud/supabase/tests/images.lock`: pull the database images by
  digest and verify nothing unpinned ran.
- **No migration.** No schema, policy or grant changes; nothing is pushed to staging or prod. The
  `sync_contract` latest-path-check test and the eval gate are untouched (no file names `models` in a
  migration; no migration at all). No privacy-page sentence: no new data class or storage.

**Code read for this plan** (at `c61c7cec`): all 41 migrations; `cloud/supabase/{README.md, config.toml,
deno.json, migrations_test.ts, migrations_sync_test.ts}`; `migrations/migrations_test.ts` (the definer,
view and corpus guards); `proofs/testers_staging.sql` and `proofs/comp_guards_staging.sql` (the
sign-up insert and `set local role` savepoint pattern); `.github/workflows/{ci,security}.yml`;
`engine/tests/workflows.rs`; cloud design §1, §5 and §8.

---

## 1. The relations, as the migrations leave them

Thirty-six tables in `public`: 37 names are created, `20260912000300` drops `sync_generation`, and the
same migration drops and recreates `sync_records` and `sync_notes`. There are six views in `public` and
three in `admin`. RLS is on for every table. Every policy is `for select to authenticated using (<key> = auth.uid())`; no table has
a client write policy (pinned statically by `migrations_test.ts`).

**Owner-read tables (10), policy `<table>_select_own`, client grants left at Supabase's defaults:**
`accounts` (key `id`), `entitlements`, `sources`, `billing_reminders`, `telemetry_events`, `corrections`,
`issues`, `sync_records`, `sync_notes`, `sync_usage` (key `account_id`).

**Service-only tables, no policy, client grants at the defaults (15):** `consents`, `webhook_events`,
`deleted_accounts`, `models`, `judgments`, `rules`, `rule_evidence`, `usage_daily`, `budget_alerts`,
`google_accounts`, `gmail_seen`, `gmail_queue`, `google_state`, `eval_cases`, `eval_runs`.

**Service-only tables, no policy, client grants revoked (11):** `mail_accounts`, `mail_addresses`,
`mail_batches`, `mail_canaries`, `mail_seen`, `mail_work`, `mail_claims`, `judge_allowances`,
`comp_grants`, `comp_audit`, `route_daily`.

**Views.** `public.billing_subscribers` is `security_invoker = true` with default grants: a client reads
it through the three tables' own policies. `public.telemetry_daily`, `correction_rates`, `monthly_spend`,
`sync_limits` and `effective_entitlements` run as their owner and are revoked from `anon` and
`authenticated`. `admin.testers`, `admin.tester_usage_daily` and `admin.tester_spend_monthly` sit in a
schema every API role has lost `usage` on.

**SECURITY DEFINER functions (7), each revoked from `public`, `anon` and `authenticated`:**
`handle_new_user`, `trim_user_metadata`, `store_google_grant`, `delete_google_grant`,
`read_google_grant`, `read_google_grant_any`, `delete_mail_address`. Every other function in `public`
and `admin` is SECURITY INVOKER and revoked from the client roles too, except the two trigger functions
`sync_notes_stamp_rev` and `sync_usage_bump`, which Postgres refuses to call outside a trigger.

**Storage and realtime.** No migration creates a bucket, a storage policy or a publication, and no
function calls Storage.

The staging advisor's figure for today was 35 tables, 26 without a policy. The migrations give 36 and 26
(26 + 10 = 36). Q2 asks the controller to settle the difference before T6 relies on the matrix.

---

## 2. Decisions

### D1. Where the tests live: `cloud/supabase/tests/`, run by `supabase test db`

- **Chosen.** The CLI's project directory here is `cloud/`, not the repo root. `config.toml` sits at
  `cloud/supabase/config.toml` and its own `content_path = "./supabase/templates/magic_link.html"`
  resolves only from `cloud/`. So every CLI call takes `--workdir cloud`, and `supabase test db` reads
  `cloud/supabase/tests/` by default. Tests sit beside the migrations they test. `deno test cloud/supabase/`
  ignores `.sql`, and `deno lint` ignores `.sql` and `.psql`.
- **Includes, not a committed helper schema.** pg_prove runs each file in its own psql session, and the CLI
  passes `--ext .pg --ext .sql`. The shared helpers are `.psql` files, so pg_prove never runs them as
  tests. Each test file pulls them in with psql's `\ir` (relative to the file) inside its own
  transaction, and the file ends in `rollback`. Nothing survives a run, no file depends on another file's
  order, and a developer's local database is left as it was.
  - Fallback if T4 finds `\ir` unusable under the pinned CLI: a `000_setup_test.sql` that creates a
    `tests` schema and commits. This is the basejump pattern, and it carries the ordering dependency
    named above. It is recorded as a deviation if used.
- **Rejected.** A hand-rolled `cloud/pgtap/` run by pg_prove on the runner loses the CLI's own pgTAP
  enable and disable and the path a developer would run locally. Putting the suite in `proofs/` mixes it
  with controller-run staging proofs, which have a different trust boundary.
- **Never from the hub or a vault.** Fixtures use `@example.invalid` addresses and random UUIDs only.

### D2. How CI runs them: a new `cloud-rls` job, the CLI's own database, everything pinned

**Chosen: `supabase --workdir cloud db start`, then `supabase --workdir cloud test db`, in a new job on
`ubuntu-24.04`.**

| | CLI database (`db start`) | `postgres` service container + psql | full `supabase start` |
|---|---|---|---|
| `auth.uid()`, `anon`/`authenticated`/`service_role`, `supabase_auth_admin` | the platform's own | absent; we would have to reimplement them | the platform's own |
| Supabase's default grants to `anon`/`authenticated` | present: the thing under test | absent, so every deny test passes vacuously | present |
| `pg_cron`, `pg_net`, `supabase_vault` (our migrations need all three) | present | absent: the migrations fail | present |
| Migration order and owner (`postgres`, as on staging) | the CLI's | ours to reimplement | the CLI's |
| Startup (estimate; T4 measures) | about 2 to 4 min, mostly the image pull | about 30 s | 4 min or more |

- **Why not the faster option.** A plain Postgres container is faster, but it is the wrong database. It
  has no `auth` schema, no client roles, and none of the default privileges that make a missing policy
  dangerous, so a suite run there would be green for the wrong reason.
- **Why not `supabase/postgres` as a service container.** It has the roles, but we would have to rebuild
  the CLI's schema init and migration runner. That is a second copy that can drift from the one Quinn uses
  on staging.
- **Why not full `supabase start`.** It adds services the suite never calls (API gateway, GoTrue, Storage
  API, Realtime, Studio, the edge runtime).
- **Fallback.** If `db start` turns out not to apply migrations under the pinned CLI, use
  `supabase --workdir cloud db reset` after it. If `db start` is missing, use `supabase start -x <every
  service but the database>`. T4 records which one ran. In every case `01_coverage_test.sql` fails loud
  on an empty schema, so a database that never got its migrations cannot pass.

**Pinning.**
- **The CLI** is downloaded the way `security.yml` downloads actionlint and gitleaks:
  `curl` → `sha256sum -c` → run.
  - The download is the GitHub release tarball, `VERSION` and `SHA256` in the job's `env`.
  - It is no third-party action.
  - The version is the current stable release on T0's day, recorded in the ledger.
- **The images** are pinned through the CLI version, which fixes their tags, and then by digest.
  - `cloud/supabase/tests/images.lock` holds one `repo:tag@sha256:<digest>` line per image the two
    commands use.
  - `scripts/ci/supabase-images.sh pull` pulls each image by digest and tags it, so the CLI finds it
    locally and pulls nothing.
  - `… verify` runs after the suite and fails if any other image is on the runner, or if a tag points at
    a different digest.
  - So a CLI bump that changes an image fails until `images.lock` changes with it, in the same diff.
- **The runner.** `ubuntu-24.04`, not `ubuntu-latest`.
  - The job does **not** read `CI_SELF_HOSTED`: it needs Linux containers, and the self-hosted runner is
    Windows.
  - The `ci.yml` header comment that says every job takes the switch gains that one named exception.

**No staging, no secrets, no link.**
- The job never runs `supabase link`, `db push`, `--project-ref`, `--linked` or `functions`, and never
  names `SUPABASE_ACCESS_TOKEN` or any `secrets.`. It is a local database on a disposable runner.
- The cron jobs the migrations schedule do run there:
  - `knowlu-mail-process` fires every minute.
  - Every HTTP job raises at its Vault guard before `net.http_post`, because no Vault secret exists
    locally.
  - So no request leaves the runner.
- The only network the job uses is setup: the CLI tarball from GitHub and the images from the CLI's
  registry. The tests themselves make no network calls.
- Fork PRs run it safely: there is nothing to withhold.

**The docs-only rule.** `cloud-rls` takes exactly the shape `workflows.rs` pins for the three required
jobs:
- `needs: changes`;
- one job-level `if: ${{ !cancelled() }}`;
- a first no-op step on `needs.changes.outputs.code == 'false'`;
- every other step on `!= 'false'`.

So a docs-only PR still gets a green `cloud-rls` in seconds, and a failed `changes` job runs the suite
rather than skipping it.
- **Required?** Not at merge. Recommendation (Q1): Quinn adds `cloud-rls` to branch protection after ten
  consecutive green runs on `main`. The shape is pinned from day one (T3's `workflows.rs` test), so
  making it required can never deadlock a docs-only PR.
- **Filter.** It runs on every non-document change rather than only on `cloud/supabase/**`. Minutes are
  free while the repository is public, and one rule leaves fewer ways to fail open. If the repository
  goes private, a path filter inside the job's steps (never on the trigger) is the follow-up.

### D3. The test matrix

One row per relation, in `_matrix.psql`, inserted into a temporary `tests.matrix`:

| column | values |
|---|---|
| `rel` | `public.accounts`, …, `admin.testers` |
| `kind` | `table`, `view` |
| `access` | `own` (owner-read policy), `service` (no policy), `invoker_view`, `closed_view` |
| `key_col` | `id`, `account_id`, or null for a global table (`models`, `webhook_events`, `deleted_accounts`, `rule_evidence`, `eval_cases`, `eval_runs`) and for views; `mail_batches` and `mail_canaries` are keyed through `address_id` |
| `touch_col` | a non-generated column an update can assign to itself |
| `grants` | `default` (client roles hold Supabase's default table rights) or `revoked` |
| `policies` | the exact policy names, `{}` for none |

**The cases.**
- "Denied" means exactly one of two outcomes, and the matrix says which:
  - `grants = revoked`: SQLSTATE `42501`;
  - `grants = default`: zero rows (on read, update and delete) or `42501` (on insert, from RLS).
- Any other error is a failure, never a pass: every probe re-raises what it does not expect.
- Every assertion runs as the owner. Only the probe runs as the client role, so pgTAP itself never runs
  under `anon`.

| # | As | Relations | Probe | Expect |
|---|---|---|---|---|
| A1 | `anon` | every table | `select count(*)` | 0, or 42501 when revoked |
| A2 | `anon` | every table | insert a valid row (for A) | 42501 |
| A3 | `anon` | every table | update `touch_col` on every row | 0, or 42501 |
| A4 | `anon` | every table | delete every row | 0, or 42501 |
| A5 | `anon` | every view | `select count(*)` | 0 for `invoker_view`, 42501 for `closed_view` |
| O1 | A | `own` | `select` | exactly A's fixture rows, every one keyed to A |
| O2 | A | `own` | `select … where <key> = B` | 0 |
| O3, O4 | A | `own` | insert a row for A; for B | 42501 |
| O5 | A | `own` | upsert onto B's existing primary key (`on conflict … do update`) | 42501 |
| O6, O7 | A | `own` | update A's rows; B's rows (`touch_col`) | 0 |
| O8 | A | `own` | `update … set <key> = B where <key> = A` (the move) | 0; the owner then sees A's and B's rows unchanged |
| O9, O10 | A | `own` | delete A's rows; B's rows | 0; the owner then sees both still there |
| O11 | B | `own` | `select` | exactly B's rows (the mirror of O1) |
| O12 | `authenticated`, no `sub` claim | `own` | `select` | 0 |
| O13 | `authenticated`, a `sub` with no account | `own` | `select` | 0 |
| S1 | A | `service` | `select`, all rows and `where <key> = A` | 0, or 42501 |
| S2 | A | `service` | insert a row for A | 42501 |
| S3, S4 | A | `service` | update A's rows; move A's rows to B | 0, or 42501 |
| S5 | A | `service` | delete A's rows | 0, or 42501 |
| V1 | A | `billing_subscribers` | `select` | A's row only (A and B both hold an active subscription in the fixture) |
| V2 | A | `closed_view` | `select` | 42501 |
| V3 | `anon`, A | every view | insert | an error: 42501, or 0A000/55000 for a view that cannot be updated; never success |
| F1 | catalog | `public`, `admin` | SECURITY DEFINER functions executable by `anon` or `authenticated` | none |
| F2 | catalog | `public`, `admin` | any non-trigger function executable by `anon` or `authenticated`, minus the matrix's RPC allowlist (empty today) | none |
| F3 | `anon`, A | the 7 definer functions | call with typed nulls | 42501 |
| F4 | A | `read_google_grant(B, …)` | call naming B's account | 42501 |
| F5 | catalog | schema `admin` | `usage` for `anon`, `authenticated`, `service_role` | false |
| P1 | catalog | `storage.buckets` (if the schema exists) | count | 0 |
| P2 | catalog | `pg_policies` in `storage` | names | the platform's own set as T4 records it (expected empty) |
| P3 | catalog | `pg_publication_tables` | rows for `public` or `admin` | none |
| C1 | catalog | relations in `public` and `admin` (`r`, `p`, `v`, `m`, `f`) | against the matrix | equal both ways; the message names each stray |
| C2 | catalog | every matrix table | `relrowsecurity` | true |
| C3 | catalog | `own` tables | `policies_are`; command `select`; roles `{authenticated}`; permissive; `qual` exactly `(<key> = auth.uid())`; no `with_check` | as the matrix |
| C4 | catalog | `service` tables | `policies_are(…, '{}')` | no policy |
| C5 | catalog | views | `security_invoker` option against `access`; `closed_view` has no client select | as the matrix |
| C6 | catalog | every table | `has_table_privilege` for the client roles, four verbs | matches `grants` |
| C7 | owner | fixtures | each keyed table has an A row and a B row; each global table has a row | non-vacuous |
| C8 | helpers | `auth.uid()` | A after `as_user(A)`; null after `as_anon()` and `as_owner()`; `current_user` follows | fidelity |

The matrix covers what a client can ask PostgREST for: select, insert, update, delete, upsert and RPC.
Privileges PostgREST cannot exercise (`truncate`, `references`, `trigger`, sequence `usage`) are out of
scope (§12).

### D4. Coverage cannot silently lag: two guards

- **C1, in pgTAP (`01_coverage_test.sql`).** `tests.unlisted()` returns every relation in `public` or
  `admin` with no matrix row, and every matrix row with no relation. The test asserts it is empty and
  prints each name.
  - Extension-owned relations (`pg_depend.deptype = 'e'`) are excluded.
  - A migration that adds a table, a view or an `admin` relation turns `cloud-rls` red until the row is
    written, and C3 and C4 then force the row to state the table's policies exactly.
- **The static twin (`cloud/supabase/rls_matrix_test.ts`), in the required `cloud` job,** so the guard
  holds even before `cloud-rls` is required, and on any machine without Docker.
  - It derives the live relation set from the migration text: `create table` in either spelling, since
    the C2 files create unqualified names, minus `drop table`, plus views in `public` and `admin`.
  - It parses `_matrix.psql`'s rows with one strict regex and a parse-count check, the pattern
    `migrations/migrations_test.ts` uses, so a row it cannot read fails loud.
  - It asserts:
    - the two relation sets are equal;
    - every table that a live `create policy` names is `own`, with those policy names;
    - every `service` row names no policy;
    - `create schema` in any migration names only `admin`.
  - It reads files only. It needs no new `deno test` permission.

### D5. Impersonation, as PostgREST does it

`auth.uid()` on the Supabase image reads `request.jwt.claim.sub` and falls back to
`request.jwt.claims ->> 'sub'`. PostgREST sets `request.jwt.claims` and then `set local role`, both scoped
to the transaction. The helpers do the same, and they clear the legacy key so a stale value can never
shadow the claims. The staging proofs already showed the owner may `set local role` to each API role
(`proofs/testers_staging.sql`, A7, A3s and R1). Sketch, in `_setup.psql`; the names are suggestions, the
behaviour is not:

```sql
create schema tests;
grant usage on schema tests to anon, authenticated;

create function tests.as_user(p_sub uuid) returns void language plpgsql as $$
begin
  perform set_config('request.jwt.claim.sub', '', true);
  perform set_config('request.jwt.claims', json_build_object(
    'sub', p_sub, 'role', 'authenticated', 'aud', 'authenticated')::text, true);
  execute 'set local role authenticated';
end $$;
-- tests.as_anon(): claims {"role": "anon"}, set local role anon.
-- tests.as_claims(jsonb): authenticated with exactly these claims (O12, O13).
-- tests.as_owner(): reset role; clear both settings.

-- Probes run as the client role and never assert. 42501 becomes -1; anything else propagates.
create function tests.visible(p_rel regclass, p_where text default 'true') returns bigint
language plpgsql as $$
declare n bigint;
begin
  execute format('select count(*) from %s where %s', p_rel, p_where) into n;
  return n;
exception when insufficient_privilege then return -1;
end $$;
-- tests.affected(sql): execute; return row_count, or -1 on 42501.
-- tests.insert_sql(rel, account): a valid insert for that table and account (one case per table).
-- tests.unlisted(): D4.
grant execute on all functions in schema tests to anon, authenticated;
```

Every test file has the same frame. The includes are psql meta-commands, so they run inside the file's
own transaction:

```sql
begin;
\ir _setup.psql
\ir _matrix.psql
\ir _fixtures.psql
select plan(<computed from tests.matrix>);
-- assertions
select * from finish();
rollback;
```

**The fixtures (`_fixtures.psql`).**
- **Accounts.** It makes two accounts the way sign-up makes them: an `auth.users` insert alone, with the
  column list `proofs/testers_staging.sql` uses. `on_auth_user_metadata` and `handle_new_user` write the
  `accounts` and `entitlements` rows.
- **Rows.** As the owner, it then writes one valid row per table for A and one for B. Global tables get
  one row each. `models` uses its seeded rows. Chains follow their foreign keys
  (`judgments` → `rules` → `rule_evidence`; `mail_addresses` → its six dependants).
- **Validity, proven before anything is denied.** Every insert that A2, O3, O4 and S2 expect to be
  refused is the same `tests.insert_sql` text. The owner runs it first, in a subtransaction that is then
  rolled back (C7). A 42501 can then only mean the client role was refused, never a bad row.
- **No real data.** No real address, name, course or note text: `rls-a@example.invalid`, a
  `tasks/rls-fixture.md` path, `{}` bodies where a body is required.

`plan()` is computed from `tests.matrix`. A row added or dropped moves the count with it, so no case is
skipped silently.

### D6. Tests first, in a lane where the schema is already right

- A deny test against a correct schema is green on its first run, so "red first" cannot mean "fails until
  the code is written". Here it means two things.
  - Every probe is first shown to fail against a planted fault, in `09_self_test_test.sql`, one
    transaction, rolled back:
    - a `using (true)` select policy on `issues` must make O2's probe see B's row;
    - a `with check (true)` insert policy must make O3's probe succeed;
    - a `grant execute` of `read_google_grant` to `anon` must appear in F1's list;
    - an unlisted `public.zz_unlisted` table must appear in `tests.unlisted()`;
    - a `grant select` on `monthly_spend` to `anon` must turn A5's 42501 into a count.
  - The static twin is written before `_matrix.psql` exists, and fails for that reason.
- **Who runs what.**
  - Agents never run the `supabase` CLI. That includes local `db start`.
  - pgTAP runs happen in `cloud-rls` on the lane's draft PR, which the controller pushes. The controller
    may also run `supabase --workdir cloud db start` and `test db` on this machine when it allows Docker.
  - Agents run `deno test`, `deno check`, `deno lint`, and `cargo test -p knowlu-engine --test
    workflows -j 2` in a granted build slot.
  - Each task's ledger line names the CI run (or local run) its red and green came from.

---

## 3. Global constraints (every task)

- **Agent.** `cloud-engineer`, Opus `high`.
  - A silent false green here hides a cross-account read of grades or email-derived rows.
  - Nothing is on the contract list. `workflows.rs` is a CI pin, not an oracle, sync or entitlement test.
  - `reviewer` reads the whole branch before it leaves draft (T11).
- **The `supabase` CLI is not an agent's tool.**
  - No agent runs the CLI, locally or against a project, and no agent uses the Supabase MCP.
  - pgTAP runs in CI or on the controller's machine.
  - Staging and prod are never touched by this lane. Q2's staging count is the controller's read-only
    query.
- **No migration, no schema change.**
  - The lane adds no file under `cloud/supabase/migrations/` and edits none of the three static
    migration tests.
  - If a test finds that a policy, a grant or a function's rights are wrong, the task **stops**:
    - the agent reports the table and the scenario to the controller only;
    - it goes in no commit message, PR text, plan or ledger line, because the repository is public;
    - the fix is a separate lane with its own migration, and this lane resumes after it.
  - A probe bug is fixed in the probe, and the ledger says which it was.
- **Privacy.** Fixtures carry only `@example.invalid` addresses, random UUIDs and placeholder text.
  - No test, `diag()` or CI log prints a token, a key, an account id or any student content.
  - A diagnostic names relations, roles and counts only.
  - No privacy-page sentence: no new data class and no new storage. `PRIVACY_VERSION` does not move.
- **Gates.**
  - Every task: CI's full Deno line from the repo root (`ci.yml` `deno test` step), plus `deno check`
    and `deno lint` as `ci.yml` runs them.
    - T0 records the baseline pass count.
    - Every later task records its own pass and fail counts.
    - Zero lint problems and zero check errors are part of green.
  - Tasks that touch `ci.yml` or `workflows.rs`: `cargo test -p knowlu-engine --test workflows -j 2` in a
    granted slot. The workspace suite runs once, at T11.
  - pgTAP tasks: the `cloud-rls` run on the draft PR is the gate, and the ledger line names its run id.
- **Commits.** Only the task's own files, named one by one; never `git add -A`.
  - LF everywhere, `.sh` included; `scripts/ci/eol-check.ps1` must stay green.
  - No commit, push or PR action without the controller's go: the controller pushes, opens the draft
    PR, and later marks it ready.

## 4. Questions for Quinn (each with a recommendation)

**Answered 2026-11-09 (controller, under Quinn's go for this security pass; Q1's branch-protection change
still asks Quinn before it is made).** Q1: yes, required after ten consecutive green runs on `main`. Q2: 36;
staging's table list has 36 rows (the 35 was a miscount). Q3: the stronger rule. Q4: digest. Q5: yes,
once, after merge.

- **Q1. Is `cloud-rls` a required check?** Recommended: yes, after ten consecutive green runs on `main`.
  Quinn adds it to branch protection. Until then it reports but does not block. T3 pins the docs-only
  shape now, so turning it on later is one setting.
- **Q2. 35 or 36 tables?** Today's advisor says 35; the migrations leave 36 (26 without a policy plus 10
  with one). Recommended: the controller runs a read-only `count(*)` over `pg_tables where schemaname =
  'public'` on staging, before T6.
  - If staging really has 35, the missing table is a migration staging never applied, and the matrix
    stays at 36 (it follows the migrations).
  - If staging has 36, the advisor figure was a miscount and nothing changes.
- **Q3. Function rule: the dispatch's definer-only rule or a stronger one?** Recommended: the stronger
  rule (F2).
  - Under it, no non-trigger function in `public` or `admin` is client-executable unless the matrix's
    RPC allowlist names it. The allowlist is empty today; every function is already revoked.
  - It makes a future `-- rpc: authenticated by design` function a written decision, not a default
    grant.
- **Q4. Image pinning: digest or CLI version only?** Recommended: digest (D2).
  - The cost is one `images.lock` edit per CLI bump, verified in CI.
  - CLI-version-only pinning trusts the registry not to move a tag.
- **Q5. A staging proof from the same suite?** Recommended: once, after merge.
  - The controller runs the files' assertions on staging in one rolled-back transaction, as
    `proofs/testers_staging.sql` does, and never from CI.
  - It catches drift between the migrations and what staging actually applied (Q2's kind).
  - It would be a new file under `proofs/`, written in a follow-up and run only on the controller's
    word.

## 5. Conflict map

| File | Change | Other lanes |
|---|---|---|
| `.github/workflows/ci.yml` | one job appended; the header comment's one exception | any lane touching CI; rebase-merge only, never a second `cloud` job (R-X-7) |
| `engine/tests/workflows.rs` | one test appended; existing tests unedited | the security-tooling lane (PR #69, merged) |
| `cloud/supabase/README.md` | a pgTAP paragraph under "Testing" | cloud lanes adding README sections |
| everything else | new files only | none |

A cloud lane that adds a table while this lane is open turns the static twin red on whichever merges
second. That is the guard working: the second lane adds the matrix row.

## 6. Tasks at a glance

| Task | Who | What | Red | Green |
|---|---|---|---|---|
| T0 | main session | base, baselines, CLI version and SHA-256, Q2 | | ledger |
| T1 | cloud-engineer | static twin `rls_matrix_test.ts` | no matrix file | |
| T2 | cloud-engineer | `_matrix.psql`, every relation | | Deno line |
| T3 | cloud-engineer (cargo slot) | `cloud-rls` job, `_setup.psql` impersonation, `00_smoke_test.sql`, images script (`list`) | `workflows.rs` pin | pin + first CI run |
| T4 | controller, then cloud-engineer | read the first run; `images.lock`; `pull` and `verify` on | | second CI run |
| T5 | cloud-engineer | `09_self_test_test.sql`, then probes and `_fixtures.psql` | self-test vs no probes | self-test |
| T6 | cloud-engineer | `01_coverage_test.sql` (C1-C8), `02_anon_test.sql` (A1-A5, V3) | T5's planted faults | CI |
| T7 | cloud-engineer | `03_own_rows_test.sql` (O1-O13, V1) | T5's planted faults | CI |
| T8 | cloud-engineer | `04_service_only_test.sql` (S1-S5, V2) | T5's planted faults | CI |
| T9 | cloud-engineer | `05_functions_test.sql` (F1-F5), `06_platform_test.sql` (P1-P3) | T5's planted faults | CI |
| T10 | docs-keeper | README "Testing", HANDOFF line | | |
| T11 | reviewer, then main session | whole-branch review, workspace suite, ready for review | | PASS |

## T0. Base, baselines and pins (main session)

- [ ] Confirm the worktree's HEAD is `c61c7cec`, and record it in the lane ledger.
- [ ] Run CI's full Deno line from the repo root, and record the pass count (the baseline).
- [ ] Pick the current stable Supabase CLI release. Record:
  - its version;
  - the SHA-256 of `supabase_linux_amd64.tar.gz`, taken from the release's checksums file
    (`gh release download`, no execution).
- [ ] Ask the controller for Q2's staging count, and record the answer.
- [ ] Record Quinn's answers to Q1, Q3, Q4 and Q5, or the controller's interim ruling.

## T1. Red: the static twin (cloud-engineer)

Files: `cloud/supabase/rls_matrix_test.ts` (new).

- [ ] Write the test D4 describes, reading `./migrations/*.sql` and `./tests/_matrix.psql` relative to
  `import.meta.url`. Its parts:
  - **The relation set.** Strip `--` comments first, as `migrations/migrations_test.ts` does.
    - `create table [if not exists] [public.]<name>`: the C2 files create unqualified names, and
      `search_path` puts them in `public`.
    - minus `drop table [if exists] [public.]<name>` in a later file;
    - plus a recreate after a drop;
    - plus `create [or replace] [materialized] view [public.|admin.]<name>`.
  - **A parse-count check on each.** The number of `create table` statements matched by a loose regex
    equals the number parsed by the strict one, so a spelling the parser misses fails loud.
  - **The matrix rows.** One strict regex per row of `_matrix.psql`, plus a count check against the
    number of lines that begin `  ('`.
  - **The assertions**, each failure naming every stray:
    - the relation sets are equal both ways;
    - every live table that a `create policy` names is `own`, with exactly those names;
    - no `service` or view row lists a policy;
    - `create schema` names only `admin`.
  - **Its own small negative tests**, in the style of `migrations/migrations_test.ts`:
    - an unqualified `create table if not exists x` is found;
    - a dropped table is gone, and a recreated one is back;
    - a malformed matrix row trips the count check.
- [ ] Run the Deno line. **Expected red:** the new file fails, because `_matrix.psql` does not exist.
  Every other test is green. Record the counts.
- [ ] Commit `rls_matrix_test.ts` alone: `test(cloud): static RLS matrix twin, red until the matrix exists`.

## T2. The matrix (cloud-engineer)

Files: `cloud/supabase/tests/_matrix.psql` (new).

- [ ] Write the matrix: 45 rows, one per line, in §1's order, with no other statement on a row's line.
  - 36 tables;
  - 6 `public` views;
  - 3 `admin` views;
  - a `tests.rpc_allowlist` insert with no rows (Q3).
  - The rows are inserted into `tests.matrix`, which `_setup.psql` creates (T3), with the columns of D3.
  - `touch_col` is never a generated column (`sync_records.keep`), an identity column, or a column
    under a trigger that rewrites it (`sync_notes.rev`).
- [ ] Run the Deno line: T1's file is green and the count is the baseline plus T1's tests. Record it.
- [ ] Commit `_matrix.psql` alone: `test(cloud): the RLS matrix, every public and admin relation`.

## T3. The `cloud-rls` job, impersonation and the smoke file (cloud-engineer; cargo slot)

Files: `engine/tests/workflows.rs` (one test appended), `.github/workflows/ci.yml`,
`cloud/supabase/tests/{_setup.psql, 00_smoke_test.sql}`, `scripts/ci/supabase-images.sh` (new).

- [ ] **Red.** Append a test to `workflows.rs`, e.g.
  `the_rls_job_is_shaped_to_be_required_runs_hosted_linux_and_holds_no_secret`. It asserts that the
  `cloud-rls` block has:
  - `needs: changes`, and exactly one job-level `if`, which is `${{ !cancelled() }}`;
  - a `code == 'false'` no-op step, every other step condition on `code != 'false'`, never `== 'true'`
    or `!= 'true'`;
  - exactly one `runs-on: ubuntu-24.04`, and no `CI_SELF_HOSTED`;
  - none of `secrets.`, `SUPABASE_ACCESS_TOKEN`, `supabase link`, `db push`, `--project-ref`, `--linked`
    or `functions deploy`;
  - `--workdir cloud` on every `supabase` call;
  - `sha256sum -c` before the CLI's first use.

  Also, the existing `docs_only_changes_still_report_the_three_required_checks` stays as it is. Run
  `cargo test -p knowlu-engine --test workflows -j 2`. **Expected:** the new test fails with
  `cloud-rls: no such top-level job key`, and every other test is green. Record it.
- [ ] **The job.** Append it to `ci.yml`, after `eval-gate`. In outline:

```yaml
  cloud-rls:
    runs-on: ubuntu-24.04
    needs: changes
    if: ${{ !cancelled() }}
    timeout-minutes: 15
    env:
      SUPABASE_CLI_VERSION: <T0>
      SUPABASE_CLI_SHA256: <T0>
    steps:
      - name: no code changed, nothing to run (this check still reports)
        if: needs.changes.outputs.code == 'false'
        run: echo "only documents changed; skipping the heavy steps"
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1  # v7.0.1
        if: needs.changes.outputs.code != 'false'
        with: { persist-credentials: false }
      - name: the supabase CLI, checked before it runs
        if: needs.changes.outputs.code != 'false'
        run: |  # curl the tarball to $RUNNER_TEMP, sha256sum -c, untar, add to $GITHUB_PATH
      - name: a local database from the migrations (no project, no secrets)
        if: needs.changes.outputs.code != 'false'
        run: supabase --workdir cloud db start
      - name: pgTAP
        if: needs.changes.outputs.code != 'false'
        run: supabase --workdir cloud test db
      - name: images used
        if: always() && needs.changes.outputs.code != 'false'
        run: scripts/ci/supabase-images.sh list
```

  Notes on the job:
  - The header comment's sentence about the switch gains: "`cloud-rls` excepted: it needs Linux
    containers."
  - A comment above the job says what it is for, why it is not the `cloud` job (R-X-7), and that it holds
    no secret.
- [ ] **`_setup.psql`, the first half.** It holds:
  - `tests` schema;
  - `tests.matrix` and `tests.rpc_allowlist`;
  - `as_user`, `as_anon`, `as_claims`, `as_owner` (D5);
  - `set search_path = public, extensions`, if T4 finds that pgTAP is not otherwise on the path.
- [ ] **`00_smoke_test.sql`.** The frame of D5 (it includes `_setup.psql` and `_matrix.psql` only).
  `plan(6)`:
  - `has_table('public', 'accounts', …)`: the migrations were applied;
  - C8's three `auth.uid()` checks;
  - `current_user` is `authenticated` after `as_user`;
  - `current_user` is the session user after `as_owner`.

  It also prints a `diag` of `version()` and `session_user`, and nothing else.
- [ ] **`supabase-images.sh`.** `list` prints `docker image ls --digests` for every image on the
  runner. `pull` and `verify` exist but are not yet called (T4).
- [ ] Run the workflows test and the Deno line: both green. Record them.
- [ ] Commit the five files, named one by one:
  `ci(cloud): cloud-rls job runs pgTAP on a local Supabase database`.
- [ ] **Controller:** push the branch, and open a **draft** PR so `pull_request` runs CI.

## T4. Read the first run; pin the images (controller, then cloud-engineer)

- [ ] **Controller:** from the first `cloud-rls` run, record in the ledger:
  - whether `db start` applied the migrations (the smoke file's `has_table`), or which D2 fallback is
    needed;
  - that the `.psql` includes ran (smoke green) and pg_prove did not pick them up as tests;
  - `session_user`;
  - the wall-clock time of each step;
  - the `list` output (each image and its digest);
  - whether `storage` exists, and its policy names (P2's expected set).
- [ ] **cloud-engineer:** write `cloud/supabase/tests/images.lock` from that list. Then:
  - add the `pull` step before `db start`, and `verify` in place of `list`;
  - extend the workflows test to require both steps, in that order;
  - apply any fallback the controller recorded (D1 or D2), as a named deviation in the ledger;
  - run the workflows test, then commit:
    `ci(cloud): pull the database images by digest and verify nothing else ran`.
- [ ] **Controller:** push. The second run must be green, with `verify` passing. If the job takes over
  6 minutes, raise it at checkpoint 3 before T5.

## T5. The probes, proven able to fail (cloud-engineer)

Files: `cloud/supabase/tests/09_self_test_test.sql` (new), then `_setup.psql` (second half) and
`_fixtures.psql` (new).

- [ ] **Red.** Write `09_self_test_test.sql` first: the five planted faults of D6, each in the file's one
  transaction.
  - Each fault is planted as the owner, then probed as the client role.
  - Then comes the assertion that the probe **saw** the fault:
    - O2's probe on `issues` returns a count above 0;
    - O3's insert returns `ok` rather than 42501;
    - F1's list contains `read_google_grant(uuid, text)`;
    - `tests.unlisted()` contains `public.zz_unlisted`;
    - A5's probe on `monthly_spend` returns a count rather than -1.
  - It also checks two guard rails:
    - a probe given a broken statement (an unknown column) raises, and does not return 0 or -1;
    - C7's validity check reports false for a row that violates a check constraint.
  - **Controller:** push. **Expected red:** the file fails because `tests.visible`, `tests.affected`,
    `tests.insert_sql` and `tests.unlisted` do not exist yet. Record the run.
- [ ] **Green.** Write the probes into `_setup.psql`, and write `_fixtures.psql` (D5).
  - The probes are `visible`, `affected`, `insert_sql`, `insert_is_valid`, `unlisted`, and F1/F2's
    `client_functions(role)`.
  - `_fixtures.psql` makes two accounts and one row per table each. `insert_sql` has one case per
    matrix table and raises on a table it does not know, so a new matrix row without a fixture fails
    loud.
  - **Controller:** push. **Expected:** `09` green, and `00` still green.
- [ ] Commit, naming the three files: `test(cloud): RLS probes and fixtures, each proven to see a planted fault`.

## T6. Coverage and `anon` (cloud-engineer)

Files: `cloud/supabase/tests/01_coverage_test.sql`, `02_anon_test.sql` (new).

- [ ] `01`: C1 to C8.
  - C3 compares `pg_policies.qual` as text with `(<key_col> = auth.uid())`.
  - C5 reads `reloptions` for `security_invoker`.
  - C6 calls `has_table_privilege` for `anon` and `authenticated` on select, insert, update and delete,
    against `grants`.
  - C7 runs `insert_is_valid` for every table, and counts A's and B's rows as the owner.
  - `plan()` is computed from `tests.matrix`.
- [ ] `02`: A1 to A5 over every matrix table and view, plus V3 as `anon`.
  - Each probe runs between `as_anon()` and `as_owner()`; the assertion runs as the owner.
  - Each description names the case and the relation, e.g. `A2 anon cannot insert into public.issues`.
- [ ] **Controller:** push. **Expected:** green on the first run. Each probe was already shown red in
  T5.
  - Any red case stops the task under §3.
  - If Q2 found a table staging lacks, record it here; the matrix follows the migrations.
- [ ] Commit: `test(cloud): every relation is listed, its policies exact, and anon gets nothing`.

## T7. One student, never another's rows (cloud-engineer)

Files: `cloud/supabase/tests/03_own_rows_test.sql` (new).

- [ ] O1 to O13, for every `own` row, and V1.
  - After each write probe (O6 to O10), the owner re-reads A's and B's rows and asserts that the count
    and the `touch_col` value are unchanged. A probe that reported 0 while a trigger moved data would be
    caught here.
  - O5 uses `insert … on conflict (<primary key>) do update set <touch_col> = excluded.<touch_col>`
    with B's existing key.
  - O13 uses a fresh random `sub` that no `auth.users` row has.
- [ ] **Controller:** push; expect green. Any red stops the task under §3.
- [ ] Commit: `test(cloud): a student reads only their own rows and writes none`.

## T8. The service-only tables (cloud-engineer)

Files: `cloud/supabase/tests/04_service_only_test.sql` (new).

- [ ] S1 to S5, for every `service` row, as A. For the keyed ones, also as B against B's own rows: a
  student gets nothing even of their own.
  - V2: every `closed_view` as A is 42501.
- [ ] **Controller:** push; expect green.
- [ ] Commit: `test(cloud): service-only tables answer no student, their own rows included`.

## T9. Functions, the admin schema, storage and realtime (cloud-engineer)

Files: `cloud/supabase/tests/05_functions_test.sql`, `06_platform_test.sql` (new).

- [ ] `05`: F1 to F5.
  - F1 and F2 read `pg_proc` in `public` and `admin`:
    - excluding extension members;
    - F2 also excludes functions whose return type is `trigger`, and anything `tests.rpc_allowlist`
      names;
    - checking `has_function_privilege` for `anon` and `authenticated`.
  - F3 builds each call from `pg_proc` (`oid::regprocedure`, one typed `null` per argument) for the
    SECURITY DEFINER set, as `anon` and as A, and expects 42501.
  - F4 calls `read_google_grant(<B>, 'https://www.googleapis.com/auth/gmail.readonly')` as A.
  - The definer set is read from the catalog, not listed by hand. A `diag` names its size; T11 checks
    that it reads 7.
- [ ] `06`: P1 to P3, each guarded by `has_schema` where the schema may be absent.
- [ ] **Controller:** push; expect green.
- [ ] Commit: `test(cloud): no client role executes a function, reaches admin, or has a bucket or publication`.

## T10. Docs (docs-keeper)

- [ ] `cloud/supabase/README.md` "Testing": a paragraph covering:
  - what `cloud/supabase/tests/` proves;
  - that it runs in `cloud-rls`;
  - the two commands, for a developer with Docker:
    `supabase --workdir cloud db start; supabase --workdir cloud test db`;
  - that a new table or view needs a `_matrix.psql` row, a fixture case and nothing else;
  - that a CLI bump updates `SUPABASE_CLI_VERSION`, its SHA-256 and `images.lock` in one commit.
- [ ] `HANDOFF.md`: one line under the cloud stream. Batch it with the merge (CI-minutes note).
- [ ] Commit, naming both files.

## T11. Review and close (reviewer, then main session)

- [ ] `reviewer` reads the whole branch against this plan and §8.
  - Every probe re-raises an unexpected SQLSTATE.
  - No assertion runs as a client role.
  - No file holds an id, address or token.
  - The job holds no secret.
  - No migration or static test is edited.
  - Its findings land in `docs/reports/`, and their answers in §11.
- [ ] Main session, cargo slot: the workspace suite once. 0 warnings beyond the accepted `.rsrc` line.
  1. `cargo build --workspace -j 2`;
  2. `cargo build -p knowlu-engine -j 2`;
  3. `cargo test --workspace -j 2`.

  A `knowlu/pending/session` failure under parallel runs is the known flake: report it, do not fix it.
- [ ] The Deno line once more; record the final counts.
- [ ] **Controller:** mark the PR ready. It merges on green CI and the reviewer's PASS (the lane-PR
  rule). Quinn's Q1 answer decides when `cloud-rls` joins branch protection.

---

## 7. Risks and what each fallback costs

| Risk | Detected by | Fallback | Cost |
|---|---|---|---|
| `db start` does not apply migrations under the pinned CLI | T4, smoke `has_table` | `db reset` after `db start` | about 20 s per run |
| `db start` does not exist, or it starts more than the database | T4, the step log | `supabase start -x <all but db>` | a longer pull |
| `\ir` is unusable under the CLI's pg_prove | T4, smoke | a committed `tests` schema in `000_setup_test.sql` (D1) | file order matters; a local database keeps the schema |
| The local `auth.users` columns differ from staging's | T5, fixtures | the minimal column list (`instance_id`, `id`, `aud`, `role`, `email`, the two metadata columns, timestamps) | none to the tests |
| pgTAP not on `search_path` | T4 | `set search_path = public, extensions` in `_setup.psql` | none |
| A CLI image moves under a tag | `verify` | bump the lock deliberately | one reviewed line per image |
| A probe reports a denial it did not get (false green) | T5's planted faults; re-raise on unknown SQLSTATE; owner re-reads after writes | | |
| A real hole is found | any T6 to T9 case | stop (§3) | the lane waits for a fix lane |

## 8. Fidelity ledger

| Requirement (dispatch) | Decision | Task | Proof |
|---|---|---|---|
| (1) Where tests live, given `cloud/supabase`'s layout | D1 | T3 | `00_smoke_test.sql` green via `supabase --workdir cloud test db` |
| (2) CI runs them; startup time and `auth.uid()`/role fidelity weighed | D2 | T3, T4 | the comparison table; T4's timings; C8 |
| (2) CLI and images pinned by version and digest | D2, Q4 | T0, T4 | SHA-256 check in the job; `images.lock` plus `verify` |
| (2) The three required checks still report on docs-only PRs | D2 | T3 | the existing `workflows.rs` test unedited and green; the new `cloud-rls` shape pin |
| (2) Whether it becomes required | D2, Q1 | T11 | Quinn's answer recorded at T0 or T11 |
| (3) Every public table: anon gets nothing | D3 | T6 | A1 to A4 per matrix table |
| (3) Policy tables: A reads only A's rows; never B's; select, insert, update, delete, and the move to B | D3 | T7 | O1 to O13 per `own` row, owner re-read after each write |
| (3) Policy-less tables: authenticated gets nothing | D3 | T8 | S1 to S5 per `service` row |
| (3) Every SECURITY DEFINER function closed to anon and authenticated | D3, Q3 | T9 | F1, F3, F4 (and F2 if Q3 is yes) |
| (3) Storage buckets, if any | D3 | T9 | P1, P2 (none today) |
| (3) Views, the `admin` schema, realtime (added) | D3 | T6, T8, T9 | A5, V1 to V3, C5, F5, P3 |
| (4) A new table not in the matrix fails | D4 | T1, T2, T6 | C1 in pgTAP; `rls_matrix_test.ts` in the required `cloud` job; T5's `zz_unlisted` fault |
| (5) Tasks in TDD order with a fidelity ledger | D6 | all | T1 red then T2; T3 red then green; T5's planted faults before T6 to T9 |
| Impersonation consistent with how Supabase evaluates `auth.uid()` | D5 | T3 | C8, on the image's own `auth.uid()` |
| `migrations_test.ts` stays as it is | §3 | T11 | the diff touches no static migration test |
| No staging, no prod, no migration, no secret | §3 | T11 | the diff; the job pin's forbidden list |

## 9. What the suite does not prove

- It proves the database refuses a client role. It does not prove an edge function scopes its
  service-role queries by the caller's account.
  - The service role bypasses RLS.
  - That property is the Deno account-scoping guards' job (`_shared/judge_db_test.ts`) and each
    handler's tests, which stay as they are.
- It proves the database built from this repository's migrations. Staging and prod can differ:
  dashboard-made buckets, a migration staging never applied, Q2's kind. Q5's controller-run proof is
  the check for that.
- It proves the four PostgREST verbs, upsert and RPC. Through `pg_graphql`, the same table privileges
  and RLS decide; through Realtime, P3 shows nothing is published.

## 10. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T0. Findings are answered in §11, and the verdict goes
   on the status line.
2. **After T0:** Quinn's answers or the controller's interim rulings on Q1 to Q5. Q2's count is
   recorded.
3. **After T4:** the controller reads the timings, the fallbacks used and the lock before T5. If the job
   takes over 6 minutes, the controller rules then.
4. **After T5:** each planted fault was seen, with the run id, before any matrix-wide file lands.
5. **After each of T6 to T9:** green, or a stop under §3.
6. **T11** before ready-for-review. The PR merges on green CI and the reviewer's PASS.

## 11. Fidelity notes

None yet (r0).

## 12. Out of scope

- Any fix to a policy, grant or function's rights. A hole found here goes to the controller privately
  and to its own lane (§3).
- Privileges PostgREST cannot exercise: `truncate`, `references`, `trigger`, and sequence `usage`.
- `service_role`'s own rights, beyond what the matrix needs to build fixtures.
- Edge-function authentication and account scoping (§9).
- Running the suite against staging or prod from CI. Q5's one-off proof is the controller's, in a
  follow-up.
- A path filter for `cloud-rls` (D2), and image caching between runs.
- The `account` function's purge list. `migrations_sync_test.ts` pins it for the sync tables; widening
  that to every account-scoped table is a separate test.
