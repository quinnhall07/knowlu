# Twelve-hour clock plan, review r3 (2026-11-07)

**FAIL at 348305b:** 0 Critical, 1 Important, 3 Minor.

No finding lets a 24-hour time ship or changes vault bytes. r2's I2, M1, M2, M3 and M4 are closed. I1 is closed in Layers 2 and 3. All seven whole-string `due_move` cases match today's `render.rs:110-133` output.

## Important

1. **T4 Layer 1 cannot be built as described.**
   - The `onboarding_static.rs:634` harness records nothing. Its `document` is a Proxy with `set: () => true` (`:638`).
   - Five supplement rows never render through `paint`:
     - `:422` and `:474` render from the update command's own answer;
     - `:588` sits inside `runSync()`;
     - `:1057` `regMeets` is reached only through `your_week` and needs `registrar_label`;
     - the drawer (`openDrawer`, `:1648`) is not lifted and returns `undefined`.
   - Scenario: T4 stays red after T5. Or the rows get quietly dropped, and Layer 1 then covers nothing for them.
   - Fix: write a new recording `document`. Then either stub `invoke` for `your_week`, `note` and `mail_applied`, lift those renderers and flush microtasks, or name those rows as covered by Layers 2 and 3 only. Require one known output string per remaining supplement row.

## Minor

1. Three Layer 2 anchors don't match the code with their quotes (`:643`, `:1659`). `data-undo-close` also matches `:2148`.
2. The `JSON.stringify(v)` ban sits on `:674`, but the call is on `:675`. It conflicts with §2.2's "other keys stay". Ban it only on the `due` branch.
3. Layer 1's reason for `surface-today-s1-migrated.json` is wrong. It adds the s1 blocks, not the runs rows.

## Questions for Quinn

Q2 and Q3 have no false premise.
