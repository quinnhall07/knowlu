# The engine's commands

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Update it with the code.

- `rank --vault <v> [--today YYYY-MM-DD] [--runner manual|local|cloud] [--run-id <id>]`
- `coursework --vault <v> [--dry-run] [--via <via>] [--run-id <id>]` — zyBooks + VHL into `tasks/`.
  Always exits 0. An empty parse is a failure, never an empty semester. A vendor figure at 100%
  (zyBooks points, VHL `percentage_complete`) files a `status: done` amend proposal as
  `agent:knowlu.completion` (`engine/src/completion.rs`); `progress` is still never written after
  creation. Passwords come from Windows Credential Manager via the vault's `credential_target`;
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
- `judge --vault <v> [--via <via>] [--run-id <id>] [--runtime <llama-cli.exe>] [--model <.gguf>]
  [--log-dir <dir>] [--limit N]` — enriches tasks flagged `needs_enrichment: true` in three tiers
  (heuristics, promoted rules, the model — one process per judgment). **Always exits 0**: no runtime
  and no model are normal outcomes. Writes as `agent:knowlu.enrich` (`provenance::is_agent` is a
  `starts_with` test) with `judged: true`, `propose: true`. Judgment logs never enter the vault.
- `sync --vault <v> [--direction pull|push|both] [--via <via>] [--run-id <id>]` — the account's copy
  of the vault: new journal records and changed note text up, another desktop's writes down and
  applied through `write`. **Always exits 0**: no account, no session, no entitlement and no network
  are normal outcomes. The account is the source of truth and the folder is its mirror (cloud design,
  amendment 2026-09-17, ruling 2); the service can read what it stores, says so on the privacy page,
  and deletes it with the account.
- **The engine gates itself** (ruling 3): `coursework`, `ingest`, `judge` and `sync` do not run past the
  72-hour entitlement grace the app caches — `engine/src/entitle.rs` reads
  `%LOCALAPPDATA%\knowlu\profiles\<id>\entitlement.json` and the refusal is a named line at exit 0.
  `rank`, `surface` and `write` are never gated.
- `surface --vault <v> --view today|overdue|week|later|all|decisions|good-to-know|issues|runs
  [--today] [--now] [--seen-at] [--build-sha] [--window <flow-sequence>]` — the read model as JSON.
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
- **The judgment service (C2).** When `config/cloud.yaml` exists (written by the wizard at
  onboarding; absent is a named skip, never an error), `judge`'s tier 3 is `POST /judge-task` /
  `-event` / `-email` on our Supabase project — the prompt, schema and pinned model id live
  server-side, so `CloudModel` (`engine/src/cloudmodel.rs`) implements the same `judge::Model` the
  local runtime implemented. `ingest`, `coursework` and `rank` reach the same service too, but only
  for **transport** (the LMS feed, the zyBooks/VHL fetch, event feeds and `cloud:`-named calendars
  move server-side) — never for judgment, so `rank` never calls a model still holds.
  `engine/src/enrich.rs`'s `run_lines_with` hosts the five cloud passes a judge step runs in one slot:
  the tier-3 judge pass, the events pass, the Gmail pull, the rule-decision pull and the label
  report (F8: card answers and rejections, keyed by `judgment_id`, to `/telemetry`; never an
  `email`-kind decision).
  The Gmail pull declares `accepts: ["completion"]` and the events request `accepts: ["unsure"]`;
  a `completion` item (an LMS submission receipt) files the same `status: done` proposal through
  `completion::propose_done`, matched to exactly one active task by title.
- `runs`, `info`, `issues`, `write` (`--actor`, `--via` from `journal::VIAS`) — run records, info
  items, issue notes, journaled note edits. A human `--actor` (and `--opened-by`/`--closed-by`)
  defaults to the vault's token (`config/actor.yaml`; absent means `quinn`); `write` refuses a human
  actor that differs from it, and a bad file exits 2 writing nothing.

