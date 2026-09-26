# C3′ (the account vault) — SDD ledger

The controller ledger of the subagent-driven execution of `docs/plans/2026-09-17-c3-account-vault-plan.md` (twelve tasks, plus Task 6b and hand-offs H1–H19), 2026-09-17/25, on branch `c3-sync`, merged as PR #15 (`e2ce40c`). The ledger of the superseded 2026-09-14 C3 plan, paused at its Task 3 on 2026-09-17, follows as an appendix. Preserved verbatim from the worktree workspace (`.superpowers/sdd/2026-09-17-c3-account-vault-plan/ and …/2026-09-14-c3-sync-plan/`, git-ignored) when the stream was closed on 2026-09-25, before the worktree and its branch were deleted.

---

# SDD ledger — plan: docs/plans/2026-09-17-c3-account-vault-plan.md

- 2026-09-22 SETUP: C3′ started in parallel with C1c (Quinn: "What else can we work on in parallel for C3?"). The plan said execute after C1b merges and rebase `c3-sync` on main; C1b cannot merge until Actions minutes return, so ruling R-C3′-exec-0: `c3-sync` is rebased onto `c1b-sign-in` (`773f529`) instead — the nine paused commits replayed, two corpus-pin conflicts in `cloud/supabase/migrations/migrations_test.ts` resolved as the union (C1b's three function creations + C3's: 21+4 → 25 at the first, 26 at the second; the migrations test green at each), head `20b6ce2`; the branch was never on origin, so nothing was rewritten remotely. Stacked like `c1c-first-day`; merges after PR #9, with C1c's and this branch's overlaps (console.js, main.rs lists) resolved at that time. The old plan's ledger `.superpowers/sdd/2026-09-14-c3-sync-plan/progress.md` stays as history; this ledger is C3′'s. Old Tasks 1–3 are on the branch; Task 3's review was never dispatched — C3′'s Task 1/2 supersede those commits per the plan, so no retroactive review.
- Pre-flight scan (sonnet, `preflight-scan.md`): Table A 23 pairs, Table B 12 tasks; no interface disagreement, no rule violation. Findings and rulings: R-C3′-exec-1 every corpus pin in this plan (Task 1 Step 4's "23 → 24", any later one) is stale after C1b and the rebase — the implementer counts against today's corpus and the test's own assertion message is the truth (R-C3-exec-4 already says whichever stream moves a pin bumps it); today `migrations_test.ts` pins 26 function creations and 5 views. R-C3′-exec-2 the plan's pre-rebase SHAs (`87eb58d`, `baa5f0c`, `934fefd`) name commits now replayed as `ca5de80`, `df9c39c`, `20b6ce2`; every claim about what is "already on the branch" still holds. R-C3′-exec-3 Task 7's file list names `app/tests/commands.rs` with no step writing to it — dropped from Task 7's commit; Task 10 Step 6 owns that edit. R-C3′-exec-4 the Tauri command counts in Task 12 Step 6, ledger row R2 and hand-off H10 (43/30/62) are C1's; C1b made them 42/29/61 and Task 12 recounts by script before quoting. A6 (Task 6's Consumes list omits `sync::PAGE`) is a doc gap the dispatch carries.
- Task 1 dispatched (sonnet) from `task-1-brief.md`, BASE `20b6ce2`.
- Task 1 implementer DONE_WITH_CONCERNS at `7fea1a3` (migration `20260912000300_sync_plaintext.sql`, the sync suite rewritten with the amendment's inversions, the pin 26 → 27; two deviations recorded: the pin counted against today's corpus per R-C3′-exec-1, and a fix to the brief's `liveTables()` helper, which applied every create before every drop and so misread a drop-then-create in one file). Hand-off H1 applied by the controller at `0779b0c` (`account/index.ts`'s purge list drops `sync_generation`; the comment follows). Lint clean; Deno 430 / 0. Review package `review-20b6ce2..0779b0c.diff`; task review dispatched (opus). The staging push (`db push --include-all`, R-C3-exec-5) waits for the review to close — a pushed migration cannot be amended, only followed.
- Task 1 review (opus): Spec ✅, Quality approved; I1 (H1's comment paraphrased) taken — the plan's verbatim block swapped in at `85ea281`; that exposed a defect in the brief's own purge test (it searched the block's text for the word `sync_generation`, which the verbatim comment names): ruling R-C3′-exec-5 — the test asserts the QUOTED name, the same shape as its positive check (`99b7d96`). Four minors left to the tasks that own them (`''` in three `--` comments; `_shared/sync_rows.ts` still pins `ciphertext` by name on 000100 — Task 2/3; the JSON-validity debt — Task 2's validator; one whitespace-exact assertion). Lint clean; Deno 430 / 0. STAGING: `db push --include-all` applied `20260912000300_sync_plaintext.sql` (R-C3-exec-5); `account` redeployed (`--use-api`). Task 1: complete.
- Staging proof after the push: `sync_records (account_id, seq, device, record_hash, body, keep, received_at)`, `sync_notes (account_id, path, rev, device, deleted, body, updated_at)`, `sync_usage (account_id, bytes, updated_at)` present; `sync_generation` absent — Task 1's Interfaces exactly.
- Task 2 dispatched (sonnet) from `task-2-brief.md`, BASE `99b7d96`; carries the Task 1 review's `sync_rows.ts` ciphertext-pin minor as a pointer.
- Task 2 implementer DONE at `988c9bb` (the envelope, the key, the recovery code and `base64` gone; `ring` stays for the hash; `sync_envelope_test.ts` deleted; workspace 1270 / 1 / 4 with the accepted date test as the 1, warnings `0 other`; Deno 427 / 0, lint clean). Two notes: the brief's Cargo.toml comment contained the literal `sha2` that its own manifest-scan test forbids — the comment reworded, the test kept verbatim; `_shared/sync_rows.ts`'s `ciphertext` pin left for Task 3 as the brief allows. Review package `review-99b7d96..988c9bb.diff`; task review dispatched (sonnet).
- Task 2 review (sonnet): Spec ✅, Quality approved, nothing open. Task 2: complete. Branch `c3-sync` on origin (`988c9bb`). Next: Task 3 (`POST /sync-push`, and what a row may be) — extract its brief with the skill's `task-brief` script (check the printed path: it writes under the shell's cwd repo root), BASE `988c9bb`; hand-off H2 (config.toml + deno.json entries) is the controller's at Task 3; the `_shared/sync_rows.ts` ciphertext pin is Task 3's to retire.
- 2026-09-22 SESSION HAND-OVER: this session (Claude Fable 5.1) stops here at the Task 2/3 boundary; a new session continues from HANDOFF's ▶ RESUME HERE block and this ledger, under its own model and session trailers.
- 2026-09-22 NEW SESSION (Claude Opus 5.5, local CLI session 667ee226-3083-4b29-ad52-d0d49d535b59 — no claude.ai URL; trailers name it). Resumed at Task 3, BASE `988c9bb`.
- Ruling R-C3′-exec-6: H2 is split — `[functions.sync-push]` lands in Task 3's own commit (the implementer adds the two lines to `cloud/supabase/config.toml`), `[functions.sync-pull]` in Task 4's — because `config_toml_test.ts` fails both on a directory with no section and on a section with no directory, so the plan's "both entries at Task 3" would leave the tree red until Task 4 — costs, if wrong: one hand-off applied by an implementer instead of the controller, visible in the diff.
- Task 3 dispatched (sonnet) from `task-3-brief.md`, BASE `988c9bb`.
  (implementer agent a9fe17e784ef5511f)
