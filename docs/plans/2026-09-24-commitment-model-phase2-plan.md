# The commitment model, phase 2 — plan (the app)

**Status: PLAN, written 2026-09-24. Not executed.** **Base:** branch `p2-commitments` (phase 1's
engine, PR #14, with C1c's app, PR #13, merged in; spec §8). Once #13 and #14 are on `main`, the
branch rebases onto it; no task here depends on the merge order.
**Written to survive a context compaction:** every task names its files with line ranges from the
code as it stands at `3d984d7`, its interfaces, its tests and its commit, so a fresh session can
execute from this document alone.

> **For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development**

**Goal.** Students confirm their week in the app. The console's first screen after Finish asks which
calendar series are theirs and sets the planning day. A *Schedule* view ("Your week") lists every
confirmed commitment with its kind and level as controls, offers office hours, and edits the planning
day with a live preview. The today view says what moved. A course with no class times gets a
"When does it meet?" card that the Decisions view can answer.

**Architecture.** The engine does all the work. `knowlu-engine commitments --confirm <file>` writes
the screen's answers. It re-derives every row from `state/calendar-series.json` and fetches nothing.
`commitments::overview` is the pure read behind the view. `commitments::emit_asks` and a
`commitment-ask` settlement arm give the per-course card. `rank` files no card on the vault's first
day. The app adds `app/src/week.rs`: two commands spawn the sibling engine and two read in-process.
`commands.rs` gains `answer_card` and two `EDITABLE` fields, each checked by an engine function.
`console.js` gains the screen, the view, the window editor, the moved line and the card form.

**Tech stack.** Rust 1.98 (`stable-x86_64-pc-windows-gnu`), `jiff`, `serde_yaml_ng`, `serde_json`,
Tauri 2, plain JavaScript (no bundler), Playwright (`scripts/wizard-check.py`).

**Spec.** `docs/specs/2026-09-24-commitment-model-phase2-design.md` (binding; "the spec", sections
§1–§8, decisions D1–D8, with its "Amendments (2026-09-24, plan review)" section, which follows the
pre-execution review `docs/plans/2026-09-24-commitment-model-phase2-plan-review.md`). Where it is silent, `docs/specs/2026-09-23-commitment-model-design.md`
governs ("the parent", §2, §3.3, §3.5, §5.1–§5.3, §6.4, §10). Where this plan and the spec disagree,
the spec wins and this plan is wrong.

## Global Constraints (binding on every task)

1. **Line endings:** LF in every file this plan touches; `engine/tests/fixtures/**` is `-text` and
   is never re-encoded.
2. **0 warnings:** `cargo build`/`cargo test` print no `warning:` line but the accepted `.rsrc merge
   failure: multiple non-default manifests`.
3. **TDD:** in every task, the failing test comes first, is run and seen failing, then the code.
4. **Frozen references never change:** the eight Python references in `engine/tests/fixtures/` and
   `surface-today-{s1,s1-migrated,full}.json` are byte-identical at the end of every task (none of
   their vaults has `courses/`, a planning-day note or a first-day journal that reaches a new path).
5. **Journal first:** every note write goes through `write::create` / `write::write_literals`, which
   journal before the file; no note is parsed and re-dumped.
6. **`app/src/commands.rs` computes nothing:** it marshals, calls the engine and returns envelopes;
   every validation named in this plan is an engine function.
7. **`rank` never calls a model:** `rank_cannot_reach_a_judgment_endpoint`
   (`engine/tests/cloud_contract.rs:828`) stays green, and `--confirm` fetches nothing (Q4's pin).
8. **No typed text on the confirm screen:** `#week-setup` holds no `input type="text"` and no
   `textarea`; times are `<input type="time">` pickers.
9. **Tauri command counts:** the console window goes from **42 → 47** (`answer_card`,
   `commitment_proposals`, `commitments_confirm`, `your_week`, `preview_window`); the wizard window
   stays **29**; **66** distinct.
10. **Human writes:** the actor is `quinn` via `dashboard` (`commands::console_ctx()`; the
    engine's `--actor quinn --via dashboard`); settlements stay `agent:commitments`.
11. **Never `git stash`.** Set work aside with a WIP commit.
12. **One cargo at a time, with `-j 2`** (host memory is low). App tests that spawn the engine need
    the real exe: run `cargo build -p knowlu-engine -j 2` first, because `app/build.rs` drops a
    zero-byte placeholder over `target/debug/knowlu-engine.exe`.
13. **Commits:** write the message with the Write tool to the session scratchpad, then
    `git commit -F <file>`. Every message ends with the trailer
    `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.
14. **No personal data:** every title, room, course and key in a test is invented.
15. **The wizard keeps nine panels:** `the_wizard_has_nine_panels_and_the_privacy_words_and_no_live_fetch`
    (`app/tests/static_assets.rs:337`) is not edited.

## Fidelity ledger

| Spec requirement | Task |
|---|---|
| D1 — the confirm screen is the console's first screen, over the first-run view; the wizard keeps nine panels | Q11 (screen), Q9 (its commands), Q12 (nine panels re-verified) |
| D2 (amended A1) — no `commitment-check` proposal or window card on the vault's first day, the vault-local date of the earliest journal record; no journal = day 1 | Q1 (`first_ts`, `vault_day`, the Athens test) |
| D3 — the screen shows on day 1 with no planning-day note; Not now hides it for the session; Finish always writes the note | Q5 (`setup`), Q11 (session flag; Finish refuses an all-blank *Your day*, review finding 2) |
| D4 — the screen's writes are `commitments --confirm <file>`; the panel's Add and the editor's Save use it | Q3, Q4, Q9, Q10 |
| D5 — `window` written by the engine; `kind`/`level` join `EDITABLE`, refused off `commitments/` and outside §2.2's lists | Q4 (window), Q8 (`EDITABLE`) |
| D6 — a new console view `schedule`, headed "Your week" | Q10 |
| D7 — the today view prints `moved.text` under the day heading | Q10 |
| D8 — no remove control | Q10 (none is built; `static_assets` pins its absence) |
| §2 where it appears, how it loads, "Reading your calendar…", failed fetch shows only *Your day* | Q9 (`commitment_proposals`), Q11 |
| §2 groups, card order, `when` label, `where` in small type | Q2 (`when`, order), Q11 |
| §2 courses with no class row; `uncovered_courses` in `commitments --json` | Q2 |
| §2 row controls (Mine / Not mine / unanswered; presets; level on *Mine* only; no typing) | Q11 |
| §2 Finish / Not now and their one line | Q11 |
| §3 input file, optional keys, temp file deleted afterwards | Q3 (parse), Q9 (temp file) |
| §3 series file is the source; stale keys warned; only `level` from the app | Q3 |
| §3 `mine` → `create_confirmed`; second Finish a no-op | Q3 |
| §3 `not_mine` → markers for the whole twin group | Q3 |
| §3 `window`: invalid → nothing written, exit 2; create or edit one line | Q4 |
| §3 actor `quinn` via `dashboard`, journal first, no cap charge; order; output; exit codes | Q3, Q4 |
| §3 pending cards withdrawn at the next `rank`; no network pin | existing `withdraw_stale` (checks), Q7 (asks), Q4 (pin) |
| §4 `overview` data, serialised through `ledger::dumps_value` | Q5 |
| §4 commitment rows: kind select, level control via `set_fields` | Q8 (engine + app), Q10 (UI) |
| §4 office hours Add as `optional`; uncovered courses text | Q10 |
| §4 window editor: seven rows, same-as-Monday, flow sequence, 400 ms preview, moved or "No change", five items, Save, errors under the row; the same pickers in both places, the preview only in *Schedule* (amended A6) | Q9 (`preview_window`), Q10, Q11 (Q11-a) |
| D6 (amended A4) — `schedule` polls the read model as `today` and never reaches `surface::View::parse` | Q10 (`stateView()`), Q12 (anatomy §2) |
| §5 emitter (amended A2): `emit_asks(vault, proposals, …)` after `emit_checks`, 2 a day, day 3 counted as D2 counts, `file_card`, slug order, counted pending | Q6 |
| §5 a course is covered by a class note's `course:` or, with none, the code leading its title — one rule for listing, asking, settling, withdrawing | Q2 (`covered_course_keys`), Q7 (`has_class`, review finding 3) |
| §5 the answer (amended A3): `answer_card` on a pending ask only, `answer_meets` via `write::to_literal`, then decide's path; invalid → pending, the warning shown from the envelope until the next repaint | Q7 (settlement), Q8 (`answer_card`, `check_answerable`, review finding 6) |
| Parent §5.3 as superseded (amended A5): the ask's class note is titled by the course note, `commitments/<slugify(title)>.md` | Q7 (Q7-b) |
| §5 the form: day toggles, times, add another time, Save times, No set times, Snooze; the deck shows **Answer…**, which opens Decisions (amended A7) | Q11 (Q11-b) |
| §6 five commands, `week.rs`, counts 47/29, nine mutating commands | Q8, Q9, Q12 |
| §6 engine and app files; docs (`anatomy.md`, `app/README.md`, `CLAUDE.md`) | Q1–Q12, Q12 |
| §7 engine tests | Q1–Q7 |
| §7 app tests (`week.rs`, `commands.rs`) | Q8, Q9 |
| §7 static and behavioural (`static_assets.rs`, `wizard-check.py` console scenario) | Q10, Q11 |
| §7 unchanged: fixtures, surface JSONs, nine panels | every task (constraint 4), Q12 (byte check) |
| §8 sequencing, C3′ not blocking, privacy unchanged | header; nothing reads new data |

## Order and file ownership

| Task | Unit | Files | Depends on |
|---|---|---|---|
| Q1 | the vault's day and the day-1 gate (D2) | `engine/src/ledger.rs`, `engine/src/commitments.rs`, `engine/src/cli.rs` | — |
| Q2 | `proposal_value` (`when`), card order, `uncovered_courses` in `--json` | `engine/src/commitments.rs`, `engine/src/cli.rs`, `engine/src/main.rs` | Q1 |
| Q3 | `commitments::confirm`: `mine` and `not_mine` as the human | `engine/src/commitments.rs` | Q2 |
| Q4 | the window, the `--confirm` flag, exit codes, the no-network pin | `engine/src/commitments.rs`, `engine/src/cli.rs`, `engine/src/main.rs`, `engine/tests/commitments_confirm.rs` (new) | Q3 |
| Q5 | `commitments::overview` | `engine/src/commitments.rs` | Q4 |
| Q6 | `emit_asks` and its `rank` wiring | `engine/src/commitments.rs`, `engine/src/cli.rs` | Q5 |
| Q7 | the `commitment-ask` settlement arm and withdrawal | `engine/src/commitments.rs`, `engine/src/approvals.rs`, `engine/src/cli.rs` | Q6 |
| Q8 | `kind`/`level` in `EDITABLE`, `answer_card` | `engine/src/commitments.rs`, `app/src/commands.rs`, `app/src/main.rs`, `app/tests/commands.rs` | Q7 |
| Q9 | `app/src/week.rs`: the four commands | `app/src/week.rs` (new), `app/src/lib.rs`, `app/src/commands.rs`, `app/src/main.rs`, `app/tests/week.rs` (new) | Q8 |
| Q10 | the moved line, the *Schedule* view, the window editor | `app/static/{index.html,console.js,console.css}`, `app/tests/static_assets.rs` | Q9 |
| Q11 | the confirm screen, the ask form, the deck, the behavioural check | `app/static/{index.html,console.js,console.css}`, `app/tests/static_assets.rs`, `scripts/wizard-check.py` | Q10 |
| Q12 | docs, the recount, full verification | `docs/surface/anatomy.md`, `app/README.md`, `CLAUDE.md` | Q11 |

Strictly sequential: `commitments.rs` is edited by Q1–Q8, `cli.rs` by Q1, Q2, Q4, Q6, Q7,
`main.rs` (engine) by Q2 and Q4, `app/src/main.rs` by Q8 and Q9, the static files by Q10 and Q11.
No two tasks run in parallel. Each task ends with a green run of its own filter and one commit.

## Plan rulings (collected; each is repeated in its task)

- **Q1-a** first day = the vault-local date of the earliest journal record (revised after review
  finding 1; file-name date only as a fallback).
- **Q1-b** the gate lives in `cli::commitment_passes`, not in `emit_checks`.
- **Q1-c** the `rank` test vaults are seeded at noon UTC on 2026-09-06.
- **Q2-a** the engine formats the `when` label.
- **Q2-b** `--json` lists proposals in §5.2's card order.
- **Q2-c** which courses count, and how a title-only class note covers one.
- **Q3-a** `create_confirmed_as` / `create_marker_as` write under the given context.
- **Q3-b** a `mine` row with a bad level is skipped with a warning.
- **Q3-c** `declined` counts markers written.
- **Q4-a** `--actor` beside `--via`.
- **Q4-b** a window naming no day is invalid.
- **Q4-c** the window line is the input re-emitted by `write::to_literal`.
- **Q4-d** the write order is `mine`, `not_mine`, window.
- **Q5-a** `Overview` holds JSON rows.
- **Q6-a** `emit_asks` also takes the proposals.
- **Q6-b** asks are skipped when the series file could not be read.
- **Q6-c** the ask card's keys.
- **Q7-a** `AskSettled::Returned` leaves the card pending.
- **Q7-b** the class note's title is the course note's title.
- **Q7-c** `withdraw_asks` runs after the proposals.
- **Q8-a** `check_console_edit` is the engine's; the drawer's list is unchanged.
- **Q8-b** `CONSOLE_KINDS` excludes `planning-day`.
- **Q8-c** `answer_card` turns this card's settlement warning into `ok: false`.
- **Q9-a** `vault_io` is held around the confirm child.
- **Q9-b** the app passes `--today`.
- **Q9-c** `preview_window` returns the whole previewed state.
- **Q9-d** a failed proposals spawn is an empty list with a reason.
- **Q10-a** `stateView()`.
- **Q10-b** the nav label is "Schedule".
- **Q11-a** the screen's editor has no preview.
- **Q11-b** the deck answers an ask in Decisions.
- **Q11-c** a second press unanswers a row.
- **Q11-d** the screen is checked once per console launch.

---

## Q1 — the vault's day and the day-1 gate (D2)

**Files.**
- `engine/src/ledger.rs`: `impl JsonlLedger` (lines 76–250; `day_files` at 232–250) gains
  `first_day` and `first_ts`; the `mod tests` block (ends at line 829) gains two tests.
- `engine/src/commitments.rs`: a new `vault_day` after `days_since` (lines 1393–1397); a new
  `#[cfg(test)] mod phase2_tests` appended at the end of the file (after line 9106).
- `engine/src/cli.rs`: `commitment_passes` (lines 802–848, the `proposals`/`emit_checks` lines at
  842–845); tests `p16_vault` (2573–2587) and `commitment_note` (2641–2660).

**Interfaces.**
- Produces `pub fn first_day(&self) -> Option<jiff::civil::Date>` (the earliest day file's name)
  and `pub fn first_ts(&self) -> Option<String>` (the smallest `ts` in that file) on
  `ledger::JsonlLedger`.
- Produces `pub fn vault_day(vault: &Path, today: Date) -> i64` in `commitments`: 1 on the first day.
- Consumes `cli::vault_zone(&Path) -> TimeZone` (`cli.rs:117`, `pub`) and
  `commitments::{proposals, emit_checks}`, unchanged.

**Plan ruling Q1-a (revised after the plan review, finding 1; spec D2 as amended):** the first day
is the **vault-local date of the earliest journal record**: the smallest `ts` in the earliest day
file, as a `jiff::Timestamp`, converted with `cli::vault_zone(vault)`. If no `ts` parses, it falls
back to the file-name date. Day 1 is `today == first`; `.max(1)` keeps a pinned `--today` before
it on day 1. *Why:* file names are UTC dates. A west-of-UTC evening onboarding got one extra quiet
day (harmless), but an east-of-UTC onboarding after local midnight got a first file dated local
yesterday. That made the student's first real day day 2, so the screen never opened and the cards
were filed (D1, D2 defeated). The local date is right in both directions. `first_ts` lives in
`ledger.rs`, so the `.jsonl` seam holds.
**Plan ruling Q1-b:** the gate is in `cli::commitment_passes`: it passes `&[]` as the proposals on
day 1. It is not inside `emit_checks`. *Why:* change cards still pass through, as D2 says. The
thirty-odd direct `emit_checks` tests in `commitments.rs` and `approvals.rs` stay valid.
**Plan ruling Q1-c:** `p16_vault` and `commitment_note` stamp their seed journal record at
**2026-09-06T12:00:00.000Z**, noon UTC the day before `P16_MONDAY`. `commitment_note`'s record
moves there from `2026-09-01T00:00:00.000Z`. *Why:* `seed_migrated` writes a record at the real
clock (2026-09-24 or later). Without an older record, every P16 rank on 2026-09-07 would be day 1
and file nothing. Noon UTC on 09-06 is 07:00 CDT on 09-06 in the P16 vaults' `America/Chicago`, so
`P16_MONDAY` is day 2: checks are filed, and Q6's asks, which start on day 3, are not. Midnight UTC
would be 19:00 CDT on 09-05 and make `P16_MONDAY` day 3 (review finding 1). Noon UTC is the same
date in every zone from UTC−11 to UTC+11, so the tests hold on any development machine.

**The `.jsonl` seam.** `ledger::tests::only_this_module_opens_ledger_files`
(`engine/src/ledger.rs:806`) fails any other module whose code names `.jsonl`. So `first_day` and
`first_ts` live in `ledger.rs`, and every test elsewhere seeds a journal day by appending a record
with a pinned `ts`. It never writes a file name.

- [ ] **Step 1 — failing tests.** In `engine/src/ledger.rs`, inside `mod tests`, before its closing
  brace (line 829):

```rust
    #[test]
    fn first_day_is_the_earliest_day_file_by_name() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        assert_eq!(JsonlLedger::new(&root).first_day(), None, "no folder is no day");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("2026-09-25.jsonl"), "").unwrap();
        fs::write(root.join("2026-09-24.jsonl"), "").unwrap();
        fs::write(root.join("notes.jsonl"), "").unwrap();
        fs::write(root.join("2026-09-01.txt"), "").unwrap();
        assert_eq!(JsonlLedger::new(&root).first_day(), Some(jiff::civil::date(2026, 9, 24)));
    }

    #[test]
    fn first_ts_is_the_smallest_ts_in_the_earliest_day_file() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        assert_eq!(JsonlLedger::new(&root).first_ts(), None);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("2026-09-23.jsonl"),
            "{\"ts\": \"2026-09-23T23:10:00.000Z\"}\n{\"ts\": \"2026-09-23T22:30:00.000Z\"}\nnot json\n",
        )
        .unwrap();
        fs::write(root.join("2026-09-24.jsonl"), "{\"ts\": \"2026-09-24T01:00:00.000Z\"}\n").unwrap();
        assert_eq!(JsonlLedger::new(&root).first_ts().as_deref(), Some("2026-09-23T22:30:00.000Z"));
    }
```

  At the end of `engine/src/commitments.rs`:

```rust
#[cfg(test)]
mod phase2_tests {
    //! Phase 2 (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md`). Every title,
    //! key, room and course is invented.

    use super::*;
    use crate::journal::{make_record, Journal, NewRecord};
    use jiff::civil::date;

    /// Tuesday 2026-09-01; `mk`'s series start on Monday 2026-08-31.
    const TODAY: Date = Date::constant(2026, 9, 1);

    /// A scratch vault removed when the test ends, pass or fail.
    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = PathBuf;
        fn deref(&self) -> &PathBuf {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scratch(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("knowlu-p2-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        Scratch(dir)
    }

    /// One journal record stamped `ts` (UTC, `…Z`).
    fn journal_at(vault: &Path, ts: &str) {
        let mut spec = NewRecord::new("create", "archive/_seed.md", "system:migration", "cli");
        spec.ts = Some(ts.to_string());
        let mut rec = make_record(spec).unwrap();
        Journal::new(vault).append(&mut rec).unwrap();
    }

    /// One journal record stamped noon UTC on `day`: that date in every zone from UTC−11 to UTC+11,
    /// so the vault's first local day is `day` on any development machine.
    fn journal_on(vault: &Path, day: &str) {
        journal_at(vault, &format!("{day}T12:00:00.000Z"));
    }

    #[test]
    fn a_vault_with_no_journal_is_on_day_one() {
        let v = scratch("noday");
        assert_eq!(vault_day(&v, TODAY), 1);
    }

    #[test]
    fn the_day_counts_from_the_earliest_journal_record() {
        let v = scratch("days");
        journal_on(&v, "2026-09-24");
        journal_on(&v, "2026-09-26");
        assert_eq!(vault_day(&v, date(2026, 9, 23)), 1, "a pinned --today before the first record is day 1");
        assert_eq!(vault_day(&v, date(2026, 9, 24)), 1);
        assert_eq!(vault_day(&v, date(2026, 9, 25)), 2);
        assert_eq!(vault_day(&v, date(2026, 9, 26)), 3);
    }

    /// Review finding 1: east of UTC, a wizard finished at 00:30 local on Sep 24 journals at
    /// 22:30 UTC on Sep 23. The first day is the vault-local date, Sep 24, not the file's.
    #[test]
    fn the_first_day_is_the_vault_local_date_of_the_earliest_record() {
        let v = scratch("athens");
        std::fs::write(v.join("config").join("ingest.yaml"), "timezone: Europe/Athens\n").unwrap();
        journal_at(&v, "2026-09-23T22:30:00.000Z");
        assert_eq!(vault_day(&v, date(2026, 9, 24)), 1);
        assert_eq!(vault_day(&v, date(2026, 9, 25)), 2);
    }
}
```

  In `engine/src/cli.rs`'s `mod tests`, right after `p16_vault` (line 2587), add the seed helper,
  and call it from `p16_vault`. The body of `p16_vault` becomes the same lines plus one call after
  `seed_migrated(&vault);`:

```rust
    /// One journal record stamped noon UTC on `day` (Q1-c): the vault's first day for
    /// `commitments::vault_day`. A pinned-`ts` `create` record, the shape `commitment_note` already
    /// appends; `verify_tail` only replays `set` records, so it is inert.
    fn seed_journal_day(vault: &Path, day: &str) {
        let mut spec = crate::journal::NewRecord::new("create", "archive/_migrated.md", "system:migration", "cli");
        spec.ts = Some(format!("{day}T12:00:00.000Z"));
        let mut rec = crate::journal::make_record(spec).unwrap();
        Journal::new(vault).append(&mut rec).unwrap();
    }
```

```rust
        seed_migrated(&vault);
        // Q1-c: P16_MONDAY is the vault's day 2, so checks are filed and asks (day 3+) are not.
        seed_journal_day(&vault, "2026-09-06");
        vault
```

  In `commitment_note` (line 2657), change `spec.ts = Some("2026-09-01T00:00:00.000Z".into());` to
  `spec.ts = Some("2026-09-06T12:00:00.000Z".into());` (Q1-c: still before any rank, the same
  local day as the seed, and noon so that `America/Chicago` reads it as 09-06, not 09-05).

  Then the gate's test, after `rank_files_a_commitment_check_for_an_injected_class_series_and_counts_it_pending`
  (it ends at line 2710):

```rust
    /// Phase-2 D2: on the vault's first day `rank` files no proposal card, because the confirm
    /// screen is asking at the same moment. From day 2 it files them as before.
    #[test]
    fn no_commitment_check_is_filed_on_the_vaults_first_day() {
        let vault = p16_vault("p2day1");
        // `p16_vault`'s journal starts 2026-09-06 (Q1-c): that date is day 1.
        rank_p16(&vault, Date::constant(2026, 9, 6), vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        assert!(checks(&vault, "approvals").is_empty(), "{:?}", checks(&vault, "approvals"));
        assert!(vault.join("state").join("calendar-series.json").is_file(), "the series file still refreshes");
        rank_p16(&vault, P16_MONDAY, vec![("cloud:google", google_entry(vec![cs100_item()]))]);
        assert_eq!(checks(&vault, "approvals").len(), 1, "day 2 files the class card");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu-engine --lib -j 2 -- ledger::tests::first_ commitments::phase2_tests cli::tests::no_commitment_check`
  Expected: compile errors: no `first_day`/`first_ts` method and no `vault_day` function.

- [ ] **Step 3 — implement.** In `engine/src/ledger.rs`, inside `impl JsonlLedger`, after
  `day_files` (line 250):

```rust
    /// The UTC date named by the earliest day file, or `None` when there is none. A file whose
    /// stem is not a date is skipped. `commitments::vault_day`'s fallback when no `ts` parses.
    pub fn first_day(&self) -> Option<jiff::civil::Date> {
        self.day_files()
            .iter()
            .filter_map(|p| p.file_stem()?.to_str()?.parse::<jiff::civil::Date>().ok())
            .min()
    }

    /// The smallest `ts` among the records of the earliest day file, or `None` when there is no
    /// file or no record in it carries a `ts`. A line that is not JSON is skipped. The earliest
    /// record of the whole ledger is in that file, because a file is named by its records' UTC
    /// date. `commitments::vault_day` counts the vault's days from this record's local date
    /// (phase-2 spec D2 as amended). It lives here because only this module names ledger files.
    pub fn first_ts(&self) -> Option<String> {
        let first = self.day_files().into_iter().next()?;
        let text = fs::read_to_string(first).ok()?;
        text.lines()
            .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
            .filter_map(|v| v.get("ts").and_then(Value::as_str).map(str::to_string))
            .min()
    }
```

  In `engine/src/commitments.rs`, after `days_since` (line 1397):

```rust
/// The vault's day number on `today` (phase-2 spec D2 as amended, parent §5.3's day 3). Day 1 is
/// the **vault-local date of the earliest journal record** (`cli::vault_zone`); the next date is
/// day 2, and so on. A vault with no journal is on day 1, and so is a pinned `today` before that
/// date. When no record's `ts` parses, the earliest file's name (a UTC date) stands in
/// (Plan ruling Q1-a).
pub fn vault_day(vault: &Path, today: Date) -> i64 {
    let ledger = crate::journal::Journal::new(vault).ledger;
    let first = ledger
        .first_ts()
        .and_then(|ts| ts.parse::<jiff::Timestamp>().ok())
        .map(|ts| ts.to_zoned(crate::cli::vault_zone(vault)).date())
        .or_else(|| ledger.first_day());
    match first {
        None => 1,
        Some(first) => (days_since(first, today) + 1).max(1),
    }
}
```

  In `engine/src/cli.rs`'s `commitment_passes`, replace lines 842–845:

```rust
    let proposals = cm::proposals(&file, &set, &codes, &names, &template, &held, today, true);
    let budget = std::cmp::max(0, planning.daily_approval_budget - count_proposals_created(vault, today));
    let (_, filed, card_warnings) = cm::emit_checks(vault, &proposals, &changes, today, budget, ctx, journal);
```

  with:

```rust
    let proposals = cm::proposals(&file, &set, &codes, &names, &template, &held, today, true);
    // Phase-2 spec D2 (Plan ruling Q1-b): on the vault's first day the confirm screen asks, so no
    // proposal or window card is filed. Change cards cannot exist yet, and pass through unchanged.
    let asked: &[cm::Proposal] = if cm::vault_day(vault, today) == 1 { &[] } else { &proposals };
    let budget = std::cmp::max(0, planning.daily_approval_budget - count_proposals_created(vault, today));
    let (_, filed, card_warnings) = cm::emit_checks(vault, asked, &changes, today, budget, ctx, journal);
```

- [ ] **Step 4 — run.** The same filter, then the whole engine lib and the two oracles:
  `cargo test -p knowlu-engine --lib -j 2 -- ledger::tests::first_ commitments::phase2_tests cli::tests::no_commitment_check`
  (2 + 3 + 1 pass); `cargo test -p knowlu-engine --lib -j 2` (all pass: the P16 tests now rank on
  day 2); `cargo test -p knowlu-engine --test oracle --test surface_oracle -j 2`;
  `git diff --exit-code -- engine/tests/fixtures`.

- [ ] **Step 5 — commit.** `git add engine/src/ledger.rs engine/src/commitments.rs engine/src/cli.rs`;
  message:

```
feat(engine): no commitment card on the vault's first day (phase 2, D2)

JsonlLedger::first_ts gives the earliest journal record; commitments::vault_day
counts from its vault-local date (spec D2 as amended), falling back to the
file-name date. rank's card pass passes no proposals on day 1, so the confirm
screen asks alone. The P16 test vaults seed their journal at noon UTC on
2026-09-06 so P16_MONDAY is day 2.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q2 — `when`, card order and `uncovered_courses` in `commitments --json`

**Files.**
- `engine/src/commitments.rs`: `proposal_order` (lines 2810–2833) gains a public comparator; a new
  `proposal_value` after `proposal_commitment` (lines 2619–2637); new `UncoveredCourse`,
  `active_courses` and `uncovered_courses` after `Codes` (lines 830–843); tests in `phase2_tests`.
- `engine/src/cli.rs`: `CommitmentsReport` (867–870), `proposal_json` (882–901),
  `commitments_report_with` (944–1023); one test after the P19 tests (from line 3121).
- `engine/src/main.rs`: the `commitments` arm's JSON (lines 362–367).

**Interfaces.**
- Produces `pub fn proposal_value(p: &Proposal) -> serde_json::Value`, with keys
  `{kind, level, title, course, meets:[{days,start,end}], when, where, from, until, source_uid,
  window}`. `cli::proposal_json` now returns it.
- Produces `pub fn card_order(a: &Proposal, b: &Proposal) -> std::cmp::Ordering`.
- Produces `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] pub struct UncoveredCourse
  { pub slug: String, pub title: String }` and
  `pub fn uncovered_courses(vault: &Path, set: &Commitments, proposals: &[Proposal], codes: &Codes) -> Vec<UncoveredCourse>`.
- `CommitmentsReport` gains `pub uncovered: Vec<crate::commitments::UncoveredCourse>`.
- `commitments --json` prints `{"proposals": [...], "uncovered_courses": [{"slug","title"}], "warnings": [...]}`,
  with the proposals in card order.

**Plan ruling Q2-a:** each proposal row carries `when`, the §5.2 card label
(`Mon/Wed/Fri 12–12:50pm`, from the private `meets_label`), or `null` when there are no meetings.
*Why:* the screen then formats no time itself, and the card and the row read the same.
**Plan ruling Q2-b:** `--json` lists proposals in §5.2's card order (`proposal_order`), not in
`source_uid` order. *Why:* spec §2 sorts rows "as the cards are", and the page must not sort.
`CommitmentsReport.proposals` keeps `source_uid` order, which the P19 tests pin. No frozen fixture
pins the JSON.
**Plan ruling Q2-c:** a course counts when its note's `status:` is absent or `active`. The wizard
writes `active`; an archived course is never asked about. A confirmed `class` note with no
`course:` covers the course whose code leads its title (`to_code(title)`). *Why:* such a note is
the parent's §3.5 title-signature case, and without this an ask card would be filed for a class
the student already wrote down.

**Watch:** `to_code` (line 671) carries `#[cfg_attr(not(test), expect(dead_code))]`. Calling it
from non-test code would leave that expectation unfulfilled, and the build would print a warning.
Call `to_code_exempt(text, &BTreeSet::new())` instead, and leave the attribute alone.

- [ ] **Step 1 — failing tests.** Add to `mod phase2_tests` (after the Q1 tests):

```rust
    fn course(vault: &Path, slug: &str, front: &str) {
        let dir = vault.join("courses");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{slug}.md")), format!("---\n{front}---\n\nInvented.\n")).unwrap();
    }

    fn t(h: i8, m: i8) -> Time {
        Time::new(h, m, 0, 0).unwrap()
    }

    fn meet(days: &[DayKey], s: Time, e: Time) -> Meet {
        Meet { days: days.to_vec(), start: s, end: e }
    }

    fn note_of(kind: &str, title: &str, course: Option<&str>) -> Commitment {
        Commitment {
            id: "cmt_00000000c1".into(),
            path: PathBuf::from("commitments/x.md"),
            kind: kind.into(),
            level: default_level(kind).unwrap_or(Level::Soft),
            title: title.into(),
            course: course.map(String::from),
            meets: vec![meet(&["tue"], t(9, 0), t(10, 0))],
            where_: None,
            from: None,
            until: None,
            source_uid: None,
        }
    }

    fn proposal_of(kind: &str, title: &str, course: Option<&str>, uid: &str) -> Proposal {
        Proposal {
            kind: kind.into(),
            level: default_level(kind).unwrap_or(Level::Soft),
            title: title.into(),
            course: course.map(String::from),
            meets: vec![meet(&["mon", "wed", "fri"], t(12, 0), t(12, 50))],
            where_: None,
            from: None,
            until: None,
            source_uid: uid.into(),
        }
    }

    /// `ant-101` (title `ANT 101`), `bui-100` (`BUI 100`) and `cs-100` (`CS 100`), all active.
    fn three_courses(name: &str) -> Scratch {
        let v = scratch(name);
        course(&v, "ant-101", "title: \"ANT 101\"\nslug: ant-101\ncode: \"ANT 101\"\nstatus: active\n");
        course(&v, "bui-100", "title: \"BUI 100\"\nslug: bui-100\ncode: \"BUI 100\"\nstatus: active\n");
        course(&v, "cs-100", "title: \"CS 100\"\nslug: cs-100\ncode: \"CS 100\"\nstatus: active\n");
        v
    }

    fn slugs(got: &[UncoveredCourse]) -> Vec<&str> {
        got.iter().map(|c| c.slug.as_str()).collect()
    }

    #[test]
    fn every_active_course_with_no_class_is_uncovered_in_slug_order() {
        let v = three_courses("uncovered");
        let (codes, _) = Codes::load(&v);
        let got = uncovered_courses(&v, &Commitments::default(), &[], &codes);
        assert_eq!(slugs(&got), ["ant-101", "bui-100", "cs-100"]);
        assert_eq!(got[1], UncoveredCourse { slug: "bui-100".into(), title: "BUI 100".into() });
    }

    #[test]
    fn a_confirmed_class_or_a_class_proposal_covers_its_course_by_slug_or_code() {
        let v = three_courses("covered");
        let (codes, _) = Codes::load(&v);
        let set = Commitments {
            confirmed: vec![note_of("class", "BUI 100", Some("BUI 100")), note_of("class", "ANT 101 Lecture", None)],
            ..Commitments::default()
        };
        let proposals = [proposal_of("class", "CS 100", Some("cs-100"), "gcal-series:cs100")];
        assert!(uncovered_courses(&v, &set, &proposals, &codes).is_empty());
    }

    #[test]
    fn a_lab_or_office_hours_does_not_cover_and_an_inactive_course_is_left_out() {
        let v = three_courses("labonly");
        course(&v, "old-200", "title: \"OLD 200\"\nslug: old-200\nstatus: archived\n");
        let (codes, _) = Codes::load(&v);
        let set = Commitments { confirmed: vec![note_of("lab", "CS 100 Lab", Some("cs-100"))], ..Commitments::default() };
        let proposals = [proposal_of("office-hours", "CS 100 Office Hours", Some("cs-100"), "gcal-series:oh")];
        assert_eq!(slugs(&uncovered_courses(&v, &set, &proposals, &codes)), ["ant-101", "bui-100", "cs-100"]);
    }

    #[test]
    fn a_proposal_value_carries_the_card_label_and_the_window_flag() {
        let p = proposal_of("class", "CS 100", Some("cs-100"), "gcal-series:cs100");
        let v = proposal_value(&p);
        assert_eq!(v["when"], serde_json::json!("Mon/Wed/Fri 12–12:50pm"));
        assert_eq!(v["meets"], serde_json::json!([{"days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50"}]));
        assert_eq!(v["window"], serde_json::json!(false));
        assert_eq!(v["source_uid"], serde_json::json!("gcal-series:cs100"));
        let mut club = proposal_of("club", "Chess Club", None, "gcal-series:a-club");
        club.meets = Vec::new();
        assert_eq!(proposal_value(&club)["when"], serde_json::Value::Null);
        let mut ps = vec![club, p];
        ps.sort_by(card_order);
        assert_eq!(ps[0].kind, "class", "classes first, as the cards are");
    }
```

  In `engine/src/cli.rs`'s tests, after `commitments_command_includes_office_hours_because_for_cards_is_false`:

```rust
    /// Phase-2 spec §2: the command names each course with no class row. `p16_vault` has
    /// `courses/cs-100.md`: with no feed it is uncovered; once CS 100's series is proposed, it is not.
    #[test]
    fn commitments_command_reports_courses_with_no_class_row() {
        let vault = p16_vault("p2uncovered");
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), Fetchers::default());
        assert_eq!(report.uncovered.len(), 1, "{:?}", report.uncovered);
        assert_eq!(report.uncovered[0].slug, "cs-100");
        assert_eq!(report.uncovered[0].title, "CS 100 Intro to Computing");
        let stash: SeriesStash = RefCell::new([("cloud:google".to_string(), google_entry(vec![cs100_item()]))].into_iter().collect());
        let empty = |_: &str| Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string());
        let fetchers = Fetchers { calendar: Some(&empty), events: None, series: Some(&stash) };
        let report = commitments_report_with(&vault, Some(&P16_MONDAY.to_string()), fetchers);
        assert!(report.uncovered.is_empty(), "{:?}", report.uncovered);
        let _ = std::fs::remove_dir_all(&vault);
    }
```

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests cli::tests::commitments_command_reports`
  Expected: compile errors (`uncovered_courses`, `UncoveredCourse`, `proposal_value`, `card_order`,
  and `report.uncovered` do not exist).

- [ ] **Step 3 — implement.** In `engine/src/commitments.rs`, after `impl Codes` (line 843):

```rust
/// A course with neither a class proposal nor a confirmed class note (phase-2 spec §2): the
/// confirm screen lists it, and from the vault's day 3 a `commitment-ask` card asks when it meets.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UncoveredCourse {
    pub slug: String,
    pub title: String,
}

/// Every active course note (`status:` absent or `active`, Plan ruling Q2-c), as `(slug, title)`
/// in slug order. The slug is read as `code_table` reads it: the `slug:` field, else the file
/// stem. The title is the `title:` field, else `name:`, else the slug.
fn active_courses(vault: &Path) -> Vec<(String, String)> {
    let dir = vault.join("courses");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out: Vec<(String, String)> = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_string) else { continue };
        if !path.is_file() || !name.ends_with(".md") {
            continue;
        }
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = split_frontmatter(&text) else { continue };
        let status = field_text(&meta, "status").unwrap_or_default();
        if !status.trim().is_empty() && status.trim() != "active" {
            continue;
        }
        let stem = name.trim_end_matches(".md").to_string();
        let slug = field_text(&meta, "slug").filter(|s| !s.is_empty()).unwrap_or(stem);
        let title = field_text(&meta, "title")
            .or_else(|| field_text(&meta, "name"))
            .map(|t| single_line(&t).trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| slug.clone());
        out.push((slug, title));
    }
    out.sort();
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// Phase-2 spec §2 and parent §5.3: every active course (`active_courses`, slug order) that has no
/// class proposal in `proposals` and no confirmed `class` note. Courses are compared the way the
/// §3.5 signature compares them, by slug or code (`course_key`). A class note with no `course:`
/// covers the course whose code leads its title (Plan ruling Q2-c). A lab or office hours covers
/// nothing.
pub fn uncovered_courses(vault: &Path, set: &Commitments, proposals: &[Proposal], codes: &Codes) -> Vec<UncoveredCourse> {
    let mut covered = covered_course_keys(set, codes);
    for p in proposals.iter().filter(|p| p.kind == "class") {
        if let Some(course) = p.course.as_deref() {
            covered.insert(course_key(course, codes));
        }
    }
    active_courses(vault)
        .into_iter()
        .filter(|(slug, _)| !covered.contains(&course_key(slug, codes)))
        .map(|(slug, title)| UncoveredCourse { slug, title })
        .collect()
}

/// Every course a confirmed `class` note covers, as `course_key`s: its `course:`, or, with none,
/// the code that leads its title (Plan ruling Q2-c). The one rule `uncovered_courses` and Q7's
/// ask settlement and withdrawal share (review finding 3).
fn covered_course_keys(set: &Commitments, codes: &Codes) -> BTreeSet<String> {
    let mut covered = BTreeSet::new();
    for note in set.confirmed.iter().filter(|n| n.kind == "class") {
        match note.course.as_deref() {
            Some(course) => {
                covered.insert(course_key(course, codes));
            }
            None => {
                if let Some(code) = to_code_exempt(&note.title, &BTreeSet::new()) {
                    covered.insert(course_key(&code, codes));
                }
            }
        }
    }
    covered
}
```

  After `proposal_commitment` (line 2637):

```rust
/// One proposal as the app reads it (phase-2 spec §2): the fields a card's `commitment:` carries,
/// plus `source_uid` (what the app answers with), `window` (true only for the window proposal),
/// and `when`, the §5.2 card label (Plan ruling Q2-a), `null` when there are no meetings.
pub fn proposal_value(p: &Proposal) -> serde_json::Value {
    serde_json::json!({
        "kind": p.kind,
        "level": p.level.as_str(),
        "title": p.title,
        "course": p.course,
        "meets": meets_json(&p.meets),
        "when": meets_label(&p.meets),
        "where": p.where_,
        "from": date_json(p.from),
        "until": date_json(p.until),
        "source_uid": p.source_uid,
        "window": p.is_window(),
    })
}
```

  After `proposal_order` (it ends at line 2833, before `emit_checks`' doc comment):

```rust
/// §5.2's card order as a comparator: the confirm screen's rows are sorted as the cards are
/// (phase-2 spec §2, Plan ruling Q2-b).
pub fn card_order(a: &Proposal, b: &Proposal) -> std::cmp::Ordering {
    proposal_order(a).cmp(&proposal_order(b))
}
```

  In `engine/src/cli.rs`, `CommitmentsReport` (lines 867–870) becomes:

```rust
pub struct CommitmentsReport {
    pub proposals: Vec<crate::commitments::Proposal>,
    /// Phase-2 spec §2: the courses with neither a class proposal nor a confirmed class note.
    pub uncovered: Vec<crate::commitments::UncoveredCourse>,
    pub warnings: Vec<String>,
}
```

  `proposal_json`'s body (lines 883–900) becomes one line. Its doc comment gains
  "Delegates to `commitments::proposal_value` (phase 2 adds `when`)":

```rust
pub fn proposal_json(p: &crate::commitments::Proposal) -> serde_json::Value {
    crate::commitments::proposal_value(p)
}
```

  At the end of `commitments_report_with` (line 1022), replace
  `CommitmentsReport { proposals, warnings }` with:

```rust
    let uncovered = cm::uncovered_courses(vault, &set, &proposals, &codes);
    CommitmentsReport { proposals, uncovered, warnings }
```

  `cli.rs`'s private `hm` stays in use: `proposal_line` still calls it.

  In `engine/src/main.rs`, the `if json { … }` block of the `commitments` arm (lines 362–367)
  becomes:

```rust
            if json {
                // Plan ruling Q2-b: the screen's rows in §5.2's card order.
                let mut ordered: Vec<&knowlu_engine::commitments::Proposal> = report.proposals.iter().collect();
                ordered.sort_by(|a, b| knowlu_engine::commitments::card_order(a, b));
                let value = serde_json::json!({
                    "proposals": ordered.into_iter().map(cli::proposal_json).collect::<Vec<_>>(),
                    "uncovered_courses": report.uncovered,
                    "warnings": report.warnings,
                });
                println!("{}", knowlu_engine::ledger::dumps_value(&value));
```

- [ ] **Step 4 — run.**
  `cargo test -p knowlu-engine --lib -j 2 -- commitments:: cli::tests::commitments_command cli::tests::proposal_json`
  (all pass, including the existing `proposal_json_carries_an_explicit_window_flag`), then
  `cargo test -p knowlu-engine --test cloud_contract -j 2` (the `commitments` arm scan still
  passes: it names no judge endpoint).

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs engine/src/cli.rs engine/src/main.rs`;
  message:

```
feat(engine): commitments --json names uncovered courses, in card order (phase 2, §2)

proposal_value adds `when` (the §5.2 card label) to every proposal row;
uncovered_courses lists active courses with no class proposal and no confirmed
class note; the command prints both, proposals in §5.2 card order.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q3 — `commitments::confirm`: `mine` and `not_mine` as the human (D4, §3)

**Files.**
- `engine/src/commitments.rs`: `create_marker` (lines 3322–3349) and `create_confirmed`
  (3428–3497) split into a public wrapper and a new `*_as` function; new `Stored`,
  `stored_proposals`, `ConfirmInput`, `ConfirmReport`, `parse_confirm` and `confirm` after
  `withdrawal_reason` (ends just before line 3906); tests in `phase2_tests`.

**Interfaces.**
- Produces `fn create_marker_as(vault: &Path, key: &str, ctx: &WriteContext, journal: &mut Journal) -> Result<Option<PathBuf>, WriteError>`
  and `fn create_confirmed_as(vault: &Path, commitment: &Mapping, source_uid: &str, today: Date, ctx: &WriteContext, journal: &mut Journal) -> Result<PathBuf, WriteError>`.
  The public `create_marker` and `create_confirmed` keep their signatures and pass
  `&ctx.with_actor(CARD_ACTOR)` to these.
- Produces `pub struct Stored { pub file: SeriesFile, pub set: Commitments, pub codes: Codes, pub template: crate::weekcal::WeekCalendar, pub proposals: Vec<Proposal>, pub unanswered: Vec<Proposal>, pub warnings: Vec<String> }`
  and `pub fn stored_proposals(vault: &Path, today: Date) -> Stored`.
- Produces `#[derive(Debug, Clone, Default, PartialEq)] pub struct ConfirmInput { pub mine: Vec<(String, String)>, pub not_mine: Vec<String>, pub window: Option<String> }`,
  `pub fn parse_confirm(text: &str) -> Result<ConfirmInput, String>`,
  `#[derive(Debug, Clone, Default, PartialEq)] pub struct ConfirmReport { pub created: usize, pub declined: usize, pub window: Option<&'static str>, pub warnings: Vec<String> }`
  with `pub fn to_json(&self) -> serde_json::Value`, and
  `pub fn confirm(vault: &Path, input: &ConfirmInput, today: Date, ctx: &crate::write::WriteContext, journal: &mut crate::journal::Journal) -> Result<ConfirmReport, String>`.
  `Err` means nothing was written, and the CLI exits 2 (Q4).
- The input JSON: `{"mine": [{"source_uid": "…", "level": "hard"}], "not_mine": ["…"], "window": "<flow sequence>"}`.
  Every key is optional.

**Plan ruling Q3-a:** `create_confirmed` and `create_marker` hard-code
`ctx.with_actor(CARD_ACTOR)`, which is right for a settlement and wrong for the screen. The spec
(§3, parent §2.5) makes the screen's writes the console's human context. So the bodies move into
`create_confirmed_as` / `create_marker_as`, which write under the context they are given, and the
public functions call them with `CARD_ACTOR`. Every existing caller is unchanged.
**Plan ruling Q3-b:** a `mine` row whose `level` is missing or is not `hard`, `soft` or `optional`
is skipped with the warning `<key>: level "<x>" is not hard, soft or optional; skipped`. *Why:* the
spec lets only `level` come from the app, and "must parse as a §2.2 level or the key is skipped".
**Plan ruling Q3-c:** `declined` counts the decline markers written, twins included. A key already
declined writes none and counts 0. *Why:* that is the observable effect, and it makes a second
Finish report `declined: 0`.

**How a key is found.** `stored_proposals` builds the proposals twice from the series file as it
stands (`read_series_file`; no refresh, no fetch):
- `proposals` uses the real notes. These are the current questions.
- `unanswered` uses an empty `Commitments`. It finds a key the notes have since answered, so a
  second Finish can be told apart from a stale key.

A `mine` key that a confirmed note holds (by `source_uid` or by signature) is skipped silently:
that is the second Finish. A key in neither list, or a window key, is warned and skipped. The
notes are re-read (`load`) before each row, so two rows of one input never write the same note.

- [ ] **Step 1 — failing tests.** Add to `mod phase2_tests`:

```rust
    /// Three weeks of a weekly series from Monday 2026-08-31 on `days`, `s`–`e`, last seen
    /// `TODAY`; `meets` as P8 derives it.
    fn mk(uid: &str, cal: &str, title: &str, days: &[DayKey], s: Time, e: Time) -> Series {
        let monday = date(2026, 8, 31);
        let mut instances = Vec::new();
        for week in 0..3 {
            for day in days {
                let offset = DAY_KEYS.iter().position(|k| k == day).unwrap() as i64;
                instances.push(Instance { date: add_days(monday, week * 7 + offset), start: Some(s), end: Some(e) });
            }
        }
        instances.sort_by_key(|i| i.date);
        Series {
            source_uid: uid.to_string(),
            calendar: cal.to_string(),
            title: title.to_string(),
            where_: None,
            event_type: None,
            rule: Rule { freq: "WEEKLY".into(), interval: 1, until: None, count: None },
            has_master: true,
            rdate: false,
            unsupported: false,
            meets: meets_of(&instances),
            instances,
            first: Some(monday),
            until: None,
            last_seen: Some(TODAY),
        }
    }

    fn write_series(vault: &Path, series: Vec<Series>) {
        let mut file = SeriesFile::default();
        for s in &series {
            file.calendars.insert(s.calendar.clone(), TODAY);
        }
        file.series = series;
        file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        std::fs::create_dir_all(vault.join("state")).unwrap();
        std::fs::write(vault.join(SERIES_FILE), file_bytes(&file)).unwrap();
    }

    // Review finding 5: this import arrives with its first use, not in Q1, so no commit carries
    // an unused-import warning. Put the `use` line at the top of `mod phase2_tests`, beside the
    // others.
    use crate::write::WriteContext;

    fn human() -> WriteContext {
        WriteContext::new("quinn", "dashboard")
    }

    /// `courses/cs-100.md` and a series file holding CS 100 (Mon/Wed/Fri 12–12:50pm), Chess Club
    /// (Wed 6–7pm) and CS 100 Office Hours (Thu 3–4pm), all on one Google calendar.
    fn week_vault(name: &str) -> Scratch {
        let v = scratch(name);
        course(&v, "cs-100", "title: \"CS 100\"\nslug: cs-100\ncode: \"CS 100\"\nstatus: active\n");
        write_series(
            &v,
            vec![
                mk("gcal-series:cs100", "google:abc", "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50)),
                mk("gcal-series:chess", "google:abc", "Chess Club", &["wed"], t(18, 0), t(19, 0)),
                mk("gcal-series:oh", "google:abc", "CS 100 Office Hours", &["thu"], t(15, 0), t(16, 0)),
            ],
        );
        v
    }

    fn files(vault: &Path, folder: &str) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(vault.join(folder))
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    fn journal_records(vault: &Path) -> Vec<crate::ledger::Record> {
        Journal::new(vault).read(None, None)
    }

    fn input(mine: &[(&str, &str)], not_mine: &[&str], window: Option<&str>) -> ConfirmInput {
        ConfirmInput {
            mine: mine.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect(),
            not_mine: not_mine.iter().map(|k| k.to_string()).collect(),
            window: window.map(String::from),
        }
    }

    fn run(vault: &Path, given: &ConfirmInput) -> ConfirmReport {
        confirm(vault, given, TODAY, &human(), &mut Journal::new(vault)).unwrap()
    }

    #[test]
    fn parse_confirm_reads_every_optional_key_and_refuses_what_is_not_json() {
        let got = parse_confirm(r#"{"mine": [{"source_uid": "gcal-series:a", "level": "soft"}], "not_mine": ["gcal-series:b"], "window": "[]"}"#).unwrap();
        assert_eq!(got, input(&[("gcal-series:a", "soft")], &["gcal-series:b"], Some("[]")));
        assert_eq!(parse_confirm("{}").unwrap(), ConfirmInput::default());
        assert!(parse_confirm("not json").is_err());
        assert!(parse_confirm(r#"{"mine": "gcal-series:a"}"#).is_err());
    }

    #[test]
    fn mine_writes_exactly_the_rows_given_at_the_given_level_as_the_human() {
        let v = week_vault("mine");
        let report = run(&v, &input(&[("gcal-series:cs100", "soft")], &[], None));
        assert_eq!((report.created, report.declined, report.window), (1, 0, None));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(files(&v, FOLDER), ["cs-100.md"], "the club and office hours were not given");
        let set = load(&v);
        assert_eq!(set.confirmed.len(), 1);
        assert_eq!(set.confirmed[0].level, Level::Soft);
        assert_eq!(set.confirmed[0].source_uid.as_deref(), Some("gcal-series:cs100"));
        let create = journal_records(&v).into_iter()
            .find(|r| r.get("op").and_then(|o| o.as_str()) == Some("create"))
            .expect("a create record");
        assert_eq!(create.get("actor").and_then(|a| a.as_str()), Some("quinn"));
        assert_eq!(create.get("via").and_then(|a| a.as_str()), Some("dashboard"));
        assert!(!v.join("approvals").exists(), "no card, so nothing is charged to the cap");
    }

    #[test]
    fn an_unknown_or_stale_key_is_warned_and_skipped() {
        let v = week_vault("stale");
        let report = run(&v, &input(&[("gcal-series:gone", "hard")], &["gcal-series:never"], None));
        assert_eq!((report.created, report.declined), (0, 0));
        assert_eq!(report.warnings.len(), 2, "{:?}", report.warnings);
        assert!(report.warnings[0].starts_with("gcal-series:gone: "), "{:?}", report.warnings);
        assert!(report.warnings[1].starts_with("gcal-series:never: "), "{:?}", report.warnings);
        assert!(files(&v, FOLDER).is_empty());
    }

    #[test]
    fn a_level_outside_the_three_is_skipped_with_a_warning() {
        let v = week_vault("badlevel");
        let report = run(&v, &input(&[("gcal-series:cs100", "urgent")], &[], None));
        assert_eq!(report.created, 0);
        assert_eq!(report.warnings, ["gcal-series:cs100: level \"urgent\" is not hard, soft or optional; skipped"]);
    }

    #[test]
    fn a_second_finish_is_a_no_op() {
        let v = week_vault("twice");
        let given = input(&[("gcal-series:cs100", "hard")], &["gcal-series:chess"], None);
        let first = run(&v, &given);
        assert_eq!((first.created, first.declined), (1, 1));
        let before: Vec<String> = files(&v, FOLDER);
        let records = journal_records(&v).len();
        let second = run(&v, &given);
        assert_eq!((second.created, second.declined), (0, 0));
        assert!(second.warnings.is_empty(), "{:?}", second.warnings);
        assert_eq!(files(&v, FOLDER), before);
        assert_eq!(journal_records(&v).len(), records, "nothing journaled either");
    }

    #[test]
    fn not_mine_declines_the_twin_group_together() {
        let v = week_vault("twins");
        // The same CS 100 seen through a direct ICS feed under its own key (an Outlook invite).
        let mut series = vec![
            mk("gcal-series:cs100", "google:abc", "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50)),
            mk("ics-series:cs100-outlook", "personal", "CS 100", &["mon", "wed", "fri"], t(12, 0), t(12, 50)),
        ];
        series.push(mk("gcal-series:chess", "google:abc", "Chess Club", &["wed"], t(18, 0), t(19, 0)));
        write_series(&v, series);
        let report = run(&v, &input(&[], &["gcal-series:cs100"], None));
        assert_eq!(report.declined, 2, "{report:?}");
        let set = load(&v);
        assert!(set.declined.contains("gcal-series:cs100") && set.declined.contains("ics-series:cs100-outlook"));
        assert!(!set.declined.contains("gcal-series:chess"));
    }
```

- [ ] **Step 2 — run and see them fail.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests`
  Expected: compile errors (`ConfirmInput`, `ConfirmReport`, `parse_confirm`, `confirm` do not
  exist).

- [ ] **Step 3 — implement.** Split the two writers. `create_marker` (lines 3322–3349) becomes:

```rust
pub fn create_marker(
    vault: &Path,
    key: &str,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Option<PathBuf>, crate::write::WriteError> {
    create_marker_as(vault, key, &ctx.with_actor(CARD_ACTOR), journal)
}

/// [`create_marker`] under the context it is given (Plan ruling Q3-a): the confirm screen's
/// markers are the student's own writes (phase-2 spec §3), a settlement's are `agent:commitments`.
fn create_marker_as(
    vault: &Path,
    key: &str,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Option<PathBuf>, crate::write::WriteError> {
    if load(vault).declined.contains(key) {
        return Ok(None);
    }
    let front = front_matter(&[
        ("id", Field::Scalar(Node::text(&crate::ids::new_id("cmt")))),
        ("type", Field::Scalar(Node::text("commitment"))),
        ("status", Field::Scalar(Node::text("declined"))),
        ("source_uid", Field::Scalar(Node::text(&single_line(key)))),
    ])
    .map_err(|e| io(format!("marker not written: {e}")))?;
    let rel = free_rel(vault, &marker_stem(key));
    let path = crate::write::create(vault, &rel, &format!("---\n{front}---\n"), ctx, journal, None)?;
    Ok(Some(path))
}
```

  In the same way, `create_confirmed` (lines 3428–3497) keeps its doc comment and signature, and its
  body becomes `create_confirmed_as(vault, commitment, source_uid, today, &ctx.with_actor(CARD_ACTOR), journal)`.
  A new `fn create_confirmed_as(vault: &Path, commitment: &Mapping, source_uid: &str, today: Date, ctx: &crate::write::WriteContext, journal: &mut crate::journal::Journal) -> Result<PathBuf, crate::write::WriteError>`
  holds the old body verbatim, with one change: the final `crate::write::create(...)` call passes
  `ctx` where it passed `&ctx.with_actor(CARD_ACTOR)`. Its doc comment:
  `/// [`create_confirmed`] under the context it is given (Plan ruling Q3-a).`

  After `withdrawal_reason` (it ends just before line 3906), add the confirm pass:

```rust
// ---------------------------------------------------------------------------------------------
// Phase 2 — `commitments --confirm` (phase-2 spec §3, D4).
// ---------------------------------------------------------------------------------------------

/// The proposals as the series file holds them now: [`read_series_file`], never
/// [`refresh_series`], so nothing is fetched. What `--confirm` and [`overview`] read. `proposals`
/// is the current set of questions over the real notes. `unanswered` is the same computation over
/// no notes, so a key the notes have since answered can still be found (a second Finish). Both are
/// built with `for_cards: false`, so office hours are included.
pub struct Stored {
    pub file: SeriesFile,
    pub set: Commitments,
    pub codes: Codes,
    pub template: crate::weekcal::WeekCalendar,
    pub proposals: Vec<Proposal>,
    pub unanswered: Vec<Proposal>,
    pub warnings: Vec<String>,
}

pub fn stored_proposals(vault: &Path, today: Date) -> Stored {
    let (file, mut warnings) = read_series_file(vault);
    let set = load(vault);
    warnings.extend(set.warnings.iter().cloned());
    let (codes, code_warnings) = Codes::load(vault);
    warnings.extend(code_warnings_hit(code_warnings, &file));
    let planning = crate::planning::load_planning(&vault.join("config").join("planning.yaml"));
    let names: Vec<String> = planning.recurring.iter().map(|r| r.name.clone()).collect();
    let template = crate::weekcal::WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), Vec::new());
    let held = successor_keys(vault);
    let current = proposals(&file, &set, &codes, &names, &template, &held, today, false);
    let unanswered = proposals(&file, &Commitments::default(), &codes, &names, &template, &held, today, false);
    Stored { file, set, codes, template, proposals: current, unanswered, warnings }
}

/// The screen's answers (phase-2 spec §3): keys, levels and a window, never event data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfirmInput {
    /// `(source_uid, level)`, in input order.
    pub mine: Vec<(String, String)>,
    pub not_mine: Vec<String>,
    /// A flow sequence shaped like the planning-day note's `window`.
    pub window: Option<String>,
}

/// What `--confirm` did: `{"created", "declined", "window", "warnings"}`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfirmReport {
    pub created: usize,
    /// Decline markers written, twins included (Plan ruling Q3-c).
    pub declined: usize,
    /// `created`, `updated` or `unchanged`; `None` when no window was given.
    pub window: Option<&'static str>,
    pub warnings: Vec<String>,
}

impl ConfirmReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "created": self.created,
            "declined": self.declined,
            "window": self.window,
            "warnings": self.warnings,
        })
    }
}

/// The `--confirm` file (phase-2 spec §3). Every key is optional; `null` is absent. `Err` is
/// unreadable input, and the CLI exits 2.
pub fn parse_confirm(text: &str) -> Result<ConfirmInput, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("--confirm is not JSON ({e})"))?;
    let obj = value.as_object().ok_or("--confirm is not a JSON object")?;
    let mut input = ConfirmInput::default();
    if let Some(rows) = obj.get("mine").filter(|v| !v.is_null()) {
        for row in rows.as_array().ok_or("mine is not a list")? {
            let key = row.get("source_uid").and_then(|v| v.as_str()).ok_or("a mine row has no source_uid")?;
            let level = row.get("level").and_then(|v| v.as_str()).unwrap_or("");
            input.mine.push((key.to_string(), level.to_string()));
        }
    }
    if let Some(keys) = obj.get("not_mine").filter(|v| !v.is_null()) {
        for key in keys.as_array().ok_or("not_mine is not a list")? {
            input.not_mine.push(key.as_str().ok_or("a not_mine key is not a string")?.to_string());
        }
    }
    if let Some(window) = obj.get("window").filter(|v| !v.is_null()) {
        input.window = Some(window.as_str().ok_or("window is not a string")?.to_string());
    }
    Ok(input)
}

/// The two signatures that close a proposal's question, as [`proposals`] closes one.
fn proposal_signatures(p: &Proposal, codes: &Codes) -> Vec<Signature> {
    vec![p.signature(codes), signature(&p.kind, None, &p.title, &p.meets, codes)]
}
```

  And `confirm` itself. In this task it handles the rows only; Q4 replaces it whole with the
  version that also writes the window:

```rust
/// `commitments --confirm` (phase-2 spec §3): the screen's answers, written under `ctx` (the
/// console's human context; parent §2.5). Every row is re-derived from the series file
/// ([`stored_proposals`]). Nothing from the app becomes a field value but `level`.
///
/// - **`mine`** → [`create_confirmed_as`] at the chosen level. A key a confirmed note already holds,
///   by `source_uid` or by signature, is skipped silently: a second Finish is a no-op.
/// - **`not_mine`** → [`create_marker_as`] for the key and every twin ([`twin_keys`]), as a card
///   rejection does. A key a confirmed note holds gets no marker. A key already declined is skipped
///   silently.
/// - A key that is no current proposal, or is the window proposal's, is skipped with a warning. A
///   write that fails is reported, and the rest still run.
///
/// No card is filed, so nothing is charged to the cap.
pub fn confirm(
    vault: &Path,
    input: &ConfirmInput,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<ConfirmReport, String> {
    let stored = stored_proposals(vault, today);
    let codes = &stored.codes;
    let mut report = ConfirmReport::default();
    let stale = |key: &str| format!("{key}: not a current proposal; skipped");

    for (key, level) in &input.mine {
        let set = load(vault);
        if set.confirmed.iter().any(|n| n.source_uid.as_deref() == Some(key.as_str())) {
            continue;
        }
        let Some(p) = stored.unanswered.iter().find(|p| &p.source_uid == key && !p.is_window()) else {
            report.warnings.push(stale(key));
            continue;
        };
        let sigs = proposal_signatures(p, codes);
        if set.confirmed.iter().any(|n| sigs.contains(&n.signature(codes))) {
            continue;
        }
        if set.declined.contains(key) || !stored.proposals.iter().any(|q| &q.source_uid == key) {
            report.warnings.push(stale(key));
            continue;
        }
        let Some(level) = Level::parse(level) else {
            report.warnings.push(format!("{key}: level {level:?} is not hard, soft or optional; skipped"));
            continue;
        };
        let chosen = Proposal { level, ..p.clone() };
        let Value::Mapping(map) = to_value(proposal_commitment(&chosen)) else {
            report.warnings.push(format!("{key}: not written (no commitment mapping)"));
            continue;
        };
        match create_confirmed_as(vault, &map, key, today, ctx, journal) {
            Ok(_) => report.created += 1,
            Err(e) => report.warnings.push(format!("{key}: not written ({e})")),
        }
    }

    for key in &input.not_mine {
        let set = load(vault);
        if set.declined.contains(key) {
            continue;
        }
        let Some(p) = stored.proposals.iter().find(|p| &p.source_uid == key && !p.is_window()) else {
            report.warnings.push(stale(key));
            continue;
        };
        let held: BTreeSet<String> = set.confirmed.iter().filter_map(|n| n.source_uid.clone()).collect();
        for twin in twin_keys(vault, key, proposal_signatures(p, codes), codes) {
            if held.contains(&twin) {
                continue;
            }
            match create_marker_as(vault, &twin, ctx, journal) {
                Ok(Some(_)) => report.declined += 1,
                Ok(None) => {}
                Err(e) => report.warnings.push(format!("{twin}: marker not written ({e})")),
            }
        }
    }
    Ok(report)
}
```

  `WriteError` implements `Display` (`write.rs:33`), so `format!("… ({e})")` compiles.

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::` (the new tests
  pass; so do the existing `card_tests` and `change_tests`, and `approvals::` still settles through
  the unchanged public writers). Then `cargo test -p knowlu-engine --lib -j 2 -- approvals::`.

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs`; message:

```
feat(engine): commitments::confirm writes the screen's rows as the student (phase 2, §3)

create_confirmed/create_marker keep agent:commitments; their bodies move to
*_as functions that write under the context given. stored_proposals reads the
series file without a fetch; confirm creates each mine row at its level,
declines each not_mine row with its twins, warns on a stale key and is a no-op
the second time.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q4 — the window, the `--confirm` flag and the no-network pin (D4, D5, §3)

**Files.**
- `engine/src/commitments.rs`: `confirm` (added in Q3) is replaced whole; tests in `phase2_tests`.
- `engine/src/cli.rs`: a new `pub fn commitments_confirm` after `commitments_report_with` (ends at
  line 1023 before Q2; find it by name).
- `engine/src/main.rs`: `Command::Commitments` (lines 63–75) gains three arguments; the arm inside
  the `// commitments command: begin`/`end` markers (lines 359–378) gains the confirm branch.
- `engine/tests/commitments_confirm.rs` (new): the binary's exit codes and the no-network pin.

**Interfaces.**
- `knowlu-engine commitments --vault <v> --confirm <file> [--today YYYY-MM-DD] [--actor quinn] [--via dashboard]`.
  With `--confirm`, `--json` is implied. On success it prints
  `{"created": n, "declined": n, "warnings": [...], "window": "created"|"updated"|"unchanged"|null}`
  (keys sorted by `ledger::dumps_value`) and exits 0. On unreadable input or an invalid window it
  prints `knowlu-engine: <why>` on stderr, exits 2 and writes nothing.
- Produces `pub fn commitments_confirm(vault: &Path, today_iso: Option<&str>, input: &str, ctx: &WriteContext) -> Result<crate::commitments::ConfirmReport, String>` in `cli`.

**Plan ruling Q4-a:** the arm takes `--actor` (default `quinn`) beside `--via` (default
`dashboard`, `value_parser = journal::VIAS`), the way `write` does (`main.rs:175–188`). *Why:* the
spec names both the actor and `--via`, and `write`'s pair is the house pattern.
**Plan ruling Q4-b:** a window with no day in it (`[]`, or entries all skipped) is invalid: exit 2,
nothing written. *Why:* such a note could not be loaded (`strict_meets` needs at least one entry).
The editor sends no `window` key when every row is blank (Q10).
**Plan ruling Q4-c:** the `window:` line written is the input's own flow sequence, parsed by
`serde_yaml_ng` and re-emitted by `write::to_literal`. The input is validated first by
`parse_window`, strictly. *Why:* the student's grouping of days survives, and the line is one flow
line like every collection this model writes.
**Plan ruling Q4-d:** the write order is every `mine` row in input order, then every `not_mine`
row, then the window. The window is validated before any write. *Why:* §3 says "the window is
validated first, then the writes run in input order". A JSON object has no order across keys, so
the order is fixed here.

- [ ] **Step 1 — failing tests.** Add to `mod phase2_tests`:

```rust
    const WEEKDAYS_8_22: &str = "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]";

    #[test]
    fn a_window_creates_the_planning_day_as_the_human() {
        let v = week_vault("wincreate");
        let report = run(&v, &input(&[], &[], Some(WEEKDAYS_8_22)));
        assert_eq!(report.window, Some("created"));
        let set = load(&v);
        let day = set.planning_day.expect("a planning-day note");
        assert_eq!(day.path, PathBuf::from("commitments/planning-day.md"));
        assert_eq!(set.window[0], Some((t(8, 0), t(22, 0))));
        assert_eq!(set.window[5], None, "Saturday keeps the template");
        let create = journal_records(&v).into_iter().find(|r| r.get("op").and_then(|o| o.as_str()) == Some("create")).unwrap();
        assert_eq!(create.get("actor").and_then(|a| a.as_str()), Some("quinn"));
    }

    #[test]
    fn a_window_edit_on_an_existing_note_changes_one_line() {
        let v = week_vault("winedit");
        run(&v, &input(&[], &[], Some(WEEKDAYS_8_22)));
        let path = v.join(FOLDER).join("planning-day.md");
        let before = std::fs::read_to_string(&path).unwrap();
        let later = "[{days: [mon, tue, wed, thu, fri], start: \"07:30\", end: \"22:00\"}, {days: [sat, sun], start: \"10:00\", end: \"22:00\"}]";
        let report = run(&v, &input(&[], &[], Some(later)));
        assert_eq!(report.window, Some("updated"));
        let after = std::fs::read_to_string(&path).unwrap();
        let changed: Vec<(&str, &str)> = before.lines().zip(after.lines()).filter(|(a, b)| a != b).collect();
        assert_eq!(before.lines().count(), after.lines().count());
        assert_eq!(changed.len(), 1, "{changed:?}");
        assert!(changed[0].1.starts_with("window: "), "{changed:?}");
        assert_eq!(load(&v).window[5], Some((t(10, 0), t(22, 0))));
        let set = journal_records(&v).into_iter().find(|r| r.get("op").and_then(|o| o.as_str()) == Some("set")).unwrap();
        assert_eq!(set.get("field").and_then(|f| f.as_str()), Some("window"));
        assert_eq!(set.get("actor").and_then(|a| a.as_str()), Some("quinn"), "a human edit journal.human_set protects");
        assert_eq!(run(&v, &input(&[], &[], Some(later))).window, Some("unchanged"));
    }

    #[test]
    fn an_invalid_window_writes_nothing_at_all() {
        let v = week_vault("winbad");
        let bad = "[{days: [mon], start: \"15:00\", end: \"14:00\"}]";
        let got = confirm(&v, &input(&[("gcal-series:cs100", "hard")], &["gcal-series:chess"], Some(bad)), TODAY, &human(), &mut Journal::new(&*v));
        assert!(got.unwrap_err().contains("planning day mon"));
        assert!(files(&v, FOLDER).is_empty(), "not even the rows");
        assert!(journal_records(&v).is_empty());
        let empty = confirm(&v, &input(&[], &[], Some("[]")), TODAY, &human(), &mut Journal::new(&*v));
        assert!(empty.is_err(), "a window naming no day is invalid (Q4-b)");
    }
```

  `engine/tests/commitments_confirm.rs` (new):

```rust
//! Phase 2 (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md` §3):
//! `knowlu-engine commitments --confirm`, run as the binary. Every key is invented.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-confirm-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    dir
}

/// The input goes in a file beside the vault, never inside it, so the tree below is the vault's.
fn run(vault: &Path, text: &str) -> Output {
    let file = vault.with_extension("confirm.json");
    std::fs::write(&file, text).unwrap();
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", "2026-09-24", "--confirm"])
        .arg(&file)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&file);
    out
}

/// Every file under the vault, relative, sorted.
fn tree(vault: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(vault, vault, &mut out);
    out.sort();
    out
}

#[test]
fn an_invalid_window_writes_nothing_and_exits_2() {
    let v = scratch("badwindow");
    let before = tree(&v);
    let out = run(&v, r#"{"window": "[{days: [mon], start: \"15:00\", end: \"14:00\"}]"}"#);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("planning day mon"), "{out:?}");
    assert!(out.stdout.is_empty());
    assert_eq!(tree(&v), before);
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn unreadable_input_exits_2() {
    let v = scratch("unreadable");
    assert_eq!(run(&v, "not json").status.code(), Some(2));
    let missing = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(&v)
        .args(["--confirm"])
        .arg(v.join("no-such-file.json"))
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(!tree(&v).iter().any(|f| f.starts_with("commitments/") || f.starts_with("state/")), "{:?}", tree(&v));
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_window_prints_the_report_and_journals_the_human() {
    let v = scratch("report");
    let out = run(&v, r#"{"window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]"}"#);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "{\"created\": 0, \"declined\": 0, \"warnings\": [], \"window\": \"created\"}\n"
    );
    assert!(v.join("commitments").join("planning-day.md").is_file());
    let journal: String = std::fs::read_dir(v.join("state").join("journal"))
        .unwrap()
        .flatten()
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    assert!(journal.contains("\"actor\": \"quinn\"") && journal.contains("\"via\": \"dashboard\""), "{journal}");
    let _ = std::fs::remove_dir_all(&v);
}

/// Spec §3: `--confirm` makes no network call. The vault names a feed on a loopback listener
/// that accepts nothing; after the command, no connection is waiting.
#[test]
fn confirm_makes_no_network_call() {
    let v = scratch("nonet");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        v.join("config").join("ingest.yaml"),
        format!("timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: http://127.0.0.1:{port}/a.ics\n"),
    )
    .unwrap();
    let out = run(
        &v,
        r#"{"mine": [{"source_uid": "gcal-series:x", "level": "hard"}], "not_mine": ["gcal-series:y"], "window": "[{days: [mon], start: \"08:00\", end: \"22:00\"}]"}"#,
    );
    assert!(out.status.success(), "{out:?}");
    match listener.accept() {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        other => panic!("--confirm opened a connection: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests` (the three window tests
  fail: `report.window` is `None` and the invalid window is not refused), then
  `cargo test -p knowlu-engine --test commitments_confirm -j 2` (fails: clap refuses `--confirm`,
  exit 2 everywhere, and the success-path tests fail).

- [ ] **Step 3 — implement.** Replace Q3's `confirm` whole:

```rust
/// `commitments --confirm` (phase-2 spec §3): the screen's answers, written under `ctx` (the
/// console's human context; parent §2.5). Every row is re-derived from the series file
/// ([`stored_proposals`]). Nothing from the app becomes a field value but `level` and the
/// window.
///
/// The window is validated first ([`parse_window`], strictly; one day at least, Plan ruling Q4-b).
/// If it is invalid, this returns `Err` before any write. Then, in order (Plan ruling Q4-d):
/// - **`mine`** → [`create_confirmed_as`] at the chosen level. A key a confirmed note already holds,
///   by `source_uid` or by signature, is skipped silently: a second Finish is a no-op.
/// - **`not_mine`** → [`create_marker_as`] for the key and every twin ([`twin_keys`]), as a card
///   rejection does. A key a confirmed note holds gets no marker. A key already declined is skipped
///   silently.
/// - **the window** → the planning day, created when there is none ([`create_confirmed_as`]).
///   Otherwise its one `window:` line is written through `write_literals`, a human edit that
///   `journal.human_set` protects. An equal window writes nothing (`unchanged`).
///
/// A key that is no current proposal, or is the window proposal's, is skipped with a warning. A
/// write that fails is reported, and the rest still run. No card is filed, so nothing is charged
/// to the cap.
pub fn confirm(
    vault: &Path,
    input: &ConfirmInput,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<ConfirmReport, String> {
    let window = match &input.window {
        None => None,
        Some(raw) => {
            let parsed = parse_window(raw)?;
            if parsed.iter().all(Option::is_none) {
                return Err(format!("window {raw:?} names no day"));
            }
            let value: Value = serde_yaml_ng::from_str(raw).map_err(|_| format!("{raw:?} is not a flow sequence"))?;
            Some((parsed, value))
        }
    };

    let stored = stored_proposals(vault, today);
    let codes = &stored.codes;
    let mut report = ConfirmReport::default();
    let stale = |key: &str| format!("{key}: not a current proposal; skipped");

    for (key, level) in &input.mine {
        let set = load(vault);
        if set.confirmed.iter().any(|n| n.source_uid.as_deref() == Some(key.as_str())) {
            continue;
        }
        let Some(p) = stored.unanswered.iter().find(|p| &p.source_uid == key && !p.is_window()) else {
            report.warnings.push(stale(key));
            continue;
        };
        let sigs = proposal_signatures(p, codes);
        if set.confirmed.iter().any(|n| sigs.contains(&n.signature(codes))) {
            continue;
        }
        if set.declined.contains(key) || !stored.proposals.iter().any(|q| &q.source_uid == key) {
            report.warnings.push(stale(key));
            continue;
        }
        let Some(level) = Level::parse(level) else {
            report.warnings.push(format!("{key}: level {level:?} is not hard, soft or optional; skipped"));
            continue;
        };
        let chosen = Proposal { level, ..p.clone() };
        let Value::Mapping(map) = to_value(proposal_commitment(&chosen)) else {
            report.warnings.push(format!("{key}: not written (no commitment mapping)"));
            continue;
        };
        match create_confirmed_as(vault, &map, key, today, ctx, journal) {
            Ok(_) => report.created += 1,
            Err(e) => report.warnings.push(format!("{key}: not written ({e})")),
        }
    }

    for key in &input.not_mine {
        let set = load(vault);
        if set.declined.contains(key) {
            continue;
        }
        let Some(p) = stored.proposals.iter().find(|p| &p.source_uid == key && !p.is_window()) else {
            report.warnings.push(stale(key));
            continue;
        };
        let held: BTreeSet<String> = set.confirmed.iter().filter_map(|n| n.source_uid.clone()).collect();
        for twin in twin_keys(vault, key, proposal_signatures(p, codes), codes) {
            if held.contains(&twin) {
                continue;
            }
            match create_marker_as(vault, &twin, ctx, journal) {
                Ok(Some(_)) => report.declined += 1,
                Ok(None) => {}
                Err(e) => report.warnings.push(format!("{twin}: marker not written ({e})")),
            }
        }
    }

    if let Some((parsed, value)) = window {
        let set = load(vault);
        let outcome: Result<&'static str, String> = match &set.planning_day {
            Some(_) if set.window == parsed => Ok("unchanged"),
            Some(day) => {
                let rel = day.path.to_string_lossy().replace('\\', "/");
                let literals = vec![("window".to_string(), crate::write::to_literal(&value))];
                crate::write::write_literals(vault, &rel, &literals, ctx, journal, &crate::write::WriteOpts::default())
                    .map(|_| "updated")
                    .map_err(|e| format!("planning day not written ({e})"))
            }
            None => {
                let mut map = Mapping::new();
                map.insert(Value::String("kind".into()), Value::String(PLANNING_DAY.into()));
                map.insert(Value::String("window".into()), value);
                create_confirmed_as(vault, &map, "", today, ctx, journal)
                    .map(|_| "created")
                    .map_err(|e| format!("planning day not written ({e})"))
            }
        };
        match outcome {
            Ok(done) => report.window = Some(done),
            Err(why) => report.warnings.push(why),
        }
    }
    Ok(report)
}
```

  In `engine/src/cli.rs`, after `commitments_report_with`:

```rust
/// Phase-2 spec §3: `commitments --confirm`. Parses the input, pins `today` (the vault's zone
/// when none is given) and runs `commitments::confirm` under `ctx`. It fetches nothing: no
/// calendar fetcher is built and `config/cloud.yaml` is not read. `Err` is the exit-2 message.
pub fn commitments_confirm(
    vault: &Path,
    today_iso: Option<&str>,
    input: &str,
    ctx: &WriteContext,
) -> Result<crate::commitments::ConfirmReport, String> {
    let input = crate::commitments::parse_confirm(input)?;
    let today = match today_iso {
        Some(iso) => Date::strptime("%Y-%m-%d", iso).map_err(|_| format!("bad --today {iso:?}"))?,
        None => Timestamp::now().to_zoned(vault_zone(vault)).date(),
    };
    let mut journal = Journal::new(vault);
    crate::commitments::confirm(vault, &input, today, ctx, &mut journal)
}
```

  In `engine/src/main.rs`, `Command::Commitments` (lines 63–75) becomes:

```rust
    /// Fetch, refresh the series file, and print the current commitment proposals — the
    /// phase-2 confirm screen's data source (spec §5.1, R14). Writes no note, no card and no
    /// journal record; `state/calendar.md` is untouched. Always exits 0. With `--confirm`
    /// (phase-2 spec §3) it fetches nothing: it writes the screen's answers from a JSON file,
    /// prints `{created, declined, warnings, window}`, and exits 2 on unreadable input or an
    /// invalid window, having written nothing.
    Commitments {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Pin the run date (YYYY-MM-DD). Without it, the vault's zone and the system date.
        #[arg(long)]
        today: Option<String>,
        #[arg(long)]
        json: bool,
        /// `{"mine": [{"source_uid", "level"}], "not_mine": [...], "window": "<flow sequence>"}`.
        #[arg(long)]
        confirm: Option<PathBuf>,
        #[arg(long, default_value = "quinn")]
        actor: String,
        #[arg(long, default_value = "dashboard", value_parser = journal::VIAS)]
        via: String,
    },
```

  and the arm (between the markers, lines 360–377) starts:

```rust
        Command::Commitments { vault, today, json, confirm, actor, via } => {
            if let Some(path) = confirm {
                let text = match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("knowlu-engine: --confirm {}: {err}", path.display());
                        return ExitCode::from(2);
                    }
                };
                let ctx = write::WriteContext::new(&actor, &via);
                return match cli::commitments_confirm(&vault, today.as_deref(), &text, &ctx) {
                    Ok(report) => {
                        println!("{}", knowlu_engine::ledger::dumps_value(&report.to_json()));
                        ExitCode::SUCCESS
                    }
                    Err(err) => {
                        eprintln!("knowlu-engine: {err}");
                        ExitCode::from(2)
                    }
                };
            }
            let report = cli::commitments_report(&vault, today.as_deref());
```

  The rest of the arm (the `if json` block from Q2, and the plain-text branch) is unchanged. The
  two marker comments stay where they are, so `rank_cannot_reach_a_judgment_endpoint` scans the
  confirm branch too.

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests`;
  `cargo test -p knowlu-engine --test commitments_confirm -j 2` (4 pass);
  `cargo test -p knowlu-engine --test cloud_contract -j 2`;
  `cargo test -p knowlu-engine --bin knowlu-engine -j 2` (`main.rs`'s own clap tests still pass).

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs engine/src/cli.rs engine/src/main.rs engine/tests/commitments_confirm.rs`;
  message:

```
feat(engine): commitments --confirm writes the screen's answers and the planning day (phase 2, §3)

The window is validated before any write (exit 2, nothing written), then
created as the planning-day note or written as its one window line, a human
edit. The flag takes --actor/--via (default quinn/dashboard), implies --json,
and fetches nothing: a loopback listener sees no connection.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

Plan ruling (final review): Q4-c's "one line changed" holds for editing an existing planning-day note; creating one writes the whole note.

---

## Q5 — `commitments::overview` (D3, §4)

**Files.**
- `engine/src/commitments.rs`: `Overview` and `overview` after `confirm`; tests in `phase2_tests`.

**Interfaces.**
- Produces:

```rust
pub struct Overview {
    pub commitments: Vec<serde_json::Value>,
    pub office_hours: Vec<serde_json::Value>,
    pub window: Vec<serde_json::Value>,
    pub uncovered_courses: Vec<UncoveredCourse>,
    pub setup: bool,
    pub warnings: Vec<String>,
}
impl Overview { pub fn to_json(&self) -> serde_json::Value }
pub fn overview(vault: &Path, today: Date) -> Overview
```

- The JSON: `{"commitments": [{"id","path","kind","level","title","course","meets","when","where","from","until"}],
  "office_hours": [<proposal_value>], "window": [{"day","start","end","source": "note"|"template"} × 7],
  "uncovered_courses": [{"slug","title"}], "setup": bool, "warnings": [...]}`. It is written as
  bytes only through `ledger::dumps_value`; the app passes the `Value` to Tauri.

**Plan ruling Q5-a:** `Overview`'s rows are `serde_json::Value`s built with the same helpers the
cards use (`meets_json`, `meets_label`, `date_json`, `proposal_value`), and `to_json` assembles
them. *Why:* one shape for a proposal row everywhere, and no parallel `Serialize` struct that could
drift from `proposal_value`.

`setup` is D3's test: `vault_day(vault, today) == 1 && set.planning_day.is_none()`.

- [ ] **Step 1 — failing tests.** Add to `mod phase2_tests`:

```rust
    #[test]
    fn overview_has_every_key_the_view_reads_and_seven_window_days() {
        let v = week_vault("overview");
        journal_on(&v, "2026-09-01");
        run(&v, &input(&[("gcal-series:chess", "soft")], &[], None));
        let o = overview(&v, TODAY);
        assert!(o.setup, "day 1 (the earliest journal file is 2026-09-01) and no planning-day note");
        let json = o.to_json();
        for key in ["commitments", "office_hours", "window", "uncovered_courses", "setup", "warnings"] {
            assert!(json.get(key).is_some(), "{key} missing: {json}");
        }
        let row = &json["commitments"][0];
        assert_eq!(row["kind"], "club");
        assert_eq!(row["level"], "soft");
        assert_eq!(row["title"], "Chess Club");
        assert_eq!(row["when"], "Wed 6–7pm");
        assert_eq!(row["path"], "commitments/chess-club.md");
        assert!(row["id"].as_str().unwrap().starts_with("cmt_"));
        assert_eq!(json["office_hours"].as_array().unwrap().len(), 1);
        assert_eq!(json["office_hours"][0]["kind"], "office-hours");
        let window = json["window"].as_array().unwrap();
        assert_eq!(window.len(), 7);
        assert_eq!(window[0], serde_json::json!({"day": "mon", "start": "08:00", "end": "18:00", "source": "template"}));
        assert_eq!(json["uncovered_courses"], serde_json::json!([]), "CS 100 has a class proposal");
        let bytes = crate::ledger::dumps_value(&json);
        assert!(bytes.contains("\"setup\": true"), "{bytes}");
    }

    #[test]
    fn setup_is_true_only_on_the_first_day_with_no_planning_day() {
        let v = week_vault("setup");
        journal_on(&v, "2026-09-01");
        assert!(overview(&v, TODAY).setup, "day 1, no planning-day note");
        assert!(!overview(&v, date(2026, 9, 2)).setup, "day 2: the cards take over");
        run(&v, &input(&[], &[], Some("[{days: [mon], start: \"07:00\", end: \"23:00\"}]")));
        let o = overview(&v, TODAY);
        assert!(!o.setup, "Finish wrote the planning day, so the screen never comes back");
        assert_eq!(o.window[0], serde_json::json!({"day": "mon", "start": "07:00", "end": "23:00", "source": "note"}));
        assert_eq!(o.window[1]["source"], "template");
    }
```

  The confirm in the first test journals at the real clock, a later file than 2026-09-01, so
  `TODAY` stays day 1; confirming a club does not end `setup`, only a planning-day note does.

- [ ] **Step 2 — run and see them fail.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests::overview commitments::phase2_tests::setup`
  Expected: compile errors (`overview` does not exist).

- [ ] **Step 3 — implement.** After `confirm`:

```rust
/// What the *Schedule* view and the confirm screen read (phase-2 spec §4). Built purely, with no
/// fetch and no write, over [`stored_proposals`]. Rows are JSON (Plan ruling Q5-a).
#[derive(Debug, Clone, PartialEq)]
pub struct Overview {
    /// Every confirmed note but the planning day: `{id, path, kind, level, title, course, meets,
    /// when, where, from, until}`.
    pub commitments: Vec<serde_json::Value>,
    /// The office-hours proposals not yet confirmed or declined, as [`proposal_value`].
    pub office_hours: Vec<serde_json::Value>,
    /// The effective window per weekday, `DAY_KEYS` order: `{day, start, end, source}`, where
    /// `source` is `note` (the planning day) or `template` (`week_template.yaml`).
    pub window: Vec<serde_json::Value>,
    pub uncovered_courses: Vec<UncoveredCourse>,
    /// D3: the vault is on its first day and has no planning-day note.
    pub setup: bool,
    pub warnings: Vec<String>,
}

impl Overview {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "commitments": self.commitments,
            "office_hours": self.office_hours,
            "window": self.window,
            "uncovered_courses": self.uncovered_courses,
            "setup": self.setup,
            "warnings": self.warnings,
        })
    }
}

pub fn overview(vault: &Path, today: Date) -> Overview {
    let stored = stored_proposals(vault, today);
    let commitments = stored
        .set
        .confirmed
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "path": n.path.to_string_lossy().replace('\\', "/"),
                "kind": n.kind,
                "level": n.level.as_str(),
                "title": n.title,
                "course": n.course,
                "meets": meets_json(&n.meets),
                "when": meets_label(&n.meets),
                "where": n.where_,
                "from": date_json(n.from),
                "until": date_json(n.until),
            })
        })
        .collect();
    let office_hours = stored.proposals.iter().filter(|p| p.kind == "office-hours").map(proposal_value).collect();
    let window = DAY_KEYS
        .iter()
        .enumerate()
        .map(|(i, day)| {
            let ((start, end), source) = match stored.set.window[i] {
                Some(span) => (span, "note"),
                None => ((stored.template.day_start, stored.template.day_end), "template"),
            };
            serde_json::json!({ "day": day, "start": hm(start), "end": hm(end), "source": source })
        })
        .collect();
    let uncovered_courses = uncovered_courses(vault, &stored.set, &stored.proposals, &stored.codes);
    let setup = vault_day(vault, today) == 1 && stored.set.planning_day.is_none();
    Overview { commitments, office_hours, window, uncovered_courses, setup, warnings: stored.warnings }
}
```

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests`.

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs`; message:

```
feat(engine): commitments::overview, the Schedule view's read (phase 2, §4, D3)

Confirmed commitments, office-hours proposals, the window per weekday with its
source, uncovered courses and `setup` (day 1 and no planning day), read purely
from the notes and the series file.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q6 — `emit_asks` and its `rank` wiring (§5 emitter)

**Files.**
- `engine/src/commitments.rs`: new constants and `emit_asks` after `emit_checks` (lines 2850–2935);
  tests in `phase2_tests`.
- `engine/src/cli.rs`: `commitment_passes` (lines 802–848 before Q1; the `emit_checks` call and
  the `CommitmentPasses { … }` return); the P16 tests that newly file an ask (see Step 4).

**Interfaces.**
- Produces `pub const COMMITMENT_ASK: &str = "commitment-ask";`, `pub const ASKS_PER_DAY: i64 = 2;`,
  `pub const ASK_FROM_DAY: i64 = 3;` and `pub fn ask_key(slug: &str) -> String` (`card:<slug>`).
- Produces `pub fn emit_asks(vault: &Path, proposals: &[Proposal], today: Date, budget: i64, ctx: &crate::write::WriteContext, journal: &mut crate::journal::Journal) -> (Vec<PathBuf>, usize, Vec<String>)`.
- The card: `approvals/commitment-ask-<slugify(title)>.md`, frontmatter `type: approval`,
  `kind: commitment-ask`, `title: When does <course title> meet?`, `status: pending`,
  `source_uid: card:<slug>`, `course: <slug>`, `proposed_at`, `first_proposed_at` (both today),
  `expires: null`, `snooze_until: null`, `created_by: agent:commitments` (all through `file_card`).

**Plan ruling Q6-a:** `emit_asks` takes `proposals: &[Proposal]` after `vault`, which the spec's
signature `emit_asks(vault, today, budget, ctx, journal)` leaves out. *Why:* "no pending class
proposal" needs the proposals. `rank` has just computed them, and recomputing would read the
series file twice in one pass.
**Plan ruling Q6-b:** `rank` skips `emit_asks` when the series file could not be read
(`series_read_failed`). *Why:* an unread file would make every course look uncovered. This is the
same rule that skips withdrawal on a bad read.
**Plan ruling Q6-c:** the card carries `source_uid: card:<slug>` (the parent's §5.3 marker key)
and `course: <slug>`. A course is asked once: an ask card for it in `approvals/` or `archive/`
closes the question, unless that card is `superseded` or `expired`, the rule `asked` uses for
check cards (parent §5.5). *Why:* one card per course, ever, unless it was withdrawn unanswered.

- [ ] **Step 1 — failing tests.** Add to `mod phase2_tests`:

```rust
    fn file_asks(vault: &Path, today: Date, budget: i64) -> (Vec<PathBuf>, usize, Vec<String>) {
        let ctx = WriteContext::new("agent:rank", "cli");
        emit_asks(vault, &[], today, budget, &ctx, &mut Journal::new(vault))
    }

    fn front_of(path: &Path) -> Mapping {
        let raw = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        split_frontmatter(&raw).unwrap().0
    }

    #[test]
    fn no_ask_before_the_vaults_third_day() {
        let v = three_courses("day3");
        journal_on(&v, "2026-08-31");
        assert_eq!(file_asks(&v, TODAY, 15).1, 0, "2026-09-01 is day 2");
        assert_eq!(file_asks(&v, date(2026, 9, 2), 15).1, 2, "day 3");
    }

    #[test]
    fn at_most_two_a_day_in_slug_order_and_each_course_once() {
        let v = three_courses("twoaday");
        journal_on(&v, "2026-08-20");
        let (paths, n, warnings) = file_asks(&v, TODAY, 15);
        assert_eq!((n, warnings.len()), (2, 0), "{warnings:?}");
        let courses: Vec<String> = paths.iter().map(|p| field_text(&front_of(p), "course").unwrap()).collect();
        assert_eq!(courses, ["ant-101", "bui-100"]);
        assert_eq!(file_asks(&v, TODAY, 15).1, 0, "two already filed today");
        let (next, ..) = file_asks(&v, date(2026, 9, 2), 15);
        assert_eq!(next.len(), 1);
        assert_eq!(field_text(&front_of(&next[0]), "course").as_deref(), Some("cs-100"));
        assert_eq!(file_asks(&v, date(2026, 9, 3), 15).1, 0, "every course asked once");
    }

    #[test]
    fn the_budget_caps_the_asks() {
        let v = three_courses("budget");
        journal_on(&v, "2026-08-20");
        assert_eq!(file_asks(&v, TODAY, 1).1, 1);
        assert_eq!(file_asks(&v, date(2026, 9, 2), 0).1, 0);
    }

    #[test]
    fn the_card_carries_its_kind_course_key_and_title() {
        let v = three_courses("card");
        journal_on(&v, "2026-08-20");
        let (paths, ..) = file_asks(&v, TODAY, 15);
        assert_eq!(paths[0], v.join("approvals").join("commitment-ask-when-does-ant-101-meet.md"));
        let meta = front_of(&paths[0]);
        let field = |k: &str| field_text(&meta, k).unwrap_or_default();
        assert_eq!(field("type"), "approval");
        assert_eq!(field("kind"), COMMITMENT_ASK);
        assert_eq!(field("title"), "When does ANT 101 meet?");
        assert_eq!(field("status"), "pending");
        assert_eq!(field("source_uid"), "card:ant-101");
        assert_eq!(field("course"), "ant-101");
        assert_eq!(field("first_proposed_at"), "2026-09-01");
        assert_eq!(field("created_by"), CARD_ACTOR);
    }

    #[test]
    fn a_declined_or_answered_course_is_not_asked_but_a_withdrawn_one_is() {
        let v = three_courses("closed");
        journal_on(&v, "2026-08-20");
        create_marker(&v, &ask_key("ant-101"), &WriteContext::new("agent:rank", "cli"), &mut Journal::new(&*v)).unwrap();
        let archive = v.join("archive");
        std::fs::create_dir_all(&archive).unwrap();
        for (name, course, status) in [("a.md", "bui-100", "rejected"), ("b.md", "cs-100", "superseded")] {
            std::fs::write(
                archive.join(name),
                format!("---\ntype: approval\nkind: commitment-ask\nstatus: {status}\ncourse: {course}\nfirst_proposed_at: 2026-08-25\n---\n"),
            )
            .unwrap();
        }
        let (paths, ..) = file_asks(&v, TODAY, 15);
        let courses: Vec<String> = paths.iter().map(|p| field_text(&front_of(p), "course").unwrap()).collect();
        assert_eq!(courses, ["cs-100"]);
    }
```

- [ ] **Step 2 — run and see them fail.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests`
  Expected: compile errors (`emit_asks`, `COMMITMENT_ASK`, `ask_key` do not exist).

- [ ] **Step 3 — implement.** In `engine/src/commitments.rs`, after `emit_checks`:

```rust
// ---------------------------------------------------------------------------------------------
// Phase 2 — the per-course fallback card, `commitment-ask` (parent §5.3, phase-2 spec §5).
// ---------------------------------------------------------------------------------------------

/// The per-course fallback card's kind (one of [`LOCAL_CARD_KINDS`]).
pub const COMMITMENT_ASK: &str = "commitment-ask";

/// At most this many `commitment-ask` cards are first proposed on any one day (§5.3).
pub const ASKS_PER_DAY: i64 = 2;

/// The first vault day ([`vault_day`]) on which an ask may be filed (§5.3).
pub const ASK_FROM_DAY: i64 = 3;

/// A course's ask key: the card's `source_uid`, and the decline marker's that closes it (§5.3).
pub fn ask_key(slug: &str) -> String {
    format!("card:{slug}")
}

fn ask_body(title: &str) -> String {
    format!(
        "**When does this class meet?** Knowlu didn't find {title} on your calendar.\n\n\
         Answer with its days and times and Knowlu plans around them.\n\n\
         Reject if it has no set meeting times (an online course), and you won't be asked again.\n"
    )
}

/// Every course an ask card in `approvals/` or `archive/` already covers, unless that card is
/// `superseded` or `expired` (Plan ruling Q6-c), and how many ask cards were first proposed
/// `today`, whatever their status.
fn asked_courses(vault: &Path, today: Date) -> (BTreeSet<String>, i64) {
    let mut courses = BTreeSet::new();
    let mut today_count = 0;
    for folder in ["approvals", "archive"] {
        let dir = vault.join(folder);
        if !dir.is_dir() {
            continue;
        }
        for path in crate::approvals::sorted_md(&dir) {
            let Ok(raw) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = split_frontmatter(&raw) else { continue };
            let field = |key: &str| field_text(&meta, key).unwrap_or_default();
            if field("type") != "approval" || field("kind") != COMMITMENT_ASK {
                continue;
            }
            if crate::approvals::as_date(get(&meta, "first_proposed_at")) == Some(today) {
                today_count += 1;
            }
            if matches!(field("status").as_str(), "superseded" | "expired") {
                continue;
            }
            let course = field("course");
            if !course.is_empty() {
                courses.insert(course);
            }
        }
    }
    (courses, today_count)
}

/// File today's `commitment-ask` cards (parent §5.3, phase-2 spec §5): one per
/// [`uncovered_courses`] course, in slug order, from the vault's day [`ASK_FROM_DAY`], at most
/// `min(budget, ASKS_PER_DAY − asks first proposed today)`. A course is skipped when its
/// `card:<slug>` marker exists or an ask card already covers it ([`asked_courses`]). Every card
/// goes through [`file_card`]. A card that cannot be filed is one warning, and ends this run's
/// asks. Returns `(paths, count, warnings)`.
pub fn emit_asks(
    vault: &Path,
    proposals: &[Proposal],
    today: Date,
    budget: i64,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> (Vec<PathBuf>, usize, Vec<String>) {
    let mut filed: Vec<PathBuf> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    if vault_day(vault, today) < ASK_FROM_DAY {
        return (filed, 0, warnings);
    }
    let (mut asked, today_count) = asked_courses(vault, today);
    let allowance = budget.min(ASKS_PER_DAY - today_count).max(0) as usize;
    if allowance == 0 {
        return (filed, 0, warnings);
    }
    let set = load(vault);
    let (codes, _) = Codes::load(vault);
    for course in uncovered_courses(vault, &set, proposals, &codes) {
        if filed.len() >= allowance {
            break;
        }
        let key = ask_key(&course.slug);
        if set.declined.contains(&key) || asked.contains(&course.slug) {
            continue;
        }
        let name = short_title(&course.title);
        let fields = vec![
            ("source_uid", Field::Scalar(Node::text(&key))),
            ("course", Field::Scalar(Node::text(&course.slug))),
        ];
        let title = format!("When does {name} meet?");
        match file_card(vault, COMMITMENT_ASK, fields, &title, &ask_body(&name), today, ctx, journal) {
            Ok(path) => {
                asked.insert(course.slug.clone());
                filed.push(path);
            }
            Err(err) => {
                warnings.push(err);
                break;
            }
        }
    }
    let count = filed.len();
    (filed, count, warnings)
}
```

  In `engine/src/cli.rs`'s `commitment_passes`, after the `emit_checks` call and its
  `warnings.extend(card_warnings);`, replace
  `CommitmentPasses { filed: filed as i64, withdrawn_pending, set }` with:

```rust
    // Phase-2 spec §5: the per-course fallback cards, with what the check cards left of the
    // budget. Skipped on a bad series read (Plan ruling Q6-b): an unread calendar would make
    // every course look uncovered.
    let mut asks = 0;
    if !read_failed {
        let ask_budget = std::cmp::max(0, planning.daily_approval_budget - count_proposals_created(vault, today));
        let (_, n, ask_warnings) = cm::emit_asks(vault, &proposals, today, ask_budget, ctx, journal);
        asks = n;
        warnings.extend(ask_warnings);
    }
    CommitmentPasses { filed: (filed + asks) as i64, withdrawn_pending, set }
```

  Update the doc comment of `CommitmentPasses::filed` to "Cards filed this run, checks and asks
  (added to the pending line)."

  Then a `rank` test in `cli.rs`'s tests, after Q1's gate test:

```rust
    /// Phase-2 spec §5: from the vault's day 3, a course with no class gets one ask card, counted
    /// into the pending line like a check card.
    #[test]
    fn rank_files_an_ask_for_a_course_with_no_class_from_day_three() {
        let vault = p16_vault("p2ask");
        rank_p16(&vault, P16_MONDAY, Vec::new());
        assert!(md_names(&vault.join("approvals"), "commitment-ask-").is_empty(), "P16_MONDAY is day 2");
        rank_p16(&vault, p16_day(1), Vec::new());
        assert_eq!(md_names(&vault.join("approvals"), "commitment-ask-"), ["commitment-ask-when-does-cs-100-intro-to-computing-meet.md"]);
        assert!(page(&vault).contains("**Approvals: 1 pending**"), "{}", page(&vault));
        let _ = std::fs::remove_dir_all(&vault);
    }
```

  (`short_title` cuts at 40 characters; `CS 100 Intro to Computing` is 25, so it is whole.)

- [ ] **Step 4 — run, and settle the P16 tests the asks reach.**
  `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests cli::tests::rank_files_an_ask`
  (pass), then `cargo test -p knowlu-engine --lib -j 2 -- cli::tests` in full. `p16_vault` holds
  `courses/cs-100.md`. A P16 test that ranks on `p16_day(1)` or later, with CS 100 neither
  confirmed nor proposed on that run, now also files
  `commitment-ask-when-does-cs-100-intro-to-computing-meet.md`, and its pending count goes up by
  one. That is the spec's behaviour, not a regression. The review traced every P16 rank past day 2
  (finding 4): exactly one test is reached,
  **`rank_withdraws_a_card_whose_series_left_the_file`** (`cli.rs:2769–2793`). It ranks on
  `p16_day(14)` with CS 100 neither confirmed nor proposed, so its `**Approvals: 1 pending**
  (oldest 2d)` and `pending == 1` would read 2. The `age_out_cs100` tests confirm `CS100_NOTE`;
  `no_card_is_withdrawn_and_refiled_across_three_ranks` keeps CS 100 proposed; every other test
  ranks only on `P16_MONDAY`, day 2. Add this helper once, beside `seed_journal_day`, and call it
  right after `p16_vault(...)` in that one test. Change nothing else in it:

```rust
    /// The student already said CS 100 has no set meeting times (a `card:cs-100` marker), so a
    /// P16 test about check cards sees no ask (phase-2 spec §5).
    fn decline_cs100_ask(vault: &Path) {
        let ctx = WriteContext::new("quinn", "dashboard");
        crate::commitments::create_marker(vault, &crate::commitments::ask_key("cs-100"), &ctx, &mut Journal::new(vault)).unwrap();
    }
```

  If any other test fails, or fails in any other way, or the marker breaks one of its own
  assertions (e.g. an assertion that `commitments/` does not exist), stop and report it: do not
  edit its assertions.
  Then `cargo test -p knowlu-engine --lib -j 2` (all green) and
  `cargo test -p knowlu-engine --test oracle --test surface_oracle -j 2` (no fixture vault has
  `courses/`).

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs engine/src/cli.rs`; message:

```
feat(engine): the commitment-ask card for a course with no class (phase 2, §5)

emit_asks files "When does <course> meet?" from the vault's day 3, at most two a
day, in slug order, once per course, through file_card; rank runs it after the
check cards with what they left of the budget and counts it pending. The one P16
test about check cards that now ranks past day 2 with CS 100 uncovered,
rank_withdraws_a_card_whose_series_left_the_file, declines CS 100's ask first.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q7 — the `commitment-ask` settlement arm and withdrawal (§5 the answer)

**Files.**
- `engine/src/commitments.rs`: `create_confirmed_as` (Q3) gains the `card:` body line; new
  `AskSettled`, `settle_ask_approved`, `settle_ask_rejected`, `ask_withdrawal_reason` after
  `emit_asks`.
- `engine/src/approvals.rs`: the `use crate::commitments::{…}` line (line 18); `transition_note`'s
  rejected branch (lines 1230–1245), the `refused|superseded` leftover (1247–1253) and the approved
  `COMMITMENT_CHECK` arm (1420–1440); a new `withdraw_asks` after `withdraw_stale` (1476–1519); a
  new `mod commitment_ask` inside `mod tests`, after `mod commitment_check` (ends at line 4852).
- `engine/src/cli.rs`: `commitment_passes`, right after the `proposals` line.

**Interfaces.**
- Produces `#[derive(Debug, Clone, PartialEq, Eq)] pub enum AskSettled { Executed, Refused(String), Returned(String) }`.
- Produces `pub fn settle_ask_approved(vault: &Path, meta: &Mapping, today: Date, ctx: &WriteContext, journal: &mut Journal) -> Result<AskSettled, WriteError>`,
  `pub fn settle_ask_rejected(vault: &Path, meta: &Mapping, ctx: &WriteContext, journal: &mut Journal) -> Result<Option<PathBuf>, WriteError>`,
  and `pub fn ask_withdrawal_reason(meta: &Mapping, proposals: &[Proposal], set: &Commitments, codes: &Codes) -> Option<String>`.
- Produces `pub fn withdraw_asks(vault: &Path, proposals: &[Proposal], set: &Commitments, ctx: &WriteContext, journal: &mut Journal) -> Withdrawal` in `approvals`.
- The card's answer: `answer_meets: [{days: [...], start: "HH:MM", end: "HH:MM"}]`, one flow line.

**Plan ruling Q7-a:** an invalid or missing `answer_meets` returns `AskSettled::Returned(why)`.
The arm writes `status: pending` back onto the card, does not archive it, and pushes
`<file name>: <why>` into `result.warnings`. That warning is the one Q8's `answer_card` shows on
the card. *Why:* spec §5 (as amended) says the console shows the engine's warning from
`answer_card`'s envelope until the next repaint. A persistent carrier (an `Option` field on
`Card`, `skip_serializing_if`) would leave the surface-oracle bytes alone, but it would need a
second write onto the card for a message the student has just seen (review, problem 3). The card
is not counted pending on the pass that returned it; the next pass counts it.
**Plan ruling Q7-b:** the class note's `title` is the course note's title (`active_courses`, else
the slug), and its file is `commitments/<slugify(title)>.md`. *Why:* §2.2 requires a title and
`create_confirmed` refuses a note without one. The parent's §5.3 names `commitments/<slug>.md`,
which is what the wizard's course titles slugify to. §2.2 says a file name is never an identity.
**Plan ruling Q7-c:** `rank` withdraws a pending or snoozed ask right after it computes the
proposals, before either emitter runs. It withdraws when a class proposal for the course appears,
a confirmed class note covers the course, or the `card:<slug>` marker exists. *Why:* the class
proposal is only known once the proposals exist. Withdrawing before `emit_asks` lets the same run
see the course as covered.

- [ ] **Step 1 — failing tests.** In `engine/src/approvals.rs`, inside `mod tests`, after
  `mod commitment_check`'s closing brace (line 4852):

```rust
    // The commitment-ask card (phase-2 spec §5, parent §5.3): approve with an answer → the class
    // note; an invalid answer → back to pending with the warning; reject → the card:<slug>
    // marker; withdrawal once the course is covered. Every course and time is invented.
    mod commitment_ask {
        use super::*;
        use crate::commitments::{self, Level, Meet, Proposal};
        use jiff::civil::Time;

        const NAME: &str = "commitment-ask-when-does-bui-100-meet.md";

        fn ask_vault() -> (PathBuf, PathBuf) {
            let v = vault();
            let courses = v.join("courses");
            std::fs::create_dir_all(&courses).unwrap();
            pystr::write_text(&courses.join("bui-100.md"), "---\ntitle: \"BUI 100\"\nslug: bui-100\ncode: \"BUI 100\"\nstatus: active\n---\n").unwrap();
            let card = proposal(
                &v,
                NAME,
                "id: appr_00000000a1\ntype: approval\nkind: commitment-ask\ntitle: When does BUI 100 meet?\nstatus: pending\n\
                 source_uid: card:bui-100\ncourse: bui-100\nproposed_at: 2026-08-20\nfirst_proposed_at: 2026-08-20\n\
                 expires: null\nsnooze_until: null\ncreated_by: agent:commitments",
                "Invented.\n",
            );
            (v, card)
        }

        /// What `answer_card` does before it approves: the answer, as the student.
        fn answer(vault: &Path, card: &Path, literal: &str) {
            let console = WriteContext::new("quinn", "dashboard");
            let literals = vec![("answer_meets".to_string(), literal.to_string())];
            write_literals(vault, &rel_path(vault, card), &literals, &console, &mut Journal::new(vault), &WriteOpts::default()).unwrap();
        }

        /// What `decide_inner` does: the status as the student, then the pass as `agent:approvals`.
        fn decide(vault: &Path, card: &Path, status: &str) -> ApprovalsResult {
            let console = WriteContext::new("quinn", "dashboard");
            let mut journal = Journal::new(vault);
            let literals = vec![("status".to_string(), status.to_string())];
            write_literals(vault, &rel_path(vault, card), &literals, &console, &mut journal, &WriteOpts::default()).unwrap();
            process_approvals(vault, TODAY, now(), &default_ctx(), &mut journal)
        }

        #[test]
        fn a_valid_answer_creates_the_class_note_and_archives_the_card_executed() {
            let (v, card) = ask_vault();
            answer(&v, &card, "[{days: [mon, wed], start: '14:00', end: '15:15'}]");
            let result = decide(&v, &card, "approved");
            assert!(result.warnings.is_empty(), "{:?}", result.warnings);
            assert_eq!(result.executed, ["commitment-ask-when-does-bui-100-meet"]);
            let set = commitments::load(&v);
            assert_eq!(set.confirmed.len(), 1);
            let note = &set.confirmed[0];
            assert_eq!(note.path, PathBuf::from("commitments/bui-100.md"));
            assert_eq!((note.kind.as_str(), note.level, note.course.as_deref()), ("class", Level::Hard, Some("bui-100")));
            assert_eq!(note.source_uid.as_deref(), Some("card:bui-100"));
            assert_eq!(note.meets, vec![Meet { days: vec!["mon", "wed"], start: Time::constant(14, 0, 0, 0), end: Time::constant(15, 15, 0, 0) }]);
            assert!(!card.exists());
            assert!(read(&v.join("archive").join(NAME)).contains("status: executed"));
        }

        #[test]
        fn an_invalid_answer_goes_back_to_pending_with_the_warning() {
            let (v, card) = ask_vault();
            answer(&v, &card, "[{days: [mon], start: '15:00', end: '14:00'}]");
            let result = decide(&v, &card, "approved");
            assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
            assert!(result.warnings[0].starts_with(&format!("{NAME}: answer_meets entry mon")), "{:?}", result.warnings);
            assert!(card.exists(), "the card stays in approvals/");
            assert!(read(&card).contains("status: pending"));
            assert!(!v.join("commitments").exists());
        }

        #[test]
        fn no_answer_at_all_also_goes_back_to_pending() {
            let (v, card) = ask_vault();
            let result = decide(&v, &card, "approved");
            assert!(result.warnings[0].starts_with(&format!("{NAME}: no answer_meets")), "{:?}", result.warnings);
            assert!(read(&card).contains("status: pending"));
        }

        #[test]
        fn rejecting_writes_the_card_marker_and_archives() {
            let (v, card) = ask_vault();
            decide(&v, &card, "rejected");
            assert!(commitments::load(&v).declined.contains("card:bui-100"));
            assert!(!card.exists() && v.join("archive").join(NAME).exists());
        }

        fn class_for_bui() -> Proposal {
            Proposal {
                kind: "class".into(),
                level: Level::Hard,
                title: "BUI 100".into(),
                course: Some("bui-100".into()),
                meets: vec![Meet { days: vec!["tue"], start: Time::constant(9, 0, 0, 0), end: Time::constant(10, 0, 0, 0) }],
                where_: None,
                from: None,
                until: None,
                source_uid: "gcal-series:bui".into(),
            }
        }

        #[test]
        fn a_pending_ask_is_withdrawn_once_a_class_proposal_for_its_course_appears() {
            let (v, card) = ask_vault();
            let set = commitments::load(&v);
            let none = withdraw_asks(&v, &[], &set, &default_ctx(), &mut Journal::new(&v));
            assert!(none.withdrawn.is_empty());
            let done = withdraw_asks(&v, &[class_for_bui()], &set, &default_ctx(), &mut Journal::new(&v));
            assert_eq!(done.withdrawn, ["commitment-ask-when-does-bui-100-meet"]);
            assert_eq!(done.pending, 1);
            assert!(!card.exists());
            assert!(read(&v.join("archive").join(NAME)).contains("status: superseded"));
        }

        /// Review finding 3: a hand-written class note with no `course:` whose title leads with
        /// the course's code covers the course (Q2-c), so the pending ask is withdrawn.
        #[test]
        fn a_title_only_class_note_withdraws_the_ask() {
            let (v, card) = ask_vault();
            let folder = v.join("commitments");
            std::fs::create_dir_all(&folder).unwrap();
            pystr::write_text(
                &folder.join("bui.md"),
                "---\nid: cmt_00000000d1\ntype: commitment\nkind: class\ntitle: \"BUI 100 Lecture\"\n\
                 meets: [{days: [tue], start: \"09:00\", end: \"10:00\"}]\nstatus: confirmed\n---\n\nMine.\n",
            )
            .unwrap();
            let set = commitments::load(&v);
            let done = withdraw_asks(&v, &[], &set, &default_ctx(), &mut Journal::new(&v));
            assert_eq!(done.withdrawn, ["commitment-ask-when-does-bui-100-meet"]);
            assert!(!card.exists());
        }
    }
```

- [ ] **Step 2 — run and see them fail.** `cargo test -p knowlu-engine --lib -j 2 -- approvals::tests::commitment_ask`
  Expected: compile error (`withdraw_asks` does not exist). With a stub it would fail on
  `unknown kind:`.

- [ ] **Step 3 — implement.** In `commitments.rs`'s `create_confirmed_as`, the body-sentence
  `let body = if key.starts_with("gcal-series:") { … } else { … };` gains a middle branch:

```rust
        let body = if key.starts_with("gcal-series:") {
            "Found as a weekly series on your Google Calendar.\n"
        } else if key.starts_with("card:") {
            "You told Knowlu when this class meets.\n"
        } else {
            "Found as a weekly series on your calendar.\n"
        };
```

  After `emit_asks`:

```rust
/// What settling an approved `commitment-ask` card decided (phase-2 spec §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskSettled {
    /// The class note was created: stamped `executed` and archived.
    Executed,
    /// Nothing written and nothing will be: stamped `refused` and archived, one warning.
    Refused(String),
    /// The answer is missing or invalid: back to `pending`, one warning (Plan ruling Q7-a).
    Returned(String),
}

fn ask_course(meta: &Mapping) -> String {
    field_text(meta, "course").map(|s| s.trim().to_string()).unwrap_or_default()
}

/// Whether a confirmed `class` note covers `slug`, by Q2-c's two rules ([`covered_course_keys`],
/// review finding 3): a title-only class note covers its course here exactly as it does for
/// [`uncovered_courses`].
fn has_class(set: &Commitments, slug: &str, codes: &Codes) -> bool {
    covered_course_keys(set, codes).contains(&course_key(slug, codes))
}

/// Settle an approved `commitment-ask` card (parent §5.3): validate `answer_meets` as §2.2
/// validates `meets` (every entry valid, at least one), then create the class note
/// `{kind: class, level: hard, title, course, meets}` with `source_uid: card:<slug>`, actor
/// `agent:commitments` (Plan ruling Q7-b). A class note the course already has refuses the card,
/// unless this card's own settlement wrote it and the run died before the stamp.
pub fn settle_ask_approved(
    vault: &Path,
    meta: &Mapping,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<AskSettled, crate::write::WriteError> {
    let slug = ask_course(meta);
    if slug.is_empty() {
        return Ok(AskSettled::Refused("the card has no course".to_string()));
    }
    let key = ask_key(&slug);
    let set = load(vault);
    let (codes, _) = Codes::load(vault);
    if has_class(&set, &slug, &codes) {
        if created_by_this_card(vault, meta, Some(&key), journal) {
            return Ok(AskSettled::Executed);
        }
        return Ok(AskSettled::Refused(format!("{slug} already has a confirmed class; nothing written")));
    }
    let meets = match strict_meets(meta, "answer_meets") {
        Ok(meets) => meets,
        Err(why) => return Ok(AskSettled::Returned(format!("{why}; answer again"))),
    };
    let title = active_courses(vault).into_iter().find(|(s, _)| *s == slug).map(|(_, t)| t).unwrap_or_else(|| slug.clone());
    let mut map = Mapping::new();
    for (k, v) in [("kind", "class"), ("level", "hard"), ("title", title.as_str()), ("course", slug.as_str())] {
        map.insert(Value::String(k.into()), Value::String(v.into()));
    }
    map.insert(Value::String("meets".into()), to_value(meets_json(&meets)));
    create_confirmed(vault, &map, &key, today, ctx, journal)?;
    Ok(AskSettled::Executed)
}

/// Settle a rejected `commitment-ask` card: the `card:<slug>` marker, which closes the question
/// (parent §5.3). Returns the marker written, `None` when it existed already.
pub fn settle_ask_rejected(
    vault: &Path,
    meta: &Mapping,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Option<PathBuf>, crate::write::WriteError> {
    let slug = ask_course(meta);
    if slug.is_empty() {
        return Ok(None);
    }
    create_marker(vault, &ask_key(&slug), ctx, journal)
}

/// Why a pending or snoozed ask's question went away (Plan ruling Q7-c), or `None` while it
/// stands: a class proposal for its course appeared (that card asks instead), a confirmed class
/// note covers the course, or the `card:<slug>` marker exists.
pub fn ask_withdrawal_reason(meta: &Mapping, proposals: &[Proposal], set: &Commitments, codes: &Codes) -> Option<String> {
    let slug = ask_course(meta);
    if slug.is_empty() {
        return None;
    }
    let wanted = course_key(&slug, codes);
    if proposals
        .iter()
        .any(|p| p.kind == "class" && p.course.as_deref().is_some_and(|c| course_key(c, codes) == wanted))
    {
        return Some(format!("a class proposal for {slug} appeared"));
    }
    if has_class(set, &slug, codes) {
        return Some(format!("{slug} already has a class"));
    }
    if set.declined.contains(&ask_key(&slug)) {
        return Some(format!("{slug} was answered"));
    }
    None
}
```

  In `engine/src/approvals.rs`, line 18 becomes
  `use crate::commitments::{AskSettled, Codes, Commitments, Proposal, SeriesFile, Settled, COMMITMENT_ASK, COMMITMENT_CHECK};`.
  In `transition_note`, the rejected branch gains an arm after the `COMMITMENT_CHECK` one:

```rust
        } else if kind == COMMITMENT_ASK {
            // Phase-2 spec §5: the card:<slug> marker first, then the generic archive.
            crate::commitments::settle_ask_rejected(vault, meta, ctx, journal)?;
        }
```

  The leftover guard (line 1247) becomes
  `if (kind == COMMITMENT_CHECK || kind == COMMITMENT_ASK) && matches!(status.as_str(), "refused" | "superseded") {`.
  In the approved branch, after the `COMMITMENT_CHECK` arm's closing `}` and before
  `} else if kind == "coursework-map" {`:

```rust
        } else if kind == COMMITMENT_ASK {
            // Phase-2 spec §5 (Plan ruling Q7-a): the class note first, the stamp second. An
            // invalid answer puts the card back to pending, unarchived, with the warning.
            match crate::commitments::settle_ask_approved(vault, meta, today, ctx, journal)? {
                AskSettled::Returned(why) => {
                    let literals = vec![("status".to_string(), "pending".to_string())];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    result.warnings.push(format!("{name}: {}", crate::commitments::single_line(&why)));
                }
                AskSettled::Executed => {
                    let literals = vec![
                        ("status".to_string(), "executed".to_string()),
                        ("executed_at".to_string(), stamped),
                    ];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    delete(vault, &rel, ctx, journal)?;
                    result.executed.push(stem);
                }
                AskSettled::Refused(why) => {
                    let literals = vec![("status".to_string(), "refused".to_string())];
                    write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
                    delete(vault, &rel, ctx, journal)?;
                    result.warnings.push(format!("{name}: {}", crate::commitments::single_line(&why)));
                }
            }
```

  (`stamped`, `rel`, `name` and `stem` are the locals the neighbouring arms already use; the
  `COMMITMENT_CHECK` arm moves `stamped` only inside its own branch, so each arm owns it.)
  After `withdraw_stale`:

```rust
/// Withdraw every pending or snoozed `commitment-ask` card whose question went away
/// ([`crate::commitments::ask_withdrawal_reason`]): stamped `superseded` and archived, no other
/// write. `rank` calls it once the proposals exist (Plan ruling Q7-c). Never raises into the run.
pub fn withdraw_asks(
    vault: &Path,
    proposals: &[Proposal],
    set: &Commitments,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Withdrawal {
    let mut out = Withdrawal::default();
    let folder = vault.join("approvals");
    if !folder.is_dir() {
        return out;
    }
    let (codes, _) = Codes::load(vault);
    for path in sorted_md(&folder) {
        let Some((meta, _)) = read_note(&path) else { continue };
        if str_field(&meta, "type") != "approval"
            || str_field(&meta, "kind") != COMMITMENT_ASK
            || !matches!(str_field(&meta, "status").as_str(), "pending" | "snoozed")
        {
            continue;
        }
        if crate::commitments::ask_withdrawal_reason(&meta, proposals, set, &codes).is_none() {
            continue;
        }
        let rel = rel_path(vault, &path);
        let literals = vec![("status".to_string(), "superseded".to_string())];
        let moved = write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())
            .and_then(|_| delete(vault, &rel, ctx, journal));
        match moved {
            Ok(_) => {
                out.withdrawn.push(stem_of(&path));
                if str_field(&meta, "status") == "pending" {
                    out.pending += 1;
                }
            }
            Err(_) => out.warnings.push(format!("transition failed: {}", name_of(&path))),
        }
    }
    out
}
```

  In `engine/src/cli.rs`'s `commitment_passes`, right after the `let proposals = …;` line:

```rust
    // Phase-2 spec §5 (Plan ruling Q7-c): an ask whose course now has a class proposal, a class
    // note or a marker is withdrawn before either emitter runs.
    let asks_out = crate::approvals::withdraw_asks(vault, &proposals, &set, ctx, journal);
    withdrawn_pending += asks_out.pending;
    warnings.extend(asks_out.warnings);
```

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- approvals:: commitments:: cli::tests`
  (all green), then `cargo test -p knowlu-engine --test cloud_contract --test oracle --test surface_oracle -j 2`.

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs engine/src/approvals.rs engine/src/cli.rs`; message:

```
feat(engine): settle and withdraw the commitment-ask card (phase 2, §5)

Approve with a valid answer_meets creates the course's class note (card:<slug>);
an invalid or missing answer puts the card back to pending with the warning;
reject writes the card:<slug> marker. rank withdraws a pending ask once a class
proposal, a class note or the marker covers its course.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q8 — `kind`/`level` in `EDITABLE`, and `answer_card` (D5, §5 the answer)

**Files.**
- `engine/src/commitments.rs`: `CONSOLE_FIELDS`, `CONSOLE_KINDS`, `check_console_edit` after
  `ask_withdrawal_reason`; tests in `phase2_tests`.
- `app/src/commands.rs`: `now_in` (lines 18–24) becomes `pub(crate)`; `EDITABLE` (line 152);
  `set_fields_inner` (209–222); `decide_inner` (267–283) splits into `decide_in`; a new
  `answer_card_inner` and `#[tauri::command(async)] answer_card` right after `decide`'s
  wrapper (line 402).
- `app/src/main.rs`: the console `generate_handler!` list (line 186) gains `commands::answer_card`
  right after `commands::decide`.
- `app/tests/commands.rs`: the `use knowlu::commands::{…}` list (lines 4–9) and four tests.
- `app/static/console.js`: the comment over its own `EDITABLE` (lines 11–12) only (review
  finding 8).

**Interfaces.**
- Produces `pub const CONSOLE_FIELDS: [&str; 2] = ["kind", "level"];`,
  `pub const CONSOLE_KINDS: [&str; 9]`,
  `pub fn check_console_edit(vault: &Path, target: &str, field: &str, value: &serde_json::Value) -> Result<(), String>`
  and `pub fn check_answerable(vault: &Path, target: &str) -> Result<(), String>` (review
  finding 6: a pending `commitment-ask` approval, or refused).
- `EDITABLE: [&str; 14]` gains `"kind", "level"`.
- Produces `pub fn answer_card_inner(cs: &ConsoleState, view: &str, id: &str, meets: &Value) -> Result<Value, String>`
  and the Tauri command
  `answer_card(cs, sch, view: String, id: String, meets: Value) -> Value`. The envelope is
  `{ok, error, state, decision}`, where `decision` has `decide`'s shape
  `{executed, expired, woken, rejected, warnings}`.
- `meets` from the page: `[{"days": ["mon"], "start": "14:00", "end": "15:15"}]`, JSON. The app
  converts it with `knowlu_engine::yaml::from_json` and emits it with `write::to_literal`.

**Plan ruling Q8-a:** the checks D5 names are the engine's `check_console_edit`, which
`set_fields_inner` calls for a field in `CONSOLE_FIELDS`. It checks that the note is under
`commitments/`, is confirmed, is not the planning day, and that the value is on the list. *Why:*
`commands.rs` computes nothing, and the vocabulary lives beside `Level::parse` and
`default_level`. `console.js`'s own `EDITABLE` (line 13, the drawer's list) stays at twelve
fields: kind and level are edited from the *Schedule* view, and the drawer would offer them on
every approval, where they would be refused.
**Plan ruling Q8-b:** `CONSOLE_KINDS` is the parent's §2.2 kind list without `planning-day`:
`class, lab, work, club, meeting, office-hours, event, exam, task-block`. The planning-day note
refuses both fields. *Why:* the planning day is the editor, not a row (spec §4), and a row turned
into a planning day would replace the student's window.
**Plan ruling Q8-c:** when the settlement did not execute the card, `answer_card` returns
`ok: false` with that card's own warnings (those in `decision.warnings` that start with
`<file name>: `, prefix removed, joined by `; `). This covers the card going back to pending and
the card being refused. Writes that did land still count through `mutate`'s `note_write`. *Why:*
the spec wants the engine's warning shown on the card. Picking out this card's lines is
marshalling, not validation.

