PASS

# Pre-push review: `security/leftovers` (`0926f0c1..c4b1f471`, eight commits)

**Worktree:** `knowlu-leftovers`. **Plan:** `docs/plans/2026-11-11-security-leftovers-plan.md`, with plan reviews r1 and r2.
**Reviewer:** `reviewer`, 2026-11-11.

**Verdict: PASS.** I found nothing Critical or Important, and four Minor items. None of the Minor items opens a client path.

- **Contract list:** the branch touches no file on it and no frozen reference. Line 1 stands.
- **Deno:** I re-ran CI's Deno line on `c4b1f471` with a clean tree: **1561 passed, 0 failed**.
- **Line endings:** every changed file is `i/lf w/lf`, and `git diff --check` is clean.

## Findings

### Minor

**m1. Some grant and revoke forms Postgres accepts still get past the static guard.**
Location: `cloud/supabase/migrations/migrations_test.ts:633` (`granteesOf`), `:681-686` (`ident`), `:737` (`schema`).
F1 and F2 in pgTAP still read the real privileges, so a false pass here only moves the failure from Deno to `cloud-rls`, which is not a required check. Each scenario below passes the guard on a definer `public.f(uuid)`:

- **`group` before a grantee.** `grant execute on function f(uuid) to group anon;` is still valid grammar in PG 17 (`grantee: [GROUP] RoleSpec`). `granteesOf` yields `"group anon"`, which is not `anon`, so the guard sees no re-grant.
- **A name `\w+` cannot read.** In `grant execute on function f(uuid), "g-h"(int) to anon;`, `ident` fails on `"g-h"`. `routineRight` then returns `undefined` for the whole statement, so the re-grant to `f` is dropped. A `$` inside a name (`f$x`) does the same.
- **A revoke in another schema.** The object list drops schema qualifiers, so `revoke execute on function admin.f(uuid) from public, anon, authenticated;` counts as a qualifying revoke for `public.f`. This is the T1b ruling, and the false pass it costs is real only if both schemas hold a function with that name.
- **The schema is assumed from the text.** An unqualified definition is taken to be in `public`. After `set search_path = admin`, a later `grant execute on all functions in schema admin to anon` is not counted.
- **Quoted mixed case.** `"F"` and `f` compare equal after lower-casing, although Postgres treats them as two functions.

Nothing in today's corpus uses any of these forms, and the plan does not require them. *Recommended:* accept them for this lane and name them beside §8 O5. If one more clause is wanted, the cheap one is to strip a leading `group ` in `granteesOf`.

**m2. Nothing on record shows that a WARN-level lint fails the exact advisor command.**
Location: `.github/workflows/ci.yml:340`; plan F-B (`plan:27`).
- The evidence shows two things:
  - `No issues found` with exit 0 on the migrated schema.
  - Exit 1 for a planted RLS-off table. That is `rls_disabled_in_public`, an ERROR-level lint, and F-B does not record which flags that run used.
- The `workflows.rs` pin locks the flags, not what they do.
- *Scenario:* CLI 3.1.0's `--fail-on warn` treats a WARN differently than its help text says. A migration then adds a function without `set search_path` (`function_search_path_mutable`, WARN), and the step stays green.
- *Recommended:* at C2, or at any local run before C5, plant one WARN lint and run R1's exact line once, for example `create function public.t() returns int language sql as 'select 1';` inside a scratch `db reset`. Record the exit code.

**m3. A new trigger function now fails F2, but neither the README nor the static guard says so.**
Location: `cloud/supabase/README.md:183` (the "A new trigger" bullet), and `migrations_test.ts:510-521` (`assertExecuteRevoked` exempts triggers by kind).
- `postgres`'s function default ACL still grants EXECUTE to both client roles (F-C, out of scope in §7).
- F2 no longer exempts trigger functions.
- *Scenario:* a later lane adds a SECURITY INVOKER trigger function. Deno passes, because the trigger is exempt there. `cloud-rls` then fails by name in 05's F2, and the author has to find out from the failure that the function needs `revoke execute … from public, anon, authenticated`.
- The failure is loud, so the cost is one round trip.
- *Recommended:* add one clause to the README bullet: a new trigger function revokes EXECUTE from `public`, `anon` and `authenticated`, or F2 fails.

**m4. The plan's status line is stale.**
Location: `docs/plans/2026-11-11-security-leftovers-plan.md:3`.
- It still reads "**Status: r1, for plan review r2.**", while the closing paragraph records the fixes made after r2.
- *Recommended:* change it to "r2, executed", or similar, before the plan goes public with the PR.

## The questions in the dispatch

### Correctness of the checks

