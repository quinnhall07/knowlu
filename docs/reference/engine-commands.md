# The engine's commands

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Update it with the code.

- `rank --vault <v> [--today YYYY-MM-DD] [--now YYYY-MM-DDTHH:MM] [--runner manual|local|cloud]
  [--run-id <id>]` — no new flag for events. **The evening order** (spec
  `docs/specs/2026-10-23-evening-ranking-rule-draft.md`; `ranking::rank_at`): from 20:00 vault time
  (`ranking::EVENING_HOUR`), tasks due later the same day come first, earliest due first, ties
  keeping the pressure order; every other task keeps the pressure order below them, and `rank_override`
  still wins. Before the hour, or with no moment, the order is unchanged. **The day and the moment**
  (`cli::rank_clock`): the day is `--today`, else `--now`'s date, else the machine's date; the moment
  is `--now`, else none when `--today` is given (daytime order, as the golden harness runs), else the
  vault's wall clock. `--now` is strict `%Y-%m-%dT%H:%M` (no seconds, no bare date); a bad value is
  `BadNow`, exit 2, before the run starts, so nothing is written. `--now` pins the ranking moment
  only (a `--now` on a different day from `--today` ranks as daytime, so the evening step stays
  off), no timestamp the engine writes (the footer and the run record keep the wall clock). Beyond
  the day, exactly as `--today`, a `--now` with no `--today` dates the run's writes (the info pass,
  the approvals budget and pass, the proposal count) by `--now`'s day, so `rank --now` is a real run,
  never a dry run. Its events pass runs in this order (`cli.rs`; events spec §6.1, P16): D9's
  rebuild of a rejected `event-accept` card's declines and the series carry
  (`eventcarry::run`), then series inheritance of a settled `event-check` answer, then the roster,
  then the cards (obligation `event-accept` cards, the digest only with `event_cards` off, `unsure`
  `event-check` cards, opportunity `event-accept` cards only with `event_cards` on), each sized to
  what the earlier steps left of the 15 a day. `event_cards` (`config/events.yaml`) switches the
  card kinds on. The carry covers executed `event-accept` series and executed `event-check` series
  with `instances:` (`answered_series`, `accepted_check_series`): each later date of an accepted
  series becomes a commitment (a lane-shaped date none), and writes **one ledger line per carried
  date** to `state/events-seen.md`, by `agent:knowlu.carry`, with `from:`, `start:` and `end:`
  (P15); a moved lane date gets one more such line (PQ5 (b2), PQ7 (c)). The carry runs before
  inheritance (P16), so inheritance skips every date the carry answered. A date the student removed
  (`eventcarry::remove_lane_date`, P17) holds a `declined` line, which the carry, never-ask-twice
  and `relevant_events` all respect. `rank` still never calls a model.
