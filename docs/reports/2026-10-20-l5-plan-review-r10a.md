# L5 plan review, r10a, delta (2026-10-20)

Plan: `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md` (r10a: the listing, its `Plan` field and its writer).
Reviewer, plan mode, against spec §5 (`docs/specs/2026-10-05-syllabus-class-times-design.md:836-889`), with the code
checked on the email-forwarding worktree. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 2 important, 4 minor. Nothing in r10a breaks determinism, `surface` never writing,
`rank` never calling a model, or a frozen reference.

## The four questions

1. **`rank` writing a `state/` file is sound.** `state/` is generated and `rank` already writes it; the spec's model,
   `state/grades.json`, is written whole with no journal record (the journal is for notes). The listing depends only on
   `today`, the cards on disk and the extraction; "write only when bytes differ" keeps a same-day rerun byte-stable.
   `vault-full` has no `state/syllabi/`. Gaps: within-day order (M-3), two writers racing (I-1).
2. **No race inside a slot** (`judge` finishes before `rank`'s step 1) **or across desktops** (`state/syllabi/` never
   syncs, spec :891). The race is with `syllabus-add` from the console during a slot (I-1).
3. **`Later` covering both cases is acceptable**: an item that did not fit is planned again next run. The problems are
   a missing `past` (I-2) and `Filed` against G9 (M-1).
4. **Order and Done are reachable.** T2c precedes T4d (:860), T4e and T4d'a; T5 depends on T2c (:870); T4d'a stays
   under 120 lines with I-1's fix.

## Important

1. **A slot's `rank` can wipe out a syllabus the student just added** (T4d'a step 5 :1732-1736, T2c :1088-1091; spec
   :884). Nothing serialises the three callers: `RunLock` is taken only by `sync` (`sync.rs:2463`, `:2518`) and
   `grades` (`grades.rs:656`), and the app has no lock around engine runs. Step 5 writes the rest of the file
   "unchanged" from what step 1 read. Scenario: `rank` reads `state/syllabi/bio-101.json` (`read`, old SHA); the
   student re-adds the syllabus (new PDF, new state file, `pending`); `rank` writes the old extraction back with the
   old SHA and `read`. The new copy is never read (`judge` sends only `pending`), cards keep filing from the old
   syllabus, and nothing tells the student. Fix: T2c's writer gets a read-modify-write form that re-reads the file,
   replaces only the per-item outcomes, and skips if the SHA or `added` differs from what `read_inputs` saw; write
   beside and rename over (as `grades.rs:583-586`); a T4dt (B) case changes the file between `read_inputs` and step 5
   and the hook skips.
2. **Version 1's outcome codes leave out `past`** (plan :1183-1188; spec :842 has `past` in T17's `items` state, but
   §5's byte-contract list :880 does not). `Later` is "not past", so a past item without a card has no code, and the
   code set freezes at the first release. Fix: add `Past` to `Outcome`, one (A) case, and have the spec say `past` is
   in version 1.

## Minor

- **M-1. `Filed` disagrees with G9** (:1184-1185 vs :1367). A key whose only history is a superseded card is
  plannable again; re-planned but not fitting the 15, it is both `Later` and `Filed`. Fix: `Filed` means planned this
  run, or held by a card that blocks the key under G9.
- **M-2. The writer has no pinned name** (:1217-1218). T4dt depends only on P0 and can be written before T2c. Fix: pin
  the writer's signature in T2c, or have (B) assert only the outcomes key changed; add a T1 case for version 1's empty
  outcomes (key name and shape) and update row :852.
- **M-3. Same-day items have no order** (:1182). Name a stable tie-break (extraction order or `source_uid`).
- **M-4. Outcomes can go stale** (T5 :1857-1858). After `judge --only syllabus` reads a new extraction, outcomes keyed
  to vanished items stay until the next `rank`. Fix: T5 clears the outcomes when the extraction's SHA changes, or T17
  ignores outcomes whose key is not a current item.
