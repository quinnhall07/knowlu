-- Knowlu C1b sign-in fix (controller ruling R-C1b-exec-8) — a Google sign-in's access token does
-- not fit where the desktop keeps it.
--
-- Windows Credential Manager caps a secret at 2,560 bytes (1,280 UTF-16 characters —
-- `app/src/account.rs`'s MAX_SESSION_CHARS), the app stores the session JSON there, and the
-- engine reads the same credential for its own cloud calls. GoTrue copies Google's profile claims
-- (avatar_url, picture, full_name, name, iss, provider_id, sub, email, email_verified,
-- phone_verified) into auth.users.raw_user_meta_data, and that object rides in every access token
-- as `user_metadata` — big enough, with a profile photo URL and a display name, to push a Google
-- sign-in's token past the cap. An email-only account's token (about 970 characters) carries none
-- of those claims and fits today.
--
-- The fix keeps raw_user_meta_data down to four keys — email, email_verified, phone_verified,
-- sub — before it ever reaches a token. Everything else Google sends here — avatar_url, picture,
-- full_name, name, iss, provider_id, and anything a future provider adds — is dropped from THIS
-- column alone: auth.identities.identity_data still holds the provider's full claim set, so
-- nothing the privacy page discloses stops being collected, only what rides in the token shrinks.
--
-- No app or engine change accompanies this: the app's MAX_SESSION_CHARS cap stays exactly as it
-- is; this migration makes every token small enough to fit it instead.
--
-- The reduction is written once, as an immutable SQL helper, and used both by the trigger that
-- guards every future row and by the one-time UPDATE below that catches the rows already here.
create or replace function public.trimmed_user_metadata(p_metadata jsonb)
returns jsonb
language sql
immutable
set search_path = public, pg_temp
as $$
  select case
    when p_metadata is null then null
    else coalesce(
      (
        select jsonb_object_agg(kv.key, kv.value)
          from jsonb_each(p_metadata) as kv(key, value)
         where kv.key in ('email', 'email_verified', 'phone_verified', 'sub')
      ),
      '{}'::jsonb
    )
  end;
$$;
revoke execute on function public.trimmed_user_metadata(jsonb) from public, anon, authenticated;

-- The trigger function: SECURITY DEFINER (ownership like C1's handle_new_user,
-- 20260910000100_accounts.sql) so it runs with the migration owner's privileges rather than
-- whatever role GoTrue's insert or an admin's update runs as, and a pinned search_path so
-- `trimmed_user_metadata` always resolves to this schema's own. Unlike handle_new_user it never
-- reads or writes a table — NEW.raw_user_meta_data is reassigned in place — so there is nothing
-- here that needs a grant to the auth admin role: handle_new_user carries none either, for the
-- same reason, a BEFORE trigger on auth.users runs regardless of execute privilege on its own
-- function.
create or replace function public.trim_user_metadata() returns trigger
language plpgsql security definer set search_path = public, pg_temp as $$
begin
  new.raw_user_meta_data := public.trimmed_user_metadata(new.raw_user_meta_data);
  return new;
end;
$$;
revoke execute on function public.trim_user_metadata() from public, anon, authenticated;

create trigger on_auth_user_metadata
  before insert or update of raw_user_meta_data on auth.users
  for each row execute function public.trim_user_metadata();

-- The rows that already exist: every account created before this migration keeps whatever GoTrue
-- wrote at sign-up, including any of the six dropped keys. One pass, the same reduction.
update auth.users
   set raw_user_meta_data = public.trimmed_user_metadata(raw_user_meta_data)
 where raw_user_meta_data is not null;
