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

select plan(36);

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
-- a non-empty set fails P2's control by name. The schema may be absent, so both are skipped then.
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
select skip('P1, P2: this image has no storage schema', 4);
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

select * from finish();
rollback;
