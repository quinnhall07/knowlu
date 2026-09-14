# C3 sync plan — review (2026-09-14)

Plan: `docs/plans/2026-09-14-c3-sync-plan.md` (5,488 lines, 14 tasks, status WRITTEN, not started).
Spec: `docs/specs/2026-09-09-knowlu-cloud-design.md` (SIGNED 2026-09-09) — §1 D1/D4/D7/D9, §3.1, §4.1,
§4.4, §5.1, **§5.5**, §5.6, §6, §9, §10, §11 R4, §11a, §12's C3 row, §13.
Also weighed against `CLAUDE.md`, `HANDOFF.md` §2/§4, `VISION.md`,
`docs/plans/2026-09-09-c2-judge-plan.md` and `docs/reports/2026-09-09-c2-judge-plan-review.md`.

Reviewer: Claude (opus), independent of the writer. Three passes: a full sequential read; a
claim-by-claim check of every statement the plan makes about existing code (`engine/src/{ingest,
history,runs,write,reconcile,journal,ledger,ids,yaml,cli,lib,main}.rs`, `engine/tests/{site,
no_console,dependency_boundary}.rs`, `engine/tests/fixtures/vault-full/**`, `app/src/{lib,main,state,
commands,scheduler,account,credentials,scaffold,onboarding}.rs`, `app/tests/{onboarding,scaffold,
scheduler,commands}.rs`, `app/static/console.js`, `site/{privacy,index}.html`,
`cloud/supabase/functions/_shared/{http,db,crypto}.ts`, `cloud/supabase/functions/account/index.ts`,
`cloud/supabase/config.toml`, `.github/workflows/ci.yml`, `Cargo.lock`, `engine/Cargo.toml`); and a
consistency pass over interfaces, endpoint paths, test counts and the envelope's security properties.

C2's shipped interfaces were read READ-ONLY from `.claude/worktrees/c2-judge/engine/src/cloudmodel.rs`
and `.claude/worktrees/c2-judge/engine/tests/cloud_contract.rs` and
`.claude/worktrees/c2-judge/.github/workflows/ci.yml`. Where the plan consumes a C2 interface, the row
in Table A says whether it is C2's **shipped** or **planned** shape.

## Verdict

**NEEDS A FIX ROUND.** The architecture is right — two opaque tables, one engine module, the pull
before `rank`, `reconcile` reused with its roles reversed, a restore that is `/sync-pull` from zero —
and Tasks 1–8 are written to this repository's standard; but eight defects block execution, of which
three are structural (the restore route can never run against a vault the wizard has just scaffolded,
both edge handlers throw the refusals their own tests assert as return values, and the unkeyed content
hashes give the server exactly the guessing oracle the encryption promise forbids) and two are
ownership gaps that turn green suites red with no owned file to fix them. None needs a redesign: the
envelope, the cursor, the conflict path and the task order all survive the fixes.

## Findings

### Blocking

**B1. `plan.md:583-595`, `:3990-4010`, `:4521-4532` — a restore can never run: the wizard scaffolds notes before `restore_into` is called, and `restore` refuses a vault that holds any.**
H8 inserts the restore block into `create_vault_in` "immediately after the `move_session` block and
before `finish_or_roll_back`" — i.e. **after** `scaffold::create_vault` has already run.
`app/src/scaffold.rs:630` writes `tasks/get-to-know-knowlu.md` through `write::create`, and `:637-648`
writes a `courses/<slug>.md` note for every seeded course. `sync::is_empty_vault` is
`note_paths(vault).is_empty()` and `note_paths` walks all six of `ids::NOTE_FOLDERS`
(`engine/src/ids.rs:19-20`: `tasks, approvals, archive, courses, issues, info`). So `restore` returns
`SyncError::Io("… is not empty; a restore only ever fills a new vault")`, `restore_into` propagates it,
and `create_vault_in` does `std::fs::remove_dir_all(&dest)` and answers `ok: false`. **Task 11's entire
route fails on every real run**, and the only proof the plan has of it is a by-hand step (`:4982-4990`)
that would discover this on the first try.
The fidelity ledger compounds it: `:71` says the bound is "refuses any `dest` whose `tasks/` already
holds a `.md`", which is a *different, narrower* rule than the code's six-folder scan.
*Fix:* decide the shape and write it — either run the restore **before** `scaffold::create_vault`'s
note seeding, or have `restore_into` remove the known seed set (`tasks/get-to-know-knowlu.md` and the
`courses/<slug>.md` notes `seeded_courses` produced, together with their `create` records) before
calling `restore`, or give `restore` a `SeedAllowlist` it tolerates. Then make the ledger row and the
doc comment say the same thing the code does.

**B2. `plan.md:1912-1925`, `:1951-1954`, `:1962-1973`, `:2298-2311` — both handlers throw their refusals; six handler tests assert a returned `Response`.**
`cloud/supabase/functions/_shared/http.ts:18` is `export function fail(status, message): Response` — it
**returns**, and this codebase's convention (stated by the plan itself at `:224` and `:207`) is
`throw fail(...)`, caught by `asResponse` in `index.ts`. Neither `sync-push/handler.ts` (`:2014-2053`)
nor `sync-pull/handler.ts` (`:2359-2382`) contains the `try { … } catch (e) { if (e instanceof Response)
return e; throw e; }` wrap the plan's *Interfaces* contract 5 promises, and `deps.requireEntitled`
**rejects** with a `Response` in two tests. So:

> `const res = await handle(push({ device: DEVICE, records: [{ …, title: "CS 100 HW 1" }] }), deps());`
> `assertEquals(res.status, 400);`

rejects instead of resolving. C1's own suite does this correctly —
`cloud/supabase/functions/telemetry/handler_test.ts:79` is
`).catch((e) => e as Response);` on exactly these cases.
Affected: "a row with a plaintext field beside the ciphertext is a 400", "a device token that is not a
device token is a 400" (both assertions), "over five hundred rows of either kind is a 400", "the
entitlement gate answers before anything is read", and sync-pull's gate half of "anything but GET is a
405, and the gate answers first".
*Fix:* add `.catch((e) => e as Response)` to those six call sites (C1's pattern), **or** add the
documented catch to both `handle`s and leave the tests. Pick one and say which, because the plan
currently states one and writes the other.

**B3. `plan.md:3911-3939` — `a_restored_vault_ranks_the_same_day_as_the_one_it_came_from` cannot pass: `rank` reads three `state/` files a restore never carries.**
The test copies only `config/` and `profile/` into `dest`. But
`engine/tests/fixtures/vault-full/state/` holds `calendar.md`, `events.md` and `events-seen.md`, and
the golden the fixture is pinned to says so out loud:
`engine/tests/fixtures/golden-today-full.md:3` — "Capacity today: 6.0h (template 8.25h − **2.25h
calendar**)" — and `:5` — "**Events: 3 in today's digest**". The restore replays notes and journal
records only (`:4007-4036`), so `dest` has no calendar snapshot and no event roster and renders a
different `today.md` by construction.
*Fix:* copy `state/calendar.md`, `state/events.md` and `state/events-seen.md` alongside `config/` and
`profile/`, with a comment saying why (they are generated inputs, not synced state), or narrow the
assertion to the tasks section. Do **not** weaken it to "both files exist".

**B4. `plan.md:54`, `:5320-5337`, `:45` — `site/index.html` carries the same privacy sentence, is asserted by `engine/tests/site.rs`, and is not in C3's ownership.**
`site/index.html:16` is byte-identical to `site/privacy.html:13`, and `engine/tests/site.rs:26-27`
asserts **both**:

> `assert!(privacy.contains(PRIVACY), "the privacy page carries the exact sentence");`
> `assert!(index.contains(PRIVACY), "so does the download page");`

Task 13 step 4 replaces `engine/tests/site.rs:15`'s `PRIVACY` const and `site/privacy.html:13` and
`app/static/console.js:1371` — three of four places. C3's ownership line (`:45`) names `site/privacy.html`
and nothing else under `site/`, no hand-off covers `site/index.html`, and the task's `git add` (`:5370`)
omits it. The suite goes red with no owned file to fix it.
*Fix:* add `site/index.html` to C3's ownership, to Task 13 step 4's list of places the sentence lives
(making it four, not three), and to the task's `git add`.

**B5. `plan.md:392-393`, `:546-568`, `:5039-5049` — `app/src/main.rs` calls `state::refresh_head` and `state::refresh_history` at startup, and no hand-off removes them.**
`app/src/main.rs:169-177`:

> `// Both git calls (`refresh_head`, and `refresh_history`'s ahead/behind check) belong`
> `//   off the UI thread (F17) — spawned once at startup; Task 12's housekeeping thread repeats both every 60 s.`
> `    knowlu::state::refresh_head(&cs);`
> `    knowlu::state::refresh_history(&cs);`

H6 deletes both functions from `state.rs`; H7 covers `scheduler.rs`; H8's `main.rs` clause is explicitly
only "the two `generate_handler!` lists". So after the controller applies H6 the workspace does not
compile, and Task 12's own `no_git_process_is_spawned_for_a_vault` scans all of `app/src/` and fails on
`app/src/main.rs still has git's sync`.
*Fix:* extend H8 (or H6) with the exact `main.rs` replacement — delete the two calls and the comment,
and decide whether the spawned thread survives at all (it has nothing else to do).

