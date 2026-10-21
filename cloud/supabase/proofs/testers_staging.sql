-- L8 friends testers, the staging proof (docs/specs/2026-10-18-friends-testers-design.md §9 layer iii,
-- under its Amendment 2026-10-20). Not a migration: it lives outside migrations/ and is never deployed.
-- The controller runs it on staging after the lane's migration is pushed there and before the merge
-- (S7a), and on prod after the prod migration and before any function deploy that reads the view.
-- A failure at P0, P1 or P3 stops the lane.
--
-- One transaction that only ever rolls back: nothing it writes survives. It raises on the first
-- mismatch, and every result line is structure only (a case id, a status, a plan, a day count),
-- never an account id. Every address is an example.invalid placeholder.
--
-- Each account is made the way a real sign-up makes it: the script inserts only the auth.users row,
-- from S1's recorded column list (the lane ledger) with placeholder values, and lets the sign-up
-- triggers write the accounts row and the entitlements row at none. It leaves out three of S1's
-- columns: confirmed_at (generated from email_confirmed_at), the password hash (no case sets a
-- password: the view reads none) and recovery_sent_at (an email-code sign-up's only). A Stripe case
-- then updates its entitlements row, P0x and P3x delete it, and grants go through the admin functions.
-- The P cases are migrations/testers_cases.ts's CASES, verbatim (pinned by migrations_test.ts).
-- The A, R and V cases follow (S2f-b), ADMIN_CASES, ROUTE_CASES and SPEND_CASES verbatim; the
-- statements run as an API role (A7, A3s, R1) each run in a savepoint that puts the owner's role back.

begin;

-- The trigger precheck. The C1 body of handle_new_user raised without sign-up metadata and would
-- fail every case for a reason that is not the view; the C1b body raises nothing.
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
create function pg_temp.days(d jsonb) returns timestamptz
language sql stable as $$ select now() + make_interval(days => (d #>> '{}')::int) $$;

-- One sign-up: the auth.users row alone; on_auth_user_metadata and handle_new_user do the rest.
create function pg_temp.signup(p_email text) returns uuid
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

-- The view's answer for each account against the case's: one row (none for a deleted account) and
-- its status, plan and period end. A mismatch names the account by its place in the case.
create function pg_temp.p_view(label text, ids uuid[], want jsonb) returns void
language plpgsql as $$
declare
  w jsonb;
  r record;
  n int;
begin
  for i in 1 .. jsonb_array_length(want) loop
    w := want -> (i - 1);
    select count(*) into n from public.effective_entitlements v where v.account_id = ids[i];
    if jsonb_typeof(w) = 'null' then
      if n <> 0 then raise exception '%: account % has a view row, want none', label, i - 1; end if;
      continue;
    end if;
    if n <> 1 then raise exception '%: account % has % view rows, want one', label, i - 1, n; end if;
    select v.status, v.plan, v.current_period_end into r from public.effective_entitlements v where v.account_id = ids[i];
    if r.status is distinct from w ->> 'status' or r.plan is distinct from w ->> 'plan'
       or r.current_period_end is distinct from pg_temp.days(w -> 'periodEnd') then
      raise exception '%: account % reads % / % / % days, want % / % / % days', label, i - 1, r.status, r.plan,
        round((extract(epoch from r.current_period_end - now()) / 86400)::numeric, 3), w ->> 'status', w ->> 'plan', w ->> 'periodEnd';
    end if;
  end loop;
end $$;

-- admin.testers' row for each grant, in the case's grants order: §5's state and the paid flag.
create function pg_temp.p_testers(label text, ids uuid[], grants jsonb, want jsonb) returns void
language plpgsql as $$
declare
  n int;
  s text;
  p boolean;
begin
  for i in 1 .. jsonb_array_length(want) loop
    select count(*), min(t.state), bool_and(t.paid) into n, s, p
      from admin.testers t where t.account_id = ids[(grants -> (i - 1) ->> 'account')::int + 1];
    if n <> 1 or s is distinct from want -> (i - 1) ->> 'state' or p is distinct from (want -> (i - 1) ->> 'paid')::boolean then
      raise exception '%: grant % reads % rows, % / paid %, want % / paid %', label, i - 1, n, s, p,
        want -> (i - 1) ->> 'state', want -> (i - 1) ->> 'paid';
    end if;
  end loop;
end $$;

-- One P case: its accounts through the sign-up triggers, their rows, its grants through the admin
-- functions; the view (and admin.testers) before the calls where the case says; the calls, each in
-- its own subtransaction so an expected refusal ends only that call; the deletion; the view at the end.
create function pg_temp.p_case(c jsonb) returns void
language plpgsql as $$
declare
  ids uuid[] := '{}';
  x jsonb;
  v_id uuid;
  v_err text;
  v_state text;
  kept bigint;
begin
  for x in select value from jsonb_array_elements(c -> 'accounts') loop
    v_id := pg_temp.signup(coalesce(x ->> 'authEmail', x ->> 'email'));
    ids := ids || v_id;
    if not exists (select 1 from public.entitlements e where e.account_id = v_id and e.status = 'none'
                   and e.plan is null and e.current_period_end is null) then
      raise exception '%: the sign-up trigger wrote no none row', c ->> 'id';
    end if;
    if x -> 'authEmail' is not null then
      update public.accounts set email = x ->> 'email' where id = v_id;
    end if;
    if jsonb_typeof(x -> 'row') = 'null' then
      delete from public.entitlements where account_id = v_id;
    elsif x -> 'row' <> '{"status": "none", "plan": null, "periodEnd": null}'::jsonb then
      update public.entitlements
         set status = x -> 'row' ->> 'status', plan = x -> 'row' ->> 'plan', current_period_end = pg_temp.days(x -> 'row' -> 'periodEnd')
       where account_id = v_id;
    end if;
  end loop;
  -- Since 20261020000600_comp_guards.sql, comp_grant refuses a date not after now(), so a grant the case
  -- puts in the past (P8, P12, P13) is made a day out and then backdated by the owner, as A4 does.
  for x in select value from jsonb_array_elements(c -> 'grants') loop
    v_id := ids[(x ->> 'account')::int + 1];
    perform admin.comp_grant(v_id, greatest(pg_temp.days(x -> 'endsIn'), now() + interval '1 day'), 'friends', null);
    if (x ->> 'endsIn')::int <= 0 then
      update public.comp_grants set ends_at = pg_temp.days(x -> 'endsIn') where account_id = v_id and ended_at is null;
    end if;
    if x -> 'endedAgo' is not null then
      perform admin.comp_end(v_id, 'done');
      update public.comp_grants set ended_at = now() - make_interval(days => (x ->> 'endedAgo')::int)
       where account_id = v_id and ended_at is not null;
    end if;
  end loop;
  if c -> 'expectBefore' is not null then perform pg_temp.p_view(c ->> 'id' || ' before', ids, c -> 'expectBefore'); end if;
  if c -> 'testersBefore' is not null then
    perform pg_temp.p_testers(c ->> 'id' || ' before', ids, c -> 'grants', c -> 'testersBefore');
  end if;
  for x in select value from jsonb_array_elements(coalesce(c -> 'calls', '[]')) loop
    v_id := case when x ->> 'account' = 'unknown' then gen_random_uuid() else ids[(x ->> 'account')::int + 1] end;
    v_err := null;
    begin
      if x ->> 'fn' = 'comp_grant' then
        perform admin.comp_grant(v_id, pg_temp.days(x -> 'endsIn'), 'friends', null);
      else
        perform admin.comp_extend(v_id, pg_temp.days(x -> 'endsIn'));
      end if;
    exception when others then
      v_err := sqlerrm;
      v_state := sqlstate;
    end;
    if x -> 'refused' is null and v_err is not null then
      raise exception '%: % refused (sqlstate %), want it to succeed', c ->> 'id', x ->> 'fn', v_state;
    elsif x -> 'refused' is not null and v_err is null then
      raise exception '%: % succeeded, want it refused', c ->> 'id', x ->> 'fn';
    elsif jsonb_typeof(x -> 'refused') = 'string' and position(x ->> 'refused' in v_err) = 0 then
      raise exception '%: % refused (sqlstate %) without naming %', c ->> 'id', x ->> 'fn', v_state, x ->> 'refused';
    end if;
  end loop;
  if c -> 'deleteAccount' is not null then
    v_id := ids[(c ->> 'deleteAccount')::int + 1];
    select (select count(*) from public.comp_grants g where g.account_id = any(ids) and g.account_id <> v_id)
         + (select count(*) from public.comp_audit u where u.account_id = any(ids) and u.account_id <> v_id) into kept;
    delete from auth.users where id = v_id;
    if kept <> (select count(*) from public.comp_grants g where g.account_id = any(ids) and g.account_id <> v_id)
              + (select count(*) from public.comp_audit u where u.account_id = any(ids) and u.account_id <> v_id) then
      raise exception '%: the deletion took another account''s grant or audit rows', c ->> 'id';
    end if;
  end if;
  perform pg_temp.p_view(c ->> 'id', ids, c -> 'expect');
  if c -> 'testers' is not null then perform pg_temp.p_testers(c ->> 'id', ids, c -> 'grants', c -> 'testers'); end if;
end $$;

-- The P cases, in the case table's order (the none-row cases first); the first mismatch raises.
do $$
declare
  c jsonb;
  n int := 0;
begin
  for c in select value from jsonb_array_elements($cases$
[
{"id":"P0","given":"sign-up none row, no grant: every unpaid account","accounts":[{"email":"p0-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[],"expect":[{"status":"none","plan":null,"periodEnd":null}]},
{"id":"P3","given":"sign-up none row, open grant: every real tester","accounts":[{"email":"p3-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"P8","given":"sign-up none row, grant past ends_at","accounts":[{"email":"p8-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":-1}],"expect":[{"status":"none","plan":null,"periodEnd":null}],"testers":[{"state":"expired","paid":false}]},
{"id":"P9","given":"sign-up none row, grant ended early","accounts":[{"email":"p9-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30,"endedAgo":1}],"expect":[{"status":"none","plan":null,"periodEnd":null}],"testers":[{"state":"ended","paid":false}]},
{"id":"P12","given":"a second grant while one is open, live (first) or expired and never ended (second)","accounts":[{"email":"p12-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"p12-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30},{"account":1,"endsIn":-1}],"calls":[{"fn":"comp_grant","account":0,"endsIn":60,"refused":"admin.comp_extend"},{"fn":"comp_grant","account":1,"endsIn":60,"refused":"admin.comp_extend"}],"expect":[{"status":"active","plan":"tester","periodEnd":30},{"status":"none","plan":null,"periodEnd":null}]},
{"id":"P13","given":"an expired open grant extended (a past date refused first); an ended grant not extendable","accounts":[{"email":"p13-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"p13-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":-1},{"account":1,"endsIn":30,"endedAgo":1}],"calls":[{"fn":"comp_extend","account":0,"endsIn":-2,"refused":true},{"fn":"comp_extend","account":0,"endsIn":45},{"fn":"comp_extend","account":1,"endsIn":45,"refused":true}],"expectBefore":[{"status":"none","plan":null,"periodEnd":null},{"status":"none","plan":null,"periodEnd":null}],"expect":[{"status":"active","plan":"tester","periodEnd":45},{"status":"none","plan":null,"periodEnd":null}],"testersBefore":[{"state":"expired","paid":false},{"state":"ended","paid":false}],"testers":[{"state":"live","paid":false},{"state":"ended","paid":false}]},
{"id":"P14","given":"two sign-up accounts, a grant on the first only","accounts":[{"email":"p14-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"p14-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30},{"status":"none","plan":null,"periodEnd":null}]},
{"id":"P15","given":"two accounts carrying one accounts.email, a grant on the first; then the second is deleted","accounts":[{"email":"p15-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"p15-0@example.invalid","authEmail":"p15-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30}],"deleteAccount":1,"expectBefore":[{"status":"active","plan":"tester","periodEnd":30},{"status":"none","plan":null,"periodEnd":null}],"expect":[{"status":"active","plan":"tester","periodEnd":30},null]},
{"id":"P0x","given":"no entitlements row, no grant","accounts":[{"email":"p0x-0@example.invalid","row":null}],"grants":[],"expect":[{"status":"none","plan":null,"periodEnd":null}]},
{"id":"P3x","given":"no entitlements row, open grant","accounts":[{"email":"p3x-0@example.invalid","row":null}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"P1","given":"Stripe active, no grant: the paying-student guard","accounts":[{"email":"p1-0@example.invalid","row":{"status":"active","plan":"monthly","periodEnd":20}}],"grants":[],"expect":[{"status":"active","plan":"monthly","periodEnd":20}]},
{"id":"P2","given":"Stripe active, open grant: paid wins","accounts":[{"email":"p2-0@example.invalid","row":{"status":"active","plan":"academic_year","periodEnd":200}}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"academic_year","periodEnd":200}],"testers":[{"state":"live","paid":true}]},
{"id":"P5","given":"Stripe canceled, open grant","accounts":[{"email":"p5-0@example.invalid","row":{"status":"canceled","plan":"monthly","periodEnd":-3}}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"P6","given":"Stripe past_due, open grant","accounts":[{"email":"p6-0@example.invalid","row":{"status":"past_due","plan":"monthly","periodEnd":2}}],"grants":[{"account":0,"endsIn":30}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"P16","given":"comp_grant naming an id with no accounts row","accounts":[],"grants":[],"calls":[{"fn":"comp_grant","account":"unknown","endsIn":30,"refused":"foreign key"}],"expect":[]}
]
$cases$::jsonb) loop
    perform pg_temp.p_case(c);
    n := n + 1;
    raise notice '%: ok', c ->> 'id';
  end loop;
  raise notice 'P cases: all % passed', n;
end $$;

-- S2f-b: the A, R and V cases (the V rows as the controller's ruling of 2026-10-20 on est_usd
-- restates V1). Each case makes its own accounts by sign-up, as the P cases do; a refusal is matched
-- by its message and only its sqlstate is printed.

-- The case's accounts, by sign-up; these cases use only the sign-up none row.
create function pg_temp.signups(label text, accounts jsonb) returns uuid[]
language plpgsql as $$
declare
  ids uuid[] := '{}';
  x jsonb;
begin
  for x in select value from jsonb_array_elements(accounts) loop
    if x -> 'row' is distinct from '{"status": "none", "plan": null, "periodEnd": null}'::jsonb or x -> 'authEmail' is not null then
      raise exception '%: this runner makes sign-up accounts only', label;
    end if;
    ids := ids || pg_temp.signup(x ->> 'email');
    if not exists (select 1 from public.entitlements e where e.account_id = ids[cardinality(ids)] and e.status = 'none') then
      raise exception '%: the sign-up trigger wrote no none row', label;
    end if;
  end loop;
  return ids;
end $$;

-- A call's outcome against the case's: success where refused is absent, else a refusal carrying it.
create function pg_temp.outcome(label text, refused jsonb, err text) returns void
language plpgsql as $$
begin
  if refused is null and err is not null then
    raise exception '%: refused (sqlstate %), want it to succeed', label, left(err, 5);
  elsif refused is not null and err is null then
    raise exception '%: succeeded, want it refused', label;
  elsif jsonb_typeof(refused) = 'string' and position(refused #>> '{}' in err) = 0 then
    raise exception '%: refused (sqlstate %) without naming %', label, left(err, 5), refused #>> '{}';
  end if;
end $$;

-- One statement as another role, in its own savepoint: the role is set local and the caller's put
-- back; a refusal rolls the savepoint back, the role switch with it. Null on success, else the error.
create function pg_temp.as_role(p_role text, p_sql text, p_id uuid) returns text
language plpgsql as $$
declare
  v_prev text := current_user;
begin
  execute format('set local role %I', p_role);
  execute p_sql using p_id;
  execute format('set local role %I', v_prev);
  return null;
exception when others then
  return sqlstate || ' ' || sqlerrm;
end $$;

-- An owner call from an A case, in its own savepoint. Null on success, else the error.
create function pg_temp.a_call(x jsonb, p_id uuid) returns text
language plpgsql as $$
begin
  if x ->> 'fn' = 'comp_grant' then
    perform admin.comp_grant(p_id, pg_temp.days(x -> 'endsIn'), 'friends', null);
  elsif x ->> 'fn' = 'comp_extend' then
    perform admin.comp_extend(p_id, pg_temp.days(x -> 'endsIn'));
  else
    perform admin.comp_end(p_id, x ->> 'reason');
  end if;
  return null;
exception when others then
  return sqlstate || ' ' || sqlerrm;
end $$;

-- A7's statements, by op and table; $1 is the case's account.
create function pg_temp.a7_sql(op text, tbl text, cols jsonb) returns text
language plpgsql as $$
begin
  if op = 'insert' and tbl = 'comp_grants' then
    return 'insert into public.comp_grants (account_id, cohort, ends_at) values ($1, ''friends'', now() + interval ''1 day'')';
  elsif op = 'insert' then
    return 'insert into public.comp_audit (account_id, action, ends_at, cohort) values ($1, ''grant'', now(), ''friends'')';
  elsif op = 'update' then
    return format('update public.%I set account_id = account_id where account_id = $1', tbl);
  elsif op = 'delete' then
    return format('delete from public.%I where account_id = $1', tbl);
  elsif op = 'call' then
    return 'select admin.comp_grant($1, now() + interval ''30 days'', ''friends'', null)';
  end if;
  return format('select %s from public.%I where account_id = $1', coalesce(
    (select string_agg(quote_ident(v), ', ') from jsonb_array_elements_text(cols) as t(v)), 'count(*)'), tbl);
end $$;

-- One A case: its owner calls, each followed by the account's audit actions; A7's statements as the
-- service role; A5's billing list; A3's route_daily rows; the deletion, by the owner from auth.users
-- (A3) or as the purge does it, under the service role (A3s); A4's and A6's backdating and the sweep;
-- each account's rows; the view.
create function pg_temp.a_case(c jsonb) returns void
language plpgsql as $$
declare
  ids uuid[] := pg_temp.signups(c ->> 'id', c -> 'accounts');
  x jsonb;
  v_id uuid;
  got jsonb;
begin
  for x in select value from jsonb_array_elements(c -> 'calls') loop
    if x ->> 'account' = 'unknown' then v_id := gen_random_uuid(); else v_id := ids[(x ->> 'account')::int + 1]; end if;
    perform pg_temp.outcome(c ->> 'id' || ' ' || (x ->> 'fn'), x -> 'refused', pg_temp.a_call(x, v_id));
    if array(select u.action from public.comp_audit u where u.account_id = v_id order by 1)
       is distinct from array(select a.v from jsonb_array_elements_text(x -> 'audit') as a(v) order by 1)
       or exists (select 1 from public.comp_audit u where u.account_id = v_id and not exists
                    (select 1 from public.comp_grants g where g.id = u.grant_id and g.account_id = v_id)) then
      raise exception '%: % leaves the wrong audit rows', c ->> 'id', x ->> 'fn';
    end if;
  end loop;
  for x in select value from jsonb_array_elements(coalesce(c -> 'asServiceRole', '[]')) loop
    perform pg_temp.outcome(format('%s %s %s', c ->> 'id', x ->> 'op', x ->> 'on'), x -> 'refused',
      pg_temp.as_role('service_role', pg_temp.a7_sql(x ->> 'op', x ->> 'on', x -> 'columns'), ids[1]));
  end loop;
  for i in 1 .. jsonb_array_length(coalesce(c -> 'billingSubscribers', '[]')) loop
    if exists (select 1 from public.billing_subscribers b where b.account_id = ids[i]) <> (c -> 'billingSubscribers' ->> (i - 1))::boolean then
      raise exception '%: account % is wrongly in or out of billing_subscribers', c ->> 'id', i - 1;
    end if;
  end loop;
  for i in 1 .. jsonb_array_length(coalesce(c -> 'routeDaily', '[]')) loop
    if (c -> 'routeDaily' ->> (i - 1))::boolean then perform public.count_route(ids[i], 'judge-task'); end if;
  end loop;
  if c -> 'deleteAccount' is not null then
    v_id := ids[(c -> 'deleteAccount' ->> 'account')::int + 1];
    if c -> 'deleteAccount' ->> 'as' = 'owner' then
      delete from auth.users where id = v_id;
    else
      perform pg_temp.outcome(c ->> 'id' || ' delete as service_role', null,
        pg_temp.as_role('service_role', 'delete from public.accounts where id = $1', v_id));
    end if;
  end if;
  for x in select value from jsonb_array_elements(coalesce(c -> 'sweep', '[]')) loop
    v_id := ids[(x ->> 'account')::int + 1];
    if x -> 'endsIn' is not null then
      update public.comp_grants set ends_at = pg_temp.days(x -> 'endsIn') where account_id = v_id;
      if not found then raise exception '%: no grant to backdate', c ->> 'id'; end if;
    end if;
    if x -> 'endedAgo' is not null then
      update public.comp_grants set ended_at = now() - make_interval(days => (x ->> 'endedAgo')::int)
       where account_id = v_id and ended_at is not null;
      if not found then raise exception '%: no ended grant to backdate', c ->> 'id'; end if;
    end if;
    if x -> 'auditAgo' is not null then
      update public.comp_audit set at = now() - (x ->> 'auditAgo')::interval where account_id = v_id;
      if not found then raise exception '%: no audit row to backdate', c ->> 'id'; end if;
    end if;
  end loop;
  if c -> 'sweep' is not null then perform admin.comp_sweep(); end if;
  for i in 1 .. jsonb_array_length(coalesce(c -> 'rows', '[]')) loop
    got := jsonb_build_object(
      'grants', (select count(*) from public.comp_grants g where g.account_id = ids[i]),
      'audit', (select count(*) from public.comp_audit u where u.account_id = ids[i]),
      'unlinked', (select count(*) from public.comp_audit u where u.account_id = ids[i] and u.grant_id is null),
      'routeDaily', (select count(*) from public.route_daily r where r.account_id = ids[i]));
    if got <> c -> 'rows' -> (i - 1) then
      raise exception '%: account % holds %, want %', c ->> 'id', i - 1, got, c -> 'rows' -> (i - 1);
    end if;
  end loop;
  perform pg_temp.p_view(c ->> 'id', ids, c -> 'expect');
end $$;

-- R1: count_route's steps, each as its role in its own savepoint; then the account's route_daily
-- calls for today.
create function pg_temp.r_case(c jsonb) returns void
language plpgsql as $$
declare
  ids uuid[] := pg_temp.signups(c ->> 'id', c -> 'accounts');
  x jsonb;
begin
  for x in select value from jsonb_array_elements(c -> 'steps') loop
    perform pg_temp.outcome(format('%s count_route %s as %s', c ->> 'id', x ->> 'route', x ->> 'as'), x -> 'refused',
      pg_temp.as_role(x ->> 'as', format('select public.count_route($1, %L)', x ->> 'route'), ids[1]));
  end loop;
  if (select coalesce(jsonb_object_agg(r.route, r.calls), '{}') from public.route_daily r
       where r.account_id = ids[1] and r.day = current_date) is distinct from c -> 'calls' then
    raise exception '%: route_daily does not read %', c ->> 'id', c -> 'calls';
  end if;
end $$;

-- One V case: its grants; the per-kind price pin set inside this rolled-back transaction, so each
-- dollar figure is the case's hand-computed sum and never read back from the pin; its usage_daily and
-- budget_alerts rows; then tester_usage_daily's rows and counts, and tester_spend_monthly's rows.
create function pg_temp.v_case(c jsonb) returns void
language plpgsql as $$
declare
  ids uuid[] := pg_temp.signups(c ->> 'id', c -> 'accounts');
  x jsonb;
  k text;
  r record;
  n int;
  v_usd numeric;
  v_alert numeric;
begin
  for x in select value from jsonb_array_elements(c -> 'grants') loop
    perform admin.comp_grant(ids[(x ->> 'account')::int + 1], pg_temp.days(x -> 'endsIn'), 'friends', null);
  end loop;
  for k, x in select key, value from jsonb_each(c -> 'prices') loop
    update public.models set usd_per_m_in = (x ->> 'in')::numeric, usd_per_m_out = (x ->> 'out')::numeric where kind = k;
    if not found then raise exception '%: no price pin for kind %', c ->> 'id', k; end if;
  end loop;
  insert into public.usage_daily (account_id, day, kind, calls, refunds, in_tokens, out_tokens)
  select ids[(s.v ->> 'account')::int + 1], (s.v ->> 'day')::date, s.v ->> 'kind', (s.v ->> 'calls')::int,
         (s.v ->> 'refunds')::int, (s.v ->> 'inTokens')::bigint, (s.v ->> 'outTokens')::bigint
    from jsonb_array_elements(c -> 'usage') as s(v);
  insert into public.budget_alerts (account_id, month, usd)
  select ids[(b.v ->> 'account')::int + 1], (b.v ->> 'month')::date, (b.v ->> 'usd')::numeric
    from jsonb_array_elements(coalesce(c -> 'alerts', '[]')) as b(v);
  for i in 1 .. jsonb_array_length(c -> 'usageDays') loop
    select count(*) into n from admin.tester_usage_daily d where d.account_id = ids[i];
    if n <> (c -> 'usageDays' ->> (i - 1))::int then
      raise exception '%: account % has % tester_usage_daily rows, want %', c ->> 'id', i - 1, n, c -> 'usageDays' ->> (i - 1);
    end if;
    for r in select d.day, d.usage_by_kind from admin.tester_usage_daily d where d.account_id = ids[i] loop
      if r.usage_by_kind is distinct from (
           select jsonb_object_agg(s.v ->> 'kind', jsonb_build_object('calls', (s.v ->> 'calls')::int,
                    'refunds', (s.v ->> 'refunds')::int, 'in_tokens', (s.v ->> 'inTokens')::bigint,
                    'out_tokens', (s.v ->> 'outTokens')::bigint))
             from jsonb_array_elements(c -> 'usage') as s(v)
            where (s.v ->> 'account')::int = i - 1 and (s.v ->> 'day')::date = r.day) then
        raise exception '%: account % on % reads %, not the seeded counts', c ->> 'id', i - 1, r.day, r.usage_by_kind;
      end if;
    end loop;
  end loop;
  for x in select value from jsonb_array_elements(c -> 'spend') loop
    if x ->> 'account' = 'total' then
      select count(*), min(t.est_usd), min(t.alert_usd) into n, v_usd, v_alert
        from admin.tester_spend_monthly t where t.is_total and t.month = (x ->> 'month')::date;
    else
      select count(*), min(t.est_usd), min(t.alert_usd) into n, v_usd, v_alert
        from admin.tester_spend_monthly t
       where not t.is_total and t.account_id = ids[(x ->> 'account')::int + 1] and t.month = (x ->> 'month')::date;
    end if;
    if n <> 1 or round(v_usd, 9) is distinct from round((x ->> 'estUsd')::numeric, 9)
       or v_alert is distinct from (x ->> 'alertUsd')::numeric then
      raise exception '%: % on % reads % rows, usd % (alert %), want usd % (alert %)', c ->> 'id', x ->> 'account',
        x ->> 'month', n, v_usd, v_alert, x ->> 'estUsd', x ->> 'alertUsd';
    end if;
  end loop;
  select count(*) into n from admin.tester_spend_monthly t where t.account_id = any(ids);
  if n <> (select count(*) from jsonb_array_elements(c -> 'spend') as s(v) where s.v ->> 'account' <> 'total') then
    raise exception '%: tester_spend_monthly has % rows for the case''s accounts', c ->> 'id', n;
  end if;
end $$;

-- A case list in its table's order; the first mismatch raises, and each case prints its id alone.
create function pg_temp.run(runner text, cases jsonb) returns void
language plpgsql as $$
declare
  c jsonb;
begin
  for c in select value from jsonb_array_elements(cases) loop
    execute format('select pg_temp.%I($1)', runner) using c;
    raise notice '%: ok', c ->> 'id';
  end loop;
  raise notice '%: all % passed', runner, jsonb_array_length(cases);
end $$;

-- The A, R and V cases: testers_cases.ts's ADMIN_CASES, ROUTE_CASES and SPEND_CASES, verbatim.
select pg_temp.run('a_case', $admin$
[
{"id":"A1","given":"comp_grant, comp_extend, comp_end: one audit row each, on the grant's account","accounts":[{"email":"a1-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]},{"fn":"comp_extend","account":0,"endsIn":60,"audit":["grant","extend"]},{"fn":"comp_end","account":0,"reason":"done","audit":["grant","extend","end"]}],"expect":[{"status":"none","plan":null,"periodEnd":null}]},
{"id":"A2","given":"a refused comp_grant writes no audit row: a second open grant (P12), an unknown id (P16)","accounts":[{"email":"a2-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":0,"endsIn":60,"refused":"admin.comp_extend","audit":["grant"]},{"fn":"comp_grant","account":"unknown","endsIn":30,"refused":"foreign key","audit":[]}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"A5","given":"a comped account is absent from billing_subscribers","accounts":[{"email":"a5-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]}],"billingSubscribers":[false],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"A7","given":"as service_role: each write on both tables and the grant call refused; the three selects succeed","accounts":[{"email":"a7-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]}],"asServiceRole":[{"op":"insert","on":"comp_grants","refused":"permission denied"},{"op":"update","on":"comp_grants","refused":"permission denied"},{"op":"delete","on":"comp_grants","refused":"permission denied"},{"op":"insert","on":"comp_audit","refused":"permission denied"},{"op":"update","on":"comp_audit","refused":"permission denied"},{"op":"delete","on":"comp_audit","refused":"permission denied"},{"op":"call","on":"admin.comp_grant","refused":"permission denied"},{"op":"select","on":"comp_grants"},{"op":"select","on":"comp_audit"},{"op":"select","on":"effective_entitlements","columns":["account_id","plan","status","current_period_end"]}],"expect":[{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"A3","given":"an account with a grant and a route_daily row deleted as the owner","accounts":[{"email":"a3-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a3-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":1,"endsIn":30,"audit":["grant"]}],"routeDaily":[true,true],"deleteAccount":{"account":0,"as":"owner"},"rows":[{"grants":0,"audit":0,"unlinked":0,"routeDaily":0},{"grants":1,"audit":1,"unlinked":0,"routeDaily":1}],"expect":[null,{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"A3s","given":"an account with a grant and a route_daily row deleted as production does","accounts":[{"email":"a3s-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a3s-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":1,"endsIn":30,"audit":["grant"]}],"routeDaily":[true,true],"deleteAccount":{"account":0,"as":"service_role"},"rows":[{"grants":0,"audit":0,"unlinked":0,"routeDaily":0},{"grants":1,"audit":1,"unlinked":0,"routeDaily":1}],"expect":[null,{"status":"active","plan":"tester","periodEnd":30}]},
{"id":"A4","given":"the sweep: grants closed 31 days ago (ended, expired, expired then ended) deleted, 29 kept; audit stays","accounts":[{"email":"a4-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a4-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a4-2@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a4-3@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a4-4@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":1,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":2,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":3,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":4,"endsIn":30,"audit":["grant"]},{"fn":"comp_end","account":0,"reason":"done","audit":["grant","end"]},{"fn":"comp_end","account":1,"reason":"done","audit":["grant","end"]},{"fn":"comp_end","account":4,"reason":"done","audit":["grant","end"]}],"sweep":[{"account":0,"endedAgo":31},{"account":1,"endedAgo":29},{"account":2,"endsIn":-31},{"account":3,"endsIn":-29},{"account":4,"endsIn":-31,"endedAgo":10}],"rows":[{"grants":0,"audit":2,"unlinked":2,"routeDaily":0},{"grants":1,"audit":2,"unlinked":0,"routeDaily":0},{"grants":0,"audit":1,"unlinked":1,"routeDaily":0},{"grants":1,"audit":1,"unlinked":0,"routeDaily":0},{"grants":0,"audit":2,"unlinked":2,"routeDaily":0}],"expect":[{"status":"none","plan":null,"periodEnd":null},{"status":"none","plan":null,"periodEnd":null},{"status":"none","plan":null,"periodEnd":null},{"status":"none","plan":null,"periodEnd":null},{"status":"none","plan":null,"periodEnd":null}]},
{"id":"A6","given":"the sweep: an audit row three years and a day old deleted, three years less a day kept; the grants stay","accounts":[{"email":"a6-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"a6-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"calls":[{"fn":"comp_grant","account":0,"endsIn":30,"audit":["grant"]},{"fn":"comp_grant","account":1,"endsIn":30,"audit":["grant"]}],"sweep":[{"account":0,"auditAgo":"3 years 1 day"},{"account":1,"auditAgo":"3 years -1 day"}],"rows":[{"grants":1,"audit":0,"unlinked":0,"routeDaily":0},{"grants":1,"audit":1,"unlinked":0,"routeDaily":0}],"expect":[{"status":"active","plan":"tester","periodEnd":30},{"status":"active","plan":"tester","periodEnd":30}]}
]
$admin$::jsonb);

select pg_temp.run('r_case', $route$
[
{"id":"R1","given":"count_route twice, then a bad route, then as authenticated","accounts":[{"email":"r1-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"steps":[{"route":"judge-task","as":"service_role"},{"route":"judge-task","as":"service_role"},{"route":"Judge_Task","as":"service_role","refused":"check constraint"},{"route":"judge-task","as":"authenticated","refused":"permission denied"}],"calls":{"judge-task":2}}
]
$route$::jsonb);

select pg_temp.run('v_case', $spend$
[
{"id":"V1","given":"seeded usage_daily rows at known token counts: est_usd is the hand-computed sum; the daily view shows the counts","accounts":[{"email":"v1-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30}],"prices":{"task":{"in":1,"out":5},"mail_claim":{"in":0.5,"out":2}},"usage":[{"account":0,"day":"2025-01-06","kind":"task","calls":4,"refunds":1,"inTokens":1000000,"outTokens":200000},{"account":0,"day":"2025-01-06","kind":"mail_claim","calls":2,"refunds":0,"inTokens":400000,"outTokens":100000},{"account":0,"day":"2025-01-20","kind":"task","calls":1,"refunds":0,"inTokens":300000,"outTokens":60000}],"usageDays":[2],"spend":[{"account":0,"month":"2025-01-01","estUsd":3,"alertUsd":null},{"account":"total","month":"2025-01-01","estUsd":3,"alertUsd":null}]},
{"id":"V2","given":"two testers, one with a budget_alerts row: the total row is the sum of the two; the alert shows on its month","accounts":[{"email":"v2-0@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}},{"email":"v2-1@example.invalid","row":{"status":"none","plan":null,"periodEnd":null}}],"grants":[{"account":0,"endsIn":30},{"account":1,"endsIn":30}],"prices":{"task":{"in":1,"out":5}},"usage":[{"account":0,"day":"2025-02-03","kind":"task","calls":1,"refunds":0,"inTokens":1000000,"outTokens":0},{"account":1,"day":"2025-02-04","kind":"task","calls":3,"refunds":0,"inTokens":0,"outTokens":400000}],"alerts":[{"account":1,"month":"2025-02-01","usd":2}],"usageDays":[1,1],"spend":[{"account":0,"month":"2025-02-01","estUsd":1,"alertUsd":null},{"account":1,"month":"2025-02-01","estUsd":2,"alertUsd":2},{"account":"total","month":"2025-02-01","estUsd":3,"alertUsd":null}]}
]
$spend$::jsonb);

rollback;
