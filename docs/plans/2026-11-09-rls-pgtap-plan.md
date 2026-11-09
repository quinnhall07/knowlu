# Row-level security isolation tests with pgTAP: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: r2, review r2 passed with notes folded.** §4 is answered. Plan review r1 (2026-11-09) failed: I1
to I5 and M1 to M6, folded in, and §13 maps them. Plan review r2 passed with ten notes, folded in, and
§13.1 maps them.

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

The migrations leave 36 tables: 26 without a policy and 10 with one. Q2 is settled at 36: staging's table
list has 36 rows, and the advisor's 35 was a miscount.

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
  - `cloud/supabase/tests/images.lock` holds one `repo:tag@sha256:<digest>` line for every image the two
    commands run. That includes the one-off containers `db start` uses on Postgres 15 and later to apply
    the platform's own auth and storage schema migrations, not only the database and pg_prove images
    (r1 M1). T4 reads the full list off the first run.
  - `bash scripts/ci/supabase-images.sh baseline` runs as the job's first heavy step. It saves
    `docker image ls --digests` to `$RUNNER_TEMP`, because the hosted runner ships with images of its own.
  - `… pull` pulls each locked image by digest and tags it, so the CLI finds it locally and pulls nothing.
  - `… verify` runs after the suite and diffs the image list against that baseline (r1 M1). It fails
    when:
    - an image that is new since the baseline is not in the lock; or
    - a locked tag points at another digest.
  - So a CLI bump that changes an image fails until `images.lock` changes with it, in the same diff.
  - The script is always invoked as `bash scripts/ci/…`. A file committed from Windows lands as 100644,
    and a bare `./script` would fail with "Permission denied" (r1 I5).
- **Every `supabase` call carries `--workdir cloud`**, `supabase --workdir cloud --version` included
  (r1 M6). T3's pin checks every line that invokes the binary.