- [ ] **Step 1 — failing tests.** In `commitments.rs`'s `phase2_tests`:

```rust
    #[test]
    fn console_edits_reach_only_a_confirmed_commitment_and_only_known_values() {
        let v = week_vault("edits");
        run(&v, &input(&[("gcal-series:cs100", "hard")], &[], Some("[{days: [mon], start: \"08:00\", end: \"22:00\"}]")));
        std::fs::create_dir_all(v.join("tasks")).unwrap();
        std::fs::write(v.join("tasks").join("essay.md"), "---\ntitle: Essay\n---\n").unwrap();
        let word = |w: &str| serde_json::json!(w);
        assert_eq!(check_console_edit(&v, "commitments/cs-100.md", "level", &word("soft")), Ok(()));
        assert_eq!(check_console_edit(&v, "commitments/cs-100.md", "kind", &word("lab")), Ok(()));
        assert!(check_console_edit(&v, "commitments/cs-100.md", "level", &word("urgent")).unwrap_err().contains("level"));
        assert!(check_console_edit(&v, "commitments/cs-100.md", "kind", &word("planning-day")).is_err());
        assert!(check_console_edit(&v, "commitments/cs-100.md", "kind", &serde_json::json!(3)).is_err());
        assert!(check_console_edit(&v, "tasks/essay.md", "kind", &word("class")).unwrap_err().contains("commitment"));
        assert!(check_console_edit(&v, "commitments/planning-day.md", "level", &word("hard")).is_err());
    }

    #[test]
    fn only_a_pending_ask_card_is_answerable() {
        let v = three_courses("answerable");
        journal_on(&v, "2026-08-20");
        let (paths, ..) = file_asks(&v, TODAY, 15);
        let rel = format!("approvals/{}", paths[0].file_name().unwrap().to_string_lossy());
        assert_eq!(check_answerable(&v, &rel), Ok(()));
        std::fs::create_dir_all(v.join("tasks")).unwrap();
        std::fs::write(v.join("tasks").join("essay.md"), "---\ntitle: Essay\n---\n").unwrap();
        assert!(check_answerable(&v, "tasks/essay.md").unwrap_err().contains("not a pending commitment-ask card"));
    }
```

  In `app/tests/commands.rs`, add `answer_card_inner` to the `use knowlu::commands::{…}` list, then:

