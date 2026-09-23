# The commitment model — phase 1 plan (engine and cloud, the Google Calendar series path)

**Status: PLAN, written 2026-09-23. Not executed.** **Base:** `main` **after `j-followups`
merges** (spec R16). Phase 1 edits `cli.rs`, `approvals.rs`, `cloudmodel.rs`, `eventemit.rs` and
`engine/tests/cloud_contract.rs`, which `j-followups` also edits, and it calls F2's
`eventemit::clock` and copies F3's `event-check` settlement arm, neither of which exists on `main`
today. Cut branch `commitments-p1` from the merge commit. Tasks P1–P5 may start earlier, on that branch
cut from `main`, and P6–P10, P15 and P17 too at the cost of a one-line `lib.rs` rebase (see
"Before the merge"); the branch then rebases onto the merge.
**Written to survive a context compaction:** every task names its files, decisions and tests, so a
fresh session can execute from this document alone.

**Phase 1 is:** the three note shapes and their loader; the series transport (`/ingest-calendar`
learns `series`); `state/calendar-series.json` and the ICS series logic; the classifier and the
proposals; local-only `commitment-check` cards (proposals, the window, changes) and their
settlement; capacity and the planning day; `state/plan.json` and a live `moved`; the read model's
blocks and the `--window` preview; `conflicts`/`fit`; the `commitments` command.
**It is not:** the onboarding confirm screen, the *Your week* panel and editor, the fallback card
(`commitment-ask`) and its form (all phase 2), the UA registrar (phase 3), or the sync work (phase 1s,
listed under "Dependencies" because it gates the release).

## Authority

- **The spec:** `docs/specs/2026-09-23-commitment-model-design.md` on `main` at `80350a9`
  ("re-review fixes"). Section numbers below (§2.1, R12, …) are its. Where this plan and the spec
  disagree, the spec wins and this plan is wrong.
- **Quinn's rulings of 2026-09-23**, in the spec's §1.1: Q1 (the tagline: every feature improves the
  morning answer), Q2 (login-only: infer, ask to confirm, typing last), Q3 (commitments in the
  vault; hard/soft/optional; `calendar.readonly` only), Q4 (the class source: Google series first),
  Q5 (the observed calendar patterns), **Q6** (the planning day: the routine window replaces
  08:00–18:00, editable at any time, and the day view says what moved) and Q7 (the UA registrar,
  phase 3, not here).
- **Above the spec:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (signed; its §1 decisions and
  the 2026-09-17 amendment), then `docs/notes/2026-09-23-know-whats-next-direction.md` (piece 1),
  then `CLAUDE.md`'s two overriding rules and its engine invariants.
- **Patterns copied:** `docs/plans/2026-09-23-engine-follow-ups-plan.md` on `j-followups` — F2's
  `event-check` emitter (budget sizing, `first_proposed_at`, the title from structured fields) and
  F3's settlement arm in `approvals::transition_note`.
- **Reviews folded into the spec:** `.superpowers/sdd/2026-09-23-commitment-model/spec-review.md`
  and `spec-rereview.md` (N1 the sync exclusion by construction; N2 `moved` in `surface`).

## Global Constraints (binding on every task)

**From `CLAUDE.md`, verbatim.** The task text never overrides them.

1. "**Add no single-user assumptions.** Anything that would need hand-editing for a second user is
   a bug. Nothing here names a person's vault, machine, account or credential."
2. "**Never regenerate a frozen reference.** Eight Python-written references live in
   `engine/tests/fixtures/`: `golden-today-s1.md`, `golden-today-full.md`,
   `calendar-snapshot-gcal.md`, `vault-full/state/events.md`, `zybooks-parsed-reference.json`,
   `vhl-parsed-reference.json`, `run-records-reference.json`, `pyyaml-safe-dump-reference.json`.
   If the engine disagrees with one, the engine is wrong. The three Rust-generated read-model
   references — `surface-today-{s1,s1-migrated,full}.json` (`engine/tests/surface_oracle.rs`) —
   have their own rule (console spec §4.6): regenerate only in a commit whose diff shows the change
   and whose message says why." **No task in this plan changes any of the eleven.**
3. "A vault is markdown + YAML frontmatter (`tasks/`, `approvals/`, `archive/`, `courses/`, `info/`,
   `issues/`, `config/`), the single source of truth. `state/` is generated; `today.md` is rewritten
   every run. The engine is **deterministic**: same input, same order."
4. "**`rank` never calls a model** (Knowlu spec decision 11)."
5. "Never rewrite a vault file wholesale. Every note write goes through `write`: journal record first
   (`state/journal/YYYY-MM-DD.jsonl`, UTC days), single-line frontmatter surgery second. **No note is
   ever parsed and re-dumped**; `src/yamlemit.rs` is the one YAML emitter and deliberately has no
   anchor or alias support. Every note has an opaque `id:`; `source_uid` is the external key."
6. "Judge once, re-propose freely: an agent never re-sets a field the journal shows the user set."
7. "Approvals are capped at 15 new proposals a day; overflow is snoozed, never deleted. `proposed_at`
   is the day a proposal charges; `first_proposed_at` is set once and drives every age."
8. "All JSON the crate writes goes through `ledger::dumps_value` (Python `json.dumps` separators), so
   a new line and an old line carrying the same data are the same bytes."
9. "`journal::VIAS`, run records, ledgers and note frontmatter are contracts with existing vaults:
   byte-identical, never renamed."
10. "`cargo build --workspace` and `cargo test --workspace` from the root. **0 warnings is part of
    green.** The one accepted line is the app's pre-existing `.rsrc merge failure: multiple
    non-default manifests` linker message. Four tests are `#[ignore]` by design … None may be
    un-ignored by changing the assertion. TDD: the test first, then the code." "**`cargo test
    --release` will not link** — test in the dev profile."
11. "**Line endings: LF everywhere in this repo** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1`
    are CRLF). `engine/tests/fixtures/**` is `-text` … **never re-encode them**."
12. "Develop and demo against scratch vaults (`scripts\scratch-vault.ps1 -Source <vault>`), never a
    live one."

**From the spec's §8, "What cannot change", verbatim.**

13. "**The eight frozen Python references** (CLAUDE.md rule 2). `golden-today-s1.md` and
    `golden-today-full.md`: the fixture vaults have no `commitments/`, no `courses/` and no reachable
    feed, so the calendar, window, capacity and page are computed exactly as today, and `today.md`
    gains no line (§6.4). `calendar-snapshot-gcal.md`: `write_snapshot`, `read_snapshot` and
    `parse_calendar_ics` are not edited; series use new code and a different file.
    `vault-full/state/events.md`, the parsed references, `run-records-reference.json` (no new step,
    R13) and `pyyaml-safe-dump-reference.json` (`meets`, `window`, `commitment`, `change` and `was`
    go through the existing flow emitter) are untouched by construction."
14. "**The three read-model references** `surface-today-{s1,s1-migrated,full}.json`: no commitment
    blocks, no planning day and no `state/plan.json` exist for them; `template_only_blocks` is
    today's computation; `moved` is omitted. They are not regenerated."
15. "**Byte contracts.** No new `journal::VIAS` entry (`agent:commitments` is an actor, and
    `provenance::is_agent` is a `starts_with` test). Every JSON line goes through
    `ledger::dumps_value`. `/ingest-calendar` without `accepts=series` replies byte-for-byte as
    today, and `toIcs` is pinned."
16. "**A vault with no `commitments/`.** By construction `commitment_spans` is empty and `window`
    falls back to the template, so `template_blocks` builds the identical busy list; the
    de-duplication in §6.2 fires only beside a commitment block; with no configured, reachable feed
    no series file is written; with no planning-day note no plan file is written."
17. "**`ids::NOTE_FOLDERS` grows to seven.** Id repair and backups iterate it and already skip a
    missing folder; `note_folders_is_a_subset_of_backup_folders` keeps checking the derivation, and
    `BACKUP_FOLDERS` grows to ten."

**Working rules for this plan.**

18. **No model, no network in tests.** The classifier, proposals, cards and settlement are
    deterministic. Engine tests inject series through `Fetchers.series`; Deno tests use injected
    deps. Loopback-only (`127.0.0.1:0`) HTTP is fine. Nothing touches Supabase staging or production;
    deploys are the controller's.
19. **No personal data.** Every title, room and calendar in a test is invented. Quinn's observed
    calendar (Q5) is a pattern, never a fixture.
20. **Proposals never become notes, and nothing about a card leaves the device** (R18, R20). No task
    writes a `proposed` note, and every card goes through `commitments::file_card`.
21. **No bare `git stash`.** A WIP commit, or `git stash push -u -m <tag>` and apply by SHA.
22. **Stay out of** `.claude/worktrees/*`, `.github/**`, `app/src/**`, `app/static/**` and
    `site/**`. The one `app/**` edit is `app/tests/scaffold.rs`'s `Fetchers` literal (P14).

## Ordering and file ownership