**B6. `plan.md:753-755`, `:1468-1492`, `:738-749`, `:4717-4718`, `:5439` — the content hashes are unkeyed SHA-256 over guessable plaintext, so the server *can* read what the design says it cannot.**
`note_ref(rel_path) = SHA-256(path)` and `record_hash(record) = SHA-256(dumps_value(record))`, both
stored in clear, both checked by the migration to be "a sha-256 of the path, never the path". A hash of
a guessable string **is** the string to anyone holding the table. Vault paths are
`tasks/<slugified assignment title>.md` over a known slug alphabet and a per-course dictionary; a few
million candidates recovers most of a student's task list. Journal records are worse: the shape is
public (`{actor, device, field, id, new, old, op, path, seq, ts, via}` in `dumps_value`'s sorted,
Python-separator form), `op ∈ journal::OPS` (6), `via ∈ journal::VIAS` (5), `field` is a small closed
set, `old`/`new` are usually small integers, `seq` is bounded, and `received_at` bounds `ts` to a
narrow window — every one of those is enumerable offline against the stored hash.
That contradicts the migration comment ("Nothing here is readable by us", `:783-788`), exit-gate item
2 (`:5439`), the switch screen's "**We store it; we cannot read it**" (`:4717`) and the privacy
paragraph Task 13 publishes ("a key we never receive and cannot reconstruct — we store a blob we
cannot open", `:5328`). `the_note_ref_is_a_hash_of_the_path` and `no column C3 creates could hold a
note, a path or a title` both pass while the property they exist for is false.
*Fix:* key both with material derived from the sync key — `note_ref = HMAC-SHA256(K_index, path)` and
`record_hash = HMAC-SHA256(K_index, dumps_value(record))`, with `K_index = HKDF(sync key, "knowlu/index")`
so the AEAD key is never used as a MAC key. Same column types, same 64-hex shape, same `sync_records_once`
idempotence, same cursor — and no guessing oracle. `ring` already provides `hmac` and `hkdf`, so the
dependency row in Task 2 does not change. `device_token` (`:1490-1492`) has the same flaw at lower
stakes; key it the same way or say why a 64-bit truncated hash of `account_id + hostname` is acceptable.

**B7. `plan.md:2589` — the push contract test asserts `Authorization:` where ureq sends `authorization:`.**
`assert!(sent.contains("Authorization: Bearer jwt-not-a-secret"));`
C2's own contract test already learned this and documents it
(`.claude/worktrees/c2-judge/engine/tests/cloud_contract.rs:127-129`):

> `// case passed to .header(...), so the request line reads "authorization:", not`
> `// "Authorization:" — this checks the actual bytes sent, not the call-site spelling.`
> `assert!(sent.contains("authorization: Bearer jwt-not-a-secret"));`

*Fix:* lower-case it, and carry C2's comment across so the next person does not "fix" it back.

**B8. `plan.md:4440-4463` — turning the switch off and on again mints a new key: the old cloud copy is orphaned and the new one is silently incomplete.**
`turn_off` deletes the credential; `turn_on` then finds none and calls `SyncKey::generate()`. Two
consequences, neither named anywhere in the plan:
(a) every row already in `sync_records`/`sync_notes` becomes permanently unopenable — while the switch's
own screen says "Turning this off later stops the copying. **It does not delete what is already
there**" (`:4720`) — and those bytes still count against the 200 MiB ceiling, because
`sync_usage` is a trigger-maintained counter (`:876-894`) and nothing decrements it.
(b) `state/sync-cursor.json` survives the off/on, so `build_push`'s `cursor.notes` digest map still
says every unchanged note has been pushed (`:2904-2906`) — under the **old** key. The new copy is
therefore missing every note that has not changed since, and a later restore under the new recovery
code produces a partial vault with no warning.
*Fix:* `turn_off` must delete `state/sync-cursor.json` as well (or `turn_on` must, when the
fingerprint it is about to write differs from the one the cursor was built under — store the
fingerprint in the cursor), and the screen's off-copy sentence must say what happens to the old copy.
Consider making `turn_on` after a `turn_off` re-offer the *same* key when the account still holds rows,
or make `turn_off` offer "stop copying" versus "stop and forget the key" as two answers.

### Important

**I1. `plan.md:5`, `:73`, `:4852-4862` versus `HANDOFF.md:130` — the "restore from a backup" row Quinn decided on 2026-09-14 is about the *local mirror*, and the plan builds a cloud restore instead.**
`HANDOFF.md:130` reads: "**Restore from a backup: a link on the picker, built in C3** … `§4.2`'s nine
panels carry no restore panel and Task 17 built exactly those, so **`restore_vault` has no caller**".
`onboarding::restore_vault` / `restore_vault_in` (`app/src/onboarding.rs:684`, `:700`) copy a
`<profile>\vault\` mirror out of the `Backups\` folder; the command is registered in the wizard's
`generate_handler!` list and `app/static/console.js` has handlers for `#pick-adopt` and `#pick-add`
only. The plan quotes the row in its own opening line and then delivers §5.5's *cloud* restore. Both
are defensible features; the plan ships one and records the other as done.
*Fix:* either wire `restore_vault` to a second picker route in Task 11 (it is ~15 lines of `console.js`
plus a `#restore-backup` dialog), or put a fidelity-ledger row and a *What is NOT in this plan* entry
saying the local-mirror restore is deferred and `restore_vault` is still caller-less — and take it back
to Quinn, because the row is theirs.

**I2. `plan.md:772`, `:1619`, `:1730`, `:1824`, `:1978`, `:2169`, `:2316`, `:2410`, `:5362`, `:624-637` — `deno test --allow-read` will not run the merged suite.**
Every whole-tree run in the plan is `deno test --allow-read --config cloud/supabase/deno.json
cloud/supabase/`. C2's branch has already widened the flags
(`.claude/worktrees/c2-judge/.github/workflows/ci.yml:79`):
`deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config …`.
C3 merges after C2, so every one of those steps fails on C2's tests. H9's conclusion ("The only edit
C3 needs is none") is right about the *path* scope and silent about the *flags*.
*Fix:* use C2's full flag set in every whole-tree run (C3's own files need nothing beyond
`--allow-read`), and change H9's note to "verify the flags, not only the paths".

**I3. `plan.md:792-795`, `:2107-2126`, `:2378-2380` — the cursor is an identity column, which is assigned at INSERT and visible at COMMIT; a concurrent push can be skipped forever.**
Two overlapping `/sync-push` calls take `seq` 100 and 101; 101 commits first; a `/sync-pull` between
the two commits returns 101 and the device saves `record_cursor = 101`; row 100 then commits and is
never delivered, because every later pull asks `seq > 101`. `sync_notes_rev` has the identical hazard.
The plan's own claim — "The cursor is an identity column. A client asks for `seq > n`. There is no
clock two machines have to agree about" (`:792-795`) — is true about clocks and silent about commit
order. One device is mostly safe; two devices pushing in the same second are the case the feature
exists for.
*Fix:* read behind a lag — `received_at < now() - interval '10 seconds'` in `readRecords`/`readNotes`,
and clamp the returned cursor to that bound — or document the hazard in Task 14's *what production
still needs* beside "a second device has never been run".

**I4. `plan.md:3687-3699`, `:4182-4192`, `:3591-3607`, `:5440` — the session is checked before the key, so "not turned on" is usually reported as "no session", and exit-gate item 3 cannot be met.**
`run_lines` calls `cloudmodel::resolve` first; `resolve` is `load` + `session_token`
(`cloudmodel.rs:132-136`), so a machine that is signed out, or a pre-C1 vault, answers
`sync (skipped: no session)` before `load_key` is ever reached. The plan's own Task 8 test has to
hedge — `assert!(lines[0].contains("not turned on") || lines[0].contains("no session"))` — while
exit-gate item 3 asserts flatly "A vault with no key reports `sync (skipped: not turned on)`".
*Fix:* order it `load` → `load_key` → `resolve`. It also saves a Credential Manager read twice a day
on every machine that has not turned sync on, which is the common case.

**I5. `plan.md:4526-4530` with `:591-594` — an account whose cloud copy is genuinely empty deletes the vault the student just made.**
`restore_into` returns `Err("your account has no cloud copy yet — nothing was restored")` when the
pull came back empty, and `create_vault_in`'s restore block answers that by
`std::fs::remove_dir_all(&dest)` and `ok: false`. A student who signs in on a second machine before
the first has pushed loses the whole nine-panel wizard run and is told, accurately but uselessly, that
there is nothing there.
*Fix:* an empty copy is not a failure of vault creation. Keep the vault, finish onboarding, and say it
on the finish panel ("your cloud copy is empty — the next sync from your other machine will fill this
one"). Reserve the rollback for a key that does not open the rows (`SyncError::Crypto`) and for I/O
failures.

**I6. `plan.md:911-925`, `:55` — 400-day retention on `sync_records` silently truncates judge-once attribution on a restored machine.**
`sync_prune` deletes journal records older than `p_days`; note rows are kept. `journal::human_set`
(`engine/src/journal.rs:271`) is what judge-once reads, and it reads *records*. A student who restores
after a long absence gets every note but only the last thirteen months of history, so an agent may
re-set a field the student decided fourteen months ago — the exact failure the invariant exists to
prevent, arriving quietly. Nothing in the plan, the ledger or Task 14's production list says so.
*Fix:* say it in P3's text (it changes what the number *means*, which is Quinn's call), and record it
in Task 14's *what production still needs*. Consider keeping `op: set` records with a human actor
indefinitely and pruning only agent and `supersede` records.

**I7. `plan.md:3257-3282`, `:3439-3453` — pulled *records* are journalled verbatim and drive `delete`/`move_note` with no shape or path check, while pulled *notes* get three.**
`pulled_from_reply` accepts any decrypted JSON object as a `Record` and `apply` appends it with
`journal.ledger.append(record)`. `apply` then does
`crate::write::move_note(vault, rel, &to, ctx, journal)` where `to = str_of(r, "new")` — a value that
came out of an envelope, unchecked — and `crate::write::delete(vault, rel, …)` on `str_of(record, "path")`.
The careful guard at `:3410-3416` (`ids::inside_vault` + `NOTE_FOLDERS` prefix + `.md` suffix) is
applied to `pulled.notes` and never to `pulled.records`. `write::resolve_target`/`ids::inside_vault`
probably catch a traversal inside `write`, but the plan asserts nothing about it and
`a_pulled_note_may_not_escape_the_vault` covers only the note half. There is also no check that `op`
is in `journal::OPS` or `via` in `journal::VIAS`, so a malformed record enters the ledger — a file
`CLAUDE.md` calls a byte-identical contract with existing vaults.
*Fix:* validate every pulled record before it is appended (`op ∈ OPS`, `via ∈ VIAS`, `ts` parses,
`path` passes the same three-way guard, `id` matches `ids::ID_RE` when present) and drop the rest with
a named warning; add `a_pulled_record_may_not_escape_the_vault` beside the note test.

**I8. `plan.md:1006`, `:1164-1167`, `:1495-1498`, `:2709-2712`, `:3198-3201`, `:3638-3641`, `:3969-3972` — Tasks 2 and 6–9 cannot reach green without H1, which they are forbidden to apply.**
`engine/src/sync.rs` is not compiled until `pub mod sync;` is in `engine/src/lib.rs`, which is hand-off
H1 and which `:1531` tells the task not to edit. Task 2 step 2 half-says this ("until H1 is applied and
the file exists"), and then step 5 asserts "PASS, eleven cases. Then `cargo test --workspace` — green".
Tasks 6, 7, 8 and 9 give no caveat at all: `engine/tests/sync_contract.rs` opens with
`use knowlu_engine::sync::{self, Cursor, SyncKey};`, which does not compile, so four consecutive
commits ship a red workspace with no statement that they do.
*Fix:* say it once, plainly, the way C2's plan does — either every one of these tasks records "red
until H1 lands" in its report and its expected results, or H1 is applied at the *start* of the branch
as a one-line exception (it is additive, it breaks nothing, and the C2 precedent for a compile-blocking
`lib.rs` line exists).

**I9. `plan.md:5161`, `:5163`, `:5360` — Task 12 requires a controller round-trip in the middle of the branch, and the plan has no procedure for one.**
Step 7 says "Until they land the workspace does not compile, and the report says so plainly"; step 8
says "Run everything, **after the controller applies H1, H6 and H7 on the branch's base**"; Task 13
step 5 then says the same for H3/H7/H8. But the plan's own hand-off preamble (`:241`) says "the
controller applies them **on `main` at merge time**". Those two sentences cannot both be true, and an
implementer reaching Task 12 has nothing to do but stop.
*Fix:* name the procedure once, at the top of *Controller hand-offs*: which hand-offs are applied
mid-branch (H1's removal, H6, H7) and by whom, and which wait for the merge. If the answer is that
Tasks 12–14 execute only after a controller checkpoint, make it a numbered checkpoint in the task list.

**I10. `plan.md:4007-4036` versus `:4080-4106` — `restore_more` is a 26-line verbatim duplicate of `restore` minus one `if`.**
The bodies are identical from `let mut report = RestoreReport { warnings: page.warnings.clone(), …}`
through `Ok(report)`. The stated reason ("Split out rather than given a boolean parameter so a caller
cannot pass `false` by accident") justifies the two *names*, not the two *bodies*.
*Fix:* `fn materialise(vault, page) -> Result<RestoreReport, SyncError>` holding the shared body;
`restore` is the emptiness check plus `materialise`, `restore_more` is `materialise`. Both names
survive, one body does.

**I11. `plan.md:1553` versus `:1578`, `:1682`, `:1701`, `:2028-2029` — `checkRecord`/`checkNote` are declared with two parameters and defined and called with three.**
*Interfaces* says `checkRecord(row, device)`, `checkNote(row, device)`; the tests, the module and the
handler all use `(raw, device, accountId)`. *Fix:* the *Interfaces* line.

### Minor

**M1. `plan.md:2466`** — `use std::path::{Path, PathBuf};` in `engine/tests/sync_contract.rs`, but no
test in that file uses `Path`. Unused import is a warning, and 0 warnings is part of green.

**M2. `plan.md:1498`** — "PASS, eleven cases"; the module has **twelve** (`a_sealed_envelope…`,
`two_seals…`, `a_tampered…`, `another_key…`, `the_recovery_code…`, `a_code_that_is_not_a_key…`,
`the_fingerprint…`, `the_key_never_appears…`, `a_record_hash…`, `a_note_ref…`, `a_device_token…`,
`the_key_target…`). Task 3 step 7 then adds a thirteenth.

**M3. `plan.md:2996`** — "six cases appended"; ten are written, and the Files line at `:2973` says ten.

**M4. `plan.md:870`, `:381-382`, `:764`** — `public.sync_usage` is the third table C3 creates and it is
absent from H5's purge list and from `the account purge names both sync tables`. The FK cascade covers
it, but H5's own comment says the list is what the deletion paragraph is written from.

**M5. `plan.md:548`** — "The file is fifteen `pub mod` lines"; `app/src/lib.rs` has **fourteen**
(`account, commands, credentials, inference, lms_link, onboarding, profiles, report, scaffold,
scheduler, state, telemetry, tray, updates`). The placement claim ("between `state` and `telemetry`")
is right.

**M6. `plan.md:4612`** — `export_data` calls `crate::account::check_api_base(&cfg.api_base)?` after
`crate::account::cloud_config`, which already calls it (`app/src/account.rs:456`) *and* refuses an
`api_base` naming a different host than this build. Harmless, but it reads as a guard the caller needs.

**M7. `plan.md:1736`** — the one-line `deno eval` embeds `a \" quote` inside a double-quoted argument.
PowerShell does not treat `\` as an escape when building a native command line; this will not survive
the shell as written. Put the generator in a temp `.ts` file and `deno run` it, or use single quotes
and a different test string.

**M8. `plan.md:4304-4313`** — `sync_is_off_until_a_key_exists` deletes a credential and then asserts it
is absent. It exercises no product code and would pass with `app/src/sync.rs` empty. Make it assert
`sync::is_on(&cs) == false` on a `ConsoleState` whose profile has no key, which is the property the
privacy policy names.

**M9. `plan.md:2809-2824` versus `:3421-3423`, `:4022-4024`** — `note_paths` is non-recursive
(`read_dir` per folder), so a note at `tasks/sub/x.md` is never pushed; but `apply` and `restore` both
`create_dir_all(parent)` for a pulled path, so the two halves disagree about whether nesting exists.
Say that `NOTE_FOLDERS` are flat, or make the write side refuse a nested path.

**M10. `plan.md:3687-3753` and `:4170-4260`** — Task 8 writes `run_lines`'s ~65-line body in full and
Task 10 replaces it wholesale with `run_lines_with`. The plan says so at `:4263`, which makes it
recorded rather than accidental, but it is two reviews of the same code and a commit whose content is
dead two tasks later. Consider writing `run_lines_with` in Task 8 and the thin `run_lines` beside it.

**M11. `plan.md:3377-3378`, `:2852`, `:3466`** — `apply` builds its `known` set from
`journal.read(None, None)` (the whole ledger) and then calls `journal.records_for(id, None)` per note,
and `build_push` reads the whole ledger again in the same run. Fine for a first-year vault; worth a
line in Task 14's production list beside the other honest numbers.

**M12. `plan.md:1490-1492`** — `device_token` is `SHA-256(account_id + "\n" + device_name())[..16]`.
The hostname is a small guessable space and the account id is in the same row, so the token is
reversible by anyone holding the table. Lower stakes than B6 but the same mistake; fix it with the
same `K_index`.

**M13. `plan.md:1334-1385`** — the recovery code carries no checksum. A mistyped code decodes to a
valid-looking 32-byte key and surfaces as "the copy could not be opened with this key", which reads as
"the wrong code" *or* "the data is corrupt". Crockford base32 has a standard check symbol; one extra
character would tell the two apart on the screen where it matters.

**M14. `plan.md:5330`** — "The *Export* paragraph's last two sentences" — it is a `<li>` in the rights
list at `site/privacy.html:96`, not a paragraph. The quoted replacement is right; the locator is not.

**M15. `plan.md:4138-4148`** — Task 10's *Produces* omits `turn_on`, `turn_off` and `stage_key`, which
its own tests (`:4320`, `:4335`, `:4350`) and its own commands (`:4581`, `:4586`, `:4656`) use.

**M16. `plan.md:352`** — "The array at `cloud/supabase/functions/account/index.ts:57-66`"; it is at
**`:52-61`**. The quoted contents match exactly.

**M17. `plan.md:1612-1614`** — `the note cap matches the column` pins `MAX_NOTE_CIPHERTEXT` to 196608;
there is no twin for `MAX_RECORD_CIPHERTEXT` against `sync_records`'s 24576. Add one — the two caps
live in two files and nothing else holds them together.

**M18. `plan.md:4397` with `:551` and `:397`** — `app/src/sync.rs` carries `#![cfg(windows)]` *and*
`lib.rs` declares it `#[cfg(windows)]`, while H6's new `pub sync: Mutex<crate::sync::SyncStatus>` field
on `ConsoleState` is ungated. The app is Windows-only in practice so this compiles; drop one of the two
gates and note that `state.rs` now depends on a gated module.

**M19. `plan.md:68`, `:70`, `:5438`** — the spec writes `/sync/push` and `/sync/pull`; the plan
correctly uses the hyphenated Supabase function names throughout (C2's review, Critical 1), and exit
gate item 1 explains why — but the fidelity ledger's two `§5.5` rows quote the slashed paths without
recording the rename. One clause in the ledger closes it.

**M20. `plan.md:5128-5133`** — the `no_console.rs` floor change to `>= 2` is correct and the reasoning
("Two, then one") checks out: `Command::new` appears in non-test code in exactly `history.rs`,
`runs.rs` and `runtime.rs` (`childproc.rs`'s occurrences are all inside its `#[cfg(test)]` module).

## Table A — shared interfaces, producer vs consumer

| Producer (where) | Consumer (where) | Verdict |
|---|---|---|
| C2 **shipped** `cloudmodel::{CloudConfig, load, resolve, Unavailable, CloudError, CloudClient::{new,account_id,post,get}, CALL_TIMEOUT}` (`c2-judge/engine/src/cloudmodel.rs:42-258`) | plan `:181-199` *Interfaces* 4; `:2934-2948` `push`; `:3287-3295` `pull`; `:3677`; `:4522-4524` | **matches, field for field.** `post` builds `base + path` and sends `dumps_value`, so `POST /functions/v1/sync-push` is the right request line. One consumer defect: header case — **B7**. |
| C2 shipped `CloudError::label()` (`:152-164`) | plan `:199`, `:2661`, `:3733` | **matches**: 401→`no session`, 402→`no entitlement`, 429→`rate limited`, transport→`no network`. |
| C2 **planned** `ingest.rs` hand-off H3 (the service call in front of the empty-URL return) | plan `:318-344` H3 amends it | Correctly framed as an amendment. `engine/src/ingest.rs:741-742` on `main` is byte-for-byte what the plan quotes. |
| C2 **planned** hand-off H4 `cloud:<name>` → `/ingest-calendar?name=<name>` | plan `:346-348` H4 (no-op), `:5306-5315` | Correctly recorded as a dependency C3 cannot land without. |
| C1 `_shared/http.ts::{json, fail, methodNotAllowed, readJson, asResponse}` (`http.ts:13-65`) | plan `:1634`, `:1994`, `:2338` | Names and arities match. `fail` **returns**, and the handlers `throw` it without catching — **B2**. |
| C1 `_shared/db.ts::{Rest, restSelect, restUpsert, restFromEnv}` (`db.ts:12-122`) | plan `:2067-2126` `sync_db.ts` | **matches.** `restUpsert(rest, table, rows, onConflict?)` ✓; `restSelect(rest, table, query)` ✓; `max_rows = 1000` in `config.toml:11` ✓ so `MAX_PAGE = 500` is safe. |
| C1 `_shared/crypto.ts::{importAesKey, encryptString, decryptString}` (`crypto.ts:36-53`) | plan `:1551`, `:1736`, `:1766-1795` | **matches.** `encryptString` returns `{ciphertext, iv}` with a 12-byte IV, base64 — 16 chars ✓, and `crypto.subtle.encrypt` appends the GCM tag, which is what `ring`'s `seal_in_place_append_tag` does ✓. The cross-language vector is the right test. |
| C1 `_shared/entitlement.ts::requireActiveEntitlement(req)` | plan `:201-207`, both `index.ts` | **matches in signature**, contradicted in the handlers' shape — **B2**. |
| C1 `account/index.ts` purge array (`:52-61`) | plan H5 `:350-386` | Contents match; the line range does not (**M16**); `sync_usage` is omitted (**M4**). |
| Task 1 `sync_records`/`sync_notes` columns | Task 3 `sync_rows.ts` caps; Task 4 `sync_db.ts` | **matches**: 24576 / 196608, `device ~ ^[0-9a-f]{16}$`, `note_ref ~ ^[0-9a-f]{64}$`, `length(iv) = 16`. Only the note cap is pinned by a test (**M17**). |
| Task 2 `seal`/`open`/`record_hash`/`note_ref`/`device_token`/`SyncKey` | Tasks 3, 6, 7, 9, 10 | **matches.** `base64 0.22.1` and `ring 0.17.14` are already in `Cargo.lock` (via rustls), so the "adds a direct edge and no new crate" claim holds. |
| Task 6 `Cursor`/`build_push`/`push`/`note_paths`/`PAGE` | Tasks 7, 8, 9, 10 | **matches.** `PAGE = 500` on both sides of the wire. |
| Task 7 `Pulled`/`PulledNote`/`pull`/`pulled_from_reply`/`apply`/`ACTOR` | Tasks 8, 9, 10 | **matches.** `pulled_from_reply` is correctly exposed so Task 9 needs no server. |
| Task 8 `run_lines`/`Direction`/`is_configured` | H2 `:302-315`; Task 10 replaces `run_lines` | H2's `sync::Direction::parse(&direction).unwrap_or(Direction::Both)` ✓. Replacement recorded (**M10**). |
| Task 10 `run_lines_with`/`Totals` | `app/src/sync.rs::run_here` `:4503` | **matches.** |
| Task 10 `app/src/sync.rs::{key_target_for, PENDING_TARGET, is_on, run_here, move_pending_key, restore_into, SyncStatus}` | H6 `:397`, H8 `:587-591`, Task 11, Task 12 | **matches**, with `turn_on`/`turn_off`/`stage_key` undeclared (**M15**) and the `#![cfg(windows)]`/ungated-field mismatch (**M18**). |
| `engine::write::{write, delete, move_note, propose_amendment, find_pending_amendment, WriteContext, WriteOpts}` (`engine/src/write.rs:132-650`) | plan `:3444-3534` | **matches, checked signature by signature.** `Value` in `write.rs` is `serde_yaml_ng::Value` ✓, `WriteResult.written: Vec<(String,String)>` ✓, `WriteOpts: Default` ✓, `propose_amendment(vault, target_path, meta, changes, ctx, journal, evidence, today)` ✓. `WriteOpts::default()` has `judged: false`, so judge-once does **not** gate a sync write — correct for this design, and worth one sentence in the plan. |
| `engine::reconcile::resolve` (`engine/src/reconcile.rs:78-170`) | plan `:2985-2994`, `:3472` | **matches, and the role reversal is right.** `res.apply` is filled in two places: the "upstream never moved" branch (no supersede) and `if local_wins` (with supersede). So "a field with a supersede record is a conflict" is exactly true, and "a conflict this device won writes nothing" follows from `if local_wins` guarding the insert. |
| `engine::approvals::{AMENDABLE_FIELDS, AMENDABLE_FOLDERS}` (`:42`, `:54`) | plan `:3482`, `:3501` | **matches** (9 fields, `["tasks","courses"]`). |
| `engine::journal::{Journal, ledger, append, read, records_for, invalidate, device_name, now_ts, DEVICE_ENV_MUTEX, NewRecord, make_record, VIAS, OPS}` | plan throughout | **matches.** `Journal.ledger` is `pub` ✓, `DEVICE_ENV_MUTEX` is `pub(crate)` and is used only from inside `engine/src/sync.rs`'s own `mod tests` ✓. |
| `engine::ledger::{Record, dumps_value, JsonlLedger::{new, append}}` | plan `:1473`, `:4028-4033` | **matches.** `dumps_value` sorts keys (`ledger.rs:365-375`), so `record_hash` is order-independent and the Task 2 test holds. |
| `engine::cli::{run, vault_zone}`, `knowlu_engine::PINNED_FIXTURE_DATE` | plan `:3705`, `:3930-3932` | **matches.** `run(vault, Option<&str>, runner, Option<&str>) -> Result<RunOutcome, _>` with `pub output: PathBuf` ✓. |
| `app::scheduler::{slot_argv, JudgePlan, IcsState, has_ics_url}` (`app/src/scheduler.rs:270-330`) | H7 `:477-544`, Task 13 test `:5237-5255` | **matches**: the "before" snippet is byte-for-byte `slot_argv`'s first three lines, and `JudgePlan::{Cloud{log_dir},Local,Skip}` already exists on `main`. |
| `app::state::{ConsoleState::open, settings, vault_io, lock, quit_flush}` | H6 `:392-419`, Task 10 tests | **matches.** `open(vault: PathBuf, app_data_dir: PathBuf)` ✓; `quit_flush`'s git branch is `state.rs:239-242` exactly as quoted. |
| `app::commands::{build_state_value, sync_inner}` (`app/src/commands.rs:26-35`, `:261-269`) | H6 `:421-473` | **matches**, line ranges included. |
| `app::credentials::{target_for, write, exists, delete}` (`credentials.rs:17-60`) | `app/src/sync.rs` | **matches**, arities included. |
| `app::account::{cloud_config, auth_base, valid_access_token_at, check_api_base, PENDING_TARGET, move_session}` | `:4137`, `:4604-4612` | **matches**; `CloudConfig`'s four fields ✓. One redundant call (**M6**). |
| `app/src/main.rs`'s two `generate_handler!` lists | H8 `:556-568` | **43 / 27 confirmed by count**, union 59, and the arithmetic 46 / 28 / 63 is right. The *insertion points* named (after `report::report_send`, after `onboarding::campus_search`) are the actual last entries ✓. The **startup thread** in the same file is uncovered — **B5**. |
| `scripts/wizard-check.py::BEFORE_FINISH_OK` (`:32-37`) | H8 `:604-611` | **matches**, character for character, plus `stage_restore_key`. |
| `app/static/console.js::PANELS` (`:1365`) | Task 11 test `:4973` | **matches**, character for character. |
| `app/static/console.js::PRIVACY` (`:1371`), `engine/tests/site.rs::PRIVACY` (`:15`) | Task 13 `:5322-5337` | Both located correctly; `site/index.html:16` is the uncovered fourth copy — **B4**. |
| `app/tests/{onboarding,scaffold,scheduler,commands}.rs` helpers | Tasks 11, 12, 13 | **every line number checked and correct**: `onboarding.rs:11 tmp`, `:236 CREDMAN_LOCK`, `:341 base_plan`; `scaffold.rs:6 temp`, `:15 plan_for`; `scheduler.rs:2` use-list, `:11 scratch`; `commands.rs:267/:332/:342` are the three named tests. |

## Table B — per-task internal consistency

| Task | Tests vs code | Files vs later tasks | Commands and paths | Verdict |
|---|---|---|---|---|
| 1 — tables, ceiling, purge | 5 Deno cases, all over text the migration actually contains; `columnLines` strips comments, which is the right call | creates the two tables Tasks 4/5 read and the third (`sync_usage`) only `index.ts` reads | `supabase db push --workdir cloud`, `db query --linked --workdir cloud`, migrations stamped `20260912…`, no Docker ✓ | **Sound.** M4 (purge omits `sync_usage`), M16 (line range). |
| 2 — envelope, key, recovery code | 12 unit cases; every identifier defined in the same step or already in-crate; the 52-character Crockford round trip is arithmetically correct (256 bits → 51 + 1 symbols) | produces everything Tasks 3, 6, 7, 9, 10 consume | `cargo test -p knowlu-engine sync::tests` — needs H1 | **Sound but unrunnable as staged** — I8. M2 (count), M13 (no checksum). |
| 3 — server envelope, cross-language vectors | 6 validator cases + 3 Deno vector cases + 1 Rust; the "written once by hand, never regenerated" rule is right and well argued | `sync_vectors.json` under `cloud/`, read from Rust by relative path — correctly outside `engine/tests/fixtures/` | `deno eval` one-liner is shell-fragile (M7); `deno test --allow-read` too narrow (I2) | **Sound.** M7, I2, I11. |
| 4 — `POST /sync-push` | 9 handler cases; dedupe-within-batch, ceiling 413, 405, gate-first are the right nine | produces `sync_db.ts` Task 5 consumes | `functions deploy --use-api`, `--project-ref <staging ref>`, hand-typed smoke bodies ✓ | **Blocked by B2** (four of the nine cases). |
| 5 — `GET /sync-pull` | 7 handler cases; the clamp, the empty-page cursor hold and "a restore is this endpoint from zero" are the right properties | consumes Task 4's `sync_db.ts` only | as Task 4 ✓ | **Blocked by B2** (one case). I3 (cursor ordering) belongs here. |
| 6 — cursor and push | 8 loopback cases; harness verbatim from C2; the `boundary` tie-break is genuinely right and the reasoning at `:2743-2746` is the best paragraph in the plan | `sync_contract.rs` extended by Tasks 7, 8, 13 | `cargo test -p knowlu-engine --test sync_contract` | **Blocked by B7**; M1 (unused import), I8. |
| 7 — pull, reconcile, amend card | 10 cases (Files says ten, Step 1 says six — M3). Traced `a_field_both_devices_moved…` through `reconcile::resolve` by hand: it files one card, and the second pull is absorbed by `find_pending_amendment` — the test passes | shares `sync.rs` with 6, 8, 9, 10 | ✓ | **Sound**, with I7 (unvalidated records) and M3. |
| 8 — the command | 4 cases; `every_refusal_is_a_named_line` over three malformed YAMLs is a good tripwire | H2 and H7 named, not edited ✓ | `cargo run -p knowlu-engine -- sync --vault <scratch>` against a scratch vault ✓, never a real one ✓ | **I4** makes one test hedge and exit-gate item 3 unmeetable. |
| 9 — restore and replay | 4 cases; `round_trip` correctly builds a reply rather than a server; `vault-full` is 8 records / 8 notes, comfortably under `PAGE` | `restore`/`restore_all`/`restore_more` consumed by Task 10's `restore_into` | `cargo test -p knowlu-engine --test sync_replay`; relative fixture path resolves from the crate root ✓ | **Blocked by B3**; I10 (duplication). |
| 10 — switch, screen, code, export | 7 app cases behind a file-scoped `CREDMAN_LOCK` with a `Drop` guard ✓; `static_assets.rs` case counts the five sentences inside the array's own slice, which is the robust way | rewrites Task 8's `run_lines` (M10); H6 and H8 named | `cargo test -p knowlu --test sync`; `python scripts/settings-check.py` | **Blocked by B8**; M8, M15, M18, M6. |
| 11 — picker restore | 2 onboarding cases + 1 static-assets case; `base_plan` gains `restore: false` in the same commit ✓ | consumes Task 10's three app functions | `python scripts/wizard-check.py`; the by-hand five-step proof is the right shape | **Blocked by B1**; I1, I5. |
| 12 — git leaves | 4 `no_git.rs` cases; `no_git_library_is_in_either_manifest` uses three `include_str!`s, all of which resolve | deletes `history.rs`; needs H1, H6, H7 | `git rm`, `git diff --stat main...c3-sync` | **Blocked by B5**; I9. The `no_console.rs` floor change is right (M20). One vacuous assertion: `Command::new("git")` never appeared in `app/src/` — `history.rs` spawns via `git_with("git", …)` in `engine/src/`. |
| 13 — capability URLs leave the vault | 2 scaffold cases + 1 scheduler case + 1 ingest case; `ingest::run_lines(&dir, "cli", None, None)` matches the real 4-arg signature ✓ | needs H3, H4, H7b, H8 | `cargo test -p knowlu --test scaffold --test scheduler` | **Blocked by B4**; M14, M19. |
| 14 — close | the account-scoping scan is a real backstop; the eol and frozen-reference checks are the right two | `git add` list is complete for what the task writes | `git ls-files --eol`, `scripts\ci\eol-check.ps1` ✓ | **Sound**, but its "extend C2's scan **or** add C3's own" is an unresolved either/or inside an exit-gate item. |

## Table C — spec coverage

One row per sentence of §5.5, per §12 C3-row item, and per `HANDOFF.md` §4 row marked for C3.

| Source | Sentence | Carried by | Verdict |
|---|---|---|---|
| §5.5 Up | "the journal is already an append-only, per-day, `ts`-ordered ledger" | Task 6 `build_push`'s `pushed_through` + `boundary` | **Carried.** The millisecond tie-break is handled properly. |
| §5.5 Up | "The client uploads new journal records (and the note text they produced) to `/sync/push`" | Tasks 4, 6 | **Carried**, endpoint renamed to `/sync-push` for Supabase's router — right call, unrecorded in the ledger (M19). |
| §5.5 Up | "the server stores them per account in Storage (encrypted at rest…)" | Task 1 | **Narrowed on the record** (ledger `:68`): two Postgres tables instead of Storage. The argument (one access path, RLS, an identity-column cursor, one purge list) is correct and I would rule the same way. |
| §5.5 Up | "client-side encryption with an account-derived key is *recommended* for note bodies — **confirm**" | Tasks 2, 4, 5; §11 R4 ruled "yes (default)" | **Carried and widened on the record** (ledger `:69`): everything is sealed, not only bodies, because a `set` record carries `old`/`new` for `title`. Correct. **But the promise is not delivered as written — B6**: the unkeyed `record_hash` and `note_ref` are an offline oracle over the same plaintext. |
| §5.5 Up | "an account-derived key" | P1, ledger `:69`, narrowing 4 `:5476` | **Narrowed on the record** to a device-generated key + recovery code, with the password-reset argument spelled out and the reversal cost bounded to one function. Correct handling; it is P1 and Quinn's. |
| §5.5 Down | "the service's own writes … are queued as journal-shaped records the client pulls at `/sync/pull` and applies through `write` … `actor: agent:knowlu.<kind>` and judge-once intact" | ledger `:70`, *What is NOT in this plan* `:5454` | **Narrowed on the record, twice, and the structural reason decides it**: a server-originated row could not be sealed with a key the server does not hold. Verified against C2: `/judge-task`, `/ingest-coursework`, `/judge-event` and the Gmail/rule queues all deliver by pull inside a slot step. I agree with the narrowing. |
| §5.5 Second device | "replaying the journal onto an empty vault reconstructs it" | Task 9 | **Carried**, with one recorded exception to `CLAUDE.md`'s write rule, argued convincingly (re-journalling a restore would make `journal::human_set` answer with the restore). **Its proof does not pass — B3**, and the "empty vault" bound is wrong for the only caller — **B1**. |
| §5.5 Second device | "Conflicts (two devices editing one field offline) surface as amend cards, never silent merges (VISION)" | Task 7 | **Carried**, narrowed on the record for fields outside `AMENDABLE_FIELDS`/`AMENDABLE_FOLDERS` (reconcile's rule + a supersede record + one named warning). Traced through `reconcile::resolve`: the card path is correct, the no-card paths are correct, and the 15/day cap applies because it goes through `propose_amendment`. |
| §5.5 Backups | "the local snapshot tick stays" | `engine/src/backup.rs` untouched; exit gate 11 | **Carried**, and pinned ("byte-identical to `main`"). |
| §5.5 Backups | "the cloud copy is the durable one" | Global Constraints `:25`; exit gate 3 | **Carried**, and inverted correctly: the cloud copy is durable but never authoritative for a running device. |
| §5.5 Backups | "'Restore from cloud' is a wizard entry point" | Task 11 | **Carried** as a picker link + the ordinary nine-panel wizard with `restore: true` — genuinely a wizard entry point, not a tenth panel. **Cannot run — B1.** |
| §12 C3 row | "`/sync/push`, `/sync/pull`" | Tasks 4, 5 | Carried. |
| §12 C3 row | "journal replay" | Task 9 | Carried (B3). |
| §12 C3 row | "restore-from-cloud" | Task 11 | Carried (B1). |
| §12 C3 row | "second-device support" | Tasks 6–8 | Carried; never actually run with two devices, and the plan says so (`:5457`). **I3** is the hazard that only appears with two. |
| §12 C3 row | "`history.rs`'s git removed" | Task 12, ledger `:74` | Carried; `runs::git_sha` correctly kept and argued. **B5** is the gap. |
| §12 C3 row | "local snapshots kept" | `backup.rs` untouched | Carried. |
| §4.1 | "The vault is **not** a git repository and the app never assumes git exists" | Task 12 + `no_git_process_is_spawned_for_a_vault` | Carried (B5). |
| §4.4 | "What the app stops doing: git commit/push/rebase (`history.rs`)" | Task 12, P4 | Carried. |
| §9 | Alabama § 8-38 SPII at rest | Tasks 1, 4, 13 | Carried, and Task 13's re-argument for removing `ics_url` (a capability credential in plain text) is better than C2's stated reason and is correctly put on the record as a correction (ledger `:76`). |
| §9 | "Export / access / delete implemented for everyone" | Tasks 1, 10; H5 | Carried. Export is a real settings row calling C1's `GET /account/export`; delete gains two of three table names (M4). |
| §11a | the personal calendar's secret iCal address | Task 13 | Carried: `ics_url: 'cloud:personal'`, routed by C2's H4. |
| §11 R4 | client-side encryption of note bodies — "yes (default)" | Tasks 2, 4, 5 | Carried in shape; **the guarantee is not achieved — B6.** |
| §6 (b) | corrections from the journal to `POST /telemetry`, no note body | ledger `:78` | **Deliberately untouched, on the record.** The argument for keeping telemetry and sync separate (two consents, two purposes) is right. |
| §5.6 | judgment logs never enter the vault | ledger `:86` | Carried: `state/sync-cursor.json` holds two integers, a timestamp and a path→hash map. Note that the **path→hash map is plaintext paths in a vault file** — harmless (they are the file names) but worth knowing it exists. |
| §13 | "the console's visual redesign (parked)" | Task 12 rewrites exactly one line of the page | Carried. |
| `HANDOFF.md` §4 | "**Restore from a backup: a link on the picker, built in C3**" | Task 11 | **NOT carried as decided — I1.** The row is about the local `Backups\` mirror and the caller-less `onboarding::restore_vault`; the plan builds a cloud restore and leaves `restore_vault` unreachable. |
| `HANDOFF.md` §4 | "export stays by email until a settings row in C3" | Task 10 `export_data` + Task 13's policy rewrite | **Carried**, both halves. |
| `HANDOFF.md` §4 step 6 | "the in-app overlay sells monthly only" / "no magic link in the overlay" | — | Not C3's; correctly untouched. |
| `HANDOFF.md` §2 | file ownership, disjoint streams | `:45` | **Two gaps**: `site/index.html` (B4) and `app/src/main.rs`'s startup thread (B5). |
| `CLAUDE.md` | every note write through `write`, journal first | Task 7 `apply` | Carried; the one exception (restore) is argued and bounded. |
| `CLAUDE.md` | no note parsed and re-dumped | `pystr::write_text` of exact bytes | Carried. |
| `CLAUDE.md` | `VIAS`/`OPS`/ledgers/frontmatter byte-identical | `:23` | Carried by intent; **not enforced on the pull path — I7**. |
| `CLAUDE.md` | `yamlemit.rs` is the one YAML emitter | no YAML is emitted by this stream | Carried. |
| `CLAUDE.md` | 15 proposals a day, overflow snoozed | Task 7 goes through `propose_amendment` | Carried. |
| `VISION.md` | "Sync replays the journal … Accounts over device pairing. Conflicts surface as proposals, never silent merges." | the whole plan | Carried. |

## Rulings needed from Quinn

1. **R-C3-1 (P1).** The sync key: device-generated + a 52-character recovery code (recommended), versus
   password-derived, versus server-held. The plan's recommendation (a) is right and its cost — lose the
   code and the laptop, lose the cloud copy — is stated honestly on the screen.
2. **R-C3-2 (P2).** The five sentences on the switch's screen and the three rewritten privacy
   paragraphs. Nothing merges until both are read; the live policy currently says the switch does not
   exist. Note **B6**: two of those sentences ("We store it; we cannot read it"; "a key we never
   receive and cannot reconstruct") are not true of the design as written until the hashes are keyed.
3. **R-C3-3 (P3).** 200 MiB per account and 400 days of journal retention. **I6** changes what the
   second number means: at 400 days a restored machine loses judge-once attribution for anything older,
   so this is a product decision about history, not only about storage.
4. **R-C3-4 (P4).** The go to delete `engine/src/history.rs` (707 lines).
5. **R-C3-5.** Which restore did you mean on 2026-09-14 — the local `Backups\` mirror (`HANDOFF.md`
   §4's row, and the caller-less `onboarding::restore_vault`), the cloud copy (§5.5), or both links on
   the picker? The plan builds the cloud one and records the local one as done. **I1.**
6. **R-C3-6.** Off-then-on: should turning the switch back on re-use the account's existing key
   (keeping the old cloud copy readable) or mint a new one and abandon the old rows? **B8.** Either
   answer is defensible; the plan makes the second silently.
7. **R-C3-7.** `/sync-pull` returns this device's own rows and the device filters them by hash
   (deferred minor M1 in the plan). Accept the wasted page, or add `device=neq.` on the server and keep
   the hash filter for the re-install case?
8. **R-C3-8.** Task 12 leaves the branch unbuildable until three hand-offs are applied mid-branch.
   Is a controller checkpoint between Tasks 11 and 12 acceptable, or should H1/H6/H7 be applied at the
   start of the branch? **I9.**

## Stale facts

Everything below is a claim the plan makes about existing code that is wrong, with the correct value.
Everything *not* listed here was checked and is right — including all four `app/tests/*` helper line
numbers, `console.js:1365`'s `PANELS` literal, `console.js:1371`'s `PRIVACY`, `engine/tests/site.rs:15`,
`engine/src/ingest.rs:741-742`, `app/src/commands.rs:26-35` and `:261-269`, `app/src/state.rs:239-242`,
`app/src/scheduler.rs`'s `slot_argv` and `JudgePlan`, the 43/27/59 command counts,
`scripts/wizard-check.py:32-37`, `config.toml`'s `max_rows = 1000`, and C2's `cloudmodel` signatures.

| Plan says | Actually |
|---|---|
| `:352` "The array at `cloud/supabase/functions/account/index.ts:57-66`" | It is at **`:52-61`**. Contents match. |
| `:548` "`app/src/lib.rs` … is fifteen `pub mod` lines" | **Fourteen**. |
| `:2589` `"Authorization: Bearer jwt-not-a-secret"` | ureq sends **`authorization:`** (lower case); C2's own test says so at `cloud_contract.rs:127-129`. |
| `:1498` "PASS, eleven cases" (Task 2) | **Twelve** are written; Task 3 adds a thirteenth. |
| `:2996` "six cases appended" (Task 7) | **Ten** are written; the task's own Files line says ten. |
| `:5019-5022` "`history.rs` drove the `git` *executable* through `Command::new("git")`" | It drives it through `git_with("git", …)` → `Command::new(program)` (`engine/src/history.rs:29`, `:39`, `:42`). The literal `Command::new("git")` appears in this repo only at `engine/src/runs.rs:63`, so Task 12's `assert!(!code.contains("Command::new(\"git\")"))` over `app/src/` was never true of anything and passes vacuously. The two assertions beside it (`history::`, `refresh_history`/`run_sync`) are the real ones — and they are what **B5** trips on. |
| `:54` "three pieces of published copy must be re-read and re-published" | **Four**: the sentence also lives at `site/index.html:16`, and `engine/tests/site.rs:27` asserts it there. **B4.** |
| `:5330` "The *Export* paragraph's last two sentences" | It is a `<li>` at `site/privacy.html:96`, not a `<p>`. |
| `:71` restore "refuses any `dest` whose `tasks/` already holds a `.md`" | `is_empty_vault` uses `note_paths`, which walks all six of `ids::NOTE_FOLDERS` (`tasks, approvals, archive, courses, issues, info`). |
| `:637` "`deno test` … `deno lint` and `deno test` walk `cloud/supabase/` whole … The only edit C3 needs is none" | True of the paths, false of the flags: C2's branch already widened `deno test` to `--allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS`. **I2.** |
| `:3920-3927` "`config/` and `profile/` are not synced … here they are copied so the two runs are comparable at all" | Not sufficient: `rank` also reads `state/calendar.md`, `state/events.md` and `state/events-seen.md`, and `golden-today-full.md:3`/`:5` prove both contribute to the rendered day. **B3.** |
| `:5130-5133` "What is left is `runs::git_sha` … and `runtime.rs`. Two, then one" | Correct as stated — verified: non-test `Command::new` occurs in `history.rs`, `runs.rs`, `runtime.rs` only (`childproc.rs`'s are all inside `#[cfg(test)]`). Listed here only because the arithmetic is easy to get wrong and it is right. |
| `:1172-1179` "`base64` and `ring` … Already in this workspace's `Cargo.lock`" | Correct — `ring 0.17.14` and `base64 0.22.1` are both resolved today (`Cargo.lock:3396`, `:314`). Note the lockfile also carries `base64` 0.21.7 and 0.23.1 from other edges; `base64 = "0.22"` unifies onto the one already there. |

## Fix round 1 — resolutions (2026-09-15)

Applied to `docs/plans/2026-09-14-c3-sync-plan.md` (now 6,352 lines, 14 tasks, status *WRITTEN … reviewed and amended (fix round 1) 2026-09-15*). **Nothing in this report is disputed.** Four findings became design changes rather than wording changes, and the plan's status line names all four so a reader who knows the first draft is told where to look. Line numbers below are the amended plan's.

### Blocking

- **B1 — resolved by design change.** `restore` and `restore_all` take a **seed allowlist** (`tolerate: &[String]`) instead of demanding an empty vault, and `unexpected_notes(vault, tolerate)` replaces `is_empty_vault`. `sync::restore_into` computes the allowlist from `note_paths(vault)` **at the moment it runs** — the vault is seconds old and nothing else has written to it, so that set is exactly what `scaffold::seed_writes` put there, with no second list to keep in step with it. A restored note at a seed's path overwrites it; anything the caller did not name is still a refusal, and the message names one. The fidelity-ledger row now states the rule the code enforces (all six of `ids::NOTE_FOLDERS`, minus the allowlist) rather than the narrower `tasks/`-only sentence. New tests: `a_restore_tolerates_exactly_the_seeds_the_wizard_just_wrote`, `a_restored_note_overwrites_the_seed_at_the_same_path`; `a_restore_refuses_a_folder_that_already_holds_notes` becomes `a_restore_refuses_a_note_it_did_not_put_there`.
- **B2 — resolved.** The plan stated one convention and wrote another; **C1's shape is now stated and written**. *Interfaces* contract 5 says plainly that nothing catches inside a `handle` — `_shared/http.ts`'s `fail` throws, `requireActiveEntitlement` rejects, and `index.ts`'s `asResponse` is what turns either into the reply, exactly as `telemetry/handler.ts` and `account/handler.ts` already do — and that **every handler test therefore awaits the rejection**. Both handler test files gain a named `refusal(req, deps)` helper (`handle(req, d).catch((e) => e as Response)`, C1's pattern at `telemetry/handler_test.ts:79`), and all six affected call sites use it.
- **B3 — resolved.** `a_restored_vault_ranks_the_same_day_as_the_one_it_came_from` now copies `state/calendar.md`, `state/events.md` and `state/events-seen.md` alongside `config/` and `profile/`, with a comment that quotes `golden-today-full.md:3` ("2.25h calendar") and `:5` ("Events: 3 in today's digest") as the reason and says these three are `rank` **inputs** a restore does not carry. The assertion is unchanged and unweakened.
- **B4 — resolved.** New hand-off **H12** (`site/index.html`, applied at Task 13), because that file is C1's. Precondition **P2** now says **four** copies of the sentence, not three, and names all four; Task 13 step 4 lists four places and the three tests that hold them together; the task's `git add` carries a comment saying `site/index.html` is the controller's and `engine/tests/site.rs` is C3's. The line reference is corrected to `engine/tests/site.rs:25-26`.
- **B5 — resolved.** New hand-off **H8c**: `app/src/main.rs`'s startup thread (`:169-177`) is deleted whole, comment included, with the reasoning that both functions leave with H6b and the thread has nothing else to do. It is named in Task 12's hand-off list, in the preamble's table, and in exit-gate item 11.
- **B6 — resolved by design change.** `SyncKey::index() -> IndexKey` derives a MAC key with `HKDF-SHA256(salt = "", ikm = the sync key)` expanded under the single label `knowlu/index`, and **`record_hash`, `note_ref` and `device_token` are all `HMAC-SHA256` under it**, each with a per-kind domain prefix (`record\n…`, `note\n…`, `device\n…`) so a crafted path cannot take a record's place in `sync_records_once`. `IndexKey` has a redacting `Debug`; the key is derived once per run and passed down, so `apply`'s whole-journal pass is one extraction rather than fifty thousand. Signatures changed through Tasks 2, 6, 7, 9 and 10 and every call site in the plan. Four new tests: `the_index_is_keyed_so_a_guessable_string_is_not_recoverable_from_its_row` (which asserts the bare digest is *not* what is stored, and that the AEAD key is not reused as a MAC key), `the_three_index_values_cannot_collide_across_kinds`, `the_index_key_never_appears_in_a_debug`, and the keyed half of `a_device_token_is_opaque_keyed_stable_and_not_the_hostname`. The migration's property 1, the `note_ref` column comment, the module header, the fidelity-ledger row, exit-gate item 2 and precondition P2 all now say it — P2 explicitly, because two of the switch screen's sentences were untrue without it. **M12 is resolved by the same change.**
- **B7 — resolved.** `assert!(sent.contains("authorization: Bearer jwt-not-a-secret"))`, lower case, with C2's own comment carried across so the next reader does not "fix" it back.
- **B8 — resolved by design change, and the product question is now Quinn's.** `Cursor` gains `key_fingerprint`; `build_push` compares it against the current key and, on a mismatch, **throws the whole cursor away and sets `reset: true`** on the batch, so the digest map cannot claim an unchanged note was pushed under a key nobody holds. `POST /sync-push` honours `reset` by deleting every row for the account **after validation and before storing** (`sync_db::resetCopy`; `sync_usage`'s triggers keep the counter honest), which also frees the orphaned bytes the ceiling was counting. `turn_off`'s doc says exactly what happens and the switch screen's off-sentence is rewritten to match. New precondition **P5** carries both answers and their costs; the plan is built to (a). New tests: `a_new_key_re_uploads_everything_and_says_so_once`, plus the two `reset` cases in `sync-push/handler_test.ts`.

### Important

- **I1 — Quinn-owned, and built to the recommendation.** New precondition **P6** asks which restore the 2026-09-14 call meant, with both features described and the recommendation *both links, on the same picker*. A fidelity-ledger row records that `HANDOFF.md` §4's row is about the local mirror and that the plan would otherwise be recording it as done. Task 11 gains **step 5**: the `#pick-restore-backup` button, its dialog, and the `console.js` handler that finally gives `onboarding::restore_vault` a caller. `the_picker_offers_both_restores_and_says_what_each_one_needs` pins both. A *What is NOT in this plan* entry spells out what to delete if Quinn rules cloud-only.
- **I2 — resolved.** All three whole-tree runs now use C2's full flag set; a Global Constraints clause names **both** command lines and which is which (C3's own files need only `--allow-read`; the whole tree needs C2's, because C3 merges after it). H9 is rewritten to say the thing to verify is the flags, not the paths, and carries the `npm:`/`jsr:` rule for edge-function imports as a Global Constraint too.
- **I3 — resolved by design change.** `sync_db.ts` gains `READ_LAG_SECONDS = 10` and `readRecords`/`readNotes` take a `now` and filter `received_at` / `updated_at` behind it, with the commit-order hazard spelled out where the constant is defined. `sync-pull/handler.ts` takes one clock through `deps.now()` so both reads share a window. New test `the_clock_the_read_lag_uses_is_the_handlers_and_it_reaches_the_reader`; new exit-gate item 12a; Task 5's smoke step now observes **both** pulls (empty, then the row) and says an implementer who sees only the second has not tested the lag.
- **I4 — resolved.** `run_lines_with` orders the three checks `load` → `load_key` → `resolve`, with a doc paragraph saying why the order is the message and that it also saves a Credential Manager read twice a day on every machine that never turns sync on. The hedged test becomes `assert_eq!(lines, vec!["sync (skipped: not turned on)"])`, and exit-gate item 3 says **exactly**.
- **I5 — resolved.** `restore_into` returns `Restored { notes, records, empty }`; `create_vault_in`'s restore block `match`es it, **keeps the vault on `empty`** and rolls back only on `Err` — with the reasoning in the code comment that a wrong recovery code is a different thing and is still a failure. The finish panel carries `RESTORE_EMPTY` ("your cloud copy is empty — the next sync from your other machine will fill this one"), pinned by `the_finish_panel_says_when_the_cloud_copy_was_empty`, and `finish_or_roll_back`'s envelope gains one `restored` key on this route only. New test `an_empty_cloud_copy_is_not_a_failure_of_making_a_vault`.
- **I6 — Quinn-owned, and built to the recommendation.** **P3** is rewritten: the retention number is now presented as a decision about *history*, with `journal::human_set` named and the failure described (an agent re-setting a field decided fourteen months ago, on a restored machine, quietly). Two shapes are offered and the second is recommended and built: `sync_records` gains a **`keep`** boolean the device sets for a record a human wrote (`provenance::is_agent` on the actor, `op` in `set`/`create`), and `sync_prune` deletes only `and not keep`. The one bit this leaks is named in the migration comment rather than hidden. A fidelity-ledger row, a migration test (`retention never deletes a record a human wrote`), a `build_push` test (`a_record_a_human_wrote_is_marked_to_be_kept_and_an_agents_is_not`) and a Task 14 production line carry it.
- **I7 — resolved by design change.** Two new guards in `engine/src/sync.rs`, each with one implementation: `is_note_path(vault, rel)` (inside the vault, under `ids::NOTE_FOLDERS`, ends `.md`) and `record_is_well_formed(record)` (`op` in `OPS`, `via` in `VIAS`, a parseable `ts`, an `actor`, a `device`, an `id` matching `ids::ID_RE` when present). Every pulled record passes both before `ledger.append`; a `move`'s destination passes `is_note_path` before `write::move_note` sees it; the pulled-note branch and both restore paths use the same function. `ApplyReport` gains `refused`. New tests `a_pulled_record_that_is_not_a_record_never_reaches_the_ledger` (six malformed shapes, plus the good one still landing) and `a_pulled_delete_or_move_may_not_escape_the_vault_either`; new exit-gate item 6a.
- **I8 — resolved.** Hand-off **H1a** is applied **at Task 2**, as its own commit, under the rule now stated in the *Controller hand-offs* preamble. Task 2's steps 2, 5 and 8 say so explicitly and the task is not green until it has landed; Task 6's run-and-fail step says what a different error message means.
- **I9 — resolved.** The hand-off preamble states the controller's rule once — every hand-off lands on the branch, at the task that first needs it, as that task's own commit, applied verbatim and reviewed with the task; no task may be reported green with a hand-off it names unapplied — followed by a table of which hand-off lands at which task. `H1`, `H6`, `H7` and `H8` are split into lettered parts because their halves are needed at different tasks. Task 12's wording is rewritten: its four are named as **the one non-buildable intermediate state in the plan**, with no checkpoint implied and no round trip to wait for.
- **I10 — resolved.** One `materialise(vault, page)` holds the shared body; `restore` is the allowlist check plus `materialise`; `restore_more` is deleted and `restore_all` calls `materialise` directly for the second and later pages.
- **I11 — resolved.** The *Interfaces* line reads `checkRecord(raw, device, accountId)` and `checkNote(raw, device, accountId)`, and says each returns the row its table takes or throws a `Response`.

### Minor

| # | Outcome |
|---|---|
| M1 | **Fixed** — `use std::path::PathBuf;` in `sync_contract.rs`; `Path` was unused. |
| M2 | **Fixed** — Task 2's step 5 now says fifteen cases (twelve, plus the three the keyed index adds); Task 3 adds a sixteenth. |
| M3 | **Fixed** — Task 7 step 1 says twelve; the Files line says twelve. |
| M4 | **Fixed** — `sync_usage` joins H5's purge list, and the test now **derives** the expected table list from the migration (`create table public.…`) rather than hard-coding it, so a fourth table cannot be forgotten. |
| M5 | **Fixed** — fourteen `pub mod` lines. |
| M6 | **Fixed** — the redundant `check_api_base` is gone, with a comment saying `cloud_config` already does it *and* refuses a foreign host. |
| M7 | **Fixed** — the `deno eval` one-liner becomes a throwaway `scripts/tmp-sync-vector.ts` run with `deno run` and deleted in step 8, with the PowerShell escaping reason stated. |
| M8 | **Fixed** — `sync_is_off_until_a_key_exists` now drives `sync::is_on` against a real `ConsoleState` across off → on → off, which is the property the privacy policy names. |
| M9 | **Deferred, half-answered** — `is_note_path` accepts any depth (the write side is now explicit) and `create_dir_all` is needed for a pulled `courses/` path on a vault that has none; making `note_paths` *recursive* is deferred because `ids::scan_notes`, `backup::BACKUP_FOLDERS` and every engine pass treat the folders as flat, so a nested note would be pushed and then invisible to everything else. In the plan's *Deferred minors*. |
| M10 | **Fixed** — `Totals` and `run_lines_with` are written in **Task 8**; Task 10 quotes the signature only and says it writes nothing in the engine. `engine/src/sync.rs` leaves Task 10's Files list and its `git add`. |
| M11 | **Deferred** — in *Deferred minors* and in Task 14's production list, with the measured shape of the cost (≈100 ms of HMAC for 50,000 records, twice a day). |
| M12 | **Fixed** — by B6's change: `device_token` is keyed like the other two. |
| M13 | **Deferred** — in *Deferred minors*, with the reason (the message it improves is already actionable, and a check symbol changes the code's length, which is on a screen Quinn reads under P2) and the trigger to revisit. |
| M14 | **Fixed** — "the Export `<li>` in the rights list (`site/privacy.html:96`, not a `<p>`)". |
| M15 | **Fixed** — `turn_on`, `turn_off` and `stage_key` are in Task 10's *Produces*, with their signatures. |
| M16 | **Fixed** — `:52-61`. |
| M17 | **Fixed** — `both ciphertext caps match their columns` reads the migration and asserts **both** 24576 and 196608 against it. |
| M18 | **Fixed** — one gate, not two: `app/src/sync.rs` loses `#![cfg(windows)]` and `lib.rs` declares `pub mod sync;` ungated, following `onboarding.rs`'s precedent (it calls the gated `credentials` module and is itself ungated). `ConsoleState`'s new field is consistent with that. |
| M19 | **Fixed** — the `§5.5 Up` ledger row records the `/sync/push` → `/sync-push` rename and why (Supabase routes `/functions/v1/<function-name>`; C2's review Critical 1 ruled the same way). |
| M20 | **No action** — a confirmation, not a defect; kept in *Deferred minors* so nobody re-derives the arithmetic. |

### One defect found while fixing, not in the report

Task 5's `the caller's limit is clamped, never trusted` asserted `assertEquals(low, 1)` for `?limit=0`, but the handler reads `Number.isSafeInteger(asked) && asked > 0 ? Math.min(asked, MAX_PAGE) : MAX_PAGE` — zero is not `> 0`, so the answer is `MAX_PAGE`. The assertion is corrected. Worth a line because it is the shape of mistake a test is supposed to catch and this one would have shipped red.

## Re-review of fix round 1 (2026-09-15)

Scope: the eight Blocking and eleven Important findings only, verdicted against the amended plan
(6,352 lines) at the lines each resolution names, plus an inspection of the four design changes for
breakage they introduce. Minors were not re-checked; nothing outside the changed text was re-reviewed.

**19 of 19 ADDRESSED.** One new **Important** consequence of the `reset` design, and five Minors in
the changed text.

### Blocking

- **B1 — ADDRESSED.** `unexpected_notes(vault, tolerate)` (`:4770`) replaces the emptiness test;
  `restore(vault, page, tolerate)` (`:4787-4798`) refuses only strangers; `restore_into` computes the
  allowlist as `note_paths(vault)` at the moment it runs (`:5252`), and `create_vault_in`'s block
  (`:640-668`) is explicitly placed after `scaffold::create_vault` with the reason written down. The
  one caller now succeeds. The guard is, by construction, vacuous at that call site — the plan says so
  and bounds it to one caller (`:6263`) — which is the right trade for a vault seconds old. The
  fidelity-ledger row (`:81`) now states the six-folder rule the code enforces. See **N4**.
- **B2 — ADDRESSED.** *Interfaces* contract 5 now states C1's shape (nothing catches inside `handle`),
  and both test files gain `function refusal(req, d)` (`:2161`, `:2586`) wrapping
  `handle(req, d).catch((e) => e as Response)`. All six affected call sites use it (`:2208`, `:2209`,
  `:2238`, `:2266`, `:2689`, and the 400-naming case). Matches `telemetry/handler_test.ts:79`.
- **B3 — ADDRESSED.** `:4626-4638` copies `state/calendar.md`, `state/events.md` and
  `state/events-seen.md` beside `config/` and `profile/`, quoting `golden-today-full.md:3` and `:5` as
  the reason and calling the three `rank` **inputs**. The assertion is unchanged.
- **B4 — ADDRESSED.** Hand-off **H12** (`:720-729`) covers `site/index.html`, applied at Task 13;
  P2 (`:61`) names four copies; the ownership line (`:52`) says `site/index.html` is C1's and
  `engine/tests/site.rs` is C3's; Task 13 (`:6173`, `:6187-6188`) lists four places and the three tests.
  See **N5**.
- **B5 — ADDRESSED.** Hand-off **H8c** (`:674`) deletes `app/src/main.rs:169-177` whole, comment
  included; it is in the hand-off table at Task 12 (`:277`), in Task 12's list, and in exit-gate item
  11 (`:6306`), which now also names `refresh_head` and `git_sha` in the scan.
- **B6 — ADDRESSED.** `SyncKey::index()` (`:1693-1698`) is `HKDF-SHA256(salt = "", ikm = sync key)`
  expanded under `b"knowlu/index"` into an `hmac::HMAC_SHA256` key; `record_hash`, `note_ref` and
  `device_token` are `mac_hex` under it with `record\n` / `note\n` / `device\n` domain prefixes
  (`:1702-1742`). The ring API used is real (`hkdf::Salt::new(...).extract()`, `Prk::expand(&[info],
  hmac::Algorithm)`, `hmac::Key::from(okm)`). `IndexKey` redacts in `Debug` (`:1686-1690`), is derived
  once per run and threaded — `apply(vault, &index, …)` at `:3992` and `:4403`, `build_push` at
  `:3321`, `restore_all` at `:4867`, every test at `:1258`–`:1328` and `:3062`, `:3512`. **The server
  gets nothing**: only MAC outputs travel; `K_index` is never a field, never a header, never in
  `push`'s body (`:3434-3438`). Column types, `sync_records_once`, the cursor and cross-device dedupe
  are all unchanged because the key is the *account's*, not the device's. The `keep`/`ceiling`/purge
  paths are untouched by it. The new test at `:1298`/`:1305` asserts both that the bare digest is not
  what is stored and that the AEAD key is not reused as a MAC key.
- **B7 — ADDRESSED.** `:3002` is `assert!(sent.contains("authorization: Bearer jwt-not-a-secret"));`
  with C2's comment carried across.
- **B8 — ADDRESSED for the case it was raised about**, and it introduces **N1**. `Cursor.key_fingerprint`
  (`:3219`, persisted `:3254`/`:3264`); `build_push` sets `rekeyed` and substitutes `&Cursor::default()`
  (`:3327-3334`), so the digest map, `pushed_through`, `boundary` **and both pull cursors** go together;
  `PushBatch.reset` rides even an empty batch (`push`'s guard at `:3431`); the handler validates first,
  then `resetCopy`, then measures the ceiling (`:2342`, `:2353`, `:2355`), with a test pinning the order
  (`:2241-2256`) and one pinning that `reset` is never sent unasked (`:2262-2266`). `sync_usage`'s
  triggers free the orphaned bytes (`:2485-2489`). No orphaned copy and no silently incomplete one on
  the re-keying device.

### Important

- **I1 — ADDRESSED.** New precondition **P6** (`:65`) puts the question to Quinn with both features
  described and *both links* recommended; a fidelity-ledger row (`:84`) records that `HANDOFF.md` §4's
  row is about the local mirror. Task 11 step 5 (`:5749-5780`) adds `#pick-restore-backup`, its dialog
  and the handler that finally calls `onboarding::restore_vault`; `:5729` pins both buttons; the
  cloud-only fallback is written down.
- **I2 — ADDRESSED.** C2's full flag set at `:32` (Global Constraints, naming both command lines),
  `:737` (H9), `:2533`, `:2819`, `:6214`. H9 now says the thing to verify is the flags.
- **I3 — ADDRESSED.** `READ_LAG_SECONDS = 10` (`:2401`) with the commit-order hazard written where the
  constant is; `settled(now)` (`:2441-2443`); `readRecords`/`readNotes` filter `received_at` /
  `updated_at` and take a `now` (`:2445-2478`); the handler passes one clock through `deps.now()`
  (`:2775`) so both reads share a window; exit-gate item 12a (`:6308`); Task 5's smoke observes both
  pulls (`:2828`). See **N3** for the residual.
- **I4 — ADDRESSED.** `run_lines_with` is `load` (`:4379`) → `load_key` (`:4380`) → `resolve` (`:4384`),
  with the reasoning in the doc; the hedged test becomes an exact equality and exit-gate item 3 says
  *exactly*.
- **I5 — ADDRESSED.** `Restored { notes, records, empty }` (`:5230-5235`), `restore_into` returns
  `empty` rather than `Err` (`:5259`), `create_vault_in` matches and keeps the vault on `Ok`
  (`:653-666`), rolls back only on `Err`; `RESTORE_EMPTY` on the finish panel (`:5709-5711`).
- **I6 — ADDRESSED.** P3 rewritten as a decision about history; `sync_records.keep` (`:946`) set by
  `build_push` from `provenance::is_agent` and `op ∈ {set, create}` (`:3356-3365`), validated and
  carried by `checkRecord` (`:1948`, `:1954`, `:1961`), and `sync_prune` deletes `and not keep`
  (`:1068`), pinned by a migration test (`:871-872`). The one cleartext bit is named in the migration
  comment rather than hidden, which is the right handling.
- **I7 — ADDRESSED.** `is_note_path(vault, rel)` (`:3926`) and `record_is_well_formed(record)`
  (`:3942`) each have one implementation and are used on every path: pulled records before
  `ledger.append` (`:4023`), the record's own `path` (`:4029`), pulled notes (`:4054`), a `move`'s
  destination before `write::move_note` sees it (`:4093`), and both restore paths (`:4806`, `:4819`,
  `:4823`). `ApplyReport.refused`, two new tests, exit-gate item 6a.
- **I8 — ADDRESSED.** **H1a** is applied at Task 2 as its own commit (`:271`, `:283`, `:1139`), and
  Tasks 2, 6 and 9 say they are not green until it lands (`:1749`, `:1782`, `:3171`, `:3453`).
- **I9 — ADDRESSED.** The hand-off preamble (`:264-282`) states the rule once — every hand-off lands on
  the branch, at the task that first needs it, as its own commit, and no task is green with one of its
  hand-offs unapplied — followed by the task→hand-off table. `H1`, `H6`, `H7`, `H8` are split into
  lettered parts. Task 12's "wait for the controller" wording is gone.
- **I10 — ADDRESSED.** `materialise(vault, page)` (`:4802`) holds the shared body; `restore` is the
  allowlist check plus `materialise` (`:4798`); `restore_more` is deleted and `restore_all` calls
  `materialise` directly (`:4856`).
- **I11 — ADDRESSED.** `:1804` reads `checkRecord(raw, device, accountId)` / `checkNote(raw, device,
  accountId)`, "three arguments each", and says each returns the row or throws a `Response`.

### New breakage

**N1 — Important. `plan.md:3327-3334`, `:2353`, `:4379-4403` — after one device re-keys, every *other*
device of the account is permanently broken and silently pollutes the new copy.**
`reset` empties `sync_records`/`sync_notes` for the whole account, but the second device's key is
unchanged, so its own `key_fingerprint` still matches its cursor and it never resets. It then (a) pulls
the new device's rows — `seq` is a table-wide identity, so the new rows sit above its cursor — and
cannot open any of them, giving `SyncError::Crypto("every row")` and the line `sync pull: the copy
could not be opened with this key` **every slot, for ever**, with `SyncStatus.last_error` permanently
set and no instruction the student can act on; and (b) keeps pushing rows sealed under the old key,
which nobody can ever open and which count against the ceiling. P5's text covers only "what happens to
the copy already in the account" and says nothing about the account's other devices.
*Recommended resolution:* the key's fingerprint is not a secret (it is Credential Manager's `UserName`
and is shown on the restore screen), so have `/sync-push` record it per account and both endpoints
return it; a device whose fingerprint differs reports one named, actionable line —
`sync (skipped: this machine's key is not the account's; restore with your recovery code)` — and
pushes nothing. That is a column, a field on two replies and one branch in `run_lines_with`. At
minimum, add the case to P5's text so Quinn is deciding the whole question, and record it in Task 14's
production list beside "a second device has never been run".

**N2 — Minor. `plan.md:4396-4412` — the re-key slot itself reports a failure.**
`run_lines_with` pulls before `build_push` detects the fingerprint mismatch, so on the first slot after
off→on the pull asks from the *old* cursor, gets rows sealed under the old key, and pushes
`the copy could not be opened with this key` into `totals.errors` — which makes `RunOutcome.ok` false
and paints the console's sync line amber on the slot the re-key actually succeeds. Exit code is still
0, so the tray and the scheduler are unaffected. *Fix:* compare the fingerprint before the pull and
skip it (with a named line) when it has changed, or suppress that one error on a `rekeyed` run.

**N3 — Minor. `plan.md:2393-2401`, `:2441-2443` — the lag is measured from `now()`, which is the
transaction's start, not its commit.**
`received_at`/`updated_at` default to `now()` (transaction start), so the guard bounds *insert-to-commit*
time, not commit visibility: a transaction that stays open longer than `READ_LAG_SECONDS` after its
insert can still be stranded. For a PostgREST single-statement upsert that is milliseconds, so the
guard closes the race in practice — but the comment's "a transaction that has not committed in ten
seconds has failed" is an assumption rather than a guarantee. *Fix:* one clause naming what enforces
it (Supabase's `statement_timeout` / `idle_in_transaction_session_timeout` on the REST role), or state
the bound as "longer than the lag after its insert".

**N4 — Minor. `plan.md:4784` — `restore`'s doc comment still says "Bounded three ways: the destination
must hold no note at all", directly contradicting the allowlist paragraph above it.** Leftover from the
first draft; rewrite it to the rule `unexpected_notes` enforces.

**N5 — Minor. `plan.md:722` versus `:6187`** — H12 cites `engine/tests/site.rs:25-26` for the two-page
assertion; Task 13 cites `:26-27`. The file's two `assert!`s are at `:26-27`.

**N6 — Minor. `plan.md:4479`** — Task 8's smoke step still explains its expected
`sync pulled 0 record(s)` as "because every row that comes back is this device's own and is recognised
by hash". With `READ_LAG_SECONDS` a pull run seconds after a push returns nothing regardless, so the
observation no longer proves the property it names. Task 5's smoke step was updated for the lag; this
one was not.

### Out-of-scope observations

- `keep` adds one cleartext bit per record ("a human wrote this"). Disclosed in the migration comment
  and in P3, which is the right handling; noted only so the privacy screen's reader knows it exists.
- `device_token` is now keyed, so it changes for every device on a re-key. Metadata only, no effect.
- The `restore`/`materialise` split, the `refusal` helper and the hand-off lettering all read as this
  repository's own style; nothing in the changed text duplicates or hand-waves.

**Verdict: findings remain open — N1 (Important).** All nineteen original findings are addressed and
the four design changes are correct as far as they go; one Important consequence of the `reset` design
(the account's other devices) and five Minors in the changed text are new and should be answered before
execution — N1 in the plan and in P5, the rest in wording.

## Fix round 2 — resolutions (2026-09-15)

Applied to `docs/plans/2026-09-14-c3-sync-plan.md` (now **6,957 lines**, 14 tasks). **Nothing in the re-review is disputed.** N1 became a design change; N2–N6 are wording, ordering and one corrected expectation.

### N1 — resolved by design change: the account carries a key generation

**The design.** A copy is only a copy to a device that can open it, so the account now records **which key generation its copy is sealed under** — `public.sync_generation(account_id, key_fingerprint, set_at)`, one row per account, holding the same non-secret eight hex characters Credential Manager already stores as that credential's `UserName`. `POST /sync-push` claims the generation on an account's first push and on a push carrying `reset: true`; both endpoints compare the fingerprint the device sends (`key_fingerprint` in the push body, `?key_fingerprint=` on the pull) and answer **409, storing nothing and reading nothing**, to a device whose key is neither the current generation nor a claim. The 409's sentence names the account's fingerprint, so the device can show the student which key it needs without a second round trip. On the device that 409 is `SyncError::StaleKey` — a **named, non-failing skip**: `sync (skipped: this machine's key is not the account's)`, exit 0, `totals.blocked` set and `totals.errors` left empty so no retry backoff starts for something retrying cannot fix — **plus one `info/` item** (`STALE_KEY_INFO`, opened once via `list_info`, closed by the first sync that succeeds) in the Good-to-know deck, and an amber line on the console's sync row. Nothing is pushed and nothing is pulled while it is stale, which is the half that stops the silent pollution. The action both the item and the line point at is a new console command, **`set_sync_key`** → `sync::adopt_key`, which writes the account's key on this machine **and resets the cursor to `Cursor::default()` carrying the new fingerprint** — because writing the key alone would leave the cursor naming the old one, and the very next `build_push` would then see a mismatch, send `reset: true`, and destroy the copy the student was trying to join. Adopting a generation and claiming one are now two different acts with two different functions.

**What it costs if it is wrong.** If the 409 is over-eager — a fingerprint mismatch that is not really a re-key — a healthy device stops syncing until someone re-enters a code, which is visible, reversible and loses nothing: the vault is untouched and the copy is untouched. If it is under-eager the failure is the one this fixes, so the bias is deliberately toward refusing. The whole of it is one table, one field on two requests, one field on two replies, one branch in `run_lines_with` and one settings row; reversing it (P5 answer (b) — no re-key at all) deletes all of it together.

**Corrected in fix round 3.** This paragraph originally named "two devices re-keying at the same moment" as the only residual, and said it degraded into the handled case. That was **wrong about the mechanism and too narrow about the shape**: the claim as written was a read-then-upsert, so *any* two devices with different keys could both pass the gate and both store — including two devices pushing for the first time, which is not a re-key at all — leaving a copy under two generations with neither device told. Round 3 replaces the claim with a compare-and-set and adds a test per shape; the residual that remains is stated there.

**Carried through:** the migration (`public.sync_generation`, RLS, its select policy, and the migration tests now expecting four tables and the fingerprint's check constraint); H5's purge list (**four** names); `_shared/sync_rows.ts` (`isFingerprint`, and its test case); `_shared/sync_db.ts` (`generation`, `claimGeneration`, and a note on why `resetCopy` does **not** delete the generation row); `sync-push`'s handler, `Deps`, `index.ts` and five new contract cases (409-stores-nothing, first-push-claims, reset-claims-in-order, own-key-claims-nothing, no-usable-fingerprint); `sync-pull`'s handler, `Deps`, `index.ts` and two new cases (409-reads-nothing plus `null` is not a conflict, and the 400); the engine's `SyncError::StaleKey`, `stale_key(&CloudError)`, `PushBatch.key_fingerprint`, the pull's query parameter and its 409 mapping; `Totals.blocked`, `STALE_KEY_INFO`, `open_stale_key_notice` and the close-on-success in `run_lines_with`; two new engine cases; `app/src/sync.rs`'s `adopt_key` and `set_sync_key` with two new app cases; `SyncStatus.blocked`, the screen's `SYNC_BLOCKED` copy and adopt row, the sync line's new branch, and three new assertions in `static_assets.rs`; the command recount (**47 / 28 / 64**, up from 46 / 28 / 63) in H8a and H11; a fidelity-ledger row; precondition **P5**'s second half; exit-gate item **11a**; and a Task 14 production line saying the sequence has never been run by two real machines.

### N2–N6

- **N2 — resolved.** `run_lines_with` now computes `rekeyed` **before** the pull and skips the pull by name (`sync: this machine has a new key; nothing is pulled and the whole vault goes up again`), so the re-key slot pushes, stays green, and never opens rows sealed under the old key. The predicate is extracted as `sync::is_rekeyed(&Cursor, &SyncKey)` so `build_push` and `run_lines_with` cannot disagree, and `a_re_keyed_slot_pulls_nothing_and_is_not_a_failure` pins all three of its answers.
- **N3 — resolved as "name the assumption", with the enforcement named too.** No cheaper stronger form was adopted: a `clock_timestamp()` column would move the write from one statement to a trigger and still not give commit visibility, and `xmin`/`pg_current_xact_id` visibility is not reachable through PostgREST. The comment on `READ_LAG_SECONDS` now says plainly that `now()` is the transaction's **start**, that the guard therefore bounds insert-to-commit rather than commit visibility, and what makes that safe here rather than lucky: every write is a **single PostgREST upsert** (one statement, one implicit transaction, milliseconds) and Supabase's REST role runs under `statement_timeout` and `idle_in_transaction_session_timeout` far below ten seconds. The residual — a hand-run bulk write through `psql` during a pull — is named as a path this product does not have.
- **N4 — resolved.** `restore`'s doc comment's "Bounded three ways" now reads: no note the caller did not name in `tolerate` (`unexpected_notes` walking all six of `ids::NOTE_FOLDERS`, explicitly not a `tasks/`-only test), every path through `is_note_path`, and a tombstone writing nothing.
- **N5 — resolved.** Both citations now read `engine/tests/site.rs:26-27`, which is where the two `assert!`s are.
- **N6 — resolved.** Task 8's smoke step now asks for **three** observations, not two: push, an immediate pull that returns nothing *because the lag has not elapsed* (and therefore proves nothing on its own), and a third pull fifteen seconds later whose zero **is** the "a device does not apply its own writes twice" property. The report must carry all three lines; one of them alone proves neither thing.

### One defect found while fixing, not in the re-review

The `reset` ordering test's fake returned a constant `bytesUsed` of 999,999 while asserting `bytes_used === 4` — true only if the fake modelled the delete, which it did not. The fake now flips on `resetCopy`, so the assertion tests the ordering it claims to (`without the ordering this is a 413`) instead of passing by accident.

## Re-review of fix round 2 (2026-09-15)

Scope: N1–N6 only, verdicted against the amended plan (6,957 lines) at the lines the resolutions name.
Nothing else was re-read.

### N1 — ADDRESSED, with one new Important finding (the claim is not atomic)

Traced end to end, and the design does what it says:

- **The migration** — `public.sync_generation(account_id uuid primary key references accounts on
  delete cascade, key_fingerprint text not null check (~ '^[0-9a-f]{8}$'), set_at timestamptz default
  now())` at `:1021-1025`, with the comment naming `sync-push` as its only writer (`:1017-1020`), RLS
  on (`:1107`) and a read-your-own select policy (`:1117`). No default fingerprint — absence *is*
  "nothing stored yet", which is the right encoding. The migration test now asserts the sorted
  four-table list (`:889`) rather than a hard-coded pair, and `isFingerprint` has its own case
  (`:1870-1872`, including the upper-case rejection). H5's purge list carries four names (`:425-430`).
- **`sync-push`'s comparison** — `:2490-2516`: `current === null` claims; `current === fingerprint`
  is normal; a different key with `reset` empties then claims; a different key without `reset` is
  `json(409, { error: "… (the account's key is <current>)", key_fingerprint: current })` returned
  **before** `resetCopy`, `claimGeneration`, the ceiling read and both saves — so it stores nothing.
  The fingerprint is in the sentence as well as the field, which is right, because `CloudError` keeps
  only `error`. A missing or malformed `key_fingerprint` is a thrown 400 (`:2471`).
- **`sync-pull`'s comparison** — `:3036-3042`: 400 without a usable fingerprint, then the same 409
  shape, returned before either read, so it reads nothing. `null` (nothing stored) is explicitly not
  a conflict.
- **The engine's 409 → skip** — `stale_key(&CloudError)` maps only `Status { code: 409 }`
  (`:3733-3736`) and is asserted not to fire on a 402 (`:4640`). `pull` turns it into
  `SyncError::StaleKey` (`:4189-4191`); `run_lines_with`'s pull arm (`:4870-4878`) prints
  `sync (skipped: this machine's key is not the account's)`, sets `totals.blocked`, opens the notice
  and **`return (0, lines, totals)`** — so **nothing is pushed** on a stale pull. The push arm
  (`:4899-4912`) handles the same 409 for the fresh-install case. `totals.errors` is left empty in
  both, which is what keeps `RunOutcome.ok` true, the tray green and the slot out of retry backoff.
- **The info item** — `STALE_KEY_INFO = "sync:key-generation"` (`:4749`); `open_stale_key_notice`
  (`:4758-4776`) scans `list_info` for an item already carrying that `close_key` and opens at most
  one; `run_lines_with` closes it by key on any run with no `blocked` and no `errors`
  (`:4923-4924`), and `close_info` by key is a no-op when there are none. The test at `:4655-4666`
  drives open-once then close.
- **`adopt_key`** — `:5715-5721`: writes the key to `knowlu/<profile_id>/sync-key` **and** saves
  `Cursor { key_fingerprint: <new>, ..Default::default() }`, so `is_rekeyed` is false on the next run
  and the next push is **not** a `reset`. That is the whole point of separating adopting from
  claiming, and it is correct. `set_sync_key` is in the console handler list (`:619`) and the counts
  are recounted to 47 / 28 / 64.
- **The tests, the exit gate and the ledger** — `a_stale_key_is_a_named_skip_and_a_notice_and_never_an_error`
  (`:4629`), `a_re_keyed_slot_pulls_nothing_and_is_not_a_failure`,
  `adopting_a_key_writes_it_and_resets_the_cursor_so_the_next_push_is_not_a_reset`, five push cases
  and two pull cases; exit-gate item **11a** (`:6912`) names all of them; the fidelity row is `:83`
  and P5's second half is `:65`.

**The race question — the claim is a read-then-write, and it is not atomic.** `generation()` is a
`restSelect` (`:2666-2673`) and `claimGeneration()` is a `restUpsert(… , "account_id")`
(`:2676-2683`), i.e. last-writer-wins `merge-duplicates` — not `insert … on conflict do nothing`, not
`update … where key_fingerprint = <the value just read>`, and nothing returns a row the handler
checks. So two devices with **different** keys can both pass the gate and both store:

- *First-push race* — both devices' first ever push for one account, concurrent: both read
  `current = null`, both skip the 409, both claim (one silently overwriting the other) and **both
  store their rows**. The copy now holds rows under two generations, the table names one, neither
  device is told anything about that push, and a later restore under the winning key opens some rows
  and warns on the rest (`pulled_from_reply` only errors when *nothing* opens) — a **silently partial
  vault**, which is the harm class N1 exists to close.
- *Re-key against a normal push* — X pushes normally under `aaaa` while Y re-keys: Y's `resetCopy`
  can land between X's gate check and X's `saveRecords`, leaving X's rows orphaned under generation
  `bbbb`. X learns next slot (handled), but the orphaned bytes stay and count against the ceiling.
- *Two simultaneous re-keys* — the plan names this one at `:751` and it does degrade into the handled
  case for the losing device; the same orphaned-rows residue applies.

The plan's residual-risk paragraph (`:751`) covers only the third of these and asserts the race
"degrades into the handled case rather than into silence". For the first it degrades into silence.

*Recommended resolution — small, and the ordering it needs is already right* (the claim already runs
before `saveRecords`/`saveNotes`, so a failed claim can 409 before anything is stored): make the claim
a compare-and-set and 409 when it loses.
- First claim: insert with `Prefer: resolution=ignore-duplicates` (a second `restUpsert` shape, or one
  direct `rest.fetch` beside C1's helpers), then re-read `generation()` and return the same 409 if
  what is stored is not this device's fingerprint.
- Re-key claim: `restPatch("sync_generation", "account_id=eq.<id>&key_fingerprint=eq.<the value just
  read>", {…})` with `Prefer: return=representation`, and 409 if no row comes back — and do the
  `resetCopy` only after the claim wins, so a loser deletes nothing.
Add one handler case per shape (`two first pushes with different keys: exactly one claims, the other
is a 409 and stores nothing`), and replace `:751`'s sentence with the three races above.

### N2–N6

- **N2 — ADDRESSED.** `is_rekeyed(&Cursor, &SyncKey)` at `:3583`, used by `build_push` (`:3620`) and
  `run_lines_with` (`:4832`); `:4834-4836` skips the pull by name on a re-key, so the slot pushes,
  stays green and never opens old-key rows. Three assertions at `:4618-4620`.
- **N3 — ADDRESSED as "name the assumption".** `:2564-2570` says `now()` is the transaction's start,
  that the guard bounds insert-to-commit rather than commit visibility, and names what makes it safe
  (a single PostgREST upsert; the REST role's `statement_timeout` and
  `idle_in_transaction_session_timeout`), with the hand-run-`psql` residual named as a path this
  product does not have. The right call — the stronger forms are not reachable through PostgREST.
- **N4 — ADDRESSED.** `:5264-5266` now reads "no note its caller did not name in `tolerate`
  (`unexpected_notes` walks all six of `ids::NOTE_FOLDERS`, so this is not a `tasks/`-only test)".
- **N5 — ADDRESSED.** `:725` reads `engine/tests/site.rs:26-27`; both citations now agree with the file.
- **N6 — ADDRESSED.** `:4951-4956` asks for three observations — push, an immediate pull that proves
  nothing because the lag has not elapsed, and a third pull fifteen seconds later whose zero is the
  property — and requires all three lines in the report.

### New breakage

**One, Important — `plan.md:2666-2683` with `:2513-2516` and `:751`: the generation claim is a
read-then-upsert, so two devices with different keys can both claim and both store.** Full analysis,
the three races and the recommended compare-and-set are under N1 above. Nothing else in the fix-round-2
text is new breakage: the 409 paths store and read nothing, the skip is genuinely non-failing, the
notice opens once and closes on success, and `adopt_key`'s cursor reset closes the adopt-then-reset
trap it was written for.

**Verdict: findings remain open — the generation claim is not atomic (Important).** Everything else in
N1–N6 is addressed and correct; this one is a compare-and-set in `claimGeneration`, a 409 when it
loses, two handler cases and one corrected sentence at `plan.md:751`.

## Fix round 3 — resolutions (2026-09-15)

Applied to `docs/plans/2026-09-14-c3-sync-plan.md` (now **7,118 lines**). **Nothing is disputed.**

**Resolved by design change: the generation claim is a compare-and-set.** `claimGeneration` becomes `claimGeneration(rest, accountId, fingerprint, expected) -> Promise<string>` and answers **who holds the generation afterwards** rather than returning `void` (`plan.md:2745-2833`): for an account's first claim it is an INSERT with `Prefer: resolution=ignore-duplicates` followed by a re-read, so a second racer's row is a no-op and the re-read says who got there first; for every later claim it is a PATCH filtered on `key_fingerprint=eq.<the value this request read>` with `Prefer: return=representation`, so an empty array is a claim somebody else took in between. The handler (`plan.md:2570-2600`) now claims on **every** push — including one whose key already matches, which is how an ordinary push learns a re-key landed underneath it — compares the answer against its own fingerprint, and returns the same 409 (shared `refuse(owner)` helper) when they differ; **`resetCopy` and both `save*` calls run only after the claim wins**, so a device that lost a race neither empties nor stores. `Deps.claimGeneration` (`plan.md:2513-2517`) and `index.ts`'s wiring (`:2869`) carry the new signature, the five existing claim-aware tests return a fingerprint from their fakes, and the reset-ordering test's expectation becomes `["claim", "reset"]`.

**`_shared/db.ts` could not express either `Prefer`, so two one-off `fetch` calls were needed** — `restUpsert` hard-codes `resolution=merge-duplicates,return=minimal` and `restPatch` hard-codes `return=minimal`, neither takes a `Prefer`, and `db.ts` is C1's file. Both calls live inside `claimGeneration` in C3's own `_shared/sync_db.ts`, with a `syncHeaders` helper rebuilding the same four headers (`db.ts`'s `headers()` is private) and a local `okOrThrow` mirroring its `ok()`. *Interfaces* contract 6 records the constraint and the decision (`plan.md:248`), and Task 14's account-scoping scan is widened to read `rest.fetch(` URLs as well as the helper calls, to name the insert as its one deliberate exception (the id travels in the posted row, keyed by `on_conflict`), and to assert there are **exactly two** raw fetches (`plan.md:6990-7007`).

**The two new handler contract tests.**
- `on a first-push race exactly one device wins and the other stores nothing` — two machines of one account with two keys, both reading `generation` as `null`; the loser's `claimGeneration` answers with the winner's fingerprint, the push is a 409 naming it, `saveRecords` is never called, and the winner's identical request is an ordinary 200.
- `a re-key landing under a normal push refuses the pusher instead of orphaning its rows` — this device's key *is* the account's when it reads and another machine re-keys before it writes; the compare-and-set matches nothing, `claimGeneration` answers with the new owner, and the push is a 409 with `saveRecords`, `saveNotes` and `resetCopy` all uncalled.

**Also changed:** `plan.md:751`'s equivalent in this report (the round-2 "what it costs if it is wrong" paragraph) is corrected above — it named only the two-simultaneous-re-keys case and was wrong about the mechanism; exit-gate item 11a now states the compare-and-set, the claim-before-reset ordering and both test names; Task 4's *Produces* carries the new signature.

**The residual, stated rather than claimed away.** The claim and the row writes are separate PostgREST requests, so a re-key that lands in the milliseconds *between* a winning claim and its `saveRecords` still orphans that batch. PostgREST cannot put them in one transaction across tables. What the fix guarantees is that the window shrinks from the whole request to that gap, that nothing is ever emptied by a device that lost, and that the orphaned device is told on its **next** push rather than never — which is the handled case, with a named skip and a Good-to-know item. Closing it completely would need a server-side function taking the batch and the fingerprint together, which is a larger change than the risk warrants today and is recorded in Task 14's production list as the thing to reach for if two-device accounts ever become common.
