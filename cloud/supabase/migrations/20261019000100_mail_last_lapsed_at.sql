-- Knowlu — email forwarding: `mail_addresses.last_lapsed_at`, each lapse recorded apart from the gap's
-- start (review of the 2026-10-19 lapsed-mailbox fix, #1; spec §4.6 step 3 and §4.11 step 3, amended).
--
-- `lapsed_since` keeps the FIRST lapse's instant until the device reports the gap covered, so a second
-- lapse that opens while a `covered` report is in flight is stored under the same stamp, and a `covered`
-- keyed on that stamp alone wiped it: the second gap's mail, deleted unread, was never recovered.
-- `mail-inbound` now sets `last_lapsed_at` on every move into `lapsed`; the device echoes the
-- `last_lapsed_at` it read before its pass, and `mail`'s `covered` clears `lapsed_since` only while both
-- stamps are still the ones it echoes. Nothing else reads or writes it but the export (D17).
--
-- Forward-only: 20261002000200_mail.sql is applied on staging and never edited. No new table, so no new
-- row-level security: `mail_addresses` keeps RLS on, no policy, every client role revoked.
alter table public.mail_addresses add column if not exists last_lapsed_at timestamptz;

-- A row lapsed before this file keeps its one known instant, so a `covered` for its gap can still match.
update public.mail_addresses set last_lapsed_at = lapsed_since
 where lapsed_since is not null and last_lapsed_at is null;

-- A stamped gap always names its last lapse, or no `covered` could ever clear it.
alter table public.mail_addresses add constraint mail_addresses_last_lapse_stamped
  check (lapsed_since is null or last_lapsed_at is not null);
