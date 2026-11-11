-- Coverage (RLS pgTAP plan, D3 C1 to C8, D4; C9 from the T6b and T7 reviews; C10 and C11 from the
-- security-leftovers plan, R3): every relation in `public` and `admin` is in the matrix, and each
-- one's row-level security, policies, view options, client grants, fixture rows and triggers are
-- exactly what the matrix and the probes assume; no client role holds a table right outside the
-- four verbs, and no default privilege hands one to a table created later. This is the real run
-- against the migrated schema.
--
-- Every check is the expression `09_self_test_test.sql` first showed failing against its own
-- planted fault, kept here in the same text: each is a psql variable (`\gset`) and `:x` runs it as
-- the owner. A check reads as a sorted `text[]` of strays, so a failure names each relation (and
-- role, verb or slot), never a row's contents. C7's validity domain is C7's count domain (every
-- matrix table), where 09 ran the same probe over the service tables only (T5c ruling).
--
-- `plan()` is computed from `tests.matrix`: one C7 validity case per slot (a table with an account
-- key has two, A's and B's; a global table one), so a matrix row added or dropped moves the count.
-- C8 reads what a client role sees into psql variables and asserts it after `as_owner()`, so
-- nothing pgTAP writes runs under a client role, and compares account ids inside `ok()`.
begin;
\ir _setup.psql
\ir _matrix.psql
\ir _fixtures.psql

-- C7's validity cases, one per slot, in a fixed order: each runs the slot's `tests.insert_sql` in
-- its `tests.clear_sql` slot as the owner, through the driver, inside `lives_ok` (r2 note 7), so a
-- bad row (a 23514, a 23505, a parent that is not there) fails its own case by name. The helpers
-- are called inside the case's SQL, so an `insert_sql` that raises is caught there too. The child's
-- insert text is built before the driver clears its slot, and each probe clears only its own table.
create function tests.c7_validity() returns setof text language plpgsql as $$
declare r record;
begin
  for r in
    select m.rel, w.who, case when m.key_col is null then 'global' else w.who end as label
      from tests.matrix m
     cross join (values ('A'), ('B')) w(who)
     where m.kind = 'table' and (m.key_col is not null or w.who = 'A')
     order by m.rel, w.who
  loop
    return next lives_ok(
      format('select tests.probe(%L, tests.insert_sql(%L, %L), tests.clear_sql(%L, %L))',
        'owner', r.rel, r.who, r.rel, r.who),
      format('C7 validity: the owner''s insert into %s for %s runs in its cleared slot',
        r.rel, r.label));
  end loop;
end $$;

-- 23 fixed cases (C1 to C6, C10 and C11 one each, C7's count and its validity aggregate, C8's
-- twelve, C9), plus one per C7 slot.
select plan(23 + (
  select count(*)::int
    from tests.matrix m
   cross join (values ('A'), ('B')) w(who)
   where m.kind = 'table' and (m.key_col is not null or w.who = 'A')));

-- Counts only: what the matrix holds, and so what the plan above was computed from.
select diag(format('matrix: %s tables (%s own, %s service), %s views, %s C7 slots',
  count(*) filter (where m.kind = 'table'),
  count(*) filter (where m.access = 'own'),
  count(*) filter (where m.access = 'service'),
  count(*) filter (where m.kind = 'view'),
  count(*) filter (where m.kind = 'table' and m.key_col is not null) * 2
    + count(*) filter (where m.kind = 'table' and m.key_col is null)))
  from tests.matrix m;

-- C1 (D4): no relation in `public` or `admin` lacks a matrix row, and no matrix row lacks its
-- relation. 09: the `zz_unlisted` table, the `admin.zz_unlisted_v` view and the `zz_ghost` row.
select $c$array(select rel from tests.unlisted() order by rel)::text$c$ as c1 \gset
select is(:c1, '{}',
  'C1: every relation in public and admin has a matrix row, and every row its relation');

-- C2: every matrix table has row-level security on. 09: `issues` with it disabled.
select $c$array(select m.rel from tests.matrix m
   join pg_class c on c.oid = m.rel::regclass
   where m.kind = 'table' and not c.relrowsecurity order by 1)::text$c$ as c2 \gset
select is(:c2, '{}', 'C2: every matrix table has row-level security on');

