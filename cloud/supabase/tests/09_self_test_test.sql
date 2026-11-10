-- D6's self-tests (RLS pgTAP plan, T5a to T5c): every check the suite makes is first shown to see
-- its own planted fault. A deny test against a correct schema is green on its first run, so this
-- file is where "red first" lives.
--
-- The frame:
-- - Each fault goes through `tests.under_fault(fault, check)`. It runs the fault as the owner, then
--   the check, inside one subtransaction it always rolls back, and returns the check's value as
--   text. The pgTAP assertion runs after that rollback, as the owner, so no fault outlives its case
--   and nothing pgTAP writes runs under a client role.
-- - A check is an owner-side SQL expression, kept once in a psql variable (`\gset`): `:'x'` hands
--   its text to `under_fault`, and `:x` runs the very same text directly as the control.
-- - Each control runs after its fault cases. It proves the check is clean without the fault, so
--   the check tells the two apart, and that `under_fault` rolled the fault back.
-- - Name sets read as sorted `text[]` literals, so a failure names each stray. Nothing here prints
--   an account id: the two C8 UUIDs are fresh and compared inside `ok()`.
--
-- T5a writes the cases that need no fixture rows; T5b and T5c append the rest.
begin;
\ir _setup.psql
\ir _matrix.psql

select plan(102);

-- The platform facts P1 to P3 depend on: no migration creates a bucket, a storage policy or a
-- publication, so whatever these print is the image's own. Names and counts only.
select diag('storage schema: ' || case when exists (
  select 1 from pg_namespace where nspname = 'storage') then 'present' else 'absent' end);
select diag('storage policies: '
  || array(select policyname::text from pg_policies where schemaname = 'storage' order by 1)::text);
select diag('publications: ' || array(select pubname::text from pg_publication order by 1)::text);
select exists (select 1 from pg_namespace where nspname = 'storage') as has_storage \gset

-- Guard rail, C8: a stale legacy key never shadows the claims (T3 review, Minor 2). Two fresh
-- accounts that exist nowhere; neither id is printed.
select gen_random_uuid()::text as c8_a, gen_random_uuid()::text as c8_b \gset
select set_config('request.jwt.claim.sub', :'c8_b', true) is not null as c8_set \gset
select coalesce(auth.uid()::text, '<null>') = :'c8_b' as c8_stale_live \gset
select tests.as_user(:'c8_a'::uuid);
select coalesce(auth.uid()::text, '<null>') as c8_seen \gset
select tests.as_owner();
select ok(:'c8_stale_live'::boolean,
  'C8 guard: the planted stale request.jwt.claim.sub alone makes auth.uid() that other account');
select ok(:'c8_seen' = :'c8_a',
  'C8 guard: as_user(A) wins over a stale request.jwt.claim.sub naming another account');

-- Guard rails: a probe that breaks fails loud. It is never read as 0, -1 or -2 (D3, r1 I2).
select throws_ok(
  $t$select tests.run('select count(*) from public.issues where zz_no_such_column is null')$t$,
  '42703', null,
  'guard: tests.run raises on a broken read statement; it returns no count');
select throws_ok(
  $t$select tests.probe('anon', 'update public.issues set zz_no_such_column = 1')$t$,
  '42703', null,
  'guard: a probe given a broken write statement raises through the driver, under anon');
-- A read that is not `select count(*)` must not fall through to the write branch, whose row
-- count of a select would read as rows seen. A count, or a write, is the statement's first word.
select throws_ok(
  $t$select tests.run('select count(1) from public.issues')$t$,
  '22023', 'tests.run: not a count read or a write statement',
  'guard: tests.run refuses select count(1) rather than read its row count');
select throws_ok(
  $t$select tests.run('with t as (select 1) select count(*) from t')$t$,
  '22023', 'tests.run: not a count read or a write statement',
  'guard: tests.run refuses a read behind a CTE');
select throws_ok(
  $t$select tests.run('/* lead */ select count(*) from public.issues')$t$,
  '22023', 'tests.run: not a count read or a write statement',
  'guard: tests.run refuses a read behind a leading comment');
-- call_outcome reads only a refused function call as `denied`; any other 42501 inside a function
-- that ran is re-raised, never counted as a deny.
select throws_ok(
  $t$select tests.call_outcome('anon',
    $p$do $d$ begin raise exception using errcode = '42501',
      message = 'permission denied for table zz_probe'; end $d$ $p$)$t$,
  '42501', 'permission denied for table zz_probe',
  'guard: call_outcome re-raises a 42501 that is not a refused function call');