```rust
/// A vault-full copy holding `courses/bui-100.md` and a pending `commitment-ask` card for it
/// (invented), with "today" pinned to the fixture's own reference date.
fn ask_vault(name: &str) -> (PathBuf, ConsoleState) {
    let v = scratch(name);
    std::fs::create_dir_all(v.join("courses")).unwrap();
    std::fs::write(v.join("courses/bui-100.md"), "---\ntitle: \"BUI 100\"\nslug: bui-100\ncode: \"BUI 100\"\nstatus: active\n---\n").unwrap();
    std::fs::write(
        v.join("approvals/commitment-ask-when-does-bui-100-meet.md"),
        "---\nid: appr_00000000a1\ntype: approval\nkind: commitment-ask\ntitle: When does BUI 100 meet?\nstatus: pending\n\
         source_uid: card:bui-100\ncourse: bui-100\nproposed_at: 2026-08-28\nfirst_proposed_at: 2026-08-28\n\
         expires: null\nsnooze_until: null\ncreated_by: agent:commitments\n---\n\nInvented.\n",
    )
    .unwrap();
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-{name}-data-{}", std::process::id())));
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    (v, cs)
}

#[test]
fn answer_card_writes_the_answer_as_the_student_and_approves() {
    let (v, cs) = ask_vault("answer");
    let meets = json!([{ "days": ["mon", "wed"], "start": "14:00", "end": "15:15" }]);
    let env = answer_card_inner(&cs, "today", "appr_00000000a1", &meets).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["state"]["schema"], 1);
    let note = std::fs::read_to_string(v.join("commitments/bui-100.md")).unwrap();
    assert!(note.contains("source_uid: card:bui-100") && note.contains("kind: class"), "{note}");
    assert!(v.join("archive/commitment-ask-when-does-bui-100-meet.md").exists());
    let records = journal_records(&v);
    let answer = records.iter().find(|r| r["field"] == "answer_meets").expect("the answer is journaled");
    assert_eq!(answer["actor"], "quinn");
    assert_eq!(answer["via"], "dashboard");
}

/// Review finding 6: `answer_card` pointed at another approval (vault-full's pending task
/// proposal) is refused before any write; the task is not materialised.
#[test]
fn answer_card_refuses_a_card_that_is_not_an_ask() {
    let (v, cs) = ask_vault("notask");
    let s = state_inner(&cs, "decisions").unwrap();
    let task = s["state"]["decisions"]["cards"].as_array().unwrap().iter().find(|c| c["kind"] == json!("task")).expect("vault-full's task proposal").clone();
    let tasks_before = std::fs::read_dir(v.join("tasks")).unwrap().count();
    let env = answer_card_inner(&cs, "today", task["id"].as_str().unwrap(), &json!([])).unwrap();
    assert_eq!(env["ok"], false, "{env}");
    assert!(env["error"].as_str().unwrap().contains("not a pending commitment-ask card"), "{env}");
    assert_eq!(std::fs::read_dir(v.join("tasks")).unwrap().count(), tasks_before, "nothing materialised");
    let card = std::fs::read_to_string(v.join("approvals").join(format!("{}.md", task["slug"].as_str().unwrap()))).unwrap();
    assert!(!card.contains("answer_meets"), "no stray line on the task proposal");
}

#[test]
fn an_invalid_answer_comes_back_as_the_engines_warning_and_the_card_stays_pending() {
    let (v, cs) = ask_vault("badanswer");
    let meets = json!([{ "days": ["mon"], "start": "15:00", "end": "14:00" }]);
    let env = answer_card_inner(&cs, "today", "appr_00000000a1", &meets).unwrap();
    assert_eq!(env["ok"], false, "{env}");
    assert!(env["error"].as_str().unwrap().starts_with("answer_meets entry mon"), "{env}");
    let card = std::fs::read_to_string(v.join("approvals/commitment-ask-when-does-bui-100-meet.md")).unwrap();
    assert!(card.contains("status: pending"), "{card}");
    assert!(!v.join("commitments").exists());
}

#[test]
fn kind_and_level_are_editable_only_on_a_confirmed_commitment() {
    let v = scratch("cmtedit");
    std::fs::create_dir_all(v.join("commitments")).unwrap();
    std::fs::write(
        v.join("commitments/chess-club.md"),
        "---\nid: cmt_00000000c1\ntype: commitment\nkind: club\nlevel: soft\ntitle: \"Chess Club\"\n\
         meets: [{days: [wed], start: \"18:00\", end: \"19:00\"}]\nstatus: confirmed\n---\n\nInvented.\n",
    )
    .unwrap();
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-cmtedit-data-{}", std::process::id())));
    let set = |field: &str, value: serde_json::Value, id: &str| {
        let mut f = serde_json::Map::new();
        f.insert(field.into(), value);
        set_fields_inner(&cs, "today", id, f).unwrap()
    };
    assert_eq!(set("level", json!("optional"), "cmt_00000000c1")["ok"], true);
    assert_eq!(set("kind", json!("meeting"), "cmt_00000000c1")["ok"], true);
    let text = std::fs::read_to_string(v.join("commitments/chess-club.md")).unwrap();
    assert!(text.contains("level: optional") && text.contains("kind: meeting"), "{text}");
    let bad = set("level", json!("urgent"), "cmt_00000000c1");
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("level"), "{bad}");
    let task = set("kind", json!("class"), &first_id(&cs));
    assert_eq!(task["ok"], false);
    assert!(task["error"].as_str().unwrap().contains("commitment"), "{task}");
}
```

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests::console_edits` (compile
  error), then `cargo build -p knowlu-engine -j 2` and
  `cargo test -p knowlu --test commands -j 2 -- answer kind_and_level` (compile error:
  `answer_card_inner` does not exist).

- [ ] **Step 3 — implement.** In `commitments.rs`, after `ask_withdrawal_reason`:

```rust
/// The fields the console may set on a confirmed commitment (phase-2 spec D5); `window` is not
/// one of them: it goes through `--confirm`.
pub const CONSOLE_FIELDS: [&str; 2] = ["kind", "level"];