-- C3: each owner-read table carries exactly its matrix policies, each a permissive select for
-- authenticated whose `qual` is exactly `(<key> = auth.uid())`, with no `with_check`. 09:
-- `issues_select_own` altered to `using (true)`.
select $c$array(select m.rel from tests.matrix m
   where m.access = 'own'
     and (array(select p.policyname::text from pg_policies p
                 where p.schemaname || '.' || p.tablename = m.rel order by 1)
            is distinct from array(select x from unnest(m.policies) x order by 1)
          or exists (select 1 from pg_policies p
                      where p.schemaname || '.' || p.tablename = m.rel
                        and (p.cmd <> 'SELECT' or p.roles <> '{authenticated}'::name[]
                             or p.permissive <> 'PERMISSIVE'
                             or p.qual is distinct from format('(%I = auth.uid())', m.key_col)
                             or p.with_check is not null)))
   order by 1)::text$c$ as c3 \gset
select is(:c3, '{}',
  'C3: every owner-read table carries exactly its policy: select, authenticated, its own key');

-- C4: every service table carries exactly its matrix policies, which is none. 09: a select
-- policy planted on `judgments`.
select $c$array(select m.rel from tests.matrix m
   where m.access = 'service'
     and array(select p.policyname::text from pg_policies p
                where p.schemaname || '.' || p.tablename = m.rel order by 1)
           is distinct from array(select x from unnest(m.policies) x order by 1)
   order by 1)::text$c$ as c4 \gset
select is(:c4, '{}', 'C4: no service-only table carries a policy');

-- C5: a view's `security_invoker` matches its access, and no closed view grants a client select.
-- 09: `billing_subscribers` set to run as its owner; `monthly_spend` granted to both roles.
select $c$array(select m.rel from tests.matrix m
   where m.kind = 'view'
     and (coalesce((select o.option_value::boolean
                      from pg_class c
                      cross join lateral pg_options_to_table(c.reloptions) o
                     where c.oid = m.rel::regclass and o.option_name = 'security_invoker'),
                   false)
            is distinct from (m.access = 'invoker_view')
          or (m.access = 'closed_view'
              and (has_table_privilege('anon', m.rel, 'select')
                   or has_table_privilege('authenticated', m.rel, 'select'))))
   order by 1)::text$c$ as c5 \gset
select is(:c5, '{}',
  'C5: every view''s security_invoker matches the matrix, and no closed view grants a select');

-- C6: each table's four client verbs, for both client roles, match the matrix's `grants`. 09:
-- select revoked from authenticated on `issues`; select granted on the revoked `mail_accounts`.
select $c$array(select m.rel || ':' || r.role || ':' || v.verb
   from tests.matrix m
   cross join (values ('anon'), ('authenticated')) r(role)
   cross join (values ('select'), ('insert'), ('update'), ('delete')) v(verb)
   where m.kind = 'table'
     and has_table_privilege(r.role::name, m.rel, v.verb)
           is distinct from (m.grants = 'default')
   order by 1)::text$c$ as c6 \gset
select is(:c6, '{}',
  'C6: every table''s select, insert, update and delete for anon and authenticated match grants');

-- C10: no relation in `public` (a table, a partitioned table, a view, a materialized view or a
-- foreign table) lets anon or authenticated truncate, reference, trigger or maintain it. TRUNCATE
-- skips row-level security, so on an owner-read table it is the dangerous one; MAINTAIN is PG17's
-- member of the same class. C6 checks the four verbs and nothing else, so these four are read here,
-- for every relation, not only the matrix's. `has_table_privilege` counts PUBLIC, so a grant to
-- PUBLIC is named for both roles. A hit is `schema.relation:role:privilege`, the schema read from
-- `pg_namespace`: a `regclass` cast follows `search_path` and would print `issues`, not
-- `public.issues`. 09: truncate on `issues` to authenticated; maintain on the revoked
-- `mail_accounts` to anon; references and trigger on `judgments` to PUBLIC.
select $c$array(select n.nspname || '.' || c.relname || ':' || r.role || ':' || p.priv
   from pg_class c
   join pg_namespace n on n.oid = c.relnamespace
   cross join (values ('anon'), ('authenticated')) r(role)
   cross join (values ('truncate'), ('references'), ('trigger'), ('maintain')) p(priv)
   where n.nspname = 'public' and c.relkind in ('r', 'p', 'v', 'm', 'f')
     and has_table_privilege(r.role::name, c.oid, p.priv)
   order by 1)::text$c$ as c10 \gset
