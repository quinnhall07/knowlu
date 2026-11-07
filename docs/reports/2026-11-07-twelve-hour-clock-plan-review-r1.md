# Twelve-hour clock plan, review r1 (2026-11-07)

**FAIL at 02798b3:** 0 Critical, 5 Important, 4 Minor.

The core of the plan holds. Every task is display-only. None touches the eight Python references or any journal, ledger or run-record writer. T3 follows console spec §4.6: its own commit, a three-substitution check, a message that says why, and `contract-reviewer` reads it. I checked the three strings in `surface-today-full.json`: each appears once, and the s1 and s1-migrated references have no `coming_up` times. Agent labels match CLAUDE.md's table and the contract list, and every task writes its tests first.

The claimed bug is real. `console.js:622` slices `ended`/`started` at 11–16. Both stamps come from `journal::now_ts`, which writes UTC with a `Z` (`app/src/scheduler.rs:956`, `:1334`, and `refuse` at `:777`). So "last slot" shows a UTC time, while `localHM` 167 lines above it converts correctly.

## Important

1. **A missed site, and a recommendation built on a wrong premise.** `render::due_move` is not only in today.md. `surface.rs:877` puts it in `AmendCard.due_move`, and `console.js:209` draws it ("due 9/25 09:00 → 9/26 09:00" when a move is a day or less, `render.rs:118-120`).
   - Scenario: a student opens an overnight due-date amendment and sees a 24-hour time beside 12-hour times everywhere else.
   - The §2.1 row's reason ("stays so the page has one format") is backwards.
   - The fix needs a surface-only label, and T2 says "Never `render.rs`", so the plan has to choose.
2. **A second UTC display is missed, and T4's pin wouldn't catch it.** The note drawer's history at `console.js:1659` shows `r.ts.slice(0, 16).replace("T", " ")`. `r.ts` is the raw journal stamp (`surface.rs:1955`, from `now_ts`), so it is UTC and 24-hour.
   - T4 only forbids `.slice(11`, so `:1659`, `:643` (`.slice(0, 16)`) and `:151` (`.replace("T", " ")`) all pass T4 unchanged.
   - Scenario: T5 ships green and the drawer still shows UTC.
3. **T2's digest test can't be written as specified.** `approvals::digest_uids` is private (`approvals.rs:1248`, plain `fn`), and so are `CHECKED`/`UNCHECKED` (`:1242-1245`).
   - A test in `eventemit.rs` that calls it won't compile unless someone makes it `pub(crate)`. That is an edit to a contract-list file, which T2 forbids.
   - Scenario: the implementer either stalls or quietly widens the visibility.
   - Better: assert the emitted lines against the same regex shape written in the test, or round-trip through the public approve path.
4. **`clockfmt` output lands in frontmatter, not just display.** Card titles (`proposal_title`, `commitments.rs:3043-3052`; `what_and_when`) become approval `title:` frontmatter. `ends_label` goes into the commitment note's `ends:` key (`eventaccept.rs:231`, asserted at `:398`).
   - §1 ("display only") and §2.1 ("accepted-event note body") misclassify this.
   - T1 is a pure move, so no bytes change today. But nothing marks `clockfmt` as writing vault bytes.
   - Scenario: a later tweak (`12pm` to `noon`, or a Q2 setting) gives new notes a different `ends:`/`title:` format from old ones.
   - T1 should state this in the module doc and pin it with a test.
5. **Q2's premise is partly false.** "The engine cannot read a locale and stay deterministic" is true only for reading the OS setting. A setting passed in from vault config or a flag is ordinary input, and same input still gives the same order.
   - The real constraint is item 4: engine strings are baked into frontmatter at write time, so a per-student format would split one vault between formats.
   - This is not a ruling on Q2, just a correction to its stated reason.

## Minor

1. §1 cites `eventemit.rs:508` and `:557`. `clock` is at `:509` and `when_label` at `:527`; `:557` is the start of `what_and_when`'s doc comment. §2.1 has them right. All other cited lines check out.
2. Two raw `due` displays are not in §2.2.
   - The drawer's frontmatter list (`console.js:1654-1657`) shows `due`/`executed_at` raw, and `due` is editable, so it must stay raw.
   - The delta and history `text` (`surface.rs:1949`, drawn at `console.js:108` and `:1659`) shows `due "2026-09-25T09:00" → …`.
   - Either list them as "stays" with the reason, or T5's "every 24h/amb row, nothing else" is incomplete.
3. The console never draws the events-digest body; it only shows counts (`console.js:307`, `:331`, `:661`). Changing it reaches only the vault file and leaves old and new digests in two formats. Say that is accepted, or drop the digest from T2.
4. T4 is missing test cases, and the ledger is missing a row.
   - T4 has no date-only or with-seconds cases for `stampOf` (`2026-08-26`, `2026-08-26T23:59:00`).
   - Amend `from`/`to` and mail `due`/`old`/`new` carry raw vault values.
   - The fidelity ledger has no row for the sync-line UTC fix (Q3) and the test that proves it.

## Questions for Quinn

- Q1's premise holds. Nothing in `app/static` shows or links today.md; code only checks that it exists (`console.js:1477`, `scheduler.rs:274`).
- Q3's premise holds, but it is not the only UTC display (Important 2).
- Q2: see Important 5.
