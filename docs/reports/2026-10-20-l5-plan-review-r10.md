# L5 plan review, round 10, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r10). Reviewer, plan mode, with the code checked on the
email branch worktree (`engine/src/approvals.rs`, `eventemit.rs`, `write.rs`, `commitments.rs`) and the spec at
`docs/specs/2026-10-05-syllabus-class-times-design.md`. Filed by the controller from the reviewer's returned text.

**Verdict: PASS.** 0 critical, 0 important, 2 minor. r9's four findings are closed. The planner's questions: (a)
folding I-1 into place 10's eleventh change is acceptable; (b) nothing pins the horizon's far edge (M-1), and an
inclusive ruling fits the spec and the engine; (c) the M-1 alternative's build-cost claim is true.

## Confirmed

- **r9 I-1 is closed** (T4d "The horizon" :1384-1394, "`expires`" :1395-1398, T4dt (A) :1186-1189, Q-B :541-546,
  place 10 :738-741). `process_approvals` wakes a held card first (`approvals.rs:1596-1602`) and expires it only when
  `expires < today` (`:1616-1617`), so a day-1 card for an item due day 2 wakes on day 2 and stays `pending`. An item
  due day 1 plans no card. Day 1 now plans exactly what day 2's horizon holds, matching the 8's day-1 exception
  (:1361-1365) and the charge day in "The 15" (:1369-1370). Q-B's corrected reason is true.
- **r9 M-1 is closed** (Q-B :551-556, §7 item 2 :2287-2299). Expiry stamps `expired` and `delete`s the card
  (`approvals.rs:1623-1625`); `delete` moves it into `archive/` (`write.rs:576-589`), so the history is there to
  block the key.
- **r9 M-2 is closed** (:1172-1173): `PlannedCard` names `first_proposed_at` and `expires: Option<Date>`.
- **r9 M-3 is closed**: (1) bounded for late-entering items at :1681-1687, :1380-1382, :2279-2283 and place 10's
  spec text :731-735; (2) Q-B (:547-551) uses `defer_over_budget`'s real order; (3) the charge-day sizing is cited as
  new at :1370-1371, :2406, :2620.
- **(a)** Place 10's eleventh change already amends §4.6 (:727, spec :690-697), and the horizon sentence is in that
  paragraph (spec :690-691); §7 item 2 (:2284-2286) shows it to Quinn as a decision taken.
- **(c)** `existing_source_uids` (`approvals.rs:1341-1357`) returns uids with no status; the `superseded` exception
  already lives in the filer, read-only in `syllabus_cards.rs` (spec :779-782). Adding `expired` with the twin's day
  rule touches only `syllabus_cards.rs` (T4d'c, under 80 lines) and one T4dt (C) case. No contract file.

## Minor

- **M-1. Nothing pins the horizon's far edge** (T4dt (A) :1180, :1188-1189; spec T4dt row :1359). The day-1 case says
  "whichever way the horizon case above pins the edge", and that case pins nothing. If the test-writer picks
  exclusive, an assignment due exactly 14 days out files a day later than an `event-check` for the same day. An
  inclusive ruling is consistent: the spec says "within 14 days … of today" and "proposed two weeks ahead"
  (:696-697); eventemit's `start_window` is `day >= today && day <= last_day` (`eventemit.rs:430-434`) with
  `last_day = today + propose_horizon_days` (`:1020-1022`), and `horizon_window` is `horizon_start <= today`
  (`:438-439`, opened at `start - horizon`, `:60`). Fix: at :1180 state the edge (charge day + 14, exam + 21, is
  inside; + 15 / + 22 is not) with one (A) case per kind; at :1188-1189 replace "whichever way …" with "day 16 plans
  a held card, day 17 does not"; record the ruling in the r10 I-1 row (:2409).
- **M-2. The course listing for items further out has no task, and r10 moves its boundary** (spec :696-697). No task
  builds the "N more dates from your syllabus" line. Under r10, on day 1 items due days 16-17 (exams 23-24) are held
  cards, and a listing measured from today would also count them. Fix: name the task that builds the listing, or
  record it as deferred; wherever built, it uses the filer's charge-day horizon.
