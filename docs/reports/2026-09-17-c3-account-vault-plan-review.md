# C3' account-vault plan — review (2026-09-17)

Plan: `docs/plans/2026-09-17-c3-account-vault-plan.md` (3,666 lines, 12 tasks, status WRITTEN, not started).
Binding text: `docs/specs/2026-09-09-knowlu-cloud-design.md` — the **Amendment 2026-09-17** (SIGNED, `9ff3b57`),
read whole first, then §5.5, §3.1, §4.4, §5.1, §9, §11a, §12 and §13; `VISION.md` commitment 2 **as amended**;
`CLAUDE.md`; `HANDOFF.md`. It supersedes `docs/plans/2026-09-14-c3-sync-plan.md`, whose review
(`docs/reports/2026-09-14-c3-sync-plan-review.md`) this plan inherits as rulings.

Reviewer: Claude (opus), independent of the writer. Three passes: a full sequential read of the plan; a
claim-by-claim check of every statement it makes about existing code
(`engine/src/{cloudmodel,journal,ledger,reconcile,write,approvals,ids,pystr,yaml,models,cli,ingest,lib,main,
runs,backup}.rs`, `engine/tests/{dependency_boundary,site,no_console,cloud_contract}.rs`,
`engine/tests/fixtures/{vault-s1,vault-full}/**`, `app/src/{state,commands,scheduler,account,main}.rs`,
`app/tests/static_assets.rs`, `app/static/console.js`, `site/{index,privacy}.html`,
`cloud/supabase/functions/_shared/{http,db,entitlement,config_toml_test}.ts`,
`cloud/supabase/functions/account/index.ts`, `cloud/supabase/config.toml`, `Cargo.lock`, `engine/Cargo.toml`);
and a pass over the paused branch `c3-sync` (worktree `.claude/worktrees/c3-sync`, head `934fefd`), read
READ-ONLY: `20260912000100_sync.sql`, `20260912000200_sync_usage_prune.sql`,
`cloud/supabase/migrations/migrations_test.ts`, `_shared/sync_rows{,_test}.ts`, `_shared/sync_envelope_test.ts`,
`_shared/sync_vectors.json`, `engine/src/{sync.rs,lib.rs}`, `engine/Cargo.toml`, `account/index.ts`.

## Verdict

**NEEDS A FIX ROUND.** The architecture is right and better than the plan it supersedes: two plaintext tables
under RLS, a `keep` bit the server computes instead of a bit the client asserts, a content hash the server
re-derives, one engine module, the pull before `rank`, a restore that is `/sync-pull` from zero and is reached
by signing in rather than by typing a code, and the four removals ruling 5 ordered (encryption, export, git,
the capability URLs). The migration is correct against what is on staging, the corpus pins are moved by the
right numbers, and the interfaces it consumes exist with the spellings it quotes. Two defects block execution:
one turns the branch red for three tasks, and one makes the plan's own conflict algorithm contradict the test
that is the whole point of the feature. Six more would produce wrong or fragile behaviour. None needs a
redesign — the tables, the cursor, the read lag, the gate and the task order all survive.

## Findings

### Critical

**C1. `plan.md:520-545` (H9a) — `app/src/state.rs` already has a `run_sync`; the hand-off adds a second one and says nothing is removed. The branch is red from Task 7 to Task 10.**
H9a's preamble is explicit: *"Additive, and nothing is removed here — Task 7 needs somewhere to put the last
sync's result, and Task 10 is what takes git out."* It then adds
`pub fn run_sync(cs: &ConsoleState) -> knowlu_engine::sync::SyncStatus` "beside `run_backup`". But
`app/src/state.rs:147` **already** declares `pub fn run_sync(cs: &ConsoleState) -> knowlu_engine::history::SyncOutcome`,
and H9b (`plan.md:573-600`, applied at Task 10) lists for removal only the `HistoryStatus` import, the three
fields, their initialisers and `refresh_history`/`refresh_head` — **it never names the old `run_sync`**. The
result is `error[E0428]: the name run_sync is defined multiple times` the moment H9a lands, and it is not
resolved by anything in Tasks 8, 9 or 10. H9a's own rewritten `commands::sync_inner` calls
`crate::state::run_sync(cs)` and reads `out.last_error`, while the surviving `state::quit_flush`
(`app/src/state.rs:239-241`) calls `run_sync(cs).status.last_error` — two incompatible shapes behind one name.
This breaks the plan's own rule twice: *"Nothing in this plan is ever red for more than that one commit"* and
*"no task in this plan may be reported green while a hand-off it names is unapplied"* (`plan.md:287-290`), and
it contradicts `plan.md:3259-3263`, which claims Task 10's four hand-offs are "the one non-buildable
intermediate state in this plan".
*Fix:* make H9a a **replacement**, not an addition — rename the git one (or delete it and its three callers in
the same hand-off) so exactly one `state::run_sync` exists at every commit. The cleanest shape: move
`sync_step`'s deletion and the two housekeeping `state::run_sync` calls from H8b into H9a's own commit, and have
H9a replace the body of `state::run_sync` and patch `quit_flush` to the engine call there rather than at H9b.
Whichever way, say in H9a which lines are **removed**, and re-run `cargo build -p knowlu` before Task 8 starts.