/// The kinds a confirmed commitment may be given from the console: §2.2's list without
/// `planning-day` (Plan ruling Q8-b).
pub const CONSOLE_KINDS: [&str; 9] =
    ["class", "lab", "work", "club", "meeting", "office-hours", "event", "exam", "task-block"];

/// Phase-2 spec D5, the engine half of `set_fields`' checks (Plan ruling Q8-a): `field` (one of
/// [`CONSOLE_FIELDS`]) may be set on `target` only when it is a confirmed note under
/// `commitments/` other than the planning day, and only to a word on §2.2's list. Reads only.
pub fn check_console_edit(vault: &Path, target: &str, field: &str, value: &serde_json::Value) -> Result<(), String> {
    let path = crate::ids::resolve_target(vault, target).map_err(|e| e.to_string())?;
    let rel = crate::ids::rel(vault, &path);
    if !rel.starts_with(&format!("{FOLDER}/")) {
        return Err(format!("{field} can only be set on a commitment, not {rel}"));
    }
    let text = pystr::read_text(&path).map_err(|e| e.to_string())?;
    let (meta, _) = split_frontmatter(&text).map_err(|e| e.to_string())?;
    if field_text(&meta, "status").as_deref() != Some("confirmed") || field_text(&meta, "kind").as_deref() == Some(PLANNING_DAY) {
        return Err(format!("{field} can only be set on a confirmed commitment, not {rel}"));
    }
    let word = value.as_str().ok_or_else(|| format!("{field} must be one word"))?;
    let known = match field {
        "kind" => CONSOLE_KINDS.contains(&word),
        "level" => Level::parse(word).is_some(),
        _ => false,
    };
    if known {
        Ok(())
    } else {
        Err(format!("{field} {word:?} is not one Knowlu knows"))
    }
}

