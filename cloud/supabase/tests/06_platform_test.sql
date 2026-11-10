-- The platform (RLS pgTAP plan, D3 P1 to P3; P4 from the T9 review): no migration creates a
-- storage bucket, a storage policy, a publication of a `public` or `admin` relation, or a client
-- policy on Realtime's `realtime.messages` (which governs private broadcast and presence
-- channels), so a client can reach none of those ways around the tables' own policies. This is
-- the real run against the migrated schema.
--
-- Each check is the expression `09_self_test_test.sql` first showed failing against its own
-- planted fault, in the same text. P2's expected set is the platform's own storage policies as
-- T4 and T5a recorded them on this image: none, so `'{}'`.
--
-- P1 and P2 are guarded by `has_schema`: without a `storage` schema the case says so and P1 and P2
-- fail by name (T5a ruling: fail, never skip, since a skip would silently drop two probes). P3
-- reads `pg_publication_tables`, a catalog view every server has, so it needs no guard. P4 reads
-- `pg_policies` for `realtime.messages`, a table the Realtime service creates, not a migration:
-- when it is absent on this image P4 asserts that absence instead, by name, and the diag says
-- which. The file reads no fixture rows, so it includes the setup and the matrix only, as
-- `00_smoke_test.sql` does.
begin;
\ir _setup.psql
\ir _matrix.psql

-- 5 fixed cases: the storage schema, P1, P2, P3, P4.
select plan(5);

-- Names and counts only: what this image holds, whatever the cases below conclude.
select diag('storage schema: ' || case when exists (
  select 1 from pg_namespace where nspname = 'storage') then 'present' else 'absent' end);
select diag('storage policies: '
  || array(select policyname::text from pg_policies where schemaname = 'storage' order by 1)::text);
select diag('publications: ' || array(select pubname::text from pg_publication order by 1)::text);
select exists (select 1 from pg_namespace where nspname = 'storage') as has_storage \gset
select to_regclass('realtime.messages') is not null as has_rt_messages \gset
select diag('realtime.messages: '
  || case when :'has_rt_messages'::boolean then 'present' else 'absent' end);
select diag('realtime.messages policies: '
  || array(select policyname::text || ' ' || roles::text from pg_policies
            where schemaname = 'realtime' and tablename = 'messages' order by 1)::text);

-- The guard: P1 and P2 read the `storage` schema.
select has_schema('storage', 'P1 and P2 guard: the storage schema exists');

\if :has_storage
-- P1: storage holds no bucket. 09: one planted `storage.buckets` row.
select is((select count(*) from storage.buckets), 0::bigint,
  'P1: storage.buckets holds no bucket');

-- P2: storage carries no policy beyond the platform's own set, which is empty. 09: a select
-- policy planted on `storage.objects`.
select is(
  array(select policyname::text from pg_policies
     where schemaname = 'storage' and policyname::text <> all ('{}'::text[])
     order by 1)::text,
  '{}',
  'P2: storage carries no policy beyond the platform''s own set (none)');
\else
select fail('P1: no storage schema, so storage.buckets cannot be read');
select fail('P2: no storage schema, so its policies cannot be read');
\endif

-- P3: no `public` or `admin` relation is in any publication, `supabase_realtime` included. 09:
-- `public.issues` added to `supabase_realtime`.
select is(
  array(select schemaname || '.' || tablename from pg_publication_tables
     where schemaname in ('public', 'admin') order by 1)::text,
  '{}',
  'P3: no public or admin relation is in a publication');

-- P4: no policy on `realtime.messages` applies to anon, authenticated or public (a policy with
-- no `to` lists `public`), so no client may read or send a private Realtime message. 09: a
-- policy planted for each of the three.
\if :has_rt_messages
select is(
  array(select p.policyname::text from pg_policies p
     where p.schemaname = 'realtime' and p.tablename = 'messages'
       and p.roles && '{anon,authenticated,public}'::name[]
     order by 1)::text,
  '{}',
  'P4: no policy on realtime.messages applies to anon, authenticated or public');
\else
select ok(to_regclass('realtime.messages') is null,
  'P4: realtime.messages is absent on this image, so no client policy can apply to it');
\endif

select * from finish();
rollback;
