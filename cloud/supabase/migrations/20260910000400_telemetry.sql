-- Knowlu C1, Task 8 — analytics (a) and (b). Spec §6, decided under D5.
--
-- Two properties are structural, not procedural:
--   * `object_id` is NOT NULL DEFAULT '' so the dedup key is a plain unique constraint. A retry after
--     a dropped connection re-sends the same rows, and the second copy must land on the first.
--   * The reading view refuses any slice under ten accounts. A minimum cohort that lives in a
--     dashboard query is a minimum cohort somebody forgets; this one is in the schema.
create table public.telemetry_events (
  id          bigint generated always as identity primary key,
  account_id  uuid not null references public.accounts (id) on delete cascade,
  ts          timestamptz not null,
  session     text not null,
  view        text not null,
  action      text not null,
  object_id   text not null default '',
  object_kind text,
  ms          integer,
  received_at timestamptz not null default now(),
  constraint telemetry_events_once unique (account_id, session, ts, action, object_id)
);

create table public.corrections (
  id          bigint generated always as identity primary key,
  account_id  uuid not null references public.accounts (id) on delete cascade,
  ts          timestamptz not null,
  item_id     text not null,
  field       text not null,
  -- **Nullable, and they stay nullable.** A `course` correction deliberately carries neither value
  -- (spec §6's content rule), and C2's eval suite reads these rows — R-X-2 fixes this shape as the
  -- authority: C2 alters this table, it never creates it, and it adds no check constraint to `kind`.
  ours        text,
  theirs      text,
  -- The note kind, from the note's own folder: task | approval | course | info | issue | archive.
  kind        text not null,
  -- **R-X-3.** The judged item as it was sent, so C2's eval suite can replay a correction as a
  -- labelled example. Present **only** under the class-(c) opt-in — which C1 does not build, so this
  -- is always null here — and **never** for a judgment whose origin was Gmail (`origin = gmail_api`),
  -- which Google's Limited Use forbids being used for anything but that user's own rules.
  request     jsonb,
  received_at timestamptz not null default now(),
  constraint corrections_once unique (account_id, ts, item_id, field)
);
comment on table public.corrections is
  'Spec §6(b) and §5.4: every row is a labelled example for the eval suite. `ours` and `theirs` carry a
   value only for closed-vocabulary and numeric fields; a course name or a title is never here.';

alter table public.telemetry_events enable row level security;
alter table public.corrections enable row level security;
create policy telemetry_events_select_own on public.telemetry_events
  for select to authenticated using (account_id = auth.uid());
create policy corrections_select_own on public.corrections
  for select to authenticated using (account_id = auth.uid());

-- The only shape a dashboard may read. `count(distinct account_id) >= 10` is the product plan's
-- minimum cohort (§7), and it is here rather than in a query so it cannot be dropped by accident.
create view public.telemetry_daily
with (security_invoker = false) as
select date_trunc('day', ts) as day,
       view,
       action,
       count(*)                    as events,
       count(distinct account_id)  as accounts
from public.telemetry_events
group by 1, 2, 3
having count(distinct account_id) >= 10;

create view public.correction_rates
with (security_invoker = false) as
select date_trunc('week', ts) as week,
       kind,
       field,
       count(*)                   as corrections,
       count(distinct account_id) as accounts
from public.corrections
group by 1, 2, 3
having count(distinct account_id) >= 10;

revoke all on public.telemetry_daily from anon, authenticated;
revoke all on public.correction_rates from anon, authenticated;