select throws_ok(
  $t$select tests.probe('anon',
    $p$do $d$ begin raise exception using errcode = '42501', message = 'other'; end $d$ $p$)$t$,
  '42501', 'other',
  'guard: a 42501 whose message is neither phrase is re-raised as it was, not classified');
-- The two phrases, planted, so the re-raise above is a choice, not a probe that classifies nothing.
-- That both really occur on this image is the I2 split's proof (T5b) and A5's control below.
select is(
  tests.probe('anon', $p$do $d$ begin raise exception using errcode = '42501',
    message = 'permission denied for table zz_probe'; end $d$ $p$),
  -1::bigint,
  'guard: a 42501 starting "permission denied for" reads as a privilege refusal (-1)');
select is(
  tests.probe('anon', $p$do $d$ begin raise exception using errcode = '42501',
    message = 'new row violates row-level security policy for table "zz_probe"'; end $d$ $p$),
  -2::bigint,
  'guard: a 42501 naming a row-level security policy reads as an RLS refusal (-2)');

-- The driver itself. `under_fault` carries a null check value across its rollback, and takes a
-- fault of several statements. `probe_write` counts a write that got through and sees the row
-- change, and sees nothing when the write is refused. The scratch table is planted with its own
-- matrix row and rolled back with it, so no fixture is needed.
select is(tests.under_fault($f$select 1$f$, $c$null::text$c$), null,
  'driver: under_fault returns a null check value as null');
select
  $f$create table public.zz_pw (id int primary key, note text);
     alter table public.zz_pw disable row level security;
     insert into public.zz_pw values (1, 'x');
     grant select, update, delete on public.zz_pw to anon;
     insert into tests.matrix
       (rel, kind, access, key_col, touch_col, grants, policies, view_insert_code)
       values ('public.zz_pw', 'table', 'service', null, 'note', 'default', '{}', null)$f$
    as pw_fault,
  $c$tests.probe_write('anon', 'public.zz_pw',
     'update public.zz_pw set note = ''rls-probe''', 'id = 1')::text$c$ as pw \gset
select is(tests.under_fault(:'pw_fault', :'pw'), '(1,t)',
  'probe_write sees a planted fault: an update that got through is counted and the row changed');
select is(
  tests.under_fault(:'pw_fault' || '; alter table public.zz_pw enable row level security', :'pw'),
  '(0,f)',
  'probe_write control: under row-level security with no policy, nothing is touched or changed');
-- The snapshot sees a change by itself: a DO block reports no row count (n = 0), so `changed`
-- here comes from the ctid alone (a self-assigning update) and from the row's absence (a delete).
select
  $c$tests.probe_write('anon', 'public.zz_pw',
     'do $d$ begin update public.zz_pw set note = note where id = 1; end $d$', 'id = 1')::text$c$
    as pw_do_update,
  $c$tests.probe_write('anon', 'public.zz_pw',
     'do $d$ begin delete from public.zz_pw where id = 1; end $d$', 'id = 1')::text$c$
    as pw_do_delete,
  $c$tests.probe_write('anon', 'public.zz_pw',
     'update public.zz_pw set note = ''rls-probe''', 'id = 2')::text$c$ as pw_empty \gset
select is(tests.under_fault(:'pw_fault', :'pw_do_update'), '(0,t)',
  'probe_write sees a self-assigning update by its new ctid, with no row count to go on');
select is(tests.under_fault(:'pw_fault', :'pw_do_delete'), '(0,t)',
  'probe_write sees a deleted row by its absence, with no row count to go on');
-- A probe aimed at no rows proves nothing, so it is an error, never a refusal.
select throws_ok(
  format('select tests.under_fault(%L, %L)', :'pw_fault', :'pw_empty'),
  '22023', 'tests.probe_write: no target rows in public.zz_pw',
  'guard: probe_write refuses a target that holds no rows');

-- C1 (D4): `tests.unlisted()` names every relation without a matrix row and every row without a
-- relation, in both schemas.
select $c$array(select rel from tests.unlisted() order by rel)::text$c$ as c1 \gset
select is(
  tests.under_fault($f$create table public.zz_unlisted (id int)$f$, :'c1'),
  '{public.zz_unlisted}',
  'C1 sees a planted fault: a public table with no matrix row is named');
