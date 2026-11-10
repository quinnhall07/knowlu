-- Functions and the admin schema (RLS pgTAP plan, D3 F1 to F5): no client role may execute a
-- function in `public` or `admin` (beyond the RPC allowlist), call a SECURITY DEFINER function,
-- or reach `admin`. This is the real run against the migrated schema.
--
-- F1, F2 and F5 are the checks `09_self_test_test.sql` first showed failing against their own
-- planted faults, in the same text. F3 and F4 go through `tests.probe`, so -1 is a refusal by a
-- privilege (09 showed the definer call refused through `tests.call_outcome`).
--
-- F3's set is read from the catalog, never listed by hand: every SECURITY DEFINER function in
-- `public` or `admin`, extension members excluded, as F1 reads them. Each is called once as anon
-- and once as account A, with one typed null per argument (`tests.f3_calls()`). The call is
-- `select count(*) from <fn>(...)`, the one read form `tests.run` accepts. A function whose
-- return type cannot sit in FROM (a pseudo-type other than `void`, so `trigger`, which Postgres
-- rejects there at parse time, before any privilege check) is called as
-- `do $d$ begin perform <fn>(...); end $d$`, and a procedure as `do $d$ begin call ...; end $d$`.
-- Either way the executor checks EXECUTE before the call, and a refusal reads as -1.
--
-- `plan()` is computed from the catalog: F3 is two cases per definer function. The diag names
-- the set's size and its functions (names only); T11 checks the size reads 7.
begin;
\ir _setup.psql
\ir _matrix.psql
\ir _fixtures.psql

-- F3's calls. `fn` names the function with its argument types, schema included; `call_sql` is
-- the probe text; `via_do` marks a call made through a DO block. A STRICT function called with
-- a null would be folded to null at plan time and never checked, which reads as a count, not -1,
-- so such a function fails its case loud rather than pass.
create function tests.f3_calls() returns table (fn text, call_sql text, via_do boolean)
language sql stable as $$
  select n.nspname || '.' || p.proname || '(' || oidvectortypes(p.proargtypes) || ')',
         case
           when p.prokind = 'p' then
             format('do $d$ begin call %I.%I(%s); end $d$', n.nspname, p.proname, a.nulls)
           when t.typtype = 'p' and p.prorettype <> 'void'::regtype then
             format('do $d$ begin perform %I.%I(%s); end $d$', n.nspname, p.proname, a.nulls)
           else format('select count(*) from %I.%I(%s)', n.nspname, p.proname, a.nulls)
         end,
         p.prokind = 'p' or (t.typtype = 'p' and p.prorettype <> 'void'::regtype)
    from pg_proc p
    join pg_namespace n on n.oid = p.pronamespace
    join pg_type t on t.oid = p.prorettype
   cross join lateral (
     select coalesce(string_agg(format('null::%s', format_type(x.typ, null)), ', '
                                order by x.ord), '') as nulls
       from unnest(p.proargtypes::oid[]) with ordinality as x(typ, ord)) a
   where n.nspname in ('public', 'admin')
     and p.prosecdef
     and not exists (
       select 1 from pg_depend d
        where d.classid = 'pg_proc'::regclass and d.objid = p.oid and d.deptype = 'e')
   order by 1
$$;

-- F3's cases, in a fixed order: each definer function as anon, then as account A.
create function tests.f3_cases() returns setof text language plpgsql as $$
declare r record; w text;
begin
  for r in select c.fn, c.call_sql from tests.f3_calls() c order by c.fn loop
    foreach w in array array['anon', 'A'] loop
      return next is(tests.probe(w, r.call_sql), -1::bigint,
        format('F3: %s may not call SECURITY DEFINER %s (refused by a privilege, -1)',
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

-- F2: no function that is not a trigger function, and not on the RPC allowlist (empty today), is
-- executable by a client role. 09: `sync_prune` granted to anon.
select is(
  array(select c.fn from tests.client_functions('anon') c
     where not c.is_trigger
       and not exists (select 1 from tests.rpc_allowlist a where a.fn = c.fn)
     order by c.fn)::text, '{}',
  'F2: no function in public or admin outside the RPC allowlist is executable by anon');
select is(
  array(select c.fn from tests.client_functions('authenticated') c
     where not c.is_trigger
       and not exists (select 1 from tests.rpc_allowlist a where a.fn = c.fn)
     order by c.fn)::text, '{}',
  'F2: no function in public or admin outside the RPC allowlist is executable by authenticated');

-- F3: the catalog names at least one definer function, so the cases below are not vacuous.
select ok((select count(*) from tests.f3_calls()) > 0,
  'F3: the catalog names SECURITY DEFINER functions in public or admin to call');

-- F3: each definer function, called with typed nulls, is refused to anon and to account A.
select * from tests.f3_cases();

-- F4: account A naming account B's grant. A grant-level deny like F3, not an account-scoping
-- proof: `read_google_grant` has no `auth.uid()` binding, and its revoke is the whole defence
-- (r1 M3). B's id is read as the owner and spliced as a literal; it is never printed here.
select is(
  tests.probe('A', format('select count(*) from public.read_google_grant(%L::uuid, %L)',
    (select a.id from tests.accounts a where a.who = 'B'),
    'https://www.googleapis.com/auth/gmail.readonly')),
  -1::bigint,
  'F4: account A may not call public.read_google_grant naming account B''s Gmail grant (-1)');

-- F5: no API role holds `usage` on `admin`. 09: the grant to each of the three roles.
select is(
  (has_schema_privilege('anon', 'admin', 'usage')
   or has_schema_privilege('authenticated', 'admin', 'usage')
   or has_schema_privilege('service_role', 'admin', 'usage')),
  false,
  'F5: anon, authenticated and service_role hold no usage on schema admin');

select * from finish();
rollback;
