# Twelve-hour clock plan, review r2 (2026-11-07)

**FAIL at f537ad8:** 0 Critical, 2 Important, 4 Minor.

## r1 findings, checked against the code

- **I1, closed.** `render::due_move` has two callers: `render.rs:187` (today.md) and `surface.rs:877`. The `due_move_with` split works with jiff's `DateTime`.
- **I2, partly closed.** Sites that use no slicing still pass the check (Important 1 below).
- **I3, closed.** The restated pattern matches `UNCHECKED` byte for byte.
- **I4, closed in substance** (Minor 1 below).
- **I5 and M1–M4, closed.**
- **T1 is a pure move.**

## Important

1. **T4's static pins miss times passed through untouched.** Five §2.2 sites print raw values today:
   - `console.js:92`, `h(t.generated_at)`
   - `:639`, `h(e.due)`: raw, though §2.2 lists it as a `.slice(0, 16)` site
   - the tail of `:1286`, `" — expected " + h(e.due)`
   - `:674-675`, `JSON.stringify(v)`
   - `:2803`, `:2806` and `:2808`, the mail rows

   Forms the forbidden list does not catch:
   - `.slice(10)`
   - `.slice(-5)`
   - `.split(" ")[1]`, `.split(/[T ]/)`
   - `.replace(/[T ]/`
   - `getUTCHours(`
   - `Intl.DateTimeFormat`

   **Fix.** Add a positive pin for every §2.2 row that names a helper, and add the forms above to the list. Better still, render `surface-today-full.json` through the real renderers under node (the harness at `onboarding_static.rs:634`). Then assert there is no 24-hour `H:MM` outside the allowed regions.
2. **today.md is "provably unchanged" only on paths the plan does not rely on.** Both goldens pin only a three-day move, which never calls the clock, so `oracle.rs` proves nothing about the clock path. The branch that adds both a year and a time is not pinned byte for byte.

   **Scenario.** The year and the time are appended in the wrong order. The output reads `12/31 23:00/2026`, and every test stays green.

   **Fix.** Commit whole-string `render.rs` tests before the split, then rest the proof on them:
   - `10/9 00:30 → 10/8 23:00 (1 day EARLIER)`
   - `12/31/2026 23:00 → 1/1/2027 01:00 (1 day later)`

## Minor

1. **The pin already exists.** `a_class_proposal_files_one_card_with_the_exact_title_frontmatter_and_body` (`commitments.rs:9315`) already pins the title and the file name, and it predates the move. Use it as the pin. The module doc should also say that the output names approval files through the slug.
2. **Two time zones on one page.**
   - `local_stamp` converts to the vault's zone, while the console helpers use the machine's zone.
   - Name this in Q3, or use the vault zone.
3. **The comment stripping needs a JS-aware scanner.** A naive `//` stripper breaks on `"https://…"`. Add a self-check case for that.
4. **The restated digest pattern can drift.** The comment should say that it and `approvals.rs:1244` change together.

## Questions for Quinn

The premises of Q1, Q2 (as corrected) and Q3 hold. There is no third UTC display. Q2 could add that the format also names approval files.