select is(
  tests.under_fault($f$create view admin.zz_unlisted_v as select 1 as x$f$, :'c1'),
  '{admin.zz_unlisted_v}',
  'C1 sees a planted fault: an admin view with no matrix row is named');
select is(
  tests.under_fault($f$insert into tests.matrix
      (rel, kind, access, key_col, touch_col, grants, policies, view_insert_code)
      values ('public.zz_ghost', 'table', 'service', null, 'x', 'default', '{}', null)$f$, :'c1'),
  '{public.zz_ghost}',
  'C1 sees a planted fault: a matrix row with no relation is named');
select is(:c1, '{}', 'C1 control: with each fault rolled back, nothing is unlisted');

-- F1 and F3: one SECURITY DEFINER function granted to both client roles. F1 lists it for each
-- role; F3's call, through `tests.call_outcome`, counts as executed (any outcome but a
-- `permission denied` 42501, since a definer function called with nulls may raise something else).
-- `stranger` is an authenticated session whose `sub` has no account, so F3 needs no fixture.
select
  $f$grant execute on function public.read_google_grant(uuid, text) to anon, authenticated$f$
    as f1_fault,
  $c$array(select fn from tests.client_functions('anon') where is_definer order by fn)::text$c$
    as f1_anon,
  $c$array(select fn from tests.client_functions('authenticated') where is_definer
     order by fn)::text$c$ as f1_auth,
  $c$tests.call_outcome('anon', 'select public.read_google_grant(null::uuid, null::text)')$c$
    as f3_anon,
  $c$tests.call_outcome('stranger', 'select public.read_google_grant(null::uuid, null::text)')$c$
    as f3_stranger \gset
select is(tests.under_fault(:'f1_fault', :'f1_anon'), '{public.read_google_grant}',
  'F1 sees a planted fault: a definer function anon may execute is listed');
select is(tests.under_fault(:'f1_fault', :'f1_auth'), '{public.read_google_grant}',
  'F1 sees a planted fault: a definer function authenticated may execute is listed');
select is(:f1_anon, '{}', 'F1 control: no definer function is listed for anon');
select is(:f1_auth, '{}', 'F1 control: no definer function is listed for authenticated');
select is(tests.under_fault(:'f1_fault', :'f3_anon'), 'executed',
  'F3 sees a planted fault: anon''s call to a granted definer function executes');
select is(tests.under_fault(:'f1_fault', :'f3_stranger'), 'executed',
  'F3 sees a planted fault: an authenticated call to a granted definer function executes');
select is(:f3_anon, 'denied', 'F3 control: anon''s call is denied');
select is(:f3_stranger, 'denied', 'F3 control: an authenticated call is denied');

-- F2: one SECURITY INVOKER, non-trigger function granted to anon alone. F2 lists it for anon only,
-- and F1 does not list it, so the role and the definer flag both discriminate.
select
  $f$grant execute on function public.sync_prune(int) to anon$f$ as f2_fault,
  $c$array(select c.fn from tests.client_functions('anon') c
     where not c.is_trigger
       and not exists (select 1 from tests.rpc_allowlist a where a.fn = c.fn)
     order by c.fn)::text$c$ as f2_anon,
  $c$array(select c.fn from tests.client_functions('authenticated') c
     where not c.is_trigger
       and not exists (select 1 from tests.rpc_allowlist a where a.fn = c.fn)
     order by c.fn)::text$c$ as f2_auth \gset
select is(tests.under_fault(:'f2_fault', :'f2_anon'), '{public.sync_prune}',
  'F2 sees a planted fault: a function anon may execute is listed');
select is(tests.under_fault(:'f2_fault', :'f2_auth'), '{}',
  'F2 under the same fault: authenticated, which was not granted, lists nothing');
select is(tests.under_fault(:'f2_fault', :'f1_anon'), '{}',
  'F1 under F2''s fault: an invoker function is not listed as a definer one');
select is(:f2_anon, '{}', 'F2 control: no unlisted function is executable by anon');
select is(:f2_auth, '{}', 'F2 control: no unlisted function is executable by authenticated');

-- F5: `usage` on `admin` for any API role.
select
  $f$grant usage on schema admin to anon$f$ as f5_fault,
  $c$(has_schema_privilege('anon', 'admin', 'usage')
     or has_schema_privilege('authenticated', 'admin', 'usage')
     or has_schema_privilege('service_role', 'admin', 'usage'))$c$ as f5 \gset
