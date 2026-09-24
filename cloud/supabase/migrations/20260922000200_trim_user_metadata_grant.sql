-- The belt (R-C1b-exec-8 follow-up, re-review): supabase_auth_admin, GoTrue's own role, gets EXECUTE explicitly on both 20260922000100's functions.
-- Postgres's trigger manager does not check EXECUTE to fire a trigger's function, so this changes no runtime behavior — only defense in depth.
grant execute on function public.trimmed_user_metadata(jsonb) to supabase_auth_admin;
grant execute on function public.trim_user_metadata() to supabase_auth_admin;