| task | unit | files | depends on | before the merge? |
|---|---|---|---|---|
| P1 | the `cmt` id kind and the `commitments/` folder | `engine/src/ids.rs`, `engine/src/backup.rs` | — | **yes** |
| P2 | the flow emitter carries `meets`, `window`, `commitment`, `change` | `engine/src/yamlemit.rs` (tests; code only if a test fails) | — | **yes** |
| P3 | `WeekCalendar` learns spans, instances and the window | `engine/src/weekcal.rs` | — | **yes** |
| P4 | `calfeed::weekly_series`, the ICS series reader | `engine/src/calfeed.rs` | — | **yes** |
| P5 | `/ingest-calendar` learns `series` | `cloud/supabase/functions/ingest-calendar/{handler.ts,handler_test.ts,index.ts}` | — | **yes** |
| P6 | `commitments.rs`: the notes, `load`, `LOCAL_CARD_KINDS`, the sync tripwire | `engine/src/commitments.rs` (new), `engine/src/lib.rs` | P1, P3 | with a one-line `lib.rs` rebase |
| P7 | the classifier and the vault's code table | `engine/src/commitments.rs` | P6 | with P6 |
| P8 | `Series`, normalisation and `state/calendar-series.json` | `engine/src/commitments.rs` | P4, P7 | with P6 |
| P9 | proposals and the window proposal | `engine/src/commitments.rs` | P8 | with P6 |
| P10 | `conflicts` and `fit`, the piece-2 API | `engine/src/commitments.rs` | P6, P8 | with P6 |
| P15 | `WeekCalendar::for_vault` and the no-commitments equivalence | `engine/src/weekcal.rs`, `engine/src/commitments.rs` | P3, P6, P8 | with P6 |
| P17 | the read model's commitment blocks and window | `engine/src/surface.rs` | P15 | with P6 |
| P11 | the `commitment-check` card: `file_card`, `emit_checks`, withdrawal | `engine/src/commitments.rs`, `engine/src/eventemit.rs` | P9, **j-followups** | no |
| P12 | change detection: changed, ended, succeeded | `engine/src/commitments.rs` | P11 | no |
| P13 | the settlement arm | `engine/src/approvals.rs`, `engine/src/commitments.rs`, `engine/Cargo.toml`, `Cargo.lock` | P11, P12, **j-followups** | no |
| P14 | the engine transport: `accepts=series`, `Fetchers.series`, the stash | `engine/src/cloudmodel.rs`, `engine/src/cli.rs`, `engine/tests/cloud_contract.rs`, `app/tests/scaffold.rs` | P5, **j-followups** | no |
| P16 | `rank` wiring: refresh, withdraw, change, emit, count, capacity, baseline | `engine/src/cli.rs`, `engine/src/commitments.rs` | P11–P15 | no |
| P18 | `moved`, live in `surface`, and the `--window` preview | `engine/src/surface.rs`, `engine/src/main.rs`, `engine/src/commitments.rs` | P16, P17 | no (after P16) |
| P19 | the `commitments` command and the model-reach test | `engine/src/main.rs`, `engine/src/cli.rs`, `engine/tests/cloud_contract.rs` | P14, P16, P18 | no |
| P20 | docs: the read model and `CLAUDE.md` | `docs/surface/anatomy.md`, `CLAUDE.md` | P19 | no |

**Shared files, each edited in sequence and never in parallel:**

- `commitments.rs`: P6 → P7 → P8 → P9 → P10 → P15 → P11 → P12 → P13 → P16 → P18.
- `weekcal.rs`: P3 → P15. `surface.rs`: P17 → P18. `main.rs`: P18 → P19.
- `cli.rs`: P14 → P16 → P19. `cloud_contract.rs`: P14 → P19.

**Parallel lanes.** Wave 1: P1, P2, P3, P4, P5 (disjoint files). Wave 2: P6. Then P7 → P8 → P9 → P10
in sequence (one file), with P15 → P17 after P8. After the merge: P11 and P14 in parallel (disjoint),
then P12, P13, P16, P18, P19, P20 in sequence.

### Before the merge

`git diff --name-only main...{c3-sync,c1b-sign-in,c1c-first-day,j-followups,j-judgment-quality}`
(2026-09-23) lists no `weekcal.rs`, `surface.rs`, `ids.rs`, `backup.rs`, `calfeed.rs`, `main.rs`,
`yamlemit.rs` or `ingest-calendar/` file, and `engine/src/commitments.rs` does not exist
anywhere. So:

- **P1–P5 touch only files no other branch touches.** They can be built now, on `commitments-p1`
  cut from `main`.