select is(tests.under_fault(:'f5_fault', :'f5')::boolean, true,
  'F5 sees a planted fault: anon holds usage on admin');
select is(tests.under_fault($f$grant usage on schema admin to authenticated$f$, :'f5')::boolean,
  true, 'F5 sees a planted fault: authenticated holds usage on admin');
select is(tests.under_fault($f$grant usage on schema admin to service_role$f$, :'f5')::boolean,
  true, 'F5 sees a planted fault: service_role holds usage on admin');
select is(:f5, false, 'F5 control: no API role holds usage on admin');

-- A5 and V2: `monthly_spend`, a closed view, granted to both client roles. It reads as a count
-- (none of its rows are needed); without the grant it is a privilege refusal. The control is also
-- the real "permission denied for" phrase, seen on this image. V2 is the authenticated side, as
-- `stranger` until T5b gives the file account A.
select
  $f$grant select on public.monthly_spend to anon, authenticated$f$ as a5_fault,
  $c$tests.probe('anon', 'select count(*) from public.monthly_spend')$c$ as a5,
  $c$tests.probe('stranger', 'select count(*) from public.monthly_spend')$c$ as v2 \gset
select cmp_ok(tests.under_fault(:'a5_fault', :'a5')::bigint, '>=', 0::bigint,
  'A5 sees a planted fault: anon reads public.monthly_spend as a count, not -1');
select is(:a5, -1::bigint, 'A5 control: anon is refused public.monthly_spend by a privilege');
select cmp_ok(tests.under_fault(:'a5_fault', :'v2')::bigint, '>=', 0::bigint,
  'V2 sees a planted fault: authenticated reads public.monthly_spend as a count, not -1');
select is(:v2, -1::bigint,
  'V2 control: authenticated is refused public.monthly_spend by a privilege');

-- P1 and P2: a bucket, and a storage policy outside the platform's own set. That set is the
-- design's expectation, empty; the `storage policies` diag above shows what this image holds, and
-- a non-empty set fails P2's control by name. Without a `storage` schema neither fault can be
-- planted, so the four cases fail by name rather than run against nothing.
\if :has_storage
select
  $f$insert into storage.buckets (id, name) values ('zz-rls-probe', 'zz-rls-probe')$f$ as p1_fault,
  $c$(select count(*) from storage.buckets)$c$ as p1,
  $f$create policy zz on storage.objects for select using (true)$f$ as p2_fault,
  $c$array(select policyname::text from pg_policies
     where schemaname = 'storage' and policyname::text <> all ('{}'::text[])
     order by 1)::text$c$ as p2 \gset
select is(tests.under_fault(:'p1_fault', :'p1')::bigint, 1::bigint,
  'P1 sees a planted fault: a storage bucket is counted');
select is(:p1, 0::bigint, 'P1 control: storage holds no bucket');
select is(tests.under_fault(:'p2_fault', :'p2'), '{zz}',
  'P2 sees a planted fault: a storage policy outside the platform''s set is named');
select is(:p2, '{}', 'P2 control: storage holds no policy beyond the platform''s set');
\else
select fail('P1 sees a planted fault: no storage schema, so no bucket can be planted');
select fail('P1 control: no storage schema to read');
select fail('P2 sees a planted fault: no storage schema, so no policy can be planted');
select fail('P2 control: no storage schema to read');
\endif

-- P3: a public table in a publication. The fault adds `issues` to `supabase_realtime` when the
-- image has it, and otherwise creates a publication of its own; the diag names which ran.
select
  case when exists (select 1 from pg_publication where pubname = 'supabase_realtime')
    then $f$alter publication supabase_realtime add table public.issues$f$
    else $f$create publication zz for table public.issues$f$
  end as p3_fault,
  $c$array(select schemaname || '.' || tablename from pg_publication_tables
     where schemaname in ('public', 'admin') order by 1)::text$c$ as p3 \gset
select diag('P3 plants: ' || :'p3_fault');
select is(tests.under_fault(:'p3_fault', :'p3'), '{public.issues}',
  'P3 sees a planted fault: a published public table is listed');
select is(:p3, '{}', 'P3 control: no public or admin relation is published');