/// `answer_card`'s engine check (review finding 6): `target` is a `pending` approval of kind
/// [`COMMITMENT_ASK`]. Anything else — a task proposal, a settled card — is refused before a
/// write, so a wrong id can never approve and materialise another card. Reads only.
pub fn check_answerable(vault: &Path, target: &str) -> Result<(), String> {
    let path = crate::ids::resolve_target(vault, target).map_err(|e| e.to_string())?;
    let rel = crate::ids::rel(vault, &path);
    let text = pystr::read_text(&path).map_err(|e| e.to_string())?;
    let (meta, _) = split_frontmatter(&text).map_err(|e| e.to_string())?;
    let field = |key: &str| field_text(&meta, key).unwrap_or_default();
    if field("type") == "approval" && field("kind") == COMMITMENT_ASK && field("status") == "pending" {
        Ok(())
    } else {
        Err(format!("{rel} is not a pending commitment-ask card"))
    }
}
```

  In `app/src/commands.rs`: `fn now_in` becomes `pub(crate) fn now_in` (Q9's `week.rs` reads
  "today" the same way). `EDITABLE` becomes:

```rust
/// Frontmatter fields the console may edit. Anything else (`id`, `source_uid`, `judgment`, …) is
/// refused rather than silently ignored. `kind` and `level` are a commitment's (phase-2 spec D5):
/// `knowlu_engine::commitments::check_console_edit` refuses them anywhere else and any value off
/// §2.2's lists. The page's own list (`console.js`, the drawer's) keeps the first twelve.
pub const EDITABLE: [&str; 14] = ["title", "course", "due", "effort_hours", "importance", "importance_reason", "status", "progress", "slice_hours", "domain", "rank_override", "effort_confidence", "kind", "level"];
```

  In `set_fields_inner`'s loop, after the `status`-null refusal:

```rust
            if knowlu_engine::commitments::CONSOLE_FIELDS.contains(&k.as_str()) {
                knowlu_engine::commitments::check_console_edit(&cs.vault, id, k, v)?;
            }