- Task 3 implementer DONE at `72b03d4` (deno check/lint clean; deno test with C2's flags 438 / 0, +8 sync_rows, +9 sync-push handler; one mechanical deviation: the brief's 4-arg `assertThrows` did not type-check against `@std/assert@1.0.19`, 2-arg form used). Review package `review-988c9bb..72b03d4.diff`; task review dispatched (opus), agent ac3169643650179b2.
- Task 3 review (opus): Spec ✅; Quality NEEDS FIXES — I1 (plan-mandated): `readJson` with C1's default 1 MiB cap refuses a legitimate 500-row page with 413, the ceiling's own status, so the device reads "account full" and re-sends the same batch forever (probe: 500 notes × ~2.2 KB → 413, nothing stored). ⚠️ cross-task: the device's planned `is_note_path` accepts any character/length while `NOTE_PATH_RE` is `[A-Za-z0-9._ /-]{1,300}` — a note like `tasks/Essay (draft).md` would 400 the whole batch every slot.
- Ruling R-C3′-exec-7 (I1): the server reads a push under an explicit `MAX_PUSH_BYTES` = 4 MiB (4194304, exported from `sync_rows.ts`), and over it keeps C1's 413 ("body over …") — transport size; the ACCOUNT CEILING moves from 413 to **403** with the error `this account's copy is at its size limit` — a quota refusal, distinct by status, passes relays unaltered, and 401/402 stay the gate's; Task 5's device (not yet written) matches 403 as the named ceiling refusal instead of 413 and pages by a byte budget of ≤ 3 MiB of bodies as well as `PAGE` rows (a single note ≤ `MAX_NOTE_BYTES` always fits) — costs, if wrong: one status constant and one device test to change in Task 5; carried into the Task 5 dispatch.
- Ruling R-C3′-exec-8 (⚠️ wedge): the device's `is_note_path` applies the server's exact rule (the same character class, length ≤ 300, no `..`/`//`); a note outside it is left out of the push with one named warning line, never a batch-wide 400 — carried into the Task 5 dispatch; the cross-language agreement test pins the character class and the length, not only the folder list — costs, if wrong: a note with an unusual name is not synced until renamed, and the student is told so.
- Ruling R-C3′-exec-9: minors m1 (an over-ceiling push that adds 0 bytes is not refused — `adding > 0 &&`), m2 (U+0000 in a record or note body → a named 400), m3 (a test pairing `MAX_RECORD_BYTES`/`MAX_NOTE_BYTES`/`NOTE_PATH_RE` with `20260912000300_sync_plaintext.sql`'s checks), m5 (`deleted` must be a boolean when present), m6 (use `bytes()`), m7 (a non-object body → 400) and m12 (fixtures' `"actor":"quinn"` → `"student"`: CLAUDE.md rule 1, no person named) are taken into fix round 1 because each is a line in code the round amends — costs, if wrong: a slightly larger fix diff.
- Task 3: minor (deferred): m4 NOTE_PATH_RE admits `.`/trailing-dot/trailing-space segments Windows aliases (the column check has the same gap — Task 9's restore should refuse them); m8 records-then-notes are two statements (converges; comment); m9 `merge-duplicates` rewrites `device` on a re-pushed foreign record (harmless under P4(a)); m10 test gaps (MAX_ROWS for records, records-then-bad-note all-or-nothing, a test named "names it" asserting status only, dropped isDeviceToken/isHash cases, an import below a const); m11 `sync_db.ts:2` says five calls (six; `ceiling()` global — Task 12's scan must exempt `sync_limits`); m13 the report's GREEN evidence summarised, not verbatim.
- Task 3 fix round 1 dispatched (the implementer resumed) with I1 under R-C3′-exec-7, the R-C3′-exec-9 minors, and one m10 test; FIX_BASE `72b03d4`.
- Fix round 1 DONE at `8fcf0ac` (C3′ files 26 / 0; whole tree with C2's flags + eval 447 / 0; check and lint clean; verbatim evidence appended). Scoped re-review dispatched (sonnet) on `review-72b03d4..8fcf0ac.diff` → `task-3-rereview-1.md`.
- Fix round 1 re-review (sonnet): Closed — 9 / 9 addressed; the 403 is the only one on the path (auth 401, entitlement 401/402, verify_jwt off). Task 3: fix round 1/5 (9 addressed, 0 open; commits 72b03d4..8fcf0ac). Task 3: minor (deferred): `MAX_PUSH_BYTES` is enforced by `readJson` on UTF-16 length, not UTF-8 bytes (permissive, never a false refusal) — the name says BYTES; Task 5's device budget should count UTF-8 bytes and stay under it either way. Task 3: complete (commits 988c9bb..8fcf0ac, review clean after 1 fix round).
- `c3-sync` pushed (`8fcf0ac`). Task 4: P4 answered **accept** (HANDOFF 2026-09-22: P3–P5 built to the plan's recommendations) — not re-asked. Task 4 dispatched (sonnet) from `task-4-brief.md`, BASE `8fcf0ac`, Steps 2–4 and 6 plus H2's `[functions.sync-pull]` half (R-C3′-exec-6); Step 5 (deploy + smoke) is the controller's, after review. Agent a2e5c5f4bcd0ee8cc.
- Task 4 implementer DONE at `067fe52` (sync-pull 9/9 after RED; whole tree check/lint clean, deno test 456 / 0; deviations: an unused `assert` import dropped; `"actor":"quinn"` → `"student"`; P4 accept recorded; no pin moved). Review package `review-8fcf0ac..067fe52.diff`; task review dispatched (sonnet).
- Task 4 reviewer stalled (stream watchdog, 600 s) before writing; resumed.
- Task 4 review (sonnet, resumed): Spec ✅, Quality approved, nothing open (scoping and read lag checked in sync_db.ts; the cursor never rewinds; a full page always advances).
- Step 5 (controller) — staging deploy of `sync-push` + `sync-pull` from `067fe52` (`--use-api`): OK. Smoke (a staging session by OTP): no bearer → 401 on both, wrong method → 405 on both; pull before push → 200, cursors 0/0, empty; **push one record + one note → 502 `upsert sync_notes failed`**; pull at +0.4 s → our record absent (the lag holds); pull at +11.4 s → our record present, `seq` 1, body byte-identical, keys `body, device, record_hash, seq`; bad hash → 400 named; bad path → 400 named; no bearer → 401. FOUND BY THE SMOKE, NOT BY ANY REVIEW: Task 1's `sync_notes_path_check` is `path ~ '^(…)/[A-Za-z0-9._ /-]{1,300}\.md$'` and Postgres's regex engine caps a repetition count at 255 (DUPMAX), so every insert into `sync_notes` fails at run time with `2201B invalid regular expression: invalid repetition count(s)` although the migration applied. Also seen live: deferred minor m8 — the record half committed while the note half failed (converges on retry; stays deferred).
- Ruling R-C3′-exec-10: forward-only fix `20260912000400_sync_note_path_check.sql` drops `sync_notes_path_check` and re-adds it as the same folders and character class with `+` plus `char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303` (the 1–300 middle + `.md`, exactly `NOTE_PATH_RE`); `000300` is never edited; a static test forbids a regex repetition count above 255 in any C3′ migration (000300's superseded path check exempted by name); the m3 pairing test reads the path rule from the new file — costs, if wrong: one more forward migration.
- R-C3′-exec-10 fix DONE at `6bac948` (touched tests 20/20; whole tree 457 / 0; deviation: three older guards assumed the latest migration defines the table's current shape and now scan the whole corpus). Controller check on staging's Postgres (a SELECT, nothing written): the new expression compiles and agrees with NOTE_PATH_RE — 300-char middle ok, 301 refused, parentheses refused, unknown folder refused, empty middle refused. Review dispatched (sonnet).
- R-C3′-exec-10 review (sonnet): Spec ✅, approved with one Important (three shape guards moved to `corpus()` can no longer see a later migration drop their check) → fix round 1 (the implementer resumed): point them at 000300 explicitly.
- STAGING (controller): `db push --linked --include-all` applied `20260912000400_sync_note_path_check.sql` (the SQL is approved and the pending round touches tests only); `sync_notes_path_check` now reads the `+`/`char_length … between 4 and 303` form. Re-smoke (same session): push one record + one note → 200 `{"records":1,"notes":1,"bytes_used":183,"bytes_ceiling":209715200}`; pull at +0.4 s → neither row (the lag); at +11.4 s → both, bodies byte-identical, note keys `body, deleted, device, path, rev`; bad hash 400, bad path 400, no bearer 401. Clean-up: the smoke rows of both passes deleted (2 records, 1 note); `sync_records` 0, `sync_notes` 0, `sync_usage` sum 0 (the counter triggers decrement on delete); tok.txt deleted. Step 5: complete.
- Path-check fix round 1 DONE at `4bbe0ac` (migrations_sync_test 8/8; whole tree 457 / 0); implementer's note: the retention guard's "and not keep" now passes against 000300 through a comment naming the clause (the live clause is 000100's). Scoped re-review dispatched (sonnet).
- Path-check re-review 1 (sonnet): I1 ADDRESSED; OPEN — the retention guard's "and not keep" now passes on a 000300 comment (the live clause is 000100's `sync_prune`). Fix round 1/5 (1 addressed, 1 open; 6bac948..4bbe0ac). Fix round 2 dispatched (the implementer resumed): that assertion reads 000100.
- Path-check fix round 2 DONE at `d7b4a72` (migrations_sync_test 8/8; whole tree 457 / 0). Re-review of round 2 done BY THE CONTROLLER (a one-assertion, test-only diff, read whole): the assertion reads `20260912000100_sync.sql` by name and matches its live `and not keep;` (line 195); the other three stay on 000300 — ADDRESSED, no new breakage. Fix round 2/5 (1 addressed, 0 open; 4bbe0ac..d7b4a72). R-C3′-exec-10: complete (commits 067fe52..d7b4a72; 000400 on staging).
- Task 4: complete (commits 8fcf0ac..d7b4a72: the endpoint 067fe52, review clean; the path-check fix 6bac948..d7b4a72 after 2 fix rounds; staging deploy + smoke proven, rows cleaned).
- Ruling R-C3′-exec-11 (how R-C3′-exec-7 and -8 land in Task 5): `sync::PUSH_BUDGET_BYTES` = 3145728, counted as the UTF-8 length of each row's `dumps_value` serialisation, shared by records then notes; the row that would carry the batch past it ends that half's page (break, not continue — the cursor never advances past a row not sent); UTF-8 bytes ≥ UTF-16 units, so a budgeted page is always under the server's 4 MiB `readJson` cap (which counts UTF-16) — this also retires Task 3's deferred UTF-16/bytes minor. `is_note_path` adds the server's middle rule (after `<folder>/` and before `.md`: 1–300 chars of `[A-Za-z0-9._ /-]`), written without a regex; `build_push` skips a note whose path fails it with one named warning line and never sends it; the brief's `a_413_…` test becomes `a_403_is_the_named_ceiling_refusal` (a 403 with the ceiling's error → `CloudError::Status { code: 403 }`); the cross-language test also pins the character class and length (`[A-Za-z0-9._ /-]{1,300}` in `sync_rows.ts`; 300 accepted, 301 refused, parentheses and non-ASCII refused) — costs, if wrong: a smaller page than necessary.
- Task 5 dispatched (sonnet) from `task-5-brief.md` with R-C3′-exec-7/8/11 carried, BASE `d7b4a72`.
- Task 5 implementer DONE at `fbbc918` (sync_contract 12/12; workspace --no-fail-fast green but the accepted date test; warnings 0 other; fixtures clean; three deviations: request-line prefix assertion, fixture()'s journal seeding, one allow(non_snake_case) for the brief's PAGE test name). Review package `review-d7b4a72..fbbc918.diff`; task review dispatched (opus).
- Task 5 review (opus): NEEDS FIXES — I1 after a budget `break` in the note loop the tombstone pass tombstones every unvisited cursor note that is still on disk (`seen` holds only visited paths); I2 (plan-mandated) a 0-byte note goes up as `body: ""`, the server refuses it (`sync_rows.ts:111`) and the all-or-nothing batch wedges every push. Record cursor, byte counting (exactly the bytes sent) and the path rule verified correct; deviations 1–3 accepted.
- Ruling R-C3′-exec-12 (the wedge principle): the device predicts every refusal the server makes of a row and skips that row with one named warning, never sending it — an empty note or one containing NUL; a journal record whose body lacks a non-empty string `op` or `actor`; a tombstone whose path fails `is_note_path`. Tombstones are computed from the COMPLETE listing (`note_paths`), never from the visited set (I1); a folder whose `read_dir` fails with anything but NotFound suppresses tombstones for that folder (M2). Tests: I1's push → grow past budget → push again → no tombstone names an on-disk path; an empty note → warning, no row; a record without `op` → warning, not sent, cursor still advances past it only as the brief's oversize case does. Also taken: M4 (the page test asserts the exact remainder; a `save_cursor`/`load_cursor` round trip). Deferred: M1 (a `ts` running backwards loses local records until the clock catches up — plan-level; `skew_warnings` names it); M5 (the symlink-escape warning blames the name); M6 (the report's count and its RED step by inspection) — costs, if wrong: a few rows the student is told about instead of a wedged account.
- Task 5 fix round 1 dispatched (the implementer resumed), FIX_BASE `fbbc918`.
- Task 5 fix round 1 DONE at `0fcbf2a` (RED shown for the five behavioural tests; sync_contract 18/18; workspace --no-fail-fast green but the accepted date test; 0 other; fixtures clean). Scoped re-review dispatched (sonnet).
- Task 5 re-review 1 (sonnet): Closed — 6 / 6 addressed; named checks (a) no cursor change without a sent row or a never-retried skip, an empty/NUL note on disk can never be tombstoned; (b) an unreadable folder keeps its cursor entries. Task 5: fix round 1/5 (6 addressed, 0 open; fbbc918..0fcbf2a). Task 5: minor (deferred): the op/actor warning does not name the row; the unreadable-folder test does not assert the returned cursor still tracks the folder. Task 5: complete (commits d7b4a72..0fcbf2a, review clean after 1 fix round).
- `c3-sync` pushed (`0fcbf2a`). Task 6 dispatched (sonnet) from `task-6-brief.md`, BASE `0fcbf2a`, with A6 (`PAGE` is Task 5's), the stricter `is_note_path` (R-C3′-exec-11) and the wedge principle's pull-side corollary carried. Agent afbb52cb69b3e8172.
- Task 6 implementer DONE at `a0e00ca` (sync_contract 31/31; workspace green but the accepted date test; 0 other; fixtures clean; deviation: the brief's test 1 pinned `old: 3` against vault-s1's actual `importance: 2` — the test value corrected, not the fixture; noted: a note with no `id:` makes local journal lookups by id miss and falls back to reconcile's mtime path). Review package `review-0fcbf2a..a0e00ca.diff`; task review dispatched (opus).
- Task 6 review (opus): NEEDS FIXES — three plan-inherited defects reproduced by probe (scratchpad/probe): I1 the brief's `ts >= first_ts` filter drops this device's older edit from `upstream_records`, reconcile falls back to the mtime contender, and a later unrelated edit makes a genuinely later foreign write lose with no card (both desktops then diverge for good); the suite cannot see it because vault-s1's note has no `id:`. I2 a pulled record's `field` is never checked — `"zz\n---\nhijack"` rewrites the frontmatter fence. I3 NTFS: `exists()` is case-blind, so a case-only rename's tombstone (pulled back from our own push) archives the live note. Guard order, the no-panic property and the new-note bound against overwrite verified.
- Ruling R-C3′-exec-13 (Task 6 fix round 1): I1 — drop the `ts >= first_ts` term; `mine` excludes records whose actor is `sync::ACTOR`; add a conflict test whose temp note carries an `id:` (the role reversal: this device's older human edit + a later unrelated edit vs a later foreign human value → a card, never a silent loss). I2 — refuse (count in `refused`, one warning) any record whose `field` is present and is not an identifier (`[A-Za-z_][A-Za-z0-9_]*`, char checks) or is `id`. I3 + M6 — a path is present only on an exact-case match of its file name in its parent directory; the new-note write uses `create_new(true)`. M1 — the plan's second new-note bound ("its record must already have been appended") is retired as self-contradictory (it would drop every hand-made note and every card meant for the other desktop): the enforced bounds are `is_note_path` + no exact-case file + create-new; the comment at sync.rs:770 corrected. M2 — `supersede` records are device-local: `build_push` skips `op == "supersede"`; `apply` gets `debug_assert_eq!(ctx.actor, ACTOR)`. M3 — a newer foreign value for a field with a pending card updates that card's proposed value through `write` (journaled), never leaves a stale `to`. M4 — a live pulled note row with no body is skipped with a warning. M5 — a pulled `set` with no `id` gets a warning. M7 — `ts` must be canonical (`now_ts(Some(parsed)) == ts`) or the record is refused — costs, if wrong: a slightly larger fix round.
- Task 6 fix round 1 dispatched (the implementer resumed), FIX_BASE `a0e00ca`.
- Task 6 fix round 1 DONE at `719ec75` (sync_contract 38/38; RED for the eight affected tests against a0e00ca; workspace green but the accepted date test; 0 other; fixtures clean). Note: M3 settles and refiles the stale card rather than editing it (its `changes:` block is block-style YAML, unsafe for single-line surgery). Scoped re-review dispatched (opus).
- Task 6 re-review 1 (opus): 9 / 9 ruled items ADDRESSED; named check: removing the floor cannot create false conflicts (resolve decides from values; mine only picks the contender) — probe N1 clean; no new breakage. OPEN: O1 (the review's second I3 case — a case-only rename on the OTHER desktop: the live row for the new spelling fails create_new on NTFS, then the tombstone for the old spelling archives the note here; pre-existing, now warned). Recommended with it: O2 (three desktops: `mine` excludes only foreign[0]'s device — a second foreign desktop's record becomes up_latest → silent loss, probe N6), O3 (a snoozed stale card is not settled — a second card files beside it; the lookup keyed on `carded` can miss a card filed with fewer fields), O4 (both sides hold the same value → a `from: 5 to: 5` card costing a cap unit — the natural end of every approved sync card). O5 (the identifier rule refuses hand-edit keys like `due-date`) — noted only.
- Ruling R-C3′-exec-14 (fix round 2): O1 — within one pull, a tombstone whose path equals a live row's path case-insensitively is dropped with a warning (a case-only rename, never a delete); the local file is renamed to the live row's spelling in two `std::fs::rename` steps through a temp name in the same folder (the note's id and body unchanged; the foreign `move` record, if any, is already journalled), and if the rename fails the note keeps its old spelling with a warning — never archived. O2 — `mine` excludes every record appended by this pull (all foreign devices in the page), not one device. O3 — the stale-card settle also covers a `snoozed` card for the same note and field; the lookup is keyed on the fields the card's `changes` names. O4 — a carded change whose `from == to` is dropped (no card, no cap unit). Each with a test — costs, if wrong: a case-only rename handled as a rename rather than a conflict.
- Task 6 fix round 2 dispatched (the implementer resumed), FIX_BASE `719ec75`.
- Task 6 fix round 1/5 closed at re-review (9 addressed; O1 open + O2/O3/O4 folded in by R-C3′-exec-14; a0e00ca..719ec75). Fix round 2 DONE at `d06d38c` (sync_contract 42/42; RED for the four new tests; workspace green but the accepted date test; one run hit a flaky loopback failure in cloud_contract.rs that cleared on re-run and in isolation — recorded; 0 other; fixtures clean). Scoped re-review 2 dispatched (the round-1 re-reviewer resumed, opus).
- Task 6 re-review 2 (opus, probes against d06d38c and a 719ec75 export): O1–O4 ADDRESSED (N2, N4, N4b, N5, N6 clean; the rename and the move pass cannot duplicate or lose a note). NEW BREAKAGE: B1 (O4's filter skips the M3 settle when every carded field converges — a stale pending `4→5` survives both desktops holding 4; regression vs 719ec75), B2 (O2 excludes by DEVICE, not by this pull's records — an earlier-applied B record vanishes from `mine`, the mtime stand-in wins, C's later write loses silently in both page orders; regression), B3 (O1 matches by name only — another desktop's delete of `cs-100-hw-01.md` plus an unrelated new `CS-100-HW-01.md` renames our deleted note to the new spelling and the next push overwrites theirs; rare).
- Ruling R-C3′-exec-15 (fix round 3): B1 — for every field O4 drops as `from == to`, settle a pending/snoozed card for that note and field (`find_amend_card`) with no refile. B2 — `mine` excludes only the records whose hash is among the records THIS pull appended (as R-C3′-exec-14 said). B3 — both halves of O1 apply only when the pulled text's `id:` equals the local file's `id:`; otherwise the ordinary path runs, with this page's tombstones processed before its live rows so the new note can land. Deferred (recorded): a tombstone arriving in an earlier PAGE than its live row (outside "within one pull"); the case rename is not journalled (per the ruling's fs::rename); the temp name has no `.md`; `live_lower` built before the path check (unreachable under the server's rule); a snooze is not carried across a refile — costs, if wrong: rare paths stay as warned today.
- Task 6 fix round 3 dispatched (the implementer resumed — the last round before escalation), FIX_BASE `d06d38c`.
- Task 6 fix round 2/5 closed at re-review 2 with B1–B3 open (719ec75..d06d38c). Fix round 3 DONE at `9ddcf4f` (sync_contract 45/45; RED for the three new tests; probe r2: N7 cards=0 live=[], N8 cards=1 both orders, N9 moved=1 notes_written=1 id_on_disk=task_0000000999; workspace green but the accepted date test; 0 other; fixtures clean). Note: B1 scoped to single-field cards. Re-review 3 dispatched (the re-reviewer resumed, opus).
- Task 6 fix round 3/5 (9ddcf4f) re-review 3 (opus; probes N2–N12 against 9ddcf4f, d06d38c and 719ec75 exports): B2, B3 ADDRESSED; B1 ADDRESSED for single-field cards. OPEN, LOAD-BEARING: R1 — B2's hash filter keeps an EARLIER foreign record that never took effect here (withheld by a card) in `mine`; it becomes `up_latest`, its `new` ≠ the note's value, reconcile falls back to the mtime stand-in and a later unrelated local edit wins — B's next value loses silently in the common "card pending, other desktop edits again" flow (probe N12; a regression vs d06d38c/719ec75, whose device filters hid it by accident). Deferrable: R2 (the convergence settle retires ANY kind:amend card, including a judge-once proposal from agent:knowlu.enrich — N11a/N11b); D1 (a stale sync card survives whenever its field set is not exactly the set the pull resolves — N10a/b/c, multi-field cards reachable when one pull carries two conflicting fields).
- Ruling R-C3′-exec-16 (fix round 4 — a FRESH implementer on opus, per the loop's escalation): R1 — additionally drop from `mine` every foreign-device `set` whose `new` is not the note's current value for that field (a write that never took effect here); add N12 as a test. R2 — `find_amend_card` matches only cards whose `created_by` is `sync::ACTOR`. D1 — settle every pending/snoozed `sync::ACTOR` card on the target that names ANY field this pull resolved (converged or carded), then file one fresh card for the remaining `changes`; tests from N10a/b/c. Every earlier probe (N2–N12) stays green — costs, if wrong: one more round and the breaker.
- Task 6 fix round 4 dispatched (fresh, opus), FIX_BASE `9ddcf4f`.
- IN FLIGHT at the 2026-09-23 compaction point: Task 6 fix round 4 (fresh opus implementer, agent a7dd0c525c82e421f, FIX_BASE 9ddcf4f, ruling R-C3′-exec-16). On its report: package FIX_BASE..HEAD, scoped re-review (resume the re-reviewer a7fa20944a5aa911b — it holds the probes), ledger `Task 6: fix round 4/5`. Round 5 is the last; after it the breaker adjudicates. Then Task 7.
- Task 6 fix round 4 DONE at `75bf8a5` (fresh opus; R1, R2, D1 each RED at 9ddcf4f then GREEN; sync_contract 49/49; workspace 1319 / 1 / 4, the 1 the known date test; oracle 3/3, surface_oracle 4/4; 0 other; fixtures clean; probes N2–N12 as ruled, N12 refiles `4 -> 6`). Implementer's notes for the re-review: D1 also carries forward a field a settled card named that this pull did not resolve (re-based on the note's current value — else N10a drops the open effort_hours conflict); a field applied cleanly is not "resolved" (literal "converged or carded"; three-desktop stale `from` possible); a judge-once card and a sync card can both be live on one field (R2 as ruled); the comment at sync.rs:912-916 credits `validate_amendment` with the `from` check (it is `apply_amendment`'s).
- Found by the round-4 implementer, outside C3′: `app/tests/commands.rs::sync_on_a_non_repo_vault_is_calm_and_backup_needs_a_folder` flakes (2 of 3 runs) — it never removes its `%TEMP%\qo-sync-data-<pid>` app-data dir, 131 stale ones exist with `backup_dir` set, and a reused PID breaks it. For C0 Task 6's hygiene list (and clean the stale dirs).
- NEXT (post-compaction): package 9ddcf4f..75bf8a5, scoped re-review 4 by resuming the re-reviewer (it holds the probes; ask it to judge the carry-forward and the clean-apply scope), ledger `Task 6: fix round 4/5`. Round 5 is the last; then the breaker.
- 2026-09-23 (post-compaction) Task 6 scoped re-review 4 dispatched: the re-reviewer a7fa20944a5aa911b resumed (opus) on review-9ddcf4f..75bf8a5.diff; asked to judge (a) D1's carry-forward, (b) the clean-apply scope, (c) a judge-once card and a sync card live together, (d) the sync.rs:912-916 comment; output task-6-rereview-4.md.
- Note (2026-09-23, controller): C1c Task 8 (R-C1c-8) adds `Scheduler.live` and routes every step `run_slot_inner` records through a recorder in `app/src/scheduler.rs`. Task 7's H8a (the slot's sync step) edits the same function. C1c merges first (it is ahead), so H8a will apply onto C1c's recorder: a new step goes through the recorder, not a bare `steps.push`. Carry this into Task 7's dispatch.
- Task 6: fix round 4/5 (9ddcf4f..75bf8a5) closed at re-review 4 (opus; probes N2–N16; task-6-rereview-4.md). R1 ADDRESSED; R2 ADDRESSED. D1 OPEN, LOAD-BEARING: N16. `live_sync_cards` matches on `created_by`, which every desktop shares, so this device settles a card that travelled in as a note. `build_push` then tombstones it and archives the other desktop's open card. Result: silent, permanent divergence (effort_hours 3 vs 4, no card anywhere); at 9ddcf4f the card survived. Deferrable:
  - (a) carry-forward resurrects a settled conflict (N14, N14b);
  - (b) a cleanly applied cardable field is not counted as resolved (N13, three desktops only; not silent);
  - (c) a judge-once card and a sync card on one field: two visible cards, no wrong write (N15). N15b: a pending sync card suppresses the judge's re-proposal (write.rs:336 `find_pending_amendment` ignores `created_by`);
  - (d) the sync.rs:915 comment credits `validate_amendment`, and write.rs:240 has the same slip.
- Ruling R-C3′-exec-17 (fix round 5, the LAST before the breaker). Fix, in this order:
  - (1) N16: `live_sync_cards` matches only cards THIS device filed, i.e. whose `id` has a `create` record in this device's own journal with `device == device_name()`. No new frontmatter key. N16 becomes a test.
  - (2) (a): carry a field forward only while the note still holds the settled card's own `from` for it; otherwise drop it. N14 and N14b become tests; N10a stays green.
  - (3) (b): a cardable field applied cleanly by this pull counts as resolved for SETTLING (never as a change). N13 becomes a test.
  - (4) (d): both comments corrected (comment-only in write.rs).
  - Every earlier probe (N2–N12) and N10a/b/c stays green.
  - Parked for C3′ Task 12's final review, not this round: (c) N15, which is user-visible but writes nothing wrong, and N15b, which is in write.rs's judge-once path, outside Task 6's files.
  - The round goes to the round-4 implementer, resumed (a7dd0c525c82e421f). It is already on the top tier and wrote the D1 code this round corrects, so a fresh agent would buy nothing a resume does not.
  - Cost if wrong: the breaker adjudicates whatever stays open after re-review 5.
- Task 6 fix round 5 dispatched (round-4 implementer resumed, opus), FIX_BASE `75bf8a5`.
- Task 6 fix round 5 DONE at `ff716e3` (sync_contract 53/53: N16, N14, N14b and N13 RED at 75bf8a5, then GREEN; workspace 1323 passed / 1 accepted date test / 4 ignored; oracle 3/3; surface_oracle 4/4; 0 other; fixtures clean). Beyond the ruling, write.rs:598-601 got the same comment correction. Concerns for the re-review and the breaker: (i) a travelled card shows on the other desktop but is unapprovable there (stale); (ii) (b)'s clean-apply settle with a late-arriving older third-desktop write drops B's later value (three desktops, silent); (iii) cards filed under an old hostname are never settled by sync. Scoped re-review 5 dispatched (the re-reviewer resumed, opus) on review-75bf8a5..ff716e3.diff.
- Task 6: fix round 5/5 (75bf8a5..ff716e3) closed at re-review 5 (opus; probes N2–N19; task-6-rereview-5.md). N16, (a), (b) and (d) are all ADDRESSED; N2–N16 hold. OPEN:
  - (i) LOAD-BEARING, present since round 1 (N18). A sync card travels to the other desktop as a note. Approving it there takes the already-applied branch, and the `executed`/`rejected` status travels back. The filing desktop's approvals pass then archives its real card without applying it: `importance` 4 vs 5, no card anywhere, after the student approved 5.
  - (ii) DEFERRABLE, a round-5 regression (N17). A late, older write from a third desktop applies cleanly and settles the card under a false warning; the desktops diverge silently. It needs three desktops and late delivery.
  - (iii) DEFERRABLE, a round-5 regression (N19). After a hostname change, old cards are orphaned: a second card is filed beside them.
- BREAKER (the loop is spent; controller adjudication):
  - Ruling R-C3′-exec-18.
    - (i) is load-bearing: a student's explicit approval ends in silent divergence. It gets ONE scoped fix outside the loop: **a sync card never leaves its device.** `build_push` skips every note whose `created_by` is `sync::ACTOR`, and every move or tombstone of one. It is device-local by construction: its `from` is this device's withheld value, so only this device can answer it. N18 becomes a test.
    - (iii) rides along, because it is one line in the same function: ownership keys on a `create` record whose actor is `sync::ACTOR` (such records never leave the device, so it is rename-safe). Hardening: a pulled record whose actor is `sync::ACTOR` is refused with a named warning (the wedge principle's pull-side corollary). N19 becomes a test.
    - Verification: the re-reviewer re-runs N2–N19 on the fix commit. That is the last look. Anything it finds beyond (i) and (iii) is parked for Task 12's final review, not a new round.
  - Ruling R-C3′-exec-19: parked for C3′ Task 12's final review, each with its recipe.
    - (ii) N17: settle on a clean apply only when the applied write is later than the write the card offers.
    - (c) N15: a judge-once card and a sync card on one field show as two cards; nothing is written wrongly.
    - (c) N15b: `write::find_pending_amendment` (write.rs:336) ignores `created_by`, so a pending sync card suppresses the judge's re-proposal.
  - Costs if wrong: the three-desktop and judge-once edges wait for the final review.
- Task 6 breaker fix dispatched (the round-4/5 implementer resumed, opus), FIX_BASE `ff716e3`.
- STOPPING POINT (2026-09-23, Quinn to bed). IN FLIGHT: the Task 6 breaker fix (R-C3′-exec-18; agent a7dd0c525c82e421f, FIX_BASE ff716e3, writing task-6-breaker-report.md). On resume: read the report, package ff716e3..HEAD and have the re-reviewer (a7fa20944a5aa911b) re-run N2–N19 as the last look (anything new is parked for Task 12), then ledger `Task 6: complete` and move to Task 7 (carry the scheduler.rs overlap note above).
- Task 6 breaker fix DONE at `63612c9` (sync_contract 57/57: 4 new, and N16 reworked because a card can no longer travel; workspace 1327 passed / 1 accepted date test / 4 ignored; oracle 3/3; surface_oracle 4/4; 0 other; fixtures clean). Five push paths are closed: the note, live or archived; the create/settle records (already ACTOR); every status, snooze, executed and archive record carrying the card id; and the tombstone, even from an old cursor. Mutation-checked. Concerns: the re-reviewer's N18 probe premise no longer holds (probebrk hand-delivers a copy); REJECTING a sync card still leaves the desktops divergent, because a reject writes nothing to the task (pre-existing, outside sync.rs); pre-existing card notes on a staging account are not refused on pull; build_push reads the whole journal and approvals/archive twice. Last look dispatched (the re-reviewer resumed) on review-ff716e3..63612c9.diff.
- Task 6 last look (re-reviewer, opus; task-6-rereview-final.md): (i) ADDRESSED; (iii) ADDRESSED; N2–N19 hold. The card filter drops only sync-card records, and an approved card's new value still travels as the task's own record (`agent:approvals`). OPEN:
  - the REJECT concern, LOAD-BEARING before sync is switched on (N2x): a rejected sync card leaves A at 4 and B at 5 with no card, so the student's explicit choice never reaches B;
  - F1, DEFERRABLE (N22): a sync-card note that an older build stored on the account is still accepted on pull;
  - F2, DEFERRABLE: `SyncCards::find` treats every `create` under `sync::ACTOR` as a card, a latent trap.
- Task 6: complete (`63612c9`; the loop ran 5 rounds, the breaker fix, and a last look).
- Ruling R-C3′-exec-20 — Task 6b, added: a rejected sync card re-asserts this device's value.
  - In the rejected branch of `approvals::transition_note`: for a card with `created_by: agent:knowlu.sync`, append one journal-only `set` per field, carrying the task's id, `old` = the card's `to`, `new` = the note's current value, a fresh `ts` and the deck's actor.
  - Skip a field whose note no longer holds the card's `from` (a later local edit is already travelling).
  - This goes through a new append-only helper (e.g. `write::reassert`), because `write_literals` drops a no-op write. No change to sync.rs.
  - N2x becomes a test (two desktops, end to end): reject on A, and B converges on A's value, or cards it if B has moved on.
  - It sits outside Task 6's files, so it is its own task with its own review. It is done before Task 7, because Task 7 switches sync on in the slot.
  - Implementer: the Task 6 round-4/5/breaker implementer, resumed (opus; it holds the sync and approvals context). Reviewer: the re-reviewer, resumed (it holds probe N2x).
  - Cost if wrong: one small task.
- Ruling R-C3′-exec-21: F1 and F2 are parked for Task 12's final review, with the recipes in task-6-rereview-final.md:
  - F1: `apply`'s note pass refuses a live note row that `sync_card_note` recognises, with a named warning;
  - F2: `SyncCards::find` also requires `kind == amend` or an approvals/ path, with a test that a pulled note's later edit still travels.
- Task 6b dispatched (implementer resumed), BASE `63612c9`.
- Task 6b implementer DONE at `520eda1`:
  - `write::reassert` (append-only), used by the rejected branch of `approvals::transition_note`. sync.rs is unchanged.
  - sync_contract 61/61: N2x and the skip case RED at 63612c9 then GREEN; the judge-once-reject and approve guards pass on both sides.
  - Workspace 1331 / 1 accepted / 4; 0 other; oracle and surface_oracle unchanged; probes N2–N19 unchanged.
  - Concerns: the re-assert runs as `agent:approvals` (the executor's actor), so on the other desktop judge-once does not treat it as a human setting. A failed archive re-asserts twice, which is harmless.
- Ruling R-C3′-exec-22: the re-assert keeps `agent:approvals`. It is consistent with the approve path, whose applied value also travels as `agent:approvals`. Whether a student's decision on a card should count as a human setting for judge-once is one question for both paths, parked for Task 12's final review with N15/N15b. Cost if wrong: a later agent judgment on the other desktop may re-set a decided field without a proposal, until that ruling.
- Task 6b task review dispatched (the re-reviewer resumed, opus: it holds N2x and the sync context) on review-63612c9..520eda1.diff.
- Task 6b review (the re-reviewer, opus; task-6b-review.md): SPEC ✅, QUALITY approved. N2x converges both at 4; N23 (the other desktop moved on) files 6→4; N24 (the other desktop is later) files 4→6; N2–N19 unchanged; 113 approvals/write unit tests pass; sync_contract 61/61. Minors: m1, N23/N24 not yet contract tests; m2, approvals reaches into `crate::sync::ACTOR`, a layering nit; m3, a failed settle re-asserts again, confirmed harmless. Ruling: m1 and m2 parked for Task 12's final review (m1 is cheap coverage worth adding there); m3 needs nothing.
- Task 6b: complete (`520eda1`).
- Task 7 brief extracted (task-7-brief.md), plus the three hand-offs (task-7-handoffs-H4a-H8a-H9a.md: the preamble, H4a, H8a and H9a verbatim from the plan) and the plan's Global Constraints (global-constraints.md; its trailer lines are the old session's, and the dispatch overrides them).
- Ruling R-C3′-exec-23: the Task 7 implementer applies H4a, H8a and H9a verbatim from the plan text, as their OWN separate commit ("hand-off H4a/H8a/H9a (C3′ Task 7)"), after the task's commits; the review checks it line by line against the plan. This follows R-C3′-exec-6's precedent. The plan's controller-commit rule exists so shared-file edits are traceable and verbatim, and a separate, named commit whose diff is checked against the text keeps both. The alternative is the controller hand-applying about 180 lines across four shared files. Cost if wrong: the hand-off's author line in git reads as the implementer's.
- Carried: R-C3′-exec-3 (app/tests/commands.rs is dropped from Task 7's commit; Task 10 owns it). The scheduler.rs overlap with C1c (see the note above) is merge-time, not now.
- Task 7 dispatched (sonnet), BASE `520eda1`.
- Task 7 implementer DONE: `f03c84b` (engine: `knowlu-engine sync`, first in the slot, always exit 0, one line each; sync_contract 66; scheduler tests) + `a650877` (hand-off H4a/H8a/H9a, verbatim; seven adjacent doc comments that named the removed git sync were reworded, as listed under 'Hand-off deviations'). Workspace green except the accepted date test; 0 other; fixtures untouched; oracle and surface_oracle unchanged; 0 CRs. Review package review-520eda1..a650877.diff; task review dispatched (opus: shared files, a replacement hand-off).
- Task 7 review (opus; task-7-review.md): SPEC ✅ (the interfaces, skip strings, six tests and H4a/H8a/H9a are verbatim; exactly one `state::run_sync`; sync_contract 66/66; scheduler 26+1; 0 other). QUALITY: changes needed.
  - I1 (plan gap): nothing stops the slot's sync child and Sync now or the quit push from running together. Foreign records get appended twice, and one conflict gets two cards.
  - I2: `SyncStatus` drops the skip reason, so a signed-out Sync now reads as in step.
  - I3: every pull failure reads "the service refused", offline included.
  - I4 (plan gap): the slot's sync is a child, so `cs.sync` is never filled and the page says "not synced yet" all day.
  - M1: the vault_io comments claim the slot sync takes the lock.
  - M2: stale git comments.
  - M3: the reworded scheduler comment breaks H8b's verbatim anchor.
  - M4: quit_flush reports `synced: true` on a skip.
  - M5: no loopback test of `run_lines_with`'s network half.
  - M6: a pulled note pushed back late can overwrite a newer account body.
  - M7: the push is attempted after a network-failed pull, holding `vault_io` up to 240 s.
  - Also shown: H9a removed the debounced (30 s) and 5-minute syncs, so edits reach the account only at a slot, Sync now or quit.
- Ruling R-C3′-exec-24 (fix round 1):
  - (I1) `run_lines_with` takes an exclusive, non-blocking OS file lock on a file under `state/` (std `File::try_lock`, stable since 1.89; the toolchain is 1.98; no new crate). Every sync takes it: the slot child, Sync now and the quit push. A run that finds it held is a named skip, `sync (skipped: another sync is running)`, at exit 0. The type is NOT named `SyncLock` (Task 10's test forbids that name in engine/src).
  - (I2) `SyncStatus` gains `skipped: Option<String>`, filled by `SyncStatus::of` from `Totals.skipped`. The brief's interface grows by one field, which Task 10's page renders.
  - (I3) Pull and push failures are named by cause: transport/offline ("offline: the account could not be reached"), 401 ("signed out"), 402 ("no entitlement"), and other statuses ("the service refused (<status>)").
  - (M7) After a transport failure on the pull, the push is not attempted; the line says so.
  - (I4) `run_lines_with` persists its `SyncStatus` on every run to `state/sync-status.json` (through `ledger::dumps_value`, written atomically). The app loads `cs.sync` from it at open and after the slot's sync step, and Sync now keeps setting it directly. So H10's claim holds, and a failed slot sync reaches the page. state/ is never pushed.
  - (M1, M2) The comments are made true.
  - (M4) quit_flush reports a skip as a skip.
  - (M5) A `CloudClient` seam gets a loopback test of the pull→apply→save→push→save half, covering I1's lock, I3's offline naming and M7.
  - (M3) Not fixed. Restoring a comment that says git would be false; Task 10 applies H8b's INTENT at that spot and records the moved anchor.
  - (M6) Parked for Task 12's final review. Recipe: the push of a pulled note carries the base version it was pulled at, and sync-push refuses a stale base (a wedge-principle skip on the device).
  - Cost if wrong: one more round.
- Guess (the away log's G1): sync cadence stays as the plan has it (slot, Sync now, quit), with no debounced sync for the pilot. Most students have one desktop, and the lag matters only for a second one. To undo: a debounced engine sync after console writes (it would take the I1 lock).
- Task 7 fix round 1 dispatched (the implementer resumed), FIX_BASE `a650877`.
- Task 7 fix round 1 DONE at `a3e5d72` (I1–I4, M1, M2, M4, M5 and M7, each behaviour RED at a650877 then GREEN; sync_contract 76; scheduler 28+1; workspace green except the accepted date test; 0 other; fixtures clean; 0 CRs). Scoped re-review 1 dispatched (the task reviewer resumed, opus) on review-a650877..a3e5d72.diff.
- Task 7: fix round 1/5 (a650877..a3e5d72) closed at re-review 1 (opus; task-7-rereview-1.md). ADDRESSED: I1 (`RunLock` on state/sync.lock, released on every path; state/ never pushed), I2, I3, I4 (atomic on all five paths), M1, M4, M5, M7. M2 PARTIAL (state.rs:250-252 and :228). New findings:
  - N1, Important: sync.rs:236 and :253 spell `SyncLock` in doc comments, which Task 10's gate scans.
  - N2, Important: a 403 size-limit refusal lost the server's reason ("this account's copy is at its size limit").
  - N3, Minor: the lock-held skip writes the status file.
  - N4, Minor: `SyncError::Service` changed shape, so Task 9's plan code (:4175) will not compile as written.
  - N5, Minor: a failed status rename is silent, and a crash leaves a `.tmp-<pid>`.
  - N6, Minor: the sync.rs:97-98 doc contradicts :106.
  - Carry-forward: at Task 8, the entitlement gate (H4b, main.rs) refuses before `sync` runs, so no status file is written and the page keeps a stale "in step".
- Ruling R-C3′-exec-25 (fix round 2):
  - N1: the doc comments name the lock without that word.
  - N2: every refusal keeps the server's reason when the body has one: `the service refused (<status>: <reason>)`, clipped with `judge::one_line`. The 403 size limit must read the reason.
  - M2: the two comments made true.
  - N3: the lock-held skip writes NOTHING, as ruled. Writing could clobber the running sync's status, which lands at its end; the test asserts the status file is untouched.
  - N5: a fixed temp name (`sync-status.json.tmp`, overwritten each save), so a crash leaves at most one stale file that the next save replaces; a failed save adds one line to the run's output.
  - N6: the doc fixed.
  - N4 goes to Task 9's dispatch: its `SyncError::Service` match follows the new shape.
  - The carry-forward goes to Task 8's dispatch: the gate's refusal writes a `SyncStatus` with `skipped: no entitlement` before it exits, or the page reads the gate's state.
  - Cost if wrong: one more round.
- Task 7 fix round 2 dispatched (the implementer resumed), FIX_BASE `a3e5d72`.
- Task 8 prep: task-8-brief.md extracted, and task-8-handoffs-H3a-H4b.md (the preamble, H3a, H4b verbatim). P5 (Task 8 Step 1, a Quinn question) was reached while Quinn is away. Per the plan's own fallback ('Without a ruling: Task 8 ships (a) and Task 12's report names the open half'), it is built to (a): the four cloud steps are gated and `rank` keeps ranking. It is logged for Quinn as the away log's G3; the undo is one line (`Command::Rank` joins H4b's gated arm) plus the §5.1 amendment.
- Task 7 fix round 2 DONE at `1860898` (N1, N2, M2, N3, N5 and N6; behaviour RED at a3e5d72 then GREEN; sync_contract 79; workspace green except the accepted date test; 0 other; 0 CRs). Scoped re-review 2 dispatched (the task reviewer resumed).
- Task 7 re-review 2 (opus; task-7-rereview-2.md): APPROVED. N1, N2, M2, N3, N5 and N6 are ADDRESSED; the only `SyncLock` left is in history.rs, which Task 10 deletes. Minors:
  - R2-1: a server reason can still carry control or bidi characters (NUL, ESC, U+202E, U+200B) through `one_line`;
  - R2-2: a failed slot status save reaches only the slot log;
  - R2-3: a failure to open the lock file writes the status without the lock.

  Ruling R-C3′-exec-26: all three are parked for Task 12's final review. Recipes: R2-1 is best fixed in `judge::one_line` itself (drop `char::is_control` and the bidi/zero-width set), which every server string passes through; R2-2 adds a `last_error` from the slot's own read-back; R2-3 skips the save when the lock cannot be opened.
- Task 7: complete (`1860898`; H4a/H8a/H9a at `a650877`).
- Task 8 dispatched (sonnet), BASE `1860898`, with:
  - P5 built to (a) (away log G3);
  - H3a and H4b applied by the implementer, verbatim, as named commits (R-C3′-exec-23's precedent); H3a is compile-blocking, so it may ride in the commit that creates entitle.rs, and the message names it;
  - the carry-forward from Task 7 re-review 1: when H4b's gate refuses `sync`, it first persists a `SyncStatus` with `skipped: Some("no entitlement")`, so the page does not keep a stale "in step".
- Task 8 implementer DONE: `5f76903` (entitle.rs + entitlement_gate.rs, with hand-off H3a), `7721c47` (hand-off H4b, verbatim), `ee36b6c` (the carry-forward: `sync::record_gated_skip` persists `skipped: no entitlement` before the gated `sync` line). entitlement_gate 9/9; workspace green except the accepted date test; oracle 3/3 and surface_oracle 4/4 unchanged; 0 other. Notes: the brief's Step 2 code used an undefined `temp()` helper, which was supplied in repo style; the scheduler tests use `KNOWLU_ENGINE_EXE=cmd` and so never hit the gate. Task review dispatched (opus).
- Task 8 review (opus; task-8-review.md): SPEC ✅; H3a and H4b line for line; entitlement_gate 9/9; sync_contract 79/79; 0 other. QUALITY: changes needed.
  - C1, Critical: a slot at launch runs before the app's first entitlement refresh, so the gate refuses a new student's first sync, coursework and ingest, and the first slot after more than 72 h away is skipped the same way.
  - I1: the app and the engine read the cache under different profile ids once a vault's path changes (a drive letter, a restore).
  - I2: no test on the 72 h or clock-skew boundary.
  - I3: nothing asserts `cache_path_in`, and no `gate_in` test on an active cache.
  - I4: exit 0 is checked only by grepping main.rs.
  - M1: `record_gated_skip` ignores the sync lock.
  - M2: the gated line is composed in two places.
  - M3: the gated line reaches only the slot log, not the Runs view, though H4b's comment claims it does.
  - M4: `profile_id` accepts `..` and backslashes.
  - M5: ee36b6c's main.rs edit is not in the plan's hand-off list.
  - M6: cosmetics.
  - Also noted: the "accepted" date test is branch-only.
- Controller: merged origin/main into c3-sync, clean `--no-ff` at `8e6d8e2` (local; the push is held). It brings main's clock pins and retires the date-test exception; the ownership check `main...c3-sync` is unaffected.
- Ruling R-C3′-exec-27:
  - (C1) Fixed at its root, in the code that owns it: C1c Task 2's synchronous in-slot entitlement refresh (`run_slot_inner`, c1c branch). Today it fires only when the cache is MISSING; C1c Task 10 (R-C1c-11) extends it to a cache past the grace. C1c merges before C3′, so the merged product never gates a launch slot on a stale or missing cache. C3′'s own branch alone keeps the race, and C3′'s exit gate records C1c as a merge prerequisite. No scheduler.rs hand-off is added on c3-sync; it would conflict with C1c's version of the same lines.
  - Fix round 1:
    - (I1) The engine resolves the cache exactly as the app does: find the app's lookup (main.rs:142, profiles.rs:24) and mirror it. If that is profiles.json's entry whose `vault` equals this vault, use it, falling back to the cloud.yaml-derived id. Test a vault path that changed.
    - (I2) Tests on both sides of the 72 h grace and of the skew boundary, plus a pin that the app's comparison lines agree.
    - (I3) Assert `cache_path_in`'s exact path, and `gate_in` → None on an active cache.
    - (I4) A test that spawns the built engine with `LOCALAPPDATA` in a temp dir and asserts exit 0 and the line.
    - (M1) `record_gated_skip` takes the lock and writes nothing if it is held.
    - (M2) The line is composed in one place.
    - (M3) The gate also appends `<cmd> (skipped: no entitlement)` to `state/runner-log.md` through `cli::append_run_log` with status `ok` (R-C1c-plan-4's convention), and H4b's comment is made true.
    - (M4) `profile_id` accepts only the app's shape (`profile_` + 10 lowercase hex).
    - (M6) Cosmetics.
  - (M5) A ledger note here; nothing to change.
  - Cost if wrong: one more round.
- Task 8 fix round 1 dispatched (the implementer resumed), FIX_BASE `8e6d8e2`.
- Task 8 fix round 1 DONE at `d0d45a1`: I1 (the registry-derived profile id), I2, I3, I4 (spawns the built binary), M1, M2, M3 (a runner-log line), M4, M6 (the 5f76903 subject left as is: no history rewrite). entitlement_gate 17; sync_contract 79; workspace 0 FAILED (the date-test exception retired by the main merge); 0 other; oracle and surface_oracle unchanged. Scoped re-review 1 dispatched (the task reviewer resumed).
- Task 8 re-review 1 (opus; task-8-rereview-1.md): APPROVED. I1–I4, M1–M4 and M6 are ADDRESSED (I1 fails safe and never loosens the gate). New Minors:
  - N1: entitle.rs:98 matches registry paths more loosely than the app's exact-spelling key.
  - N2: the Runs view is built from run records, not runner-log.md, so the main.rs:390-392 comment is false, and a gated step still shows `coursework 0`.
  - N3: the spawned test does not assert sync-status.json is absent for the other three commands, and its `println!` substring check is not specific.
  - N4: a stale state.rs line reference; "lowercase hex" should be case-insensitive.

  Ruling R-C3′-exec-28: all four are parked for Task 12's final review fix wave, with recipes:
  - N1: compare `as_os_str()`, or reuse `ids::derived_id("profile", …)`, the app's own key;
  - N2: make the comment true, or re-rule M3 so the gate also writes a run record;
  - N3: add the two assertions;
  - N4: fix the text.
- Task 8: complete (`d0d45a1`; H3a in 5f76903; H4b at 7721c47).
- Task 9 dispatched (sonnet), BASE `d0d45a1`, carrying:
  - P3 built to yes (away log G4);
  - H11a applied by the implementer, verbatim, as a named commit (R-C3′-exec-23's precedent);
  - N4 from Task 7 re-review 1: the plan's `Err(SyncError::Service(why))` (brief ~:289) follows `SyncError::Service`'s current shape in sync.rs.
- Task 9 implementer DONE: `b76070a` (hand-off H11a) + `75dc3a5` (signing in on a new desktop fills the mirror from the account; the picker's Restore link, per P3 yes). Workspace 1379 passed / 0 failed / 4; 0 other; wizard-check and settings-check ok. Notes: N4 adapted (`SyncError::Service { cause, .. }`); H11a's `restored` is merged into `create_vault_in`'s envelope, not `finish_or_roll_back`'s; `PendingSession` (the test helper) points KNOWLU_API_BASE at loopback, now that cloud.yaml's api_base is live in create_vault_in; one refusal arm is unreachable via `restore_into` and covered at the lower level. Task review dispatched (opus).
- Ruling R-C3′-exec-29 (order): Task 10 needs Quinn's P2 (the go to delete history.rs, which cannot practically be undone). Quinn is away, so it is logged as the away log's Q1, and Task 10 WAITS. Task 11 (the privacy sentences, capability URLs out of the vault; H11b, H12) does not depend on Task 10 (its files and interfaces name none of Task 10's), so it runs next, out of plan order. Its new privacy wording is P1, Quinn's and the lawyer's before MERGE, not before it is written: the plan builds to its drafted text. Cost if wrong: a hidden Task 10→11 dependency surfaces in Task 11's review.
- Task 9 review (opus; task-9-review.md): SPEC ✅. QUALITY changes needed: 6 Important, 5 Minor.
  - Important:
    - I1: b76070a (H11a) did not build alone; it calls `sync::restore_into` from 75dc3a5.
    - I2: a page-2+ failure keeps a half vault, reports `empty: true` and saves no cursor.
    - I3: the cursor a restore saves has no note hashes or push position, so the first push re-sends everything and bumps every rev (and may overwrite another desktop's newer body).
    - I4: an account-deleted note at a seed's path leaves the seed live, and it resurrects on the other desktop.
    - I5: restore lacks apply's protections: it journals `agent:knowlu.sync` records and writes sync-card notes as live cards. `apply` has the same gap for notes (Task 6's F1).
    - I6: the app success-path test cannot tell success from failure, and `join()` can hang; `restore_all` has no direct paging, cursor or fallback test.
  - Minor:
    - M1: the page ignores `restored`, and the static sentence misstates it.
    - M2: a dead Err arm plus a brittle source-text test.
    - M3: the Backups restore hard-codes the name "Knowlu" and turns autostart on.
    - M4: no run lock (safe by construction).
    - M5: one fsync per record.
- Controller (I1): reordered the two local, unpushed commits with `git rebase --onto d0d45a1 b76070a c3-sync` plus `git cherry-pick b76070a`. Now the task is `1b1e0c3` and H11a is `50a750d`; the tree is identical to 75dc3a5. The reviewer's bisect worktree `scratchpad/wt-b76070a` is still registered: its removal is held by away mode (away log H4).
- Ruling R-C3′-exec-30 (fix round 1):
  - (I2) Restore is all-or-nothing. Any page failure rolls back to the pristine scaffold, and the envelope says the restore failed and why (no false `empty: true`); the cursor is saved only after the last page.
  - (I3) A completed restore saves a cursor equal to a completed pull-then-push: every restored note's hash, and the push position at the journal's end, because restored records are foreign and never pushed back. The first push after a restore sends only new local writes.
  - (I4) Restore applies the account's tombstones over the scaffold's seeds with `apply`'s tombstone semantics, so a deleted seed stays deleted.
  - (I5) Restore and `apply` share one filter. It refuses pulled `sync::ACTOR` records and live sync-card notes, each with a named warning. This also closes Task 6's F1 (un-parked).
  - (I6) The success-path test asserts the restored content, and its fake server has a timeout. Direct tests cover `restore_all`: paging, the cursor saved, and the network-failure rollback.
  - (M1) The page reads `restored`: restored / the account could not be reached, so the vault fills at the next sync / no vault yet.
  - (M2) Keep the guard and drop the brittle source-text test.
  - (M3) The Backups restore asks for a name (the picker gains a name field for it) and does not force autostart.
  - (M4) A comment on why no lock is needed.
  - (M5) One fsync per page, if the journal writer allows it cleanly; otherwise record why not.
  - Before starting, the implementer confirms `1b1e0c3` builds alone (`cargo check --workspace --tests` in a temp worktree), then removes that temp worktree's build dir only if away mode allows.
  - Cost if wrong: one more round.
- Task 9 fix round 1 dispatched (the implementer resumed), FIX_BASE `50a750d`.
- Task 9 fix round 1 DONE: `caec555` (I2–I6, M1–M4) + `fa6bd93` (I6 completed: the app test asserts restored content, and the loopback helper has an accept deadline). Workspace 1385 / 0 failed / 4; 0 other; wizard-check and settings-check ok.
  - Step 0: `1b1e0c3` builds alone. The temp worktree `scratchpad/wt-1b1e0c3` was left in place; its removal is held (added to away-log H4).
  - Also found and fixed: `#pick-restore-options` used `.wiz-nav`, whose `display:flex` beats `[hidden]`; a fourth `.wiz-nav[hidden]` rule was added.
  - M5 was NOT done: `JsonlLedger::append` fsyncs unconditionally, is shared engine-wide and sits outside C3′'s files. Ruling R-C3′-exec-31: parked for Task 12's final review / C0's perf list, with a recipe: a batched append on the ledger that fsyncs once per batch, used by restore.
- Task 9 scoped re-review 1 dispatched (the task reviewer resumed, opus) on review-50a750d..fa6bd93.diff.
- Task 9: fix round 1/5 (50a750d..fa6bd93) closed at re-review 1 (opus; task-9-rereview-1.md). ADDRESSED: I1–I6, M1–M4. Re-run probes: the page-2 503 rolls back byte-identical; the re-push is 0/0; the tombstone settles the seed. Open:
  - N1, Important (a round-1 regression): the restore cursor hashes EVERY note on disk and puts the push position at the journal's end, so the scaffold's seeds are marked pushed. A course added in the wizard that the account lacks never reaches it.
  - N2, Important (pre-existing): after a FAILED restore, the first sync pushes untouched seeds over the account's copies at the same paths (a seed's empty grade weights replaced the account's courses/cs-100.md). The page's "fills in at the next sync" is false.
  - N3: the restore sentence shows only while `entitlement_now` runs before the relaunch, and keying on `warnings.length` misreads an all-refused pull.
  - N4: the empty-account test does not assert `warnings` is empty.
  - N5: the rollback deletes files the snapshot could not read and leaves empty folders.
  - N6: the picker's Restore button is not disabled during the call.
- Ruling R-C3′-exec-32 (fix round 2):
  - (N1) After a restore, the cursor marks as pushed ONLY what came from the account: the hashes of notes whose content equals the account's copy, and the restored (foreign) records. Local writes the account lacks (the scaffold's seeds, onboarding's config and create records) stay unpushed and go on the first push. Test: a wizard-only course reaches the first push, and a restored note does not.
  - (N2) An untouched wizard seed loses to the account. In `apply` (and so in every sync, not only restore), an incoming account note at a path whose local note is an UNTOUCHED seed replaces it: the local note's journal history is exactly its scaffold create, never written since. The seed's create is never pushed over the account. A seed the student edited is an ordinary conflict, handled as today. The page's failed-restore sentence says the notes arrive over the next syncs. Test: the reviewer's cs-100 probe.
  - (N3) The page keys on explicit fields (ok / count / error), not `warnings.length`, and the sentence stays until the relaunch, without racing `entitlement_now`.
  - (N4) The assertion.
  - (N5) The rollback removes only paths the restore wrote (tracked) and prunes only folders it created.
  - (N6) Disable the button during the call.
  - Cost if wrong: one more round (2 of 5 is used here).
- Task 9 fix round 2 dispatched (the implementer resumed), FIX_BASE `fa6bd93`.
- 2026-09-23, QUINN BACK:
  - **P2: "Yes, delete it"**. Task 10 deletes engine/src/history.rs as the plan says; the order is now plan order, Task 10 then Task 11, after Task 9 closes.
  - The guesses stand: G1 (sync cadence: slot, Sync now and quit), G3 (P5 (a): `rank` keeps ranking past the grace), G4 (P3 yes: the picker's Restore link).
  - c3-sync pushed ff716e3..fa6bd93, a fast-forward (the Task 9 commit reorder touched only unpushed commits).
  - The reviewers' two scratch worktrees (wt-b76070a, wt-1b1e0c3) and their target folders were removed.
- Task 9 fix round 2 DONE at `77973b4`: N1 (a per-page cursor marking only account content pushed), N2 (an untouched seed loses to the account, in apply), N3 (an explicit ok field; the sentence set before `entitlement_now`), N4, N5 (precise rollback tracking through a generic directory diff, keeping ledger.rs the only module that names .jsonl), N6. Workspace 1389 / 0 failed / 4; 0 other; wizard-check and settings-check ok. Scoped re-review 2 dispatched (the task reviewer resumed).
- Task 9: fix round 2/5 (fa6bd93..77973b4) closed at re-review 2 (opus; task-9-rereview-2.md). ADDRESSED: N1 (the wizard-only course reaches the first push; no restored note is re-sent), N3, N4, N5, N6 (a nit: `open_profile` ok:false leaves the button disabled with no message). OPEN:
  - N2 still fails when the account's copy has field history: apply first copies fields onto the seed and journals an echo, so the seed stops being "untouched" and is pushed over the account.
  - R1, Important, a regression caused by MY ruling R-C3′-exec-32: "untouched" was defined as exactly one create record, and it applies to every note. Hand body edits are never journalled, so a veteran desktop's hand-typed course, or a console-created task, is overwritten by another desktop's note at the same path. Before round 2, apply never overwrote a note the desktop already had.
  - R2 (minor): the rollback misses the `delete` record written when a tombstone settles a seed.
  - R3 (minor): `materialise`'s signature drifted from the brief; `Touched` being pub is not a leak.
  - R4 (minor): the per-page cursor can hash a seed as account content when a write was skipped.
- Ruling R-C3′-exec-33 (fix round 3). It CORRECTS R-C3′-exec-32's definition of a seed, which was the controller's error.
  - (R1 and N2) A seed is identified PRECISELY. Before `restore_into` changes anything, it writes `state/seed-hashes.json` (through `ledger::dumps_value`, atomically): {path: sha256 of the file's bytes} for every note the new vault holds, all of them wizard seeds by construction. The file is written whether the restore succeeds or fails, and state/ is never pushed. `apply`'s replacement rule fires ONLY for a path listed there whose current bytes still hash to the recorded value, so it is byte-identical to what the wizard wrote.
    - No file (every vault not created through the restore flow) means the rule never fires; an edited seed is an ordinary conflict, as before.
    - The replacement runs as a PRE-PASS over the page's notes, before any record is applied, so no field is merged onto a seed and no echo is journalled under a seed's id. The replaced path is then removed from seed-hashes.json.
    - Tests: the reviewer's R1 probes (a hand-edited course; a console task at the same path) are NOT overwritten; N2 with field history, where the account copy survives the first sync; and a vault with no seed-hashes.json, where apply is unchanged from before round 2.
  - (R2) Tombstones are collected and applied only after the last page succeeds, so a failed restore never has settled a seed.
  - (R4) The cursor marks a note pushed only if its on-disk hash equals the account row's hash.
  - (N6 nit) On `open_profile` ok:false, show the message and re-enable the button.
  - (R3) Accepted: the report records the signature change.
  - Cost if wrong: rounds 4–5 (escalation) and then the breaker.
- Task 9 fix round 3 dispatched (the implementer resumed; the last round before escalation), FIX_BASE `77973b4`.
- Task 9 fix round 3 DONE at `1f61e60`: R1/N2 (`state/seed-hashes.json`, written unconditionally before anything else, is the only basis for the pre-pass; the old heuristic is deleted), R2 (tombstones settled once, after full success), R4 (a precise cursor), the N6 nit, R3 (materialise/Touched/RestoreState made private). Workspace 1392 / 0 failed / 4; 0 other; wizard-check and settings-check ok. (The implementer backgrounded its workspace run against instructions, then waited on it; it finished on its own.) Scoped re-review 3 dispatched (the task reviewer resumed).
- Task 9: fix round 3/5 (77973b4..1f61e60) closed at re-review 3 (opus; task-9-rereview-3.md). R1, N2 (single page), R2, R4, N6 and R3 are ADDRESSED; seed-hashes.json can list only wizard seeds today, and a stale entry cannot replace an edited note. OPEN:
  - E1, Important, pre-existing: after a failed restore, when the first slot's page lacks the account's copy of a seed path, the push still sends the untouched seed over it.
  - E2, a minor regression from deferral: a page-1 tombstone plus a page-2 live re-create at the same path archives the live copy.
  - E3, a minor regression, frequent: the usual case (the welcome task done elsewhere) archives the seed as `archive/get-to-know-knowlu-2.md` and pushes it to every desktop.
  - E4 (docs): (a) the global constraints name `state/sync-cursor.json` as the only new vault file; (b) `restore_into` must state or check that the vault is fresh; (c) optionally prune stale entries.
  - E5 (low, pre-existing from Task 6): a foreign move is applied to whatever sits at the old path, without an id check.
- Ruling R-C3′-exec-34 (fix round 4, ESCALATION: a FRESH implementer on opus):
  - (E1) `build_push` holds back every path listed in seed-hashes.json whose bytes still match, until a pull has reached `more: false` once (a flag in the cursor). After that, an unmatched seed is local-only truth and is pushed normally.
  - (E2) A pending tombstone is dropped when a live note is written at its path later in the same restore.
  - (E3) The seed is REMOVED outright when the account's tombstone or archived copy settles its path during a restore. It is never archived as a second copy, because it is a wizard placeholder with no history worth keeping. Its seed-hashes entry goes with it.
  - (E4a) A dated amendment line in the plan's Global Constraints naming `state/seed-hashes.json`.
  - (E4b) `restore_into` checks the vault is fresh (only scaffold content, no journal records beyond the scaffold's) and refuses otherwise, with a named error.
  - (E4c and E5) Parked for Task 12's final review, with recipes: prune entries whose paths no longer exist or no longer match; a foreign move checks the id at the old path.
  - Cost if wrong: round 5, then the breaker.
- STOPPING POINT (2026-09-23, Quinn: "come to a stop"). Task 9 fix round 4 (the escalation, a fresh opus implementer, R-C3′-exec-34) was STOPPED before it changed anything: the tree is clean at `1f61e60`, and c3-sync is pushed through 1f61e60. On resume: re-dispatch round 4 FRESH (opus) with the same prompt: E1, E2, E3, E4a and E4b per R-C3′-exec-34, FIX_BASE `1f61e60`. Then re-review 4 (resume the Task 9 reviewer a48997d7e518bd5a7), round 5 if needed, then the breaker. After Task 9: Task 10 (P2 = YES; brief and hand-offs H3b/H8b/H9b/H10 extracted as task-10-brief.md and task-10-handoffs-*.md), then Task 11 (brief and hand-offs extracted; its privacy wording needs P1, Quinn plus the lawyer, before MERGE), then Task 12.
- 2026-09-24, QUINN: "do the privacy wording yourself". P1 is decided by delegation to the controller. Ruling R-C3′-exec-35: the final wording is task-11-privacy-wording.md, which Task 11 applies VERBATIM, superseding the brief's Step 3 drafts.
  - Every claim was checked against the build: plaintext to the service and encrypted at rest; 200 MiB; journal kept 400 days except human-written records; note rows never pruned; purged by DELETE /account; never trained on (the training export is judgments rows only, no body).
  - Beyond the brief's sentence and bullet, the page's other now-false claims are fixed: the "What stays on your machine" intro ("We hold no copy of your notes"); a new "Your tasks and notes" collect entry; the Supabase bullet; a retention bullet; the Export sentence ("they were never here").
  - Storing note text is a new category collected, so the page version moves to 2026-09-24. `app/src/account.rs`'s PRIVACY_VERSION follows as a new hand-off, **H16**, beside Task 11.
  - The test string "Delete my account" is corrected to the real label, "Delete my data".
  - New promise, Quinn's to keep: "No human at Knowlu reads it without your explicit consent" (it mirrors the Gmail promise).
  - RELEASE GATE, recorded for Task 12 and HANDOFF: the page promises to ASK before a materially different use begins. Before C3′ reaches production, count the prod accounts whose accepted privacy_version is older than 2026-09-24. If any exist, a re-ask screen must ship first. If none (the expected case before the pilot), the version bump is enough.
  - The merge is no longer gated on the lawyer (Quinn's call). A lawyer's read before the paid launch stays advisable and is noted to Quinn once.
- Task 9 fix round 4 (the escalation, fresh opus, re-dispatched 2026-09-24) DONE at `e250f37`: E1, E2, E3, E4a, E4b; each behaviour test RED at 1f61e60. Workspace 1398 / 0 failed / 4; 0 other; wizard-check and settings-check ok.
  Concerns:
  - (1) E3's TWIN on the failed-restore path: `apply` archives a seed when a tombstone arrives, skips the account's archived copy, and the next push sends the seed over it. E1 does not catch this, and a fix needs a ruling, because apply never unlinks.
  - (2) A second cursor field, `withheld_ids`, keeps a removed seed's create record off the wire (a design choice beyond the ruling; mutation-checked).
  - (3) E3's outright removal bypasses `write` and journals nothing; the Global Constraints need a line.
  - (4) Residuals: a replaced seed's create record still goes up (harmless, pre-existing); the one-page `restore()` records no withheld ids.
- Scoped re-review 4 dispatched (the Task 9 reviewer resumed), asked to judge (1) and (2).
- Task 9: fix round 4/5 (1f61e60..e250f37) closed at re-review 4 (opus; task-9-rereview-4.md). E1, E2, E3, E4a and E4b ADDRESSED; R1 and R2 re-run; no regressions. `withheld_ids` is sound: it matches only a removed seed's own create and never moves the push position.
  - Implementer concern 1 (E3's twin in apply, after a failed restore) is REPRODUCED but not load-bearing: it touches only archive/ and follows only a failed restore. Ruling R-C3′-exec-37: parked for Task 12's final review. Recipe: a narrow exception, after the case-only-rename check, where a tombstone landing on a listed byte-identical seed removes it outright, as E3 does.
  - Concern 3: the one must-fix was docs.
- Task 9 fix round 5 (controller, docs-only, R-C3′-exec-36) at `353deea`: the plan's Global Constraints gain the two seed-only recorded exceptions (apply's seed overwrite; restore's unjournalled seed removal), confined to listed byte-identical seeds.
- Task 9: complete (`353deea`: task 1b1e0c3, hand-off H11a 50a750d, fix rounds caec555/fa6bd93, 77973b4, 1f61e60, e250f37, 353deea). The breaker was not needed.
- Task 10 dispatched (sonnet), BASE `353deea`, with P2 = YES (Quinn, 2026-09-23). It carries:
  - Task 7's M3: H8b's verbatim anchor moved when Task 7 reworded that scheduler comment, so apply H8b's INTENT there and record it;
  - the hand-offs H3b, H8b, H9b and H10, applied by the implementer (R-C3′-exec-23's precedent). They are compile-blocking together, the plan's one non-buildable intermediate, so they land together in one named commit.
- Task 10 implementer DONE: `517adc3` (history.rs deleted, app/tests/no_git.rs, plus test and page edits; red by design, the plan's one non-buildable state) + `074ca28` (hand-off H3b/H8b/H9b/H10; green). Workspace 1387 / 0 failed / 4; 0 other; oracle and surface_oracle unchanged; SyncLock grep empty; wizard-check and settings-check ok. Six hand-off deviations, incl. tray.rs and report.rs reading `cs.history.last_error` (moved to cs.sync) and a second refresh_* call site. One staging slip was amended before anything depended on it. Task review dispatched (opus).
- Task 10 review (opus; task-10-review.md): SPEC ❌. The miss was the CONTROLLER's dispatch: R-C3′-exec-24 (I2) said Task 10's page renders `SyncStatus.skipped`, and the dispatch did not carry it. Otherwise the brief, the hand-offs (H8b by intent) and all six deviations are justified; `517adc3` is the only red commit; workspace 1387 / 0 / 4. QUALITY changes needed:
  - I1: console.js:388-390 never reads `skipped`, so every skipped sync (no account, signed out, lock busy, lapsed) says "in step".
  - I2: no_git.rs:29/:47 split on the first `#[cfg(test)]` text, even in a doc comment, which hides state.rs from :141 and most of account.rs. The same bug is in both no_console.rs guards.
  - M1: stale console.js comments.
  - M2: false git comments in Task-10-touched files (no_console.rs:4-7, commands.rs tests :404, scheduler.rs tests :629, app/src/commands.rs:339/:343, app/src/scheduler.rs:122), others for Task 12, and anatomy.md for H14.
  - M3: `commit_opt_in` is dead (for Task 12).
  - M4: tray and report show only `last_error`, not `skipped`.
  - M5: the commit message and report overstate no_git's scan.
- Ruling R-C3′-exec-38 (fix round 1):
  - (I1) The page renders `skipped` BEFORE `at`: amber for "no entitlement" and "signed out", a calm grey for "another sync is running" and "no account". A test covers each.
  - (I2) Every guard splits on `"\n#[cfg(test)]\nmod "`; no_git, and both no_console guards. A test proves a git call placed after a doc-comment mention is caught.
  - (M1, M2 in Task-10 files) Comments made true.
  - (M4) The tray diagnostics and the issue report name `skipped` too.
  - (M5) Correct the report; the commit message is not rewritten.
  - (M3 and the non-Task-10 files of M2, plus anatomy.md) go to Task 12.
- Task 10 fix round 1 dispatched (the implementer resumed), FIX_BASE `074ca28`.
- Task 10 fix round 1 DONE at `1d53fa6` (I1 the page renders skipped; I2 the test-module split fixed in no_git and both no_console guards; M1; M2 in Task-10 files; M4 tray and report name skipped; M5 in the report). Workspace 1393 / 0 failed / 4; 0 other; wizard-check and settings-check ok. The implementer chose the copy "sync skipped — <reason>" (amber: no entitlement and no session; calm: no account and lock busy). Scoped re-review 1 dispatched (the reviewer resumed).
- Task 10 re-review 1 (opus; appended to task-10-review.md): SPEC ✅, QUALITY approved. I1, I2, M1, M2, M4 and M5 ADDRESSED: renderSyncLine was run in node over seven states; the stricter split misses no file; a stale "sync skipped" over a successful run cannot happen. New Minors:
  - N1: the amber copy uses internal words ("no session", "no entitlement").
  - N2: four small comment inaccuracies (console.js:382-383, :389; no_console.rs:26; the report's state.rs line).
  - N3: a stale git comment at app/tests/scheduler.rs:577-582.
  - N4: the page's skip words are not tied to the engine's emitted words.
- Ruling R-C3′-exec-39: N1–N4 go to Task 12's final fix wave. N1's copy is ruled now (the controller, since Quinn delegated user-facing wording today): "signed out — sign in to sync" (amber), "subscription inactive — changes stay on this computer" (amber); the calm ones stay. N4's recipe: a shared constant table or a test that reads the engine's skip labels (`cloudmodel::Unavailable::label` and the sync.rs literals) and asserts the page maps each one.
- Task 10: complete (`1d53fa6`; history.rs deleted at 517adc3; hand-offs H3b/H8b/H9b/H10 at 074ca28).
- Task 11 dispatched (sonnet), BASE `1d53fa6`, with:
  - the privacy wording file (task-11-privacy-wording.md, P1 decided, applied verbatim);
  - hand-offs H11b and H12, applied by the implementer; H5, H7 and H8c verified;
  - H16 (`PRIVACY_VERSION` → "2026-09-24", R-C3′-exec-35).
- Task 11 implementer DONE: `9172d61` (the task), `66bb8f6` (H11b), `6632bd1` (H12), `3894693` (H16). Workspace 0 failed, 0 other; wizard-check and settings-check ok. Deviations: the new privacy test lives in engine/tests/site.rs (where the `site()` helper is); H11b rippled into onboarding and scaffold tests (a literal calendar URL is now `cloud:personal`). Task review dispatched (opus).
- Task 11 review (opus; task-11-review.md): SPEC ✅ (the wording is character for character, the one-liner is byte-identical in four copies, PRIVACY_VERSION matches the page, and nothing breaks). QUALITY changes needed. None of these is the implementer's error.
  - I1 (a defect in H11b as planned): if the account save of a pasted calendar link fails, the link is lost for good. The vault gets '' or `cloud:personal`, nothing retries at Finish, and the wizard still says "saved on this machine".
  - I2 (a gap in the controller's wording): "a correction to a title is not recorded at all" is now false, because the journal syncs every human set.
  - I3: "We hold the text for the length of the call and then let it go" is now false.
  - I4: every journal record carries COMPUTERNAME, the server keeps it, and the page never lists it; sync.rs:7's "a machine name does not leave" is false.
  - Minors:
    - M1: the Google test runs an impossible no-account case.
    - M2: two test names over-claim.
    - M3: two privacy assertions were already true.
    - M4: the "no LMS feed" check is always satisfied.
    - M5: the retention wording is over-broad.
    - M6: the release gate is only in progress.md.
    - M7: three red commits in a row (noted).
- Ruling R-C3′-exec-40 (fix round 1):
  - (I2, I3, I4, M5) The wording file gains §5 (i)–(v): the corrections paragraph, the providers paragraph, the computer-name disclosure, the precise retention wording and the sync.rs doc. Applied verbatim.
  - (I1) Amend H11b in a named commit, "hand-off H11b amendment". Finish retries the account save of any pasted calendar link. If it still fails, that one URL is written into the vault (today's fallback), so it is never lost. The two messages say where the link was actually saved, the stale doc paragraph is corrected, and a test covers the failed-save path.
  - (M1) Add the real account-case test.
  - (M2) Strengthen or rename the tests.
  - (M3) Assert the NEW entries: "Your tasks and notes", "200 MiB", "400 days", and the computer-name sentence.
  - (M4) Make the check specific.
  - (M6) Carried into Task 12's dispatch (the production-needs list and HANDOFF).
  - (M7) Noted.
- Task 11 fix round 1 dispatched (the implementer resumed), FIX_BASE `3894693`.
- Task 11 fix round 1 DONE: `c815f12` (wording §5 (i)–(v), M3, M4), `1e169fc` (hand-off H11b amendment: I1's retry at Finish, then a vault fallback, and M2), `b0898ce` (M1). Workspace 0 failed; the one-liner is byte-identical in four copies. The I1 design retries every pasted feed at Finish (no per-feed flag). Scoped re-review 1 dispatched (the reviewer resumed).
- Task 11 re-review 1 (opus; appended to task-11-review.md): SPEC ✅, QUALITY approved. I1–I4 and M1–M5 ADDRESSED; the page is exactly 3894693 plus §5, byte for byte. New Minors:
  - N1: the unconditional retry can land a paste-time-saved link in the vault after a transient Finish failure, send a rejected link to the account, and hold Finish up to 60 s.
  - N2: the 402 message says "until it reaches your account", which never happens.
  - N3: two onboarding tests run one path, and there is no personal-calendar retry-success test.
  - N4: issue notes also carry the computer name (`opened_on`).
  - N5: the retention wording is narrower than what is kept.
  - A nit at sync_contract.rs:2686.
- Ruling R-C3′-exec-41, all for C3′'s FINAL FIX WAVE:
  - (N1) A per-feed "stored" flag from the paste into the plan. Finish retries only feeds that were not stored. Only a link that passed validation is ever sent to the account; a rejected link is not.
  - (N2) The message reuses "if it still can't".
  - (N3) The test.
  - (N4, N5) Wording §6 (vi), (vii).
  - (the nit) `find(...).expect(...)`.
- Task 11: complete (`b0898ce`; hand-offs H11b 66bb8f6 + amendment 1e169fc, H12 6632bd1, H16 3894693).
- 2026-09-24, the RELEASE GATE CHECK (controller; read-only, counts only, from a scratch folder linked to prod with no password): prod `public.accounts` holds 2 accounts, both with privacy_version 2026-09-16, older than the new 2026-09-24. One is Quinn's own Gmail address; the other is NOT. The re-ask gate is therefore live unless Quinn says the second account is theirs or a disposable test. Asked of Quinn. If it is someone else's, a re-ask screen (a consent route that updates an existing account's privacy_version on an explicit yes, plus the page) ships before C3′'s sync reaches production.
- Task 12 dispatched (sonnet), BASE `b0898ce`, with the 21-entry hand-off list (H16, the H11b amendment), the H14 text including anatomy.md, and production needs (a)–(e): the re-ask gate, C3′ migrations and functions to prod, C1c's ingest-coursework and first-day cap to prod, and the merge order.
- Task 12 dispatched (opus, no subagents), BASE `b0898ce`. Gate green twice (RUSTFLAGS=-D warnings,
  --no-fail-fast): 1403/0/4 both runs, `warnings: 1 accepted (.rsrc), 2 tallies, 0 other`; oracle 3/3,
  surface_oracle 4/4 unchanged. Cloud with C2's full flag set (scoped to `cloud/supabase/`, no
  `cloud/eval/`): check/lint clean, `423 passed | 0 failed`. eol-check.ps1 holds over 440 files;
  wizard-check.py/settings-check.py both `ok` (H13 verified). Fixtures untouched;
  `engine/src/backup.rs` byte-identical to `main`. Step 3's account-scoping scan added verbatim to
  `sync_rows_test.ts` (13/13 alone). Step 4's ownership check: 74 paths in `main...c3-sync`, all seven
  buckets accounted for (38 C3′-owned, 14 hand-off-covered, 2 ruled exceptions —
  `migrations/migrations_test.ts` under R-C3-exec-4, `config.toml` under R-C3′-exec-6 — 2 unrestricted
  engine files, 1 flagged minor `scripts/wizard-check.py`, 2 docs cross-cutting, 15 inherited from the
  still-unmerged `c1b-sign-in` stack); no unlisted overlap. Step 5: 21-entry hand-off list compiled
  with exact code, commit, task and applied-by. Step 6: H14's text written for `CLAUDE.md`,
  `HANDOFF.md`, `README.md`, `app/README.md`, `VISION.md` (no change) and
  `docs/surface/anatomy.md`'s git sync line, none applied by this session (the controller's own
  commit). Step 7: production-needs list (brief's items, corrected to four migrations; the
  orchestrator's five additional items (a)–(e); an appendix of ~2 dozen review findings parked for
  this close). **Found, reported, not fixed:** ruling R-C3′-exec-41 (Task 11 re-review 1's fix wave,
  N1–N5 + a nit) is not applied anywhere on the branch — HEAD before this task equalled its own BASE
  (`b0898ce`); flagged prominently in the report and in the plan's own Status line rather than
  silently left implicit. Also found: `engine/src/judgelog.rs:5`'s doc comment still names
  `history::sync` (a stale reference outside `app/tests/no_git.rs`'s scan, which only covers
  `app/src/`); `engine::uievents::commit_opt_in` is dead code (Task 10 review M3) — both named in the
  report's appendix, neither fixed (outside this task's docs scope). Step 8: committed `b519da9`
  ("cloud+docs: the account-scoping scan, and this plan marked DONE (C3' Task 12)") — the two files
  only; the plan's own Status line marked DONE 2026-09-24. Full report:
  `.superpowers/sdd/2026-09-17-c3-account-vault-plan/task-12-report.md`. Task 12: complete, pending
  the controller's H14 commit and the merge-order decisions (PR #9 → C1c → C3′) this report names.
- 2026-09-24, QUINN: both prod accounts that accepted privacy 2026-09-16 are Quinn's own or test accounts. The re-ask gate is CLEARED: no re-ask screen is needed, and the PRIVACY_VERSION bump is enough. (Production-needs item (a) is closed; recorded for the H14/HANDOFF text.)
- Task 12 DONE at `b519da9` (the account-scoping scan; the plan marked DONE; the gate green three times, 1403 / 0 / 4, 0 other; ownership fully classified; the 21-entry hand-off list; H14 text; production needs; the parked-findings appendix). Its concern 1 (R-C3′-exec-41 unapplied) is by design: those items go to the final fix wave. Task 12: complete.
- Ruling R-C3′-exec-42: H14 is applied on the branch for CLAUDE.md, README.md, app/README.md and anatomy.md (a sonnet agent, verbatim, one named commit). H14's HANDOFF.md block is applied on MAIN at merge time instead, because main's HANDOFF is edited by other sessions and a branch copy would conflict. The final whole-branch review runs in parallel (opus, read-only).
- H14 applied at `7574168` (CLAUDE.md, README.md, app/README.md, anatomy.md; verbatim except two formatting-only adjustments; command counts recounted by script: 42/29/61, unchanged; the site, workflows and static_assets tests pass; 0 CRs). HANDOFF.md's block waits for merge time (R-C3′-exec-42).
- FINAL REVIEW (opus; final-review.md): READY WITH FIXES. Gate 1403 / 0 / 4, 0 other; cloud 458/0; wizard-check and settings-check ok.
  - C1, Critical, confirmed by the controller from the code: the engine only READS the session token (cloudmodel.rs:106-124, "Refresh is C1's job"), and the app refreshes it only through `valid_access_token_at`. In a slot that happens at the END (telemetry) or at the start only when entitlement has lapsed, so a noon or 6 pm slot starts with an expired one-hour token and every cloud step gets a 401. This has been latent since C2 and would hit every student on day two. The live proofs ran just after sign-in, so they never showed it.
  - I1: a future-dated foreign record advances `pushed_through`, so this desktop's later edits are never pushed.
  - I2: config/ does not sync, so two desktops both run coursework, ingest and judge and create the same task under two ids (silent divergence, double judging). It needs Quinn's ruling before any student uses two desktops.
  - Triage: 42 rows, 18 FIX-IN-WAVE, 19 DEFER, 5 DROP.
  - The merge section: C1c then C3′, six conflict files with recipes. The migration pin becomes 28.
  - Production gaps: one ordered migration push with C2's ten; `account` deployed after the migrations; the privacy page live before any release uploads note text; a two-desktop live proof is a release gate.
- Ruling R-C3′-exec-43 (the ONE final fix wave):
  - All 18 FIX-IN-WAVE rows per final-review.md, EXCEPT: C1's slot half goes to C1c (R-C1c-13, C1c Task 11, before PR #13 merges); C3′ fixes `run_sync` / `quit_flush` / Sync now.
  - Row 7's copy is corrected: "no entitlement" reads "can't confirm your subscription — changes stay on this computer" (amber). A paying student offline for more than 72 h also lands there, so "subscription inactive" would be false. "signed out — sign in to sync" stands.
  - I2 is DEFERRED pending Quinn's ruling (asked).
  - The wave does not touch scheduler.rs (merge hunks).
- Final fix wave dispatched (opus, fresh), FIX_BASE `7574168`.
- 2026-09-24, QUINN on I2 (two desktops): "Is there a way we can do 1 AND make it so that we can track devices and try each one, one at a time?" DECISION: BOTH.
  - (1) Deterministic ids for imported notes, derived from (vendor, source uid), so every computer creates the same note and sync merges it. This is the backstop.
  - (2) Device tracking with a fetch TURN: the account lists the student's computers (by the existing device token; named by the Windows name, which the privacy page now discloses). At slot time a computer claims a short lease (~20 min) from the account. The holder runs coursework, ingest and judge; others skip those steps as named lines and get the results through sync. An expired lease passes the turn to the next slot on any computer.
  - Scope: a follow-up stream AFTER C3′ merges (spec, plan, tasks); not needed for a single-computer pilot. The controller writes the spec once C3′ is merged. I2 stays DEFERRED in this wave.
- Both fix agents stalled on the stream watchdog and were resumed by SendMessage with their on-disk state: the C1c Task 11 changes were uncommitted and intact; the C3′ wave had c696325 in (I1 clamp, M1).
- FINAL FIX WAVE DONE (opus; resumed once after a watchdog stall): c696325, efba911 (hand-off H17: Sync now and the quit push refresh the session), 77c8abb (hand-off H18: per-feed stored/validated flags, the N2 sentence, N3 tests), 1726737 (the SYNC_SAYS table pinned to the engine's words, with R-43's copy), 31b779a (privacy §6), da312ed (the sweep), ec2ac89 (hand-off H19: the lib.rs and main.rs comments), 7ccd784 (the anatomy copy). Workspace 1414 / 0 / 4, 0 other; cloud 458/0; wizard-check and settings-check ok; the one-liner byte-identical; scheduler.rs untouched (the trial merge shows the same six files and hunks).
  - Not done by design: row 37's slot_argv doc (scheduler.rs, left for the merge); row 8's fourth pointer (in a .superpowers report).
- Ruling R-C3′-exec-44 on the wave's concerns:
  - (1) The copy "offline — changes stay on this computer" (amber) is ACCEPTED.
  - (2) A failed session refresh named only in `lines` (a view shows `last_error`): DEFERRED; a rare auth 5xx.
  - (3) A rejected link kept in the vault: ACCEPTED (never lost; it never reaches the account).
  - (4) C1's live check: a staging proof before release, recorded for the release checklist.
  - H17, H18 and H19 extend the hand-off list to 24.
- Scoped re-review of the wave dispatched (the final reviewer resumed).
- Re-review of the final fix wave (opus; appended to final-review.md): READY WITH RESIDUALS, no open rows. All 18 rows are ADDRESSED; the I1 and C1 app-half probes pass; the SYNC_SAYS pin holds; the merge picture is unchanged (six files); gate 1414 / 0 / 4, 0 other; cloud 458/0; wizard-check and settings-check ok.
  - Residuals for merge time:
    - C1c Task 11 (the session pre-flight) must land on PR #13 before C3′ merges; do not add a second refresh at the merge, and extend C1c's test to assert the session step precedes `sync`.
    - Row 37: `slot_argv`'s doc gets `sync → …` at the merge.
    - The C1 staging proof (Sync now more than an hour after a refresh) is on the release checklist.
  - Deferred minors: an empty LMS feed stays vault-only; a stale stored flag across an account switch; out-of-order paste replies; a 150 s lock on a black-holed network; the restore clamp gap.
- C3′ STREAM STATUS (2026-09-24): CODE-COMPLETE at `7ccd784`, pushed. Remaining:
  - (1) merge after PR #13 (C1c), taking the six conflict files per final-review.md's recipes (the migrations_test pin 28; scheduler.rs keeps SlotSteps; row 37 doc);
  - (2) CI on the runner, then Quinn's word to merge;
  - (3) H14's HANDOFF.md block on main at merge;
  - (4) production: one ordered migration push (C2's ten plus C3′'s four plus C1c's first-day cap), `account` deployed after the migrations, sync-push/sync-pull deploys, the privacy page live before any release that uploads note text, a two-desktop proof as a release gate, and the C1 Sync-now staging proof;
  - (5) the follow-up stream for Quinn's two-desktop decision (deterministic ids plus a fetch-turn lease).
- 2026-09-24: PR #13 (C1c) MERGED to main as `f30da69`. It includes C1c Task 11 (the slot's session pre-flight, 45-minute floor) and R-C1c-exec-14 (`SESSION_REFRESH_LOCK` around every refresh-and-save).
  - A trial merge of origin/main into c3-sync gives the same six conflicting files.
  - Ruling R-C3′-exec-45: the recipe's step 3, "C1's slot half", is SUPERSEDED by C1c's session block, which runs before the entitlement block and every `slot_argv` child, so before `sync`. No second refresh. At the merge, a test asserts that session precedes sync.
    - H17's `sync_with_a_fresh_session` calls `valid_access_token_at`, so it inherits the lock.
    - Cost if wrong: none at run time; a slot with a fresh token does no refresh either way.
  - Merge dispatched (opus, fresh). Brief: merge-brief.md; report: merge-report.md. The agent pushes c3-sync when the gate is green; the controller opens the PR.
- MERGE DONE (opus): `825f5f4` = origin/main (f30da69) merged into c3-sync; pushed. merge-report.md.
  - The controller read `git show --remerge-diff 825f5f4`: all six hunks follow the recipe.
    - scheduler.rs: (a) take C3′; (b) C3′'s F11 comment plus C1c's `steps.start("backup")`; (c) `refresh_sync` then `let steps`; row 37's doc; the D8 comment rewritten.
    - commands.rs, console.js, the scheduler `use` union, migrations pin 28, wizard-check both sides.
    - The session test now asserts session < sync < coursework and exactly one `session` step.
  - Gate: 1451 / 0 / 4, 1 accepted, 0 other; eol ok; deno check and lint clean; deno test 461/0; wizard-check and settings-check ok. The lock grep is clean (H17 goes through `valid_access_token_at`).
  - Concerns ruled:
    - (1) Comment edits inside C1c's session block: ACCEPTED (they say sync now exists; the code is identical to main's).
    - (2) A PS 5.1-wrapped `.rsrc` line: noted.
    - (3) Main's plan doc has a trailing blank line: another stream's, left alone.
    - (4) Sync now's 120 s margin against the slot's 45 min: ACCEPTED as-is (Sync now makes one short engine call).
- PR #15 opened (c3-sync → main). CI is running on the runner. On green, ask Quinn's word to merge; then apply H14's HANDOFF block on main.
- 2026-09-25: PR #15's CI is GREEN (run 36065920637: test, cloud, eval-gate; the watcher was reaped for low memory and the result was checked by hand). QUINN said merge when green. PR #15 MERGED to main as `e2ce40c` (merge commit, --match-head-commit 825f5f4).
  - H14's HANDOFF block went onto main in `db03078`, a docs-only commit. It is rewritten to the facts of 2026-09-25: merged, the re-ask gate cleared, the pointer to this ledger. It adds a §1 ▶ block, the §3 resume point, and §4's production checklist.
  - The C3′ stream's code is DONE.
  - Remaining, all outside this branch:
    - the production checklist (HANDOFF §4);
    - the two-desktop follow-up spec;
    - closing the stream: preserve this ledger into docs/reports, then delete the worktree and branch, on Quinn's word.
- 2026-09-25: STREAM CLOSED on Quinn's word. This ledger (with the paused C3 plan's ledger as an appendix), the whole-branch review, fix wave and merge report, and Task 12's stream report are preserved in docs/reports/2026-09-25-c3-account-vault-{sdd-ledger,whole-branch-review,stream-report}.md; the worktree and branch c3-sync are deleted (PR #15 keeps the record). The DEFER rows and the production checklist travel with them; the checklist itself is HANDOFF §4.

---

<!-- appendix: the paused C3 plan's ledger: progress.md -->

# SDD ledger — plan: docs/plans/2026-09-14-c3-sync-plan.md

Worktree: `C:\Users\danie\GitHub\knowlu\.claude\worktrees\c3-sync` (branch `c3-sync`, forked from `origin/main` at 7585ec6, 2026-09-17).
Spec: `docs/specs/2026-09-09-knowlu-cloud-design.md` (§5.5 is the section this plan implements). Reachable; rulings are made against it.
Controller: Claude Fable 5.1, session https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z. Trailers on every commit:
`Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` / `Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z`.

Session rules that bind every dispatch (from HANDOFF §5 and the session): implementers never push, never dispatch subagents, never run a reviewer, never `cd`, never hold a token, never touch staging or production; the controller runs every live step (db push, db query, deploys, smokes) and applies every hand-off as its own commit beside the task that needs it. Quinn-owned preconditions P1–P7 are asked one at a time when a task reaches them; without a ruling each task is built to the plan's recommendation ("Without a ruling" clause), so no task waits.

## Baseline (2026-09-17)

- Cloud: `deno test` over `cloud/supabase/` (C2's merged command) — 369 passed, 0 failed, 12 s.
- Rust: `cargo build --workspace --tests` in the worktree — started in the background (cold target dir); main at 7585ec6 is green on CI (run 35179340785 on the last code commit dcd26aa).

## Pre-flight scan (2026-09-17)

Per-task self-consistency: every task's Files block names the files its steps touch and its Interfaces block names what later tasks consume; checked from the extracted header blocks (`scratchpad/c3-task-heads.md`) against the checkout with a symbol grep — **every one of the 73 existing symbols the tasks consume exists** (engine, app, cloud `_shared/`, test files).

| Tasks | Shared file / interface | Produces vs consumes | Found |
|---|---|---|---|
| 2 → 3 | `engine/src/sync.rs`; `sync::seal` / `sync::open` | T2 produces both; T3 adds one test to `mod tests` | consistent |
| 2 → 6, 7 | `SyncKey`, `IndexKey`, `seal`, `record_hash`, `note_ref`, `device_token` | all in T2's Produces | consistent |
| 3 → 4, 5 | `sync_rows.ts`: `checkRecord`, `checkNote`, `isFingerprint` | all in T3's Produces | consistent |
| 4 → 5 | `sync_db.ts`: `readRecords`, `readNotes`, `generation`, `READ_LAG_SECONDS` | all in T4's Produces | consistent |
| 6 → 7 → 8 → 9 | `engine/src/sync.rs` sections; `engine/tests/sync_contract.rs` | sequential, section per task | consistent |
| 7 → 9 | `pulled_from_reply`, `is_note_path` | consumed by T9, **absent from T7's Produces list** but defined `pub fn` in T7's own code (plan lines 4298, 4398) | listing gap, not a conflict — no ruling needed; T7's brief carries the code |
| 8 → 10 | `Direction`, `run_lines_with`, `Totals` | in T8's Produces | consistent |
| 10 → 11 | `stage_restore_key`, `move_pending_key`, `restore_into` | in T10's Produces | consistent |
| 10, 11, 12, 13 | `app/static/console.js`, `index.html`, `app/tests/static_assets.rs` | sequential edits, disjoint regions (switch screen / picker / sync line / PRIVACY var) | consistent |
| 2, 12 | `engine/src/lib.rs` (H1a add, H1b remove) | controller hand-offs at different tasks | consistent |
| 8, 12, 13 | `app/src/scheduler.rs` (H7a, H7b, H7c) | controller hand-offs at different tasks | consistent |
| 10, 11, 12 | `app/src/main.rs` handler lists / startup thread (H8a, H8b, H8c) | controller hand-offs | consistent |
| 12 | `engine/tests/no_console.rs` floor `>= 3` → `>= 2` | current text has `>= 3` | consistent |
| 13 | `site/privacy.html`, `engine/tests/site.rs` `PRIVACY`, `console.js` `PRIVACY`, `site/index.html` (H12) | four copies move together (P2) | consistent |
| 1 | H5's "reads" block vs `account/index.ts:52-61` | identical today; **no test pins the purge list** | consistent |
| Global | Task 9 writes notes with `pystr::write_text` — the one exception to "every write through `write`" | argued in the fidelity ledger and the constraints | recorded, allowed |
| Global | Task 2 adds `ring` and `base64` to `engine/Cargo.toml` | both already in `Cargo.lock` via `ureq`/`rustls`; `dependency_boundary.rs` gains the test naming them | consistent |

Rubric-mandated defects: none found in the header blocks; the per-task review is the net for the bodies.

## Rulings

- **R-C3-exec-1 (2026-09-17):** Task 1's Steps 6 and 7 (`supabase db push`, live reads on staging) are the controller's, not the implementer's — subagents never hold a token or touch staging (session rule). The implementer does Steps 1–5 and 8; the controller runs 6–7 after applying H5 and appends the live output to the task report before review. Cost if wrong: none; the same commands run, by a different hand.
- **P3 (Task 1):** asked of Quinn 2026-09-17 with the plan's context; built to the recommendation meanwhile (200 MiB ceiling, 400 days, pruning only agent-authored and `supersede` records). A different ruling is one migration row.

## Task log

- Task 1: dispatched 2026-09-17 (sonnet), BASE 7585ec6.
- Task 1: implementer DONE_WITH_CONCERNS 2026-09-17 — commit 5422c8a (the migration + migrations_sync_test.ts); C3 suite 5/6 (the purge case red as planned, but by a path bug, see R-C3-exec-2); whole corpus 370 passed / 5 failed (four C1 corpus-wide guards trip on the new migration, see R-C3-exec-3/4). Steps 6–7 (staging) reserved for the controller per R-C3-exec-1. H5 named.
- H5 applied by the controller as its own commit beside Task 1 (account/index.ts purge list: the four sync tables).
- Ruling R-C3-exec-2 (plan defect, Task 1 Step 1): the purge test resolves `./functions/account/index.ts` against `DIR` (= `cloud/supabase/migrations/`), a path that cannot exist. Fix: resolve it against `import.meta.url`. Cost if wrong: none; the intended file is unambiguous.
- Ruling R-C3-exec-3 (Task 1 Step 3): `sync_prune` is a writing, non-definer function with no `revoke execute … from public, anon, authenticated`, so C1's corpus-wide guard (R-C2-E29 / R-C2-E47) fails and PostgREST would expose it as an RPC. The plan's SQL was incomplete; the migration gains the revoke (cron calls it as postgres; the service role keeps execute). This is a real gap the guard caught, not test appeasement. Cost if wrong: none.
- Ruling R-C3-exec-4: C1's `migrations/migrations_test.ts` pins the corpus-wide function and view counts and the revoked-view names "so a diff shows the change" — those pins exist to be bumped by whichever stream adds a function or view, with the reason in the comment. C3 edits exactly those pins (function count to what the corpus now parses, naming C3's functions; view count 4 → 5; `sync_limits` appended to `revoked`). The Global Constraint "C3 carries its own migrations_sync_test.ts rather than editing C1's" is about C3's own guards and did not foresee the corpus-wide pins; the pre-flight scan's claim that C1 filters `20260910…` was wrong for these tests. Cost if wrong: a two-line revert.
- Task 1: fix round 1/5 dispatched (resumed implementer, sonnet) — R-C3-exec-2/3/4; expected: C3 suite 6/6 (H5 is in), corpus 375 passed / 0 failed.
- Task 1: fix round 1/5 — implementer DONE, commit 4907170 (three files: the revoke on `sync_prune`, the purge test's path, C1's pins 18→22 functions and 4→5 views with `sync_limits` in `revoked`); C3 suite 6/6, corpus 375/0.
- Ruling R-C3-exec-5: `supabase db push` needs `--include-all` for every C3 migration — the plan-assigned 2026-09-12 stamp sorts before migrations already on the remote (20260916000100). Applied to staging that way 2026-09-17; production needs the same flag (Task 14 records it). Cost if wrong: none; the migration is self-contained.
- Task 1: Steps 6–7 run by the controller on staging — all six live proofs match (four tables, cron `41 4 * * *`, ceiling 209715200, prune(0)=0, FK violation 23503 on the account-less tombstone, `sync_prune` ACL = postgres + service_role only). Appended to the task report.
- Task 1: task review dispatched (opus) on review-7585ec6..4907170.diff (3 commits incl. H5).
- Task 1: review (opus) — spec ❌ (one plan contradiction, one robustness gap), quality Needs fixes. Important 1: the note cap (196608 chars) vs the ledger's "64 KiB cap" — plan-mandated. Important 2: `sync_usage_bump` inserts a usage row on DELETE, so an `accounts` cascade can abort on trigger ordering. Minors 3–10: comment "above"→"below"; no index on the prune predicate; write-time seq/rev stamps (Task 5's lag mitigates); the policy regex misses a policy without `for`; the `keep` whitespace pin; `sync_ceiling_bytes()` callable by anon; the dangling comment in C1's test; dead defaults. ⚠️ carried forward: (a) Task 6's `build_push` must set `keep` on every human `op: set`/`create` record and test it; (b) the ceiling is enforced in Task 4; (c) the usage trigger's DELETE path is unexercised live.
- Ruling R-C3-exec-6 (Important 1): the schema stands (24 KiB per record, 192 KiB per note); the plan's ledger sentence was wrong and is amended (commit 9b693e4). Cost if wrong: a number in a doc.
- Ruling R-C3-exec-7 (Important 2 + Minors 4, 8): fixed in a SECOND migration `20260912000200_sync_usage_prune.sql` (the first is already on staging; C2's precedent) — the bump never inserts on DELETE, a partial index on `(received_at) where not keep`, `sync_ceiling_bytes()` revoked from clients. Minors 3, 6, 7, 9 fixed in place; Minors 5 and 10 ruled no change (5 is Task 5's read lag; 10 is harmless). C1's function pin 22 → 23.
- Task 1: fix round 2/5 dispatched (resumed implementer, sonnet).
- Task 1: fix round 2/5 — implementer DONE, commit c4a201b (the second migration `20260912000200_sync_usage_prune.sql`; the comment word; the two test hardenings; C1's function pin 22→23 and the sentence); C3 suite 6/6 over two files, corpus 375/0.
- The second migration applied to staging (`--include-all`); ACLs and the index read back; the delete path and the whole-account cascade proven live inside a rolled-back transaction (no 23503; usage 100 → 0; nothing persisted). Appended to the task report.
- Task 1: scoped re-review dispatched (sonnet) on review-4907170..c4a201b.diff.
- Task 1: re-review (sonnet) — all eight findings ADDRESSED, the two ruled-no-change confirmed followed, no new breakage. **Task 1: complete** at c4a201b (commits 5422c8a, 87eb58d H5, 4907170, 9b693e4 docs, c4a201b). Carried forward: Task 6's `build_push` must set `keep` on every human `op: set`/`create` record with a test; Task 4 enforces the ceiling; Task 5's read lag is the mitigation for write-time stamps; production needs `db push --include-all` (Task 14).
- H1a applied by the controller as its own commit beside Task 2 (engine/src/lib.rs `pub mod sync;`) — the branch is red for cargo until Task 2's sync.rs lands, as the plan allows for one commit.
- Task 2: dispatched 2026-09-17 (sonnet), BASE = the H1a commit. P1 asked of Quinn; built to (a) meanwhile.
- Task 2: implementer DONE 2026-09-17 — commit a7f20c4 (`engine/src/sync.rs` first six sections, `engine/Cargo.toml` + `Cargo.lock` two direct edges only, `dependency_boundary.rs` one test); 15 `sync::tests` green, workspace 1256 passed / 0 failed / 4 ignored, the accepted `.rsrc` line only; fixtures clean. (The implementer yielded twice waiting for a notification from its own background test run; resumed with the totals read from its log.)
- Ruling R-C3-exec-8 (plan defect, Task 2 Step 4): the brief's module text carried imports (`BTreeMap`, `Path`, `PathBuf`) and a `sha256_hex` helper that only later sections use; they warn as unused under `-D warnings`, so they were dropped. Tasks 6–9 re-add what they use. Cost if wrong: an import line.
- Ruling R-C3-exec-9 (plan defect, Task 2 Step 3 vs Step 6): the brief's `Cargo.toml` comment named `aes-gcm` literally, and the brief's own Step 6 test forbids that substring in any manifest; the comment says "a second AES-GCM crate" instead. Cost if wrong: none.
- Task 2: task review dispatched (opus) on review-baa5f0c..a7f20c4.diff.
- Task 2: review (opus) — spec ✅ (verbatim to the brief plus R-C3-exec-8/9; caps consistent with Task 1's schema; canonical bytes, POSIX paths, the env mutex, no network, no real Credential Manager in tests all checked), quality "Approved with fixes". Important 1 (plan-mandated): `load_key` maps every `CredError` to `NoKey`, contradicting `KeyUnreadable`'s documented purpose. Important 2: `generate()` untested. Minor 3: the cross-kind test does not guard the domain prefixes. Minors 4–10: no AAD row binding; recovery code has no checksum; `PartialEq` on `SyncKey`; allocation churn in `record_hash`; `SystemRandom` per call; `BadKey`'s label wording; the env-var test's cleanup.
- Ruling R-C3-exec-10 (Important 1): `NotFound { code: 1168 }` → `NoKey`; every other `CredError` → `KeyUnreadable`; factored pure and tested if the variants are constructible. R-C3-exec-11 (Important 2 + Minor 3): a `generate()` test and a prefix-pinning test. Minors 6, 7, 8, 10 ruled no change.
- **Carried forward (obligations on later tasks):** (a) Tasks 4, 5 and 7 — a consumer that opens an envelope must re-derive `note_ref`/`record_hash` from the OPENED plaintext and compare to the row's column, never trust the column (no AAD binds a ciphertext to its row); the Task 7 review checks it. (b) Tasks 10 and 11 — `set_sync_key`/restore must compare `fingerprint()` to the account's generation BEFORE writing the key to Credential Manager (a mistyped 52-char code decodes silently to a wrong key). (c) Task 3's vectors should pin exact hex for all three index kinds. (d) The restore UI must not quote `BadKey`'s "recovery code" label on the base64 path.
- Task 2: fix round 1/5 dispatched (resumed implementer, sonnet).
- C0 aside (2026-09-17): PR #8 merged 274e137 on Quinn's word; the worktree and branch removed; main CI run 35192587097 watched. Knowlu 0.1.0 installed silently on the laptop from the Release asset (Task 5 Step 3's machine half; not launched by the controller).
- Aside (2026-09-17): the installed 0.1.0's uninstall entry is `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Knowlu` (Tauri's NSIS keys it by product name), while CLAUDE.md says the uninstall key is keyed by `com.knowlu.desktop`. A doc nit for the C0 close (Task 6), not a defect.
- Quinn's direction question (2026-09-17, during Task 2): multi-device parity across web, mobile and desktop. Recorded in HANDOFF §4 as an open direction item; C3 continues unchanged (its rows are client-agnostic; P1 (a) plus a later pairing flow is compatible).
- Task 2: fix round 1/5 — implementer DONE, commit 47f8698 (`load_key` maps only `NotFound{code:1168}` to `NoKey`, the rest to `KeyUnreadable`; `generate()` test; the prefix-pinning test); focused `sync::` 18/18, workspace 1259 passed / 0 failed / 4 ignored, the accepted `.rsrc` trio only; fixtures clean.
- Ruling R-C3-exec-12: `SyncError::KeyUnreadable` is now `KeyUnreadable(String)` (the folded cause, never key bytes); `label()`/`Display` unchanged. The plan names the variant only inside Task 2's own text, so no later task's code is affected; Task 8's and Task 10's dispatches carry the note. Cost if wrong: a one-line match arm.
- Task 2: scoped re-review dispatched (sonnet) on review-a7f20c4..47f8698.diff.
- Task 2: re-review (sonnet) — all three findings ADDRESSED (the `CredError` `Display` traced: prints target, code or a fixed reason, never blob bytes; the fixed-string `from_base64` failure; the exhaustive two-variant match), no new breakage. **Task 2: complete** at 47f8698 (commits baa5f0c H1a, a7f20c4, 47f8698).
- Ruling R-C3-exec-13: carried item (c) from Task 2's review (index-hash vectors in Task 3) is dropped — `the_domain_prefixes_are_load_bearing…` pins the prefixes in Rust, and the server never computes `record_hash`/`note_ref`/`device_token` (device-only by design), so a Deno-side index implementation would be code that exists only for a test. Cost if wrong: none.
- Task 3: dispatched 2026-09-17 (sonnet), BASE 47f8698. No hand-off.
- Task 3: implementer DONE 2026-09-17 — commit 934fefd (`sync_rows.ts` + test, `sync_vectors.json`, `sync_envelope_test.ts`, one Rust case); C3 files 9 passed, corpus 384/0 (375 + 9), Rust `sync::` 19/19, workspace green; both vector generators deleted; fixtures clean. **Task review NOT yet dispatched.**
- **PAUSED by Quinn 2026-09-17 ("let's pause C3 for a second and discuss the vision")** at 934fefd, after Task 3's commit and before its review. Resume: dispatch Task 3's task review (opus, BASE 47f8698 → 934fefd), then H10 + Task 4. Quinn's open direction question: multi-device parity (web, mobile) versus VISION commitment 2 ("plain text on the user's machine").
- **SUPERSEDED 2026-09-17.** Quinn's vision rulings (the cloud design's Amendment 2026-09-17, SIGNED 2026-09-17): desktop only; the account is the source of truth and the service can read the vault (P1 = (c), client-side encryption reversed, no key/recovery code/switch, the export dropped); the engine stays on the device; the fetch sequence moves to the cloud with the device as a credential relay; order C3′ → C5 → C4 → pilot, all before the pilot. This plan is not resumed. Tasks 1–3's commits (5422c8a, 87eb58d, 4907170, 9b693e4, c4a201b, baa5f0c, a7f20c4, 47f8698, 934fefd) stay on `c3-sync` for the C3′ plan to reuse: the two tables (plaintext columns replace the ciphertext shape), the purge list, the validators' shape checks; the envelope, the key, the vectors and `pub mod sync;`'s crypto sections are retired. The two staging migrations stay applied until C3′'s first migration supersedes them. Task 3's review was never dispatched.
- C3′ plan (`docs/plans/2026-09-17-c3-account-vault-plan.md`, 31f3549, 12 tasks) reviewed 2026-09-17 (`docs/reports/2026-09-17-c3-account-vault-plan-review.md`, 4aff334): execute after fix round 1 — C1 duplicate `state::run_sync` across H9a/H9b; C2 apply writes the foreign value AND files a card on a two-sided conflict (the silent merge); I1 `sha2` is a new engine crate (use `ring::digest`); I2 the grace pin reads the wrong file; I3 `name_of` undefined; I4 the second write exception unargued; I5 prose outlines for apply/run_lines_with/restore; I6 Task 12 stages hand-off files. Fix round 1 sent to the planner with rulings; a scoped re-review follows. C3′ executes after C1b merges, in a fresh worktree forked from main.