-- The owner-read faults (T5b), planted on `public.issues` unless named. They read accounts A and B
-- from `tests.accounts` and the fixture rows `_fixtures.psql` writes: exactly one `issues` row for
-- A and one for B, and an active subscription on each for V1. Ids never appear in this file: a check
-- looks each one up as the owner and splices it into the probe as a literal (`format('%L')`), since
-- the probe runs as the client, who cannot read `tests.accounts`. `probe_write` re-reads its target
-- as the owner, so a target may name `tests.accounts` directly. The select-all companions are D6's:
-- Postgres applies SELECT policies to the rows an update, delete or upsert reads, so without one an
-- update, delete or move fault would still report nothing.

-- A1: a select policy for anon.
select $c$tests.probe('anon', 'select count(*) from public.issues')$c$ as a1 \gset
select cmp_ok(
  tests.under_fault($f$create policy zz on public.issues for select to anon using (true)$f$, :'a1')
    ::bigint, '>', 0::bigint,
  'A1 sees a planted fault: anon counts public.issues rows under a select-all policy');
select is(:a1, 0::bigint, 'A1 control: anon counts no public.issues row');

-- O1, O2 and O11: a select-all policy for authenticated. O1 is A's count and the rows A sees that
-- are not A's; O2 is B's rows asked for by key; O11 is the mirror, A's rows seen by B.
select
  $f$create policy zz on public.issues for select to authenticated using (true)$f$ as o1_fault,
  $c$tests.probe('A', 'select count(*) from public.issues')$c$ as o1,
  $c$tests.probe('A', format(
     'select count(*) from public.issues where account_id is distinct from %L',
     (select id from tests.accounts where who = 'A')))$c$ as o1_keyed,
  $c$tests.probe('A', format('select count(*) from public.issues where account_id = %L',
     (select id from tests.accounts where who = 'B')))$c$ as o2,
  $c$tests.probe('B', format(
     'select count(*) from public.issues where account_id is distinct from %L',
     (select id from tests.accounts where who = 'B')))$c$ as o11 \gset
select is(tests.under_fault(:'o1_fault', :'o1')::bigint, 2::bigint,
  'O1 sees a planted fault: A counts both fixture rows of public.issues, not its own one');
select cmp_ok(tests.under_fault(:'o1_fault', :'o1_keyed')::bigint, '>', 0::bigint,
  'O1 sees a planted fault: A sees a public.issues row keyed to another account');
select cmp_ok(tests.under_fault(:'o1_fault', :'o2')::bigint, '>', 0::bigint,
  'O2 sees a planted fault: A reads B''s public.issues row by its key');
select cmp_ok(tests.under_fault(:'o1_fault', :'o11')::bigint, '>', 0::bigint,
  'O11 sees a planted fault: B sees a public.issues row keyed to another account');
select is(:o1, 1::bigint, 'O1 control: A counts exactly its one fixture row of public.issues');
select is(:o1_keyed, 0::bigint, 'O1 control: every public.issues row A sees is keyed to A');
select is(:o2, 0::bigint, 'O2 control: A reads none of B''s public.issues rows by key');
select is(:o11, 0::bigint, 'O11 control: every public.issues row B sees is keyed to B');

-- V1: `billing_subscribers` inner-joins `accounts` and `entitlements`, so the fault opens both.
select
  $f$create policy zz on public.accounts for select to authenticated using (true);
     create policy zz on public.entitlements for select to authenticated using (true)$f$
    as v1_fault,
  $c$tests.probe('A', 'select count(*) from public.billing_subscribers')$c$ as v1 \gset
select is(tests.under_fault(:'v1_fault', :'v1')::bigint, 2::bigint,
  'V1 sees a planted fault: A reads two rows of public.billing_subscribers');
select is(:v1, 1::bigint, 'V1 control: A reads exactly its own row of public.billing_subscribers');

-- O3, O4 and A2: an insert policy for both client roles. Each insert is the fixture's own valid
-- row (`tests.insert_sql`) into the account's cleared slot (`tests.clear_sql`), so a refusal can
-- only be a grant or a policy. The I2 split: with the grant revoked instead, O3 is a privilege
-- refusal (-1), and its control below is the RLS refusal (-2), so both English phrases occur here.
select
  $f$create policy zz on public.issues for insert to anon, authenticated with check (true)$f$
    as o3_fault,
  $c$tests.probe('A', tests.insert_sql('public.issues', 'A'),
     tests.clear_sql('public.issues', 'A'))$c$ as o3,
  $c$tests.probe('A', tests.insert_sql('public.issues', 'B'),
     tests.clear_sql('public.issues', 'B'))$c$ as o4,
  $c$tests.probe('anon', tests.insert_sql('public.issues', 'A'),
     tests.clear_sql('public.issues', 'A'))$c$ as a2 \gset
