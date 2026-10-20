# L5 plan review, r10b, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r10b). Reviewer, plan mode, against r10a's report, spec
§5 (`docs/specs/2026-10-05-syllabus-class-times-design.md:836-891`) and the email-forwarding worktree
(`grades.rs:573-588`, `ledger.rs:379-385`). Filed by the controller from the reviewer's returned text.

**Verdict: PASS.** 0 critical, 0 important, 5 minor. All six r10a findings are closed. r10b breaks nothing on
determinism, byte stability, task order or Done, the contract list after T4e, or the frozen references.

## r10a findings

- **I-1, closed.** `write_outcomes` (:1106-1111) re-reads and returns `Skipped` when the file is gone, its `version`
  is unknown, or its SHA or `added` differs; both forms write beside and rename over. The (B) case (:1251-1257) is
  buildable (both functions exist by T4d's Done); step 5 (:1770-1776) matches.
- **I-2, closed.** `Past` is in the interface (:1214-1216) with an (A) case (:1243-1245); place 10's twelfth change
  brings spec :880 into line with :842.
- **M-1** (:1210-1212), **M-3** (:1207-1208), **M-4** (T5 :1899-1902, T5t :1890-1892; T2c already writes `[]` on every
  add, so T5's rule is a second guard) and **M-2** (signatures and `Written` pinned :1104-1111; T1 :1069-1071; §5 row
  :862; (B) :1247-1250) are closed.

## The planner's choices

- **`item_outcomes` as an ordered list of `{source_uid, outcome}`: sound.** `ledger::dumps_value` sorts keys
  (`with_sorted_keys`, `ledger.rs:382`) and `serde_json` has no `preserve_order`, so the whole write and the
  re-read-and-dump give the same bytes and a same-day rerun is `Unchanged`. T1's "in the key order T2c's `write_state`
  uses" means sorted; it could say so.
- **`(String, &str)` pairs: acceptable** (T2c lands before `Outcome`), but the frozen code set is unchecked at the
  writer (M-2).
- **The re-read-to-rename gap: acceptable** (milliseconds of local I/O, not the whole of `rank`); worst case unchanged
  from I-1, and nothing self-heals it (M-1).
- **`Past`: coherent** and consistent with r10's day-1 horizon; false on day 1 (M-3), precedence open (M-4).

## Minor

- **M-1. Nothing repairs a state file that disagrees with its copy** (T2c :1112-1113; T5 :1894-1902). Reached when
  `syllabus-add` renames inside `rank`'s gap, or is killed between writing the copy and the state (copy first,
  :1099-1100, pre-r10b). The state is `read` with SHA A while the copy is B; `judge` sends only `pending`, so cards keep
  filing from the old syllabus silently. Fix: in a `--syllabus` run T5 treats a state whose SHA differs from the copy's
  as `pending`, with one T5t case.
- **M-2. Nothing stops a seventh code reaching version 1.** Fix: `syllabus.rs` holds `ITEM_OUTCOME_CODES` (the six);
  `write_outcomes` refuses any other; a T4dt (A) assertion checks every `Outcome::code()` is listed and the list has six.
- **M-3. `past` is false on day 1** (:1215-1216, §7 :2365-2367): an assignment due tonight on the vault's first day lists
  as `past`. Fix: T17 and T11b word an item dated today as due today, not past; say so in §7 and T11b.
- **M-4. Which code an item gets is not settled** (:1210-1217): a past item matching a task note is both `duplicate` and
  `past`. Fix: state the order (`Filed` > `duplicate` > `feed-differs` > `needs-date` > `past` > `later`) and add one
  (A) case for a past duplicate.
- **M-5. Leftovers.** The status note at :13 still says "a twelfth place-10 change" for the M-1 alternative (now
  thirteenth). Undated items' order among themselves is unnamed; extend "extraction order, then `source_uid`".

## Checked and clean

Determinism (listing a `BTreeMap` by slug, total item order, key-sorted dumps); byte stability (`Unchanged`, (B)'s
same-day rerun); contract list after T4e (T4d'a only `cli.rs`, T2c only `syllabus.rs`, `ledger.rs` used, not edited);
order (T2c before T4d, T4d'a, T5; T4d'a about 50 lines); temp files (`grades.rs`' pid-suffixed temp name).
