-- No client role holds a table right beyond select, insert, update and delete, and no client role
-- holds EXECUTE on the two SECURITY INVOKER trigger functions.
--
-- `postgres`'s default privileges in `public` gave `anon` and `authenticated` every table right on
-- every table it created, so each table no migration revoked from them still carried TRUNCATE,
-- REFERENCES, TRIGGER and MAINTAIN for both. TRUNCATE is not subject to row-level security, so on a
-- table a client reads by policy it is the one that matters most; MAINTAIN is PG17's right of the
-- same kind. The first statement takes the four from every relation in `public`, so no list of
-- tables has to stay complete; the second takes them from those default privileges, so a table
-- created later does not regain them. Select, insert, update and delete are untouched.
--
-- `public.sync_notes_stamp_rev()` and `public.sync_usage_bump()` (20260912000100_sync.sql) are
-- trigger functions the client roles could EXECUTE. The trigger manager does not check EXECUTE on a
-- trigger's function, so both triggers keep firing for every caller; the functions themselves are
-- unchanged.
revoke truncate, references, trigger, maintain on all tables in schema public from anon, authenticated;
alter default privileges for role postgres in schema public
  revoke truncate, references, trigger, maintain on tables from anon, authenticated;
revoke execute on function public.sync_notes_stamp_rev() from public, anon, authenticated;
revoke execute on function public.sync_usage_bump() from public, anon, authenticated;