select is(:c10, '{}',
  'C10: anon and authenticated hold no truncate, references, trigger or maintain on a public relation');

-- C11: `postgres`'s default privileges on tables give anon, authenticated and PUBLIC nothing
-- beyond select, insert, update and delete, so a table a later migration creates does not start
-- with C10's four rights. Two entries apply to a table `postgres` creates in `public`: the schema's
-- and the global one (`defaclnamespace = 0`); the global one adds to the schema's, and no
-- per-schema revoke removes it, so both are read. PUBLIC is grantee 0 and is named `PUBLIC`. A hit
-- is `grantee:privilege`; a right both entries carry is named twice. 09: truncate on tables granted
-- to anon in the schema entry; trigger on tables granted to PUBLIC in a global entry.
select $c$array(select coalesce(nullif(a.grantee::regrole::text, '-'), 'PUBLIC') || ':' || a.privilege_type
   from pg_default_acl d
   cross join lateral aclexplode(d.defaclacl) a
   where d.defaclrole = 'postgres'::regrole::oid and d.defaclobjtype = 'r'
     and d.defaclnamespace in (0::oid, 'public'::regnamespace::oid)
     and a.grantee in (0::oid, 'anon'::regrole::oid, 'authenticated'::regrole::oid)
     and a.privilege_type not in ('SELECT', 'INSERT', 'UPDATE', 'DELETE')
   order by 1)::text$c$ as c11 \gset
select is(:c11, '{}',
  'C11: postgres''s table defaults give anon, authenticated and PUBLIC only the four verbs');

-- C7: the fixtures are not vacuous. Each table with an account key holds a row for A and a row
-- for B, and each global table its one row, in the slot `tests.slot_where` names. 09: B's
-- `issues` row, B's `judgments` row, A's mailbox `mail_batches` rows and the `models` task pin,
-- each deleted.
select $c$array(select m.rel || ':' || case when m.key_col is null then 'global' else w.who end
   from tests.matrix m
   cross join (values ('A'), ('B')) w(who)
   left join tests.accounts a on a.who = w.who
   where m.kind = 'table' and (m.key_col is not null or w.who = 'A')
     and case when a.id is null then true
              else (xpath('/row/n/text()', query_to_xml(format(
                      'select count(*) as n from %s where %s', m.rel,
                      tests.slot_where(m.rel, w.who)), false, true, '')))[1]::text::bigint = 0
         end
   order by 1)::text$c$ as c7 \gset
select is(:c7, '{}',
  'C7: every keyed table holds a row for A and for B, and every global table its row');

-- C7's validity, slot by slot: no owner insert raises in its cleared slot.
select * from tests.c7_validity();

-- C7's validity, aggregate: each owner insert into its cleared slot reports exactly one row. 09's
-- `svc_valid` predicate (seen failing under a `do instead nothing` rule on `judgments`; its 23514
-- guard rail shows a bad row raises, never counts), over C7's domain of every matrix table. It
-- runs after the cases above, which have shown none of these probes raises. It does not check
-- that the row matches `tests.slot_where` (T6a review M2, a T10 fidelity note).
select $c$array(select m.rel || ':' || w.who
   from tests.matrix m
   cross join (values ('A'), ('B')) w(who)
   where m.kind = 'table' and (m.key_col is not null or w.who = 'A')
     and tests.probe('owner', tests.insert_sql(m.rel, w.who), tests.clear_sql(m.rel, w.who)) <> 1
   order by 1)::text$c$ as c7_valid \gset
select is(:c7_valid, '{}',
  'C7 validity: as the owner, every table''s insert into its cleared slot reports one row');

