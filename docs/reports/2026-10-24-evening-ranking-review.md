# Evening ranking lane: pre-push review (T6, 2026-10-24)

Reviewer on `cb3b661..123ee02` (8 commits), against the plan and the spec. Filed by the controller. The docs commits after the range (209e768 T5, 021e12b T7) are docs-only and LF; their file lists were checked, but not their wording.

**Verdict: PASS.** No Critical or Important findings. The reviewer did not run cargo, so 0 warnings and the CI line come from CI.

## The checks T6 lists
- **No contract-list file and no fixture in the diff.** The diff touches only `cli.rs`, `main.rs`, `ranking.rs`, `surface.rs`, the new `engine/tests/evening_rank.rs`, and docs.
- **`oracle.rs` and `surface_oracle.rs` are unedited at every commit.** Both goldens and the three `surface-today-*.json` references are byte-identical.
- **Existing tests are unedited.** Every removed line is a stub, an import, a match arm, or a verbatim move of `Window`, `with_window` or `load_with`. No `#[ignore]` changed, and no CR bytes were added.
- **T0's clock record** is in §9.
- **Q-1:** S4 and T4's fourth item are absent, as dispatched.

## The code
- **Determinism:** a stable sort on exact `DateTime` values (R11).
- **Strict `--now`:** `strptime("%Y-%m-%dT%H:%M")`, with single digits accepted (C1). A bad `--today` is still `BadDate`.
- **A bad `--now`** writes nothing and exits 2, because `rank_clock` runs before `start_run` (C2, E3).
- **`--now` pins no written timestamp.** The moment reaches only `rank_at`, and the footer, run record and `crash` keep the wall clock (C4). With no `--today`, `--now`'s date sets the day (C5).
- **Overrides still win (R9).** The step is off before 20:00 and on any other day (R2, R3, R8). Tonight ends at midnight (R7). A task due exactly at the moment does not count as closing tonight (R5).
- **The page ranks at the moment it shows:** one `civil = now.datetime()` (S2, S5). The fixtures are unchanged at 21:00 (S3, E2).
- **`note_detail` keeps `load`,** and `row` reads no rank position.
- **No single-user assumptions, and no model call.**

## T2's call: reorder every non-override task, not only Must do
This is sound. With `DEFAULT_BUFFER_DAYS = 1`, any task due today has slack ≤ −1, so it is already Must do. Recommended's order therefore cannot change, and Must do's members stay fixed (R10). Only `designate_today`'s capacity order moves, which is the intended spill swap (S5). One ranked list keeps `today.md`, the console and `commitments::moved` aligned.

## Minors
1. **A mismatched `--today`/`--now`** (for example `--today 08-28 --now 08-29T21:00`) is silently daytime. That is deliberate (R8), but consider a stderr note or a line in `engine-commands.md`.
2. **The zone split (Q-3)** stays a follow-up. It is named in the HANDOFF row.
3. **C4's `"ts": "2026-08-28` check depends on spacing.** The `2026-08-28T21:00` check is the stronger guard. Acceptable as is.

## Contract review of `engine/tests/evening_rank.rs` (Q-2)
**Verdict: PASS** (contract-reviewer). No Critical or Important findings.
- **Scope.** None of the 9 files is on the contract list or a fixture. `git log` over the range against `oracle.rs`, `surface_oracle.rs`, `tests/fixtures`, every contract-list source, `sync_*.rs` and `app/` finds no commits.
- **Copy and strip.** `copy_vault`, `scratch` and `strip_footer` are line-for-line copies of `oracle.rs`, so E1 and E3 always run on temp copies. E2's in-place `surface` is sound: the arm only prints, and the lane adds no I/O to `ranking.rs`.
- **E2's fields** (`must_do`, `recommended`, `meter`, and `nav_counts` less `runs_warn`) exist in all three references, are never null, and don't read the clock.

Minors:
1. E3's "names `--now`" assertion would also pass on clap's own `unexpected argument '--now'` error, if a rebase dropped the field. Assert the `BadNow` text instead (for example "--now must look like").
2. The module doc says every run uses a scratch copy, but E2 runs in place. Amend the header.
3. E2's comment overclaims coverage. Either add `the_day`, `verdict`, `ahead`, `topline` and `moved` to `ranked_parts` (all ranking-fed and clock-free), or reword the comment.