- **`config.toml`'s SMTP block** (`[auth.email.smtp]`, `enabled = true`, `pass =
  "env(SMTP_PASSWORD)"`) is read by every CLI command, and the job leaves that variable unset (r1 M2).
  - T4 records whether the pinned CLI warns or errors on it.
  - If it errors, the named fallback is a job-level `SMTP_PASSWORD: ci-local-unused`. That is a literal,
    not a secret: the local database sends no mail, and `db start` starts no auth server.
  - The fallback is recorded as a deviation. The `workflows.rs` pin allows exactly that literal and no
    `secrets.` reference.
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
| `touch_col` | a column that an update probe sets to a sentinel (r1 I1). Never one of: generated; identity; trigger-rewritten; under a CHECK constraint; of an enum type; part of a foreign key (r2 note 6). A sentinel there would fail for its own reason, under the owner as well, and C7's validity run would show it |
| `grants` | `default` (client roles hold Supabase's default table rights) or `revoked` |
| `policies` | the exact policy names, `{}` for none |
| `view_insert_code` | views only: the SQLSTATE V3 expects from `insert … default values` (r2 note 1) |

**V3's expected SQLSTATE, row by row** (`view_insert_code`):
- `55000` for the five public closed views and for `billing_subscribers`.
  - `telemetry_daily`, `correction_rates` and `monthly_spend` aggregate; `effective_entitlements` and
    `billing_subscribers` join; `sync_limits` has no `from`.
  - None of them is updatable, and Postgres raises 55000 for that before it checks any privilege.
- `42501` for the three `admin` views, because the schema's `usage` is checked first.

The probe catches only 42501, 55000 and 0A000, and the case compares the code with the row's value.
Success, or a code outside the caught set, fails the case.

**What a probe returns (r1 I2).** One integer, so a grant-level refusal and an RLS refusal never look
alike:
- `n >= 0`: the statement ran, and `n` is the rows it saw or touched;
- `-1`: refused by a privilege. SQLSTATE 42501 with `MESSAGE_TEXT` starting `permission denied for`
  (table, view, schema or function);
- `-2`: refused by row-level security. SQLSTATE 42501 with `MESSAGE_TEXT` containing `violates row-level
  security policy`.

The probe reads the message with `get stacked diagnostics`. Any other SQLSTATE, or a 42501 whose message
matches neither phrase, is re-raised. It is never mapped to a denial, so a probe that breaks fails loud.
The phrases are English server messages. T3's smoke file checks `lc_messages` is `C`, `C.*`, `POSIX` or
`en*` (r2 note 5). The stated proof that both phrases really occur is the I2-split self-test (D6): no
fallback relies on `set lc_messages`, which probably needs a superuser on the Supabase image.

**"Per `grants`"** in the table below means:

| Probe | `grants = default` | `grants = revoked` |
|---|---|---|
| read | 0 | -1 |
| update, delete | 0 | -1 |
| insert, upsert | -2 | -1 |

**An update or delete that "touched 0 rows" is checked by the owner too (r1 I1).**
- One driver call does the whole check (D5's `tests.probe_write`, r2 note 4). Before the write probe, the
  owner records the target rows' `(primary key, ctid)` pairs.
- Every update probe sets `touch_col` to a per-type sentinel, not to itself: `'rls-probe'` for text,
  `-424242` for numbers, `now() - interval '1 year'` for timestamps, `not touch_col` for booleans.
- After the probe, the owner re-reads the pairs and the `touch_col` value. Any changed `ctid`, missing row
  or sentinel value fails the case, whatever count the probe returned. Any update, a self-assigning one
  included, gives a row a new `ctid`, so the re-read sees it.

Every assertion runs as the owner. Only the probe runs as the client role, through D5's driver.

| # | As | Relations | Probe | Expect |
|---|---|---|---|---|
| A1 | `anon` | every table | `select count(*)` | per `grants` |
| A2 | `anon` | every table | insert a valid row (for A) | per `grants` |
| A3 | `anon` | every table | update `touch_col` to the sentinel on every row | per `grants`; owner re-read unchanged |
| A4 | `anon` | every table | delete every row | per `grants`; owner re-read unchanged |
| A5 | `anon` | every view | `select count(*)` | 0 for `invoker_view`, -1 for `closed_view` |
| O1 | A | `own` | `select` | exactly A's fixture rows, every one keyed to A |
| O2 | A | `own` | `select … where <key> = B` | 0 |
| O3, O4 | A | `own` | insert a row for A; for B (D5's cleared slot) | -2 |
| O5 | A | `own` | upsert onto B's existing primary key (`on conflict … do update`) | -2 |
| O6, O7 | A | `own` | update A's rows; B's rows (sentinel) | 0; owner re-read unchanged |
| O8 | A | `own` | `update … set <key> = B where <key> = A` (the move) | 0; owner re-read unchanged |
| O9, O10 | A | `own` | delete A's rows; B's rows | 0; owner re-read unchanged |
| O11 | B | `own` | `select` | exactly B's rows (the mirror of O1) |
| O12 | `authenticated`, no `sub` claim | `own` | `select` | 0 |
| O13 | `authenticated`, a `sub` with no account | `own` | `select` | 0 |
| S1 | A | `service` | `select`, all rows and `where <key> = A` | per `grants` |
| S2 | A | `service` | insert a row for A (cleared slot) | per `grants` |
| S3, S4 | A | `service` | update A's rows (sentinel); move A's rows to B | per `grants`; owner re-read unchanged |
| S5 | A | `service` | delete A's rows | per `grants`; owner re-read unchanged |
| V1 | A | `billing_subscribers` | `select` | A's row only (A and B both hold an active subscription in the fixture) |
| V2 | A | `closed_view` | `select` | -1 |
| V3 | `anon`, A | every view | `insert … default values`, through its own probe `tests.view_insert` (r1 I4a) | the row's `view_insert_code`: `55000` for the six public views, `42501` for the three `admin` views (r2 note 1) |
| F1 | catalog | `public`, `admin` | SECURITY DEFINER functions executable by `anon` or `authenticated` | none |
| F2 | catalog | `public`, `admin` | any non-trigger function executable by `anon` or `authenticated`, minus the matrix's RPC allowlist (empty today) | none |
| F3 | `anon`, A | the 7 definer functions | call with typed nulls | -1 |
| F4 | A | `read_google_grant(B, …)` | call naming B's account | -1. A grant-level deny, like F3, and not an account-scoping proof: the function has no `auth.uid()` binding, and revoking it is the whole defence (r1 M3) |
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

-- The one probe body (r1 I2, I4c). It takes SQL TEXT, never a regclass: a cast such as
-- 'admin.testers'::regclass made in the caller's argument list would resolve under the client role,
-- outside this handler, and raise there. The owner builds the text from tests.matrix, and the
-- relation is first resolved inside the EXECUTE below, under the handler.
create function tests.run(p_sql text) returns bigint language plpgsql as $$
declare n bigint; msg text;
begin
  if p_sql ~* '^\s*select\s+count\(\*\)' then
    execute p_sql into n;                -- a read probe: the count it saw
  else
    execute p_sql;                       -- a write probe: the rows it touched
    get diagnostics n = row_count;
  end if;
  return n;
exception when insufficient_privilege then
  get stacked diagnostics msg = message_text;
  if msg like 'permission denied for %' then return -1; end if;
  if msg like '%violates row-level security policy%' then return -2; end if;
  raise;                                 -- an unknown 42501 is not a denial
end $$;

-- The driver crosses roles (r1 M4). It is called by the owner, switches to the client inside, keeps
-- the result in a plpgsql local, which survives `as_owner()`, and returns it to the owner's assertion.
-- Nothing pgTAP writes ever happens under a client role.
create function tests.probe(p_who text, p_sql text, p_clear text default null) returns bigint
language plpgsql as $$
declare r bigint; d text;
begin
  begin                                  -- subtransaction: a clear, or a write that got through, is undone
    if p_clear is not null then execute p_clear; end if;   -- the owner empties the target slot (r1 I4b)
    perform tests.as_who(p_who);         -- 'owner' | 'anon' | 'A' | 'B' | 'nosub' | 'stranger'
    r := tests.run(p_sql);
    perform tests.as_owner();
    raise exception using errcode = 'KNRLS', detail = r::text;   -- KNRLS: this file's own code
  exception when sqlstate 'KNRLS' then
    get stacked diagnostics d = pg_exception_detail;
  end;
  return d::bigint;
end $$;
```

**A write that got through is undone, but it is still caught.**
- `tests.probe` rolls its subtransaction back on purpose, and Postgres reverts the role and claim
  settings with it. So the effect of a write that got through is undone. Its count is not: the probe
  reports it.
- **The write probes use `tests.probe_write(who, rel, sql, target_where)`**, which does the whole check
  of D3 in **one** driver call (r2 note 4). It returns `(n bigint, changed boolean)` to the owner. Inside
  the same KNRLS subtransaction it:
  1. as the owner, takes the snapshot of the target rows: `(primary key, ctid, touch_col::text)`, built
     from the matrix's `key_col`, `touch_col` and the table's primary key;
  2. switches to the client and runs the probe through `tests.run`;
  3. resets to the owner and re-reads the same rows;
  4. sets `changed` when any `ctid` moved, any row is missing, any row left its key, or any `touch_col`
     holds the sentinel;
  5. raises KNRLS with `n` and `changed` in the detail.

  The subtransaction then undoes the write, and the owner's assertions read the two returned values.
- There is no psql savepoint and no `\gset`. A write case is one SQL call that a loop over
  `tests.matrix` can make, so a new table needs only its matrix row and its fixture case (T10's promise
  holds).
- `tests.probe`, `tests.probe_write`, `tests.under_fault` and `tests.call_outcome` are all SECURITY
  INVOKER. They switch roles with `set local role` and never run with the definer's rights.