- **P6–P10, P15 and P17** add one line to `engine/src/lib.rs` (`pub mod commitments;`, in P6),
  which `j-followups` and `c3-sync` also edit. Everything else they touch is untouched elsewhere.
  They can be built before the merge if the executor accepts a one-line textual conflict in
  `lib.rs` at rebase (keep every branch's `pub mod` lines). Strictly by the "no other branch
  touches it" rule, they wait.
- **P11–P14, P16, P18–P20 wait for the merge.** They edit `approvals.rs`, `cli.rs`, `cloudmodel.rs`,
  `eventemit.rs`, `cloud_contract.rs`, `anatomy.md` or `CLAUDE.md` (all in `j-followups`' diff), or
  they call `eventemit::clock`, which only `j-followups` has. P13 also adds `ring` to
  `engine/Cargo.toml`, the line `c3-sync` adds too. P18 needs P16's baseline, so it follows the merge
  though `surface.rs` and `main.rs` are otherwise free.
- **Nothing is merged to `main` early.** A `main` that carries P1's `cmt` without phase 1s's
  migration live on prod breaks the R3 release gate. The pre-merge tasks live on the branch.

## Dependencies outside this plan

1. **`j-followups` merges first** (R16). It owns `approvals::transition_note`'s `event-check` arms
   (the pattern P13 copies), `eventemit::clock` (P11's titles) and the `approvals.pending += checks`
   count in `cli.rs` (P16 copies it).
2. **Phase 1s, the C3 sync dependency (spec §10), is not in this plan, but gates the release.** It
   lands with C3′ (inside `c3-sync` if phase 1 merges first, or after it):
   - a migration adding `commitments` to `sync_notes_path_check`, and `NOTE_PATH_RE` in
     `cloud/supabase/functions/_shared/sync_rows.ts` with its test;
   - `sync.rs`'s local-card predicate by **kind**, reading `commitments::LOCAL_CARD_KINDS`, and its
     gate `build_push_sends_no_local_card_nor_any_record_about_one`;
   - privacy line 2 (§9).

   **The gates that make it unmissable.** C3′'s `is_note_path_and_the_servers_regex_agree` fails
   once `NOTE_FOLDERS` holds `commitments` (P1) and the regex does not; P6's tripwire
   `sync_keeps_every_local_card_kind_local` fails once `sync.rs` exists in the crate without the
   predicate and its gate. Either merge order fails the second build until 1s is in.
   **Release gate (R3):** the phase-1s migration is live on prod before any release carrying `cmt`
   ships. `C3 migrations need db push --include-all` (the SDD lessons note) — the controller's.
3. **Privacy line 1 (§9) ships with phase 1's first release** — a release gate, worded by Quinn with
   the lawyer's read. `site/privacy.html` is outside this plan's files (c1b, c1c and c3-sync all
   edit it); the line goes in whichever branch owns the page at release time.
4. **The ingest-calendar deploy (P5) precedes the engine release** that sends `accepts=series`.
   Either order is safe (§4.3), but series only flow once both are out. Deploys are the
   controller's.

---

## P1 — the `cmt` id kind and the `commitments/` folder (`ids.rs`, `backup.rs`)

**Decisions.**

1. `KINDS` gains `"cmt"`; `ID_RE` becomes `^(task|appr|info|iss|course|cmt)_[0-9a-f]{10}$`. *Reason:*
   R3 — an id kind is a vault contract, so it is fixed once, here.
2. `NOTE_FOLDERS` gains `"commitments"` **at the end** (index 6). *Reason:* `BACKUP_FOLDERS` is
   derived by index, and appending keeps indices 0–5 and every existing iteration order.
3. `kind_for` maps `type: commitment` → `cmt`, and a note under `commitments/` with no `type:` →
   `cmt` (the `courses` folder rule's shape). *Reason:* §2.2's `id` and `type` rows.
4. `BACKUP_FOLDERS: [&str; 10]` adds `ids::NOTE_FOLDERS[6]` before `"state"`. *Reason:* spec §8
   constraint 17.

**Tests first** (in-module):
- `ids.rs`: `kind_for_maps_type_commitment_to_cmt` — `type: commitment` in any folder gives `cmt`;
  `a_note_in_commitments_without_type_is_cmt`; `is_id_accepts_cmt_and_still_refuses_cmx` —
  `cmt_3f9a1c2b7d` true, `cmx_3f9a1c2b7d` false; `id_repair_gives_a_commitment_note_a_cmt_id` — a
  scratch vault with `commitments/cs-100.md` and no `id:` gains exactly one `cmt_` id through the
  existing repair path, and nothing else in the vault changes.
- `backup.rs`: the existing `note_folders_is_a_subset_of_backup_folders` still passes;
  `backup_folders_has_ten_entries_with_commitments_before_state` — `BACKUP_FOLDERS[6] ==
  "commitments"`, `[7] == "state"`.

**Verify:** `cargo test -p knowlu-engine --lib -- ids:: backup::`, then `cargo test -p knowlu-engine
--test oracle --test surface_oracle` and `git diff --exit-code engine/tests/fixtures`.

## P2 — the flow emitter carries the new fields (`yamlemit.rs`, tests only)

**Decisions.**

1. **Prove before editing.** `safe_dump_flow` is already a general PyYAML flow emitter; the spec's
   §10 row says `yamlemit.rs` changes only if it cannot emit these shapes. This task writes the
   tests; code changes only if one fails, and then only to match PyYAML 6's bytes.
2. `pyyaml-safe-dump-reference.json` is never edited (constraint 2). If an expected string below
   disagrees with a line in that reference, the reference wins and this plan's string is wrong.

**Tests first** (`yamlemit.rs` `mod tests`):
- `flow_emits_a_meets_sequence_on_one_line` — `[{days: [mon, wed, fri], start: "12:00", end:
  "12:50"}]` emits exactly `[{days: [mon, wed, fri], end: '12:50', start: '12:00'}]` (keys sorted,
  sexagesimal-looking times single-quoted, per R2).
- `flow_emits_a_commitment_mapping_with_nulls_and_a_nested_sequence` — `{kind: class, level: hard,
  title: "CS 100", course: cs-100, meets: [...], where: null, from: 2026-08-19 (a string), until:
  null}` is one line, `null` literal, the date-shaped string quoted as PyYAML quotes it.
- `flow_emits_a_change_and_was_pair` — `{until: "2026-12-04"}` and `{until: null}`.
- `a_title_with_a_colon_or_quote_round_trips` — `CS 100: Lecture` and `Bob's club` emit on one line
  and parse back (via `serde_yaml_ng`) to the same value.

**Verify:** `cargo test -p knowlu-engine --lib -- yamlemit::`, then `cargo test -p knowlu-engine
--test oracle` (which reads `pyyaml-safe-dump-reference.json`).

## P3 — `WeekCalendar` learns spans, instances and the window (`weekcal.rs`)

**Decisions.**

1. **Plain data in, no knowledge of `commitments.rs`.** New `pub struct CommitmentSpan { day:
   DayKey, start: Time, end: Time, from: Option<Date>, until: Option<Date>, title: String, kind:
   String, source_uid: String }` (the spec's `Span`, renamed to stay clear of `jiff::Span`);
   `WeekCalendar` gains `commitment_spans: Vec<CommitmentSpan>`, `instances:
   BTreeMap<String, (Date, Date, Vec<(Date, Time, Time)>)>` and `window: [Option<(Time, Time)>; 7]`,
   all empty by default, set through `with_commitments(self, spans, window) -> Self` and
   `with_instances(self, map) -> Self`. *Reason:* keeps P3 buildable before `commitments.rs`
   exists, and keeps `weekcal` a pure calendar.
2. `pub fn window(&self, day: Date) -> (Time, Time)` — the planning day's entry for `day_key(day)`,
   else `(day_start, day_end)`. Every date-based use of `day_start`/`day_end` in this file goes
   through it. *Reason:* §6.1.
3. `template_blocks(day)`'s busy list = the template's spans **plus** each span active that day:
   inside its source's horizon in `instances` (`read ≤ day < read + 28`), that source's actual
   instances on that date (none if cancelled, the moved time if moved); otherwise the weekly span on
   its weekday when `from ≤ day ≤ until`. The sort, cursor walk and clamp are unchanged. *Reason:*
   §6.1, R21.
4. `pub fn template_only_blocks(&self, day)` — today's computation (template classes only) within
   `window(day)`. *Reason:* §6.2's gap walk must not paint a club as a class (R15).
5. `pub fn spans_on(&self, day) -> Vec<(DateTime, DateTime, &CommitmentSpan)>` — the active
   spans as drawn, the same rule as decision 3; P17 draws from it. *Reason:* one rule, two readers.
6. `pub fn with_day_window(self, day: Date, start: Time, end: Time) -> Self` — a copy whose window
   for `day`'s weekday is replaced. *Reason:* P18 plans today under the baseline window and the
   `--window` preview under a proposed one, from the same calendar otherwise.

**Tests first** (`weekcal.rs` `mod tests`, invented spans):
- `window_falls_back_to_template_day_start_and_end` — no window: `window(d) == (day_start, day_end)`.
- `a_window_entry_replaces_that_weekday_only` — Mon–Fri 08:00–22:00 moves a Wednesday's last block
  end to 22:00 and leaves Saturday at the template's.
- `a_commitment_span_is_busy_on_its_weekday_within_from_until` — Mon/Wed 12:00–12:50 splits
  Monday's block; absent before `from` and after `until`.
- `inside_the_horizon_actual_instances_replace_the_weekly_span` — a moved instance is busy at its new
  time; a cancelled date (no instance) is free; a day past the horizon uses the weekly span.
- `template_only_blocks_ignores_commitment_spans` — with a club span, `template_only_blocks` equals
  the no-span `template_blocks`.
- `with_day_window_changes_only_that_weekday`.
- `overlapping_spans_behave_like_overlapping_classes` — a span identical to a template class leaves
  `template_blocks` unchanged.

**Verify:** `cargo test -p knowlu-engine --lib -- weekcal::`, `cargo test -p knowlu-engine --test
oracle --test surface_oracle`, `git diff --exit-code engine/tests/fixtures`.

## P4 — `calfeed::weekly_series`, the ICS series reader (`calfeed.rs`)

**Decisions.**

1. `pub fn weekly_series(ics: &str, tz: &TimeZone, from: Date, to: Date) -> (Vec<IcsSeries>, Vec<String>)`
   — **new, pure; nothing existing is edited**. `IcsSeries { key, title, location, description,
   rule_lines, first, instances: Vec<(Date, Time, Time)>, all_day, ineligible: Option<String> }`,
   defined in `calfeed.rs`. *Reason:* §3.2 step 1 and constraint 13 (`parse_calendar_ics` feeds the
   frozen snapshot).
2. Masters and overrides grouped by `UID`; the master expanded with the existing private
   `occurrence_starts`; `EXDATE` removed; a `RECURRENCE-ID` override replaces its instance, or
   removes it with `STATUS:CANCELLED`. *Reason:* R6, review C4.
3. A master with `STATUS:CANCELLED` or `TRANSP:TRANSPARENT` yields nothing; the owner's
   `PARTSTAT=DECLINED` (owner = the single `ORGANIZER` address) yields nothing; with no single
   organizer, any `DECLINED` attendee marks `ineligible`. *Reason:* §3.2, "when in doubt, do not
   propose".
4. `occurrence_starts` returning `UnsupportedRule`, or an `RDATE`, marks `ineligible` with the reason
   instead of dropping the series. *Reason:* the file keeps every series (M-g); §3.4 decides.
5. `key` = `ics-series:<UID>`, or `gcal-series:<UID without @google.com>` when the UID ends in
   `@google.com`. *Reason:* R6 — one series seen by both routes is one key.
6. The description is returned only for P8's place-like reduction and is never stored by P8.

**Tests first** (`calfeed.rs` `mod tests`, invented ICS strings):
`an_override_moves_its_instance`; `a_cancelled_override_removes_its_instance`;
`exdate_removes_an_instance`; `a_cancelled_master_yields_no_series`;
`a_transparent_master_yields_no_series`; `the_owner_declined_yields_no_series`;
`an_unknown_owner_with_any_declined_attendee_is_ineligible`;
`an_unsupported_rule_is_ineligible_not_dropped` (`BYMONTHDAY`); `an_rdate_is_ineligible`;
`a_google_uid_is_keyed_gcal_series_without_the_suffix`; `rule_lines_are_kept_raw` (`UNTIL=…Z` and
`COUNT` unconverted); and `parse_calendar_ics_output_is_unchanged_for_gcal_ics` — the existing
`tests/fixtures/gcal.ics` parses to the same events as before (the snapshot oracle also pins it).

**Verify:** `cargo test -p knowlu-engine --lib -- calfeed::`, `cargo test -p knowlu-engine --test
oracle`, `git diff --exit-code engine/tests/fixtures`.

## P5 — `/ingest-calendar` learns `series` (`cloud/supabase/functions/ingest-calendar/`)

**Decisions.**

1. `accepts` is a comma-separated query parameter; unknown words are ignored; **without `series` the
   response body is byte-for-byte today's.** *Reason:* §4.1, constraint 15; old engines keep working.
2. `series` is only for `name=google`. `name=personal` never carries it (the device reads the ICS
   itself). *Reason:* §4.1.
3. `CalendarDeps` gains `calendarList(token)`, `seriesInstances(token, calendarId, from, to,
   pageToken)` and `seriesMasters(token, calendarId, from, to, pageToken)` (raw pages), implemented in
   `index.ts` with `fetch`; `handler.ts` holds every rule, so `handler_test.ts` tests every rule with
   injected deps. *Reason:* the existing split (deps in `index.ts`, logic in `handler.ts`).
4. Calendars: `primary`, then `accessRole: owner` and not `hidden`, ordered by id, at most 10; named
   only as `google:` + first 16 hex of `sha256(account_id + "\n" + calendarId)`. *Reason:* R5.
5. Items: `singleEvents=true`, `nextPageToken` followed up to 5 pages; keep items with a
   `recurringEventId`, a `dateTime` start, not `cancelled`, and no declined `self: true` attendee;
   `eventType` passed through (`default` when absent). Masters: `singleEvents=false`, same paging;
   `recurrence` filtered to `RRULE`/`EXDATE`/`RDATE`; `first` = the master's start. *Reason:* §4.1.
6. **Complete or not at all:** a calendar past 5 pages, or whose series would cross the 100-series
   cap (calendar order, then series id), is left out of `calendars_read` and its items are not sent;
   a series with more than 40 instances is dropped. *Reason:* review I6 — no flicker.
7. `title`, `location` cut to 200; `description` only when `location` is empty, cut to 200. Nothing is
   logged but exception class names. *Reason:* R4, §9.
8. **A 5-second budget** for series gathering after `ics` is built; on overrun or any error `series`
   is omitted and `ics` returned as today. *Reason:* a series problem never costs the day's busy time.

**Tests first** (`handler_test.ts`, Deno, injected deps, invented events):
- `without accepts=series the google reply is byte-for-byte today's` — the body equals the
  pre-change body for the same deps (assert the exact JSON string).
- `toIcs output is pinned byte-for-byte` — a fixed three-event input gives a fixed string.
- `accepts=series,unknown gives series; unknown words are ignored`.
- `name=personal never carries series, with or without accepts`.
- `calendars are primary then owned non-hidden by id, at most 10` — 12 owned + 1 shared + 1 hidden in;
  the 10 expected keys out, in order.
- `a calendar key is google: plus 16 hex of sha256(account_id newline id)` — computed in the test.
- `pages are followed; a sixth page leaves the calendar out of calendars_read and sends none of its
  items`.
- `the 100-series cap leaves out the whole calendar that would cross it`.
- `a series with 41 instances is dropped`.
- `cancelled instances, all-day items and a declined self attendee are not items`.
- `recurrence keeps RRULE, EXDATE and RDATE lines only`.
- `description is sent only when location is empty, cut to 200`.
- `past the 5-second budget series is omitted and ics is unchanged` (fake clock in deps).
- `a throwing seriesMasters omits series and keeps ics`.

**Verify:** `deno test cloud/supabase/functions/ingest-calendar/` and `deno check
cloud/supabase/functions/ingest-calendar/index.ts`. **Deploy is the controller's**, to staging first.

## P6 — `commitments.rs`: the notes, `load`, `LOCAL_CARD_KINDS`, the sync tripwire

Files: `engine/src/commitments.rs` (new), `engine/src/lib.rs` (`pub mod commitments;`).

**Decisions.**

1. Types: `Level { Hard, Soft, Optional }`; `Commitment { id, path, kind, level, title, course,
   meets: Vec<Meet>, where_, from, until, source_uid }`; `Meet { days: Vec<DayKey>, start, end }`;
   `Commitments { confirmed: Vec<Commitment>, declined: BTreeSet<String>, window: [Option<(Time,
   Time)>; 7], planning_day: Option<Commitment>, warnings: Vec<String> }`. *Reason:* §2, §7.
2. `pub fn load(vault) -> Commitments` never fails: missing folder → empty; notes read in file-name
   order; frontmatter read, never re-dumped. *Reason:* §6.5, constraint 5.
3. Field rules exactly as §2.2: `status: confirmed` only counts; `declined` → its `source_uid` into
   `declined` and nothing else read (§2.3); any other status ignored with a warning; kinds and default
   levels (C1, R17: class/lab/work/exam hard; club/meeting/event/task-block soft; office-hours
   optional); unknown kind → soft with a warning; invalid `meets` entry skipped, none valid → note
   ignored; times `HH:MM`, `start < end`.
4. Duplicates: two confirmed notes with one `source_uid` → lowest `id` kept, the rest named in one
   warning (§2.5). Several `planning-day` notes → lowest `id` wins, one warning; a duplicate weekday
   inside `window` is skipped with a warning (§2.4).
5. `pub const LOCAL_CARD_KINDS: [&str; 2] = ["commitment-ask", "commitment-check"];` *Reason:* R20 —
   the one constant C3′'s sync reads (phase 1s).
6. `pub fn spans(&self) -> Vec<weekcal::CommitmentSpan>` — hard and soft confirmed notes only, one
   span per `(meet, day)`. *Reason:* §6.1 (optional, markers and the planning day add no busy time).

**Tests first** (`commitments.rs` `mod tests`, scratch vaults under a temp dir):
- `load_reads_a_confirmed_commitment` — §2.1's example note gives every field.
- `a_missing_folder_loads_empty_with_no_warning`.
- `a_hand_written_proposed_note_is_ignored_with_a_warning`.
- `reserved_kinds_load_without_a_warning_at_their_default_levels` — `event` soft, `exam` hard,
  `task-block` soft.
- `an_unknown_kind_loads_soft_with_a_warning`; `an_unknown_level_loads_soft_with_a_warning`.
- `an_invalid_meets_entry_is_skipped_and_a_note_with_none_is_ignored` — `start >= end`, `25:00`,
  a bad day key.
- `duplicate_source_uid_keeps_the_lowest_id_and_warns_once`.
- `a_decline_marker_contributes_its_key_only` — a marker with a stray `title:` still adds only the key.
- `the_planning_day_lowest_id_wins_and_a_duplicate_weekday_is_skipped`.
- `an_invalid_window_entry_warns_with_the_spec_text` — `start >= end` gives exactly `planning day
  <days>: <start>–<end> ignored; using week_template` (§6.3) and that weekday keeps the template.
- `spans_come_from_hard_and_soft_notes_only`.
- **`sync_keeps_every_local_card_kind_local`** (the tripwire, spec §10 Phase 1s gate 3): reads
  `concat!(env!("CARGO_MANIFEST_DIR"), "/src/sync.rs")`; if the file is absent the test passes; if
  present, it asserts the text contains `commitments::LOCAL_CARD_KINDS` and
  `fn build_push_sends_no_local_card_nor_any_record_about_one`, with a message naming phase 1s.

**Verify:** `cargo test -p knowlu-engine --lib -- commitments::`, `cargo build --workspace` (0
warnings: `dead_code` on items used only by later tasks is avoided by landing their tests here).

## P7 — the classifier and the vault's code table (`commitments.rs`)

**Decisions.**

1. `pub fn code_table(vault) -> (BTreeMap<String, String>, Vec<String>)` — compact code → course
   slug, read from `courses/*.md` in file-name order (`code`, `title`, `name`, the slug) then
   `config/ingest.yaml` `course_map` keys; the leading-code reading
   `^[A-Za-z]{2,8}[ ._-]?[0-9]{1,4}[A-Za-z]?\b`, then c1c D4's reading anywhere as a fallback; first
   mapping wins; a code claimed by two slugs dropped with one warning. *Reason:* §3.4, R7.
2. `pub fn classify(series: &Series, codes, planning) -> Option<Class>` — pure; eligibility first
   (§3.4's bullets, including the `occurrence_starts`-unsupported rule), then rules 0–6 in order,
   first match wins, on the title trimmed of surrounding punctuation. *Reason:* §3.4; C5.
3. `Class = Kind { kind, course } | Routine { wake: bool, bed: bool }`; a midnight-crossing `sleep`
   is both sides. *Reason:* §3.4 rule 1.
4. `pub struct Series` — §3.2's record, plain data (`source_uid, calendar, title, where_, event_type,
   rule, has_master, rdate, unsupported, instances, meets, first, until, last_seen`) — is defined
   here so the classifier's tests build it by hand; P8 fills it from the sources. *Reason:* keeps
   P7 testable before normalisation exists.

**Tests first:** `compact_codes_match_across_spellings` (`CS 100`, `cs-100`, `CS100`);
`cs_1110_compsci_61a_and_math_20a_match_their_courses`;
`lms_names_reduce_by_the_d4_fallback` (`202640-BUI-100-101`);
`a_code_claimed_by_two_courses_is_dropped_with_a_warning`;
`work_on_cs_100_cs_100_study_group_and_study_for_ph_106_are_never_proposed`;
`a_planning_yaml_recurring_name_is_never_proposed`;
`cs_100_lab_is_a_lab_and_cs_100_lecture_is_a_class`;
`words_from_the_course_name_after_the_code_are_a_class`;
`cs_100_ta_hours_is_not_a_class_and_falls_through_to_meeting`;
`oh_matches_only_in_capitals` (`CS 100 OH` office-hours with course; `oh no` not);
`routines_match_the_whole_title_only` (`Wake Up` wake; `Sleep study` not a routine);
`a_midnight_sleep_series_is_both_sides`; `work_shift_club_team_practice` (rules 4–5);
`ineligible_series_are_never_classified` — `fromGmail`, `focusTime`, no master, `INTERVAL=2`, an
every-other-week pattern, one instance, all-day, `until` before today, an `RDATE`, `BYMONTHDAY`;
`a_holiday_gap_is_still_weekly` (a 14-day gap between two 7-day pairs).

**Verify:** `cargo test -p knowlu-engine --lib -- commitments::`.

## P8 — `Series`, normalisation and `state/calendar-series.json` (`commitments.rs`)

**Decisions.**

1. `pub fn series_from_google(value: &Value, tz, today) -> (Vec<Series>, Vec<String>, Vec<String>)`
   (series, calendars read, warnings) and `pub fn series_from_ics(feed_name, ics, tz, today)` (via
   P4's `weekly_series`) both reduce to P7's `Series`. *Reason:* §3.2 — one input shape.
2. Instances over `[today, today + 28)` in the vault's timezone (`config/ingest.yaml`'s `timezone:`,
   as `calfeed` reads it). Google `UNTIL` and `COUNT` are interpreted **on the device**: `UNTIL`
   converted to a local date (`20261205T055959Z` → 2026-12-04 in America/Chicago); `COUNT` expanded
   from `first` to its last date. *Reason:* §3.2 steps 2 and 4, M3.
3. `meets`: triples seen at least twice, grouped by `(start, end)`, days in `DAY_KEYS` order, entries
   ordered by `(first day, start)`. *Reason:* §3.2 step 3.
4. `where_`: the location trimmed; else the first description line that looks like a place (≤ 80
   characters; no `://`, `www.`; none of `zoom`, `teams`, `meet.google`, `webex`, `pwd`, `passcode`,
   `password`, `pin`, `meeting id`; no run of six digits). The description is then dropped and never
   stored. *Reason:* R4, I5.
5. `pub fn refresh_series(vault, fresh: &[(String /*calendar*/, Vec<Series>)], today) -> (SeriesFile,
   Vec<String>)`: a calendar in `fresh` replaces its series and its date; a series it no longer
   returns keeps its old `last_seen` until 14 days old, then is dropped; a calendar not in `fresh`
   is untouched; a feed removed from `config/ingest.yaml` keeps its series 14 days past its last read,
   then drops them. **A fetched-and-parsed ICS with zero masters is fresh** (M-e); a Google reply
   without `series`, a failed fetch, or text without `BEGIN:VCALENDAR` is not read. *Reason:* §3.3.
6. Bytes: `ledger::dumps_value` of `{calendars, series}` (keys sorted, series by `source_uid`) plus a
   trailing newline; **written only when some calendar was fresh and the bytes differ**; a missing
   file is silent and empty, an unreadable one empty with one warning. *Reason:* §3.3, constraint 8.
7. `pub fn read_series_file(vault) -> (SeriesFile, Vec<String>)` and `SeriesFile::instances_map()`
   giving P3's `instances` shape (per `source_uid`, its calendar's read date, that + 28, the
   instances). *Reason:* R21 uses the same file.

**Tests first:**
`google_value_normalises_to_one_series_with_meets_and_until` (§4.1's example → §3.3's example);
`until_z_is_a_local_date_in_the_vault_timezone`; `count_gives_until`;
`meets_keeps_triples_seen_twice_and_groups_by_time`;
`where_takes_the_location_else_a_place_like_description_line`;
`a_zoom_link_or_passcode_line_is_never_where`; `the_description_is_never_in_the_file`;
`a_fresh_calendar_replaces_its_series`; `a_series_unseen_for_14_days_is_dropped_not_before`;
`an_unread_calendar_keeps_its_series_and_date`;
`an_ics_fetched_with_zero_series_is_fresh_and_ages_its_old_series`;
`a_removed_feed_keeps_its_series_14_days_then_drops_them`;
`the_file_is_written_only_when_fresh_and_changed` (second identical refresh: mtime and bytes
unchanged); `the_file_bytes_are_sorted_dumps_value_with_a_trailing_newline`;
`a_malformed_file_reads_empty_with_one_warning`; `no_fresh_calendar_writes_no_file`.

**Verify:** `cargo test -p knowlu-engine --lib -- commitments::`.

## P9 — proposals and the window proposal (`commitments.rs`)

**Decisions.**

1. `pub fn proposals(file: &SeriesFile, set: &Commitments, codes, planning, template_window, today,
   for_cards: bool) -> Vec<Proposal>` — pure, in `source_uid` order. A series is proposed only if
   eligible and classified, **no note** has its `source_uid` (confirmed or marker), **no confirmed
   note has its signature** `(kind, course or lower-cased title, meets)`, and — when `for_cards` —
   it is not `office-hours`. *Reason:* §3.5, R8, R22.
2. `Proposal { kind, level, title, course, meets, where_, from, until, source_uid }`, level the
   kind's default. *Reason:* what §2.1's note needs.
3. **The window proposal**, only when no `planning-day` note and no `window` marker exist: per
   weekday, `start` = latest wake-side end, `end` = earliest bed-side start; one side missing →
   the template's; inverted, shorter than `min_block_minutes`, or neither side → the weekday is
   left out; entries grouped by equal `(start, end)`; `source_uid` = `window:` + the routine keys
   joined by `,`. A rejected window (the `window` marker) suppresses every later one. *Reason:* §3.5,
   M-i.

**Tests first:** `a_confirmed_or_declined_key_is_never_proposed`;
`a_signature_match_is_never_proposed` (same class from ICS and Google; a hand-written note);
`office_hours_is_proposed_for_the_screen_not_for_cards`;
`a_window_is_proposed_per_weekday_from_wake_and_bed` (weekday 07:30 wake, weekend 10:00 wake, 22:00
bed → two entries); `a_weekday_with_one_side_takes_the_other_from_the_template`;
`an_inverted_or_too_short_day_is_left_out`; `a_planning_day_note_suppresses_the_window`;
`a_window_marker_suppresses_every_later_window_even_with_a_new_routine`;
`proposals_are_in_source_uid_order_and_deterministic` (same input twice, equal output).

**Verify:** `cargo test -p knowlu-engine --lib -- commitments::`.

## P10 — `conflicts` and `fit`, the piece-2 API (`commitments.rs`)

**Decisions.**

1. `pub fn conflicts<'a>(set: &'a Commitments, instances: &InstancesMap, start: DateTime, end:
   DateTime) -> Vec<(&'a Commitment, Level)>` — half-open, local wall-clock, honouring actual
   instances inside the horizon (the rule of P3 decision 3), a midnight-crossing span split at
   midnight; sorted by `(level: hard first, meeting start, title, id)`; optional ones included.
   *Reason:* §7 — the same data `for_vault` uses, so overlap and capacity cannot disagree.
2. `pub enum Fit { Clear, OverlapsSoft(Vec<String>), OverlapsHard(Vec<String>) }` and `pub fn
   fit(&[(&Commitment, Level)]) -> Fit` — hard wins, then soft, optional ignored. *Reason:* C8.

**Tests first** (§7's list): `a_hard_class_overlapping_by_one_minute_is_overlaps_hard`;
`touching_end_to_start_is_clear`; `a_soft_club_gives_overlaps_soft_with_its_title`;
`decline_markers_never_conflict`; `a_span_outside_from_until_is_clear`;
`a_cancelled_instance_inside_the_horizon_is_clear`;
`a_midnight_crossing_span_meets_the_next_days_commitment`; `optional_conflicts_are_returned_but_fit_is_clear`.

**Verify:** `cargo test -p knowlu-engine --lib -- commitments::`.

## P15 — `WeekCalendar::for_vault` and the no-commitments equivalence (`weekcal.rs`, `commitments.rs`)

**Decisions.**

1. `pub fn for_vault(vault, events) -> WeekCalendar` = `from_file(config/week_template.yaml)` then
   `with_commitments(commitments::load(vault).spans(), window)` then `with_instances(read_series_file
   (vault).instances_map())`; warnings from both reads are returned by a sibling
   `for_vault_with_warnings`. *Reason:* §6.1 — one constructor for `rank` and `surface`.
2. The test-only `from_file` calls on fixtures in `ranking.rs`, `scheduling.rs`, `render.rs` and
   `surface.rs` stay. *Reason:* §6.1.

**Tests first** (`weekcal.rs` `mod tests`, reading `tests/fixtures/vault-*` in place, never writing):
- `for_vault_equals_from_file_without_commitments` — for `vault-s1`, `vault-s1-migrated` and
  `vault-full`, over the 35 days from 2026-08-24: equal `template_blocks`, `free_blocks`, `capacity`,
  `template_capacity` and `window(day)` (spec §8 test 1).
- `template_only_blocks_equals_template_blocks_without_commitments` — same fixtures and days (test 2).
- `decline_markers_change_nothing` — a scratch copy of `vault-full` whose `commitments/` holds only
  markers compares equal as in test 1 (test 3).
- `a_confirmed_class_note_reduces_capacity_on_its_days` — a scratch vault, one Mon/Wed class,
  capacity lower on Monday by exactly the overlap.

**Verify:** `cargo test -p knowlu-engine --lib -- weekcal:: commitments::`, `cargo test -p
knowlu-engine --test oracle --test surface_oracle`, `git diff --exit-code engine/tests/fixtures`.

## P11 — the `commitment-check` card: `file_card`, `emit_checks` (`commitments.rs`, `eventemit.rs`)

**After the merge.** Copies F2's emitter shape; calls F2's `eventemit::clock`.

**Decisions.**

1. `eventemit::clock` becomes `pub(crate)`; nothing else in `eventemit.rs` changes. *Reason:* M4 —
   one 12-hour format across every card.
2. `pub fn file_card(vault, kind: &str, fields: Vec<(&str, Node)>, title, body, today, ctx, journal)
   -> Result<PathBuf, String>` — **refuses a `kind` outside `LOCAL_CARD_KINDS`**, and is the only
   function in this piece that writes into `approvals/`. `write::create` with `yamlemit::Node::map`;
   `proposed_at` and `first_proposed_at` both `Node::Date(today)` (never an anchor); `expires: null`,
   `snooze_until: null`, `created_by: agent:commitments`; journal actor `agent:commitments`; path
   `approvals/commitment-check-<ingest::slugify(title)>.md`, `-2`, `-3` on collision. *Reason:* R20
   by construction (spec §5.2 "One constructor"), constraint 5.
3. `pub fn emit_checks(vault, proposals, changes, today, budget: i64, ctx, journal) -> (Vec<PathBuf>,
   usize, Vec<String>)`. `allowance = min(budget, 5 − commitment-check cards in approvals/ and
   archive/ whose first_proposed_at is today)`. Order: change cards (P12), then `class`, `lab`,
   `work`, the window, `club`, `meeting`, any other kind; within a kind by the first meeting's
   `(day, start)`, then `source_uid`. *Reason:* R9, §5.2; the F2 sizing means `defer_over_budget`
   never has overflow to snooze (constraint 7).
4. **Asked once:** a proposal is skipped when any card in `approvals/` or `archive/` has its
   `source_uid` **unless that card's status is `superseded`** (M-f). Office-hours proposals never
   reach here (P9 `for_cards`). *Reason:* §5.2, §5.5.
5. **Title** from structured fields only: `{title≤40} · {days} {range} · {question}`; days
   `Mon/Wed/Fri`, three or more consecutive collapse to `Mon–Fri`; `range` by `clock`
   (`12–12:50pm`); a second `meets` entry adds ` +1 more time`; questions `a class?`, `a lab?`,
   `work?`, `a club?`, `a meeting?`; the window `Your day · Mon–Fri 8am–10pm · plan in this
   window?`. *Reason:* §5.2; direction note §4.
6. **Frontmatter** as §5.2: `type: approval`, `kind: commitment-check`, `title`, `status: pending`,
   `source_uid`, `commitment:` (one flow mapping, P2's bytes), dates, `created_by`. **Body** as §5.2,
   first paragraph `**Is this part of your week?** Knowlu found it repeating on your calendar.`, then
   the level's sentence, `where` if present, and `Reject and it's ignored. Either way you won't be
   asked again.` *Reason:* the first paragraph is what `surface::first_paragraph` shows as `why`.

**Tests first** (`commitments.rs` `mod tests`; F2's test helpers where they fit):
- `file_card_refuses_a_kind_outside_local_card_kinds` — `event-check` and `amend` give `Err`, write
  nothing, journal nothing.
- `a_class_proposal_files_one_card_with_the_exact_title_frontmatter_and_body` — `CS 100 · Mon/Wed/Fri
  12–12:50pm · a class?`; the `commitment:` line's bytes; `created_by: agent:commitments`; both dates
  today; `expires: null`.
- `day_lists_collapse_and_a_second_meeting_says_plus_one_more_time`.
- `the_window_card_title_reads_your_day`.
- `at_most_five_a_day_counted_by_first_proposed_at` — 7 proposals, budget 15 → 5 cards; a card
  deferred to tomorrow by `defer_over_budget` still counts today.
- `the_budget_caps_below_five` — budget 2 → 2 cards.
- `order_is_changes_then_class_lab_work_window_club_meeting`.
- `a_second_run_files_nothing` — every proposal already has a card.
- `a_superseded_card_does_not_close_the_question` — archived `superseded` card, series back → one
  new card at `-2`.

**Verify:** `cargo test -p knowlu-engine --lib -- commitments:: eventemit::`.

## P12 — change detection: changed, ended, succeeded (`commitments.rs`)

**Decisions.**

1. `pub fn detect_changes(file: &SeriesFile, set: &Commitments, fresh: &BTreeSet<String>, today)
   -> Vec<Change>` — pure; only for confirmed notes keyed `gcal-series:`/`ics-series:` whose
   calendar is in `fresh`. **Changed:** series differs in `meets`, a non-empty `where`, or `until`.
   **Ended:** the series was dropped from the file and no fresh series has the note's signature →
   `until` = the last instance date the file held. **Succeeded:** an eligible class or lab series
   for the same course, different `meets`, first instance on or after the old series' last one,
   while the old one is ending → `meets`, `where` **and** `source_uid` = the new key, in one change.
   *Reason:* §5.4, R22.
2. `Change { target, change: Mapping, was: Mapping }` — changed fields only; `was` the note's current
   values, absent as `null`; no field proposed as `null`. *Reason:* §5.4; C1.
3. Titles: `CS 100 now meets Tue/Thu 9:30–10:45am · update?`, `CS 100 ends Dec 4 · update?`. Cards
   carry `target: commitments/<file>.md`, `change:`, `was:`, and the note's `source_uid`; filed
   through P11's `file_card`, charged to the cap, first in order.
4. **Asked once:** a card with the same `target` and the same canonical `change` text
   (`safe_dump_flow`) in `approvals/` or `archive/` suppresses another; a successor's key is not
   proposed on its own while its change card is pending, approved or rejected (P9 reads the cards'
   `change.source_uid`). *Reason:* §5.4, §5.5.

**Tests first:** `a_meets_change_on_a_fresh_calendar_files_one_change_card`;
`a_calendar_not_read_fresh_files_nothing`; `a_non_empty_where_change_is_a_change_and_an_empty_one_is_not`;
`an_ended_series_proposes_until_the_last_instance`;
`a_split_series_with_the_same_meets_files_nothing` (signature seen);
`a_split_with_new_meets_files_exactly_one_card_moving_source_uid`;
`the_successor_is_not_proposed_while_its_change_card_exists`;
`the_same_change_is_never_asked_twice_and_a_different_one_is`; `no_field_is_proposed_as_null`.

**Verify:** `cargo test -p knowlu-engine --lib -- commitments::`.

## P13 — the settlement arm (`approvals.rs`, `commitments.rs`, `engine/Cargo.toml`, `Cargo.lock`)

**After the merge.** A new `commitment-check` arm in `approvals::transition_note`, beside F3's
`event-check` arms in both the `rejected` branch and the `approved` branch.

**Decisions.**

1. **SHA-256 from `ring`.** The marker file name needs `sha256(source_uid)`; the engine has no
   SHA-256 on `main`. Add `ring = "0.17"` to `engine/Cargo.toml` — the **identical line** `c3-sync`
   adds, with its comment, so the two branches resolve to one line — and use `ring::digest::SHA256`.
   *Reason:* `ring` is already linked through `rustls`; C3′'s `dependency_boundary.rs` forbids
   `sha2`.
2. Helpers in `commitments.rs`: `create_confirmed(vault, &Mapping /*commitment:*/, today, ctx,
   journal)` (the §2.1 note; the window as §2.4; file name `slugify(title)`, `-2` on collision) and
   `create_marker(vault, key, ctx, journal)` (`commitments/declined-<10 hex>.md`, fields `type`,
   `status: declined`, `source_uid` only; no-op when a marker for the key exists). Actor
   `agent:commitments`. *Reason:* §2.3, §2.5.
3. **approved** → if a confirmed note has the card's `source_uid` or signature (or, for the window, a
   `planning-day` note exists): stamp `refused`, archive, one warning, no write. Else create the
   note, stamp `executed`, archive. **A change card** (it has `target`): for each field of `change`,
   compare the note's current value and `was` as canonical `safe_dump_flow(parse(value))` text; any
   mismatch → `refused`, archive, warning; else `write::write_literals` each field (a sequence
   through `to_literal`), `executed`, archive. *Reason:* §5.2, §5.4, R12 — no `amend` machinery;
   `AMENDABLE_FOLDERS` unchanged.
4. **rejected** → a proposal card writes its marker; the window card writes the one `window`
   marker; a change card writes nothing to the note, **but** when its `change` carries a new
   `source_uid`, the marker for that key (M-d). Then the generic delete-to-archive. *Reason:* §5.2,
   §5.4, §5.5.
5. **Withdrawal:** `pub fn withdraw_stale(vault, file, set, today, ctx, journal)` in `approvals.rs`
   (it owns the archive path) — a pending `commitment-check` whose `source_uid` left the series file,
   or which a confirmed note now covers, is stamped `superseded` and archived, no other write. Called
   by `rank` (P16) after `refresh_series`. *Reason:* §5.2 "Withdrawn", R9.
6. `pending`/`snoozed`: the existing machinery; no expiry. An old engine's `unknown kind` warning is
   accepted (§5.2). *Reason:* R9.

**Tests first** (`approvals.rs` `mod tests`, driving `write_literals` then `process_approvals` exactly
as `decide_inner` does):
- `approving_a_class_card_creates_the_confirmed_note_and_archives_executed` — the note's fields,
  `confirmed_at` today, `id` `cmt_…`, the card in `archive/` `executed`.
- `approving_when_a_confirmed_note_has_the_key_or_signature_is_refused_and_writes_nothing`.
- `approving_the_window_card_writes_the_planning_day_note`.
- `rejecting_a_class_card_writes_one_anonymous_marker` — the marker's bytes hold only `id`, `type`,
  `status`, `source_uid`; its file name is `declined-` + 10 hex of SHA-256 of the key.
- `rejecting_the_window_card_writes_the_window_marker`.
- `an_until_change_onto_a_note_with_no_until_applies`.
- `a_meets_change_applies_and_reads_back_as_a_sequence_load_accepts`.
- `a_stale_was_is_refused_and_archived`.
- `rejecting_a_successor_change_writes_the_successors_marker`.
- `a_pending_card_whose_series_left_the_file_is_withdrawn_superseded`.
- `amendable_folders_is_unchanged` — `AMENDABLE_FOLDERS == ["tasks", "courses"]`.

**Verify:** `cargo test -p knowlu-engine --lib -- approvals:: commitments::`, `cargo test -p
knowlu-engine --test dependency_boundary`, `cargo build --workspace` at 0 warnings.

## P14 — the engine transport (`cloudmodel.rs`, `cli.rs`, `cloud_contract.rs`, `app/tests/scaffold.rs`)

**After the merge** (all four files but the last are in `j-followups`' diff).

**Decisions.**

1. `cloudmodel::fetch_calendar(client, name) -> Result<(String, Option<Value>), String>` sends
   `?name=<name>&accepts=series` and returns the reply's `series` as `Some` only when present.
   *Reason:* §4.2; a reply without it is today's reply (§4.3 "new engine, old function").
2. `cli::Fetchers` gains `pub series: Option<&'a SeriesStash>` and `#[derive(Default)]`;
   `pub type SeriesStash = RefCell<BTreeMap<String, StashEntry>>`, `pub enum StashEntry {
   Google(Value), Ics(String) }`, keyed by the feed's **URL** as the calendar closure receives it.
   *Reason:* review I8 — explicit plumbing, so `run_with` tests inject series without a network.
3. `cli::run` owns one stash; its calendar closure stashes the Google `series` for `cloud:google`, and
   the ICS text for any other feed it fetched; a failed fetch stashes nothing. The closure keeps its
   `Fn(&str) -> Result<String, String>` shape, so `load_calendar_events` and every oracle test are
   untouched. *Reason:* §4.2.
4. `app/tests/scaffold.rs`'s one `Fetchers` literal (line 99 on `main`) gains `..Default::default()`.
   *Reason:* the one literal outside the engine (review I8).

**Tests first:**
- `cloud_contract.rs`: the existing request-line assertion becomes `GET
  /functions/v1/ingest-calendar?name=google&accepts=series HTTP/1.1` (same commit, §4.2);
  `a_calendar_reply_without_series_parses_to_none`; `a_calendar_reply_with_series_returns_it`.
- `cli.rs`: `fetchers_default_has_no_calendar_events_or_series`;
  `the_calendar_closure_stashes_google_series_by_url` and `…_stashes_ics_text_for_a_direct_feed`
  (loopback server on `127.0.0.1:0`); `a_failed_fetch_stashes_nothing`.

**Verify:** `cargo test -p knowlu-engine --test cloud_contract`, `cargo test -p knowlu-engine --lib --
cli::`, `cargo test -p knowlu --test scaffold`, `cargo build --workspace` at 0 warnings.

## P16 — `rank` wiring (`cli.rs`, `commitments.rs`)

**Decisions.**

1. In `run_with`, directly after `load_calendar_events` and before the `WeekCalendar` is built:
   normalise every stash entry (P8), `refresh_series`, then `approvals::withdraw_stale` (P13), then
   `load`, `detect_changes` (P12), `proposals(.., for_cards: true)` (P9) and `emit_checks` (P11)
   with `budget = max(0, planning.daily_approval_budget − count_proposals_created(vault,
   today))` — recounted here, after the events pass took its share. The number filed is added
   to `approvals.pending`, as `j-followups` does for its checks (M8, re-review M-a). *Reason:* §4.2, §5.2.
2. `WeekCalendar::from_file(...)` in `run_with` becomes `WeekCalendar::for_vault(vault, cal_events)`.
   *Reason:* §6.1, C3.
3. `commitments::record_baseline(vault, &set, &cal, today)`: only when a confirmed `planning-day`
   note exists; if `state/plan.json` is missing, write `{date: today, start, end}` of the
   **template's** window for today; if its `date` is another day, write today's current
   `window(today)`; if it is today, leave it. `ledger::dumps_value`, trailing newline. **No diff
   here.** *Reason:* §6.4, R23 as revised (re-review N2).
4. Every warning from normalisation, the series file, `load`, withdrawal and the card passes joins
   the **`calendar`** step's message; **no new run-record step**. *Reason:* R13;
   `run-records-reference.json` and `rank_writes_today_md_and_reports_the_step_sequence` pin the
   step names.

**Tests first** (`cli.rs` `mod tests`):
- **`rank_on_vault_full_writes_no_new_state`** (spec §8 test 5) — a scratch copy of `vault-full`
  ranked with the fixture's feed configuration gains no `state/calendar-series.json`, no
  `state/plan.json`, no `commitments/` and no card, and its `calendar` step message equals the
  message the same run produced before this task (captured as a literal in the test).
- `rank_files_a_commitment_check_for_an_injected_class_series_and_counts_it_pending` — a stash with
  one `CS 100` series and a `courses/cs-100.md` → one card; `Approvals: N pending` counts it;
  and **`commitments/` does not exist afterwards** (R18: a proposal is never a note).
- `commitment_checks_take_only_what_the_events_pass_left` — events cards + ours ≤ 15, nothing
  deferred.
- `rank_subtracts_a_confirmed_class_from_capacity` — `today.md`'s capacity line drops by the class.
- `rank_withdraws_a_card_whose_series_left_the_file`.
- `rank_records_the_template_window_when_plan_json_is_missing`;
  `rank_keeps_the_days_first_window_across_runs`; `rank_records_todays_window_on_a_new_day`;
  `rank_writes_no_plan_json_without_a_planning_day_note`.
- `a_malformed_series_file_warns_in_the_calendar_step` — and the step names are unchanged.

**Verify:** `cargo test -p knowlu-engine --lib -- cli::`, `cargo test -p knowlu-engine --test oracle
--test surface_oracle`, `git diff --exit-code engine/tests/fixtures`.

## P17 — the read model's commitment blocks and window (`surface.rs`)

**Decisions.**

1. `surface::load` builds `WeekCalendar::for_vault(vault, events)`. *Reason:* §6.1 — the console
   and `rank` see the same week.
2. `the_day`: `day_start`/`day_end` come from `cal.window(today)`; the class gap walk runs over
   `template_only_blocks(today)`; each span from `spans_on(today)` is its own block — `kind: "class"`
   for class and lab, `"busy"` otherwise, `label` = the note's title — clamped to the window and
   dropped if wholly outside; a `busy` event block whose start and end equal a commitment block's is
   dropped. *Reason:* §6.2, R15; both kinds exist, so the console renders them unchanged.

**Tests first** (`surface.rs` `mod tests`, scratch copies via the existing `fixture_full()` helper):
`the_day_draws_a_club_as_busy_with_its_title_not_as_class`;
`a_class_commitment_is_drawn_as_class_with_its_title`;
`a_commitment_straddling_the_window_is_clamped_and_one_outside_is_not_drawn`;
`a_google_event_identical_to_a_commitment_is_drawn_once`;
`the_day_uses_the_planning_window`.

**Verify:** `cargo test -p knowlu-engine --lib -- surface::`, `cargo test -p knowlu-engine --test
surface_oracle` (the three references unchanged), `git diff --exit-code engine/tests/fixtures`.

## P18 — `moved`, live in `surface`, and the `--window` preview (`surface.rs`, `main.rs`, `commitments.rs`)

**Decisions.**

1. `commitments::baseline(vault, &set, &cal, today) -> Option<(Time, Time)>` — pure read: no note →
   `None`; `state/plan.json` dated today → its window; missing or unreadable → the template's
   `(day_start, day_end)` for today; dated another day (today's first `rank` has not run) → the
   current `window(today)`, so nothing is reported. *Reason:* §6.4 — the first window is diffed
   against the template (re-review N2).
2. `commitments::moved(ranked, today, now_cal, base_cal) -> Option<Moved>` — pure: both
   `designate_today_explained` runs from the **same** ranked list; part of day from the take's free
   block start (before 12:00 morning, before 17:00 afternoon, else evening); moved-to when the part
   changed or the task is new; `dropped` when only in the baseline plan. `Moved { to: {morning,
   afternoon, evening}, dropped, text }`, `text` largest group first, ties morning→evening, `; 1 no
   longer fits today` appended; `None` when nothing moved. *Reason:* §6.4.
3. `surface::load`: when `baseline != cal.window(today)`, runs the second designation over
   `cal.clone().with_day_window(today, base)` and stores `moved`; **writes nothing**. The today view
   serialises `moved` with `skip_serializing_if = "Option::is_none"`. *Reason:* §6.4; constraint 14.
4. `main.rs`: `surface --view today --window '<flow sequence>'` parses the sequence, validates it as
   §2.4 validates `window` (an invalid value exits 2 with a message, as an unknown view does), and
   computes the day under it with `moved` against the **current** window. Writes nothing. *Reason:*
   §6.4 "Preview"; the phase-2 editor's data source.

**Tests first:**
- `commitments.rs`: `moved_text_orders_largest_first_and_names_dropped`;
  `part_of_day_boundaries_are_12_and_17`; `identical_plans_move_nothing`.
- `surface.rs`: **`surface_reports_moved_at_once_after_a_window_edit_without_a_rank`** — a scratch
  vault with a planning-day note and `plan.json` recording 08:00–18:00 today; the note's window is
  edited to 08:00–22:00 through `write`; `load` (no `rank`) reports `moved` with a non-zero
  `evening`; **`the_first_window_is_diffed_against_the_template`** — note present, no `plan.json`;
  `a_second_edit_is_reported_against_the_days_first_window`; `an_undone_edit_reports_nothing`;
  `a_plan_json_from_yesterday_reports_nothing_before_the_first_rank`;
  `no_note_means_no_moved_key_in_the_json`; `surface_load_writes_nothing` (the vault's file list and
  bytes are equal before and after).
- `main.rs` (or `surface.rs` through the function `main` calls):
  `window_preview_diffs_against_the_current_window_and_writes_nothing`;
  `a_bad_window_argument_is_refused`.

**Verify:** `cargo test -p knowlu-engine --lib -- surface:: commitments::`, `cargo test -p
knowlu-engine --test surface_oracle`, `git diff --exit-code engine/tests/fixtures`.

## P19 — the `commitments` command and the model-reach test (`main.rs`, `cli.rs`, `cloud_contract.rs`)

**Decisions.**

1. `knowlu-engine commitments --vault <v> [--today YYYY-MM-DD] [--json]` — **always exits 0**. It
   builds the same calendar fetcher and stash as `rank` (a private `cli::calendar_fetcher` shared
   with `cli::run`, extracted here), fetches each `calfeed::calendar_entries` feed **directly
   through that fetcher, not through `load_calendar_events`**, so `state/calendar.md` is not
   rewritten; refreshes the series file; prints `{"proposals": [...], "warnings": [...]}` (the
   window proposal among them, office hours included) through `ledger::dumps_value` with `--json`,
   or one card-style title per line without it. **Writes no note, no card, no journal record.**
   *Reason:* R14 — the phase-2 screen's data source.
2. `rank_cannot_reach_a_judgment_endpoint` scans `commitments.rs` whole, and `main.rs` between the
   markers `// commitments command: begin` and `// commitments command: end`, for the same forbidden
   words. *Reason:* §6.5, M5 — constraint 4.

**Tests first:**
- `cli.rs`: `commitments_command_prints_proposals_and_writes_no_note_card_or_journal` — injected
  stash; afterwards the only changed path under the vault is `state/calendar-series.json`;
  `commitments_command_with_no_feed_prints_an_empty_list`.
- `cloud_contract.rs`: the extended `rank_cannot_reach_a_judgment_endpoint`, plus
  `the_commitments_arm_markers_exist` so the scan cannot pass vacuously.

**Verify:** `cargo test -p knowlu-engine --lib -- cli::`, `cargo test -p knowlu-engine --test
cloud_contract`, then a hand smoke on a scratch vault (`scripts\scratch-vault.ps1`):
`cargo run -p knowlu-engine -- commitments --vault <scratch> --json` exits 0.

## P20 — docs (`docs/surface/anatomy.md`, `CLAUDE.md`)

**Decisions.**

1. `anatomy.md`: one paragraph on where commitment blocks come from (`class`/`busy`, clamp,
   de-duplication), one on `moved` (baseline, live, omitted when none), one on the
   `commitment-check` card. *Reason:* §6.2, §10.
2. `CLAUDE.md`: `commitments/` in the vault folder list; the `commitments` command in the engine's
   command list (one entry, the `coursework-discover` style); `surface --window`; a line that
   `commitment-check` cards are local-only by kind (`commitments::LOCAL_CARD_KINDS`). Nothing else.
   *Reason:* §10; CLAUDE.md is what the next session reads first.

**Tests first:** none (docs). **Verify:** `cargo test --workspace` at 0 warnings (the full gate,
last), `git diff --exit-code engine/tests/fixtures`, and `git diff --stat` shows only this plan's
files.

---

## Decisions taken, with their cost if wrong

| decision | task | cost if wrong |
|---|---|---|
| `CommitmentSpan` in `weekcal.rs`, not `commitments.rs` | P3 | a rename; the spec's `Span` is the same thing |
| the series file keeps ineligible series with a reason | P4, P8 | a larger generated file; nothing syncs it |
| `ring` for the marker's SHA-256, the same line as `c3-sync` | P13 | a one-line merge resolution; `sha2` is forbidden by C3′'s boundary test |
| `withdraw_stale` lives in `approvals.rs` | P13 | a move between two files |
| the `commitments` command bypasses `load_calendar_events` | P19 | the snapshot is one run staler than it could be |
| the baseline is today's single span, not the whole note | P16, P18 | only today is ever diffed, which is all `moved` reports |
| main.rs markers bound the model-reach scan | P19 | a marker moved by hand makes `the_commitments_arm_markers_exist` fail loudly |

## Fidelity ledger (spec section → task → proof)

| spec | requirement | task | what proves it |
|---|---|---|---|
| §2.1–2.2, R17 | the confirmed note, its field rules, reserved kinds, levels | P6 | `load_reads_a_confirmed_commitment`, `reserved_kinds_load_without_a_warning_at_their_default_levels`, `an_unknown_kind_loads_soft_with_a_warning`, `an_invalid_meets_entry_is_skipped_and_a_note_with_none_is_ignored`, `a_hand_written_proposed_note_is_ignored_with_a_warning` |
| §2.2, R3 | `cmt` ids; `commitments/` a note folder; backups | P1 | `kind_for_maps_type_commitment_to_cmt`, `id_repair_gives_a_commitment_note_a_cmt_id`, `backup_folders_has_ten_entries_with_commitments_before_state` |
| §2.3, R19 | the anonymous decline marker | P6, P13 | `a_decline_marker_contributes_its_key_only`, `rejecting_a_class_card_writes_one_anonymous_marker` |
| §2.4, §6.3, R11 | the planning-day note; per-weekday window; lowest id; invalid entry warning | P3, P6, P13 | `a_window_entry_replaces_that_weekday_only`, `the_planning_day_lowest_id_wins_and_a_duplicate_weekday_is_skipped`, `an_invalid_window_entry_warns_with_the_spec_text`, `approving_the_window_card_writes_the_planning_day_note` |
| §2.5 | duplicates: lowest id; settlement never adds a third | P6, P13 | `duplicate_source_uid_keeps_the_lowest_id_and_warns_once`, `approving_when_a_confirmed_note_has_the_key_or_signature_is_refused_and_writes_nothing` |
| R2 | single-line flow values with PyYAML's bytes | P2 | `flow_emits_a_meets_sequence_on_one_line`, `flow_emits_a_commitment_mapping_with_nulls_and_a_nested_sequence`; oracle green |
| §3.1, R6 | ICS series; `@google.com` keyed as `gcal-series:` | P4 | `a_google_uid_is_keyed_gcal_series_without_the_suffix` |
| §3.2 step 1, C4 | overrides, cancellations, declines, transparency, EXDATE | P4 | the seven per-rule tests in P4 |
| §3.2 steps 2–6, R4, I5 | UNTIL/COUNT on the device; meets; place-like `where`; description dropped | P8 | `until_z_is_a_local_date_in_the_vault_timezone`, `count_gives_until`, `meets_keeps_triples_seen_twice_and_groups_by_time`, `a_zoom_link_or_passcode_line_is_never_where`, `the_description_is_never_in_the_file` |
| §3.3, M-e, M-g | the series file: freshness, 14 days, removed feeds, zero-series fresh, bytes, write-on-change | P8 | P8's eleven file tests |
| §3.4, R7, C3, I2 | eligibility; rules 0–6; the code table; blind spots; unsupported rules | P7 | P7's tests, including `cs_1110_compsci_61a_and_math_20a_match_their_courses`, `work_on_cs_100_cs_100_study_group_and_study_for_ph_106_are_never_proposed`, `ineligible_series_are_never_classified` |
| §3.5, R8, R22, M-i | proposals; signature; office hours not carded; window proposal and its marker | P9 | P9's nine tests |
| §4.1, R5, I6, M17 | `/ingest-calendar` `series`: owned calendars, paging, completeness, caps, budget, bounds, `toIcs` pin | P5 | P5's fourteen Deno tests |
| §4.2, I8 | `accepts=series`; `Fetchers.series`, `Default`; URL-keyed stash; scaffold literal | P14 | the updated request-line assertion, `a_calendar_reply_without_series_parses_to_none`, the stash tests, `cargo test -p knowlu --test scaffold` |
| §4.3 | old engine / new function and the reverse | P5, P14 | `without accepts=series the google reply is byte-for-byte today's`; `a_calendar_reply_without_series_parses_to_none` |
| §5.2, R9, R20, M-f | the card: one constructor, cap 5, budget, order, title, frontmatter, body, superseded exemption | P11 | `file_card_refuses_a_kind_outside_local_card_kinds`, `a_class_proposal_files_one_card_with_the_exact_title_frontmatter_and_body`, `at_most_five_a_day_counted_by_first_proposed_at`, `a_superseded_card_does_not_close_the_question` |
| §5.2 settlement, R12 | approve → note; reject → marker; window; refused; withdrawn | P13 | the eleven P13 tests |
| §5.2 count, M8, M-a | cards filed count in `Approvals: N pending`, in `cli.rs` | P16 | `rank_files_a_commitment_check_for_an_injected_class_series_and_counts_it_pending` |
| §5.4, R22, M-d | changed / ended / succeeded; `was` check; `write_literals`; successor marker | P12, P13 | P12's nine tests; `a_meets_change_applies_and_reads_back_as_a_sequence_load_accepts`, `a_stale_was_is_refused_and_archived`, `rejecting_a_successor_change_writes_the_successors_marker` |
| §5.5 | never re-asked, every row | P9, P11, P12, P13 | `a_confirmed_or_declined_key_is_never_proposed`, `a_second_run_files_nothing`, `the_same_change_is_never_asked_twice_and_a_different_one_is`, `a_window_marker_suppresses_every_later_window_even_with_a_new_routine` |
| §6.1, R21, C3 | `for_vault`; spans in `template_blocks`; actual instances in the horizon | P3, P15, P16 | `inside_the_horizon_actual_instances_replace_the_weekly_span`, `a_confirmed_class_note_reduces_capacity_on_its_days`, `rank_subtracts_a_confirmed_class_from_capacity` |
| §6.2, R15 | blocks by kind, clamp, de-duplication, `template_only_blocks` | P3, P17 | P17's five tests; `template_only_blocks_ignores_commitment_spans` |
| §6.4, R23, N2 | baseline in `rank`; `moved` live in `surface`; first window vs template; preview | P16, P18 | `rank_records_the_template_window_when_plan_json_is_missing`, `surface_reports_moved_at_once_after_a_window_edit_without_a_rank`, `the_first_window_is_diffed_against_the_template`, `window_preview_diffs_against_the_current_window_and_writes_nothing` |
| §6.5, R13, M5 | warnings into the `calendar` step; no new step; model-reach scan | P16, P19 | `a_malformed_series_file_warns_in_the_calendar_step`, the extended `rank_cannot_reach_a_judgment_endpoint`, `the_commitments_arm_markers_exist` |
| §7, C8 | `conflicts`, `fit` | P10 | §7's eight tests |
| §8 | frozen and read-model references; no-commitments equivalence; no new state on `vault-full` | P1, P3, P15, P16, P17, P18 | `for_vault_equals_from_file_without_commitments`, `template_only_blocks_equals_template_blocks_without_commitments`, `decline_markers_change_nothing`, `rank_on_vault_full_writes_no_new_state`; `--test oracle --test surface_oracle` with `git diff --exit-code engine/tests/fixtures` in every relevant task |
| §9, R18 | proposals never notes; description never stored; function logs nothing | P8, P16, P5 | `the_description_is_never_in_the_file`; the `commitments/`-absent assertion in P16; P5's error-line rule |
| §9 privacy lines | line 1 with phase 1's first release | **release gate** (Dependencies 3) | `site/privacy.html` is outside this plan's files |
| §10 Phase 1s, R20, N1 | local cards never sync, by kind | P6 (tripwire), **phase 1s** | `sync_keeps_every_local_card_kind_local`; phase 1s's `build_push_sends_no_local_card_nor_any_record_about_one` |
| R3 | the migration live on prod before a release with `cmt` | **release gate** (Dependencies 2) | the controller's release checklist |
| R14 | the `commitments` command, exits 0, writes no note or card | P19 | `commitments_command_prints_proposals_and_writes_no_note_card_or_journal` |
| §10 files | `eventemit::clock` `pub(crate)`; `render.rs` not edited; docs | P11, P20 | the P11 title tests; `git diff --stat` in P20 |

**Not in phase 1, by the spec:** §5.1 (the screen), §5.3 and R10 (the fallback card), the *Your week*
panel and editor, M-c's `EDITABLE` work (phase 2); R24 and the registrar (phase 3); §10 Phase 1s
(sync, with C3′).

## Open questions for Quinn

1. **Privacy line 1's final wording** (spec §9). It is a release gate for phase 1's first release,
   worded by you with the lawyer's read. It does not block building any task here.

§11 Q1 (the myBama login) stays open in the spec and affects only phase 3. §11 Q2 (the consent-screen
justification) has the spec's recommendation — change it at the next submission — and blocks nothing
in this plan.

## Self-review (2026-09-23)

Checked this plan against the spec at `80350a9`, section by section, for phase 1's scope:

- **Every phase-1 requirement maps to a task.** Every row of §10's phase-1 file table has a task
  (`render.rs` is intentionally absent, per M-a). Every R-row that applies to phase 1 (R1–R9, R11–R23)
  appears in the ledger; R10 and R24 are phases 2 and 3.
- **Task size.** P13 is the largest at four files (two of them `Cargo.toml` and `Cargo.lock` for one
  dependency line); P14 is four; every other task is one to three.
- **Two gaps found and closed while writing:** §6.3's exact warning text had no test (added to P6),
  and R18's "a proposal is never a note" had no end-to-end assertion (added to P16's rank test).
- **One spec gap found and fixed in the spec:** with a `state/plan.json` from yesterday and no rank
  yet today, the spec did not say what `surface` diffs against; it now says the current window, so
  nothing is reported (§6.4), and P18 tests it.
- **Frozen references:** no task edits a fixture; P1, P3, P4, P15, P16, P17 and P18 each verify with
  `--test oracle`/`--test surface_oracle` and `git diff --exit-code engine/tests/fixtures`.