select is(tests.under_fault(:'o3_fault', :'o3')::bigint, 1::bigint,
  'O3 sees a planted fault: A inserts a public.issues row for A');
select is(tests.under_fault(:'o3_fault', :'o4')::bigint, 1::bigint,
  'O4 sees a planted fault: A inserts a public.issues row for B');
select is(tests.under_fault(:'o3_fault', :'a2')::bigint, 1::bigint,
  'A2 sees a planted fault: anon inserts a public.issues row for A');
select is(
  tests.under_fault($f$revoke insert on public.issues from authenticated$f$, :'o3')::bigint,
  -1::bigint,
  'I2 split: with insert revoked from authenticated, O3 is a privilege refusal (-1), not RLS');
select is(:o3, -2::bigint,
  'O3 control: A''s insert for A is an RLS refusal (-2), the other phrase of the I2 split');
select is(:o4, -2::bigint, 'O4 control: A''s insert for B is an RLS refusal (-2)');
select is(:a2, -2::bigint, 'A2 control: anon''s insert for A is an RLS refusal (-2)');

-- O5: an upsert onto B's existing primary key. The fault adds insert, update and select-all
-- policies, since the upsert checks all three.
select
  $f$create policy zz_i on public.issues for insert to authenticated with check (true);
     create policy zz_u on public.issues for update to authenticated using (true);
     create policy zz_s on public.issues for select to authenticated using (true)$f$ as o5_fault,
  $c$tests.probe_write('A', 'public.issues',
     format('insert into public.issues (id, account_id, body) values (%L, %L, %L)'
       || ' on conflict (id) do update set app_version = %L',
       (select i.id from public.issues i
         where i.account_id = (select id from tests.accounts where who = 'B')
         order by i.id limit 1),
       (select id from tests.accounts where who = 'B'), '{}', 'rls-probe'),
     $w$account_id = (select id from tests.accounts where who = 'B')$w$)::text$c$ as o5 \gset
select is(tests.under_fault(:'o5_fault', :'o5'), '(1,t)',
  'O5 sees a planted fault: A''s upsert onto B''s public.issues key updates B''s row');
select is(:o5, '(-2,f)',
  'O5 control: A''s upsert onto B''s public.issues key is an RLS refusal, B''s row unchanged');