**The cleared slot (r1 I4b).**
- `accounts`, `entitlements`, `billing_reminders`, `sync_usage`, `google_accounts` and `mail_accounts`
  hold one row per account. Several of those rows are made by triggers or by sign-up. So an insert "for
  A" collides with A's fixture row and raises 23505, and that error is not a denial.
- Every insert probe and every owner validity check therefore runs with `p_clear =
  tests.clear_sql(rel, account)`. As the owner, inside the driver's subtransaction, that deletes the
  target account's rows from that table first.
  - For `accounts`, the delete cascades; `auth.users` keeps the account's row, so the re-insert is
    valid.
  - For a global table, it deletes the one row whose key the insert reuses.
- The client then inserts into an empty slot, so the only thing that can refuse it is a grant or a
  policy.

Other helpers:
- `tests.insert_sql(rel, account)`: a valid insert for that table and account, one case per matrix
  table. It raises on a table it does not know.
- `tests.view_insert(rel)`: V3's own probe. It returns a SQLSTATE and catches only `42501`, `55000` and
  `0A000`. The case compares it with the row's `view_insert_code`.
- `tests.call_outcome(who, sql)`: for F3 and F4 under a fault (r2 note 8). It returns `denied` for a
  `permission denied` 42501, and `executed` for success or any other SQLSTATE.
- `tests.unlisted()`: D4.
- `tests.client_functions(role)`: F1 and F2.
- `tests.under_fault(fault, check)`: D6.

The `tests` schema gets `usage` and `execute` for `anon` and `authenticated`, because the probe body runs
under them. The whole schema is rolled back with the file.

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
- **Validity, proven before anything is denied.** Every insert that A2, O3, O4, O5 and S2 expect to be
  refused uses the same `tests.insert_sql` text and the same `tests.clear_sql` slot.
  - The owner runs that pair first, through the driver with `p_who = 'owner'`, and the driver's
    subtransaction undoes it. C7 makes one case per table and account, each wrapped in pgTAP's
    `lives_ok`, because `tests.run` re-raises a constraint error such as 23514 rather than return a
    count (r2 note 7).
  - A -1 or -2 from the client can then only mean a grant or a policy refused it. It can never mean a
    bad row or a 23505 on a one-row-per-account table (r1 I4b).
- **No real data.** No real address, name, course or note text: `rls-a@example.invalid`, a
  `tasks/rls-fixture.md` path, `{}` bodies where a body is required.

`plan()` is computed from `tests.matrix`. A row added or dropped moves the count with it, so no case is
skipped silently.

### D6. Tests first, in a lane where the schema is already right

- A deny test against a correct schema is green on its first run, so "red first" cannot mean "fails until
  the code is written". Here it means two things.
  - Every check is first shown to fail against its own planted fault, in `09_self_test_test.sql` (r1 I3).
  - Each fault is isolated. `tests.under_fault(fault_sql, check_sql)` runs the fault as the owner, then
    the check, inside one subtransaction it always rolls back, and returns the check's result to the
    owner, the way `tests.probe` does.
    - The check is any owner-side expression: a `tests.probe` call, a `tests.probe_write` call, or a
      catalog query.
    - Nested subtransactions are fine.
    - No fault outlives its own case, so no fault can mask or cause another's result.
    - The pgTAP assertion runs after the rollback, so its result row is not rolled back with the fault.
  - The faults. Every row must make its check report the fault:

| Check | Planted fault (on `issues` unless named) | The check must report |
|---|---|---|
| A1 | `create policy zz on issues for select to anon using (true)` | a count above 0 |
| O1, O2, O11 | `… for select to authenticated using (true)` | B's row visible to A (O1, O2); A's row visible to B (O11) |
| V1 | select-all policies on **both** `accounts` and `entitlements` (the view inner-joins them; r2 note 2) | V1 shows two rows |
| O3, O4, A2 | `… for insert to anon, authenticated with check (true)` | a count of 1, not -2, for A's slot and B's |
| the I2 split | `revoke insert on issues from authenticated` | O3 returns -1, not -2 |
| O5 | the insert policy, plus `… for update using (true)`, plus a select-all policy | the upsert succeeds |
| O6, O7, A3 | `… for update to anon, authenticated using (true)`, plus a select-all policy for both roles | O7 and A3 above 0; the owner's re-read shows a new `ctid` and the sentinel |
| O8 | `… for update using (true) with check (true)`, plus a select-all policy (r2 note 3) | the move above 0; the row is under B in the re-read |
| O9, O10, A4 | `… for delete to anon, authenticated using (true)`, plus a select-all policy for both roles | O10 and A4 above 0; the row is gone from the re-read |
| O12, O13 | `… for select to authenticated using (auth.uid() is null or true)` | above 0 |
| S1 to S5 | `create policy zz on judgments for all to authenticated using (true) with check (true)` | each S probe changes result |
| C1 | `create table public.zz_unlisted (id int)` | the name in `tests.unlisted()` |
| C2 | `alter table issues disable row level security` | C2 false |
| C3 | `alter policy issues_select_own on issues using (true)` | the `qual` mismatch |
| C4 | `create policy zz on judgments for select using (true)` | a policy where none is listed |
| C5 | `alter view billing_subscribers set (security_invoker = false)` | the option mismatch |
| C6 | `revoke select on issues from authenticated` | the privilege mismatch (and O1 returns -1) |
| C7 | `delete from issues where account_id = <B>` | the non-vacuous check fails |
| A5, V2 | `grant select on monthly_spend to anon, authenticated` | a count, not -1 |
| V3 | `create view public.zz_v as select * from issues` (updatable, runs as its owner), then `grant insert` to `anon` | `view_insert` reports success, which V3 fails |
| F1, F3, F4 | `grant execute on function read_google_grant(uuid, text) to anon, authenticated` | listed by F1. F3 and F4 report `executed`: under a fault, `tests.call_outcome` counts any outcome other than a `permission denied` 42501 as executed, because a definer function called with nulls may raise something else (r2 note 8) |
| F2 | `grant execute on function public.sync_prune(int) to anon` | listed by F2 |
| F5 | `grant usage on schema admin to anon` | F5 true |
| P1 | one `storage.buckets` row (when the schema exists) | count 1 |
| P2 | `create policy zz on storage.objects for select using (true)` | an unlisted policy |
| P3 | `alter publication supabase_realtime add table public.issues` (or `create publication zz for table public.issues` if T4 finds none) | listed |