```

  `decide_inner` splits, so `answer_card` reuses the decision path verbatim:

```rust
pub fn decide_inner(cs: &ConsoleState, view: &str, id: &str, verdict: &str, note: &str, snooze_until: Option<String>) -> Result<Value, String> {
    let mut decision = Value::Null;
    let env = mutate(cs, view, |journal| {
        decision = decide_in(cs, journal, id, verdict, note, snooze_until.as_deref())?;
        Ok(())
    })?;
    let mut env = env; env["decision"] = decision; Ok(env)
}

/// The verdict as Quinn's write, then `process_approvals` in-process as `executor_ctx()` —
/// `decide_inner`'s body, shared with `answer_card_inner`. Returns the `decision` summary.
fn decide_in(cs: &ConsoleState, journal: &mut Journal, id: &str, verdict: &str, note: &str, snooze_until: Option<&str>) -> Result<Value, String> {
    if !["approved", "rejected", "snoozed"].contains(&verdict) { return Err(format!("verdict must be approved, rejected or snoozed, not {verdict:?}")); }
    let snooze = match (verdict, snooze_until) { ("snoozed", Some(d)) if d.len() == 10 => d.to_string(), ("snoozed", _) => return Err("snoozed needs snooze_until as YYYY-MM-DD".into()), _ => "null".into() };
    let literals = vec![
        ("status".to_string(), verdict.to_string()),
        ("decision_note".to_string(), write::to_literal(&serde_yaml_ng::Value::String(note.to_string()))),
        ("snooze_until".to_string(), snooze),
    ];
    write::write_literals(&cs.vault, id, &literals, &console_ctx(), journal, &WriteOpts::default()).map_err(|e| e.to_string())?;
    let now = now_in(cs);
    let r = knowlu_engine::approvals::process_approvals(&cs.vault, now.date(), now.datetime(), &executor_ctx(), journal);
    Ok(json!({ "executed": r.executed, "expired": r.expired, "woken": r.woken, "rejected": r.rejected, "warnings": r.warnings }))
}

/// Phase-2 spec §5: the `commitment-ask` card's answer. `answer_meets` is written as the
/// student's edit (the form's list, emitted by `write::to_literal`), then `decide`'s approve path
/// runs. The engine's settlement validates; nothing here does. When the card was not executed,
/// the envelope is `ok: false` with this card's own warnings (Plan ruling Q8-c).
pub fn answer_card_inner(cs: &ConsoleState, view: &str, id: &str, meets: &Value) -> Result<Value, String> {
    let name = knowlu_engine::ids::resolve_target(&cs.vault, id).ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    let mut decision = Value::Null;
    let env = mutate(cs, view, |journal| {
        // Review finding 6: only a pending commitment-ask card takes an answer — the engine says so.
        knowlu_engine::commitments::check_answerable(&cs.vault, id)?;
        let literal = write::to_literal(&knowlu_engine::yaml::from_json(meets));
        write::write_literals(&cs.vault, id, &[("answer_meets".to_string(), literal)], &console_ctx(), journal, &WriteOpts::default()).map_err(|e| e.to_string())?;
        decision = decide_in(cs, journal, id, "approved", "", None)?;
        Ok(())
    })?;
    let mut env = env;
    let prefix = name.map(|n| format!("{n}: ")).unwrap_or_default();
    let mine: Vec<String> = decision["warnings"].as_array().into_iter().flatten()
        .filter_map(|w| w.as_str())
        .filter(|w| !prefix.is_empty() && w.starts_with(&prefix))
        .map(|w| w[prefix.len()..].to_string())
        .collect();
    if env["ok"] == true && !mine.is_empty() { env["ok"] = json!(false); env["error"] = json!(mine.join("; ")); }
    env["decision"] = decision;
    Ok(env)
}
```

  In `app/static/console.js`, the two comment lines over `var EDITABLE` (lines 11–12) become
  (review finding 8; the list itself is unchanged, Plan ruling Q8-a):

```js
  // Task 13: the drawer's twelve editable fields — every one `set_fields` accepts but `kind` and
  // `level`, which it takes only on a commitment, from the Schedule view (commands.rs's EDITABLE).
```

  The Tauri wrapper, on the line after `decide`'s:

```rust
#[tauri::command(async)] pub fn answer_card(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, id: String, meets: Value) -> Value { let mut env = answer_card_inner(&cs, &view, &id, &meets).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
```

  In `app/src/main.rs`'s console list, `commands::decide,` becomes `commands::decide, commands::answer_card,` (43).

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- commitments::phase2_tests`;
  `cargo build -p knowlu-engine -j 2`; `cargo test -p knowlu --test commands -j 2` (every test,
  including `approving_a_task_proposal_puts_the_task_in_the_returned_state`, which pins the split
  `decide` path); `cargo test -p knowlu --test static_assets -j 2`
  (`no_multi_word_command_argument_is_sent_in_the_wrong_case` sees `answer_card`'s single-word
  arguments).

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs app/src/commands.rs app/src/main.rs app/tests/commands.rs app/static/console.js`; message:

```
feat(app): kind and level on a commitment, and answer_card (phase 2, D5, §5)

set_fields accepts kind and level only where the engine's check_console_edit
allows them (a confirmed note under commitments/, a word on §2.2's lists).
answer_card writes answer_meets as the student and approves through decide's
own path; an answer the settlement returns comes back as ok:false with its
warning. Console commands: 43.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q9 — `app/src/week.rs`: the four commands (§2, §4, §6)

**Files.**
- `app/src/week.rs` (new).
- `app/src/lib.rs`: `pub mod week;` after `pub mod updates;` (line 17).
- `app/src/main.rs`: the `use knowlu::{…}` line (line 3) gains `week`; the console list (line 186)
  gains four commands (47).
- `app/tests/week.rs` (new).

**Interfaces.**
- `pub fn proposals_argv(vault: &Path) -> Vec<String>` → `["commitments", "--vault", <v>, "--json"]`.
- `pub fn confirm_argv(vault: &Path, file: &Path, today: jiff::civil::Date) -> Vec<String>` →
  `["commitments", "--vault", <v>, "--today", <YYYY-MM-DD>, "--confirm", <file>, "--via", "dashboard"]`.
- `pub fn your_week_inner(cs: &ConsoleState) -> Value` → `{ok, error, week: <Overview::to_json>}`.
- `pub fn preview_window_inner(cs: &ConsoleState, window: &str) -> Value` → `{ok, error, state}`,
  the today view built by `surface::build_state_preview`.
- `pub fn commitment_proposals_inner(cs: &ConsoleState) -> Value` →
  `{ok: true, error, proposals, uncovered_courses, warnings}`.
- `pub fn commitments_confirm_inner(cs: &ConsoleState, view: &str, confirm: &Value) -> Value` →
  `{ok, error, result: {created, declined, warnings, window}|null, state}`.
- Tauri: `commitment_proposals(cs)` (async), `commitments_confirm(cs, sch, view, confirm)`
  (async), `your_week(cs)`, `preview_window(cs, window)` (async). Every argument is one word, so
  `no_multi_word_command_argument_is_sent_in_the_wrong_case` has nothing to catch.

**Plan ruling Q9-a:** `commitments_confirm` holds `cs.vault_io` for the child's whole run.
*Why:* the child writes notes, and a sync must not rewrite the tree under it. The hold is short
because `--confirm` fetches nothing. That differs from a slot's `run_child`, which can run for 20
minutes and never holds the lock.
**Plan ruling Q9-b:** the app passes `--today` from `commands::now_in`. *Why:* a test's pinned
date then reaches the engine, and `confirmed_at` agrees with the console's own "today".
**Plan ruling Q9-c:** `preview_window` returns the whole previewed today `State`. The page reads
`moved` and the first five takes of `the_day.blocks`. *Why:* the app computes nothing, and the
state already carries both.
**Plan ruling Q9-d:** when the engine cannot be spawned, or prints no JSON, `commitment_proposals`
answers `ok: true` with empty lists and the reason in `error`. *Why:* spec §2 says a failed fetch
shows only *Your day* and never an error wall. `discover_coursework` takes the same approach.

- [ ] **Step 1 — failing tests.** `app/tests/week.rs` (new):

```rust
//! Phase 2 of the commitment model (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md`
//! §7): the four `week.rs` commands against a scratch copy of `vault-full`. The spawning half
//! runs the real sibling engine: build it first with `cargo build -p knowlu-engine -j 2`, because
//! `app/build.rs` leaves a zero-byte placeholder at `target/debug/knowlu-engine.exe`.

use std::path::{Path, PathBuf};
use serde_json::json;
use knowlu::state::ConsoleState;
use knowlu::week::{commitment_proposals_inner, commitments_confirm_inner, confirm_argv, preview_window_inner, proposals_argv, your_week_inner};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qo-week-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(Path::new("../engine/tests/fixtures/vault-full"), &dir);
    dir
}
fn copy(from: &Path, to: &Path) { std::fs::create_dir_all(to).unwrap(); for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy(&p, &t); } else { std::fs::copy(&p, &t).unwrap(); } } }

fn open(v: &Path, name: &str) -> ConsoleState {
    ConsoleState::open(v.to_path_buf(), std::env::temp_dir().join(format!("qo-week-data-{name}-{}", std::process::id())))
}

/// `KNOWLU_ENGINE_EXE` is process-wide; this file's own lock and seam, `onboarding.rs`'s shape.
static ENGINE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct EngineExeSeam(Option<std::ffi::OsString>);
impl EngineExeSeam {
    fn set(exe: &std::ffi::OsStr) -> Self {
        let prev = std::env::var_os("KNOWLU_ENGINE_EXE");
        unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", exe) };
        EngineExeSeam(prev)
    }
}
impl Drop for EngineExeSeam {
    fn drop(&mut self) {
        match self.0.take() {
            Some(p) => unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", p) },
            None => unsafe { std::env::remove_var("KNOWLU_ENGINE_EXE") },
        }
    }
}

#[test]
fn your_week_reports_setup_on_the_vaults_first_day_only_and_writes_nothing() {
    let v = scratch("yourweek");
    let cs = open(&v, "yourweek");
    // vault-full's earliest journal file is 2026-08-27.
    cs.set_test_today(Some("2026-08-27".parse().unwrap()));
    let env = your_week_inner(&cs);
    assert_eq!(env["ok"], true, "{env}");
    let week = &env["week"];
    assert_eq!(week["setup"], true, "{week}");
    assert_eq!(week["window"].as_array().unwrap().len(), 7);
    assert_eq!(week["window"][0]["source"], "template");
    assert_eq!(week["commitments"], json!([]));
    assert_eq!(week["uncovered_courses"], json!([]), "vault-full has no courses/");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    assert_eq!(your_week_inner(&cs)["week"]["setup"], false);
    assert!(!v.join("commitments").exists() && !v.join("state/calendar-series.json").exists());
}

#[test]
fn preview_window_reports_moved_and_refuses_a_bad_window() {
    let v = scratch("preview");
    let cs = open(&v, "preview");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    // vault-full's pinned day schedules work inside 08:00–18:00 (golden-today-full.md), so a
    // window of 21:00–21:59 moves or drops every take: `moved` must be present.
    let env = preview_window_inner(&cs, "[{days: [mon, tue, wed, thu, fri, sat, sun], start: \"21:00\", end: \"21:59\"}]");
    assert_eq!(env["ok"], true, "{env}");
    assert!(env["state"]["moved"]["text"].is_string(), "{}", env["state"]["moved"]);
    assert!(env["state"]["the_day"]["blocks"].is_array());
    let bad = preview_window_inner(&cs, "[{days: [fri], start: \"15:00\", end: \"14:00\"}]");
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("planning day fri"), "{bad}");
    assert!(!v.join("commitments").exists(), "a preview writes nothing");
}

#[test]
fn the_argv_carries_the_vault_the_file_today_and_the_consoles_via() {
    let v = Path::new(r"C:\v");
    assert_eq!(proposals_argv(v), ["commitments", "--vault", r"C:\v", "--json"]);
    assert_eq!(
        confirm_argv(v, Path::new(r"C:\d\tmp\c.json"), "2026-09-24".parse().unwrap()),
        ["commitments", "--vault", r"C:\v", "--today", "2026-09-24", "--confirm", r"C:\d\tmp\c.json", "--via", "dashboard"]
    );
}

#[test]
fn commitment_proposals_with_no_engine_is_an_empty_list_and_a_reason() {
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(std::ffi::OsStr::new(r"C:\nowhere\knowlu-engine.exe"));
    let v = scratch("noengine");
    let env = commitment_proposals_inner(&open(&v, "noengine"));
    assert_eq!(env["ok"], true, "no error wall: {env}");
    assert_eq!(env["proposals"], json!([]));
    assert_eq!(env["uncovered_courses"], json!([]));
    assert!(env["error"].is_string(), "{env}");
}

#[test]
fn commitments_confirm_against_the_real_engine_sets_the_planning_day_as_the_student() {
    let exe = Path::new("../target/debug/knowlu-engine.exe");
    let len = std::fs::metadata(exe).map(|m| m.len()).unwrap_or(0);
    assert!(len > 0, "{}: run `cargo build -p knowlu-engine -j 2` first (build.rs leaves a zero-byte placeholder)", exe.display());
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seam = EngineExeSeam::set(exe.as_os_str());
    let v = scratch("confirm");
    let cs = open(&v, "confirm");
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let given = json!({
        "mine": [{ "source_uid": "gcal-series:none", "level": "hard" }],
        "window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]",
    });
    let env = commitments_confirm_inner(&cs, "today", &given);
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["result"]["window"], "created");
    assert_eq!(env["result"]["created"], 0);
    assert!(env["result"]["warnings"][0].as_str().unwrap().starts_with("gcal-series:none: "), "{env}");
    assert_eq!(env["state"]["schema"], 1);
    let note = std::fs::read_to_string(v.join("commitments/planning-day.md")).unwrap();
    assert!(note.contains("kind: planning-day") && note.contains("confirmed_at: 2026-08-28"), "{note}");
    let journal: String = std::fs::read_dir(v.join("state/journal")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
    assert!(journal.contains("commitments/planning-day.md") && journal.contains("\"actor\": \"quinn\""), "journaled as the student");
    let tmp = cs.data_dir.join("tmp");
    assert!(std::fs::read_dir(&tmp).map(|d| d.count()).unwrap_or(0) == 0, "the input file is deleted afterwards");
    let bad = commitments_confirm_inner(&cs, "today", &json!({ "window": "[{days: [mon], start: \"15:00\", end: \"14:00\"}]" }));
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("planning day mon"), "{bad}");
    assert_eq!(std::fs::read_to_string(v.join("commitments/planning-day.md")).unwrap(), note, "nothing written");
}
```

- [ ] **Step 2 — run and see them fail.** `cargo build -p knowlu-engine -j 2`, then
  `cargo test -p knowlu --test week -j 2`. Expected: compile error (no `knowlu::week`).

- [ ] **Step 3 — implement.** `app/src/week.rs`:

```rust
//! Phase 2 of the commitment model (spec `docs/specs/2026-09-24-commitment-model-phase2-design.md`
//! §2, §4, §6): the confirm screen's and the *Schedule* view's four commands. Two read in-process
//! (`your_week`, `preview_window`). Two spawn the sibling engine the way
//! `onboarding::discover_coursework` does (`commitment_proposals`, `commitments_confirm`). Nothing
//! here computes: the engine derives every row, validates every window and writes every note.
use std::path::Path;
use serde_json::{json, Value};
use tauri::State;
use knowlu_engine::childproc::NoConsole;
use crate::scheduler::Scheduler;
use crate::state::ConsoleState;

pub fn proposals_argv(vault: &Path) -> Vec<String> {
    vec!["commitments".into(), "--vault".into(), vault.to_string_lossy().into_owned(), "--json".into()]
}

/// `--via dashboard`: the console's human context (`commands::console_ctx`); `--actor` keeps the
/// engine's default, `quinn`.
pub fn confirm_argv(vault: &Path, file: &Path, today: jiff::civil::Date) -> Vec<String> {
    vec![
        "commitments".into(), "--vault".into(), vault.to_string_lossy().into_owned(),
        "--today".into(), today.to_string(),
        "--confirm".into(), file.to_string_lossy().into_owned(),
        "--via".into(), "dashboard".into(),
    ]
}

/// `commitments::overview` in-process: no fetch, no write.
pub fn your_week_inner(cs: &ConsoleState) -> Value {
    let Ok(_g) = cs.lock.lock() else { return json!({ "ok": false, "error": "console lock poisoned", "week": Value::Null }) };
    let today = crate::commands::now_in(cs).date();
    json!({ "ok": true, "error": Value::Null, "week": knowlu_engine::commitments::overview(&cs.vault, today).to_json() })
}

/// The today view under a proposed window (`surface::build_state_preview`), whole (Plan ruling
/// Q9-c). An invalid window is the engine's own message. Writes nothing.
pub fn preview_window_inner(cs: &ConsoleState, window: &str) -> Value {
    let Ok(_g) = cs.lock.lock() else { return json!({ "ok": false, "error": "console lock poisoned", "state": Value::Null }) };
    let now = crate::commands::now_in(cs);
    let built = knowlu_engine::surface::build_state_preview(&cs.vault, knowlu_engine::surface::View::Today, now.date(), &now, cs.seen_at().as_deref(), window);
    match built.and_then(|s| serde_json::to_value(&s).map_err(|e| e.to_string())) {
        Ok(state) => json!({ "ok": true, "error": Value::Null, "state": state }),
        Err(e) => json!({ "ok": false, "error": e, "state": Value::Null }),
    }
}