-- O6, O7 and A3: an update policy, with its select-all companion, for both client roles. Each
-- update sets `app_version` (the matrix's `touch_col`) to the text sentinel.
select
  $f$create policy zz_u on public.issues for update to anon, authenticated using (true);
     create policy zz_s on public.issues for select to anon, authenticated using (true)$f$
    as o6_fault,
  $c$tests.probe_write('A', 'public.issues',
     format('update public.issues set app_version = %L where account_id = %L', 'rls-probe',
       (select id from tests.accounts where who = 'A')),
     $w$account_id = (select id from tests.accounts where who = 'A')$w$)::text$c$ as o6,
  $c$tests.probe_write('A', 'public.issues',
     format('update public.issues set app_version = %L where account_id = %L', 'rls-probe',
       (select id from tests.accounts where who = 'B')),
     $w$account_id = (select id from tests.accounts where who = 'B')$w$)::text$c$ as o7,
  $c$tests.probe_write('anon', 'public.issues',
     format('update public.issues set app_version = %L', 'rls-probe'), 'true')::text$c$
    as a3 \gset
select is(tests.under_fault(:'o6_fault', :'o6'), '(1,t)',
  'O6 sees a planted fault: A''s update of its own public.issues row lands');
select is(tests.under_fault(:'o6_fault', :'o7'), '(1,t)',
  'O7 sees a planted fault: A''s update of B''s public.issues row lands');
select is(tests.under_fault(:'o6_fault', :'a3'), '(2,t)',
  'A3 sees a planted fault: anon''s update of every public.issues row lands');
select is(:o6, '(0,f)', 'O6 control: A''s update of its own public.issues row touches nothing');
select is(:o7, '(0,f)', 'O7 control: A''s update of B''s public.issues row touches nothing');
select is(:a3, '(0,f)', 'A3 control: anon''s update of public.issues touches nothing');

-- O8: the move, A's row re-keyed to B. The new row is checked against the select policies too,
-- hence the companion; `probe_write` re-reads A's rows, so a row that left A reads as changed.
select
  $f$create policy zz_u on public.issues for update to authenticated using (true) with check (true);
     create policy zz_s on public.issues for select to authenticated using (true)$f$ as o8_fault,
  $c$tests.probe_write('A', 'public.issues',
     format('update public.issues set account_id = %L where account_id = %L',
       (select id from tests.accounts where who = 'B'),
       (select id from tests.accounts where who = 'A')),
     $w$account_id = (select id from tests.accounts where who = 'A')$w$)::text$c$ as o8 \gset
select is(tests.under_fault(:'o8_fault', :'o8'), '(1,t)',
  'O8 sees a planted fault: A moves its public.issues row to B');
select is(:o8, '(0,f)', 'O8 control: A''s move of its public.issues row to B touches nothing');

-- O9, O10 and A4: a delete policy, with its select-all companion, for both client roles.
select
  $f$create policy zz_d on public.issues for delete to anon, authenticated using (true);
     create policy zz_s on public.issues for select to anon, authenticated using (true)$f$
    as o9_fault,
  $c$tests.probe_write('A', 'public.issues',
     format('delete from public.issues where account_id = %L',
       (select id from tests.accounts where who = 'A')),
     $w$account_id = (select id from tests.accounts where who = 'A')$w$)::text$c$ as o9,
  $c$tests.probe_write('A', 'public.issues',
     format('delete from public.issues where account_id = %L',
       (select id from tests.accounts where who = 'B')),
     $w$account_id = (select id from tests.accounts where who = 'B')$w$)::text$c$ as o10,
  $c$tests.probe_write('anon', 'public.issues', 'delete from public.issues', 'true')::text$c$
    as a4 \gset
select is(tests.under_fault(:'o9_fault', :'o9'), '(1,t)',
  'O9 sees a planted fault: A deletes its own public.issues row');
select is(tests.under_fault(:'o9_fault', :'o10'), '(1,t)',
  'O10 sees a planted fault: A deletes B''s public.issues row');
select is(tests.under_fault(:'o9_fault', :'a4'), '(2,t)',
  'A4 sees a planted fault: anon deletes every public.issues row');
select is(:o9, '(0,f)', 'O9 control: A''s delete of its own public.issues row touches nothing');
select is(:o10, '(0,f)', 'O10 control: A''s delete of B''s public.issues row touches nothing');
select is(:a4, '(0,f)', 'A4 control: anon''s delete of public.issues touches nothing');

-- O12 and O13: a policy that lets a session with no account through.
select
  $f$create policy zz on public.issues for select to authenticated
       using (auth.uid() is null or true)$f$ as o12_fault,
  $c$tests.probe('nosub', 'select count(*) from public.issues')$c$ as o12,
  $c$tests.probe('stranger', 'select count(*) from public.issues')$c$ as o13 \gset
select cmp_ok(tests.under_fault(:'o12_fault', :'o12')::bigint, '>', 0::bigint,
  'O12 sees a planted fault: a session with no sub counts public.issues rows');
select cmp_ok(tests.under_fault(:'o12_fault', :'o13')::bigint, '>', 0::bigint,
  'O13 sees a planted fault: a sub with no account counts public.issues rows');
select is(:o12, 0::bigint, 'O12 control: a session with no sub counts no public.issues row');
select is(:o13, 0::bigint, 'O13 control: a sub with no account counts no public.issues row');

-- C2: every matrix table has row-level security on.
select $c$array(select m.rel from tests.matrix m
   join pg_class c on c.oid = m.rel::regclass
   where m.kind = 'table' and not c.relrowsecurity order by 1)::text$c$ as c2 \gset
select is(
  tests.under_fault($f$alter table public.issues disable row level security$f$, :'c2'),
  '{public.issues}',
  'C2 sees a planted fault: a table with row-level security off is named');
select is(:c2, '{}', 'C2 control: every matrix table has row-level security on');

-- C3: each owner-read table carries exactly its matrix policies, each a permissive select for
-- authenticated whose `qual` is exactly `(<key> = auth.uid())`, with no `with_check`.
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
select is(
  tests.under_fault($f$alter policy issues_select_own on public.issues using (true)$f$, :'c3'),
  '{public.issues}',
  'C3 sees a planted fault: an owner-read policy whose qual is not the key test is named');
select is(:c3, '{}', 'C3 control: every owner-read table carries exactly its matrix policy');

-- C5: a view's `security_invoker` matches its access, and no closed view grants a client select.
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
select is(
  tests.under_fault(
    $f$alter view public.billing_subscribers set (security_invoker = false)$f$, :'c5'),
  '{public.billing_subscribers}',
  'C5 sees a planted fault: an invoker view running as its owner is named');
select is(:c5, '{}', 'C5 control: every view''s security_invoker and select grants match');

-- C6: each table's four client verbs, for both client roles, match the matrix's `grants`. Under
-- the same fault, O1's probe becomes a privilege refusal.
select
  $f$revoke select on public.issues from authenticated$f$ as c6_fault,
  $c$array(select m.rel || ':' || r.role || ':' || v.verb
     from tests.matrix m
     cross join (values ('anon'), ('authenticated')) r(role)
     cross join (values ('select'), ('insert'), ('update'), ('delete')) v(verb)
     where m.kind = 'table'
       and has_table_privilege(r.role::name, m.rel, v.verb)
             is distinct from (m.grants = 'default')
     order by 1)::text$c$ as c6 \gset
select is(tests.under_fault(:'c6_fault', :'c6'), '{public.issues:authenticated:select}',
  'C6 sees a planted fault: a table verb authenticated lost is named');
select is(tests.under_fault(:'c6_fault', :'o1')::bigint, -1::bigint,
  'C6''s fault seen by O1: A''s count of public.issues is a privilege refusal (-1)');
select is(:c6, '{}', 'C6 control: every table''s client verbs match the matrix');

-- C7: the fixtures are not vacuous. Each owner-read table holds a row for A and a row for B; a
-- missing account counts as no row. The count is dynamic SQL run as the owner, read back through
-- `query_to_xml`.
select $c$array(select m.rel || ':' || w.who
   from tests.matrix m
   cross join (values ('A'), ('B')) w(who)
   left join tests.accounts a on a.who = w.who
   where m.access = 'own'
     and (xpath('/row/n/text()', query_to_xml(format(
           'select count(*) as n from %s where %I = %L', m.rel, m.key_col, a.id),
           false, true, '')))[1]::text::bigint = 0
   order by 1)::text$c$ as c7 \gset
select is(
  tests.under_fault(
    $f$delete from public.issues
         where account_id = (select id from tests.accounts where who = 'B')$f$,
    :'c7'),
  '{public.issues:B}',
  'C7 sees a planted fault: an owner-read table with no row for B is named');
select is(:c7, '{}', 'C7 control: every owner-read table holds a row for A and a row for B');

-- V3: each view's `insert ... default values` raises the matrix's code, as anon and as A. The fault
-- is an updatable view over `issues` that runs as its owner, given defaults for the two columns
-- `issues` cannot default (A's account, a placeholder body) and its own matrix row expecting 55000,
-- so the insert succeeds (00000) and the view is named.
select
  $f$create view public.zz_v as select * from public.issues;
     do $d$ begin
       execute format('alter view public.zz_v alter column account_id set default %L',
         (select id from tests.accounts where who = 'A'));
     end $d$;
     alter view public.zz_v alter column body set default '{}';
     grant insert on public.zz_v to anon, authenticated;
     insert into tests.matrix
       (rel, kind, access, key_col, touch_col, grants, policies, view_insert_code)
       values ('public.zz_v', 'view', 'closed_view', null, null, 'revoked', '{}', '55000')$f$
    as v3_fault,
  $c$array(select m.rel from tests.matrix m
     where m.kind = 'view'
       and tests.view_insert('anon', m.rel) is distinct from m.view_insert_code
     order by 1)::text$c$ as v3_anon,
  $c$array(select m.rel from tests.matrix m
     where m.kind = 'view'
       and tests.view_insert('A', m.rel) is distinct from m.view_insert_code
     order by 1)::text$c$ as v3_a \gset
select is(tests.under_fault(:'v3_fault', :'v3_anon'), '{public.zz_v}',
  'V3 sees a planted fault: anon''s insert into an updatable owner-run view succeeds');
select is(tests.under_fault(:'v3_fault', :'v3_a'), '{public.zz_v}',
  'V3 sees a planted fault: A''s insert into an updatable owner-run view succeeds');
select is(:v3_anon, '{}', 'V3 control: anon''s insert into every view raises the matrix''s code');
select is(:v3_a, '{}', 'V3 control: A''s insert into every view raises the matrix''s code');

select * from finish();
rollback;
