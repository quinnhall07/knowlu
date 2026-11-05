# Onboarding replan draft, review r2 (2026-11-04)

**FAIL at 5cd5d0b:** 0 Critical, 5 Important, 8 Minor. The design stands; all five Important findings are text fixes. Round 1: 15 of 16 closed against the code, finding 13 reopened (I2). RQ11, RQ12 and RQ13 were ruled at d61a7c3, after this review's base.

## Important

- **I1. §4.7 has a cycle and two missing edges.** `scaffold.rs`'s order (line 161) puts S2 before C4's and C8's rows, C8's brief (927) follows it, line 266 has S1 to S4c follow C8, and line 170 holds S2 for RQ4. Land both rows before S2. C2c runs beside C2b (264) but its tests delete C2b's block (818), and its merge write needs `refresh_series_held` (`commitments.rs:2322`) while lines 164-165 list C2c under `registrar.rs` only: add C2b to C2c's dependencies and C2c to the `commitments.rs` row. Line 167 puts C5b after plan work in `onboarding_state.rs` that waits on R0a, while 132 and 262 run C5b at once.
- **I2. `for_cards` and "no ask fires beside it" cannot both hold** (504, 510, 781-782). `commitment_passes` builds one list at `cli.rs:869` for `withdraw_asks` (872), `emit_checks_locked` (884) and `emit_asks` (894); `uncovered_courses` counts coverage only from it (`commitments.rs:892`). Split the list (cards filtered, asks unfiltered), add `cli.rs` to C2a, test through `rank`.
- **I3. A `classsearch:` key can reach a synced marker or a card by routes §6.7 never walks.** `settle_rejected` calls `twin_keys` at `commitments.rs:4903`, `:4913`, then `create_marker` (`:4925`). `detect_changes` builds `live` from every series (`:4052`), so an open row can be a successor (`:4095-4120`). The cited tripwires read only `sync.rs`, `sync_rows.ts` and migration text (`:6067-6121`). Guard `create_marker_as` (`:4359`), exclude `classsearch:` from `live`, test each route.
- **I4. Try again has no data path** (492, 527-528). Per-course state exists only in the bundle, which `class_times.apply` deletes (583); no test or button exists; a job retry re-fetches and a rejected row returns at step 10. Cut it from wave B, or name a stored record and add it to §6.12.
- **I5. RQ6 has a hole at Finish, and the launch retry has no seam** (610, 627-628, 869-870). `purge_class_search` touches only the series file; launch does not delete the bundle. If the delete fails, raw answers stay after "deleted when setup finishes". Sign-out relaunches `--pick` (`onboarding_state.rs:1062`) with no profile open, and `main.rs` has two `.setup` blocks (`:91`, `:160`). C5d names the walk over profiles, defines signed-out as no session credential, and deletes the bundle at launch too.

## Minor

- **M1.** RQ12 leaves the approved text literally false between Finish and sign-out. The backup guard tests series only, so `ended` keys carrying a CRN and the no-time block reach the mirror and 30 snapshots; "the file is generated" (637) is false for the `registrar` block.
- **M2.** A registrar read deletes the no-time block (467, 612), undoing RQ9. Keep it until the term changes.
- **M3.** After purge, a collapsed Google twin returns with `setupRow`'s Mine preset. Today's behaviour for a Google row; state it in §6.7.
- **M4.** The `keep_alive` arm (442) covers only `configured`; a fresh read with an old `last_seen` ages out at `commitments.rs:2383`. C2a needs that case.
- **M5.** §6.12's Logs line misses `confirm`'s warnings (`:5131`, `:5151`, `:5161`, `:5182`), which print the key. C2b pins fixed words.
- **M6.** "Not mine on a Google row" (491, 799) works only for a by-title twin; a same-signature key is reported stale (`:5170`). Test the real path.
- **M7.** Delete R0a item (e) (738), line 671's sentence, 672-674's founder fallback, and §12 item 3's item (e) line (1063-1064). Keep the redirect rule for other schools.
- **M8.** Line 592's "step 3 was skipped" cannot happen (`onboarding_state.rs:919`). C6 misses `onboarding.js:635`. C2a, C2b, C2c carry no line estimates; split C2b (plan `:1582`).

## Held up

§6.6's write path (`write_literals`, journal first, no re-dump); note bytes and frozen references; adoption order; the `class_search` block against existing readers; §6.7's page paths; sign-out order (`account.rs:767`); backups' `wanted` note; no change to `journal::VIAS`, actors, `LOCAL_CARD_KINDS`, `NOTE_FOLDERS` or `generate_handler!`; no rule-1 breach; no conflict with VISION.