**Why the select-all companions.**
- Postgres also applies SELECT policies to:
  - the rows an UPDATE or DELETE reads through its `where`;
  - the new row of an UPDATE that has a `where` (r2 note 3);
  - the existing row an upsert conflicts with.
- Without the companion, an update, delete or move fault alone would still report 0 or -2, and so
  would prove nothing.
- Each such row therefore plants the select-all policy too, and its case still names one check.

  - **Guard rails.**
    - A probe given a broken statement (an unknown column) raises; it is never read as 0, -1 or -2.
    - A 42501 raised by `raise exception using errcode = '42501', message = 'other'` is re-raised, not
      classified.
    - The owner's validity run is wrapped in pgTAP's `lives_ok` (r2 note 7), because `tests.run`
      re-raises a 23514. The guard rail shows a row that breaks a check constraint makes that run raise,
      with `throws_ok(…, '23514')`, so C7 cannot pass on a bad `insert_sql`.
    - The I2 split doubles as the proof that both message phrases occur on this image (r2 note 5): with
      no fault, O3 returns -2 (the RLS phrase), and under the `revoke insert` fault it returns -1 (the
      `permission denied` phrase). A locale that changed either phrase fails one of the two.
    - With `request.jwt.claim.sub` set to B, a following `as_user(A)` still makes `auth.uid()` A (C8).
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
  'public'` on staging. **Closed:** staging has 36, so the advisor's figure was a miscount and nothing
  changes. The matrix follows the migrations either way.
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
| T3 | cloud-engineer (cargo slot) | `cloud-rls` job, `_setup.psql` impersonation, `00_smoke_test.sql`, images script (`baseline`, `list`) | `workflows.rs` pin | pin + first CI run |
| T4 | controller, then cloud-engineer (cargo slot) | read the first run; `images.lock`; `pull` and `verify` on; SMTP outcome | `workflows.rs` pin extended | second CI run |
| T5a | cloud-engineer | the driver and probes; the faults that need no fixture rows | self-test vs no probes | CI |
| T5b | cloud-engineer | accounts and the 10 `own` tables: fixtures, `insert_sql`, `clear_sql`; their faults | their faults vs no fixtures | CI |
| T5c | cloud-engineer | the 26 `service` tables: fixtures, `insert_sql`, `clear_sql`; their faults | their faults vs no fixtures | CI |
| T6a | cloud-engineer | `01_coverage_test.sql` (C1-C8) | T5a-T5c faults | CI |
| T6b | cloud-engineer | `02_anon_test.sql` (A1-A5, V3) | T5a-T5c faults | CI |
| T7 | cloud-engineer | `03_own_rows_test.sql` (O1-O13, V1) | T5a-T5c faults | CI |
| T8 | cloud-engineer | `04_service_only_test.sql` (S1-S5, V2) | T5a-T5c faults | CI |
| T9 | cloud-engineer | `05_functions_test.sql` (F1-F5), `06_platform_test.sql` (P1-P3) | T5a-T5c faults | CI |
| T10 | docs-keeper | README "Testing", HANDOFF line | | |
| T11 | reviewer, then main session | whole-branch review, workspace suite, ready for review | | PASS |

**Edit size (r1 I5).** No single edit is longer than about 80 lines. A file longer than that (the
fixtures, `insert_sql`, any matrix-wide test) is written in successive edits of 80 lines or fewer, each
one complete SQL statements, and each task's ledger line counts its edits.

## T0. Base, baselines and pins (main session)

- [ ] Confirm that the worktree's HEAD sits on `c61c7cec`, with only this plan's commits above it
  (`c93ae9e2` and its r1 revision). Record it in the lane ledger.
- [ ] Run CI's full Deno line from the repo root, and record the pass count (the baseline).
- [ ] Pick the current stable Supabase CLI release. Record:
  - its version;
  - the SHA-256 of `supabase_linux_amd64.tar.gz`, taken from the release's checksums file
    (`gh release download`, no execution).
- [ ] Copy §4's answers into the ledger: Q1 required after ten greens, Q2 36, Q3 the stronger rule, Q4
  digest, Q5 yes.

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
  - `touch_col` is never:
    - a generated column (`sync_records.keep`);
    - an identity column;
    - a column under a trigger that rewrites it (`sync_notes.rev`);
    - a column under a CHECK constraint;
    - an enum-typed column;
    - part of a foreign key (r2 note 6).
  - Every view row carries its `view_insert_code` (D3, r2 note 1).
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
  - `--workdir cloud` on every line that invokes the `supabase` binary, `--version` included (r1 M6);
  - `sha256sum -c` before the CLI's first use;
  - every call of a script under `scripts/ci/` is `bash scripts/ci/…`, never a bare path (r1 I5);
  - `env:` names no `SMTP_PASSWORD`, unless it is the literal `ci-local-unused` (D2, r1 M2).

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
      - name: images already on the runner
        if: needs.changes.outputs.code != 'false'
        run: bash scripts/ci/supabase-images.sh baseline
      - name: the supabase CLI, checked before it runs
        if: needs.changes.outputs.code != 'false'
        run: |  # curl the tarball to $RUNNER_TEMP, sha256sum -c, untar, add to $GITHUB_PATH,
                # then: supabase --workdir cloud --version
      - name: a local database from the migrations (no project, no secrets)
        if: needs.changes.outputs.code != 'false'
        run: supabase --workdir cloud db start
      - name: pgTAP
        if: needs.changes.outputs.code != 'false'
        run: supabase --workdir cloud test db
      - name: images used
        if: always() && needs.changes.outputs.code != 'false'
        run: bash scripts/ci/supabase-images.sh list
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
  `plan(7)`:
  - `has_table('public', 'accounts', …)`: the migrations were applied;
  - C8's three `auth.uid()` checks;
  - `current_user` is `authenticated` after `as_user`;
  - `current_user` is the session user after `as_owner`;
  - `current_setting('lc_messages')` is `C`, `C.*`, `POSIX` or `en*` (r1 I2, r2 note 5). The
    phrases themselves are proven by T5b's I2-split self-test.

  It also prints a `diag` of `version()` and `session_user`, and nothing else.
- [ ] **`supabase-images.sh`** (LF, run with `bash`).
  - `baseline` saves `docker image ls --digests` to `$RUNNER_TEMP`.
  - `list` prints the images that are new since the baseline, with their digests.
  - `pull` and `verify` exist but are not yet called (T4).
- [ ] Run the workflows test and the Deno line: both green. Record them.
- [ ] Commit the five files, named one by one:
  `ci(cloud): cloud-rls job runs pgTAP on a local Supabase database`.
- [ ] **Controller:** push the branch, and open a **draft** PR so `pull_request` runs CI.

## T4. Read the first run; pin the images (controller, then cloud-engineer with a named cargo slot)

- [ ] **Controller:** from the first `cloud-rls` run, record in the ledger:
  - whether `db start` applied the migrations (the smoke file's `has_table`), or which D2 fallback is
    needed;
  - that the `.psql` includes ran (smoke green) and pg_prove did not pick them up as tests;
  - `session_user` and `lc_messages`;
  - the wall-clock time of each step;
  - the `list` output: every image new since the baseline, with its digest. That includes the one-off
    auth and storage migration containers `db start` ran (r1 M1);
  - whether the CLI warned or errored on `config.toml`'s unset `SMTP_PASSWORD` (r1 M2);
  - whether `storage` exists, its policy names (P2's expected set), and whether the
    `supabase_realtime` publication exists (P3's fault).
- [ ] **cloud-engineer**, in the cargo slot the controller names for this task (r1 I5):
  - write `cloud/supabase/tests/images.lock` from the `list` output, every image in it;
  - add `bash scripts/ci/supabase-images.sh pull` before `db start`, and `… verify` in place of `list`.
    `verify` diffs against the baseline (D2);
  - if the CLI errored on SMTP, add the job-level `SMTP_PASSWORD: ci-local-unused` (D2's named fallback);
  - extend the workflows test to require `baseline`, `pull` and `verify` in that order;
  - apply any other fallback the controller recorded (D1 or D2);
  - record each of the last two as a named deviation in the ledger;
  - run `cargo test -p knowlu-engine --test workflows -j 2`, then commit:
    `ci(cloud): pull the database images by digest and verify nothing else ran`.
- [ ] **Controller:** push. The second run must be green, with `verify` passing. If the job takes over
  6 minutes, raise it at checkpoint 3 before T5a.

## T5a. The driver and the probes, proven able to fail (cloud-engineer)

Files: `cloud/supabase/tests/09_self_test_test.sql` (new), `_setup.psql` (second part).

- [ ] **Red.** Write the parts of `09_self_test_test.sql` that need no fixture rows. Each fault goes
  through `tests.under_fault` (D6):
  - the faults for C1, F1/F3, F2, F5, P1, P2, P3, and A5/V2 (`monthly_spend` reads as a count, and
    needs no fixture to be readable);
  - the guard rails: a broken statement raises; a 42501 with an unknown message is re-raised; C8's
    stale-legacy-key check.
  - **Controller:** push. **Expected red:** the file fails because `tests.run`, `tests.probe`,
    `tests.under_fault`, `tests.unlisted` and `tests.client_functions` do not exist yet. Record the run.
- [ ] **Green.** Write those functions, `tests.probe_write`, `tests.call_outcome` and `tests.view_insert` into `_setup.psql`
  (D5), in edits of 80 lines or fewer.
  - **Controller:** push. **Expected:** `09` green, and `00` still green.
- [ ] Commit both files: `test(cloud): RLS probe driver, each probe proven to see a planted fault`.

## T5b. Accounts and the owner-read tables: fixtures and their faults (cloud-engineer)

Files: `_fixtures.psql` (new), `_setup.psql` (`insert_sql` and `clear_sql`, owner-read cases), and
`09_self_test_test.sql` (appended).

- [ ] **Red.** Append the faults that need the owner-read fixtures to `09`:
  - A1, O1/O2/V1, O3/A2, the I2 split, O5, O6/O7/A3, O8, O9/O10/A4, O12/O13, C2, C3, C5, C6, C7, V3.
  - **Controller:** push. **Expected red:** they fail because accounts A and B and their rows do not
    exist.
- [ ] **Green.** In edits of 80 lines or fewer:
  - In `_fixtures.psql`: A and B by sign-up (D5), then one row each in the 10 `own` tables, plus the
    active subscription V1 needs on both.
  - In `_setup.psql`: `insert_sql` and `clear_sql` for those 10 tables. `insert_sql` raises on any
    table it does not know yet.
  - **Controller:** push. **Expected:** `09` green.
- [ ] Commit the three files: `test(cloud): owner-read fixtures, each fault seen`.

## T5c. The service-only tables: fixtures and their faults (cloud-engineer)

Files: `_fixtures.psql`, `_setup.psql` and `09_self_test_test.sql` (each appended).

- [ ] **Red.** Append S1 to S5's fault and C4's fault to `09`. Push. **Expected red:** no `judgments`
  fixture yet.
- [ ] **Green.** In edits of 80 lines or fewer, add fixtures, `insert_sql` and `clear_sql` for the 26
  `service` tables.
  - Chains follow their foreign keys: `judgments` → `rules` → `rule_evidence`, and `mail_addresses` →
    its six dependants.
  - `models` uses a seeded row, and its `clear_sql` deletes the one `kind` its insert reuses.
  - **Controller:** push. **Expected:** `09` green, with every planted fault of D6 seen.
- [ ] Commit the three files: `test(cloud): service-only fixtures, each fault seen`.

## T6a. Coverage (cloud-engineer)

Files: `cloud/supabase/tests/01_coverage_test.sql` (new).

- [ ] C1 to C8.
  - C3 compares `pg_policies.qual` as text with `(<key_col> = auth.uid())`.
  - C5 reads `reloptions` for `security_invoker`.
  - C6 calls `has_table_privilege` for `anon` and `authenticated` on select, insert, update and delete,
    against `grants`.
  - C7 runs each table's `insert_sql` in its `clear_sql` slot as the owner, through the driver, inside
    `lives_ok` (r2 note 7). It also counts A's and B's rows.
  - `plan()` is computed from `tests.matrix`.
- [ ] **Controller:** push. **Expected:** green on the first run. Each check was already shown able to
  fail in T5a to T5c. Any red case stops the task under §3.
- [ ] Commit: `test(cloud): every relation is listed and its policies and grants are exact`.

## T6b. `anon` (cloud-engineer)

Files: `cloud/supabase/tests/02_anon_test.sql` (new).

- [ ] A1 to A5 over every matrix table and view, plus V3 as `anon`. Every probe goes through
  `tests.probe`, except A3 and A4, which go through `tests.probe_write` and assert `n` per `grants`
  and `changed = false`. Expectations are per `grants` (D3). V3 uses `tests.view_insert` against each
  row's `view_insert_code`.
  - Each description names the case and the relation, e.g. `A2 anon cannot insert into public.issues`.
- [ ] **Controller:** push; expect green. Any red stops the task under §3.
  - Drift between these results and staging is not this file's business: Q5's proof checks staging
    later.
  - Any staging difference the controller sees goes to Quinn privately. It never goes in a commit, the
    PR or the ledger (r1 M5).
- [ ] Commit: `test(cloud): anon gets nothing from any relation`.

## T7. One student, never another's rows (cloud-engineer)

Files: `cloud/supabase/tests/03_own_rows_test.sql` (new).

- [ ] O1 to O13, for every `own` row, and V1.
  - Each write probe (O6 to O10) is one `tests.probe_write` call (D5, r2 note 4). It asserts that
    `n = 0` and `changed = false`. `changed` covers every `(primary key, ctid)` pair, the sentinel, and
    a row leaving A or B.
    - An update or delete that touched a row fails the case even if its count was 0. T5b's
      `for update` and `for delete` faults showed both values fire.
    - The cases are generated by a loop over `tests.matrix`; nothing is written per table.
  - O3 and O4 run in the cleared slot (`clear_sql` for A, then for B) and expect -2 (D3, r1 I4b).
  - O5 uses `insert … on conflict (<primary key>) do update set <touch_col> = excluded.<touch_col>`
    with B's existing key, and expects -2.
  - O13 uses a fresh random `sub` that no `auth.users` row has.
- [ ] **Controller:** push; expect green. Any red stops the task under §3.
- [ ] Commit: `test(cloud): a student reads only their own rows and writes none`.

## T8. The service-only tables (cloud-engineer)

Files: `cloud/supabase/tests/04_service_only_test.sql` (new).

- [ ] S1 to S5, for every `service` row, as A, per `grants` (D3).
  - For the keyed ones, also as B against B's own rows: a student gets nothing, even of their own.
  - S2 runs in the cleared slot.
  - S3 to S5 use `tests.probe_write`, as in T7.
  - V2: every `closed_view` as A is -1.
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
    SECURITY DEFINER set, as `anon` and as A, through `tests.probe`, and expects -1.
  - F4 calls `read_google_grant(<B>, 'https://www.googleapis.com/auth/gmail.readonly')` as A, and expects
    -1. It is a grant-level deny, like F3, not an account-scoping test (r1 M3).
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
| The local `auth.users` columns differ from staging's | T5b, fixtures | the minimal column list (`instance_id`, `id`, `aud`, `role`, `email`, the two metadata columns, timestamps) | none to the tests |
| pgTAP not on `search_path` | T4 | `set search_path = public, extensions` in `_setup.psql` | none |
| A CLI image moves under a tag | `verify` | bump the lock deliberately | one reviewed line per image |
| The CLI errors on `config.toml`'s unset `SMTP_PASSWORD` (r1 M2) | T4, the step log | job-level `SMTP_PASSWORD: ci-local-unused` | one pinned literal |
| Server messages are not English, so D3's phrases do not match | the smoke file's `lc_messages` check (`C`, `C.*`, `POSIX`, `en*`); the I2-split self-test, which proves both phrases | none: stop and rule at checkpoint 3. `set lc_messages` probably needs a superuser on this image, so no fallback relies on it (r2 note 5) | the lane waits |
| A probe reports a denial it did not get (false green) | T5a to T5c per-check faults; re-raise on unknown SQLSTATE or message; the owner's `ctid` and sentinel re-read after writes | | |
| A real hole is found | any T6a to T9 case | stop (§3) | the lane waits for a fix lane |

## 8. Fidelity ledger

| Requirement (dispatch) | Decision | Task | Proof |
|---|---|---|---|
| (1) Where tests live, given `cloud/supabase`'s layout | D1 | T3 | `00_smoke_test.sql` green via `supabase --workdir cloud test db` |
| (2) CI runs them; startup time and `auth.uid()`/role fidelity weighed | D2 | T3, T4 | the comparison table; T4's timings; C8 |
| (2) CLI and images pinned by version and digest | D2, Q4 | T0, T4 | SHA-256 check in the job; `images.lock` plus `verify` |
| (2) The three required checks still report on docs-only PRs | D2 | T3 | the existing `workflows.rs` test unedited and green; the new `cloud-rls` shape pin |
| (2) Whether it becomes required | D2, Q1 (answered: after ten greens) | T11 | Quinn makes the branch-protection change |
| (3) Every public table: anon gets nothing | D3 | T6b | A1 to A4 per matrix table, per `grants`, with the owner's re-read after A3 and A4 |
| (3) Policy tables: A reads only A's rows; never B's; select, insert, update, delete, and the move to B | D3, D5 | T7 | O1 to O13 per `own` row: -2 on insert and upsert; 0 plus an unchanged `ctid` and sentinel re-read on every write |
| (3) Policy-less tables: authenticated gets nothing | D3 | T8 | S1 to S5 per `service` row, per `grants` |
| (3) Every SECURITY DEFINER function closed to anon and authenticated | D3, Q3 (answered: the stronger rule) | T9 | F1, F2, F3 (catalog and calls). F4 is the same grant-level deny on one cross-account call; it is **not** counted as an account-scoping proof (r1 M3, §9) |
| (3) Storage buckets, if any | D3 | T9 | P1, P2 (none today) |
| (3) Views, the `admin` schema, realtime (added) | D3 | T6b, T7, T8, T9 | A5, V1 to V3, C5, F5, P3 |
| (4) A new table not in the matrix fails | D4 | T1, T2, T6a | C1 in pgTAP; `rls_matrix_test.ts` in the required `cloud` job; T5a's `zz_unlisted` fault |
| (5) Tasks in TDD order with a fidelity ledger | D6 | all | T1 red then T2; T3 red then green; T5a to T5c show each check's own fault seen (red to green) before T6a to T9 |
| Every check able to fail, each fault isolated | D6 | T5a to T5c | one `tests.under_fault` row per check, rolled back per case (r1 I3) |
| Grant denial told apart from RLS denial | D3, D5 | T5a, T5b | -1 against -2 through `MESSAGE_TEXT`; the `revoke insert` fault moves O3 from -2 to -1 (r1 I2) |
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
  dashboard-made buckets, or a migration staging never applied. Q5's controller-run proof is the check
  for that.
- It does not prove that a SECURITY DEFINER function scopes by account. `read_google_grant` and its
  siblings bind no `auth.uid()`; revoking them from every client role is their whole defence, and F1 to
  F4 prove that revocation, nothing more (r1 M3).
- It proves the four PostgREST verbs, upsert and RPC. Through `pg_graphql`, the same table privileges
  and RLS decide; through Realtime, P3 shows nothing is published.

## 10. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T0. Findings are answered in §11, and the verdict goes
   on the status line.
2. **After T0:** §4's answers copied into the ledger (all five answered 2026-11-09).
3. **After T4:** the controller reads the timings, the fallbacks used, the SMTP outcome and the lock
   before T5a. If the job takes over 6 minutes, the controller rules then.
4. **After T5c:** every planted fault in D6's table was seen, each with its run id, before any
   matrix-wide file lands.
5. **After each of T6a to T9:** green, or a stop under §3.
6. **T11** before ready-for-review. The PR merges on green CI and the reviewer's PASS.

## 11. Fidelity notes

- **r0 to r1 (plan review r1, 2026-11-09, FAIL).** Five important findings and six minor ones, all
  folded in. §13 maps each to where it is answered.
- **r1 to r2 (plan review r2, 2026-11-09, PASS with notes).** Ten notes, all folded in; §13.1 maps them.
  The `probe_keep` savepoint mechanism of r1 is replaced by the single-call `tests.probe_write`.

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

## 13. Plan review r1: where each finding is answered

| Finding | Answer | Where |
|---|---|---|
| **I1** A self-assigning update is invisible to the owner's re-read; `tests.affected` was never planted against | Every update probe sets `touch_col` to a per-type sentinel. The owner snapshots `(primary key, ctid)` before each write probe and re-reads after it, so any update (self-assigning included) or delete is caught whatever count it returned. `for update using (true)` and `for delete using (true)` faults make O7 and O10 report above 0 and show a new `ctid`, the sentinel, or a missing row. The overstated "a trigger moved data" claim is gone | D3 ("checked by the owner too"); D5 (`probe_write`, which replaced r1's `probe_keep` at r2 note 4); D6 fault table (O6/O7/A3, O8, O9/O10/A4); T7; T8 |
| **I2** A grant deny and an RLS deny both map to -1 | `tests.run` reads `MESSAGE_TEXT` with `get stacked diagnostics`: `permission denied for …` is -1, `… violates row-level security policy …` is -2, and anything else is re-raised. Insert and upsert expect -2 on `grants = default` and -1 on `revoked`; closed views expect -1. The `revoke insert on issues from authenticated` fault turns O3 from -2 to -1. The "0, or 42501" cells now read "per `grants`", with a three-row key | D3 (return codes, the "per `grants`" table, the cases table); D5 (`tests.run`); D6 (the I2 split); T3 smoke (`lc_messages`); §7 |
| **I3** Five shared-transaction faults left most checks unproven and leaked into each other | One fault per check: 25 rows covering A1-A5, O1-O13, S1-S5, V1-V3, C1-C7, F1-F5 and P1-P3. Each runs through `tests.under_fault`, inside a subtransaction rolled back per case, with the assertion made after the rollback. The table includes every fault the review named (RLS disabled, `alter policy … using (true)`, `revoke select`, `grant execute … sync_prune` to `anon`, `grant usage on schema admin`, a `storage.buckets` row, `alter publication supabase_realtime add table`) | D6 (fault table and guard rails); T5a, T5b, T5c; §10 checkpoint 4 |
| **I4a** V3's insert into `billing_subscribers` raises 55000 and the re-raise aborts the file | V3 has its own probe, `tests.view_insert`. It returns the SQLSTATE and catches only 42501, 55000 and 0A000. It expects `42501` for closed views and `55000` for `billing_subscribers`; success or any other code fails the case | D3 (V3); D5 (other helpers); T6b; the V3 fault in D6 |
| **I4b** One-row-per-account and trigger-made rows make the validity insert raise 23505; `accounts` needs an `auth.users` row | Every insert probe and validity check runs in a cleared slot. `tests.clear_sql(rel, account)` deletes the target account's rows from that table, as the owner, inside the driver's subtransaction, before the insert. For `accounts`, the account's `auth.users` row stays, so the re-insert is valid. Global tables clear the one key they reuse | D5 ("The cleared slot", fixtures' validity bullet); T5b, T5c; T6a (C7); T7 (O3, O4); T8 (S2) |
| **I4c** A `regclass` argument resolves under the client role, outside the handler | Probes take SQL text that the owner builds from `tests.matrix`. The relation is first resolved inside the `execute`, under the handler, so `admin.testers` yields -1 | D5 (`tests.run` comment) |
| **I5** The script would be committed 100644; T5 and T6 were too big; T4's cargo slot was unnamed | Every script call is `bash scripts/ci/…`, pinned by T3's workflows test. T5 is split into T5a (driver and probes), T5b (owner-read fixtures) and T5c (service-only fixtures); T6 into T6a (coverage) and T6b (`anon`). The edit-size rule (80 lines or fewer per edit) is stated under §6. T4 runs in a cargo slot the controller names | D2 (pinning); T3; §6; T4 heading and steps |
| **M1** `db start` runs one-off auth and storage migration images; "any other image" is wrong on a hosted runner | `images.lock` lists every image the two commands run, the one-off migration containers included. `baseline` saves the runner's own images first, and `verify` diffs against it | D2 (pinning); T3 (`baseline` step and script); T4 |
| **M2** The SMTP block's unset `env(SMTP_PASSWORD)` | T4 records whether the pinned CLI warns or errors. If it errors, the named fallback is the literal `SMTP_PASSWORD: ci-local-unused`, allowed by the workflows pin and recorded as a deviation | D2; T3 pin; T4; §7 |
| **M3** F4 is a grant-level deny, not account scoping | F4 expects -1 and is labelled a grant-level deny. The ledger counts it with F1-F3 and states it is not an account-scoping proof. §9 says what it does not prove | D3 (F4); T9; §8; §9 |
| **M4** How a probe's result crosses `as_owner()` | `tests.probe` keeps the result in a plpgsql local, which survives the role reset, and returns it to the owner, so pgTAP never runs under a client role. `under_fault` and `probe_write` (r2) do the same | D5 (driver), D6 |
| **M5** Stale text: Q2 open in §1 and T6; staging drift in a public file; "F2 if Q3"; status | §1 and §4's Q2 say "closed at 36". T6b sends any staging difference to Quinn privately and never into a commit, the PR or the ledger. The ledger names F2 unconditionally (Q3 answered). The status line reads r1 | Status; §1; §4; T6b; §8 |
| **M6** `--workdir` on every `supabase` call | Every invocation carries `--workdir cloud`, `supabase --workdir cloud --version` included, pinned line by line in T3's workflows test | D2; T3 (pin and the CLI step) |

### 13.1 Plan review r2 (PASS with notes): where each note is answered

| Note | Answer | Where |
|---|---|---|
| **1** V3 per row: non-updatable views raise 55000 before any privilege check; only the `admin` views give 42501 | A new matrix column, `view_insert_code`. It is 55000 for the five public closed views and `billing_subscribers`, and 42501 for the three `admin` views. The case compares the probe's code with the row; only success, or a code outside {42501, 55000, 0A000}, fails | D3 (column table, "V3's expected SQLSTATE", V3 row); D5 (`view_insert`); T2; T6b |
| **2** V1's fault must cover both joined tables | Select-all policies on both `accounts` and `entitlements` | D6 fault table (V1 row) |
| **3** O8's fault needs the select-all companion | Added. The companion paragraph names an UPDATE's new row, not only its `where` | D6 fault table (O8 row); "Why the select-all companions" |
| **4** Drop the psql savepoint/`\gset` mechanism | `tests.probe_write(who, rel, sql, target_where)` does the snapshot, the client probe, the reset, the re-read and the raise in one KNRLS subtransaction. It returns `(n, changed)`. `savepoint w` is gone from T6b, T7 and T8; the cases are loops over `tests.matrix`, so T10's "a matrix row, a fixture case and nothing else" holds. Every driver is SECURITY INVOKER | D3 (owner check); D5 (`probe_write`, the invoker bullet); T5a; T6b; T7; T8 |
| **5** `lc_messages`: accept C, C.*, POSIX, en*; no superuser-dependent fallback | The smoke check accepts all four. The I2-split self-test is the stated proof that both phrases occur. §7 has no `set lc_messages` fallback; a mismatch stops at checkpoint 3 | D3 (message phrases); D6 guard rails; T3 smoke; §7 |
| **6** `touch_col` exclusions | Also never a column under a CHECK constraint, of an enum type, or part of a foreign key | D3 (`touch_col` row); T2 |
| **7** C7 guard rail: `lives_ok` around the owner validity run | C7's owner runs are wrapped in `lives_ok`. The guard rail shows a constraint-breaking row raises 23514 (`throws_ok`) | D5 (fixtures, validity); D6 guard rails; T6a |
| **8** F3/F4 under a fault: any non-42501 outcome counts as executed | `tests.call_outcome(who, sql)` returns `denied` only for a `permission denied` 42501, and `executed` for anything else | D5 (other helpers); D6 fault table (F1/F3/F4 row); T5a |
| **9** The fault table must name O4 and O11 | The O1/O2/O11 row and the O3/O4/A2 row | D6 fault table |
| **10** Paragraphs inside tables | "Why the select-all companions" now follows the fault table, and "Edit size" follows the tasks table, after T11's row | D6; §6 |
