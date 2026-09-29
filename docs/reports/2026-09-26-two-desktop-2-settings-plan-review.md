# Two desktops, plan 2 of 3 (the account holds the shared settings): plan review

Reviewed 2026-09-26: `docs/plans/2026-09-26-two-desktop-2-settings-plan.md` at `e896047` on `two-desktop`,
against the signed spec `docs/specs/2026-09-25-two-desktop-design.md` (D17–D19, §4.8, §6.1–§6.3, Q13) and
Plan 1's final text at `4ebc7d6`. Read-only: no build, no test run, no subagent. Code was read at `e896047`
(Plan 1 Tasks 1–3 executed); `p1-`/`p2-commitments` and `p3-registrar` were read for the migration overlap.

## Plan Review

**Status:** Issues Found

**Consistency with Plan 1 (4ebc7d6)**

Consistent, with one gap (Issue 3). Checked against the final text, not the first draft the scratch copy
was built on:

- **`Totals::offline`, `pull_event_verdicts`.** Plan 2 reads neither. Plan 1's final Task 10 leaves
  `run_lines_with_client` untouched (the verdict pull lives in `judge`), so Plan 2's order table and its
  loopback harness (two scripted replies per run: pull, then push) are right for the final Plan 1. Under the
  first draft a `GET /judge-event` would have sat between them.
- **`PushBatch`.** Plan 1 never touches it. Plan 2's amendment (`settings`) is complete: the only struct
  literals are `engine/tests/sync_contract.rs:315` and `:336` (`build_push` uses `..Default::default()`),
  and Plan 1 adds none.
- **`Cursor`.** Task 4's anchor is the final wording of Plan 1 Task 10's `verdicts_after` doc lines, which
  close the struct; the two new fields land beside it, as Plan 1's interface table asked.
- **`apply`'s step order.** Plan 2 adds one `continue` at the top of step 6's `for note in ordered_notes`
  loop, ahead of Plan 1's by-id branch, and runs `apply_settings` after `apply` returns. None of Plan 1's
  other passes can meet a settings row: `placed_elsewhere` needs a tombstone (never sent for a settings
  path), the seed pre-pass and `IdIndex` cover note folders only, and Task 8's `note_frontmatter_id` finds
  no `id:` in a config text.