/// `knowlu-engine commitments --json`: it fetches the calendars and refreshes the series file,
/// then lists the proposals and the uncovered courses. A failure is an empty list with the reason
/// in `error` (Plan ruling Q9-d).
pub fn commitment_proposals_inner(cs: &ConsoleState) -> Value {
    let empty = |why: String| json!({ "ok": true, "error": why, "proposals": [], "uncovered_courses": [], "warnings": [] });
    let exe = match crate::scheduler::engine_exe() { Ok(e) => e, Err(e) => return empty(e) };
    let out = match std::process::Command::new(exe).no_console().args(proposals_argv(&cs.vault)).output() {
        Ok(o) => o,
        Err(e) => return empty(format!("could not run the engine ({e})")),
    };
    match serde_json::from_slice::<Value>(&out.stdout) {
        Ok(v) if v.is_object() => json!({
            "ok": true, "error": Value::Null,
            "proposals": v["proposals"], "uncovered_courses": v["uncovered_courses"], "warnings": v["warnings"],
        }),
        _ => {
            let code = out.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".into());
            empty(format!("commitments exited with status {code}: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }
}

/// `knowlu-engine commitments --confirm <file>` (spec §3). The input file sits in the profile's
/// `tmp\` folder and is deleted afterwards. The state is rebuilt for `view`, like every write's.
pub fn commitments_confirm_inner(cs: &ConsoleState, view: &str, confirm: &Value) -> Value {
    let today = crate::commands::now_in(cs).date();
    let dir = cs.data_dir.join("tmp");
    let stamp = knowlu_engine::journal::now_ts(None).replace([':', '.', '-'], "");
    let file = dir.join(format!("confirm-{}-{stamp}.json", std::process::id()));
    let ran = run_confirm(cs, &dir, &file, confirm, today);
    let _ = std::fs::remove_file(&file);
    let (ok, error, result) = match ran {
        Err(e) => (false, json!(e), Value::Null),
        Ok(out) if out.status.success() => {
            cs.note_write();
            match serde_json::from_slice::<Value>(&out.stdout) {
                Ok(v) => (true, Value::Null, v),
                Err(e) => (false, json!(format!("commitments --confirm printed no report ({e})")), Value::Null),
            }
        }
        Ok(out) => {
            let why = String::from_utf8_lossy(&out.stderr).trim().trim_start_matches("knowlu-engine: ").to_string();
            (false, json!(why), Value::Null)
        }
    };
    let state = crate::commands::state_inner(cs, view).map(|env| env["state"].clone()).unwrap_or(Value::Null);
    json!({ "ok": ok, "error": error, "result": result, "state": state })
}

/// Writes the input and runs the child under `vault_io` (Plan ruling Q9-a).
fn run_confirm(cs: &ConsoleState, dir: &Path, file: &Path, confirm: &Value, today: jiff::civil::Date) -> Result<std::process::Output, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(file, serde_json::to_vec(confirm).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let exe = crate::scheduler::engine_exe()?;
    let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
    std::process::Command::new(exe).no_console().args(confirm_argv(&cs.vault, file, today)).output().map_err(|e| e.to_string())
}

#[tauri::command(async)] pub fn commitment_proposals(cs: State<'_, ConsoleState>) -> Value { commitment_proposals_inner(&cs) }
#[tauri::command(async)] pub fn commitments_confirm(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, confirm: Value) -> Value { let mut env = commitments_confirm_inner(&cs, &view, &confirm); let _ = crate::commands::attach_scheduler(&mut env, &sch); env }
#[tauri::command] pub fn your_week(cs: State<'_, ConsoleState>) -> Value { your_week_inner(&cs) }
#[tauri::command(async)] pub fn preview_window(cs: State<'_, ConsoleState>, window: String) -> Value { preview_window_inner(&cs, &window) }
```

  `app/src/lib.rs`: add `pub mod week;` after `pub mod updates;`. `app/src/main.rs`: line 3's
  `use knowlu::{…, tray};` becomes `use knowlu::{…, tray, week};`. The console list gains
  `week::commitment_proposals, week::commitments_confirm, week::your_week, week::preview_window`
  after `commands::answer_card`.

  `app/tests/no_console.rs` counts `Command::new` against `.no_console()` in each source file:
  `week.rs` has two of each.

- [ ] **Step 4 — run.** `cargo build -p knowlu-engine -j 2`; `cargo test -p knowlu --test week -j 2`
  (5 pass); `cargo test -p knowlu --test no_console --test static_assets --test commands -j 2`.
  Then the count, by script (no backslashes, so no heredoc trouble):
  `python -c "t=open('app/src/main.rs',encoding='utf-8').read(); L=[set(x.strip() for x in s.split(']')[0].split(',') if x.strip()) for s in t.split('generate_handler![')[1:]]; print(len(L[1]), len(L[0]), len(L[0]|L[1]))"`
  prints `47 29 66` (console, wizard, distinct).

- [ ] **Step 5 — commit.** `git add app/src/week.rs app/src/lib.rs app/src/main.rs app/tests/week.rs`; message:

```
feat(app): week.rs — your_week, preview_window, commitment_proposals, commitments_confirm (phase 2, §6)

Two in-process reads (commitments::overview, surface::build_state_preview) and
two engine spawns (commitments --json, commitments --confirm with a temp file,
under vault_io). Console commands: 47; wizard: 29.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q10 — the moved line, the *Schedule* view and the window editor (D6, D7, D8, §4)

**Files.**
- `app/static/index.html`: the nav (lines 13–25), `#main-today`'s headline (line 46), a new
  `#main-schedule` section between `#main-issues` (lines 66–68) and `#main-runs` (line 69).
- `app/static/console.js`: `renderNav` (38–48), `paint` (534–560), `VIEW_RENDERERS` (523),
  `route` (569–577), `poll` (680–697), every `view: current.view` but `ui_event`'s (lines 516,
  681, 826, 943, 948, 972, 1069, 1079, 1097, 1098, 1241, 1291), the bind calls (1129–1130); new
  functions after `renderIssuesView` (ends at line 512).
- `app/static/console.css`: a new block at the end.
- `app/tests/static_assets.rs`: one new test at the end.

**Interfaces.**
- Consumes `your_week` → `{ok, error, week}`, `preview_window({window})` → `{ok, error, state}`,
  `set_fields({view, id, fields: {kind|level}})`, and
  `commitments_confirm({view, confirm})` → `{ok, error, result, state}`.
- Produces `stateView()`, `renderMoved(state)`, `renderScheduleView()`, `windowEditorHtml(byDay)`,
  `windowSequence(host)`, `showWindowError(host, message)`, `bindWindowEditor(host, onEdit)`,
  `schedulePreview()`, `runPreview()`, `levelButtons(level)`, `saveCommitment(id, fields)`,
  `confirmWeek(payload)`, `bindScheduleView()`; constants `DAYS`, `DAY_NAMES`, `KIND_NAMES`,
  `LEVELS`, `PREVIEW_MS = 400`. Q11 reuses the editor functions, `levelButtons` and `confirmWeek`.

**Plan ruling Q10-a:** `schedule` is a page view, not a read-model view. `surface::View::parse`
refuses it (anatomy §2, R-P4a-4), so every call that builds state sends `view: stateView()`, which
is `"today"` on the *Schedule* view and `current.view` elsewhere. That covers `poll` and every
mutation, the rail's deck included. `ui_event` keeps `current.view`, since `schedule` is a valid
event token. *Why:* otherwise a write made from the *Schedule* view would land and then report
`unknown view "schedule"`.
**Plan ruling Q10-b:** the nav link reads **Schedule** and the view's heading reads **Your week**
(D6). *Why:* *This week* already exists, and a second "week" link would be ambiguous.

D8: no remove control is built. The test pins the absence of any `data-remove` in `console.js`.

- [ ] **Step 1 — failing test.** At the end of `app/tests/static_assets.rs`:

```rust
/// Phase 2 of the commitment model (spec D6, D7, D8, §4): the Schedule view headed "Your week",
/// the window editor with its 400 ms preview, the today view's moved line, and `schedule` never
/// reaching the read model as a view name.
#[test]
fn the_schedule_view_the_window_editor_and_the_moved_line_are_there() {
    let html = read("index.html");
    assert!(html.contains("<a href=\"#schedule\" data-view=\"schedule\"><span class=\"dot\"></span>Schedule<span class=\"ct\"></span></a>"), "D6: a Schedule link");
    assert!(html.contains("<section id=\"main-schedule\" hidden>") && html.contains("<h2>Your week</h2>"), "D6: headed Your week");
    for id in ["sched-list", "sched-oh", "sched-window", "sched-save", "sched-say", "sched-moved", "sched-items", "moved"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    let js = read("console.js");
    assert!(js.contains("schedule: renderScheduleView"), "in VIEW_RENDERERS");
    for f in ["stateView", "renderMoved", "renderScheduleView", "windowEditorHtml", "windowSequence", "bindWindowEditor", "runPreview", "confirmWeek", "bindScheduleView"] {
        assert!(js.contains(&format!("function {f}(")), "missing function {f}");
    }
    assert!(js.contains("var PREVIEW_MS = 400;") && js.contains("setTimeout(runPreview, PREVIEW_MS)"), "§4: the preview is debounced 400 ms");
    assert!(js.contains("invoke(\"preview_window\", { window: seq })"));
    assert!(js.contains("\"No change to today's plan\""));
    assert!(js.contains("items.slice(0, 5)"), "§4: the first five items of the previewed day");
    assert!(js.contains("el.textContent = m ? m.text : \"\";"), "D7: the today view prints moved.text");
    assert!(js.contains("data-same-as-monday"), "§4: the same-as-Monday shortcut");
    assert!(js.contains("invoke(\"commitments_confirm\", { view: stateView(), confirm: payload })"));
    // Q10-a: `schedule` never reaches `surface::View::parse`. ui_event is the one call that names
    // the page's own view.
    assert_eq!(js.matches("view: current.view").count(), 1, "only ui_event sends current.view");
    assert!(js.contains("{ action: action, view: current.view,"), "…and it is ui_event");
    assert!(js.contains("runs: 1, schedule: 1 }"), "route() accepts schedule");
    assert!(!js.contains("data-remove"), "D8: no remove control in phase 2");
    let css = read("console.css");
    assert!(css.contains(".row.sched { grid-template-columns: minmax(0,1fr) auto; }"), "a Schedule row restates its tracks");
    assert!(css.contains(".moved[hidden] { display: none; }"));
}
```

- [ ] **Step 2 — run and see it fail.** `cargo test -p knowlu --test static_assets -j 2 -- the_schedule_view`
  Expected: fails on the first assertion (no Schedule link).

- [ ] **Step 3 — implement.** `index.html`: after the `#all` link (line 18):

```html
      <a href="#schedule" data-view="schedule"><span class="dot"></span>Schedule<span class="ct"></span></a>
```

  after `<h1 id="headline"></h1>` (line 46):

```html
      <p class="moved" id="moved" hidden></p>
```

  and between `#main-issues`' closing `</section>` (line 68) and `#main-runs` (line 69):

```html
    <section id="main-schedule" hidden>
      <div class="sec"><div class="sec-hd"><h2>Your week</h2><span class="n" id="sched-n"></span></div><div id="sched-list"></div></div>
      <div class="sec"><div class="sec-hd"><h2>Office hours</h2></div><div id="sched-oh"></div></div>
      <div class="sec"><div class="sec-hd"><h2>Your day</h2></div>
        <div class="wined" id="sched-window"></div>
        <div class="wined-foot"><button class="b pri y" type="button" id="sched-save">Save</button><span class="hint" id="sched-say"></span></div>
        <div class="preview"><p id="sched-moved"></p><ol id="sched-items"></ol></div>
      </div>
    </section>
```

  `console.js`. In `renderNav`, inside the `forEach` callback, right after
  `var ct = a.querySelector(".ct");`, add
  `if (c === undefined) { ct.textContent = ""; return; }   // Schedule carries no count`.
  `VIEW_RENDERERS` (line 523) becomes
  `var VIEW_RENDERERS = { runs: renderRunsView, decisions: renderDecisionsView, "good-to-know": renderGoodToKnowView, issues: renderIssuesView, schedule: renderScheduleView };`.
  In `paint`, after `EL("main-issues").hidden = current.view !== "issues";` add
  `EL("main-schedule").hidden = current.view !== "schedule";`. Its today line becomes
  `if (current.view === "today") { renderVerdict(state); renderMeter(state); renderMoved(state); }`.
  In `route`, the whitelist object ends `…, issues: 1, runs: 1, schedule: 1 }`. Replace every
  `view: current.view` except the one in `ev()`'s `ui_event` call (line 734) with
  `view: stateView()`. That is twelve places, `poll`'s `invoke("state", …)` among them. After
  `bindDecisionsView();` (line 1130) add `bindScheduleView();`.

  New functions, after `renderIssuesView` and `resolveIssue` (before the `VIEW_RENDERERS` comment
  at line 521):

```js
  // ---- Phase 2 of the commitment model (spec §4, D6–D8): the Schedule view ("Your week"), the
  // window editor and the moved line. Rows come from `your_week` (the engine's
  // `commitments::overview`). A kind or level goes through `set_fields`; the window and an
  // office-hours Add go through `commitments_confirm`. Nothing here computes a time: the flow
  // sequence is string assembly from the pickers, and the engine validates it.
  var DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  var DAY_NAMES = { mon: "Mon", tue: "Tue", wed: "Wed", thu: "Thu", fri: "Fri", sat: "Sat", sun: "Sun" };
  var KIND_NAMES = [["class", "Class"], ["lab", "Lab"], ["work", "Work"], ["club", "Club"], ["meeting", "Meeting"], ["office-hours", "Office hours"]];
  var LEVELS = [["hard", "Must keep"], ["soft", "Usually"], ["optional", "Optional"]];
  var PREVIEW_MS = 400;
  var previewTimer = null;

  // Q10-a: `schedule` is the page's view, not the read model's — the state it paints is today's.
  function stateView() { return current.view === "schedule" ? "today" : current.view; }

  // D7: the today view says what moved, when the read model carries it (§6.4).
  function renderMoved(state) {
    var m = state.moved, el = EL("moved");
    el.hidden = !m;
    el.textContent = m ? m.text : "";
  }

  function levelButtons(level) {
    return '<span class="lvl" role="group" aria-label="How much it binds">' + LEVELS.map(function (l) {
      return '<button class="b" type="button" data-level-set="' + l[0] + '" aria-pressed="' + (l[0] === level) + '">' + h(l[1]) + "</button>";
    }).join("") + "</span>";
  }

  // One row per weekday, Mon to Sun, each a start and an end picker; `byDay` is {mon: {start, end}}.
  function windowEditorHtml(byDay) {
    return DAYS.map(function (d) {
      var w = byDay[d] || {};
      return '<div class="wrow" data-day="' + d + '"><span class="wday">' + DAY_NAMES[d] + "</span>" +
        '<input type="time" data-part="start" aria-label="' + DAY_NAMES[d] + ' start" value="' + h(w.start || "") + '">' +
        '<input type="time" data-part="end" aria-label="' + DAY_NAMES[d] + ' end" value="' + h(w.end || "") + '">' +
        (d === "mon" ? '<button class="lnk" type="button" data-same-as-monday>same as Monday for Tue&ndash;Fri</button>' : "") +
        '<span class="werr" data-werr="' + d + '"></span></div>';
    }).join("");
  }

  // The flow sequence from the pickers. Days with the same start and end share one entry; a day
  // with an empty picker is left out, so it keeps week_template.yaml's hours. Null when every
  // row is blank: no window is sent (Plan ruling Q4-b).
  function windowSequence(host) {
    var groups = [], byKey = {};
    DAYS.forEach(function (d) {
      var row = host.querySelector('.wrow[data-day="' + d + '"]'); if (!row) { return; }
      var s = row.querySelector('[data-part="start"]').value, e = row.querySelector('[data-part="end"]').value;
      if (!s || !e) { return; }
      var k = s + "-" + e;
      if (!byKey[k]) { byKey[k] = { days: [], start: s, end: e }; groups.push(byKey[k]); }
      byKey[k].days.push(d);
    });
    if (!groups.length) { return null; }
    return "[" + groups.map(function (g) { return "{days: [" + g.days.join(", ") + '], start: "' + g.start + '", end: "' + g.end + '"}'; }).join(", ") + "]";
  }

  // The engine's own message, under the row it names ("planning day fri: …"), else under Monday.
  function showWindowError(host, message) {
    host.querySelectorAll("[data-werr]").forEach(function (e) { e.textContent = ""; });
    if (!message) { return; }
    var m = /planning day (\w+)/.exec(message);
    var slot = (m && host.querySelector('[data-werr="' + m[1] + '"]')) || host.querySelector('[data-werr="mon"]');
    if (slot) { slot.textContent = message; }
  }

  function bindWindowEditor(host, onEdit) {
    host.addEventListener("click", function (e) {
      if (!e.target.closest("[data-same-as-monday]")) { return; }
      var mon = host.querySelector('.wrow[data-day="mon"]');
      ["tue", "wed", "thu", "fri"].forEach(function (d) {
        var row = host.querySelector('.wrow[data-day="' + d + '"]');
        row.querySelector('[data-part="start"]').value = mon.querySelector('[data-part="start"]').value;
        row.querySelector('[data-part="end"]').value = mon.querySelector('[data-part="end"]').value;
      });
      onEdit();
    });
    host.addEventListener("input", onEdit);
  }

  function schedulePreview() {
    if (previewTimer) { clearTimeout(previewTimer); }
    previewTimer = setTimeout(runPreview, PREVIEW_MS);
  }

  // §4: beside the editor, the preview's moved line (or "No change to today's plan") and the
  // first five items of the previewed day, in order.
  function runPreview() {
    var host = EL("sched-window"), seq = windowSequence(host);
    if (!seq) { EL("sched-moved").textContent = "No change to today's plan"; EL("sched-items").innerHTML = ""; return; }
    invoke("preview_window", { window: seq }).then(function (r) {
      if (!r.ok) { showWindowError(host, r.error); return; }
      showWindowError(host, null);
      var s = r.state, items = [];
      EL("sched-moved").textContent = s.moved ? s.moved.text : "No change to today's plan";
      s.the_day.blocks.forEach(function (b) { b.takes.forEach(function (t) { items.push(t.title); }); });
      EL("sched-items").innerHTML = items.slice(0, 5).map(function (t) { return "<li>" + h(t) + "</li>"; }).join("");
    }).catch(function () {});
  }
```

```js
  // The view's renderer (VIEW_RENDERERS.schedule). Its data is `your_week`, not the state `paint`
  // passes it. The editor is rebuilt only while nobody is typing in it.
  function renderScheduleView() {
    return invoke("your_week", {}).then(function (r) {
      if (!r || !r.ok || !r.week) { EL("sched-n").textContent = (r && r.error) || ""; return; }
      var w = r.week;
      EL("sched-n").textContent = w.commitments.length + (w.commitments.length === 1 ? " commitment" : " commitments");
      EL("sched-list").innerHTML = w.commitments.map(function (c) {
        var known = KIND_NAMES.some(function (k) { return k[0] === c.kind; });
        var kinds = (known ? "" : '<option value="' + h(c.kind) + '" selected>' + h(c.kind) + "</option>") + KIND_NAMES.map(function (k) {
          return '<option value="' + k[0] + '"' + (k[0] === c.kind ? " selected" : "") + ">" + h(k[1]) + "</option>";
        }).join("");
        return '<div class="row sched" data-id="' + h(c.id) + '" data-kind="commitment"><div class="ttl"><span class="a">' + h(c.title) +
          '</span><span class="meta">' + h(c.when || "") + (c.where ? " · " + h(c.where) : "") + "</span></div>" +
          '<div class="acts"><select data-kind-set aria-label="Kind">' + kinds + "</select>" + levelButtons(c.level) + "</div></div>";
      }).join("") + w.uncovered_courses.map(function (u) {
        return '<div class="row sched uncovered"><div class="ttl"><span class="a">' + h(u.title) + '</span><span class="meta">Knowlu will ask when it meets</span></div></div>';
      }).join("") + (w.commitments.length || w.uncovered_courses.length ? "" : '<div class="empty">Nothing confirmed yet.</div>');
      EL("sched-oh").innerHTML = w.office_hours.map(function (p) {
        return '<div class="row sched oh"><div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") +
          '</span></div><div class="acts"><button class="b" type="button" data-oh-add="' + h(p.source_uid) + '">Add</button></div></div>';
      }).join("") || '<div class="empty">No office hours found on your calendar.</div>';
      var host = EL("sched-window"), byDay = {};
      w.window.forEach(function (d) { byDay[d.day] = d; });
      if (!host.contains(document.activeElement)) { host.innerHTML = windowEditorHtml(byDay); }
      schedulePreview();
    }).catch(function () {});
  }

  // No explicit renderScheduleView() here: applyEnvelope paints, and paint already runs the
  // Schedule view's renderer (review finding 9 — one your_week round trip per click, not two).
  function saveCommitment(id, fields) {
    return invoke("set_fields", { view: stateView(), id: id, fields: fields }).then(function (env) {
      applyEnvelope(env, function (m) { showRefusal(null, m, id); });
    }).catch(function () {});
  }

  // Every `commitments_confirm` goes through here: the Schedule view's Save and Add, and Q11's
  // Finish. A returned state paints like any write's.
  function confirmWeek(payload) {
    return invoke("commitments_confirm", { view: stateView(), confirm: payload }).then(function (env) {
      if (env.state) { current.pendingOrder = null; paint(env.state, true); }
      return env;
    });
  }

  function bindScheduleView() {
    var list = EL("sched-list");
    list.addEventListener("click", function (e) {
      // A row's controls never reach the document handler, which would open the drawer.
      if (e.target.closest(".row.sched .acts")) { e.stopPropagation(); }
      var b = e.target.closest("[data-level-set]"); if (!b) { return; }
      saveCommitment(b.closest(".row.sched").getAttribute("data-id"), { level: b.getAttribute("data-level-set") });
    });
    list.addEventListener("change", function (e) {
      var sel = e.target.closest("[data-kind-set]"); if (!sel) { return; }
      saveCommitment(sel.closest(".row.sched").getAttribute("data-id"), { kind: sel.value });
    });
    EL("sched-oh").addEventListener("click", function (e) {
      var b = e.target.closest("[data-oh-add]"); if (!b) { return; }
      b.disabled = true;
      // §4: office hours default to optional (§2.2).
      confirmWeek({ mine: [{ source_uid: b.getAttribute("data-oh-add"), level: "optional" }] }).then(renderScheduleView).catch(function () { b.disabled = false; });
    });
    bindWindowEditor(EL("sched-window"), schedulePreview);
    EL("sched-save").addEventListener("click", function () {
      var host = EL("sched-window"), seq = windowSequence(host);
      if (!seq) { return; }
      confirmWeek({ window: seq }).then(function (env) {
        if (!env.ok) { showWindowError(host, env.error); return; }
        showWindowError(host, null);
        EL("sched-say").textContent = env.result && env.result.window === "unchanged" ? "No change" : "Saved";
        renderScheduleView();
      }).catch(function () {});
    });
  }
```

  `console.css`, at the end:

```css
/* ---- Commitment model phase 2: the moved line, the Schedule view, the window editor ---- */
.moved { margin: 0 0 var(--s4); color: var(--acc); font-size: 13px; }
.moved[hidden] { display: none; }
.row.sched { grid-template-columns: minmax(0,1fr) auto; }
.row.sched .acts select, .wrow input[type=time] { background: var(--s2c); color: var(--t1); border: 1px solid var(--hair-2); border-radius: var(--r1); padding: 2px var(--s2); }
.wrow input[type=time] { font-family: var(--mono); }
.lvl { display: inline-flex; gap: 2px; margin-left: var(--s2); }
.lvl .b[aria-pressed="true"] { background: var(--acc-dim); color: var(--t1); border-color: var(--acc); }
.wrow { display: grid; grid-template-columns: 44px 120px 120px auto; gap: var(--s2); align-items: center; padding: 2px 0; }
.werr { grid-column: 2 / -1; color: var(--crit); font-size: 12px; }
.werr:empty { display: none; }
.wined-foot { display: flex; gap: var(--s3); align-items: center; margin-top: var(--s3); }
.preview { margin-top: var(--s4); color: var(--t2); font-size: 13px; }
```

- [ ] **Step 4 — run.** `cargo test -p knowlu --test static_assets -j 2` (the new test and every
  existing one, notably `the_good_to_know_and_issues_views_are_real_and_every_observed_row_names_its_kind`:
  the Schedule row carries both `data-id` and `data-kind`). Then the headless check stays green:
  `.wv\Scripts\python scripts/wizard-check.py` prints `ok`.

- [ ] **Step 5 — commit.** `git add app/static/index.html app/static/console.js app/static/console.css app/tests/static_assets.rs`; message:

```
feat(app): the Schedule view, the window editor and the moved line (phase 2, D6, D7, §4)

"Your week" lists confirmed commitments with a kind select and a level control
(set_fields), office hours with Add (optional), and courses Knowlu will ask
about; the planning-day editor previews through preview_window, debounced
400 ms, and saves through commitments_confirm. The today view prints
moved.text. `schedule` never reaches the read model: stateView() sends today.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q11 — the confirm screen, the ask form, the deck, and the behavioural check (D1, D3, §2, §5)

**Files.**
- `app/static/index.html`: a new `#week-setup` section between `.app`'s closing `</div>` (line 79)
  and `<section class="picker" …>` (line 80).
- `app/static/console.js`: `renderDeck`'s card template (line 302); `renderDecisionsView`'s map
  (lines 453–472); `bindDeck` (986–998); `bindDecisionsView` (1000–1012); `bootConsole`
  (1489–1508); new functions after Q10's `bindScheduleView`.
- `app/static/console.css`: a new block at the end.
- `app/tests/static_assets.rs`: one new test.
- `scripts/wizard-check.py`: `WEEK_FAKE`, `check_week_setup`, two more pages in `main()`, and the
  module docstring.

**Interfaces.**
- Consumes `your_week`, `commitment_proposals` → `{ok, error, proposals: [proposal_value…],
  uncovered_courses, warnings}`, `commitments_confirm`, and
  `answer_card({view, id, meets})` → `{ok, error, state, decision}`.
- Produces `checkWeekSetup()`, `openWeekSetup(week)`, `setupRow(p)`, `finishWeekSetup(btn)`,
  `closeWeekSetup()`, `askRow(c, tomorrow)`, `askTimeRow()`, `answerAsk(row)`.
- The Finish payload: `{"mine": [{"source_uid", "level"}], "not_mine": [...], "window": "<flow sequence>"}`.
  `window` is always present: an all-blank *Your day* is refused on the page before any call
  (review finding 2, spec D3).

**Plan ruling Q11-a:** the screen's *Your day* editor is Q10's component (`windowEditorHtml`,
`windowSequence`, `bindWindowEditor`, `showWindowError`) without the preview. *Why:* on the first
day there is no plan yet to diff against, since the first slot is still running. The review
accepted this (finding 7), and spec §4 is amended: "the same pickers; the preview runs only in
the *Schedule* view".
**Plan ruling Q11-b:** in the rail's deck, a `commitment-ask` card shows **Answer…** in place of
Approve, and it routes to `#decisions`. Reject and Snooze stay. *Why:* the parent's §5.3 says
"a card the console cannot answer would teach the student to reject", and the spec only builds
the form in the Decisions view. Approve with no answer would only send the card back to pending.
**Plan ruling Q11-c:** pressing the pressed *Mine* or *Not mine* again returns the row to
unanswered. *Why:* the spec has three states and two buttons, so there must be a way back to
"unanswered". The alternative, a third button, would add a control the spec does not name.
**Plan ruling Q11-d:** `checkWeekSetup()` runs once per console launch, from `bootConsole`, after
`route()`. "Not now" sets a session flag. *Why:* D3's state is day 1 plus no planning-day note,
and it can only change by Finish or by the date. The next launch asks again on the same day,
which is the intended "for the rest of that console session".

- [ ] **Step 1 — failing tests.** At the end of `app/tests/static_assets.rs`:

```rust
/// Phase 2 of the commitment model (spec D1, D3, §2, §5): the confirm screen over the first-run
/// view — no typed text anywhere in it — and the commitment-ask card's form in the Decisions view,
/// with the deck sending that card there.
#[test]
fn the_confirm_screen_and_the_ask_form_are_there() {
    let html = read("index.html");
    let start = html.find("<section class=\"week-setup\" id=\"week-setup\" hidden").expect("#week-setup");
    let screen = &html[start..start + html[start..].find("</section>").unwrap()];
    assert!(html.find("id=\"week-setup\"").unwrap() > html.find("<div class=\"app\">").unwrap() && html.find("id=\"week-setup\"").unwrap() < html.find("id=\"picker\"").unwrap(), "outside .app, so the first-run view does not hide it");
    for id in ["ws-status", "ws-classes", "ws-class-rows", "ws-week", "ws-week-rows", "ws-oh", "ws-oh-rows", "ws-window", "ws-later", "ws-finish"] {
        assert!(screen.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    assert!(screen.contains("When do your classes meet?") && screen.contains("Reading your calendar"));
    assert!(screen.contains("Rows you leave blank will come back as questions over the next few days."));
    assert!(!screen.contains("type=\"text\"") && !screen.contains("<textarea"), "constraint 8: nothing typed");
    let js = read("console.js");
    for f in ["checkWeekSetup", "openWeekSetup", "setupRow", "finishWeekSetup", "closeWeekSetup", "askRow", "askTimeRow", "answerAsk"] {
        assert!(js.contains(&format!("function {f}(")), "missing function {f}");
    }
    let row = js.split("function setupRow(").nth(1).unwrap().split("\n  function ").next().unwrap();
    assert!(!row.contains("type=\"text\""), "a setup row takes no typing");
    assert!(row.contains("{ class: 1, lab: 1, work: 1 }[p.kind] ? \"mine\" : \"\""), "§2: class, lab and work start at Mine");
    assert!(js.contains("invoke(\"commitment_proposals\", {})") && js.contains("invoke(\"your_week\", {})"));
    assert!(js.contains("weekSetup.dismissed = true"), "D3: Not now hides it for the session");
    assert!(js.contains("no class times found; Knowlu will ask this week"));
    let view = js.split("function renderDecisionsView(").nth(1).unwrap().split("\n  function ").next().unwrap();
    assert!(view.contains("if (c.kind === \"commitment-ask\") { return askRow(c, tomorrow); }"), "§5: the per-kind branch");
    let ask = js.split("function askRow(").nth(1).unwrap().split("\n  function ").next().unwrap();
    for piece in ["data-ask-save", "Save times", "No set times", "data-verdict=\"rejected\"", "data-verdict=\"snoozed\"", "data-ask-more", "add another time"] {
        assert!(ask.contains(piece), "the ask form lacks {piece}");
    }
    assert!(js.contains("function askTimeRow()") && js.contains("data-ask-day"));
    assert!(js.contains("invoke(\"answer_card\", { view: stateView(), id: id, meets: meets })"));
    assert!(js.contains("data-answer-in"), "Q11-b: the deck sends an ask to Decisions");
    assert!(read("console.css").contains(".week-setup[hidden] { display: none; }"));
}
```

  In `scripts/wizard-check.py`, after `CONSOLE_FAKE`:

```python
# The confirm screen (commitment model phase 2, spec D1/D3/§2): a first-day vault whose `your_week`
# says `setup`, with a slot in flight, so the screen opens over the first-run view. The proposals
# are the engine's `commitments --json` rows, invented: a class, a club, office hours and the window
# proposal (Mon–Fri 7:30am–11pm), plus BUI 100 with no class row.
WEEK_FAKE = r"""
window.__CALLS = [];
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args]);
  if (cmd === 'launch_state') { return Promise.resolve({ ok: true, mode: 'console', profiles: [] }); }
  if (cmd === 'state') { return Promise.resolve({ ok: true, error: null, state: JSON.parse(JSON.stringify(window.__STATE)),
      first_run: { running: true, current: 'coursework', steps: [] } }); }
  if (cmd === 'your_week') { return Promise.resolve({ ok: true, error: null, week: { setup: true, commitments: [], office_hours: [],
      uncovered_courses: [{ slug: 'bui-100', title: 'BUI 100' }], window: [], warnings: [] } }); }
  if (cmd === 'commitment_proposals') { return Promise.resolve({ ok: true, error: null, warnings: [],
      uncovered_courses: [{ slug: 'bui-100', title: 'BUI 100' }],
      proposals: [
        { kind: 'class', level: 'hard', title: 'CS 100', course: 'cs-100', when: 'Mon/Wed/Fri 12–12:50pm', where: 'Room 101',
          source_uid: 'gcal-series:cs100', window: false, meets: [{ days: ['mon', 'wed', 'fri'], start: '12:00', end: '12:50' }] },
        { kind: 'planning-day', level: 'optional', title: 'Your day', course: null, when: 'Mon–Fri 7:30am–11pm', where: null,
          source_uid: 'window:gcal-series:wake', window: true, meets: [{ days: ['mon', 'tue', 'wed', 'thu', 'fri'], start: '07:30', end: '23:00' }] },
        { kind: 'club', level: 'soft', title: 'Chess Club', course: null, when: 'Wed 6–7pm', where: null,
          source_uid: 'gcal-series:chess', window: false, meets: [{ days: ['wed'], start: '18:00', end: '19:00' }] },
        { kind: 'office-hours', level: 'optional', title: 'CS 100 Office Hours', course: 'cs-100', when: 'Thu 3–4pm', where: null,
          source_uid: 'gcal-series:oh', window: false, meets: [{ days: ['thu'], start: '15:00', end: '16:00' }] }] }); }
  if (cmd === 'commitments_confirm') { return Promise.resolve({ ok: true, error: null, state: null,
      result: { created: 1, declined: 0, window: 'created', warnings: [] } }); }
  if (cmd === 'account_status') { return Promise.resolve({ ok: true, needs_account: false }); }
  return Promise.resolve({ ok: true, error: null });
} } };
"""
```

  and before `def main()`:

```python
WINDOW_SENT = '[{days: [mon, tue, wed, thu, fri], start: "07:30", end: "23:00"}]'


def check_week_setup(page, errors, finish) -> list:
    """The confirm screen on a first-day vault (spec §2): open over the first-run view, the class
    preset to Mine and nothing else answered, BUI 100 named, no typed input anywhere. Then either
    Not now (nothing sent) or Finish (exactly one `commitments_confirm` with the class, no
    declines, and the window pre-filled from the window proposal)."""
    bad = []
    if not page.is_visible("#week-setup"):
        return [f"the confirm screen did not open on a first-day vault (calls: {names(page)!r})"] + [f"page error: {e}" for e in errors]
    rows = dict(page.evaluate("""Array.from(document.querySelectorAll('#week-setup .wsrow[data-key]'))
        .map(r => [r.getAttribute('data-key'), r.getAttribute('data-answer')])"""))
    if rows.get("gcal-series:cs100") != "mine": bad.append(f"the class row is not preset to Mine: {rows!r}")
    for key in ("gcal-series:chess", "gcal-series:oh"):
        if rows.get(key) != "": bad.append(f"{key} is not left unanswered: {rows!r}")
    if any(k.startswith("window:") for k in rows): bad.append(f"the window proposal was listed as a row: {rows!r}")
    if "no class times found" not in page.inner_text("#ws-classes"): bad.append("BUI 100's missing class row is not named")
    if page.evaluate("document.querySelectorAll('#week-setup input[type=text], #week-setup textarea').length"):
        bad.append("the confirm screen accepts typed text")
    if page.is_visible("#week-setup .wsrow[data-key='gcal-series:chess'] .lvl"): bad.append("an unanswered row shows the level control")
    if finish:
        # Review finding 2: with every Your day picker cleared, Finish writes nothing and says why.
        saved = page.evaluate("""Array.from(document.querySelectorAll('#ws-window input[type=time]')).map(i => i.value)""")
        page.evaluate("document.querySelectorAll('#ws-window input[type=time]').forEach(i => { i.value = ''; })")
        page.click("#ws-finish"); page.wait_for_timeout(200)
        if "commitments_confirm" in names(page): bad.append("Finish with no hours set sent a write")
        if "at least one day" not in page.inner_text("#ws-window"): bad.append("Finish with no hours set did not say why")
        page.evaluate("(vals => document.querySelectorAll('#ws-window input[type=time]').forEach((i, n) => { i.value = vals[n]; }))", saved)
        page.click("#ws-finish"); page.wait_for_timeout(300)
        sent = page.evaluate("window.__CALLS.filter(c => c[0] === 'commitments_confirm').map(c => c[1])")
        if len(sent) != 1:
            bad.append(f"Finish sent {len(sent)} commitments_confirm calls")
        else:
            c = sent[0]["confirm"]
            if c.get("mine") != [{"source_uid": "gcal-series:cs100", "level": "hard"}]: bad.append(f"Finish sent mine {c.get('mine')!r}")
            if c.get("not_mine") != []: bad.append(f"Finish declined {c.get('not_mine')!r}; blank rows must stay blank")
            if c.get("window") != WINDOW_SENT: bad.append(f"Finish sent the window {c.get('window')!r}")
        if page.is_visible("#week-setup"): bad.append("Finish did not close the screen")
    else:
        page.click("#ws-later"); page.wait_for_timeout(300)
        if page.is_visible("#week-setup"): bad.append("Not now did not close the screen")
        if "commitments_confirm" in names(page): bad.append("Not now sent a write")
    for e in errors: bad.append(f"confirm-screen page error: {e}")
    return bad
```

  In `main()`, after `bad += check_first_run(console, errors)`:

```python
                for finish in (False, True):
                    week = browser.new_context(viewport={"width": 1280, "height": 860}).new_page()
                    werrors = []
                    week.on("pageerror", lambda e, sink=werrors: sink.append(str(e)))
                    week.add_init_script("window.__STATE = " + STATE_FIXTURE.read_text(encoding="utf-8") + ";\n" + WEEK_FAKE)
                    week.goto(url); week.wait_for_timeout(600)
                    bad += check_week_setup(week, werrors, finish)
```

  The module docstring gains a third paragraph, after the first-run one: "A third and fourth page
  boot the console over a first-day vault (commitment model phase 2): the confirm screen opens
  over the first-run view with class rows preset to *Mine*, Not now sends nothing, and Finish
  sends one `commitments_confirm` holding the expected keys."

- [ ] **Step 2 — run and see them fail.** `cargo test -p knowlu --test static_assets -j 2 -- the_confirm_screen`
  (fails: no `#week-setup`); `.wv\Scripts\python scripts/wizard-check.py` (prints
  `FAIL: the confirm screen did not open …` twice).

- [ ] **Step 3 — implement.** `index.html`, between line 79 (`</div>`) and line 80:

```html
<section class="week-setup" id="week-setup" hidden aria-labelledby="ws-title">
  <h1 id="ws-title">When do your classes meet?</h1>
  <p class="lede" id="ws-status">Reading your calendar&hellip;</p>
  <div class="ws-group" id="ws-classes"><h2>Your classes</h2><div id="ws-class-rows"></div></div>
  <div class="ws-group" id="ws-week"><h2>Your week</h2><div id="ws-week-rows"></div></div>
  <div class="ws-group" id="ws-oh"><h2>Office hours</h2><div id="ws-oh-rows"></div></div>
  <div class="ws-group"><h2>Your day</h2><div class="wined" id="ws-window"></div></div>
  <div class="ws-nav"><button class="b" type="button" id="ws-later">Not now</button><button class="b pri y" type="button" id="ws-finish">Finish</button></div>
  <p class="hint">Rows you leave blank will come back as questions over the next few days.</p>
</section>
```

  (The class is `ws-nav`, not `wiz-nav`, so the wizard's nav tests never see these buttons.)
  The static test searches for the literal `Reading your calendar`, which `&hellip;` does not
  split.

  `console.js`, after `bindScheduleView`:

```js
  // ---- Phase 2 (spec §2, D1, D3): the confirm screen, over the first-run view. `your_week` says
  // `setup` while the vault is on its first day and has no planning-day note. Not now hides the
  // screen for this console session; Finish writes the planning-day note, so it never returns.
  var weekSetup = { dismissed: false, open: false };

  function checkWeekSetup() {
    if (weekSetup.dismissed || weekSetup.open) { return; }
    invoke("your_week", {}).then(function (r) {
      if (r && r.ok && r.week && r.week.setup && !weekSetup.dismissed) { openWeekSetup(r.week); }
    }).catch(function () {});
  }

  // One row: title, the §5.2 label, `where` in small type; Mine / Not mine; the level control,
  // shown by CSS only on a Mine row. Nothing typed.
  function setupRow(p) {
    var preset = { class: 1, lab: 1, work: 1 }[p.kind] ? "mine" : "";
    return '<div class="wsrow" data-key="' + h(p.source_uid) + '" data-answer="' + preset + '" data-level="' + h(p.level) + '">' +
      '<div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") + "</span>" +
      (p.where ? "<small>" + h(p.where) + "</small>" : "") + "</div>" +
      '<div class="acts"><button class="b" type="button" data-answer-set="mine" aria-pressed="' + (preset === "mine") + '">Mine</button>' +
      '<button class="b" type="button" data-answer-set="not" aria-pressed="false">Not mine</button>' + levelButtons(p.level) + "</div></div>";
  }

  function paintSetupRows(ps, uncovered) {
    var classes = ps.filter(function (p) { return !p.window && (p.kind === "class" || p.kind === "lab"); });
    var office = ps.filter(function (p) { return !p.window && p.kind === "office-hours"; });
    var rest = ps.filter(function (p) { return !p.window && classes.indexOf(p) === -1 && office.indexOf(p) === -1; });
    EL("ws-class-rows").innerHTML = classes.map(setupRow).join("") + uncovered.map(function (u) {
      return '<div class="wsrow uncovered"><div class="ttl"><span class="a">' + h(u.title) + '</span><span class="meta">no class times found; Knowlu will ask this week</span></div></div>';
    }).join("");
    EL("ws-week-rows").innerHTML = rest.map(setupRow).join("");
    EL("ws-oh-rows").innerHTML = office.map(setupRow).join("");
    EL("ws-classes").hidden = !classes.length && !uncovered.length;
    EL("ws-week").hidden = !rest.length;
    EL("ws-oh").hidden = !office.length;
  }

  function openWeekSetup(week) {
    weekSetup.open = true;
    EL("week-setup").hidden = false;
    EL("ws-status").textContent = "Reading your calendar…";
    var byDay = {};
    DAYS.forEach(function (d) { byDay[d] = { start: "08:00", end: "22:00" }; });
    EL("ws-window").innerHTML = windowEditorHtml(byDay);
    paintSetupRows([], week.uncovered_courses || []);
    invoke("commitment_proposals", {}).then(function (r) {
      var ps = (r && r.proposals) || [];
      var win = ps.filter(function (p) { return p.window; })[0];
      if (win) {
        byDay = {};
        win.meets.forEach(function (m) { m.days.forEach(function (d) { byDay[d] = { start: m.start, end: m.end }; }); });
        EL("ws-window").innerHTML = windowEditorHtml(byDay);
      }
      paintSetupRows(ps, (r && r.uncovered_courses) || week.uncovered_courses || []);
      EL("ws-status").textContent = ps.some(function (p) { return !p.window; }) ? "Knowlu found these on your calendar. Mark each one." : "Knowlu found nothing repeating on your calendar. Set your day below.";
    }).catch(function () { EL("ws-status").textContent = "Knowlu found nothing repeating on your calendar. Set your day below."; });
  }

  function closeWeekSetup() { weekSetup.open = false; EL("week-setup").hidden = true; }

  function finishWeekSetup(btn) {
    var mine = [], notMine = [];
    document.querySelectorAll("#week-setup .wsrow[data-key]").forEach(function (r) {
      var key = r.getAttribute("data-key"), a = r.getAttribute("data-answer");
      if (a === "mine") { mine.push({ source_uid: key, level: r.getAttribute("data-level") }); }
      else if (a === "not") { notMine.push(key); }
    });
    // Review finding 2: D3 says Finish always writes the planning-day note, so the screen never
    // returns. An all-blank Your day would write none, so Finish asks for one day first.
    var seq = windowSequence(EL("ws-window"));
    if (!seq) { showWindowError(EL("ws-window"), "Set the hours for at least one day"); return; }
    var payload = { mine: mine, not_mine: notMine, window: seq };
    btn.disabled = true;
    confirmWeek(payload).then(function (env) {
      btn.disabled = false;
      if (!env.ok) { showWindowError(EL("ws-window"), env.error); EL("ws-status").textContent = env.error; return; }
      closeWeekSetup();
    }).catch(function () { btn.disabled = false; });
  }

  EL("week-setup").addEventListener("click", function (e) {
    var row = e.target.closest(".wsrow[data-key]");
    var a = e.target.closest("[data-answer-set]");
    if (a && row) {
      // Q11-c: a second press on the pressed button returns the row to unanswered.
      var v = a.getAttribute("data-answer-set"), next = row.getAttribute("data-answer") === v ? "" : v;
      row.setAttribute("data-answer", next);
      row.querySelectorAll("[data-answer-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-answer-set") === next)); });
      return;
    }
    var l = e.target.closest("[data-level-set]");
    if (l && row) {
      row.setAttribute("data-level", l.getAttribute("data-level-set"));
      row.querySelectorAll("[data-level-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b === l)); });
      return;
    }
    if (e.target.closest("#ws-later")) { weekSetup.dismissed = true; closeWeekSetup(); return; }
    var fin = e.target.closest("#ws-finish");
    if (fin) { finishWeekSetup(fin); }
  });
  bindWindowEditor(EL("ws-window"), function () {});
```

  In `bootConsole`, after `route(location.hash.slice(1));`:

```js
    // Phase 2 (D1, D3, Q11-d): on the vault's first day the confirm screen opens, once a launch.
    checkWeekSetup();
```

  The ask form. In `renderDecisionsView`, the first line inside `host.innerHTML = d.cards.map(function (c) {`
  becomes the per-kind branch:

```js
        if (c.kind === "commitment-ask") { return askRow(c, tomorrow); }
```

  and after `renderDecisionsView` (before `renderGoodToKnowView`'s comment):

```js
  // Phase 2 (spec §5): a `commitment-ask` card is answered here. Day toggles, a start and an end
  // picker, "add another time", Save times (answer_card), No set times (reject), Snooze. The
  // engine validates the answer; an invalid one comes back as its message, shown on the card.
  function askTimeRow() {
    return '<div class="ask-time">' + DAYS.map(function (d) {
      return '<button class="b day" type="button" data-ask-day="' + d + '" aria-pressed="false">' + DAY_NAMES[d] + "</button>";
    }).join("") + '<input type="time" data-part="start" aria-label="start"><input type="time" data-part="end" aria-label="end"></div>';
  }
  function askRow(c, tomorrow) {
    return '<div class="row dec ask" data-id="' + h(c.id) + '" data-kind="approval">' +
      '<button class="flag" data-flag="' + h(c.id) + '" title="agent-authored — flag it">&#9873;</button>' +
      '<div class="ttl"><span class="a">' + h(c.title) + '</span><span class="meta">' + h(c.age_days + "d old") + "</span>" +
      '<div class="why">' + h(c.why) + '</div><div class="ask-times">' + askTimeRow() + "</div>" +
      '<button class="lnk" type="button" data-ask-more>add another time</button><div class="ask-err" hidden></div></div>' +
      '<div class="acts"><button class="b pri y" type="button" data-ask-save>Save times</button>' +
      '<button class="b n" data-verdict="rejected">No set times</button>' +
      '<button class="b" data-verdict="snoozed" data-snooze="' + h(tomorrow) + '">Snooze</button></div></div>';
  }
  function answerAsk(row) {
    var id = row.getAttribute("data-id"), meets = [];
    row.querySelectorAll(".ask-time").forEach(function (t) {
      var days = [];
      t.querySelectorAll('[data-ask-day][aria-pressed="true"]').forEach(function (b) { days.push(b.getAttribute("data-ask-day")); });
      var s = t.querySelector('[data-part="start"]').value, e = t.querySelector('[data-part="end"]').value;
      if (days.length || s || e) { meets.push({ days: days, start: s, end: e }); }
    });
    row.querySelectorAll("button").forEach(function (b) { b.disabled = true; });
    return invoke("answer_card", { view: stateView(), id: id, meets: meets }).then(function (env) {
      ev("decision_made", id, "approval", null);
      applyEnvelope(env, function (m) {
        var fresh = EL("dec-list").querySelector('.row.dec[data-id="' + id.replace(/"/g, "") + '"] .ask-err');
        if (fresh) { fresh.textContent = m; fresh.hidden = false; } else { showRefusal(null, m, id); }
      });
    }).catch(function () { row.querySelectorAll("button").forEach(function (b) { b.disabled = false; }); });
  }
```

  In `bindDecisionsView`'s click handler, right after the existing
  `if (e.target.closest(".row.dec .acts")) { e.stopPropagation(); }` line (the static test pins
  it verbatim, so it stays as it is):

```js
      // Phase 2: the ask form's own controls live in .ttl and must not open the drawer either.
      if (e.target.closest(".row.dec.ask .ttl")) { e.stopPropagation(); }
      var day = e.target.closest("[data-ask-day]");
      if (day) { day.setAttribute("aria-pressed", String(day.getAttribute("aria-pressed") !== "true")); return; }
      if (e.target.closest("[data-ask-more]")) { e.target.closest(".row.dec").querySelector(".ask-times").insertAdjacentHTML("beforeend", askTimeRow()); return; }
      var save = e.target.closest("[data-ask-save]");
      if (save) { answerAsk(save.closest(".row.dec")); return; }
```

  The deck (Q11-b). In `renderDeck`'s card template (line 302), the fixed
  `'<button class="b pri y" data-verdict="approved">Approve</button>'` becomes `approve`, with this
  line first inside the `forEach` callback:

```js
      var approve = c.kind === "commitment-ask" ? '<button class="b pri y" type="button" data-answer-in>Answer&hellip;</button>' : '<button class="b pri y" data-verdict="approved">Approve</button>';
```

  and in `bindDeck`, the handler's first line becomes two:

```js
      if (e.target.closest("[data-answer-in]")) { location.hash = "#decisions"; return; }
      var b = e.target.closest("button[data-verdict]"); if (!b) { return; }
```

  `console.css`, at the end:

```css
/* ---- Commitment model phase 2: the confirm screen and the ask form ---- */
/* Outside `.app`, so `.app.first-run` hides nothing of it; fixed over the whole window. */
.week-setup { position: fixed; inset: 0; overflow: auto; background: var(--canvas); z-index: 25; padding: var(--s7) max(var(--s5), calc(50vw - 320px)); display: block; }
.week-setup[hidden] { display: none; }
.week-setup h1 { font-size: clamp(22px, 2vw, 27px); margin: 0 0 var(--s3); }
.ws-group { margin-top: var(--s5); }
.ws-group[hidden] { display: none; }
.ws-group h2 { font-size: 13px; letter-spacing: .06em; text-transform: uppercase; color: var(--t3); font-weight: 600; }
.wsrow { display: grid; grid-template-columns: minmax(0,1fr) auto; gap: var(--s3); align-items: center; padding: var(--s2) 0; border-bottom: 1px solid var(--hair); }
.wsrow small { display: block; color: var(--t3); }
.wsrow:not([data-answer="mine"]) .lvl { display: none; }
.wsrow .b[aria-pressed="true"], .b.day[aria-pressed="true"] { background: var(--acc-dim); color: var(--t1); border-color: var(--acc); }
.ws-nav { display: flex; justify-content: flex-end; gap: var(--s3); margin-top: var(--s6); }
.ask-time { display: flex; flex-wrap: wrap; gap: 2px; align-items: center; margin-top: var(--s2); }
.ask-time input[type=time] { background: var(--s2c); color: var(--t1); border: 1px solid var(--hair-2); border-radius: var(--r1); padding: 2px var(--s2); font-family: var(--mono); margin-left: var(--s2); }
.ask-err { color: var(--crit); font-size: 12px; margin-top: var(--s2); }
.ask-err[hidden] { display: none; }
```

- [ ] **Step 4 — run.** `cargo test -p knowlu --test static_assets -j 2` (all, including
  `the_decisions_view_lists_every_pending_approval_and_acts_like_the_deck`: the view still holds
  all three `data-verdict`s, and the ask row names both `data-id` and `data-kind`);
  `.wv\Scripts\python scripts/wizard-check.py` prints `ok`. The wizard scenario is unchanged, and
  the first-run scenario's `your_week` answer carries no `week`, so no screen opens there.

- [ ] **Step 5 — commit.** `git add app/static/index.html app/static/console.js app/static/console.css app/tests/static_assets.rs scripts/wizard-check.py`; message:

```
feat(app): the confirm screen and the commitment-ask form (phase 2, D1, D3, §2, §5)

On the vault's first day the console opens "When do your classes meet?" over
the first-run view: rows from commitment_proposals, class/lab/work preset to
Mine, a level control on Mine rows, the Your day editor, Finish (one
commitments_confirm) and Not now (nothing). The Decisions view answers a
commitment-ask card with day toggles and pickers; the deck sends it there.
wizard-check drives both paths.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Q12 — docs, the recount, and full verification (§6, §7)

**Files.**
- `docs/surface/anatomy.md`: §2 "Three panels that are not pages" (lines 57–75), §3.3 or the
  headline section (the moved line), §3.7's `moved` bullet (lines 225–232), §3.8 (234–238), §3.9
  (240–290).
- `app/README.md`: the command paragraph and lists (lines 21–51), the file table (260–279).
- `CLAUDE.md`: the engine invariants' `LOCAL_CARD_KINDS` sentence, the `commitments` command
  entry, and the app's command count paragraph.

**Interfaces.** None; documentation only.

- [ ] **Step 1 — recount by script, before writing a number.** From the worktree root:
  `python -c "t=open('app/src/main.rs',encoding='utf-8').read(); L=[set(x.strip() for x in s.split(']')[0].split(',') if x.strip()) for s in t.split('generate_handler![')[1:]]; print(len(L[1]), len(L[0]), len(L[0]|L[1]))"`
  Expected `47 29 66`. Count the `#[tauri::command` attributes per module with
  `grep -c "#\[tauri::command" app/src/commands.rs app/src/week.rs app/src/onboarding.rs app/src/account.rs app/src/lms_link.rs app/src/report.rs`
  (run in PowerShell, or with the Grep tool, to avoid Git Bash's backslash handling). Write the
  numbers the script prints, not the ones this plan expects.

- [ ] **Step 2 — `docs/surface/anatomy.md`.**
  - §2's panel table gains a row: **Confirm screen** (`#week-setup`) — "When do your classes
    meet?". It groups *Your classes* (with courses that have no class row), *Your week*,
    *Office hours* and *Your day* (the window editor). Each row has Mine / Not mine / unanswered
    and a level control on Mine. Reached when `your_week` says `setup` (the vault's first day, no
    planning-day note), over the first-run view. Finish writes through `commitments_confirm`;
    Not now writes nothing and hides it for the session. The "Three panels" sentence becomes
    "Four panels".
  - §2's page table gains a row, **Schedule** — "What is my week made of, and which hours do I
    plan in?" — and the "not pages" paragraph gains one sentence (review finding 8): "A nav view
    may be a page view, not a read-model view, as long as it polls `state` as `today`
    (`stateView()`): *Schedule* is the one."
  - A new §3.x subsection **SCHEDULE ("Your week")**. The nav link is *Schedule* (view id
    `schedule`, a page view that polls the read model as `today`: `stateView()`, R-P4a-4). The
    data is `your_week` (`commitments::overview`). Each commitment is a row with its kind select
    and level control (`set_fields`); office hours have Add; uncovered courses read "Knowlu will
    ask when it meets"; the window editor has a 400 ms `preview_window` preview (the moved text
    or "No change to today's plan", and the first five items). There is no remove control
    (spec D8).
  - §3.7's `moved` bullet gains: "The today view prints `moved.text` under the headline
    (`#moved`, spec D7)."
  - §3.9 gains: a `commitment-ask` card is answered in the Decisions view (day toggles, pickers,
    add another time, Save times → `answer_card`, No set times → reject). The deck shows
    **Answer…** for it, which opens Decisions.

- [ ] **Step 3 — `app/README.md`.** The count paragraph becomes the recounted numbers
  (**sixty-six** distinct; the console **47**, the vault-less shell **29**), with the per-module
  counts from Step 1, `week.rs` among them. The `commands.rs` list gains `answer_card` among the
  mutating commands. A new sentence: "`week.rs`: `your_week` and `preview_window` (in-process
  reads), `commitment_proposals` and `commitments_confirm` (the sibling engine, the latter with a
  temp file under the profile's `tmp\` and under `vault_io`)." Under *Known wrinkles*, three
  named, accepted costs (review finding 9): `commitment_proposals` has no timeout of its own, so a
  hung calendar fetch leaves the screen on "Reading your calendar…" while Finish still works; a
  repaint on a new `revision` closes an open kind `<select>` on the *Schedule* view; and it drops
  half-entered day toggles and times in an ask form. The file table gains a row
  `src/week.rs`. The `static/…` row gains "the Schedule view, the confirm screen (`#week-setup`)
  and the ask form".

- [ ] **Step 4 — `CLAUDE.md`.**
  - In the engine invariants, "(`commitment-ask`, `commitment-check`; today only
    `commitment-check` is filed)" becomes "(`commitment-ask`, `commitment-check`; both are
    filed, and no proposal card is filed on the vault's first day — the vault-local date of its
    earliest journal record, `commitments::vault_day`)".
  - The `commitments` command entry becomes:
    `commitments --vault <v> [--today YYYY-MM-DD] [--json] [--confirm <file> [--actor quinn] [--via dashboard]]`.
    Without `--confirm`, it reads as today, and `--json` adds `uncovered_courses` and orders the
    proposals as the cards are. With `--confirm` (phase 2), it fetches nothing and writes the
    confirm screen's answers from a JSON file (`mine`, `not_mine`, `window`) as the student. It
    prints `{created, declined, warnings, window}` and exits 2 on unreadable input or an invalid
    window, having written nothing.
  - The app paragraph: "recounted 2026-09-24 (commitment model phase 2)", the console **47**, the
    vault-less window **29**, **66** distinct, `week.rs` among the modules commands live beside.
    "Seven mutate notes" becomes "Nine mutate notes": add `answer_card` and
    `commitments_confirm`. `set_fields` edits a commitment's `kind`/`level` only as
    `commitments::check_console_edit` allows.

- [ ] **Step 5 — full verification.** One cargo at a time:
  1. `cargo build -p knowlu-engine -j 2` (the real sibling exe over the placeholder).
  2. `cargo test --workspace --no-fail-fast -j 2`. Every test passes but the four `#[ignore]`d
     ones, and the output has no `warning:` line but the accepted
     `.rsrc merge failure: multiple non-default manifests`. Check with
     `cargo test --workspace --no-run -j 2 2>&1 | grep -E "^warning"`.
  3. The Deno suite, unchanged, in CI's form:
     `deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
     (phase 2 touches no file under `cloud/`: `git diff --name-only 3d984d7 -- cloud` prints nothing).
  4. `.wv\Scripts\python scripts/wizard-check.py` prints `ok`.
  5. The byte check: `git diff --exit-code 3d984d7 -- engine/tests/fixtures` exits 0, and
     `git status --porcelain -- engine/tests/fixtures` prints nothing. That covers the eight
     Python references and `surface-today-{s1,s1-migrated,full}.json`.
  6. `pwsh -File scripts/ci/eol-check.ps1` (or `powershell -File …`) passes: every new file is LF.

- [ ] **Step 6 — commit.** `git add docs/surface/anatomy.md app/README.md CLAUDE.md`; message:

```
docs: the commitment model's phase 2 in anatomy, the app README and CLAUDE.md

The confirm screen, the Schedule view, the moved line and the ask form; the
commitments --confirm flag; Tauri commands recounted by script: console 47,
wizard 29, 66 distinct; nine commands mutate notes.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

---

## Self-review (2026-09-24, done while writing)

- **Spec coverage.** Every row of the fidelity ledger names a task, and every §7 test has a named
  test:
  - `--confirm` rows and level: `mine_writes_exactly_the_rows_given_at_the_given_level_as_the_human`.
  - Stale keys: `an_unknown_or_stale_key_is_warned_and_skipped`.
  - Second Finish: `a_second_finish_is_a_no_op`.
  - Twins: `not_mine_declines_the_twin_group_together`.
  - Invalid window, exit 2: `an_invalid_window_writes_nothing_and_exits_2` and
    `an_invalid_window_writes_nothing_at_all`.
  - One-line edit: `a_window_edit_on_an_existing_note_changes_one_line`.
  - Day-1 gate: `no_commitment_check_is_filed_on_the_vaults_first_day`.
  - `emit_asks`, day 3 and two a day: `no_ask_before_the_vaults_third_day` and
    `at_most_two_a_day_in_slug_order_and_each_course_once`.
  - Decline marker: `rejecting_writes_the_card_marker_and_archives` and
    `a_declined_or_answered_course_is_not_asked_but_a_withdrawn_one_is`.
  - Withdrawal: `a_pending_ask_is_withdrawn_once_a_class_proposal_for_its_course_appears`.
  - Invalid answer back to pending: `an_invalid_answer_goes_back_to_pending_with_the_warning`.
  - `uncovered_courses`: three tests in Q2.
  - `overview`'s shape: Q5.
  - No-network pin: `confirm_makes_no_network_call`.
  - App tests: `app/tests/week.rs` (Q9) and `app/tests/commands.rs` (Q8).
  - Static tests: Q10 and Q11. Behavioural: `wizard-check.py` (Q11).
- **Placeholder scan.** No "TBD", "similar to" or "handle edge cases". Two instructions depend on
  what the run shows, and each gives a concrete rule, not a gap:
  - Q6 Step 4 names the one P16 test that takes `decline_cs100_ask` (review finding 4), and when
    to stop and report.
  - Q12 Step 1 says to write the recounted numbers, not the expected ones.
- **Review fixes applied (2026-09-24):** finding 1 (Q1-a, `first_ts`, noon stamps), 2
  (`finishWeekSetup`), 3 (`covered_course_keys`, `has_class`), 4 (Q6 Step 4), 5 (the
  `WriteContext` import moves to Q3), 6 (`check_answerable`), 7 (Q11-a kept, spec §4 amended), 8
  (the `console.js` comment in Q8, anatomy §2 in Q12), 9 (`saveCommitment` drops its second
  render; the rest named in `app/README.md`), 10 (line references). The later tests use the new
  helpers `journal_at`, `covered_course_keys`, `has_class` and `check_answerable` by these names.
- **Names checked against the code at `3d984d7`:** `commitments::{proposals, create_confirmed,
  create_marker, emit_checks, file_card, parse_window, settle_approved, settle_rejected,
  withdrawal_reason, load, read_series_file, twin_keys, created_by_this_card, strict_meets,
  course_key, to_code_exempt, meets_json, meets_label, date_json, hm, to_value, short_title,
  proposal_order, proposal_commitment, days_since, code_warnings_hit, successor_keys}`,
  `ledger::JsonlLedger::day_files`, `approvals::{sorted_md, as_date, withdraw_stale, Withdrawal,
  read_note, str_field, rel_path, stem_of, name_of}`, `ids::{resolve_target, rel}`,
  `yaml::from_json`, `surface::{build_state_preview, View::Today}`, `scheduler::engine_exe`,
  `childproc::NoConsole`, `commands::{mutate, console_ctx, executor_ctx, attach_scheduler,
  state_inner, now_in}` and `ConsoleState::{lock, vault_io, data_dir, seen_at, note_write,
  set_test_today}`. `to_code` is deliberately not called (see Q2's watch note).
- **Consistency.** Every other task uses the names this plan defines:
  - Q3: `stored_proposals` / `Stored`, `ConfirmInput`, `ConfirmReport`,
    `create_confirmed_as` / `create_marker_as`.
  - Q2: `proposal_value`, `card_order`, `UncoveredCourse`, `uncovered_courses`.
  - Q6: `ask_key`, `COMMITMENT_ASK`.
  - Q7: `AskSettled`.
  - Q8: `CONSOLE_FIELDS`, `CONSOLE_KINDS`, `check_console_edit`.
  - Q10: `stateView`, `confirmWeek`, `levelButtons`, `windowEditorHtml`, `windowSequence`,
    `bindWindowEditor`, `showWindowError`.
  - Command arguments are single words throughout (`view`, `confirm`, `window`, `id`, `meets`).

## Spec problems found while planning, and how they were settled

The pre-execution review (`docs/plans/2026-09-24-commitment-model-phase2-plan-review.md`, "ready
with fixes") ruled on all seven. The spec now carries an "Amendments (2026-09-24, plan review)"
section for 1–5 and 7. The parent spec is not edited; where it disagrees, the amendments section
supersedes it. These spec lines land with Quinn's go at the checkpoint.

1. **§5's `emit_asks` signature omitted the proposals.** Amended to
   `emit_asks(vault, proposals, today, budget, ctx, journal)` (Q6-a).
2. **An ask-created note had no title.** Amended: its title is the course note's title, its file
   `commitments/<slugify(title)>.md` (Q7-b). This supersedes parent §5.3's `commitments/<slug>.md`.
3. **The answer's warning had no read-model carrier.** Amended: the console shows the engine's
   warning from `answer_card`'s envelope until the next repaint (Q7-a, Q8-c).
4. **`schedule` is not a read-model view.** Amended D6: the view polls the read model as `today`
   (Q10-a).
5. **The deck could not answer an ask.** Amended §5: the deck shows **Answer…**, which opens
   Decisions; Reject and Snooze stay (Q11-b).
6. **The P16 `rank` tests' vaults change.** Accepted, narrowed: the seeded journal day and the
   noon `commitment_note` stamp (Q1-c), and a `card:cs-100` marker in exactly one test (Q6 Step
   4). Setup only, never an assertion. No spec change.
7. **Day 1 as a UTC date was wrong east of UTC** (review finding 1). Amended D2: the first day is
   the vault-local date of the earliest journal record (Q1-a revised). This supersedes parent
   §5.3's "earliest `state/journal/` file" wording, and §5.3's day 3 counts from the same date.
   Spec §4 is also amended for Q11-a (review finding 7).