-- C9 (T6b and T7 reviews): every non-internal trigger on a matrix relation, which is every
-- relation 02 or 03 probes, is either one the insert, upsert and update probes disable
-- (`tests.unmask_triggers`: row-level, AFTER, on INSERT or UPDATE) or on the stamp-only allowlist
-- (`tests.stamp_only`), there by relation, name, function and the function's `md5(prosrc)`, and
-- still BEFORE and row-level (tgtype bits 2 and 1). A new trigger, a masking one rewritten so the
-- unmask set no longer holds it, or a stamp function whose body changed, is named here rather
-- than left on to mask a probe; the body pin is deliberate, and a real change to the function
-- updates it in the same commit. 09: `sync_records_usage` rewritten as BEFORE;
-- `sync_notes_rev_stamp` repointed at `sync_usage_bump`; `sync_notes_stamp_rev`'s body changed;
-- a statement-level AFTER INSERT trigger on `issues`.
select $c$array(select m.rel || ':' || t.tgname
   from tests.matrix m
   join pg_trigger t on t.tgrelid = m.rel::regclass
   where not t.tgisinternal
     and t.tgname::text <> all (tests.unmask_triggers(m.rel))
     and not exists (select 1 from tests.stamp_only s
                      where s.rel = m.rel and s.tgname = t.tgname::text
                        and t.tgfoid = s.fn::regprocedure and (t.tgtype::int & 3) = 3
                        and (select md5(p.prosrc) from pg_proc p where p.oid = t.tgfoid)
                              = s.src_md5)
   order by 1)::text$c$ as c9 \gset
select is(:c9, '{}',
  'C9: every trigger on a probed relation is unmasked by the write probes or stamp-only');

-- C8: the impersonation every probe runs under (`tests.as_who`, D5) is PostgREST's. What a client
-- role sees is read into psql variables, then asserted as the owner; account ids are compared
-- inside `ok()` and never printed. 09's guard: a stale `request.jwt.claim.sub` naming B alone
-- makes `auth.uid()` B, and a following session for A still makes it A.
select tests.as_who('A');
select coalesce(auth.uid()::text, '<null>') as c8_a_uid, current_user as c8_a_role \gset
select tests.as_owner();
select ok(:'c8_a_uid' = (select id::text from tests.accounts where who = 'A'),
  'C8: auth.uid() is account A in a session for A');
select is(:'c8_a_role'::text, 'authenticated', 'C8: current_user is authenticated in A''s session');

select tests.as_who('B');
select coalesce(auth.uid()::text, '<null>') as c8_b_uid \gset
select tests.as_owner();
select ok(:'c8_b_uid' = (select id::text from tests.accounts where who = 'B'),
  'C8: auth.uid() is account B in a session for B');

select set_config('request.jwt.claim.sub', (select id::text from tests.accounts where who = 'B'),
  true) is not null as c8_set \gset
select coalesce(auth.uid()::text, '<null>') = (select id::text from tests.accounts where who = 'B')
  as c8_stale_live \gset
select tests.as_who('A');
select coalesce(auth.uid()::text, '<null>') as c8_seen \gset
select tests.as_owner();
select ok(:'c8_stale_live'::boolean,
  'C8 guard: a stale request.jwt.claim.sub naming B alone makes auth.uid() B');
select ok(:'c8_seen' = (select id::text from tests.accounts where who = 'A'),
  'C8 guard: a session for A wins over a stale request.jwt.claim.sub naming B');

select tests.as_who('anon');
select coalesce(auth.uid()::text, '<null>') as c8_anon_uid, current_user as c8_anon_role \gset
select tests.as_owner();
select ok(:'c8_anon_uid' = '<null>', 'C8: auth.uid() is null for anon');
select is(:'c8_anon_role'::text, 'anon', 'C8: current_user is anon for anon');

select tests.as_who('nosub');
select coalesce(auth.uid()::text, '<null>') as c8_nosub_uid, current_user as c8_nosub_role \gset
select tests.as_owner();
select ok(:'c8_nosub_uid' = '<null>', 'C8: auth.uid() is null in a session with no sub');
select is(:'c8_nosub_role'::text, 'authenticated',
  'C8: current_user is authenticated in a session with no sub');

select tests.as_who('stranger');
select coalesce(auth.uid()::text, '<null>') as c8_stranger_uid \gset
select tests.as_owner();
select ok(:'c8_stranger_uid' <> '<null>'
          and :'c8_stranger_uid' not in (select id::text from tests.accounts),
  'C8: auth.uid() is a sub no fixture account has in a stranger''s session');

select tests.as_who('A');
select tests.as_owner();
select ok(auth.uid() is null, 'C8: auth.uid() is null after as_owner, from A''s session');
select is(current_user::text, session_user::text,
  'C8: current_user is the session user after as_owner');

select * from finish();
rollback;