- `coursework --vault <v> [--dry-run] [--via <via>] [--run-id <id>]` — zyBooks + VHL into `tasks/`.
  Always exits 0. An empty parse is a failure, never an empty semester. A vendor figure at 100%
  (zyBooks points, VHL `percentage_complete`) is **applied, not proposed** (lane O1):
  `completion::apply_vendor_completions` writes `status: done` as `agent:knowlu.completion` through
  `write` (journal first, `judged: false`, the figure on the record) and logs
  `marked done: <stem> (<source> <n>%)` (`would mark done: …` under `--dry-run`); the surface lists it
  under "Closed this week" with an Undo (the student's own `set` back to `active`). **Card rule**
  (`completion::apply_done`, all four of `propose_done`'s rules): only a task note directly under
  `tasks/` with `status: active`; never one whose `status` the student set by hand; never one with a
  `status → done` amend card in `approvals/` or `archive/`. A pending or snoozed "Mark done" card is
  left and the task skipped; **a card the student rejected, whatever filed it (a mail completion claim
  included), blocks every later gradebook or vendor answer for that task, by design: the student's
  answer stands.** An item at 100% in a source group **new to the vault** is never created as a task:
  it goes straight to `archive/` with `archived_reason: imported-done` (below `imported-past` in
  precedence: an item already past stays `imported-past`; `needs_enrichment: false`, so it is never
  judged) and logs `archived (imported-done) <stem>`; the summary line
  `coursework: N item(s) already turned in were archived from a source seen for the first time` follows
  when N > 0. The run record counts `imported-done` under the existing `archived` key (no new key).
  `progress` is still never written after creation. Passwords come from Windows Credential Manager via the vault's `credential_target`;
  zyBooks 403s without a `User-Agent`; VHL is CAS with a one-time `lt` ticket and a dashboard on
  `m3a.vhlcentral.com`.
- `grades --vault <v> --input <bundle> [--via <via>] [--run-id <id>]` — a Blackboard capture
  bundle (M1 spec §5, version 1; the app writes it into its own data, never the vault) into
  `grades/`: one note per gradebook column at `grades/<course-slug>-<column-id-slug>.md`, id
  `ids::derived_id("grade", path)`, matched to a course note by its `code:` (`membership.courseId`,
  else `course.courseId`, else `course.id`; nothing is created in `courses/`). New columns are
  `write::create`d and existing notes get only the fields that differ, so an unchanged gradebook
  writes no journal record; a column gone from a matched course becomes `status: removed`. Writes as
  `agent:knowlu.grades` and drops, before every write, each field `Journal::human_set` shows the
  student set. Gated like `coursework`. Holds `state/sync.lock` (`sync::RunLock`, the lock every
  sync takes) from its first read of the vault to its exit, because `grades/` is a synced folder;
  finding it held, it writes nothing and prints `grades (skipped: the vault is busy with a sync or
  another grades run)` at exit 0. A bundle whose `fetched_at` is strictly older than the one
  `state/grades.json` records (compared as instants; a recorded stamp that does not parse, or is
  later than now, orders nothing) writes nothing and prints `grades (skipped: a newer capture is
  already applied)` at exit 0, so the slot's early capture never undoes a Refresh made while it
  waited. Otherwise rewrites `state/grades.json` (`fetched_at`, `host`, the
  course counts `matched`/`skipped`/`failed`; device-local, never synced). Output is counts, course
  codes, note paths and error codes only — never a column name, category or score (spec §11):
  `grades: 2 courses, 3 changed items`, `grades: <code> not matched`, `grades: <code> failed (403)`,
  `grades: <path> not written (<kind>)`; the same lines, joined with `; `, go to
  `state/runner-log.md`. Exits 0 for every per-course outcome; 1 only for a missing or unparseable
  bundle (or a schema other than 1), whose line names serde's category, line and column, never the
  offending value.
- `coursework-discover [--vault <v>] [--zybooks-target <t>] [--vhl-target <t>]` — read-only: the
  zyBooks books and VHL sections the stored logins can see, as JSON (`errors`, `vhl`, `zybooks`, each
  row marked `mapped` against the vault's `course_map`). Always exits 0; the wizard's mapping rows
  come from it, and it writes nothing.
- `ingest --vault <v> [--via <via>] [--run-id <id>]` — the LMS `.ics` feed into `tasks/`. A vault
  with no account and an empty `ics_url` exits 1, which is why the app leaves the step out rather
  than run it — but **a cloud vault runs `ingest` regardless of `ics_url`**, because the feed lives
  in the account, not the vault (C2 final review A-1/A-2): `/ingest-ics` answering 404 (no
  `lms_ics` source configured) is a named skip at exit 0, never a failure, and any other service
  failure with no local `ics_url` names the real cause honestly and keeps exit 1.
  **Turned-in work (lane O1)**: `ingest` joins a feed task to its gradebook item exactly, by the
  `GradableItem-<id>` in its `source_uid` against a `kind: item` grade note's `source_uid` (an Overall
  column never joins; only `grades/` is read, which is why `grades` runs first in the slot). **Done rule
  (Quinn, Q-A, 2026-10-23):** the grade note's status is `needs-grading`, `exempt`, or `graded` with a
  score above 0. `in-progress`, `not-submitted`, `graded` with a score of 0 and `graded` with no score
  stay live, and the late-work rule decides. On the **first** ingest such an item (not already past) is
  archived as `imported-done` (`archived (imported-done) <stem>`; the first-run line is kept and a second
  one, `ingest: first run — N item(s) already turned in were archived`, follows when N > 0). After
  first sight an active task so joined is marked done through `completion::apply_done` and logs
  `marked done: <stem> (gradebook)`, in feed order; the card rule above applies. With no `grades/` notes
  (grades not connected) nothing joins and nothing changes.
- `judge --vault <v> [--via <via>] [--run-id <id>]
  [--log-dir <dir>] [--limit N]` — enriches tasks flagged `needs_enrichment: true` in three tiers
  (heuristics, promoted rules, the judgment service). **Always exits 0**: no account, session,
  entitlement or network are normal outcomes. Writes as `agent:knowlu.enrich` (`provenance::is_agent` is a
  `starts_with` test) with `judged: true`, `propose: true`. Judgment logs never enter the vault.
- `sync --vault <v> [--direction pull|push|both] [--via <via>] [--run-id <id>]` — the account's copy
  of the vault: new journal records and changed note text up, another desktop's writes down and
  applied through `write`. **Always exits 0**: no account, no session, no entitlement and no network
  are normal outcomes. The account is the source of truth and the folder is its mirror (cloud design,
  amendment 2026-09-17, ruling 2); the service can read what it stores, says so on the privacy page,
  and deletes it with the account.
- `mail-applied --vault <v> [--since <timestamp>]` — read-only: what forwarded mail wrote, as one JSON
  object through `ledger::dumps_value`. Never writes and is never gated (like `surface`); always exits 0.
  The journal is the only source: records by `agent:knowlu.mail` (`mail::MAIL_ACTOR`) at or after
  `--since` (else from the first mail record), joined to each note's current value. Shape:
  `{since, courses: [{course, created, changed, archived: {count, items}, notices}]}`, one group per
  course, and a group with `course: null` (the notices) last. A `created` or `archived` row carries
  `path`, `id`, `uid`, `title`, `due`, `ts`, `current` and `changed_since`; a `changed` row carries
  `path`, `id`, `title`, `field`, `old`, `new`, `current`, `sender`, `date`, `ts` and `changed_since`
  (sender and date are read back from the `<date> · <sender>: <why> — "<evidence>"` body line);
  a notice row carries `path`, `id`, `kind`, `title`, `opened_at`, `ts` and `changed_since`.
  `changed_since` is true when the note is gone, a non-agent has written to it since mail's create, or
  it no longer holds what mail left; the app then offers no Undo. `mail_asof` is the pass's own
  bookkeeping and never a row. The app's `mail_applied` command runs it.
- **The engine gates itself** (ruling 3): `coursework`, `ingest`, `judge` and `sync` do not run past the
  72-hour entitlement grace the app caches — `engine/src/entitle.rs` reads
  `%LOCALAPPDATA%\knowlu\profiles\<id>\entitlement.json` and the refusal is a named line at exit 0.
  `rank`, `surface` and `write` are never gated.
- `surface --vault <v> --view today|overdue|week|later|all|decisions|good-to-know|issues|runs
  [--today] [--now] [--seen-at] [--build-sha] [--window <flow-sequence>]` — the read model as JSON.
  Its `--now` is also the ranking moment: the page ranks in the evening order from 20:00 on that
  moment's day (`surface::load_at`); the note drawer (`note_detail`) keeps `load`.
  Never writes. `--window` (today view only) previews a planning-day `window:` edit — invalid input
  exits 2 — and reports `moved` against the current window without writing one.
- `commitments --vault <v> [--today YYYY-MM-DD] [--json] [--confirm <file> [--actor quinn] [--via dashboard]]`
  — without `--confirm`, it fetches the configured calendar feeds, refreshes
  `state/calendar-series.json`, and prints the current commitment proposals as of today
  (`coursework-discover`'s style: read-only except for the generated series-file refresh, always
  exits 0; no note, no card, no journal record); `--json` adds `uncovered_courses` and orders the
  proposals as the cards are. With `--confirm` (phase 2), it fetches nothing and writes the confirm
  screen's answers from a JSON file (`mine`, `not_mine`, `window`) as the student (default
  `--actor` the vault's token, `--via dashboard`), journal first. It prints `{created, declined, warnings, window}`
  and exits 2 on unreadable input or an invalid window, having written nothing.
  `--json` carries a top-level `registrar_changes` key beside `proposals`, `uncovered_courses` and
  `warnings` (the same object the overview returns): `{applied, ended, kept, status, unread}`, the
  lists of every registrar term in play (terms in key order, each list in the block's order), read
  from `state/calendar-series.json`'s registrar block; `kept` entries carry their `card`, read from
  `approvals/` and `archive/`. With no registrar term every list is empty.
- `commitments --vault <v> --registrar <file> --school <key> [--today YYYY-MM-DD] [--actor …] [--via dashboard]`
  — the UA registrar (phase 3). Conflicts with `--confirm`; `--school` is required (a key in
  `registrar::SCHOOLS`, today `ua`). **No network:** the app fetches Banner through the student's
  sign-in window and hands the answer over in a temp file. It parses first, merges the term's
  calendar into the series file (`refresh_series`), and for every row read whole that names a vault
  course confirms a hard, weekly commitment note **as the student** (the vault's human token, or
  `--actor`; `--via dashboard`), journal first. Later fetches apply a changed time, room or place
  and end a dropped course **as `agent:knowlu.registrar`**, journal first (the note gets an `until`,
  never a delete); a field the student set by hand is kept, and the engine files a `commitment-check`
  card (`agent:commitments`, local by kind, under the day's cap) instead. Rows with no meeting times,
  across midnight, `EXAM` meetings (skipped and counted until a later lane decides), other meeting
  types and not-registered statuses are dropped and counted. It holds the sync run lock for its
  writes. Prints `{term, rows, confirmed, proposed, dropped: {exam, meeting_type, midnight, no_time,
  status}, warnings, applied, ended, kept, status, unread}` and **exits 0, or 2 on an unknown school,
  a file that is unreadable or not JSON, a bad `config/actor.yaml`, or an answer with no usable row,
  having written nothing**. It is never gated and calls no model.
- **The judgment service (C2).** When `config/cloud.yaml` exists (written by the wizard at
  onboarding; absent is a named skip, never an error), `judge`'s tier 3 is `POST /judge-task` /
  `-event` / `-email` on our Supabase project — the prompt, schema and pinned model id live
  server-side, so `CloudModel` (`engine/src/cloudmodel.rs`) is the one implementation of
  `judge::Model`. `ingest`, `coursework` and `rank` reach the same service too, but only
  for **transport** (the LMS feed, the zyBooks/VHL fetch, event feeds and `cloud:`-named calendars
  move server-side) — never for judgment, so `rank` never calls a model still holds.
  `engine/src/enrich.rs`'s `run_lines_with` hosts the five cloud passes a judge step runs in one slot:
  the tier-3 judge pass, the events pass, the Gmail pull, the rule-decision pull and the label
  report (F8: card answers and rejections, keyed by `judgment_id`, to `/telemetry`; never an
  `email`-kind decision).
  The Gmail pull declares `accepts: ["completion"]` and the events request `accepts: ["unsure"]`;
  a `completion` item (an LMS submission receipt) files a `status: done` proposal (a card, not an
  apply: mail only suggests) through `completion::propose_done`, matched to exactly one active task by title.
  **Gmail connect (D4, D7).** The cloud arm always probes, and the Gmail pull runs on every cloud slot
  past the probe, whether or not the vault carries a calendar marker; the service decides (no grant is
  the silent `no_gmail_scope`, a revoked one prints `gmail: skipped (gmail is not connected;
  re-connect from settings)`). Every Gmail item is a proposal: `task` items file a card like
  `borderline`, `event` and `opportunity` ones, and only approving the card makes the note, so the
  summary reads `gmail: 0 task(s), N proposed, …`. **The transport stop** (spec Q1 (a′), PQ1 answer (i), on branch
  `email-forwarding`): a probe that failed in transport ends the cloud arm's network passes
  (events, Gmail, mail, rules, labels) with one leading `judge: skipped (no network (…))` line at
  exit 0; tier 1's answers are still written first (`enrich_with` with the service marked missing).
  **Out of credit (L7).** A judge reply carrying `unavailable: "out of credit"` is Knowlu's own
  provider account, never the student's subscription. `CloudModel::verdict_of` reads it before
  `outcome` and `cause` and sets `fatal()` to `judge::OUT_OF_CREDIT_LABEL`; unlike the daily cap it is
  global, so `CloudModel::call` makes no further `/judge-*` request in the run (task, event or email).
  `judge_task` maps it to `Outcome::ServiceUnavailable` (tier 1 is still written, the flag stays set),
  and `judge_event` passes it through as an error, never `unsure`, so the uid is asked again next slot.
  `run_lines_with` prints one line, `judge: the service answered out of credit on Knowlu's side, so the
  rest of the batch was not sent`, and `judge_roster` ends the events pass with one summary line ending
  `(out of credit on Knowlu's side)`. `keep_out_of_credit_heads_up` then opens one `heads-up` keyed
  `judge-service:out-of-credit`, "Knowlu's AI checks are paused", as `agent:knowlu.enrich`; a second
  run opens no twin (`source_issues::raise`). It closes the item (`source_issues::clear`) only when a
  tier-3 reply in the run showed the model answering (`CloudModel::heard_model`: a verdict, or a
  cause in `REPEATABLE_CAUSES`); a run that made no tier-3 call, or only tier 0 and 2 replies, leaves
  it alone. A failed write adds `judge: a heads-up could not be written` and changes neither the
  exit code (0) nor the pass.
  **A quiet Gmail (L7).** `/gmail-read` answers `went_quiet` (true when the connected Gmail's own
  7-day list held no message and the grant is at least 7 days old; see `cloud/supabase/README.md`);
  `GmailPull.went_quiet` is `None` for an absent or non-boolean key (an older server). `pull_gmail`
  reads it on round 0, before the empty-pull early return: `Some(true)` prints `gmail: no mail found
  in the connected Gmail for 7 days` and opens one `heads-up` keyed `gmail-source:quiet`, "Gmail has
  gone quiet", as `agent:knowlu.gmail`; `Some(false)` closes it; `None` does nothing. Every
  `CloudError::Quiet(_)` (not connected, revoked, not configured) closes it too, keeping its own
  words (`NoGmailScope` still prints nothing); transport, status and body errors leave it alone. A
  failed write adds `gmail: a heads-up could not be written`. Both keys are source issues
  (`source_issues::is_source_issue`): see `docs/surface/anatomy.md` §4.1.
- **Forwarded mail (email spec 2026-09-30; `engine/src/mail/`).** `judge`'s cloud arm runs the mail
  pass (`mail::pull_mail`) after the Gmail pull and before the rules pass, inside the same entitlement
  gate, so a lapsed account pulls nothing; it is not a new command and `rank` never reaches it. Per
  slot it takes up to `enrich::PULL_ROUNDS` pulls from `POST /mail-pull` (a closed backfill batch drains
  past them, inside the budget), plans each page with the pure `mailreconcile::plan` (the one place
  card or direct is decided), asks `/mail-resolve` once per course for ambiguous claims, and writes as
  `agent:knowlu.mail` through `write`, journal first. A message is acknowledged on the next pull, and
  recorded in `state/ingest-seen.md` under the fixed title `(email)`, only when every claim of it was
  written or deliberately skipped; a held one comes back. A claim `/mail-resolve` never answers (no
  answer, or the call fails) is held for at most 3 slots, counted once per pass that asked, in
  `state/mail-resolve-held.json`; at the 3rd its message is acked and D12's "Knowlu could not match this
  to an item" notice is opened, keyed by the claim's own uid (PQ14); the entry stays at 3 until the message is acked, so a message that returns first is given up on again without being asked. No mail text, address or session enters a
  log, the journal or telemetry. **Run line**, printed when a mailbox is connected or a message arrived
  and never otherwise: `mail: N new, M changed, K proposed, J notices, P past, S superseded, U
  unmatched`, with `, X not written` and `, Y unreadable` on the end when non-zero. Other lines are
  named failures: `mail: skipped (<why>)`, `mail: N written but not acknowledged (<why>); they will
  come back next slot`, `mail: could not ask which item (<why>); left for the next slot`, `mail: gave
  up asking which item for N claim(s) after 3 slots; each is an unmatched notice`, `mail: more
  is waiting, left for the next slot`, `mail: mailbox <id> is <state>` (a heads-up item for a mailbox
  that stopped delivering). A mail card on the vault's first day is filed `snoozed` to day 2 and
  `pending` from then on (`mailreconcile::proposal` over `commitments::vault_day`); a due that came
  from mail is recorded in the note's `mail_asof`.
- **`ingest`'s ruling-3 card** (ruling 3 and email spec D11 (5); `ingest::propose_feed_value`). When the
  feed changes a `title` or `due` the student edited by hand (`Journal::human_edited`), the note keeps
  its value and `ingest` files one `kind: amend` card from the note's value to the feed's, created by
  the feed's own name (`approvals::TITLE_CARD_CREATOR`), with no `source_uid` and no judgment field,
  pending from today and charged to the 15-a-day budget. It files nothing when the note's side is null
  or a collection, a value would not fit on one line, a pending or snoozed amend card for that note and
  field already exists, or the student rejected a card proposing this same value. Every other field the
  feed changed is written as before.
- `runs`, `info`, `issues`, `write` (`--actor`, `--via` from `journal::VIAS`) — run records, info
  items, issue notes, journaled note edits. A human `--actor` (and `--opened-by`/`--closed-by`)
  defaults to the vault's token (`config/actor.yaml`; absent means `quinn`); `write` refuses a human
  actor that differs from it, and a bad file exits 2 writing nothing.