- **C10** (`01:120-129`; 09's copy at `:265-285` is the same text):
  - It reads `has_table_privilege` for each relkind `r p v m f` in `public`. That function counts PUBLIC and role membership.
  - It names relations from `pg_namespace`.
  - Each of these is caught by name: a later `grant all on public.x to authenticated`, a grant to PUBLIC, and a new table created through a re-added default.
  - It reads table-level rights only. Column-level `REFERENCES` is the one column-level right among the four. It has no use without CREATE, and the approved scope is table rights.
- **C11** (`01:138-147`, `09:295-324`):
  - It reads both `postgres` entries, the schema one and the global one (`0::oid`), and both client roles plus PUBLIC.
  - The global plant starts from the built-in defaults, so it yields exactly `{PUBLIC:TRIGGER}`.
  - The `\if` fallback keeps `plan(174)` exact either way: two `is` or two `fail`.
  - `'t'` and `'f'` are valid `\if` values.
- **F2** (`05:66-79`, `09:227-256`):
  - Dropping `not c.is_trigger` is the whole change.
  - The three new plants tell apart a grant to one role and a grant to PUBLIC.
- **Counts:** 01 goes to 23 fixed cases, and 09 gains 10 (F2 3, C10 4, C11 3). 858 + 12 = 870, which matches the controller's run.

### Migration safety (`20261111000100_client_table_rights.sql`)

- **Coverage of the revoke:** `on all tables in schema public` reaches tables, partitioned tables, views, materialized views and foreign tables.
- **Hosted projects:** the controller found `postgres` owns every `public` table there and is the grantor of the defaults, so the revoke takes effect. A revoke by a non-grantor only warns, and C10 would name what it left.
- **What the clients still have:** select, insert, update and delete are untouched, and so are `service_role` and sequences. PostgREST and pg_graphql need only the four verbs. Foreign-key checks run as the table's owner, so they never need REFERENCES.
- **The triggers:** the trigger manager does not check EXECUTE. The masking cases in 09 (`-2` rather than `-1`) are what would show it if it did, and they are green at 870/870.
- **Versions:** `maintain` needs PG 17. `config.toml` has `major_version = 17`, and the hosted projects run 17.6.
- **The pin test:** the migration's own test pins exactly R2's four statements, no function created or replaced, no `grant`, and no `models` in any case. The word is built in halves so the test file does not trip the eval gate.

### The guard and the cloud engineer's T1b rulings

These hold as built:

| Ruling | Where |
|---|---|
| A space is kept after `)` and `]` | `:615`, the second `replace` |
| `all functions` or `all routines in schema X` counts only for the definer's own schema | `names`, through `allIn.includes(schema)` |
| `revoke grant option for` does not qualify; the inner `grant option for …` reads as `execute: false` | `:666` |
| `'…'` literals and `"…"` identifiers are skipped in the balanced scan and the top-level split | `balancedParens`, `splitTopLevel` |
| Schema qualifiers are dropped from object names, so `public.f(uuid) to service_role` is not a re-grant | `ident` |

Other properties I checked:
- **Fails closed:** an unreadable signature on a matching name throws, naming the function.
- **Statement words:** `\bgrant\b` does not match inside `store_google_grant`, `granted by` or `with grant option`.
- **Search range:** the search starts at the definition's offset.
- **Only the matching name is parsed:** `names` short-circuits on the name before it parses another function's arguments.

The real false passes are m1. The false failures are a missing alias (`int2`, `float4`, `decimal`, `timetz`), `revoke … on all functions`, `from … group anon`, and a parameter named like a multi-word type start (`time timestamptz`). Each of them is loud and in the safe direction.

### The CI pin and the step

- The new test reads the same comment-stripped `code` lines and the same `"\n      - "` step split as the shape test.
- **The comment avoids every needle:** no `db advisors`, no `scripts/ci/`, no `== 'true'` and no `continue-on-error` in the comment or the step name.
- **The step meets the shape rules:** one `if:` holding `code != 'false'`, and `--workdir cloud`.
- **Placement:** it runs after `test db` and before `verify`, so `verify` (`always()`) still judges any image it pulled.
- **No vacuous pass:** with nothing after the flags, a skip only on the docs-only path pgTAP shares, and no `continue-on-error` anywhere in the job, the step can pass without running the lints only through m2.
- **Not yet required:** `cloud-rls` is still not a required check (§7, out of scope).

### CLAUDE.md

- **The two overriding rules:**
  - The README and `toolchain.md` give `SUPABASE_DB_PORT` only as an example override, so the branch adds no single-user assumption.
  - The scan for personal paths, project refs and keys was clean.
  - No frozen reference is touched.
- **Gates and conventions:**
  - Line endings are LF throughout.
  - No `uses:` line was added, so SHA pins are unaffected.
  - No test is newly ignored.
  - No test leaves the machine; `NO_PROXY` strengthens that rule.
  - The new Rust has no visible unused bindings. Zero warnings is the controller's C1 check.
- **`NO_PROXY` and ureq 3.4.0:** `NoProxy::try_from_env` reads `NO_PROXY` before `no_proxy`, and no other Rust code reads a proxy variable. So the forced upper-case value also wins over a developer's lower-case `no_proxy` on Linux, and it covers the Windows registry proxy path.

### Public wording

- **The Fixed section** of the security note describes only what this branch fixes, in neutral terms, and it goes public only after C4 (R7).
- **The production WARN** is recorded as R6 rules: an Auth setting, left to the HANDOFF row that already names it publicly. *(One sentence removed by the controller before commit: it stated a reason R6 rules out of the public record.)*
- **Plan §7 "Sequences":** this points at no open weakness. Client inserts and the SECURITY INVOKER `sync_notes_stamp_rev` need sequence USAGE, and no client route reaches `setval`.
- **F-J and O5** name gaps in a static test that F2 covers at run time; they are not a client path.
- **Commit messages** are factual.

### The fidelity ledger

| Item | Closed by the branch | Still open (the controller's close) |
|---|---|---|
| (1) The advisor in CI | Pin red, then green, then red on four planted edits; equal image lists | The PR's advisor and `verify` steps (C5); m2 |
| (1) Production's first advisor read | Recorded in the security note, dated 2026-11-11 | C4's re-read |
| (2a) Table rights and their defaults | C10 and C11 red, then green; 09's plants, one of them global; the migration test; the local queries | C2 and C4 |
| (2a) EXECUTE on the trigger functions | F2 red, then green; three plants | C2's live `sync-push` check |
| (2b) Loopback tests and a proxy | Test red, then green; the manual proof, also with `NO_PROXY=example.org` | Nothing |
| (2c) A later re-grant, and PUBLIC required | Cases 6, 8, 1 and 2; `leaky_trigger` still throws | Nothing |
| (2c) Overloads | Cases 3, 4, 5 and 7; corpus 1561/0 with the 51-function and 10-view pins unchanged | Nothing |
