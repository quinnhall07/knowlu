-- L8's S6 follow-up, the staging proof for 20261020000600_comp_guards.sql (docs/specs/
-- 2026-10-18-friends-testers-design.md §9 layer iii, in proofs/testers_staging.sql's form). Not a
-- migration: it lives outside migrations/ and is never deployed. The controller runs it on staging
-- after the follow-up is pushed there, and on prod after the prod migration.
--
-- One transaction that only ever rolls back: nothing it writes survives. It raises on the first
-- mismatch, and every result line is structure only (a case id, a status, a count), never an
-- account id. Every address is an example.invalid placeholder. Accounts are made as the testers
-- proof makes them: the auth.users row alone, by S1's recorded columns with no password, and the
-- sign-up triggers write the rest. The G cases are migrations/testers_cases.ts's GUARD_CASES,
-- verbatim (pinned by migrations_test.ts); each call runs in its own subtransaction, and a refused
-- call must leave its account's grant and audit rows as they were. Then the year's exact edge, which
-- whole days cannot state, and the structure: admin.testers' columns in order, and the two
-- functions' settings and rights. Its pg_temp names differ from the testers proof's, so both can
-- run in one rehearsal.

begin;

-- The trigger precheck, as the testers proof has it.
do $$
declare
  v_src text;
begin
  select p.prosrc into v_src
    from pg_trigger t
    join pg_proc p on p.oid = t.tgfoid
    join pg_namespace n on n.oid = p.pronamespace
   where t.tgrelid = 'auth.users'::regclass and t.tgenabled <> 'D'
     and n.nspname = 'public' and p.proname = 'handle_new_user';
  if v_src is null then
    raise exception 'precheck: no enabled handle_new_user trigger on auth.users';
  end if;
  if position('raise' in lower(v_src)) > 0 then
    raise exception 'precheck: handle_new_user is not the C1b body (it raises)';
  end if;
  raise notice 'precheck: handle_new_user is the C1b body';
end $$;

