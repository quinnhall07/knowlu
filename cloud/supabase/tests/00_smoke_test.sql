-- The frame every pgTAP file shares (RLS pgTAP plan, D5), and proof that it holds: the migrations
-- were applied, the `.psql` includes ran inside this file's transaction, impersonation behaves as
-- PostgREST's does (C8), and the server speaks the English messages D3's probes read.
--
-- Values seen under a client role are kept in psql variables and asserted after `as_owner()`:
-- nothing pgTAP writes ever happens under a client role. The account is a fresh random UUID, never
-- printed.
begin;
\ir _setup.psql
\ir _matrix.psql

select plan(7);

select diag('server: ' || version());
select diag('session_user: ' || session_user);

select has_table('public', 'accounts', 'the migrations were applied: public.accounts exists');

select gen_random_uuid()::text as smoke_a \gset

-- C8: an authenticated session for A.
select tests.as_user(:'smoke_a'::uuid);
select coalesce(auth.uid()::text, '<null>') as seen_user_uid, current_user as seen_user_role \gset
select tests.as_owner();
select is(:'seen_user_uid'::text, :'smoke_a'::text, 'C8: auth.uid() is the account after as_user');
select is(:'seen_user_role'::text, 'authenticated', 'C8: current_user is authenticated after as_user');

-- C8: the anon key, no session.
select tests.as_anon();
select coalesce(auth.uid()::text, '<null>') as seen_anon_uid \gset
select tests.as_owner();
select is(:'seen_anon_uid'::text, '<null>', 'C8: auth.uid() is null after as_anon');

-- C8: back to the owner, from a session for A.
select tests.as_user(:'smoke_a'::uuid);
select tests.as_owner();
select is(auth.uid(), null, 'C8: auth.uid() is null after as_owner');
select is(current_user::text, session_user::text, 'C8: current_user is the session user after as_owner');

-- D3's probes tell a grant denial from an RLS denial by the English message text (r1 I2, r2 note 5).
select matches(
  current_setting('lc_messages'),
  '^(C|C\..*|POSIX|en.*)$',
  'server messages are English: lc_messages is C, C.*, POSIX or en*'
);

select * from finish();
rollback;