**C2. `plan.md:2625-2632` (Task 6 step 5) versus `plan.md:2517-2540` (its own load-bearing test) — the conflict path both writes the foreign value and files a card for it; the note cannot end up holding what the test asserts.**
Step 5 says: *"`resolution.apply` goes through `write::write_literals` under `ACTOR`; `resolution.supersede`
records are appended; a field with a supersede record whose folder is in `AMENDABLE_FOLDERS` and whose field is
in `AMENDABLE_FIELDS` becomes a `write::propose_amendment`"* — with nothing withholding the conflicting field
from `apply`. But `reconcile::resolve` (`engine/src/reconcile.rs:146-150`) inserts into `res.apply` **exactly
when the `local_records` side wins**, and under this plan's role reversal `local_records` are the *foreign*
records. So on the case the feature exists for — both desktops moved one field, the other desktop later — the
foreign value lands in `apply`, `write_literals` writes it to the note, **and** a card is filed. The plan's own
test `a_field_both_desktops_moved_becomes_one_amend_card_and_not_a_silent_merge` asserts the opposite:
`importance == 4` (this device's value) with a card carrying `from: 4` / `to: 5`. With the algorithm as
written the note holds 5, the card's `from` no longer matches the note, and `approvals::validate_amendment`
(`engine/src/approvals.rs:339`) — which compares `from` against the note before applying — would refuse it.
That is the silent merge VISION and §5.5 forbid, with an unapplyable card on top.
*Fix:* state the rule in step 5 and prove it in the test: **a field that produced a supersede record AND is
card-eligible is removed from `apply` before `write_literals` sees it** — the card is the write. Fields with no
supersede record apply cleanly; fields with a supersede record that no card could ever apply (outside
`AMENDABLE_FOLDERS`/`AMENDABLE_FIELDS`) keep reconcile's rule, apply, and emit the named warning the ledger row
already promises. Add one assertion that `report.applied` does not count a carded field.

### Important

**I1. `plan.md:1325-1332`, `:1146`, `:3618` — `sha2` is NOT in the engine's dependency graph, and `ring` is. The plan's own argument for `sha2` is the argument for `ring`.**
The claim is *"already resolved in this workspace's `Cargo.lock`, so this is one direct edge and no new crate
to audit and no new version to resolve"*. `Cargo.lock` on `main` resolves `sha2 0.10.9` — but its only
dependents are `knowlu` (the app), `tauri-codegen` and `wry`. `knowlu-engine`'s own `dependencies` array
(`Cargo.lock:2358-2372`) has no `sha2`. Adding it pulls `sha2`, `digest 0.10.7`, `cpufeatures 0.2.17`,
`block-buffer`, `crypto-common`, `generic-array` and `typenum` into the **engine's** graph, next to the
engine's existing `sha1 0.11.0`, which resolves `digest 0.11.3` and `cpufeatures 0.3.1` — two majors of two
crates compiled into a binary with a 6 MiB CI size gate. Meanwhile `ring` is already linked into the engine
through `ureq` to `rustls`; the branch's own manifest comment
(`.claude/worktrees/c3-sync/engine/Cargo.toml:36-38`) makes exactly that argument, and `ring::digest::SHA256`
is the same primitive. The plan's replacement test then forbids `ring = ` in the manifest outright
(`plan.md:1146`), foreclosing the zero-cost option and pinning the more expensive one.
*Fix:* either use `ring::digest::SHA256` and keep `ring = "0.17"` in `engine/Cargo.toml` with a comment saying
the envelope is gone and the hash is what remains (the forbidden list then keeps `aes-gcm`, `chacha20`,
`hkdf`, `base64` and drops `ring`), or keep `sha2` but pick the major that shares `sha1`'s `digest`, and in
either case replace the "no new crate to audit" sentence with what the lockfile actually shows.

**I2. `plan.md:2904-2917` — the test that pins the engine's grace to the app's reads the wrong file for one of its three assertions and fails as written.**
`the_engines_grace_is_the_apps_grace_and_the_path_is_the_apps_path` reads `app/src/account.rs` and asserts it
contains `base.join("knowlu")`. It does not: `from_secs(72 * 60 * 60)` is `app/src/account.rs:479` and
`data_dir.join("entitlement.json")` is `:485`, but `base.join("knowlu")` is **`app/src/state.rs:190`**, inside
`app_data_root_in`. `entitle.rs`'s own doc comment (`plan.md:2995`) names `state.rs` correctly; only the test
is pointed at the wrong file. As written the task's TDD loop never goes green.
*Fix:* read both files in the test — `account.rs` for the grace and the cache filename, `state.rs` for
`base.join("knowlu")` — and say in the assertion message which file moved.

**I3. `plan.md:410-455` (H4b) versus `plan.md:3038-3042` (Task 8 step 4) — the hand-off and the task print different lines, and the function the task names does not exist.**
H4b, which the controller applies **verbatim** (`plan.md:283-286`), prints `println!("{line}")`, and
`entitle::gate` returns `"skipped: no entitlement"` — so a gated `coursework` prints a bare
`skipped: no entitlement` with no command name. Task 8 step 4 asserts something else: *"hand-off H4b prints
`format!("{} ({})", name_of(&cli.command), line)` — `sync (skipped: no entitlement)`"*. `name_of` is defined
nowhere in the plan, in `engine/src/main.rs`, or in the engine. Exit-gate item 11 and the `CLAUDE.md` text in
Task 12 step 6 both describe the composed line, so the verbatim hand-off is the half that is wrong — but no
test anywhere asserts the composed line, so nothing would catch it.
*Fix:* put the composition in H4b itself with `name_of` written out (a `match` over `Command` returning the
subcommand's own word), and add one case to `engine/tests/entitlement_gate.rs` or `sync_contract.rs` that
asserts the process prints `sync (skipped: no entitlement)` — otherwise the one student-visible artefact of
ruling 3 is untested.

**I4. `plan.md:35`, `:2631-2633`, `:3614` — the plan claims exactly one exception to "every note write goes through `write`", and then takes two.**
Global Constraints: *"There is exactly one recorded exception in this plan, argued in the fidelity ledger and
again at Task 9: **restore materialises bytes**…"*. But Task 6 step 6 has `apply` write a pulled note for a
path this device has never seen with `pystr::write_text`, and exit-gate item 7 concedes it — *"no second code
path that writes a note file except **the two** in `sync.rs` that say so in their own doc comments"*. The
fidelity ledger's §5.5-Second-device row argues only the restore. The argument for the second one is the same
and is good (the foreign `create` record comes down verbatim in the same batch, so `write::create` would
fabricate a duplicate stamped with this device and today's `ts`), but it is not written down, and the bound is
weaker: `apply` has no allowlist, only `is_note_path`.
*Fix:* add a ledger row for the `apply` case with that argument, say "two recorded exceptions" in Global
Constraints, and state the bound `apply` actually enforces (`is_note_path` plus "never overwrites a note this
device already has").

**I5. `plan.md:2612-2633`, `:2795-2812`, `:3196-3210` — the three hardest functions in the plan are prose outlines, not code.**
Every other step carries real code. `sync::apply` (the pull applier: the dedupe set, the verbatim ledger append
by the record's own UTC day, the role-reversed `reconcile::resolve` call, the supersede records, the card and
its `find_pending_amendment` guard, the tombstone through `write::delete`, the move-destination check) is six
numbered sentences. `sync::run_lines_with` — where "the check order is the message" (review I4) lives — is six
more. `restore`/`materialise`/`restore_all`/`restore_into` are five bullets. Task 6 step 1 also writes six of
its eleven tests and names the other five in one paragraph (`plan.md:2606-2610`). These are exactly the places
C2 is where review found its blocking defects, and C2 above is one of them — an outline is what let the
apply/card contradiction survive the writing.
*Fix:* write `apply` and `run_lines_with` out in full, including the point at which a carded field is withheld
from `apply` (C2) and the exact `Totals` fields each branch sets; write the five named tests. `materialise` may
stay a bullet list if `restore`'s refusal and the allowlist are code.

**I6. `plan.md:3595-3603` (Task 12 step 8) versus `plan.md:281-283` — the close stages four files the plan reserves for hand-off H14, which its own rule says is a stop.**
*Controller hand-offs* opens with *"Every change below is outside C3' s file ownership. **No task in this plan
edits these files.**"* and Global Constraints ends with *"A task that silently edits a hand-off file is a plan
defect — stop and report it instead of editing."* Task 12 step 6 then writes the exact `CLAUDE.md`,
`app/README.md` and `HANDOFF.md` text, and step 8 runs
`git add HANDOFF.md CLAUDE.md app/README.md docs/plans/…` in the implementer's commit. An executor following
the rule stops at the close; one following the step edits reserved files. (`plan.md:3255-3258`, Task 10 step 7,
has the same collision for `engine/src/lib.rs` and at least flags it without resolving it.)
*Fix:* say once, in the hand-off preamble, that H14's text is authored in Task 12 step 6 and committed by the
controller as H14's own commit, and drop those three paths from step 8's `git add`.

### Minor

- **M1. `plan.md:2765-2773`** — `a_line_never_carries_a_vault_path_a_bearer_or_a_hostname` builds
  `["Bearer", &dir.to_string_lossy().to_string(), &journal::device_name()]`: a `&str` and two `&String` in one
  array literal, over temporaries that drop at the end of the statement. It does not compile. Bind the two
  strings first and make the array `[&str; 3]`.
- **M2. `plan.md:2832-2838`** — `entitle`'s *Produces* list omits `gate_in`, which the task's own test calls
  (`plan.md:2925`). It is the only omission I found across all twelve Produces lists.
- **M3. `plan.md:106` (fidelity ledger, `§5.5 Down`)** — "`/judge-event` inside `rank`'s roster pass" is wrong.
  `/judge-event` is reached from `engine/src/enrich.rs:1704-1733`, inside the **judge** step, and
  `engine/tests/cloud_contract.rs:446` (`rank_cannot_reach_a_judgment_endpoint`) forbids the string in `cli.rs`
  outright. The row's conclusion survives; the mechanism it names does not.
- **M4. `plan.md:3357-3363`** — `the_local_snapshot_mirror_is_untouched` asserts only that `backup.rs` contains
  `pub fn`, while exit-gate item 12 claims it is *byte-identical to `main`*. Either assert the stronger thing
  (`git diff --quiet main -- engine/src/backup.rs` in the gate) or weaken the gate's sentence.
- **M5. `plan.md:983-988`** — the generated `keep` column is NOT NULL over an expression that is NULL when `op`
  is absent, and its `body::jsonb` cast raises on a non-JSON body; both surface as a 5xx from PostgREST rather
  than the 400 the validator would have given. `sync_rows.ts` guards both today, so this is robustness: wrap in
  `coalesce(..., false)` and add a JSON-validity check on `body`, or say in the migration that the validator is
  the only guard.
- **M6. `plan.md:2189-2199`** — `a_push_carries_no_token_no_hostname_and_no_app_data_path` cannot assert the
  hostname: every journal record carries `device` inside `body`, as the test's own comment concedes. Rename it
  to what it checks.
- **M7. `plan.md:1791-1795`, `:1656-1660`, `:2094`** — `sha256Hex` is written twice, verbatim, in
  `sync-push/handler.ts` and `sync-push/handler_test.ts`; `refusal()` twice across the two handler suites; and
  the loopback harness is "C2's `engine/tests/cloud_contract.rs`, copied verbatim (a second harness would be a
  second thing to keep true)" — which is the argument against copying it. Put `sha256Hex` in
  `_shared/sync_rows.ts`, `refusal()` in a shared test helper, and either extract C2's harness or say plainly
  that the copy is accepted and why.
- **M8. `plan.md:1841`, `:2035`** — both `index.ts` files, which are what actually get deployed, are specified
  only by reference ("follows C1's shape exactly"; "is `sync-push`'s, with `readRecords`/`readNotes` from
  `sync_db.ts`"). Write the ten lines out: they are the only place `restFromEnv`, `requireActiveEntitlement`
  and `asResponse` are wired, and Task 4's staging smoke depends on them.
- **M9. `plan.md:2744-2755`** — `every_refusal_is_a_named_line_and_never_a_non_zero_exit` feeds
  `api_base: 'https://example.invalid'` with no other key. It passes only because `cloudmodel::load` requires
  all four keys and returns `None`; if `CloudConfig` ever gained a defaulted field the test would resolve
  `example.invalid` and break the no-network constraint while still passing. State that dependency in the test.
- **M10. `plan.md:2711-2725`** — `a_vault_with_an_account_and_no_session_says_exactly_that` makes
  `engine/tests/sync_contract.rs` read Windows Credential Manager through `cloudmodel::resolve`. `CLAUDE.md`
  asks a new test file that touches the store to carry its own file-scoped lock; Global Constraints only argues
  that the plan adds no credential, which is true of writes and not of this read. Add the lock or record the
  exemption.
- **M11. `plan.md:3400-3403`** — Task 10 step 7's `git add` includes `engine/src/lib.rs` while H3b owns the
  declaration line; the plan names the collision and leaves it to the implementer. `git rm engine/src/history.rs`
  does not stage `lib.rs`, so the path can simply leave the task's `git add`.
- **M12. `plan.md:3578`** — Task 12 step 5 says "fifteen entries" and then enumerates twenty
  (H1, H2, H3a, H3b, H4a, H4b, H5, H7, H8a, H8b, H8c, H9a, H9b, H10, H11a, H11b, H12, H13, H14, H15), which is
  also the number exit-gate item 16 lists. There is no H6 anywhere; if that is deliberate, say so.
- **M13. `plan.md:1953`** — Task 4 step 5 pastes the staging project ref into the plan. It is a public
  subdomain, not a secret, and C2 did the same; noted only so nobody treats it as one.

## Fidelity to the Amendment 2026-09-17

The plan's own fidelity ledger is its claim; this is that claim checked against its tasks.

| Ruling | Verdict | Evidence, and where it narrows |
|---|---|---|
| **1** — desktop only; no web, no mobile, no pairing flow | **Carried** | Nothing is built toward either and no pairing exists. Plaintext rows the service reads are strictly less client-specific than the ciphertext they replace, so the one door ruling 1 leaves open (a server-run engine later) is not closed. |
| **2** — the service holds the notes and journal, readable, encrypted at rest, purged by `DELETE /account` | **Carried** | Tasks 1, 3, 4. `sync_records.body` and `sync_notes.body` are `text` under RLS with select-only policies; H1 fixes the purge list (verified: the branch's `account/index.ts:60-76` names four tables today, and `sync_generation` must leave because Task 1 drops it — a name left there is a 404 on every deletion). Narrowed on the record to Postgres over Storage, argued. |
| **2** — the key, the recovery code, the switch, its screen and the key machinery all go | **Carried** | Task 2 removes all of it; `sync_generation` dropped. Verified that Task 2 is self-consistent: `sync_vectors.json`'s only consumers are `sync_envelope_test.ts` (deleted in the same commit) and `engine/src/sync.rs`'s own test module (rewritten in the same commit), and no other engine module references `crate::sync`, so both suites stay green. |
| **2** — the folder export is struck; `GET /account/export` stays as the §9 right | **Carried, with a gap the plan names** | Nothing is added to settings. But the plan concedes `exportAll` should now carry the two tables and does not do it (`plan.md:317-320`, `:1836-1840`, `:3592`). With the folder export struck, `GET /account/export` is the **only** way a student leaves with their data, and after C3' it will not return the notes the service now holds. That is a §9 gap C3' opens; it deserves a named owner and a date in Task 12, not only a shape. |
| **2** — plain-text mirror, local journalling, amend card and never a silent merge | **Carried in design, broken in Task 6 step 5** | See **C2**. The role-reversal table (`plan.md:2450-2457`) is kept verbatim from the superseded plan (`2026-09-14-c3-sync-plan.md:3953`) and **is correct against the code**: `reconcile::resolve` builds `chains` from `local_records` and writes to `res.apply` only when that side wins (`engine/src/reconcile.rs:96-150`), so foreign records as `local_records` and the on-disk note as `upstream_meta` is the right substitution. What is wrong is what the plan does with the result. |
| **2** — restoring is signing in on a new desktop | **Carried** | Task 9 and H11a, with review B1's seed allowlist computed from `note_paths` at the moment it runs, review I5's empty-copy rule and review I10's single `materialise` all inherited intact. |
| **3** — the engine refuses a slot past the 72-hour grace | **Carried; narrowed exactly as the controller ruled, and no wider** | `gated_vault` (`plan.md:429-437`) matches `Coursework`, `Ingest`, `Judge`, `Sync` and nothing else; `surface`, `write`, `runs`, `info`, `issues`, `coursework-discover` and `rank` are ungated, and the module doc says so. §5.1's "the slots keep ranking" is unmarked by the amendment and stands; ruling 3's purpose — an orphaned binary ranks a hand-made folder and nothing else — is met because all four fillers of the folder are gated. It is raised as P5 with (b) costed at one line plus a §5.1 amendment. Verified against `engine/src/main.rs`: the `Command` enum is `Rank, Surface, Coursework, Ingest, Judge, Runs, Write`, so H4a's insertion point between `Judge` and `Runs` is real, and `entitle::decide` reproduces `app/src/account.rs:531-544` including the one-hour skew tolerance. |
| **4** — the relay fetch | **Correctly out of scope** | `engine/src/{coursework,zybooks,vhl}.rs` untouched; the ledger says why Task 11's `ics_url` change is not the same thing. |
| **5** — the C3' scope, clause by clause | **Carried** | Tasks 1-7 the sync, 8 the gate, 9 the restore, 10 git, 11 the privacy sentences and `ics_url`, 12 the close. The paused branch's Tasks 1-3 are kept where they fit (the table shapes, `sync_usage`, `sync_ceiling_bytes`, `sync_limits`, `sync_prune`, the cron job, the validator shape and its `noExtras` rule) and retired where they do not. |
| **6** — the privacy wording is Quinn's and the lawyer's *before merge* | **Carried as a precondition, not a fait accompli** | P1 (`plan.md:76`) is stated as a merge blocker, exit-gate item 15 repeats it, and Task 11 step 3 presents the words as *drafted replacements* for Quinn to read and the lawyer to review with the C1 packet's P5 list. Nothing later in the plan weakens it. Verified that all four locations and their pins are where the plan says: `site/privacy.html:13` and `:54`, `site/index.html:16`, `app/static/console.js:1371`, `engine/tests/site.rs:15`, with `engine/tests/site.rs:25-26` asserting both site pages and `app/tests/static_assets.rs:622` finding the sentence by the literal `Your vault stays on this machine` — which is why that finder has to move too, as the plan says. The drafted sentence carries no double quote and no backslash, so it is safe as a JS literal and a Rust `const`. |

## The migration, checked against what is on staging

`20260912000300_sync_plaintext.sql` is a correct forward-only correction of `20260912000100` and `000200`.

- **Nothing orphaned.** Dropping `sync_records` and `sync_notes` drops their own triggers, indexes and
  policies with them. The plan re-creates the `sync_notes_rev_stamp` trigger, `sync_notes_rev_idx`, and
  `sync_records_prune_idx` (which `000200` created on the dropped table). `public.sync_notes_stamp_rev()`
  survives — it is a standalone function — and so does `public.sync_notes_rev`, which `000100` created as a
  standalone sequence (never `owned by`), so a second desktop cannot be handed a `rev` it has seen. `sync_prune`
  is plpgsql and binds its table late; `sync_ceiling_bytes()` and the `sync_limits` view touch no dropped table;
  the `knowlu-sync-prune` cron job calls `sync_prune()` and is unaffected. `sync_usage` is kept and zeroed in
  the same statement, which is right because `drop table` fires no row triggers.
- **RLS re-enabled** on both recreated tables, with select-only policies; `sync_usage` keeps `000100`'s.
- **Revokes.** The one function the migration issues is `create or replace function public.sync_usage_bump()`,
  which returns `trigger` and is therefore exempt by kind — verified in
  `cloud/supabase/migrations/migrations_test.ts:142` (`/returns\s+trigger/i` skips the revoke requirement), and
  `create or replace` preserves existing privileges. `sync_prune` and `sync_ceiling_bytes` keep `000100`/`000200`'s
  revokes; no new view is created, so the view guard is untouched.
- **Corpus pins.** Function creations move 23 to 24 — verified: `migrations_test.ts:310` is
  `assertEquals(parsed, 23, ...)` today and the new migration adds exactly one parsed definition. The view pin
  stays 5 (`:333`), and the drop/truncate guard reads only `20260911...` files through `ours()`
  (`migrations_test.ts:8-18`, `:620-633`), so C3's three `drop table` statements are outside it by construction.
  The plan asks for the reason in the comment, which is R-C3-exec-4's rule.
- **Application.** Task 1 step 6 is `supabase db push --include-all --workdir cloud`, controller-run, staging
  only (R-C3-exec-5, with the reason: the 2026-09-12 stamp sorts before `20260916000100` already on the remote).
  Step 7's six read-backs are real proofs, including a generated-`keep` check inside a rolled-back transaction.
  Task 12 step 7 lists what production needs, `--include-all` included, plus the cron job and the ceiling
  read-back. That requirement is met.

## Table A — interfaces the plan consumes, verified against the checkout

| Symbol the plan names | Status | Where |
|---|---|---|
| `cloudmodel::{CloudConfig, load, resolve, Unavailable, CloudError, CloudClient::{new,account_id,post,get}, CALL_TIMEOUT}` | **present**, spellings exact | `engine/src/cloudmodel.rs:42,47,60,88,136,177,224,232,271,285` |
| `journal::{OPS, VIAS, device_name, NewRecord, make_record, Journal::{new,append,read,records_for,human_set,invalidate}}` | **present**; `NewRecord` has `id/field/old/new/ts/device` as the plan's helpers use them | `engine/src/journal.rs:27,31,71,121,156,216-311` |
| `ledger::{Record, JsonlLedger::{new,append}, dumps_value}` | **present**; `JsonlLedger::append` already files by the record's own UTC day (`file_for`), which is what Task 9 step 4 claims | `engine/src/ledger.rs:36,69,82-109,356` |
| `reconcile::resolve(upstream_meta, upstream_records, local_records, upstream_mtime_ts, note_id, path, via)` | **present**, 7 args; the plan's table covers 4 and the other 3 are named in its Consumes list | `engine/src/reconcile.rs:78-86` |
| `write::{WriteContext, WriteOpts, write_literals, create, delete, move_note, find_pending_amendment, propose_amendment}` | **present**; `propose_amendment` takes `(vault, target_path, meta, changes, ctx, journal, evidence, today)` as used | `engine/src/write.rs:132,211,225,379,444,472,545,622` |
| judge-once does not fire on the plan's writes | **confirmed**: `write_literals`' judge-once branch is gated on `opts.judged`, and the plan passes `WriteOpts::default()` (`judged: false`), so an `agent:knowlu.sync` actor writes through and no `judgment:` block is minted | `engine/src/write.rs:252` |
| `approvals::{AMENDABLE_FIELDS (9), AMENDABLE_FOLDERS (tasks, courses), validate_amendment, defer_over_budget}` | **present**; the 15-a-day cap applies through `propose_amendment` as the ledger claims | `engine/src/approvals.rs:42,54,339,920` |
| `ids::{NOTE_FOLDERS (6), ID_RE, is_id, read_meta, inside_vault, scan_notes}` | **present** | `engine/src/ids.rs:20,23,26,98,107,250` |
| `pystr::{read_text, write_text, NEWLINE}`, `yaml::{get, i64_of, to_json}`, `models::split_frontmatter` | **present** | `engine/src/pystr.rs:20,84,90`; `engine/src/yaml.rs:19,51,92`; `engine/src/models.rs:63` |
| `cli::run(vault, today_iso, runner, run_id)`, `PINNED_FIXTURE_DATE` | **present**, signature exact | `engine/src/cli.rs:222`; `engine/src/lib.rs:121` |
| `cli.rs`'s `cloud:<name>` calendar routing (H5, verified not edited) | **present and live** | `engine/src/cli.rs:241-253` |
| `ingest.rs`'s cloud-before-refusal order and the 404 skip (H7, verified not edited) | **present**; `resolve` at `:904` precedes the first `no ics_url configured` refusal at `:911`, and the skip line is at `:847` — Task 11's `find`-offset test passes | `engine/src/ingest.rs:847,904,911` |
| `scheduler::{slot_argv, JudgePlan, IcsState, ics_state, has_ics_url, ingest_included}` | **present**; `ingest_included` is C2's A-2 fix, so H8c is genuinely a no-op | `app/src/scheduler.rs:162,180,190,201,292,339` |
| `account::{EntitlementCache, GRACE, cache_path, decide, EntitlementState}` and the cache path | **present**; `GRACE = from_secs(72*60*60)` at `:479`, `data_dir.join("entitlement.json")` at `:485`, `decide` at `:531` — but `base.join("knowlu")` is in `state.rs:190`, **not** `account.rs` (see **I2**) | `app/src/account.rs:471-544`; `app/src/state.rs:189-193` |
| `state::{ConsoleState.{vault_io, backup, history, head_sha, auto_sync}, run_sync, run_backup, refresh_history, refresh_head, quit_flush, app_data_root_in}` | **present** — and `run_sync` already exists, which is **C1** | `app/src/state.rs:74-85,129,137,147,171,189,232` |
| `commands::{sync_inner, backup_now_inner, build_state_value}` and the four topline keys H9b removes | **present** at the lines the plan quotes | `app/src/commands.rs:28,29,33,35,264,274` |
| Tauri command counts 43 / 30 / 62 | **verified by script** over the two `generate_handler!` lists: 43 console, 30 vault-less, 62 distinct. C3' adds none, so the counts hold | `app/src/main.rs` |
| `_shared/http.ts::{json, fail, methodNotAllowed, subPath, readJson, asResponse}` | **present**; `fail` **returns** a `Response`, so `throw fail(...)` and the `.catch((e) => e as Response)` convention are both right (review B2, inherited) | `cloud/supabase/functions/_shared/http.ts:13,18,22,36,48,65` |
| `_shared/db.ts::{Rest, restSelect, restSelectAll, restUpsert, restPatch, restDelete, authGetUser, restFromEnv}` | **present**; `max_rows = 1000` confirmed, so `MAX_PAGE = 500` is safely under it | `cloud/supabase/functions/_shared/db.ts:12-122`; `cloud/supabase/config.toml:11` |
| `_shared/entitlement.ts::requireActiveEntitlement` | **present** | `cloud/supabase/functions/_shared/entitlement.ts:69` |
| `public.accounts (id)` and C3's kept objects (`sync_usage`, `sync_ceiling_bytes()`, `sync_limits`, `sync_prune(int)`, the cron job, `sync_notes_rev`, `sync_notes_stamp_rev()`) | **present on the branch and applied to staging** | `.claude/worktrees/c3-sync/cloud/supabase/migrations/20260912000100_sync.sql`, `20260912000200_sync_usage_prune.sql` |
| `config_toml_test.ts`'s `deployed.length >= 19` | **present**; 19 function entries today, 21 after H2, so no pin to bump — as the plan states | `cloud/supabase/functions/_shared/config_toml_test.ts:39`; `cloud/supabase/config.toml:64-103` |
| `sha2` in the engine's graph | **absent** — see **I1** | `Cargo.lock:2358-2372, 3823-3842` |
| fixture vaults `vault-s1`, `vault-full`, and `tasks/cs-100-hw-01.md` the tests target | **present**; none carries `config/cloud.yaml`, so the oracles cannot move and the gate cannot fire, exactly as Global Constraints argues | `engine/tests/fixtures/` |

## What I did not check

- **Nothing was executed.** No `cargo`, no `deno`, no `supabase`, no git state change. Compile and test claims
  above are read from the source, not from a run; the two that I assert cannot compile (C1, M1) are name- and
  type-level and do not need one, but every "green" in this review is the plan's claim, not an observation.
- **The staging database.** I did not connect to it, so "applied to staging" for `20260912000100`/`000200` is
  taken from the branch's commit history and the plan; the six read-backs in Task 1 step 7 are the right
  proofs and are the controller's to run.
- **Whether `body::jsonb` is accepted in a generated column** on the exact Postgres version staging runs. It
  should be (the cast is immutable), but M5's NULL and non-JSON cases are the ones I reasoned about, not the
  parser's acceptance.
- **The five Task 6 tests named but not written**, the two `index.ts` files and the bodies of `apply`,
  `run_lines_with` and `materialise` — there is nothing there to review yet (**I5**).
- **`app/static/console.js`'s `renderSyncLine` beyond the lines Task 10 quotes**, `app/tests/onboarding.rs`'s
  existing cases, `scripts/{wizard-check,settings-check}.py`, and `.github/workflows/ci.yml`'s `deno test` line
  (H13 and H15 are verifications the close performs; I confirmed only that they are framed as verifications).
- **The lawyer's read of the drafted privacy wording**, which is P1 and is Quinn's to route. I checked that the
  four copies and their three pins exist where the plan says and that the draft is a draft; I did not assess
  whether the sentence is legally sufficient.
- **Cost, latency and the two-desktop case.** The plan itself records that no second desktop has ever run and
  that `READ_LAG_SECONDS` has never been exercised by two real pushes; I have nothing to add to that.

---

**Verdict: Execute after fix round 1 (findings C1, C2, I1, I2, I3, I4, I5, I6).**
The Minor list is polish and can ride the same round or the task reports. C2 must be fixed before Task 6 is
written, C1 before Task 7's hand-offs land, and I2/I3 before Task 8 closes; I1 is a Task 2 decision and should
be settled before the manifest is touched. P1 remains the merge blocker the plan says it is.