-- A day count from the case table as a time; one transaction, so now() is one instant throughout.
create function pg_temp.g_days(d jsonb) returns timestamptz
language sql stable as $$ select now() + make_interval(days => (d #>> '{}')::int) $$;

-- One sign-up: the auth.users row alone; on_auth_user_metadata and handle_new_user do the rest.
create function pg_temp.g_signup(p_email text) returns uuid
language plpgsql as $$
declare
  v_id uuid := gen_random_uuid();
begin
  insert into auth.users (instance_id, id, aud, role, email, email_confirmed_at, last_sign_in_at,
                          raw_app_meta_data, raw_user_meta_data, created_at, updated_at,
                          confirmation_token, recovery_token, email_change_token_new, email_change,
                          email_change_token_current, email_change_confirm_status, phone_change,
                          phone_change_token, reauthentication_token, is_anonymous, is_sso_user)
  values ('00000000-0000-0000-0000-000000000000', v_id, 'authenticated', 'authenticated', p_email,
          now(), now(), '{"provider": "email", "providers": ["email"]}', '{}', now(), now(),
          '', '', '', '', '', 0, '', '', '', false, false);
  return v_id;
end $$;

-- The view's answer for each account against the case's, as the testers proof's p_view.
create function pg_temp.g_view(label text, ids uuid[], want jsonb) returns void
language plpgsql as $$
declare
  w jsonb;
  r record;
  n int;
begin
  for i in 1 .. jsonb_array_length(want) loop
    w := want -> (i - 1);
    select count(*) into n from public.effective_entitlements v where v.account_id = ids[i];
    if n <> 1 then raise exception '%: account % has % view rows, want one', label, i - 1, n; end if;
    select v.status, v.plan, v.current_period_end into r from public.effective_entitlements v where v.account_id = ids[i];
    if r.status is distinct from w ->> 'status' or r.plan is distinct from w ->> 'plan'
       or r.current_period_end is distinct from pg_temp.g_days(w -> 'periodEnd') then
      raise exception '%: account % reads % / % / % days, want % / % / % days', label, i - 1, r.status, r.plan,
        round((extract(epoch from r.current_period_end - now()) / 86400)::numeric, 3), w ->> 'status', w ->> 'plan', w ->> 'periodEnd';
    end if;
  end loop;
end $$;

-- admin.testers' row for each grant, in the case's grants order: state, paid and the two new columns.
create function pg_temp.g_testers(label text, ids uuid[], grants jsonb, want jsonb) returns void
language plpgsql as $$
declare
  w jsonb;
  r record;
  n int;
begin
  for i in 1 .. jsonb_array_length(want) loop
    w := want -> (i - 1);
    select count(*) into n from admin.testers t where t.account_id = ids[(grants -> (i - 1) ->> 'account')::int + 1];
    select t.state, t.paid, t.entitlement_status, t.has_subscription into r
      from admin.testers t where t.account_id = ids[(grants -> (i - 1) ->> 'account')::int + 1];
    if n <> 1 or r.state is distinct from w ->> 'state' or r.paid is distinct from (w ->> 'paid')::boolean
       or r.entitlement_status is distinct from w ->> 'entitlementStatus'
       or r.has_subscription is distinct from (w ->> 'hasSubscription')::boolean then
      raise exception '%: grant % reads % rows, % / paid % / % / subscription %, want % / paid % / % / subscription %',
        label, i - 1, n, r.state, r.paid, r.entitlement_status, r.has_subscription,
        w ->> 'state', w ->> 'paid', w ->> 'entitlementStatus', w ->> 'hasSubscription';
    end if;
  end loop;
end $$;

-- One G case: its accounts through the sign-up triggers, their rows (and a Stripe subscription id
-- where the case says), its grants through admin.comp_grant; the view before the calls where the case
-- says; the calls, each in its own subtransaction, a refusal matched word for word and leaving the
-- account's grant and audit rows as they were, a success adding exactly one audit row; the view and
-- admin.testers at the end.
create function pg_temp.g_case(c jsonb) returns void
language plpgsql as $$
declare
  ids uuid[] := '{}';
  x jsonb;
  v_id uuid;
  v_err text;
  v_state text;
  g0 bigint;
  a0 bigint;
  g1 bigint;
  a1 bigint;
begin
  for x in select value from jsonb_array_elements(c -> 'accounts') loop
    v_id := pg_temp.g_signup(x ->> 'email');
    ids := ids || v_id;
    if not exists (select 1 from public.entitlements e where e.account_id = v_id and e.status = 'none'
                   and e.plan is null and e.current_period_end is null and e.stripe_subscription_id is null) then
      raise exception '%: the sign-up trigger wrote no none row', c ->> 'id';
    end if;
    if jsonb_typeof(x -> 'row') = 'null' then
      delete from public.entitlements where account_id = v_id;
    elsif x -> 'row' <> '{"status": "none", "plan": null, "periodEnd": null}'::jsonb or x -> 'subscription' is not null then
      update public.entitlements
         set status = x -> 'row' ->> 'status', plan = x -> 'row' ->> 'plan', current_period_end = pg_temp.g_days(x -> 'row' -> 'periodEnd'),
             stripe_subscription_id = case when (x ->> 'subscription')::boolean then 'sub_guard_' || lower(c ->> 'id') end
       where account_id = v_id;
    end if;
  end loop;
  for x in select value from jsonb_array_elements(c -> 'grants') loop
    perform admin.comp_grant(ids[(x ->> 'account')::int + 1], pg_temp.g_days(x -> 'endsIn'), 'friends', null);
  end loop;
  if c -> 'expectBefore' is not null then perform pg_temp.g_view(c ->> 'id' || ' before', ids, c -> 'expectBefore'); end if;
  for x in select value from jsonb_array_elements(coalesce(c -> 'calls', '[]')) loop
    v_id := case when x ->> 'account' = 'unknown' then gen_random_uuid() else ids[(x ->> 'account')::int + 1] end;
    select count(*) into g0 from public.comp_grants g where g.account_id = v_id;
    select count(*) into a0 from public.comp_audit u where u.account_id = v_id;
    v_err := null;
    begin
      if x ->> 'fn' = 'comp_grant' then
        perform admin.comp_grant(v_id, pg_temp.g_days(x -> 'endsIn'), 'friends', null);
      else
        perform admin.comp_extend(v_id, pg_temp.g_days(x -> 'endsIn'));
      end if;
    exception when others then
      v_err := sqlerrm;
      v_state := sqlstate;
    end;
    select count(*) into g1 from public.comp_grants g where g.account_id = v_id;
    select count(*) into a1 from public.comp_audit u where u.account_id = v_id;
    if x -> 'refused' is null and v_err is not null then
      raise exception '%: % refused (sqlstate %), want it to succeed', c ->> 'id', x ->> 'fn', v_state;
    elsif x -> 'refused' is not null and v_err is null then
      raise exception '%: % succeeded, want it refused', c ->> 'id', x ->> 'fn';
    elsif x -> 'refused' is not null and v_err is distinct from x ->> 'refused' then
      raise exception '%: % refused (sqlstate %) without the words %', c ->> 'id', x ->> 'fn', v_state, x ->> 'refused';
    elsif x -> 'refused' is not null and (g1 <> g0 or a1 <> a0) then
      raise exception '%: a refused % wrote % grant and % audit rows', c ->> 'id', x ->> 'fn', g1 - g0, a1 - a0;
    elsif x -> 'refused' is null and a1 <> a0 + 1 then
      raise exception '%: % wrote % audit rows, want one', c ->> 'id', x ->> 'fn', a1 - a0;
    end if;
  end loop;
  perform pg_temp.g_view(c ->> 'id', ids, c -> 'expect');
  if c -> 'testers' is not null then perform pg_temp.g_testers(c ->> 'id', ids, c -> 'grants', c -> 'testers'); end if;
end $$;

-- The G cases, in the case table's order; the first mismatch raises.
do $$
declare
  c jsonb;
  n int := 0;
begin
  for c in select value from jsonb_array_elements($cases$
[
{"id":"G1","given":"comp_grant at a past date and at now() refused, so no expired grant holds the one open slot; then a grant","accounts":[{"email":"g1-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[],"calls":[{"fn":"comp_grant","account":0,"endsIn":-1,"refused":"the ends_at is not in the future"},{"fn":"comp_grant","account":0,"endsIn":0,"refused":"the ends_at is not in the future"},{"fn":"comp_grant","account":0,"endsIn":30}],"expectBefore":[{"status":"none","plan":null,"periodEnd":null}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"G2","given":"comp_grant more than a year out refused; 365 days granted","accounts":[{"email":"g2-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[],"calls":[{"fn":"comp_grant","account":0,"endsIn":367,"refused":"the ends_at is more than a year away"},{"fn":"comp_grant","account":0,"endsIn":365}],"expectBefore":[{"status":"none","plan":null,"periodEnd":null}],"expect":[{"status":"active","plan":"tester","periodEnd":365}]},
{"id":"G3","given":"comp_grant at a past date on an id with no accounts row: the date is refused first","accounts":[],"grants":[],"calls":[{"fn":"comp_grant","account":"unknown","endsIn":-1,"refused":"the ends_at is not in the future"}],"expect":[]},
{"id":"G4","given":"comp_extend more than a year out refused and the grant keeps its end; 365 days extended","accounts":[{"email":"g4-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30}],"calls":[{"fn":"comp_extend","account":0,"endsIn":367,"refused":"the new ends_at is more than a year away"},{"fn":"comp_extend","account":0,"endsIn":365}],"expectBefore":[{"status":"active","plan":"tester","periodEnd":30}],"expect":[{"status":"active","plan":"tester","periodEnd":365}]},
{"id":"G5","given":"comp_extend more than a year out with no open grant: the date is refused first","accounts":[{"email":"g5-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[],"calls":[{"fn":"comp_extend","account":0,"endsIn":367,"refused":"the new ends_at is more than a year away"}],"expect":[{"status":"none","plan":null,"periodEnd":null}]},
{"id":"G6","given":"Stripe past_due with a subscription, open grant: paid unset, past_due and a subscription","accounts":[{"email":"g6-0@example.invalid","row":{"status":"past_due","plan":"monthly","periodEnd":2},"subscription":true}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}],"testers":[{"state":"live","paid":false,"entitlementStatus":"past_due","hasSubscription":true}]},
{"id":"G7","given":"Stripe canceled with a subscription, open grant: paid unset, canceled and a subscription","accounts":[{"email":"g7-0@example.invalid","row":{"status":"canceled","plan":"monthly","periodEnd":-3},"subscription":true}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}],"testers":[{"state":"live","paid":false,"entitlementStatus":"canceled","hasSubscription":true}]},
{"id":"G8","given":"Stripe active with a subscription, open grant: paid wins, active and a subscription","accounts":[{"email":"g8-0@example.invalid","row":{"status":"active","plan":"monthly","periodEnd":20},"subscription":true}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"monthly","periodEnd":20}],"testers":[{"state":"live","paid":true,"entitlementStatus":"active","hasSubscription":true}]},
{"id":"G9","given":"sign-up none row, open grant: none and no subscription","accounts":[{"email":"g9-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}],"testers":[{"state":"live","paid":false,"entitlementStatus":"none","hasSubscription":false}]},
{"id":"G10","given":"no entitlements row, open grant: the coalesce answers none, and no subscription","accounts":[{"email":"g10-0@example.invalid","row":null}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}],"testers":[{"state":"live","paid":false,"entitlementStatus":"none","hasSubscription":false}]}
]
$cases$::jsonb) loop
    perform pg_temp.g_case(c);
    n := n + 1;
    raise notice '%: ok', c ->> 'id';
  end loop;
  raise notice 'G cases: all % passed', n;
end $$;

-- The year's exact edge: a null date is not in the future; a year and a second is refused by both
-- functions; a year to the second is inside for both, and the two successes write two audit rows.
do $$
declare
  v_id uuid := pg_temp.g_signup('edge-0@example.invalid');
  v_err text;
  n bigint;
begin
  begin
    perform admin.comp_grant(v_id, null, 'friends', null);
  exception when others then
    v_err := sqlerrm;
  end;
  if v_err is distinct from 'the ends_at is not in the future' then
    raise exception 'edge: comp_grant with a null date reads %', coalesce(v_err, 'granted');
  end if;
  v_err := null;
  begin
    perform admin.comp_grant(v_id, now() + interval '1 year 1 second', 'friends', null);
  exception when others then
    v_err := sqlerrm;
  end;
  if v_err is distinct from 'the ends_at is more than a year away' then
    raise exception 'edge: comp_grant a year and a second out reads %', coalesce(v_err, 'granted');
  end if;
  perform admin.comp_grant(v_id, now() + interval '1 year', 'friends', null);
  v_err := null;
  begin
    perform admin.comp_extend(v_id, now() + interval '1 year 1 second');
  exception when others then
    v_err := sqlerrm;
  end;
  if v_err is distinct from 'the new ends_at is more than a year away' then
    raise exception 'edge: comp_extend a year and a second out reads %', coalesce(v_err, 'extended');
  end if;
  perform admin.comp_extend(v_id, now() + interval '1 year');
  select count(*) into n from public.comp_audit u where u.account_id = v_id;
  if n <> 2 then raise exception 'edge: % audit rows, want the grant and the extension', n; end if;
  if (select g.ends_at from public.comp_grants g where g.account_id = v_id) is distinct from now() + interval '1 year' then
    raise exception 'edge: the grant does not end a year out';
  end if;
  raise notice 'edge: ok';
end $$;

-- The structure: admin.testers' columns in order, the old eight first; both functions security
-- invoker with an empty search path; no API role may execute them or read the view.
do $$
declare
  v_cols text;
  r record;
  v_role text;
begin
  select string_agg(c.column_name || ' ' || c.data_type, ', ' order by c.ordinal_position) into v_cols
    from information_schema.columns c where c.table_schema = 'admin' and c.table_name = 'testers';
  if v_cols is distinct from 'account_id uuid, email text, cohort text, state text, paid boolean, '
      'granted_at timestamp with time zone, ends_at timestamp with time zone, last_active_day date, '
      'entitlement_status text, has_subscription boolean' then
    raise exception 'structure: admin.testers reads %', v_cols;
  end if;
  for r in select p.proname, p.prosecdef, p.proconfig from pg_proc p join pg_namespace n on n.oid = p.pronamespace
            where n.nspname = 'admin' and p.proname in ('comp_grant', 'comp_extend') loop
    if r.prosecdef or r.proconfig is distinct from array['search_path=""'] then
      raise exception 'structure: % is definer % or search path %', r.proname, r.prosecdef, r.proconfig;
    end if;
  end loop;
  foreach v_role in array array['anon', 'authenticated', 'service_role'] loop
    if has_function_privilege(v_role, 'admin.comp_grant(uuid, timestamptz, text, text)', 'execute')
       or has_function_privilege(v_role, 'admin.comp_extend(uuid, timestamptz)', 'execute')
       or has_table_privilege(v_role, 'admin.testers', 'select') then
      raise exception 'structure: % holds a right on the guards file''s objects', v_role;
    end if;
  end loop;
  raise notice 'structure: ok';
end $$;

rollback;
