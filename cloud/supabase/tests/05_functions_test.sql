-- Functions and the admin schema (RLS pgTAP plan, D3 F1 to F5): no client role may call a
-- SECURITY DEFINER function, execute any other function in `public` or `admin` (beyond the RPC
-- allowlist, empty today), or reach `admin`. F2 has no trigger exemption (security-leftovers plan,
-- R3): Postgres does not check EXECUTE when a trigger fires, so a client role needs none on a
-- trigger function, and `20261111000100_client_table_rights.sql` revokes it on the two SECURITY
-- INVOKER trigger functions, `sync_notes_stamp_rev` and `sync_usage_bump`, which were the only
-- client-executable ones. This is the real run against the migrated schema.
--
-- F1, F2 and F5 are the checks `09_self_test_test.sql` first showed failing against their own
-- planted faults, in the same text. F3 goes through `tests.call_outcome`, which reads `denied`
-- only for a 42501 whose message is `permission denied for function ...`, re-raises any other
-- 42501, and reads anything else as `executed`; 09 saw both call forms below read `executed`
-- once EXECUTE was granted. F4 goes through `tests.call_outcome` the same way, and 09 saw its
-- call read `executed` under the same grant.
--
-- F3's set is read from the catalog, never listed by hand (`tests.f3_calls()`, in `_setup.psql`
-- so 09 proves the same text): every SECURITY DEFINER function in `public` or `admin`, extension
-- members excluded, as F1 reads them. Each is called once as anon and once as account A, with one
-- typed null per argument, as `select count(*) from <fn>(...)`. A function whose return type
-- cannot sit in FROM (`trigger`, rejected there at parse time) is called as
-- `do $d$ begin perform <fn>(...); end $d$`, and a procedure as `do $d$ begin call ...; end $d$`.
-- A DO block is first checked for `usage` on its language (`plpgsql`, which PUBLIC holds); a
-- refusal there would be `permission denied for language`, which `tests.call_outcome` re-raises,
-- never counts. Then the executor checks EXECUTE on the function before the call, and a refusal
-- names the function.
--
-- `plan()` is computed from the catalog: F3 is two cases per definer function. The diag names
-- the set's size and its functions (names only); T11 checks the size reads 7.
begin;
\ir _setup.psql
\ir _matrix.psql
\ir _fixtures.psql

-- F3's cases, in a fixed order: each definer function as anon, then as account A.
create function tests.f3_cases() returns setof text language plpgsql as $$
declare r record; w text;
begin
  for r in select c.fn, c.call_sql from tests.f3_calls() c order by c.fn loop
    foreach w in array array['anon', 'A'] loop
      return next is(tests.call_outcome(w, r.call_sql), 'denied',
        format('F3: %s may not call SECURITY DEFINER %s (permission denied for function)',
          case w when 'A' then 'account A' else w end, r.fn));
    end loop;
  end loop;
end $$;

-- 7 fixed cases (F1 and F2 for each client role, F3's non-vacuity, F4, F5), plus two per
-- definer function.
select plan(7 + 2 * (select count(*)::int from tests.f3_calls()));

-- Names and counts only: the definer set F3 calls, and how many go through a DO block.
select diag(format('F3: %s SECURITY DEFINER functions in public and admin, %s via a DO block: %s',
  count(*), count(*) filter (where c.via_do), coalesce(array_agg(c.fn order by c.fn)::text, '{}')))
  from tests.f3_calls() c;

-- F1: no SECURITY DEFINER function in `public` or `admin` is executable by a client role. 09:
-- `read_google_grant` granted to both roles.
select is(
  array(select fn from tests.client_functions('anon') where is_definer order by fn)::text, '{}',
  'F1: no SECURITY DEFINER function in public or admin is executable by anon');
select is(
  array(select fn from tests.client_functions('authenticated') where is_definer
     order by fn)::text, '{}',
  'F1: no SECURITY DEFINER function in public or admin is executable by authenticated');

-- F2: no function outside the RPC allowlist (empty today), a trigger function included, is
-- executable by a client role. 09: `sync_prune` granted to anon; the trigger function
-- `sync_notes_stamp_rev` granted to authenticated; the trigger function `sync_usage_bump` granted
-- to PUBLIC, seen for both roles.
select is(
  array(select c.fn from tests.client_functions('anon') c
     where not exists (select 1 from tests.rpc_allowlist a where a.fn = c.fn)
     order by c.fn)::text, '{}',
  'F2: no function in public or admin outside the RPC allowlist is executable by anon');
select is(
  array(select c.fn from tests.client_functions('authenticated') c
     where not exists (select 1 from tests.rpc_allowlist a where a.fn = c.fn)
     order by c.fn)::text, '{}',
  'F2: no function in public or admin outside the RPC allowlist is executable by authenticated');

-- F3: the catalog names at least one definer function, so the cases below are not vacuous.
select ok((select count(*) from tests.f3_calls()) > 0,
  'F3: the catalog names SECURITY DEFINER functions in public or admin to call');

-- F3: each definer function, called with typed nulls, is refused to anon and to account A by a
-- refusal that names a function. 09: `read_google_grant` (the select form) and `handle_new_user`
-- (the DO form) granted to both roles.
select * from tests.f3_cases();

-- F4: account A naming account B's grant (`tests.f4_call()`, in `_setup.psql` so 09 proves the
-- same text). A grant-level deny like F3, not an account-scoping proof: `read_google_grant` has
-- no `auth.uid()` binding, and its revoke is the whole defence (r1 M3). Through
-- `tests.call_outcome`, as F3, so only a refusal that names a function reads `denied`. 09: the
-- same call under `read_google_grant` granted to both roles reads `executed`.
select is(tests.call_outcome('A', tests.f4_call()), 'denied',
  'F4: account A may not call public.read_google_grant naming account B''s Gmail grant');

-- F5: no API role holds `usage` on `admin`. 09: the grant to each of the three roles.
select is(
  (has_schema_privilege('anon', 'admin', 'usage')
   or has_schema_privilege('authenticated', 'admin', 'usage')
   or has_schema_privilege('service_role', 'admin', 'usage')),
  false,
  'F5: anon, authenticated and service_role hold no usage on schema admin');

select * from finish();
rollback;
