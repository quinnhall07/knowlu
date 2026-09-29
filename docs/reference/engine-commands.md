# The engine's commands

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Update it with the code.

- `rank --vault <v> [--today YYYY-MM-DD] [--runner manual|local|cloud] [--run-id <id>]`
- `coursework --vault <v> [--dry-run] [--via <via>] [--run-id <id>]` — zyBooks + VHL into `tasks/`.
  Always exits 0. An empty parse is a failure, never an empty semester. Passwords come from Windows
  Credential Manager via the vault's `credential_target`; zyBooks 403s without a `User-Agent`; VHL is
  CAS with a one-time `lt` ticket and a dashboard on `m3a.vhlcentral.com`.
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
  [--today] [--now] [--seen-at] [--build-sha]` — the read model as JSON. Never writes.
- **The judgment service (C2).** When `config/cloud.yaml` exists (written by the wizard at
  onboarding; absent is a named skip, never an error), `judge`'s tier 3 is `POST /judge-task` /
  `-event` / `-email` on our Supabase project — the prompt, schema and pinned model id live
  server-side, so `CloudModel` (`engine/src/cloudmodel.rs`) implements the same `judge::Model` the
  local runtime implemented. `ingest`, `coursework` and `rank` reach the same service too, but only
  for **transport** (the LMS feed, the zyBooks/VHL fetch, event feeds and `cloud:`-named calendars
  move server-side) — never for judgment, so `rank` never calls a model still holds.
  `engine/src/enrich.rs`'s `run_lines_with` hosts the four cloud pulls a judge step runs in one slot:
  the tier-3 judge pass, the events pass, the Gmail pull and the rule-decision pull.
- `runs`, `info`, `issues`, `write` (`--actor`, `--via` from `journal::VIAS`) — run records, info
  items, issue notes, journaled note edits.

