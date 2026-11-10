-- anon (RLS pgTAP plan, D3 A1 to A5 and V3): a request with the anon key and no session reads,
-- writes and changes nothing in any relation in `public` or `admin`. This is the real run against
-- the migrated schema; `09_self_test_test.sql` first showed each probe failing against its own
-- planted fault on `public.issues` (A1 to A5) and on a planted updatable view (V3).
--
-- Every case comes from a loop over `tests.matrix`, one per relation, so a matrix row added or
-- dropped moves `plan()` with it and nothing here names a table. Expectations are D3's "per
-- `grants`": a read, update or delete is 0 at the default grants and -1 (a privilege) when they
-- are revoked; an insert is -2 (row-level security) or -1. A5 is 0 for the invoker view and -1
-- for a closed one; V3 is each view's `view_insert_code`.
--
-- Every probe runs through the driver as the owner, which switches to anon only inside it: reads
-- and A2 through `tests.probe`, A3 and A4 through `tests.probe_write`, which also re-reads every
-- row of the table as the owner, so a write that touched a row fails its case whatever count it
-- gave.
-- A2 inserts the fixture's own valid row for account A (`tests.insert_sql`) into A's cleared slot
-- (`tests.clear_sql`; a global table's one slot), so a refusal can only be a grant or a policy. Its
-- arguments are built before the driver clears anything, and each probe clears only its own table.
--
-- A2 and A3 run through `tests.unmasked` (`_setup.psql`): the target table's non-internal,
-- row-level AFTER triggers that fire on INSERT or UPDATE (`tests.unmask_triggers`; today
-- `sync_records_usage` and `sync_notes_usage`) are disabled by owner DDL inside
-- `tests.under_fault`, rolled back with the probe. Such a trigger fires only for a row that has
-- already passed the table's grant and policies, so it never refuses a probe on a sound schema;
-- but those two run `sync_usage_bump` as the caller, whose `sync_usage` upsert would turn an insert
-- that got through into `sync_usage`'s -2, the value A2 expects. With them off, the probe reports
-- the table's own grant and policy. Disabling a trigger also stops its DELETE arm, inside that
-- subtransaction only. A4 calls the driver directly, so the triggers stay on (on DELETE the
-- function runs a plain update, which RLS filters to no row without an error, so it masks
-- nothing). 09 shows the mask is real and the unmask lifts it, for anon on each sync table; 01's
-- C9 fails on any other trigger on a relation this file probes.
--
-- No id, address or row content is printed: descriptions name the case and the relation only.
begin;
\ir _setup.psql
\ir _matrix.psql
\ir _fixtures.psql

-- The relation as SQL text, each part quoted as an identifier (`public.issues`).
create function tests.a_qualified(p_rel text) returns text language sql stable as $$
  select format('%I.%I', split_part(p_rel, '.', 1), split_part(p_rel, '.', 2))
$$;

-- A3's `set` clause: the matrix's `touch_col` to D3's per-type sentinel, by the column's type
-- category. A type with no sentinel raises; it is never skipped.
create function tests.a_sentinel(p_rel text) returns text language plpgsql as $$
declare v_col text; v_cat text;
begin
  select m.touch_col, t.typcategory::text into v_col, v_cat
    from tests.matrix m
    join pg_attribute a
      on a.attrelid = p_rel::regclass and a.attname = m.touch_col and not a.attisdropped
    join pg_type t on t.oid = a.atttypid
   where m.rel = p_rel and m.kind = 'table';
  case coalesce(v_cat, '?')
    when 'S' then return format('%I = %L', v_col, 'rls-probe');
    when 'N' then return format('%I = -424242', v_col);
    when 'D' then return format('%I = now() - interval %L', v_col, '1 year');
    when 'B' then return format('%I = not %I', v_col, v_col);
    else
      raise exception 'tests.a_sentinel: no sentinel for % (type category %)',
        p_rel, coalesce(v_cat, '<none>');
  end case;
end $$;

-- One case per relation for `p_case`, in `rel` order: A1 to A4 over the tables, A5 and V3 over the
-- views. Each group is its own statement below, so a probe that raises stops the file after the
-- groups before it have printed.
create function tests.anon_cases(p_case text) returns setof text language plpgsql as $$
declare
  r record; v_n bigint; v_sql text; v_check text; v_desc text;
begin
  if p_case not in ('A1', 'A2', 'A3', 'A4', 'A5', 'V3') then
    raise exception 'tests.anon_cases: unknown case %', coalesce(p_case, '<null>');
  end if;
  for r in
    select m.rel, m.access, m.grants, m.view_insert_code, tests.a_qualified(m.rel) as q
      from tests.matrix m
     where m.kind = case when p_case in ('A5', 'V3') then 'view' else 'table' end
     order by m.rel
  loop
    case p_case
      when 'A1' then
        v_n := case r.grants when 'default' then 0 else -1 end;
        v_desc := case r.grants
          when 'default' then format('A1 anon reads no row of %s', r.rel)
          else format('A1 anon is refused a read of %s by a privilege', r.rel) end;
        return next is(tests.probe('anon', format('select count(*) from %s', r.q)), v_n, v_desc);
      when 'A2' then
        v_n := case r.grants when 'default' then -2 else -1 end;
        v_desc := format('A2 anon cannot insert into %s: %s', r.rel, case r.grants
          when 'default' then 'a row-level security refusal (-2)'
          else 'a privilege refusal (-1)' end);
        v_check := format('tests.probe(%L, tests.insert_sql(%L, %L), tests.clear_sql(%L, %L))',
          'anon', r.rel, 'A', r.rel, 'A');
        return next is(tests.unmasked(r.rel, v_check)::bigint, v_n, v_desc);
      when 'A3' then
        v_n := case r.grants when 'default' then 0 else -1 end;
        v_desc := format('A3 anon cannot update %s: %s, every row unchanged', r.rel,
          case r.grants when 'default' then 'no row touched' else 'a privilege refusal' end);
        v_sql := format('update %s set %s', r.q, tests.a_sentinel(r.rel));
        v_check := format('tests.probe_write(%L, %L, %L, %L)::text', 'anon', r.rel, v_sql,
          'true');
        return next is(tests.unmasked(r.rel, v_check), format('(%s,f)', v_n), v_desc);
      when 'A4' then
        v_n := case r.grants when 'default' then 0 else -1 end;
        v_desc := format('A4 anon cannot delete from %s: %s, every row unchanged', r.rel,
          case r.grants when 'default' then 'no row touched' else 'a privilege refusal' end);
        v_sql := format('delete from %s', r.q);
        return next is(tests.probe_write('anon', r.rel, v_sql, 'true')::text,
          format('(%s,f)', v_n), v_desc);
      when 'A5' then
        v_n := case r.access when 'invoker_view' then 0 else -1 end;
        v_desc := case r.access
          when 'invoker_view' then format('A5 anon reads no row of view %s', r.rel)
          else format('A5 anon is refused a read of view %s by a privilege', r.rel) end;
        return next is(tests.probe('anon', format('select count(*) from %s', r.q)), v_n, v_desc);
      when 'V3' then
        return next is(tests.view_insert('anon', r.rel), r.view_insert_code,
          format('V3 anon cannot insert into view %s: SQLSTATE %s', r.rel, r.view_insert_code));
    end case;
  end loop;
end $$;

-- Four cases per table (A1 to A4) and two per view (A5, V3).
select plan((
  select (4 * count(*) filter (where m.kind = 'table')
          + 2 * count(*) filter (where m.kind = 'view'))::int
    from tests.matrix m));

-- Counts only: what the matrix holds, and so what the plan above was computed from.
select diag(format('anon: %s tables (%s default grants, %s revoked), %s views (%s invoker)',
  count(*) filter (where m.kind = 'table'),
  count(*) filter (where m.kind = 'table' and m.grants = 'default'),
  count(*) filter (where m.kind = 'table' and m.grants = 'revoked'),
  count(*) filter (where m.kind = 'view'),
  count(*) filter (where m.access = 'invoker_view')))
  from tests.matrix m;

-- A1: `select count(*)` of every table.
select * from tests.anon_cases('A1');
-- A2: a valid row for A, in A's cleared slot, into every table.
select * from tests.anon_cases('A2');
-- A3: every row of every table set to the sentinel.
select * from tests.anon_cases('A3');
-- A4: every row of every table deleted.
select * from tests.anon_cases('A4');
-- A5: `select count(*)` of every view.
select * from tests.anon_cases('A5');
-- V3: `insert ... default values` into every view.
select * from tests.anon_cases('V3');

select * from finish();
rollback;
