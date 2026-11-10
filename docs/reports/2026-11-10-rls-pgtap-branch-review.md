PASS (Spec ✅, Quality ✅)

# RLS pgTAP lane: whole-branch review (T11)

- Branch `security/rls-pgtap` (PR #73). Reviewed `git diff c61c7cec 7d2482f6`, the lane's own work.
- The merge of main, `dcd7d5d6`, touches no migration, no file under `cloud/supabase/tests/`, and
  nothing in the `cloud-rls` job. Its one `ci.yml` change adds `cloud/eval/redteam/*.ts` to `deno check`.
- Plan: `docs/plans/2026-11-09-rls-pgtap-plan.md` (D1 to D6, T11, §8, §11). Ledger rulings taken as given.

No Critical or Important findings. Seven Minor ones, below. None lets a client-role hole pass the suite
as it stands: where a probe has a gap, a catalog check (C2, C4, C6 or F1) still names the fault.
The controller's answers are in the plan's §11.

## T11's own checks

| Check | Result |
|---|---|
| Every probe re-raises an unexpected SQLSTATE or message | Holds for `tests.run`, `tests.probe`, `tests.probe_write` and `tests.call_outcome`. `tests.view_insert` classifies any 42501 whatever its message: Minor 4. |
| No assertion runs as a client role | Holds. Every `is`/`ok`/`lives_ok` gets its value after the driver has returned to the owner. |
| No real id, address or token | Holds. Account ids come from `gen_random_uuid()`; addresses are `example.invalid`; `instance_id` is the all-zero constant; the SHA-256 values are public release and image digests. |
| `cloud-rls` holds no secret; env and permissions minimal | Holds. Env is `SUPABASE_CLI_VERSION` and `SUPABASE_CLI_SHA256` only; workflow-level `contents: read`; `persist-credentials: false`; no `secrets.`. The pin does not lock that in: Minor 7. |
| No migration or static test edited | Holds. `migrations_test.ts` untouched; `rls_matrix_test.ts` is new. |
| §8 ledger | Every row holds except "Every check able to fail", which F4 does not meet: Minor 2. |
| Docs match the code | README, HANDOFF R1 row and §11 match. One stale comment in `_matrix.psql`: Minor 1. |

## Cross-file checks

- **Helpers defined more than once:** yes, Minor 5. No `tests.*` name collides between files, and each
  file rolls back.
- **Leftover state between files:** none. Every file is `begin; \ir …; … rollback;`; every fault lives in
  `tests.under_fault`'s rolled-back subtransaction. Only non-transactional sequences advance (harmless).
- **`\if` blocks and includes:** 06 and 09 compute `has_storage` and `has_rt_messages` with the same
  expressions; both branches of every `\if` emit the same number of cases.
- **pg_prove file order:** nothing depends on it.

## Findings

### Minor 1: `_matrix.psql:32` still sends a new table's case to `_fixtures.psql`

The case lives in `tests.insert_sql` (and `tests.slot_where` for a global table) in `_setup.psql`;
`_fixtures.psql` only loops over the matrix. The README and §11 were corrected; this comment was not.
A developer following it finds nothing to copy, and every file aborts at include time with
`tests.insert_sql: no insert for public.foo`. Fix: one comment line.

### Minor 2: F4 has no planted fault, and its -1 accepts any "permission denied for"

Plan D6's fault table and §8 say F3 and F4 report `executed` under the `read_google_grant` grant. 09 has
no F4 case, and 05 runs F4 through `tests.probe`, which reads any 42501 starting `permission denied for`
as -1. If a later rewrite of `read_google_grant` reads a table its owner lacks rights on and a migration
grants it to `authenticated`, F4 reads -1 ("permission denied for table …") and passes; F1 still fails
the run. Fix: F4 through `tests.call_outcome`, with a 09 case under `f1_fault`, or record it in §11.

### Minor 3: 04's insert and update probes never unmask, though C9 accepts AFTER triggers on service tables

C9 lets an AFTER trigger on any matrix relation through when it is in `tests.unmask_triggers`. That is
sound only for probes that run through `tests.unmasked`; 02 and 03 do, 04 (where `authenticated` probes
the service tables) does not. An invoker AFTER INSERT trigger on `judgments` that bumps `usage_daily`,
plus a stray `for insert to authenticated with check (true)` on `judgments`, would make S2 read -2 from
`usage_daily` and pass; C4 still names the policy. Fix: route S2, S3 and S4 through `tests.unmasked`.

### Minor 4: `tests.view_insert` does not read a 42501's message

`_setup.psql:309` returns `42501` for any `insufficient_privilege`. With `usage` on `admin` and `insert`
on an auto-updatable admin view granted, V3 would get an RLS 42501 and pass; F5 still names the usage
grant. Fix: accept 42501 only for `permission denied for (schema|view|table) %`, and re-raise the rest.

### Minor 5: the same helpers defined three times

The sentinel helper (`tests.a_sentinel`, `tests.own_sentinel`, `tests.t04_sentinel`) and the primary-key
column lookup (`tests.probe_write`, `tests.pk_where`, `tests.own_upsert_sql`) each exist in three copies
whose shapes already differ. A new type category added to one copy makes the others raise (loud, never
a false pass), and 09 proves none of the sentinel copies. Fix: one `tests.sentinel(rel)` and one
`tests.pk_cols(rel)` in `_setup.psql`.

### Minor 6: 01's and 05's check texts are copies of 09's, and nothing pins them equal

C1 to C7, C9, F1, F2 and F5 are each written in 01/05 and again in 09. A later edit that loosens one in
01 and leaves 09 alone keeps 09 green, proving the old text. Fix: move each expression into `_setup.psql`
(as `f3_calls`, `p4_client_policies` and `p5_rls_on` already are), or assert the texts identical.

### Minor 7: two small CI-file nits

- `ci.yml:21` is 169 columns in a header comment wrapped at about 100.
- The `workflows.rs` pin for `cloud-rls` forbids `secrets.` but not a job-level `permissions:` block or
  `github.token` in the job's env. Fix: assert no `permissions:` line, no `github.token`, and only the
  two `SUPABASE_CLI_*` keys under `env:`.

## §8 ledger, row by row (at `7d2482f6`)

| Row | Holds? |
|---|---|
| (1) Where the tests live | Yes. |
| (2) CI, startup time, `auth.uid()` fidelity | Yes. |
| (2) CLI and images pinned | Yes. |
| (2) Required checks report on docs-only PRs | Yes. |
| (2) Required | Pending the owner's Q1: after ten greens. |
| (3) anon gets nothing | Yes. |
| (3) Policy tables, own rows only | Yes. |
| (3) Policy-less tables | Yes (Minor 3 aside). |
| (3) SECURITY DEFINER closed | Yes. F4 is labelled a grant-level deny. |
| (3) Storage | Yes. |
| (3) Views, admin, realtime | Yes. |
| (4) A new table fails | Yes. |
| (5) TDD order | Yes. |
| Every check able to fail | Yes, except F4 (Minor 2). |
| Grant denial told apart from RLS denial | Yes. |
| Impersonation | Yes. |
| `migrations_test.ts` unchanged | Yes. |
| No staging, prod, migration or secret | Yes. |

## Not verified here

- The CI log's F3 diag (7 definer functions); the ledger records 7 and no migration has changed since.
- T11's cargo and Deno runs belong to the main session.
