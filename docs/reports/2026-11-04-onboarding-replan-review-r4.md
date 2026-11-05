# Onboarding replan draft, review r4 (2026-11-04)

**FAIL at 5e252f7:** 0 Critical, 1 Important, 4 Minor; all text fixes. All 13 r3 findings closed. No sweep path deletes a signed-in student's setup data.

## Important

1. **The no-time drop writes without the series lock** (§6.5 462-466, C2b 967-969). `refresh_series_with` locks only when `fresh` is non-empty (`commitments.rs:2299-2302`); `rank` goes through it (`cli.rs:837`); a vault with no feed always has `fresh` empty. The first `rank` of a term can rename a stale copy over a locked registrar write and lose `applied` and `ended` history. Fix: read unlocked; if a drop is due, lock, re-read, write. Test that the drop holds the lock.

## Minor

1. The drop's condition differs between sections ("past term", "another term"). A wrong clock can drop the block, and a re-setup can write a future block that the next `rank` deletes. Fix: drop only a term strictly before `term_for(today)`, ordered by `School::terms`; `None` keeps the block; a clock set ahead is a named accepted limit; C2b gains future-block and `None` cases.
2. The purge must say it re-reads under the lock (§6.12 715-717), with a C2b case for a file that changes between the read and the lock.
3. §6.12 775 and 757-758 are false for a moved vault: sign-out deletes `session_target(id)`, not `cloud.yaml`'s target (`account.rs:759-775`), and `attach_config_and_session` keeps an existing `cloud.yaml` (`:1088-1089`). State both as accepted limits; the real fix goes in §9.
4. Nothing proves a real miss returns 1168. Add one read of a generated missing target asserting `code: 1168`, under `CREDMAN_LOCK`, carved out of C5d's no-real-credential line.

## Held up

The fail-safe rule; `try_lock`; `change_school`; backups (prefix filter, `registrar` block survives); §4.7 has no cycle, parent-plan merge state stays the controller's re-check.