- **Restore.** Every anchor survives `4ebc7d6`: `RestoreState`'s head (Plan 1 appends `written_ids` and
  `undone` after `confirmed_records`); `materialise`'s tombstone arm and live-arm header (Plan 1 inserts
  after the foreign-card refusal); the write arm's first lines (Plan 1 appends after
  `pending_tombstones.retain`); and `restore_all`'s adjacent `cursor.pulled_to_end = reached_end;` and
  `total.empty` (Plan 1's `undone` loop lands above them, after the `pending_tombstones` loop). R4's hash is
  `read_text`'s, so a restored settings base follows the spec's hash convention.
- **`journal::created_here` / `without_foreign_creates`, Task 7b's lineage** in `detect_external` and
  `verify_tail`: disjoint from Plan 2 (a settings file has no journal record, and no `rank` pass reads
  `config/` as notes).
- **The gap.** Plan 1's R-TD1-4 made the one writer of the cursor file outside `sync`
  (`pull_event_verdicts`) take `sync`'s run lock, because a console command runs in-process beside a
  slot's child. Plan 2 adds a second such writer, `settings_edited`, without it (Issue 3).
- Stale header only: "Tasks 1–2 at `3010cdc`"; Tasks 1–3 are built (`2d6a98b`).

**Issues:**

1. **[Tasks 4–5, `run_lines_with_client`'s push and pull; *Hand-off to the controller* item 2] (Important)
   A settings row this computer's cursor has passed without taking is never offered again, so its
   `config_refused` repeats every run and its settings never converge.** — `run_lines_with_client` sets
   `cursor.note_cursor = page.note_cursor` over every row of a page, taken or not. Three ways a row is
   passed untaken:
   - (a) **The rollout the spec calls harmless.** An old build pulls another computer's settings rows,
     refuses each ("a pulled note named a path outside the vault's notes") and advances. Once it updates it
     has no base, pushes `base: ""`, the row exists, and the push is refused; "the next pull" never brings
     the row back. This is Q13's own vault (two computers updating on different days), so "the first to
     sync provides the settings" fails for the second one, with six "another computer changed it first"
     lines on every slot, for good.
   - (b) **A downgrade** (§6.4 step 5 runs the previous release on the laptop). The old build's tombstone
     pass drops every `config/…` base from `cursor.notes` ("not a name the account can store; dropped"),
     and its `save_cursor` drops the two provisional fields (serde ignores unknown fields). Back on the
     candidate, (a) again.
   - (c) **`apply_settings`' own failure arms** (a write, or a conflict copy, that fails): the file stays
     stale with the cursor already past its row.

   Fix: when a push answers `config_refused`, and in the run after a take failed (a cursor flag), read
   `GET /sync-pull?view=settings` (`account_settings`, already built for R-TD2-9) and run `apply_settings`'
   three-way check over those paths as a synthetic page, in the same run. That makes the spec's own
   outcome ("that pull then takes the conflict branch") unconditional. Test: cursor past the row, no base,
   push refused; the same run ends with the account's text and the local one in `state/config-conflicts/`.
   Correct item 2's "an old build against the new server is harmless".

2. **[Task 5 `apply_settings`; Task 6 `settings_edited`] (Important) A provisional computer's own echo
   ends the mark for every file.** — After ending (b) on `config/runners.yaml`, the next push stores it; the
   pull after that brings the row back (`readNotes` has no device filter), `latest` is non-empty, and
   `if provisional { … clear() }` ends the mark on the five untouched files and prints "your account's
   settings arrived". Those files (skipped-panel defaults, or a fresh wizard's answers) then go up with
   empty bases. An event that is none of D18's three ends the mark, which is what R3-I1 exists to prevent
   (an old-build working computer then meets them as "first to sync"). Latent until the seam's first caller,
   but H5 writes the seam into `CLAUDE.md` as the rule. — Fix: a row ends the mark only when it is not
   this computer's own: its `device` is not `device_token(account_id)`, or its hash is not the base this
   computer last sent for that path. Test: (b), push, pull the echo; the other five stay marked.

3. **[Task 6 `settings_edited`] (Important) It loads, edits and saves the cursor file without `sync`'s run
   lock.** — Its callers are console commands, running in-process beside a slot's `sync` child, whose
   `run_lines_with_client` loaded the cursor at its start and saves it after its pull and after its push.
   A save landing between the two restores the mark and the base, so the edit is never published, while
   `settings_edited` returned `Ok(true)`. Plan 1 ruled exactly this for `pull_event_verdicts` (R-TD1-4). —
   Fix: `RunLock::try_acquire` first; a held lock is a named error the caller shows ("Knowlu is syncing; try
   again in a minute"), never a silent pass. One test with the lock held.

4. **[Task 8 `console.js` `wizGo` and `skipped`; R-TD2-14] (Important) In the settings-exist walk the name
   panel can never be reached, so a refused Finish is a dead end.** — `vault_dest_in` refuses an existing
   folder ("… already exists — pick another name"); `wizFinish` shows that on Finish, and Back now steps
   over `vault` both ways (Finish → logins → account). The parent is fixed (`default_parent`), so the name
   is the only way out. A second computer where `%USERPROFILE%\Knowlu\Knowlu` already exists (an earlier
   profile whose folder was kept, a reinstall, the proof desktop's earlier runs) cannot finish. — Fix: set a
   flag in `wizFinish`'s `!r.ok` arm that stops `skipped()` skipping `vault`, and go there. Extend H4's
   `check_second_computer`: `create_vault` answers "already exists", and Back reaches `#wiz-vault`.

5. **[Task 2 `edit_device`; Task 1 `overlay`; R-TD2-2] (Medium) A `device.yaml` born by a later-addition
   helper on a vault not yet split shadows the shared `ics_url` with `''`, and the migration then deletes
   the real one.** — `edit_device` births a missing file from `DeviceKeys::default()`, which writes
   `ics_url: ''`. `overlay` takes a device `ics_url` even when empty, so `ingest` loses its feed at once;
   `move_device_keys` then keeps the device's `''` (`set_device_ics_url(…, false)`: "a value already there
   wins") and deletes the shared line. A vault-held LMS feed URL is lost with no line. Unreachable inside
   Plan 2 (`restore_capability_url` runs only on wizard-born vaults), but Plan 2 publishes
   `insert_source_key` for Plan 3's "a login saved after onboarding", which can run on an unsplit vault:
   between an update and its first sync, and always on a vault with no account, which `sync` never splits
   (R-TD2-3). — Fix: treat an empty device `ics_url` as absent, in `overlay` and in the migration's
   `set_device_ics_url` presence test (or birth a missing `device.yaml` through `move_device_keys`). Test:
   shared `ics_url: 'https://…'`, no `device.yaml`, `insert_source_key`; `ingest` still asks that URL, and
   after `move_device_keys`, `device.yaml` holds it.

6. **[Task 6 `materialise`'s settings arm; R-TD2-17] (Minor) In a walk that asked every question, the
   restore overwrites the student's answers with no conflict copy and no line.** — R-TD2-17 rests on
   "they are the skipped panels' defaults", which holds only in the settings-exist walk. When
   `account_settings_exist` failed (`ok: false`, including a 402 for a returning student whose subscription
   lapsed, since it is asked before the subscribe panel), or the working computer published between that
   question and Finish, the student typed slots, time zone and calendars, and D18 (a)'s "copied to the
   conflict folder first when it differs" is skipped. — Fix: pass whether the page's answer was "settings
   exist" into `restore_into_as`, and keep the copy and a line otherwise. Also re-ask
   `account_settings_exist` when leaving the subscribe panel if the first answer was not `ok`, so a lapsed
   student still gets the logins-only walk.

7. **[*Hand-off to the controller* item 2, "With the commitment model's migration"; R-TD2-18] (Minor) The
   hazard is right; its consequence and its remedies need tightening before the controller acts on them.**
   - (a) A late `000100` does not produce 400s. `add constraint … check` validates existing rows, so where
     settings rows exist `db push` stops at `000100`; where none exist yet it narrows the check silently, and
     `save_config_row` later raises a check violation: a 5xx for every push that carries a settings row
     (its records and notes already stored, the device retrying for ever).
   - (b) Staging is exposed first, and soon: this plan deploys `000200` after Task 3, and the commitment
     branches may push `000100` for their own proofs with `--include-all`. The controller should read
     staging's applied list before either deploy.
   - (c) "Drop `000100`" is safe only where it was never applied (elsewhere Supabase's history mismatch
     needs `migration repair`); "re-assert the union later" does not stop `000100` running after `000200`.
   - Recommended instead: whichever stream merges second rewrites its own, still unapplied, path-check
     migration as the union and stamps it after the other's. Add a `migrations_test.ts` pin that the last
     file (by name) re-adding `sync_notes_path_check` carries the six settings paths and every folder of
     `NOTE_PATH_RE`, so any later narrowing is a red test on the branch that lands second.

8. **[Hand-off H5, Edit 6] (Minor)** "`config/runners.yaml`'s `local` entry only when `device.yaml` has
   neither" misstates the rule: `local_runner_settings` falls back per key. — Fix: "… `runners.yaml`'s
   `local` entry for a key `device.yaml` lacks".

9. **[Task 8 `create_vault_in`; R-TD2-15] (Minor)** The wizard's `zybooks_ignore` entries for a portal first
   set up here are not put back after the restore replaces `config/ingest.yaml`, so each ignored book
   returns as a map card. — Fix: insert them under `coursework.zybooks.ignore:` beside the mapping rows (a
   line insertion), or say in R-TD2-15 that they are not carried.

**Rulings assessment:**
- R-TD2-2: change. An empty device value must not win over a shared one (Issue 5).
- R-TD2-9 / S-2: extend. The same settings read must also settle a refused or passed-over path, not only a
  missing file (Issue 1).
- R-TD2-11 / S-3: keep the seam, but it must take the run lock (Issue 3) and its echo must not end the mark
  (Issue 2).
- R-TD2-14: change. Skip the name panel only until a Finish refusal needs it (Issue 4).
- R-TD2-15: extend to the ignore list, or say it is not carried (Issue 9).
- R-TD2-17: narrow to the settings-exist walk (Issue 6).
- R-TD2-18: sound; tighten the hand-off text (Issue 7).
- Sound as written: R-TD2-1 and S-1 (a widened `is_note_path` really would let a pulled `set` reach
  `write_literals` on a config file), -3, -4 with S-7, -5, -6, -7, -8, -10, -12, -13 with S-4, -16, -19,
  -20 with S-5, -21, S-6 as ruled, S-8. The function pin split (29 here, 30 in Plan 3) is consistent.

**Recommendations (advisory)**

Checked and sound, for the brief's own questions:
- **The compare-and-set is one statement** either way: `update … where encode(sha256(…)) = p_base` (the
  row lock makes a concurrent claimant re-check the committed text) or `insert … on conflict do nothing`.
  The path is re-checked inside the function, `rev` and `updated_at` move through the existing trigger (so
  the pull's read lag still covers settings rows), and a settings row never reaches `saveNotes`.
- **No capability URL reaches a synced file** on any path the plan writes: the wizard's `ingest_yaml`,
  `restore_capability_url` (now `device.yaml`), the migration, `insert_mapping`, and the pull (whose texts
  passed the pushing computer's guard). The guard refuses the rest, `http://` and unreadable YAML too.
- **The migration only moves lines:** values are read from a parse, written into `device.yaml` as quoted
  literals or inserted lines, and deleted from the shared files by span. No config file is re-dumped.
- **A card-made mapping survives every race the plan models** (conflict, copy, `apply_approved_mappings`);
  a wizard row lost to a conflict is named. The silent losses found are Issue 5 (latent) and Issue 6.
- **H1–H5 apply verbatim:** each quoted text was found exactly once at `e896047`, and H5's anchors
  survive Plan 1's H4.

Advisory:
- `apply_settings`, `restore_missing_settings` and `materialise`'s settings arm write with `fs::write`.
  Until Plan 3's *Sync now* guard lands, a `coursework` child can read a half-written `ingest.yaml`; a
  temporary file and a rename cost nothing.
- `restore_missing_settings` and the wizard's restore replace settings files without
  `apply_approved_mappings`; the spec says "after every pull that replaced a settings file". Cheap to call.
- `move_device_keys`' doc promises "nothing else moved", but the code checks only that the device values
  are gone. Deleting a block's last child turns `coursework.<source>:` into null. Compare the parsed
  document minus its device spots before and after, or soften the doc.
- `mark_provisional` marks only files present, so a shared file created later on a provisional computer
  goes up at once; marking all six paths closes it. A lost cursor (read as fresh) also drops the mark.
- The §6.4 proof's step 5 (the downgrade) will reproduce Issue 1 on the laptop. Add "no
  `config_refused` line repeats across two slots" to what the proof must show.
