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
-- S2f-b appends the A, R and V cases before the final rollback.

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
  for x in select value from jsonb_array_elements(c -> 'grants') loop
    v_id := ids[(x ->> 'account')::int + 1];
    perform admin.comp_grant(v_id, pg_temp.days(x -> 'endsIn'), 'friends', null);
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

rollback;
