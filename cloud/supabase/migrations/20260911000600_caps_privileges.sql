-- Knowlu C2, Task 12 fix round 2 (ruling R-C2-E48). No C2 SQL function is reachable from a
-- browser: the cap store (`charge_call`, `record_tokens`, `enforce_budget`, 20260911000100) is the
-- service's own, keyed by the account the service names, and belongs to this same uniform rule —
-- fix 1's `-- rpc: authenticated by design` marker on these three was wrong (they are called only
-- by `_shared/judge_caps.ts` through the service role; nothing in `app/src` or `engine/src` calls
-- them with a user session), so they are revoked here instead of marked.

revoke execute on function public.charge_call(uuid, text, integer) from public, anon, authenticated;
grant execute on function public.charge_call(uuid, text, integer) to service_role;

revoke execute on function public.record_tokens(uuid, text, bigint, bigint) from public, anon, authenticated;
grant execute on function public.record_tokens(uuid, text, bigint, bigint) to service_role;

revoke execute on function public.enforce_budget(uuid, numeric) from public, anon, authenticated;
grant execute on function public.enforce_budget(uuid, numeric) to service_role;
