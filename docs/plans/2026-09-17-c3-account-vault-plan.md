# Knowlu C3′ — the account vault — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: WRITTEN 2026-09-17; valid on Quinn's signature, which landed the same day; not started.** This plan is written from the cloud design's **Amendment 2026-09-17 — desktop only, the account vault, the relay fetch** (`docs/specs/2026-09-09-knowlu-cloud-design.md`, its last section). It was drafted while that amendment stood as *PROPOSED* and is written to be valid **on signature and not before**; Quinn **signed it on 2026-09-17** (`9ff3b57`, *SIGNED by Quinn, 2026-09-17*), so Task 1 may start. Every ruling this plan argues from is one of the amendment's six; nothing in it depends on a sentence Quinn has not spoken. It **supersedes `docs/plans/2026-09-14-c3-sync-plan.md`**, which was written for an end-to-end-encrypted copy under a device-held key and whose Tasks 1–3 are already executed on the paused branch `c3-sync` (head `934fefd`).

Execute on the **existing** branch `c3-sync` in its existing worktree (`C:\Users\danie\GitHub\knowlu\.claude\worktrees\c3-sync`), forked from `origin/main` at `7585ec6`. **Rebase it on `main` first** (`main` has moved: PR #8, `274e137`), then Task 1. The nine commits already on it stay; this plan reuses some and retires the rest, and says which in every task.

**What changed, for a reader who knows the superseded plan.** Six things, and they are all ruling 2 or ruling 3:

1. **The rows are plaintext.** The account holds the record's canonical JSON and the note's text, readable by the service, encrypted at rest by the platform, deleted with the account. There is no envelope, no IV and no key.
2. **There is no device key, no recovery code, no switch and no switch screen.** The copy is not opt-in: it is what an account *is*. `sync_generation`, the key fingerprint, the 409 path and `set_sync_key` all go.
3. **Restore is signing in.** A new desktop runs the ordinary wizard; when the account already holds a vault, Finish fills the folder from it. There is no picker link for the cloud copy and no code to type.
4. **The folder export is dropped**, not moved — VISION commitment 2's "open the folder, copy it, and leave with it" is struck on Quinn's word. `GET /account/export` stays untouched as the data-access right (§9).
5. **The engine gates itself on entitlement** (ruling 3). Today only the app does.
6. **The four privacy sentences are rewritten** to say that the service stores the student's tasks and notes to keep their desktops in step, encrypted at rest, deleted with the account. That wording is Quinn's and the lawyer's to approve **before merge** (precondition **P1**).

**Goal.** The journal stops being a thing that exists only on one laptop. Every note write this device makes goes up to `POST /sync-push`; everything another desktop of the same account wrote comes back down `GET /sync-pull` and is applied through `write`, so a second desktop is journalled exactly like the first and a genuine two-sided conflict surfaces as an amend card instead of a silent merge. A student whose laptop is gone signs in on a new one and the wizard fills the folder from the account. The engine refuses to spend a slot's cloud work without a valid entitlement. Git leaves the product entirely: `engine/src/history.rs` is deleted, the app stops spawning `git`, and the local snapshot mirror stays exactly as it is.

**Architecture.** One plaintext row store, two endpoints, one engine module. On the **device**, `engine/src/sync.rs` collects each new journal record and each changed note, posts them to `/sync-push`, pulls what it has not seen from `/sync-pull`, appends foreign journal records to the local ledger **verbatim** — they carry their own `ts`, `device` and `seq`, and rewriting those would destroy the attribution judge-once reads — and settles the note-level effect through the existing pure `reconcile::resolve`, writing the winners through `write` and filing an amend card for every field both sides moved. On the **server**, two thin Deno handlers over two Postgres tables that hold the record's canonical JSON and the note's text under RLS, with an opaque 16-hex device token, a 64-hex content hash and a receive timestamp beside them. The cursor is a per-account identity column; a retried push is idempotent on the content hash, which the server **re-derives from the body it was given** rather than trusting. `knowlu-engine sync` is one more slot step with the same discipline as `judge` — **it always exits 0** — and it runs **first**, so the day ranks on what came down.

**Tech Stack:** Supabase (Postgres 15 + Edge Functions on Deno) for the two endpoints, reusing C1's `_shared/{auth,db,entitlement,http}.ts` and C2's `CloudClient` — **no second HTTP client anywhere**; Rust 1.98 `stable-x86_64-pc-windows-gnu` with `sha2` 0.10 for the content hash (already resolved in this workspace's `Cargo.lock`, so this is one direct edge and no new crate — the same argument the retired envelope made for `ring`, and the reason `ring` and `base64` leave in Task 2); `deno test` for every pure module and one request/response contract test per function; `std::net::TcpListener` on `127.0.0.1:0` for every Rust test that exercises a client.

**Spec.** `docs/specs/2026-09-09-knowlu-cloud-design.md` — read the **Amendment 2026-09-17 whole** before Task 1, then **§5.5** (the section this plan implements, as amended), §3.1, §4.4 and §9; `VISION.md`'s commitment 2 **as amended** is the sentence the whole plan serves. The parent design for the vault, the journal and `write` is `docs/specs/2026-08-11-personal-ops-system-design.md`. The superseded plan is `docs/plans/2026-09-14-c3-sync-plan.md` and its review is `docs/reports/2026-09-14-c3-sync-plan-review.md`; **the review's resolutions are rulings this plan inherits** and are carried in the fidelity ledger and in the tasks that hold them (B1's seed allowlist, B2's thrown-`Response` convention, B3's three `state/` inputs, B5's startup thread, I3's read lag, I4's check order, I5's empty-copy rule, I7's two guards, I9's hand-off rule, I10's one `materialise`). The paused branch's own ledger is `.superpowers/sdd/2026-09-14-c3-sync-plan/progress.md`; **R-C3-exec-4** (the corpus-wide pins are bumped by whichever stream moves them) and **R-C3-exec-5** (`db push --include-all` for every C3 migration) still bind and are stated again below.

---

## Global Constraints

Every task's requirements implicitly include this section. It is the superseded plan's, verbatim, except where the amendment changes it — and each change says so.

- **Add no single-user assumptions.** Nothing in `cloud/`, `engine/`, `app/` or a fixture names a person, a vault, a machine, an account or a credential. Anything that would need hand-editing for a second user is a bug. (`CLAUDE.md`, rule 1.)
- **Never regenerate a frozen reference.** The eight Python-written references in `engine/tests/fixtures/` are read-only oracles. The three Rust-generated surface references (`surface-today-{s1,s1-migrated,full}.json`) may be regenerated only in a commit whose diff shows the change and whose message says why (console spec §4.6) — **no task here regenerates one**, and `git status --porcelain --untracked-files=all engine/tests/fixtures/` is **empty at every task boundary**: no task in this plan adds, edits or removes a file under `engine/tests/fixtures/`.
- `engine/tests/oracle.rs` and `engine/tests/surface_oracle.rs` must pass **unchanged** at every task boundary. None of the three fixture vaults carries a `config/cloud.yaml`, so `sync` skips them, the entitlement gate never fires on them, and their `today.md` cannot move.
- **Every note write goes through `write` — journal record first, single-line frontmatter surgery second.** `src/yamlemit.rs` is the crate's one YAML emitter. **No note is ever parsed and re-dumped.** There is exactly one recorded exception in this plan, argued in the fidelity ledger and again at Task 9: **restore materialises bytes that were already journalled on the device that made them**, so it writes note files with `pystr::write_text` and journal lines with `JsonlLedger::append`, and it runs only into a vault holding no note its caller did not name.
- **`journal::VIAS`, `journal::OPS`, run records, ledgers and note frontmatter are contracts with existing vaults: byte-identical, never renamed.** This plan adds no `via`, no `op`, no frontmatter key and no run-record field. The one new file it puts in a vault is `state/sync-cursor.json`, which is generated, device-local and never synced.
- **All JSON this crate writes goes through `ledger::dumps_value`** (Python `json.dumps` separators), never `serde_json::to_string`. **The canonical bytes a record is hashed over are `dumps_value`'s**, which is what makes a hash computed on two machines the same hash — and, now that the server can read the row, what makes the server's own re-derivation agree with the device's.
- **The vault stays plain text on the student's machine, and the device ranks from the folder on disk — never from the account.** *(Amended: the account is the source of truth for the **data**; the folder is the source of truth for **a run**.)* A pull that fails, a 402, a 500: every one is a named line and exit 0, and the day still ranks from the folder. **Nothing in this plan ever deletes a local note because the cloud does not have it.**
- **`rank` never calls a model, and no path under `cli.rs` may reach `/judge-*`** (Knowlu spec decision 11, `CLAUDE.md`). `sync` is transport and carries no judgment; `rank_cannot_reach_a_judgment_endpoint` (C2) must still pass.
- **`knowlu-engine sync` always exits 0.** No `config/cloud.yaml`, no session, no entitlement, no network, a 402, a 429, a 5xx, a timeout — all normal outcomes reported on stdout as named lines. A non-zero exit sets `RunSummary.engine_ok = false` in the app's scheduler, which paints the tray amber and puts the slot into retry backoff twice a day forever. **The same rule binds the entitlement gate of Task 8**: a gate that refuses is a named line at exit 0, never a failure.
- **No test reaches the network.** A `TcpListener` (Rust) or `Deno.serve` (TypeScript) bound to `127.0.0.1:0` inside one test, answering that same test's own request, is not egress: no DNS, no route off the machine, no listener on a routable interface. Every such test binds `127.0.0.1`, never `0.0.0.0`, and joins its listener thread (or `await`s its server's `shutdown()`) before returning.
- **Two `deno test` command lines, and which is which.** A run over **C3′'s own files alone** is `deno test --allow-read --config cloud/supabase/deno.json <path>` — nothing C3′ writes needs more. A run over **the whole tree** is C2's merged command and must be copied exactly, because a narrower flag set fails on C2's tests rather than on C3′'s:

  ```
  deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/
  ```

- **A third-party import in an edge function is an `npm:` or `jsr:` specifier on the import line**, never a bare name resolved through the root `deno.json` import map: the `--use-api` bundler does not read that file. C3′ imports nothing third-party — only `@std/assert` (test-only) and the platform's own `crypto.subtle` — so this costs it nothing and is stated so the next person adding one does not learn it at deploy time.
- **No engine command is ever run against the worktree root or a real vault** (ruling R-3a-12). No `--vault .`, no path inside the worktree, and never Quinn's own `C:\Users\danie\Knowlu\Vault`. Tests copy a fixture vault into a temp directory.
- **Tests that touch the real Credential Manager are serialised.** Windows races parallel `CredWriteW`/`CredReadW` calls (spurious `ERROR_NOT_FOUND`). `app/tests/account.rs` holds a file-scoped `CREDMAN_LOCK`; **any new test file in this plan that writes, reads or deletes a real credential carries its own file-scoped lock**, a generated test id, and a `Drop` guard that deletes what it wrote (`CLAUDE.md`). *(Amended: this plan adds **no** credential — the sync key is gone — so no new file should need one. A task that finds itself reaching for Credential Manager has misread the amendment; stop and report it.)*
- **TDD, per task:** the failing test first, run it and see it fail with the message the step names, the minimal implementation, run it and see it pass, commit.
- `cargo build --workspace` and `cargo test --workspace` from the root at **0 warnings**. The one accepted line is the app's `.rsrc merge failure: multiple non-default manifests` linker message. `cargo test --release` will not link — test in the dev profile.
- **Line endings: LF everywhere** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1` CRLF; `engine/tests/fixtures/** -text`). New `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and `.md` files are LF. `git diff --stat` never shows a whole-file flip. Vault files the engine writes go through `pystr::write_text`.
- **No secret in the repo, a log, a fixture, a test name, a migration, a commit message or this plan.** *(Amended: the one key-shaped literal the superseded plan committed — `SYNC_TEST_KEY_NOT_A_SECRET` — leaves with the envelope in Task 2. This stream now commits no key-shaped literal at all, and no test in it needs one.)*
- **Migrations are applied to the STAGING project only** (§11 R6). Nothing in this plan touches the production project; Task 12 records what production still needs.
- **Migrations are stamped `cloud/supabase/migrations/20260912……_<name>.sql`.** C1 owns 2026-09-10, C2 owns 2026-09-11 and 2026-09-16 (the provider swap), C3/C3′ own **2026-09-12**. **`20260912000100_sync.sql` and `20260912000200_sync_usage_prune.sql` are already applied to staging and are never edited** — migrations are forward-only, and the superseded shape is corrected by a **new** file, `20260912000300_sync_plaintext.sql`. **`supabase db push --include-all`** is required for every migration in this stream (**R-C3-exec-5**): the 2026-09-12 stamp sorts before migrations already on the remote (`20260916000100`), and without the flag the CLI skips it silently.
- **The corpus-wide pins in `cloud/supabase/migrations/migrations_test.ts` are bumped by whichever stream moves them** (**R-C3-exec-4**), with the reason in the comment, in that stream's own commit. C3′'s own guards stay in `cloud/supabase/migrations_sync_test.ts`, filtered to `20260912…` (R-X-8's rule, applied from C3′'s side).
- **Commits:** specific `git add` (never `git add -A`), the message through a file (`git commit -F <file>`), and both trailers. The two literals below are **the writing session's**; the executing session substitutes its own model name and session URL, exactly as `HANDOFF.md` §5 says (ruling R-C2-6 — this is not a defect to fix, it is a template to fill):

```
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
```

- Never bare `git stash` / `git stash pop` (ruling R-3a-29): the stash stack is shared with the main checkout and every worktree.
- **File ownership.** C1 and C2 are merged, so their exclusive-ownership rows retire with them; the **shared files with a single owner** in `HANDOFF.md` §2 do not, and every one of them is a hand-off here. **C3′ owns:** `cloud/supabase/functions/{sync-push,sync-pull}/**`, `cloud/supabase/functions/_shared/sync_*.ts`, `cloud/supabase/migrations/20260912*.sql`, `cloud/supabase/migrations_sync_test.ts`, `engine/src/sync.rs`, `engine/src/entitle.rs` (new), `engine/Cargo.toml` and `Cargo.lock`, the deletion of `engine/src/history.rs`, `engine/tests/**`, `app/tests/**`, `app/static/**`, `site/privacy.html`, and this plan file. **C0 still owns** `.github/**`, `scripts/ci/**`, `scripts/release.ps1`, `scripts/sign.ps1`, `engine/tests/workflows.rs` and the three `app/tauri.conf.json` updater keys — C3′ touches none of them except through hand-off **H15**, which is a verification. `site/index.html` is hand-off **H12**; `cloud/supabase/migrations/migrations_test.ts` is edited **in task** under R-C3-exec-4 and is the one exception to the sentence above. **A task that silently edits a hand-off file is a plan defect** — stop and report it instead of editing.

---

## Quinn-owned preconditions (asked one at a time, when a task reaches them, with the context)

Five. Two of the superseded plan's seven are **settled by the amendment** and are not asked again: **P1** (where the sync key comes from) is settled as its option (c) — the service holds the notes readable, encrypted at rest — and **P5** (off-then-on) has no switch to turn off. **P3** (the ceiling and retention) was **asked and answered on 2026-09-17**: 200 MiB per account, 400 days, and a record a human wrote kept for ever; it is built to that answer and appears in the fidelity ledger, not here.

| # | Needed by | What, and what breaks without it |
|---|---|---|
| **P1** | Task 11 | **The four privacy sentences, and the bullet above them.** Ruling 2 names this as C3′'s own task and says the wording is Quinn's *and the lawyer's* to approve **before merge**. Today `site/privacy.html:13`, `site/index.html:16`, `app/static/console.js:1371`'s `var PRIVACY` and `engine/tests/site.rs:15`'s `const PRIVACY` all carry, character for character: *"Your vault stays on this machine. Knowlu's servers hold your account, the judgments they make for you, and what you correct; they never hold the text of your notes, and nothing here is ever sold or shared."* — and `site/privacy.html:54` carries the **"No note bodies stored."** bullet, which pre-commits the design of a switch this plan does not build. **After C3′ both are false.** Task 11 step 3 carries the drafted replacements and the three tests that hold all four copies in step. **Quinn reads both, and they go to the lawyer with the C1 packet's P5 list** (`docs/reports/2026-09-10-c1-quinn-packets.md`). **Without it:** every task still runs and the branch still passes its gate, but **it must not merge** — shipping a product whose live policy says the opposite is the one thing in this stream that is not fixable afterwards. |
| **P2** | Task 10 | **The one-line go to delete `engine/src/history.rs`.** It is 707 lines with a full test suite, it is the last of plan 2's sync work, and §4.4 says plainly that the app stops doing git. This is not a reversal — it is the removal the spec ordered — but it is irreversible in the sense that re-adding it would be a rewrite, so it is said out loud once. **Without it:** Tasks 1–9 and 11 all land; Task 10 stops, `history.rs` stays compiled and unreferenced, `app/src/state.rs` goes on spawning `git` against a folder that is not a repository, and the exit gate records the gap. |
| **P3** | Task 9 | **Does the picker still get a *Restore from a backup folder…* link?** The superseded plan's P6 asked which of two restores `HANDOFF.md` §4's row (Quinn, 2026-09-14) meant. **Half of it is now answered by the amendment**: the *cloud* restore is not a picker link any more, it is what Finish does when the account already holds a vault (ruling 2). The other half is untouched — `onboarding::restore_vault` / `restore_vault_in` are registered in the wizard window's handler list and have **no caller**, and the local `Backups\` mirror is still the answer to *this machine is fine and I deleted a file yesterday*. **Recommendation: yes, one link**, about fifteen lines of `console.js` and a dialog, because the function already copies the mirror out to a new folder rather than adopting it in place and because the alternative is shipping two registered commands nothing can reach. **The plan is built to yes** (Task 9 step 6). If Quinn says no, delete that step and add a *What is NOT in this plan* line recording `restore_vault` as still caller-less. |
| **P4** | Task 4 | **`/sync-pull` returns this device's own rows and the device filters them by content hash.** Unchanged from the superseded plan's P7 and re-asked because the answer is still a product judgement. For a single-device account — every account at launch — that is one wasted page per slot immediately after a push. **(a) Accept it** (recommended): the hash filter has to exist anyway, because it is also what makes a re-install on the same machine idempotent, and a wasted page is a few hundred kilobytes twice a day. **(b) Add `device=neq.<token>` on the server** and keep the hash filter for the re-install case: two lines in `sync_db.ts`, at the cost of the server learning which device is asking, which it has no other reason to know. **Recommendation: (a)**; Task 12 records (b) as the first lever if the pull ever costs anything. **The plan is built to (a).** |
| **P5** | Task 8 | **Past the 72-hour grace, does `rank` still run?** Ruling 3 says *"The engine refuses to run a slot without a valid entitlement past the 72-hour grace the app already caches"*. §5.1 — which the amendment's *Markers* paragraph does **not** mark, so it stands — says *"past grace the page shows a banner and the slots keep ranking but the cloud steps are skipped as named steps, never a failed slot."* They disagree about exactly one command. **(a) Gate the four cloud steps — `sync`, `coursework`, `ingest`, `judge` — and let `rank` keep ranking** (recommended). Ruling 3's own stated purpose is satisfied: *"With the fetch sequence, the parsers, the rules and every judgment server-side and entitlement-gated, an orphaned binary ranks a hand-made folder and nothing else."* Nothing fills the folder, so within a day the ranking is of a vault that has stopped moving — and today's page never blanks, which is the failure the grace exists to prevent. **(b) Gate `rank` too**, so `today.md` freezes at its last run and the page shows what it last said. Cost: a student whose card expires on a Friday loses the page, not just the updates, and §5.1's sentence has to be amended in the same commit. **The plan is built to (a)**, and (b) is **one line** — `Command::Rank` joins the gated arm's pattern in hand-off **H4** — plus the §5.1 amendment. **Without a ruling:** Task 8 ships (a) and Task 12's report names the open half. |

Nothing in this stream needs a new Supabase project secret, a new Stripe object, a new Google scope, a new DNS record or a new Tauri command.

---
## Fidelity ledger

One row per ruling, spec sentence or inherited resolution C3′ carries or narrows, with the narrowing on the record and its reason. The amendment's six rulings come first, because they are what makes this plan a different plan.

| # | Decision / sentence | Source | Carried by |
|---|---|---|---|
| **R1** | "Knowlu is a Windows desktop app… There is no web app and no mobile app in the plan… no device-pairing flow" | Amendment ruling 1 | **The whole plan, by what it does not build.** The superseded plan's rows were client-agnostic ciphertext precisely so a browser could one day open them; this one stores plaintext the *service* reads, which is a strictly easier thing for a future client and needs no pairing. Nothing here builds toward a browser or a phone, and nothing here forecloses the one door ruling 1 leaves open: a server-run engine would read these same rows. |
| **R2** | "the service holds each account's notes and journal **readable by the service**, encrypted at rest by the platform, purged by `DELETE /account`" | Amendment ruling 2; §11 R4 **reversed** | Tasks 1, 3, 4. Two Postgres tables under RLS holding the record's canonical JSON (`sync_records.body`) and the note's text (`sync_notes.body`), encrypted at rest by Supabase's own storage encryption, deleted by the purge list C1 maintains (hand-off **H1**). **Narrowed, on the record: Postgres, not Supabase Storage** — unchanged from the superseded plan's argument. A vault's whole corpus is markdown and JSON (500 notes is about a megabyte); Postgres gives it row-level security, one access path, a cursor that is just an identity column and a purge that is one more table name in a list C1 already maintains. Storage would add a bucket, a second authorisation surface, a second retention mechanism and object names that are themselves metadata. **And the endpoints are `/sync-push` and `/sync-pull`, hyphenated**, not §5.5's slashed spelling: Supabase routes `/functions/v1/<function-name>` and a slash in a function name is a different function (C2's review Critical 1 ruled the same way). |
| **R2** | "The device sync key, the recovery code, the sync switch and its own screen (C3's P2 screen), the key-generation machinery, 'restore with your recovery code'… what goes" | Amendment ruling 2 | Task 2 retires all of it: `engine/src/sync.rs`'s key, envelope, keyed-index and recovery-code sections are deleted, `ring` and `base64` leave `engine/Cargo.toml`, `sync_vectors.json` and `sync_envelope_test.ts` are deleted, and Task 1's migration drops `public.sync_generation`. **No `app/src/sync.rs` is created and no Tauri command is added** — the superseded plan's five (`sync_state`, `set_sync`, `set_sync_key`, `export_data`, `stage_restore_key`) existed to drive a switch, a key and an export that no longer exist. The counts in `CLAUDE.md` stay **43 / 30 / 62**, and Task 12's docs step says so rather than recounting to the same number by accident. |
| **R2** | "the folder export — VISION commitment 2's 'open the folder, copy it, and leave with it' is struck on Quinn's word; `GET /account/export` stays as the data-access right (§9)" | Amendment ruling 2 | **Dropped, not moved.** The superseded plan's Task 10 added an *Export my data* settings row calling C1's `GET /account/export`; C3′ adds nothing to the settings panel. `site/privacy.html`'s rights list (`:96`) already names export and is **not edited** — the right is unchanged and C1 already implements it. `scripts/settings-check.py` is verified, not edited (hand-off **H13**). |
| **R2** | "Each desktop keeps a plain-text **mirror**… every write journals locally exactly as today and syncs through the account; two desktops moving one field offline still surface an amend card, never a silent merge" | Amendment ruling 2; §5.5; VISION | Tasks 5, 6, 7. Carried through the existing pure `reconcile::resolve` with the roles reversed (Task 6 states the reversal twice, once in prose and once in the code). **Narrowed where it must be:** `approvals::validate_amendment` only ever applies a card whose target is under `AMENDABLE_FOLDERS` (`tasks`, `courses`) and whose field is one of `AMENDABLE_FIELDS` (nine), with non-null scalars on both sides. A conflict outside that set cannot become a card that would ever apply, so it takes reconcile's own rule (later `ts` wins), keeps its `supersede` record for the loser, and emits **one named warning line** naming the note and the field. Nothing is silently dropped; the losing value is in the journal either way. |
| **R2** | "restoring is signing in on a new desktop; the mirror fills from the account — old Task 11's picker link becomes the wizard's ordinary path when the account already holds a vault" | Amendment ruling 2 | Task 9. `create_vault_in` calls `knowlu_engine::sync::restore_into(&dest)` after `move_session` and before `finish_or_roll_back` (hand-off **H11**). **Three inherited resolutions ride on it and are kept:** the **seed allowlist** (review B1 — the vault is seconds old and `scaffold::seed_writes` has already written `archive/_migrated.md`, `tasks/get-to-know-knowlu.md` and one `courses/<slug>.md` per seeded course, so the allowlist is computed from `note_paths(vault)` at the moment it runs rather than demanded to be empty); the **empty-copy rule** (review I5 — an empty account copy keeps the vault and says so on the finish panel; only an `Err` rolls the folder back); and the **one `materialise`** (review I10 — `restore` is the allowlist check plus `materialise`, and `restore_all` calls `materialise` directly for the second and later pages). |
| **R3** | "The engine refuses to run a slot without a valid entitlement past the 72-hour grace the app already caches (a task of C3′ below; today only the app gates)" | Amendment ruling 3 | Task 8, and **precondition P5 is the one open half**. `engine/src/entitle.rs` reads the cache the app already writes — `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\entitlement.json`, whose `<profile_id>` is the middle segment of `config/cloud.yaml`'s `session_credential_target` — and re-implements `account::decide`'s arithmetic (status in `active`/`trialing`, `checked_at` within 72 hours, an hour of clock-skew tolerance) in the engine, pinned to the app's by a test that reads both constants. **Narrowed, on the record: the gate is in `engine/src/main.rs`'s match, not in each command.** One `if let Some(line) = entitle::gate(&vault)` in front of the slot arms is one place to read, one place to review and one place to widen if P5 is answered (b); a gate repeated in five `run_lines` bodies is five places to forget it. The consequence, stated so it is chosen deliberately: a *library* caller (`knowlu_engine::cli::run` from a test, or the app calling into the crate) is not gated. The binary is the product and the binary is what an orphaned copy runs. |
| **R3** | "D4 stands: ranking, capacity, rendering, the journal and the scheduler run on the device and work offline, and today's list exists with no connectivity because it is computed from the mirror" | Amendment ruling 3; §3.1 | Global Constraints, and exit-gate item 3: a device with no network or no entitlement ranks the day from the folder on disk and says why in one line. This is also the argument for P5's recommendation (a). |
| **R4** | "the relay fetch; D11 amended" | Amendment ruling 4 | **Not in this plan — C5's.** Nothing here touches `engine/src/{coursework,zybooks,vhl}.rs`, and Task 11's change to `engine/src/ingest.rs` is about the **capability URL**, not about the fetch sequence. Named here so nobody reads the two into each other. |
| **R5** | "C3′ — the account vault (sync without encryption on the paused branch's tables; the privacy sentences; the export dropped; the entitlement check in the engine; git leaves the product; `ics_url` leaves the vault)" | Amendment ruling 5 | **This plan's scope, exactly**, one clause per task group: Tasks 1–7 the sync, Task 8 the entitlement check, Task 9 the restore that makes a second desktop real, Task 10 git, Task 11 the privacy sentences and `ics_url`, Task 12 the close. |
| **R5** | "The paused `c3-sync` branch's Tasks 1–3 are kept where they fit… and retired where they do not (the envelope, the key, the vectors)" | Amendment ruling 5 | Tasks 1, 2, 3. **Kept:** the two tables' *shape* (an identity-column cursor, a content-hash uniqueness constraint, a `rev` sequence and its stamping trigger for notes, `sync_usage` and its O(1) counter triggers, `sync_ceiling_bytes()`/`sync_limits`, `sync_prune(int)` and its cron job, the RLS-and-no-write-policy rule, the purge list, `_shared/sync_rows.ts`'s validator shape and its `noExtras` rule). **Retired:** the envelope, the key, the recovery code, the keyed index, `sync_generation`, the vectors and both languages' vector tests. **Never edited:** `20260912000100_sync.sql` and `20260912000200_sync_usage_prune.sql`, which are on staging; Task 1's `20260912000300_sync_plaintext.sql` is a new file that drops and recreates the two content tables. |
| **R6** | "the wording of the privacy sentence (C3′'s task, read by Quinn and the lawyer before merge, as C3's P2 already required)" | Amendment ruling 6 | **Precondition P1**, and Task 11 step 3 carries the drafted words. |
| §5.5 Down | "the service's own writes… are queued as journal-shaped records the client pulls at `/sync/pull` and applies through `write`… with `actor: agent:knowlu.<kind>` and judge-once intact" | cloud design §5.5 | **Narrowed, on the record, and the reason has changed with the amendment.** All four kinds are **already delivered by pull, inside a slot step, by C2** — `/judge-task` per item inside `judge`, `/ingest-coursework` inside `coursework`, `/judge-event` inside `rank`'s roster pass, and the Gmail and promoted-rule queues inside `judge` — each written through `write` under `agent:knowlu.<kind>` with judge-once intact, which is exactly the property the sentence asks for. The superseded plan's *structural* reason for not re-routing them (a server-originated row could not be sealed) **no longer applies**: the server could now write a row. What decides it instead is smaller and still decides it: re-routing would re-open every C2 test for no behaviour change, and it would give the service a second way to write a note — one that skips the `judge`-step reporting a student reads in the Runs view. **C3′'s `/sync-pull` therefore carries exactly one thing: records another desktop of the same account pushed.** Revisit in C5, where the relay moves more of the fetch server-side and the question arises for real. |
| §5.5 Second device | "replaying the journal onto an empty vault reconstructs it — the migration script already proved the shape" | cloud design §5.5 | Task 9. **Carried, with one recorded exception to `CLAUDE.md`'s "every note write goes through `write`".** A restore writes note files with `pystr::write_text` and journal lines with `JsonlLedger::append`, because every one of those records **already exists** and carries the `ts`, the `device` and the `seq` of the machine that made it. Putting them through `write` would fabricate a second `create` record, stamped with *this* device's name and *today's* `ts`, for a note created three months ago on another laptop — and `journal::human_set`, which is what judge-once reads, would then answer with the restore instead of with the student's own decision. The invariant exists to make attribution complete; re-journalling a restore is the one way to break it while appearing to obey it. **Bounded exactly as the code enforces it:** `restore` refuses a `dest` holding any note in any of `ids::NOTE_FOLDERS` (`tasks, approvals, archive, courses, issues, info`) other than the ones its caller explicitly tolerates. |
| §5.5 Backups | "the local snapshot tick stays; the cloud copy is the durable one" | cloud design §5.5 | Tasks 9 and 10, and the backup mirror is untouched. **`engine/src/backup.rs` is not modified by any task in this plan**, which is the whole of "the local snapshot tick stays"; exit-gate item 11 asserts it is byte-identical to `main`. |
| §5.5 / P3 | The ceiling and the retention window, **and the `keep` bit** | cloud design §5.5; the superseded plan's P3, **answered by Quinn 2026-09-17** | Task 1. 200 MiB of ciphertext per account becomes 200 MiB of **text** per account (`sync_ceiling_bytes()` is unchanged at `209715200`, and the unit it counts is now honest); journal records are pruned at 400 days; **a record a human wrote is never pruned**, which is the half that is a correctness property rather than a storage one — `journal::human_set` is what judge-once reads and it reads *records*, so a pruned human `set` on a restored machine is a decision the student made and the device can no longer see. **Changed by ruling 2, and this is the improvement the amendment buys:** the device no longer *asserts* `keep`. The server decides it, from the plaintext record, in a **stored generated column** — `op` in (`set`, `create`) and `actor` not starting with `agent:`, which is `provenance::is_agent`'s own test. The one cleartext bit the superseded design had to leak, and the trust it had to place in the client, both go. |
| §9 | "Export / access / delete implemented for everyone (`DELETE /account`, `GET /account/export`)" | cloud design §9 | Task 1 (hand-off **H1**) and the ledger row above. The purge list loses `sync_generation` and keeps `sync_records`, `sync_notes` and `sync_usage`. **`exportAll` is extended this time**, and the reason is the amendment: the superseded plan left it alone because the rows were ciphertext this service cannot read, so "the plaintext is the vault folder" was a true and better answer. The rows are readable now, and with the folder export struck (ruling 2) `GET /account/export` is the **only** way a student takes their data elsewhere. Task 4 step 8 records this as the one item C3′ hands to whoever owns `account/index.ts` next, with the exact shape; it is **not** built here, because widening `exportAll` is a change to C1's own function body rather than a list, and *What is NOT in this plan* names it. |
| §9 | Alabama § 8-38: session tokens and OAuth tokens are SPII, encrypted at rest, 45-day notice | cloud design §9 | Tasks 1 and 4. The sync tables hold no token. **What they now hold that they did not is the student's own coursework text** — which is why encryption at rest is named in the privacy sentence (P1) and why `DELETE /account` naming every table is a gate item rather than a nicety. The device adds no credential at all. |
| §6 (b) | Corrections are derived from the journal on the device and sent to `POST /telemetry`; **no note body** | cloud design §6 | **Unchanged, and deliberately separate.** `app/src/telemetry.rs` keeps its own watermark, its own endpoint and its own rules; sync does not feed it and it does not feed sync. The two read the same ledger for different purposes, and merging them would put a pseudonymous analytics row and the student's own vault under one consent. **The class-(c) opt-in is not touched and is not implied**: a note in the account is the student's own copy of their own data, not training material, and *What is NOT in this plan* says so. |
| §12 C3 | "`history.rs`'s git removed" | cloud design §12's C3 row; §4.4 | Task 10. `engine/src/history.rs` is deleted whole; `state::run_sync`, `state::refresh_history`, `state::refresh_head`, `ConsoleState::{history, head_sha, auto_sync}` and the page's repo/remote/ahead/behind copy go with it. **`runs::git_sha` stays**: it fills the `sha` field of a run record, that field is a contract with existing vaults, and on a vault that is not a repository — which is every vault the app creates (§4.1) — it already answers `None`. Its only app-side caller (`state::refresh_head`) goes, so no `git` process is spawned by the app any more. |
| §4.1 | "The vault is **not** a git repository and the app never assumes git exists" | cloud design §4.1 | Task 10, and the static test `no_git_process_is_spawned_for_a_vault` that keeps it true. |
| C2 ledger | "the vault copy of `ics_url` … **who removes it and when:** C3" | C2 plan's fidelity ledger; ruling R-X-16 | Task 11. **Carried, and the stated reason is corrected on the record**, exactly as the superseded plan corrected it: journal sync gives the device nowhere else to get the URL from — `/ingest-ics` and `/ingest-calendar` do, and have since C2. The reason to do it now is that `config/ingest.yaml` holds, in plain text the student can read, edit and paste into a support ticket, a URL that reads their whole timetable (§9, Alabama SPII), and it silently diverges from the account's copy the moment they re-generate the secret address. The cost — an offline `ingest` — is paid in the same task. |
| §11a | "the personal calendar … C1 delivers it by its secret iCal address … written to the vault's `calendars:`" | cloud design §11a | Task 11, the same removal and the same argument: `- name: personal` / `ics_url:` in `config/ingest.yaml` becomes `- name: personal` / `ics_url: 'cloud:personal'`, which C2's hand-off H4 already routes to `/ingest-calendar?name=personal`. |
| `CLAUDE.md` | Approvals are capped at 15 new proposals a day; overflow is snoozed, never deleted | `CLAUDE.md`; `approvals::defer_over_budget` | Task 7 files conflict cards through the ordinary `propose_amendment` path, so the existing cap applies unchanged and a hundred-conflict first pull spreads over days rather than burying the deck. |
| `CLAUDE.md` | `journal::VIAS` does not grow | `CLAUDE.md` | The slot's sync step runs `--via local-runner`; the console's *Sync now* runs `via: "dashboard"`. Both are already in `VIAS`. |
| §8 | `dependency_boundary.rs` pins the engine's dependency budget; TLS is rustls/ring, never OpenSSL | cloud design §8 | Task 2. The crypto test that named `ring` and `base64` is **replaced**, not deleted: `ring` and `base64` leave the manifest with the envelope, `sha2 = "0.10"` arrives as the one direct edge the content hash needs (already resolved in `Cargo.lock`, so no new crate and no new version to audit), and the test refuses the return of `ring`, `base64`, `aes-gcm`, `chacha20` and any second hash. |
| §5.6 | Judgment logs never enter the vault | cloud design §5.6; `CLAUDE.md` | Unchanged: `sync` writes no judgment log, and `state/sync-cursor.json` is a cursor, not a log — it holds two integers, one timestamp and a map of note path → content hash, all of which are already in the vault in plainer form. |
| §13 | "the console's visual redesign (parked)" | cloud design §13 | Task 10 rewrites exactly one line of the page — the sync line — because the facts behind it no longer exist. Nothing else in the console's appearance is touched, and no settings row is added. |
| Review resolutions | B2 (thrown `Response`), B3 (three `state/` inputs a restore does not carry), B5 (`main.rs`'s startup thread), I3 (the ten-second read lag), I4 (the check order), I7 (`is_note_path` + `record_is_well_formed`), I9 (the hand-off rule) | `docs/reports/2026-09-14-c3-sync-plan-review.md` | **Inherited whole.** B2 is *Interfaces* contract 5 and the `refusal()` helper in both handler suites; B3 is Task 9's rank comparison; B5 is hand-off **H10**; I3 is `READ_LAG_SECONDS` in Tasks 3 and 4; I4 is `run_lines_with`'s check order in Task 7; I7 is the two guards in Task 6; I9 is the *Controller hand-offs* preamble. |

---

## File structure

### What this plan does *not* move, and why — read this before Task 1

Four things a reader will look for and not find, each with its argument in the fidelity ledger:

1. **Server-originated writes do not come down `/sync-pull`.** C2 already delivers all four kinds by pull inside its own slot steps. `/sync-pull` carries other desktops' records and nothing else.
2. **Supabase Storage is not used.** Two Postgres tables under RLS, purged by the account function C1 already owns.
3. **`engine/src/backup.rs` is not touched.** The local mirror and its thirty snapshots are the answer to "I lost the file five minutes ago"; the account's copy is the answer to "I lost the laptop". They are different products and this plan keeps both.
4. **No `app/src/sync.rs`, no new Tauri command, no new settings row.** Everything the superseded plan put in an app module was switch, key, recovery-code and export machinery that ruling 2 removes. What is left is a status struct, and it belongs in the engine beside the run that produces it.

### New — the service

- `cloud/supabase/migrations/20260912000300_sync_plaintext.sql` — drops `sync_generation` and the two ciphertext tables, recreates `sync_records` and `sync_notes` with plaintext bodies and a server-decided `keep`, rebuilds their triggers, index and policies, and leaves `sync_usage`, `sync_ceiling_bytes()`, `sync_limits`, `sync_prune(int)` and the cron job exactly where 20260912000100/000200 put them.
- `cloud/supabase/functions/_shared/sync_db.ts` — the PostgREST calls these two endpoints make, over C1's `Rest`. The only database access C3′ adds.
- `cloud/supabase/functions/sync-push/{handler.ts,handler_test.ts,index.ts}` — `POST /sync-push`.
- `cloud/supabase/functions/sync-pull/{handler.ts,handler_test.ts,index.ts}` — `GET /sync-pull`.

Every `_shared/` file this plan creates is prefixed `sync_`, because `_shared/entitlement.ts`, `crypto.ts`, `db.ts`, `http.ts`, `auth.ts`, `scrub.ts`, `guarded_fetch.ts` and `stripe.ts` are C1's and `_shared/judge_*.ts`, `google_*.ts` are C2's.

### New — the device (engine)

- `engine/src/entitle.rs` — the entitlement gate: the app's cache, read by the engine, and the five commands it stands in front of. One module, three functions, no network.
- `engine/tests/sync_contract.rs` — the loopback contract tests: every request shape the device sends, every reply it tolerates, and the data-minimisation assertions that still apply (no session token on the wire, no app-data path in a line, no vault path in an error).
- `engine/tests/sync_replay.rs` — the round trip: a fixture vault's notes and journal are pushed into a loopback server, pulled back into a temp directory, and compared **byte for byte**.
- `engine/tests/entitlement_gate.rs` — the gate's own suite, including the test that pins the engine's grace to `app/src/account.rs`'s.

### Modified — the device

- `engine/src/sync.rs` — **rewritten from its first line down**. After Task 2 it is the module doc and `SyncError`; Tasks 5–7 and 9 fill it with the cursor, the push builder, the pull reader, the applier, the restorer and the command body. It reaches the network only through C2's `cloudmodel::CloudClient`.
- `engine/Cargo.toml`, `Cargo.lock` — `ring` and `base64` out, `sha2` in (Task 2).
- `engine/src/lib.rs` — `pub mod sync;` is already on the branch (`baa5f0c`); `pub mod entitle;` is added and `pub mod history;` removed (hand-off **H3**).
- `engine/src/main.rs` — the `Sync` subcommand (hand-off **H4**) and the entitlement gate in front of the slot arms (hand-off **H4**, same file, two tasks).
- `engine/src/ingest.rs` — Task 11: the vault's `ics_url` stops being required (hand-off **H7**).
- `engine/src/cli.rs` — verified at Task 11, not edited (hand-off **H5**).
- `engine/src/runs.rs` — untouched; `git_sha` stays (fidelity ledger).
- `engine/tests/dependency_boundary.rs` — the crypto test replaced (Task 2).
- `engine/tests/no_console.rs` — its `with_spawns >= 3` floor becomes `>= 2` when `history.rs` goes, with the reason in the assertion message (Task 10).
- `engine/tests/site.rs` — the `PRIVACY` const and one new case (Task 11).

### Modified — the app

- `app/src/state.rs`, `app/src/commands.rs` — the `sync` status field, then git out (hand-off **H9**).
- `app/src/scheduler.rs` — `sync` becomes the slot's first step; the two git `sync_step` calls and the `has_remote` gate go; `ics_state` gains a cloud arm (hand-off **H8**).
- `app/src/main.rs` — the startup thread deleted (hand-off **H10**). **The two `generate_handler!` lists are not touched**, and H10 says so out loud so the controller does not go looking for a recount.
- `app/src/onboarding.rs`, `app/src/scaffold.rs`, `app/src/lms_link.rs` — the wizard's fill-from-the-account block, then the two URLs (hand-off **H11**).
- `app/static/index.html`, `app/static/console.js`, `app/static/console.css` — the rewritten sync line, the picker's backup-restore link (P3), the `PRIVACY` var. **C3′'s own files.**
- `app/tests/{commands,scaffold,scheduler,static_assets,onboarding}.rs`, `app/tests/no_git.rs` (new) — **C3′'s own files.**
- `site/privacy.html` — Task 11's copy change, under P1. **C3′'s own file.**
- `cloud/supabase/config.toml` — two `[functions.<name>]` entries (hand-off **H2**).
- `cloud/supabase/functions/account/index.ts` — the purge list (hand-off **H1**).
- `cloud/supabase/migrations/migrations_test.ts` — the corpus-wide pins, **in task**, under R-C3-exec-4.
- `.github/workflows/ci.yml` — verified, not edited (hand-off **H15**).

### Deleted

- `engine/src/history.rs` (707 lines, with its own test module) — Task 10, under **P2**.
- `cloud/supabase/functions/_shared/sync_vectors.json`, `cloud/supabase/functions/_shared/sync_envelope_test.ts` — Task 2, with the envelope.

---

## Interfaces with C1 and C2

**Eight contracts, checked against the checkout on 2026-09-17.** Use these spellings verbatim; changing one is a conversation, not an edit. Contract 3 of the superseded plan — the sync key credential — is **gone**, and this stream adds no credential of any kind.

**1. `config/cloud.yaml`** — written into the vault by C1's `scaffold::create_vault` at onboarding, absent on a vault that has never signed in. Exactly four keys, in this order:

```yaml
api_base: 'https://<ref>.supabase.co/functions/v1'
anon_key: '<the project anon key — public>'
session_credential_target: 'knowlu/<profile_id>/session'
account_id: '<uuid>'
```

C3′ **adds no fifth key**. `entitle::profile_id` derives the profile id from `session_credential_target` — the middle segment of `knowlu/<profile_id>/session` — which is a documented derivation and not a new field: C1's own `account::auth_base` derives the auth base from `api_base` for exactly this reason ("adding a fifth would be a conversation"). The file is read through `cloudmodel::load`, which uses `pystr::read_text` + `serde_yaml_ng`, never by comparing bytes.

**2. The session credential is a JSON object, not a bare JWT.** Windows Credential Manager, target `knowlu/<profile_id>/session`. `UserName` is the `account_id`; the blob is `{"access_token","refresh_token","expires_at","email"}`, `expires_at` in Unix seconds. **Refresh is C1's job** (`account::valid_access_token_at` refreshes at fewer than 120 seconds remaining and rewrites the entry). C3′, like C2, **only reads it**, through C2's `cloudmodel::resolve`, and tolerates an expired one by reporting `no session` and waiting for the app's next slot.

**3. The entitlement cache — C1's file, read by the engine for the first time.** `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\entitlement.json`, written by `app/src/account.rs`:

```rust
pub struct EntitlementCache { pub status: String, pub current_period_end: Option<String>, pub plan: Option<String>, pub checked_at: String }
pub const GRACE: std::time::Duration = std::time::Duration::from_secs(72 * 60 * 60);
pub fn cache_path(data_dir: &std::path::Path) -> std::path::PathBuf;   // data_dir.join("entitlement.json")
pub fn decide(cloud_configured: bool, cache: Option<&EntitlementCache>, now: jiff::Timestamp) -> EntitlementState;
pub enum EntitlementState { Entitled, NotEntitled, NoAccount, Unreadable }
```

The bytes are `ledger::dumps_value`'s, so the engine reads them with `serde_json` and gets the same four fields. **The app refreshes it; the engine only reads it** — an engine that refreshed would race the app's own six-hourly refresh and could spend a token the app is about to rotate. `state::app_data_root()` is the app's one place the path is decided and it is `%LOCALAPPDATA%\knowlu`; `entitle::app_data_root_in(base)` is the engine's seam for the same path, so every test passes a temp directory and nothing reads a real profile.

**4. `cloudmodel::CloudClient` — C2's, reused, and C3′ adds no second HTTP client.**

```rust
pub struct CloudConfig { pub api_base: String, pub anon_key: String, pub session_credential_target: String, pub account_id: String }
pub fn load(vault: &Path) -> Option<CloudConfig>;
pub enum Unavailable { NoConfig, NoSession(String) }           // .label() -> &'static str
pub fn resolve(vault: &Path) -> Result<CloudClient, Unavailable>;
pub enum CloudError { Transport(String), Status { code: u16, detail: String }, Body(String), Quiet(QuietReason) }
                                                               // .label() -> &'static str, .fatal() -> bool
impl CloudClient {
    pub fn new(cfg: &CloudConfig, token: &str) -> CloudClient;
    pub fn account_id(&self) -> &str;
    pub fn post(&self, path: &str, body: &Value) -> Result<Value, CloudError>;
    pub fn get(&self, path: &str) -> Result<Value, CloudError>;
}
pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);
```

`post` serialises through `ledger::dumps_value`, sends `Authorization: Bearer <the session's access token>` and `apikey: <anon key>`, and scrubs the bearer out of every error string it produces. C3′ calls `post("/sync-push", …)` and `get("/sync-pull?…")` and nothing else. `CloudError::label()` maps 401 to `no session`, 402 to `no entitlement`, 403 to `not allowed`, 429 to `rate limited`, a transport failure to `no network`, anything else to `the service refused` — the same words the `sync` step prints, so a student reads the same sentence whichever cloud step hit it first.

**5. `cloud/supabase/functions/_shared/entitlement.ts`** — C1's, imported by both C3′ functions and called first:

```ts
export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }>;
```

It **throws a `Response`** — 401 on a missing or invalid bearer, 402 when the account has no `active` or `trialing` entitlement — and so does `_shared/http.ts`'s `fail(status, message)`, which every validator in this stream uses.

**Nothing catches it inside a `handle`, and that is C1's shape, not an omission** (review finding B2). C1's own `telemetry/handler.ts` and `account/handler.ts` let a thrown `Response` propagate out of `handle` to `asResponse(e)` in `index.ts`, which returns it verbatim. **The consequence is a rule for every handler test in this plan**: a case that expects a refusal must await the rejection, exactly as C1's suite does at `cloud/supabase/functions/telemetry/handler_test.ts:79` —

```ts
const res = await handle(req, deps()).catch((e) => e as Response);
```

— and every such case in Tasks 3 and 4 is written that way. A 402 on `/sync-push` is a named skip on the device (`sync (skipped: no entitlement)`), exit 0, and nothing is lost: the records are still in the journal and go up at the next slot after the subscription resumes.

**6. `cloud/supabase/functions/_shared/db.ts`** — C1's PostgREST client, used verbatim:

```ts
export interface Rest { url: string; serviceKey: string; fetch: typeof fetch }
export async function restSelect<T>(rest: Rest, table: string, query: string): Promise<T[]>;
export async function restSelectAll<T>(rest: Rest, table: string, query: string, pageSize?: number): Promise<T[]>;
export async function restUpsert(rest: Rest, table: string, rows: unknown[], onConflict?: string): Promise<void>;
export async function restPatch(rest: Rest, table: string, query: string, patch: unknown): Promise<void>;
export async function restDelete(rest: Rest, table: string, query: string): Promise<void>;
export async function authGetUser(rest: Rest, token: string): Promise<AuthedUser | null>;
export function restFromEnv(): Rest;
```

Two things this constrains: **PostgREST caps a select at `config.toml`'s `max_rows = 1000`**, so `/sync-pull` pages with an explicit `limit` and never relies on the default; and **the service role bypasses RLS**, so `account_id=eq.<id>` in every query string *is* the access control, and Task 12 scans for it.

**And one thing that is simpler than it was.** The superseded plan needed two raw `rest.fetch` calls because the key generation's compare-and-set wanted `Prefer: resolution=ignore-duplicates` and `return=representation`, and `db.ts`'s helpers hard-code their `Prefer`. **There is no generation any more**, so `sync_db.ts` is C1's five helpers and nothing else — Task 12's scan asserts **zero** raw fetches, which is a stronger statement than the two the superseded plan had to allow.

**7. `cloud/supabase/functions/_shared/http.ts`** — C1's, used verbatim: `json(status, body)`, `fail(status, message)` (thrown at the call site), `methodNotAllowed(allowed)`, `subPath(url, fnName)`, `readJson<T>(req, limit?)` (throws 413 over the cap, 400 for non-JSON), `asResponse(e)`. Every error body in this codebase is `{"error": "<one sentence>"}`.

**8. `app/src/scheduler.rs`'s slot, as C1 and C2 left it.**

```rust
pub enum JudgePlan { Cloud { log_dir: PathBuf }, Local(JudgeArgs), Skip(&'static str) }
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)>;
pub enum IcsState { Feed, NoUrl, Unreadable }
pub fn ics_state(vault: &Path) -> IcsState;
pub fn has_ics_url(vault: &Path) -> bool;
```

producing `coursework → [ingest] → [judge] → rank`, each the sibling `knowlu-engine.exe`. C3′ prepends **one** step — `sync` — and changes nothing else about the function's shape (hand-off **H8a**). `run_slot_inner` names a step by `args[0]`, so one `sync` step is one row in the Runs view; the two-step `sync pull … sync push` shape was considered and rejected for exactly that reason, and because the pull is the half that has to precede `rank`.

---
## Controller hand-offs

Every change below is outside C3′'s file ownership. **No task in this plan edits these files.**

**How and when they are applied — the controller's rule, inherited from the superseded plan's ruling R-C3-8 (review findings I8 and I9).** Every hand-off in this section lands **on the branch, at the task that first needs it, as that task's own separate commit**, applied **verbatim from the hand-off text** by the controller and reviewed together with the task. It is not a merge-time activity and there is no mid-branch round trip to wait for: the implementer finishes the task, names the hand-off in its report, and the controller's commit follows immediately. Nothing in this plan is ever red for more than that one commit, and **no task in this plan may be reported green while a hand-off it names is unapplied**.

Each entry carries **"applied at Task N"**. The order is the task order:

| Task | Hand-off applied with it |
|---|---|
| 1 | **H1** — `account/index.ts`'s purge list (Task 1's last test is red until it lands) |
| 3 | **H2** — `config.toml`'s two `[functions.…]` entries (nothing deploys without them) |
| 7 | **H4a** — `engine/src/main.rs`'s `Sync` subcommand; **H8a** — `scheduler.rs`'s slot step; **H9a** — `ConsoleState.sync` and `commands::sync_inner` |
| 8 | **H3a** — `engine/src/lib.rs`'s `pub mod entitle;` (**compile-blocking**); **H4b** — `engine/src/main.rs`'s entitlement gate |
| 9 | **H11a** — `app/src/onboarding.rs`'s fill-from-the-account block |
| 10 | **H3b** — `lib.rs`'s `history` removal; **H8b** — `scheduler.rs`'s git removal; **H9b** — `state.rs`/`commands.rs`'s git removal; **H10** — `app/src/main.rs`'s startup thread (all four **compile-blocking together**, and they are the one non-buildable intermediate state in this plan) |
| 11 | **H11b** — `scaffold.rs` and `lms_link.rs`; **H12** — `site/index.html`; **H5**, **H7**, **H8c** — verified, not edited |
| 12 | **H13** — the two `scripts/*.py` checkers, verified; **H14** — the docs; **H15** — `ci.yml`, verified |

`H3`, `H4`, `H8`, `H9` and `H11` are split into lettered parts because their halves are needed at different tasks; each part is a separate commit and each is written out in full below.

### H1 — `cloud/supabase/functions/account/index.ts`, the purge list (applied at Task 1)

The branch already carries the superseded plan's version of this hand-off (`87eb58d`). **Three names, not four**: `sync_generation` is dropped by Task 1's migration, and a purge naming a table that does not exist is a 404 from PostgREST on every account deletion. Replace the C3 comment and the four names with:

```ts
            // C3′ (cloud design, amendment 2026-09-17, ruling 2): the account's own copy of the
            // student's vault — the records and the note text, readable by this service and
            // encrypted at rest. The foreign key already cascades from `accounts`, but this list is
            // what the privacy policy's deletion paragraph is written from, so a table that holds
            // the student's data is named here whether or not the cascade would also reach it.
            // `sync_usage` is the account's byte counter: it holds no content and the same argument
            // applies. `sync_generation` is gone with the device key it named.
            "sync_records",
            "sync_notes",
            "sync_usage",
```

`exportAll` is **not** extended here. It should be — with the folder export struck (ruling 2) `GET /account/export` is now the only way a student takes their data elsewhere — but that is a change to the body of C1's own export function rather than to a list, and it is recorded in *What is NOT in this plan* and in Task 12's production notes with the exact shape rather than smuggled into a hand-off.

### H2 — `cloud/supabase/config.toml` and `cloud/supabase/deno.json` (applied at Task 3)

Two `[functions.<name>]` entries appended after C1's eight and C2's eleven. `verify_jwt = false` for the same reason both give: each handler calls C1's `requireActiveEntitlement` first and answers 401 or 402 **in our shape**, which is what `CloudError::label()` maps to `no session` / `no entitlement`; Supabase's gateway rejection is a different body with a different status and would make that contract untestable.

```toml
[functions.sync-push]
verify_jwt = false
[functions.sync-pull]
verify_jwt = false
```

`deno.json` needs **no new import-map line**: C3′ imports only `@std/assert` (already there) and the platform's own `crypto.subtle`. `cloud/supabase/functions/_shared/config_toml_test.ts` derives its expectation from the function directories and asserts `deployed.length >= 19`, so it goes green on this edit with no pin to bump — checked, and stated so nobody looks for one.

### H3a — `engine/src/lib.rs`, the addition (applied at Task 8; **compile-blocking**)

`pub mod sync;` is already on the branch (`baa5f0c`) and its comment is rewritten by Task 2, which owns the file's body, not its declaration. **What this hand-off adds is the gate's module**, immediately after `pub mod sync;`:

```rust
// Knowlu C3′ — the entitlement gate (cloud design, amendment 2026-09-17, ruling 3). The app caches
// `GET /entitlement` with a 72-hour grace; past it, the four cloud slot steps refuse to run and say
// so in one line at exit 0. Reads the app's cache, never the network, and never writes it.
pub mod entitle;
```

### H3b — `engine/src/lib.rs`, the removal (applied at Task 10)

**Delete** these four lines:

```rust
// Git as a transport for the console's sync loop (Knowlu plan 1, Task 5). The journal is the
// real history; this module only drives `git` — status, commit by name, push/rebase, the lock.
pub mod childproc;
pub mod history;
```

replacing them with:

```rust
// `childproc` outlives `history`: `runs::git_sha` and `runtime.rs` both spawn children, and a GUI
// application's child process must never flash a console window.
pub mod childproc;
```

### H4a — `engine/src/main.rs`, the `Sync` subcommand (applied at Task 7)

One import change and one subcommand. In the `use` block, `knowlu_engine::{cli, coursework, enrich, ingest, journal, runs}` becomes `knowlu_engine::{cli, coursework, enrich, ingest, journal, runs, sync}`. Then, **between** the `Judge { … }` and `Runs { … }` variants:

```rust
    /// Send this device's new journal records and changed note text to the account, and apply what
    /// another desktop of the same account wrote.
    ///
    /// Always exits 0: no account, no session, no entitlement and no network are all normal
    /// outcomes (cloud design §5.5 as amended 2026-09-17), and a non-zero exit here would put the
    /// app's scheduler into retry backoff and paint the tray amber for a student on a train.
    Sync {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// pull | push | both. The slot runs `both`; the console's Sync now runs `both`; the two
        /// halves are separable for a smoke test and for a restore that must not push.
        #[arg(long, default_value = "both", value_parser = ["pull", "push", "both"])]
        direction: String,
        /// Without these the run's writes journal as `via: cli, run_id: null` — indistinguishable
        /// from someone typing the command by hand.
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
    },
```

and, in `main`'s `match`, **between** the `Command::Judge` and `Command::Runs` arms:

```rust
        Command::Sync { vault, direction, via, run_id } => {
            // Always SUCCESS: `sync::run_lines` only ever returns 0, and this arm says so out loud
            // rather than mapping a code that cannot occur.
            let (_, lines) = sync::run_lines(
                &vault,
                sync::Direction::parse(&direction).unwrap_or(sync::Direction::Both),
                &via,
                run_id.as_deref(),
            );
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
```

### H4b — `engine/src/main.rs`, the entitlement gate (applied at Task 8)

`knowlu_engine::{cli, coursework, enrich, ingest, journal, runs, sync}` becomes `knowlu_engine::{cli, coursework, enrich, entitle, ingest, journal, runs, sync}`. Then one function above `fn main`:

```rust
/// Which commands the entitlement gate stands in front of — ruling 3 of the cloud design's
/// amendment of 2026-09-17: *"The engine refuses to run a slot without a valid entitlement past the
/// 72-hour grace the app already caches."*
///
/// **The four cloud steps, and deliberately not the others.** `surface` is what the console reads
/// on every poll and `write` is what the console's own edits go through: gating either would freeze
/// the window rather than the subscription, which is not what ruling 3 is for. `runs`, `info`,
/// `issues` and `coursework-discover` are the same argument.
///
/// **`rank` is not here, and that is precondition P5.** Ruling 3's own reason — *"an orphaned binary
/// ranks a hand-made folder and nothing else"* — is satisfied by gating the four steps that fill the
/// folder; §5.1, which the amendment does not mark, promises that past the grace "the slots keep
/// ranking" and the page never blanks. If Quinn rules the other way, `Command::Rank { vault, .. }`
/// joins the pattern below and §5.1 is amended in the same commit. That is the whole of answer (b).
fn gated_vault(command: &Command) -> Option<&PathBuf> {
    match command {
        Command::Coursework { vault, .. }
        | Command::Ingest { vault, .. }
        | Command::Judge { vault, .. }
        | Command::Sync { vault, .. } => Some(vault),
        _ => None,
    }
}
```

and, in `main`, **immediately before** the `match cli.command { … }` that follows the `Cli::parse()`:

```rust
    // A refusal is a named line at exit 0, never a failure: a non-zero exit sets
    // `RunSummary.engine_ok = false`, which is retry backoff and an amber tray twice a day for a
    // student whose card simply expired — and retrying fixes nothing here.
    if let Some(vault) = gated_vault(&cli.command) {
        if let Some(line) = entitle::gate(vault) {
            println!("{line}");
            return ExitCode::SUCCESS;
        }
    }
```

### H5 — `engine/src/cli.rs` (verified at Task 11; no edit)

C2's own hand-off H4 routes a `cloud:<name>` calendar url to `/ingest-calendar?name=<name>` and every other url to `calfeed::fetch_ics`; it is **merged and live** (`engine/src/cli.rs:241-250`). C3′ needs nothing new there — the change is that the vault now *carries* `cloud:personal` where it used to carry an address. **The hand-off is a no-op with a note**, recorded so the controller does not go looking for one. Task 11 verifies the branch exists and says so in its report.

### H7 — `engine/src/ingest.rs` (verified at Task 11; no edit, and this corrects the superseded plan)

The superseded plan's H3 rewrote `ingest::run_lines`'s blank-`ics_url` refusal so a cloud vault exits 0 when the service cannot be reached. **C2's own final review did it first** (A-1, the `H5 fix`): `engine/src/ingest.rs` now moves the empty-URL refusal *after* the cloud attempt, and `cloud_ics_failure_with_no_local_url` answers a 404 carrying `no lms_ics source for this account` with `(0, ["ingest: no LMS feed on this account — skipped"])` and any other failure with exit 1 and an honest sentence. That is exactly what C3′ needs and it is already on `main`. **No edit.** Task 11 asserts the behaviour from the outside rather than assuming it, and its report records that the superseded plan's H3 was made redundant by a C2 fix rather than dropped.

### H8a — `app/src/scheduler.rs`, the slot's first step (applied at Task 7)

In `slot_argv`:

```rust
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    let mut steps = vec![(exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
```

becomes

```rust
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    // **`sync` first** (C3′, cloud design §5.5 as amended). The pull is the half that has to precede
    // `rank`: a field another desktop set this morning must be in the note before the day is
    // ordered, or every second desktop ranks a slot behind forever. The push then carries
    // everything written since the last sync — the console's own edits and the previous slot's
    // machine writes. One step, not two, because `run_slot_inner` names a step by `args[0]` and two
    // rows both reading `sync` would say less than one row does. It always exits 0.
    let mut steps = vec![(exe.to_path_buf(), vec!["sync".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
    steps.push((exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
```

### H8b — `app/src/scheduler.rs`, git out (applied at Task 10)

**The two git sync steps go.** In `run_slot_inner`, delete

```rust
    // The pull decision below reads `cs.history.has_remote` — refresh it first, since
    // `ConsoleState::open` starts it at the all-false default and neither `--run-slot-once` nor a
    // tick thread's very first iteration is guaranteed to have refreshed it yet (review item 1).
    state::refresh_history(cs);
```

and

```rust
    if lock(&cs.history).has_remote {
        steps.push(sync_step(cs, "pull"));
    }
```

and the `steps.push(sync_step(cs, "push"));` at the end of the same function, and the whole of `fn sync_step` above it, and the trailing `state::refresh_head(cs); state::refresh_history(cs);` pair. In `spawn`'s housekeeping loop, delete the two debounced `state::run_sync(&cs)` calls and the `lock(&cs.history).has_remote` gate around the second:

```rust
            if !slot_running && n % 30 == 0 && lock(&cs.history).has_remote && cs.auto_sync.load(Ordering::SeqCst) {
                let _ = state::run_sync(&cs);
            }
```

goes whole, and the `due_write` block above it keeps its backup and loses its `run_sync` line. The slot's own `sync` step is the only sync the scheduler runs, and the console's *Sync now* button is the only other caller.

### H8c — `app/src/scheduler.rs`, `ics_state` (verified at Task 11; no edit, and this corrects the superseded plan too)

The superseded plan's H7c added an `IcsState::Cloud` arm so a cloud vault with no `ics_url` would still run `ingest`. **C2's A-2 fix did it first**, and better: `pub fn ingest_included(vault: &Path) -> bool { has_ics_url(vault) || vault.join("config").join("cloud.yaml").is_file() }` is the one predicate both `slot_argv` and `run_slot_inner`'s skip line read, so a cloud vault already runs `ingest` whatever `ics_state` says and never gets a fake skip row beside the real step. **No edit, and no fourth `IcsState` variant** — a fourth state with no decision behind it would be a name for something already decided elsewhere. Task 11 asserts `ingest_included` from a test rather than assuming it.

### H9a — `app/src/state.rs` and `app/src/commands.rs`, the sync status (applied at Task 7)

**Additive, and nothing is removed here** — Task 7 needs somewhere to put the last sync's result, and Task 10 is what takes git out.

**`app/src/state.rs`.** One field on `ConsoleState`, beside `backup`:

```rust
    /// What the last sync did, for the page's sync line. Filled by `commands::sync_inner` and by
    /// the slot's own sync step; never computed in `commands.rs` (console spec §3.1). The type is
    /// the ENGINE's, because the engine is what produces it and a second struct in the app would be
    /// a second thing to keep in step with the run that fills it.
    pub sync: Mutex<knowlu_engine::sync::SyncStatus>,
```

initialised in `ConsoleState::open`, beside the `backup` initialiser, as:

```rust
            sync: Mutex::new(knowlu_engine::sync::SyncStatus::default()),
```

and one function beside `run_backup`:

```rust
/// Runs the engine's sync in-process and records what it did. **Takes `vault_io`, never `lock`**:
/// a pull writes notes and can file a card, so it must not run under the console's read lock, and
/// `commands::sync_inner` takes `lock` only afterwards to rebuild `state`.
pub fn run_sync(cs: &ConsoleState) -> knowlu_engine::sync::SyncStatus {
    let (_, lines, totals) = {
        let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
        knowlu_engine::sync::run_lines_with(&cs.vault, knowlu_engine::sync::Direction::Both, "dashboard", None)
    };
    let status = knowlu_engine::sync::SyncStatus::of(&totals, lines);
    *cs.sync.lock().unwrap_or_else(|e| e.into_inner()) = status.clone();
    status
}
```

**`app/src/commands.rs`.** `sync_inner` (`app/src/commands.rs:264-269`) becomes:

```rust
/// Runs the engine's sync OUTSIDE `cs.lock` — a pull can take seconds and must not block a
/// concurrent `state` poll — then takes the lock only to rebuild `state`. `ok` is
/// `last_error.is_none()`, not whether anything actually moved.
pub fn sync_inner(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    let out = crate::state::run_sync(cs);
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let state = build_state_value(cs, view)?;
    Ok(json!({ "ok": out.last_error.is_none(), "error": out.last_error, "state": state }))
}
```

The `history::sync` name in the three doc comments that explain why `vault_io` exists (`commands.rs:142-143`, `:261`, `:276-277`) is re-worded to name the engine's sync instead; the lock and its ordering are unchanged, and so is `backup_now_inner`.

### H9b — `app/src/state.rs` and `app/src/commands.rs`, git out (applied at Task 10)

**`app/src/state.rs`.** Remove the `use` of `HistoryStatus`, the three fields `history`, `head_sha`, `auto_sync`, their initialisers, and the two functions `refresh_history` and `refresh_head`. In `quit_flush`, the git branch

```rust
            let has_remote = cs.history.lock().unwrap_or_else(|e| e.into_inner()).has_remote;
            if has_remote && cs.auto_sync.load(SeqCst) {
                q.synced = run_sync(cs).status.last_error.is_none();
            }
```

becomes

```rust
            // A quit flush pushes what is already on disk; it never pulls. A pull applies writes and
            // can file cards, and doing that while the window is closing would show the student a
            // deck they never saw change.
            if knowlu_engine::sync::is_configured(&cs.vault) {
                let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
                let (_, _, totals) =
                    knowlu_engine::sync::run_lines_with(&cs.vault, knowlu_engine::sync::Direction::Push, "dashboard", None);
                q.synced = totals.errors.is_empty();
            }
```

**`app/src/commands.rs`.** Remove the three git-derived topline keys in `build_state_value`:

```rust
    let head = cs.head_sha.lock().map_err(|_| "lock")?.clone();
    v["topline"]["console_build"] = json!(CONSOLE_BUILD);
    v["topline"]["vault_head"] = json!(head);
    v["topline"]["engine_newer"] = json!(matches!((CONSOLE_BUILD, head.as_deref()), (Some(c), Some(h)) if c != h));
```

becomes

```rust
    // `vault_head` and `engine_newer` went with git (C3′, Task 10): they compared the VAULT's git
    // HEAD against this build, and a vault has not been a git repository since §4.1. The console's
    // own build stays — the diagnostics blob and the issue report both name it.
    v["topline"]["console_build"] = json!(CONSOLE_BUILD);
```

the sync line's source

```rust
    v["topline"]["sync"] = serde_json::to_value(&*cs.history.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())?;
```

becomes

```rust
    v["topline"]["sync"] = serde_json::to_value(&*cs.sync.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())?;
```

and `auto_sync` goes:

```rust
    v["topline"]["auto_sync"] = json!(cs.auto_sync.load(std::sync::atomic::Ordering::SeqCst));
```

is deleted — it existed to record "a merge conflict turned auto-sync off", and there are no merge conflicts any more: a conflict is an amend card and the next sync runs normally.

### H10 — `app/src/main.rs`, the startup thread (applied at Task 10)

**The gap the superseded plan's review found as B5.** `app/src/main.rs:169-177`, inside `run_console`'s `setup`:

```rust
            // Both git calls (`refresh_head`, and `refresh_history`'s ahead/behind check) belong
            // off the UI thread (F17) — spawned once at startup; Task 12's housekeeping thread
            // repeats both every 60 s.
            let h = app.handle().clone();
            std::thread::spawn(move || {
                let cs = h.state::<ConsoleState>();
                knowlu::state::refresh_head(&cs);
                knowlu::state::refresh_history(&cs);
            });
```

**Delete the whole block, comment included.** Both functions leave with H9b, the thread has nothing else to do, and `no_git_process_is_spawned_for_a_vault` scans every file under `app/src/` — this one included. Nothing replaces it: `ConsoleState::open` already starts `sync` at its default, and the first slot's own sync step fills it.

**The two `generate_handler!` lists are NOT touched by this stream.** C3′ adds no Tauri command: ruling 2 removed the switch, the key, the recovery code and the export that the superseded plan's five commands existed to drive. The counts in `CLAUDE.md` and `app/README.md` stay **43 console / 30 vault-less / 62 distinct**, and hand-off **H14** says so rather than recounting to the same number by accident.

### H11a — `app/src/onboarding.rs`, the wizard fills the mirror from the account (applied at Task 9)

`create_vault_in`, immediately after the `move_session` block and before `finish_or_roll_back`:

```rust
    // **The restore, and it is not a route — it is what Finish does** (cloud design, amendment
    // 2026-09-17, ruling 2: "restoring is signing in on a new desktop; the mirror fills from the
    // account"). There is no code to type and no link on the picker: a student who already has a
    // vault in their account gets it here, and one who does not gets the nine-panel wizard's own
    // seeds and notices nothing.
    //
    // **Placed after `scaffold::create_vault` and after `move_session`, and that is deliberate.**
    // The vault has to exist (its `config/cloud.yaml` is what `restore_into` reads) and the session
    // has to be on this profile (the pull needs a bearer). `scaffold` has therefore already seeded
    // `archive/_migrated.md`, `tasks/get-to-know-knowlu.md` and one `courses/<slug>.md` per course —
    // which is exactly why `restore_into` computes its allowlist from what is on disk right now
    // rather than demanding an empty folder.
    let restored = match knowlu_engine::sync::restore_into(&dest) {
        // **An empty copy keeps the vault.** A student signing in on their first desktop, or on a
        // second one before the first has ever pushed, has made a perfectly good vault; rolling it
        // back would throw away a nine-panel wizard run to tell them, accurately and uselessly,
        // that there was nothing to restore. The finish panel says so instead.
        Ok(r) => r,
        // A copy that will not read IS a failure of this path: the student asked for their vault and
        // a half-filled folder is worse than none. A network that is simply down is NOT this arm —
        // `restore_into` reports that as an empty result with a warning, because a first slot will
        // fill the folder anyway and refusing to make a vault over a hotel Wi-Fi is the wrong trade.
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dest);
            return json!({ "ok": false, "error": e, "profile": Value::Null });
        }
    };
```

and `finish_or_roll_back`'s returned envelope gains one key, so the page can say what happened:

```rust
    out["restored"] = json!({ "notes": restored.notes, "records": restored.records, "empty": restored.empty });
```

**`WizardPlan` gains no field.** The superseded plan's `restore: bool` existed because the picker offered a route; there is no route now, and a flag whose only value is `true` is a flag nobody reads.

### H11b — `app/src/scaffold.rs` and `app/src/lms_link.rs` (applied at Task 11)

**`app/src/scaffold.rs`.** In `ingest_yaml`, the two lines that write a capability URL into the vault become account-aware. The LMS feed:

```rust
    if let Some(u) = &p.ics_url { s.push_str(&format!("ics_url: {}\n", yaml_scalar("LMS feed URL", u)?)); }
```

becomes

```rust
    // C3′, Task 11 (cloud design §9, Alabama SPII): a capability URL is a credential in all but
    // name, and on a vault that has an account the one place it belongs is the account, encrypted,
    // where `PUT /account/sources` already put it. The KEY stays, empty, so `ingest` still parses
    // the file and so a reader can see the feed is elsewhere rather than missing.
    if !p.account_id.is_empty() {
        s.push_str("ics_url: ''\n");
    } else if let Some(u) = &p.ics_url {
        s.push_str(&format!("ics_url: {}\n", yaml_scalar("LMS feed URL", u)?));
    }
```

and the personal calendar:

```rust
    if let Some(u) = &p.personal_calendar {
        entries.push(format!("  - name: personal\n    ics_url: {}\n", yaml_scalar("personal calendar address", u)?));
    }
```

becomes

```rust
    if p.personal_calendar.is_some() {
        if p.account_id.is_empty() {
            let u = p.personal_calendar.as_deref().unwrap_or_default();
            entries.push(format!("  - name: personal\n    ics_url: {}\n", yaml_scalar("personal calendar address", u)?));
        } else {
            // The same removal and the same argument; `cloud:personal` is C2's own routing
            // (`cli.rs:241-250`) and resolves to `/ingest-calendar?name=personal`.
            entries.push("  - name: personal\n    ics_url: 'cloud:personal'\n".to_string());
        }
    }
```

**`app/src/lms_link.rs`** — one doc sentence. `put_source_at`'s comment says the link is stored twice; `lms_link` itself only ever writes the **account** copy, and the vault copy travelled `WizardPlan.ics_url` → `VaultPlan.ics_url` → `scaffold::ingest_yaml`, which is where it stops above. Replace the "stored twice" sentence with:

```rust
/// **The account is the only writer of this URL from C3′ on** (amendment 2026-09-17, ruling 2, and
/// §9's SPII line). `scaffold::ingest_yaml` writes `ics_url: ''` into a vault that has an account;
/// a vault with no account still keeps its own copy, because it has nowhere else to keep it.
```

### H12 — `site/index.html`, the second copy of the privacy sentence (applied at Task 11)

The sentence Task 11 rewrites lives in **four** places: `site/privacy.html:13`, `site/index.html:16`, `app/static/console.js:1371`'s `var PRIVACY` and `engine/tests/site.rs:15`'s `const PRIVACY` — and `engine/tests/site.rs:25-26` asserts it in both site pages:

```rust
    assert!(privacy.contains(PRIVACY), "the privacy page carries the exact sentence");
    assert!(index.contains(PRIVACY), "so does the download page");
```

`site/index.html` is C1's file and is not in C3′'s ownership, so the controller applies this one: **replace the sentence at `site/index.html:16` with the same new sentence Task 11 step 3 writes into `site/privacy.html:13`, character for character.** `engine/tests/site.rs`, `app/static/console.js` and `site/privacy.html` are C3′'s and Task 11 edits them directly; this entry exists only so the two halves of one sentence can never move apart, and the controller's commit for it and Task 11's own commit are reviewed together.

### H13 — `scripts/wizard-check.py` and `scripts/settings-check.py` (verified at Task 12; no edit)

**Nothing to add, and that is the point.** The superseded plan extended `BEFORE_FINISH_OK` with `stage_restore_key` and taught the settings fake three new commands; C3′ adds no Tauri command, no wizard route and no settings row, so both scripts describe the app exactly as they already do. Task 12 **runs** `python scripts/wizard-check.py` (expect `ok`, exit 0) and `python scripts/settings-check.py`, and records both in the report. If either fails, a task has changed the page in a way this plan did not intend — stop and report it.

### H14 — `CLAUDE.md`, `HANDOFF.md`, `README.md`, `app/README.md`, `VISION.md` (applied at Task 12)

Applied at the close, with the exact text in Task 12 step 6. In summary: `CLAUDE.md`'s engine-command list gains `sync` and the entitlement-gate sentence; its `A slot is coursework → ingest → judge → rank` becomes `sync → coursework → ingest → judge → rank`; its "Seven mutate notes" becomes **eight** with `sync` named; the `history.rs` mention in the app section goes; the Tauri-command counts **do not move** and the recount date does, with one clause saying C3′ added none. `app/README.md` gains a `src/sync.rs`-free note (the module is the engine's) and the `history.rs` row goes. `HANDOFF.md` gains a `▶ C3′ DONE` block. **`VISION.md` needs no change** — commitment 2 and the Sync and Mobile rows were amended on 2026-09-17 in the amendment's own commit, and this plan implements them as written.

### H15 — `.github/workflows/ci.yml` (C0's file; verified at Task 12)

**C3′ needs no edit here, and the thing to verify is the FLAGS, not the paths.** `main` carries the `cloud` job's `deno test` line as

```yaml
      - name: deno test
        run: deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/
```

because C2's own loopback and SDK tests need them. **That is the command every whole-tree run in this plan uses**, and the command Task 12's gate runs. The **path** scope needs nothing either: `deno lint` and `deno test` walk `cloud/supabase/` whole and `deno check` globs every `.ts` under `functions/`. Verify at Task 12 rather than pre-empting: if the merged job ever names a *list* of directories instead of the tree, C3′'s test files must be added to that list.

---
## The tasks

Twelve. Tasks 1–4 are the service; 5–7 the device; 8 the gate ruling 3 asks for; 9 the restore that makes a second desktop real; 10–11 the two removals the spec ordered; 12 closes. **The first seven can be executed and reviewed without a single line of UI**, and exit-gate items 1–9 are all reachable from them.

---
### Task 1: The account's copy, in plain text

The store. Two tables holding what the amendment says they hold — the record's canonical JSON and the note's text — under RLS, with an identity column for a cursor, a content hash for idempotence, and a `keep` bit the **server** decides rather than the client.

**Files:**
- Create: `cloud/supabase/migrations/20260912000300_sync_plaintext.sql`
- Modify: `cloud/supabase/migrations_sync_test.ts` (rewritten), `cloud/supabase/migrations/migrations_test.ts` (the corpus pins only, under **R-C3-exec-4**)
- **Never edited:** `20260912000100_sync.sql`, `20260912000200_sync_usage_prune.sql` — both are applied to staging and migrations are forward-only
- **Hand-off this task needs:** **H1** (`account/index.ts`'s purge list), applied by the controller beside this task

**Interfaces:**
- Consumes: C1's `public.accounts (id)`, `sync_usage`, `sync_ceiling_bytes()`, `sync_limits`, `sync_prune(int)`, the `knowlu-sync-prune` cron job and the `public.sync_notes_rev` sequence — all from `20260912000100`/`000200`, which stay.
- Produces: `public.sync_records (account_id, seq, device, record_hash, body, keep, received_at)` and `public.sync_notes (account_id, path, rev, device, deleted, body, updated_at)`.

- [ ] **Step 1: Write the failing test** — `cloud/supabase/migrations_sync_test.ts`, replacing its body. The two tests the amendment inverts are named so a reader sees the inversion rather than a deletion:

```ts
import { assert, assertEquals } from "@std/assert";

const DIR = new URL("./migrations/", import.meta.url);

/** C3′'s own migrations, and only C3′'s. C1 filters `20260910…` and C2 filters `20260911…` from
 * their own sides (R-X-8); a helper that read the whole directory would turn someone else's suite
 * red for a reason that has nothing to do with their change. */
const MINE = /^20260912\d{6}_[a-z0-9_]+\.sql$/;

async function migrations(): Promise<{ name: string; sql: string }[]> {
  const out: { name: string; sql: string }[] = [];
  for await (const e of Deno.readDir(DIR)) {
    if (e.isFile && e.name.endsWith(".sql") && MINE.test(e.name)) {
      out.push({ name: e.name, sql: await Deno.readTextFile(new URL(e.name, DIR)) });
    }
  }
  out.sort((a, b) => a.name.localeCompare(b.name));
  return out;
}

/** Every table C3′ leaves BEHIND: created by one of its migrations and not dropped by a later one.
 * Derived, never hand-typed, so a table added or dropped later cannot be forgotten in two places. */
async function liveTables(): Promise<string[]> {
  const live = new Set<string>();
  for (const m of await migrations()) {
    for (const c of m.sql.matchAll(/create\s+table\s+(?:if\s+not\s+exists\s+)?public\.(\w+)/gi)) live.add(c[1]);
    for (const d of m.sql.matchAll(/drop\s+table\s+(?:if\s+exists\s+)?public\.(\w+)/gi)) live.delete(d[1]);
  }
  return [...live].sort();
}

Deno.test("C3′'s migrations are stamped in C3's day, and the two already on staging are untouched", async () => {
  const mine = await migrations();
  assert(mine.length >= 3, `expected the two applied migrations and C3′'s own, found ${mine.length}`);
  for (const m of mine) assert(m.name.startsWith("20260912"), `${m.name}: C3's are stamped 20260912…`);
  // A forward-only corpus: the shape correction is a NEW file, never an edit of an applied one.
  assertEquals(mine[0].name, "20260912000100_sync.sql");
  assertEquals(mine[1].name, "20260912000200_sync_usage_prune.sql");
  assertEquals(mine[2].name, "20260912000300_sync_plaintext.sql");
});

Deno.test("the account's copy is three tables: the records, the notes and the byte counter", async () => {
  // `sync_generation` held the fingerprint of the device key. Ruling 2 removed the key, so the
  // table is dropped rather than left as a row nobody writes and a name in a purge list.
  assertEquals(await liveTables(), ["sync_notes", "sync_records", "sync_usage"]);
});

Deno.test("row-level security is on for every table C3′ leaves, and none has a client write policy", async () => {
  const secured: string[] = [];
  for (const m of await migrations()) {
    for (const s of m.sql.matchAll(/alter\s+table\s+public\.(\w+)\s+enable\s+row\s+level\s+security/gi)) {
      secured.push(s[1]);
    }
    // Every `create policy` must spell `for` explicitly: `create policy p on public.x to
    // authenticated using (…)` with no `for` clause defaults to FOR ALL — a write policy — and
    // would slip past a scan that only matched the `for`-bearing shape.
    const everyPolicy = m.sql.match(/create\s+policy\s+\w+\s+on\s+public\.\w+/gi) ?? [];
    const forPolicies = [...m.sql.matchAll(/create\s+policy\s+\w+\s+on\s+public\.\w+\s+for\s+(\w+)/gi)];
    assertEquals(forPolicies.length, everyPolicy.length, `${m.name}: a 'create policy' with no explicit 'for' is a write policy`);
    for (const p of forPolicies) {
      assertEquals(p[1].toLowerCase(), "select", `${m.name}: only select policies; writes go through an edge function`);
    }
  }
  for (const t of await liveTables()) assert(secured.includes(t), `public.${t} has no 'enable row level security'`);
});

Deno.test("the rows hold the student's own text, bounded in BYTES, and every one is scoped to an account", async () => {
  // **The inversion.** Until the amendment of 2026-09-17 this file asserted that no column could
  // hold a note, a path or a title. Ruling 2 reversed that: the service holds the tasks and notes so
  // the student's desktops stay in step, readable by us, encrypted at rest, deleted with the
  // account. What is still true — and what this test now pins — is that every row belongs to exactly
  // one account, that the payload is bounded, and that the bound is in BYTES on both sides of the
  // wire (`octet_length`, not `length`: Postgres counts characters and the device counts bytes, and
  // a vault full of accented Spanish would otherwise disagree with its own cap).
  const sql = (await migrations()).at(-1)!.sql.toLowerCase();
  assert(sql.includes("account_id  uuid        not null references public.accounts (id) on delete cascade"), "records cascade from the account");
  assert(sql.includes("octet_length(body) between 2 and 16384"), "a record's body is bounded in bytes");
  assert(sql.includes("octet_length(body) between 1 and 131072"), "a note's body is bounded in bytes");
  assert(sql.includes("record_hash ~ '^[0-9a-f]{64}$'"), "the content hash is 64 hex characters");
  assert(sql.includes("device ~ '^[0-9a-f]{16}$'"), "the device token is opaque, 16 hex characters");
});

Deno.test("a note's path is checked, not trusted", async () => {
  // The path is the note's primary key now, and it is a string a client sends. Without this a
  // pushed `../../etc/hosts` would sit in the table waiting for a restore to write it.
  const sql = (await migrations()).at(-1)!.sql;
  assert(sql.includes("(tasks|approvals|archive|courses|issues|info)/"), "only the six note folders");
  assert(sql.includes("\\.md$"), "and only markdown");
  assert(sql.includes("path !~ "), "and a path that can climb out is refused by its own check");
});

Deno.test("retention never deletes a record a human wrote, and the SERVER is what decides that", async () => {
  // P3's answer (Quinn, 2026-09-17), and the half that is a correctness property rather than a
  // storage one: `journal::human_set` is what judge-once reads, and it reads records. A `sync_prune`
  // without this clause quietly costs a restored machine its attribution.
  //
  // **What the amendment changed:** the device used to assert `keep` and the server had to believe
  // it, because the row was ciphertext. The row is plaintext now, so the rule is a generated column
  // computed from the record itself — `op` in (set, create) and an actor that is not an agent, which
  // is `provenance::is_agent`'s own `starts_with("agent:")` test. A client cannot lie about it and
  // cannot forget it.
  const sql = (await migrations()).map((m) => m.sql).join("\n");
  assert(sql.includes("and not keep"), "sync_prune must exempt the records marked `keep`");
  const last = (await migrations()).at(-1)!.sql;
  assert(/keep\s+boolean\s+not null generated always as/.test(last), "`keep` is generated, not sent");
  assert(last.includes("'agent:%'"), "an agent's record is not kept");
  assert(last.includes("in ('set', 'create')"), "only a set or a create is a human decision worth keeping");
});

Deno.test("the account purge names every table C3′ leaves, and no table it dropped", async () => {
  // Hand-off H1. The foreign key cascades anyway; this list is what the privacy policy's deletion
  // paragraph is written from, and a table missing from it is a table nobody remembers to mention —
  // while a table NAMED in it that no longer exists is a 404 on every account deletion.
  const index = await Deno.readTextFile(new URL("./functions/account/index.ts", import.meta.url));
  const purge = index.slice(index.indexOf("purge:"), index.indexOf("deleteAuthUser:"));
  for (const t of await liveTables()) assert(purge.includes(`"${t}"`), `DELETE /account does not purge ${t}`);
  assert(!purge.includes("sync_generation"), "sync_generation is gone; purging it is a 404 every time");
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/migrations_sync_test.ts`
Expected: FAIL — `expected the two applied migrations and C3′'s own, found 2`, and every case after it. The purge case stays red until **H1** lands.

- [ ] **Step 3: Write the migration** — `cloud/supabase/migrations/20260912000300_sync_plaintext.sql`:

```sql
-- Knowlu C3′, Task 1 — the account's copy of its own vault, in plain text.
--
-- **This supersedes the shape 20260912000100 created, and does not edit it.** That migration is
-- applied to staging and migrations are forward-only; Quinn's amendment of 2026-09-17 (ruling 2)
-- reversed §11 R4, so the rows that were AES-256-GCM ciphertext under a device-held key become the
-- record's canonical JSON and the note's own text, readable by this service, encrypted at rest by
-- the platform, and deleted with the account. The tables held nothing — `/sync-push` was never
-- built — so they are dropped and recreated rather than altered column by column into a shape with
-- a different primary key.
--
-- What ruling 2 buys, and it is worth naming because it is the whole reason the reversal is not
-- only a loss: the student's desktops can actually stay in step (a promise the product could not
-- keep across their own machines is not trust); `keep` stops being a bit the client asserts and
-- becomes a fact the server computes; a pushed row can be checked against its own hash instead of
-- taken on faith; and nothing is lost when a laptop and a printed code are lost together.
--
-- What is NOT here, because 20260912000100/000200 already put it there and it is unchanged:
-- `public.sync_usage` and its O(1) counter, `public.sync_ceiling_bytes()` (200 MiB, precondition P3
-- as Quinn answered it on 2026-09-17), `public.sync_limits`, `public.sync_prune(int)` (400 days,
-- `and not keep`) and the `knowlu-sync-prune` cron job. `sync_prune` is plpgsql and binds its table
-- late, so it goes on working over the tables recreated below.

-- The device key is gone (ruling 2), and so is the generation its fingerprint named.
drop table if exists public.sync_generation;

-- `drop table` fires no row triggers, so the byte counter is not decremented by the two drops
-- below. On staging both tables are empty — the endpoints did not exist — and on any project this
-- keeps the counter honest rather than leaving the ceiling guarding bytes that are not there.
drop table if exists public.sync_records;
drop table if exists public.sync_notes;
update public.sync_usage set bytes = 0, updated_at = now();

-- The sequence survives both drops (it is standalone, never `owned by`), so a second desktop's
-- cursor cannot be handed a `rev` it has already seen.

create table public.sync_records (
  account_id  uuid        not null references public.accounts (id) on delete cascade,
  seq         bigint      generated always as identity,
  -- Opaque, and it is the only thing here that is. `sha256(account_id + "\n" + hostname)[..16]`,
  -- computed on the device: the server needs to know that two rows came from the same machine and
  -- has no reason ever to learn which machine that is.
  device      text        not null check (device ~ '^[0-9a-f]{16}$'),
  -- `sha256` of `body`, hex. The SERVER re-derives it and refuses a mismatch (`sync-push`), so this
  -- column is a fact about the row rather than a claim about it — which is what makes
  -- `sync_records_once` an honest idempotence key for a retried batch.
  record_hash text        not null check (record_hash ~ '^[0-9a-f]{64}$'),
  -- **The journal record, exactly as `ledger::dumps_value` wrote it**: sorted keys, Python's
  -- separators, no trailing newline. `text` and not `jsonb` on purpose — jsonb renormalises numbers
  -- and would hand back bytes that no longer hash to `record_hash`, and the canonical bytes are the
  -- contract two machines agree on. 16 KiB is far past any record that exists (a record is a handful
  -- of scalars and, for a `create`, one frontmatter mapping); `octet_length`, not `length`, because
  -- the device's cap is in bytes and Postgres's `length` counts characters.
  body        text        not null check (octet_length(body) between 2 and 16384),
  -- **Decided here, from the record, never sent.** `sync_prune` keeps a record a human wrote for
  -- ever (precondition P3, Quinn 2026-09-17): `journal::human_set` is what judge-once reads and it
  -- reads records, so a pruned human `set` on a restored machine is a decision the student made and
  -- the device can no longer see. `not like 'agent:%'` is `provenance::is_agent`'s own test.
  keep        boolean     not null generated always as (
                            (body::jsonb ->> 'op') in ('set', 'create')
                            and coalesce(body::jsonb ->> 'actor', '') not like 'agent:%'
                          ) stored,
  received_at timestamptz not null default now(),
  primary key (account_id, seq),
  constraint sync_records_once unique (account_id, record_hash)
);
comment on table public.sync_records is
  'Spec §5.5 as amended 2026-09-17. One row per journal record, in the canonical JSON the device
   wrote. Readable by this service, encrypted at rest, deleted with the account.';

create table public.sync_notes (
  account_id uuid        not null references public.accounts (id) on delete cascade,
  -- **The note''s vault-relative path, POSIX-separated, and it is the primary key.** A path a client
  -- sends is a path a restore would write, so it is checked rather than trusted: one of the six
  -- note folders, markdown, and no segment that can climb out of the vault. `ids::NOTE_FOLDERS` is
  -- the list, and `engine/src/sync.rs::is_note_path` is the same rule on the device.
  path       text        not null
                         check (path ~ '^(tasks|approvals|archive|courses|issues|info)/[A-Za-z0-9._ /-]{1,300}\.md$')
                         check (path !~ '(^|/)\.\.(/|$)')
                         check (path !~ '//'),
  rev        bigint      not null default nextval('public.sync_notes_rev'),
  device     text        not null check (device ~ '^[0-9a-f]{16}$'),
  -- A note that has been settled into `archive/` is pushed as a tombstone at its old path: the row
  -- stays, so a restore knows not to resurrect it, and carries no bytes.
  deleted    boolean     not null default false,
  -- The note''s whole text, frontmatter and body, as the file holds it. 128 KiB in bytes: a note
  -- longer than that is a pasted document, not a task, and the device refuses it by name rather
  -- than truncating it — a truncated note is a lie.
  body       text        null check (body is null or octet_length(body) between 1 and 131072),
  updated_at timestamptz not null default now(),
  primary key (account_id, path),
  constraint sync_notes_tombstone check ((deleted and body is null) or (not deleted and body is not null))
);
comment on table public.sync_notes is
  'Spec §5.5 as amended 2026-09-17, "the note text they produced". Current state, not history: one
   row per note path, upserted.';

-- A note''s row is UPSERTED — a note is current state, not history — so `generated always as
-- identity` would not move on a re-push and a second desktop would never learn the text changed.
-- The sequence plus this trigger gives every write, insert or update, a fresh cursor value.
create trigger sync_notes_rev_stamp
  before insert or update on public.sync_notes
  for each row execute function public.sync_notes_stamp_rev();

create index sync_notes_rev_idx on public.sync_notes (account_id, rev);
-- The nightly prune scans by exactly this predicate; without a matching partial index that scan is
-- a full table scan across every account, every night, forever (20260912000200's item 2, re-made
-- because the table it was on is gone).
create index sync_records_prune_idx on public.sync_records (received_at) where not keep;

-- `sync_usage_bump` is 20260912000200's fixed body and reads `new.ciphertext`/`old.ciphertext`,
-- which no longer exist. Same shape, same DELETE rule (it must never INSERT on a delete, or an
-- `accounts` cascade can abort on trigger ordering), new column name — and the name is `body` on
-- BOTH tables precisely so one trigger function serves both.
create or replace function public.sync_usage_bump() returns trigger
language plpgsql set search_path = public as $$
declare
  v_account uuid   := coalesce(new.account_id, old.account_id);
  v_delta   bigint := coalesce(octet_length(new.body), 0) - coalesce(octet_length(old.body), 0);
begin
  if (tg_op = 'DELETE') then
    update public.sync_usage set bytes = greatest(bytes + v_delta, 0), updated_at = now()
      where account_id = v_account;
  else
    insert into public.sync_usage (account_id, bytes) values (v_account, greatest(v_delta, 0))
    on conflict (account_id) do update
      set bytes = greatest(public.sync_usage.bytes + v_delta, 0), updated_at = now();
  end if;
  return null;
end;
$$;

create trigger sync_records_usage
  after insert or update or delete on public.sync_records
  for each row execute function public.sync_usage_bump();
create trigger sync_notes_usage
  after insert or update or delete on public.sync_notes
  for each row execute function public.sync_usage_bump();

alter table public.sync_records enable row level security;
alter table public.sync_notes   enable row level security;

-- Read-your-own, and nothing else. There is deliberately no client write policy on either: a push
-- goes through `sync-push`, which verified the JWT and then used the service role.
create policy sync_records_select_own on public.sync_records
  for select to authenticated using (account_id = auth.uid());
create policy sync_notes_select_own on public.sync_notes
  for select to authenticated using (account_id = auth.uid());
```

- [ ] **Step 4: Bump C1's corpus-wide pins** (**R-C3-exec-4** — the pins exist to be moved by whichever stream moves them, with the reason in the comment). In `cloud/supabase/migrations/migrations_test.ts`, the function-creation pin goes from **23** to **24** — this migration issues one more `create or replace function public.sync_usage_bump()`, still a trigger function and still exempt by kind — and the comment gains one sentence saying so. The view pin stays at **5**: `sync_limits` is untouched. The `no migration in this stream drops or truncates a table` guard reads only `20260911…` files (`ours()`), so C3′'s three `drop table` statements are outside it by construction, and C3′'s own suite is where the drops are asserted to be exactly the ones intended.

- [ ] **Step 5: Run both suites.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/migrations_sync_test.ts` → all seven pass once **H1** has landed; six pass before it.
Then the whole tree with C2's flag set (Global Constraints) → 0 failed.

- [ ] **Step 6 (controller, not the implementer — R-C3-exec-1): apply it to staging.**

```
supabase db push --include-all --workdir cloud
```

`--include-all` is **required** (R-C3-exec-5): the 2026-09-12 stamp sorts before `20260916000100`, already on the remote, and without the flag the CLI skips it in silence.

- [ ] **Step 7 (controller): read it back, six proofs**, appended to the task report:
  1. `select table_name from information_schema.tables where table_schema='public' and table_name like 'sync_%' order by 1;` → `sync_notes`, `sync_records`, `sync_usage` (three, not four).
  2. `select column_name, is_generated from information_schema.columns where table_name='sync_records' order by 1;` → `keep` is `ALWAYS`.
  3. `insert … values (…, '{"actor":"quinn","op":"set", …}') returning keep;` inside a transaction → `true`; the same row with `"actor":"agent:knowlu.enrich"` → `false`; **roll back**.
  4. `select public.sync_ceiling_bytes();` → `209715200`.
  5. `insert into public.sync_notes (account_id, path, device, body) values (gen_random_uuid(), '../../etc/hosts', …)` → refused by the path check (`23514`), and the same insert with `tasks/x.md` refused by the foreign key (`23503`) because the account does not exist. **Both refusals are the point**: the shape is checked before the ownership.
  6. `delete from public.sync_notes …` inside a transaction with a seeded account → `sync_usage.bytes` returns to `0`; **roll back**, and confirm nothing persisted.

- [ ] **Step 8: Commit.**

```bash
git add cloud/supabase/migrations/20260912000300_sync_plaintext.sql cloud/supabase/migrations_sync_test.ts cloud/supabase/migrations/migrations_test.ts
git commit -F .git-commit-msg.txt   # "cloud: the account's copy in plain text — the records, the notes and a keep bit the server decides (C3' Task 1)"
```

---

### Task 2: The envelope retires

A pure removal, and the smallest task in the plan. Everything ruling 2 struck leaves in one commit, and the one hash the new design still needs arrives in the same commit so the manifest never sits in an intermediate state.

**Files:**
- Modify: `engine/src/sync.rs` (rewritten down to the module doc and `SyncError`), `engine/Cargo.toml`, `Cargo.lock`, `engine/tests/dependency_boundary.rs`
- Delete: `cloud/supabase/functions/_shared/sync_vectors.json`, `cloud/supabase/functions/_shared/sync_envelope_test.ts`
- **No hand-off.** `pub mod sync;` is already on the branch (`baa5f0c`); only its comment moves, and the comment is inside `lib.rs`, so **Task 2 does not touch it** — hand-off **H3a** rewrites it at Task 8 when `entitle` joins it. Until then `lib.rs` carries a comment about an envelope that no longer exists, and the task report says so rather than editing a hand-off file.

**Interfaces:**
- Produces: `sync::SyncError` (five variants), `sync::MAX_RECORD_BYTES`, `sync::MAX_NOTE_BYTES`, `sync::sha256_hex`, `sync::device_token`.

- [ ] **Step 1: Write the failing test** — replace `the_crypto_dependencies_are_the_two_already_in_the_graph` in `engine/tests/dependency_boundary.rs` with:

```rust
/// The one hash the account vault needs, and the four crates that are not coming back.
///
/// **What left, and why.** Until Quinn's amendment of 2026-09-17 this engine sealed every journal
/// record and every note with AES-256-GCM under a key that lived only in Credential Manager, and
/// named `ring` and `base64` for it. Ruling 2 reversed that: the account holds the student's tasks
/// and notes readable by the service, so there is no envelope, no IV, no key and no recovery code —
/// and two direct dependencies leave with them.
///
/// **What arrived.** `sha2`, for the content hash that makes a retried push idempotent and lets the
/// server check a row against its own body. It is already resolved in this workspace's lockfile, so
/// naming it adds a direct edge and no new crate and no new version to audit — the same argument
/// the retired test made for `ring`. `sha1` stays and is a different thing: it is `ids::derived_id`,
/// a note identity contract with every existing vault, and it is not a hash for anything new.
#[test]
fn the_content_hash_is_sha2_and_the_envelope_is_not_coming_back() {
    assert!(MANIFEST.contains("sha2 = \"0.10\""), "engine/Cargo.toml must name sha2 for the sync content hash");
    for (name, manifest) in MANIFESTS {
        for gone in ["ring = ", "base64 = ", "aes-gcm", "chacha20", "rust-crypto", "sodiumoxide", "hkdf"] {
            assert!(
                !manifest.contains(gone),
                "`{gone}` must not appear in {name}: the account's copy is plain text (cloud design, \
                 amendment 2026-09-17, ruling 2), and a crypto crate with nothing to encrypt is a \
                 dependency nobody is auditing."
            );
        }
    }
}
```

and add, to `engine/tests/no_console.rs`'s neighbours — a new case at the end of `dependency_boundary.rs`:

```rust
/// The module itself, scanned. A manifest check alone would pass on the day the code still held a
/// hand-rolled key schedule; this is the sentence that would have to be argued with.
#[test]
fn the_sync_module_holds_no_key_and_no_envelope() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("sync.rs"),
    )
    .expect("engine/src/sync.rs");
    for gone in ["SyncKey", "recovery", "Recovery", "seal(", "fn open(", "IndexKey", "HKDF", "hkdf", "aead", "CROCKFORD"] {
        assert!(!src.contains(gone), "engine/src/sync.rs still names {gone}");
    }
}
```

- [ ] **Step 2: Run them and watch them fail.**

Run: `cargo test -p knowlu-engine --test dependency_boundary`
Expected: FAIL — `engine/Cargo.toml must name sha2 for the sync content hash`, then ``ring = ` must not appear in engine/Cargo.toml``, then `engine/src/sync.rs still names SyncKey`.

- [ ] **Step 3: Rewrite `engine/src/sync.rs` down to its foundations.** Everything from the `SyncKey` section to the end of the file goes; the module doc is replaced and `SyncError` loses the four variants that described a key. The whole file after this step:

```rust
//! Journal sync (cloud design §5.5, as amended 2026-09-17): the account's own copy of its own vault.
//!
//! **What leaves this machine.** Every journal record this device writes, as `ledger::dumps_value`
//! wrote it, and the whole text of every note that changed. The account is the source of truth for
//! that data (amendment ruling 2) and this folder is a mirror of it; our service can read both, and
//! says so on the privacy page in the words Quinn and the lawyer approved. What does **not** leave
//! is a portal password, a session token in a log, an app-data path or a machine name: the `device`
//! column is `sha256(account_id + "\n" + hostname)` truncated to sixteen hex characters, which is an
//! identity for "the same machine as last time" and nothing else.
//!
//! **What this module is not.** It is transport, never judgment: `rank` does not reach it and
//! neither does anything under `cli.rs` except the `sync` subcommand itself. Nothing here calls a
//! model. And it never deletes a local note because the account does not have one — the mirror is
//! filled by the account, never emptied by it.
//!
//! **Every failure is a named line and exit 0.** No account, no session, no entitlement, no network,
//! a 402, a 5xx: the day still ranks from the folder on disk, because a slot step that exits
//! non-zero is retry backoff and an amber tray twice a day for ever.
//!
//! **Delivery is a pull, and the pull carries one thing.** `/sync-pull` returns records **another
//! desktop of this account** pushed. The service's own writes — judgment fields, Gmail-derived
//! notes, event verdicts, rule proposals — are delivered by C2, per item, inside the slot step that
//! asks for them, and are journalled on the device that asked; they then go up through this module
//! like any other record. The fidelity ledger's `§5.5 Down` row argues it.

use sha2::{Digest, Sha256};

/// The largest journal record this device will send, in bytes, matching
/// `sync_records.body`'s `octet_length` check. A record is a handful of scalars and, for a
/// `create`, one frontmatter mapping; 16 KiB is far past any that exists. A record over it is
/// reported by name and left in the journal, never truncated — a truncated record is a lie.
pub const MAX_RECORD_BYTES: usize = 16 * 1024;

/// The largest note text this device will send, in bytes, matching `sync_notes.body`'s check. A
/// note longer than 128 KiB is a pasted document, not a task; it stays in the vault and the push
/// says so in one line.
pub const MAX_NOTE_BYTES: usize = 128 * 1024;

/// Everything this module can refuse. **A closed set of words** (ruling R-3a-20's shape): no value
/// here came from a note or a server body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// No `config/cloud.yaml`: this vault has no account.
    NoAccount,
    /// There is an account but no usable session: the student is signed out on this machine.
    NoSession(String),
    /// The service refused or could not be reached. Carries `CloudError`'s own sentence, which is
    /// already scrubbed of the bearer.
    Service(String),
    /// A pulled row is not what it claims to be.
    Shape(&'static str),
    /// The vault, or the cursor file.
    Io(String),
}

impl SyncError {
    /// The word the `sync` step prints and the page shows. Never a path, never a body.
    pub fn label(&self) -> &'static str {
        match self {
            SyncError::NoAccount => "no account",
            SyncError::NoSession(_) => "no session",
            SyncError::Service(_) => "the service refused",
            SyncError::Shape(_) => "an unreadable row",
            SyncError::Io(_) => "the vault could not be read",
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NoSession(why) | SyncError::Service(why) | SyncError::Io(why) => {
                write!(f, "{} ({why})", self.label())
            }
            SyncError::Shape(what) => write!(f, "{} ({what})", self.label()),
            other => write!(f, "{}", other.label()),
        }
    }
}

/// Lowercase hex of `SHA-256(bytes)`. The one hash this module computes, and the one the server
/// re-derives: `sync-push` refuses a row whose `hash` is not this of its `body`, so the idempotence
/// key is a fact about the row rather than a claim about it.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Which machine pushed a row, to a server that has no reason to learn which machine that is.
///
/// The hostname is in the mix so two desktops of one account differ; the account id is in the mix so
/// the same hostname under two accounts does not collide; sixteen hex characters is enough to
/// separate the handful of desktops one student owns and short enough that it is obviously not a
/// name. `journal::device_name()` is the same value every journal record already carries **inside**
/// the row, where the student can read it — this is the outside, where nobody needs to.
pub fn device_token(account_id: &str) -> String {
    let mut input = String::with_capacity(account_id.len() + 1 + 32);
    input.push_str(account_id);
    input.push('\n');
    input.push_str(&crate::journal::device_name());
    sha256_hex(input.as_bytes())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_content_hash_is_sha256_of_the_canonical_bytes() {
        // The empty-string vector, so a reader can check it against any SHA-256 implementation.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(sha256_hex(b"abc").len(), 64);
    }

    #[test]
    fn a_device_token_is_sixteen_hex_stable_and_not_the_hostname() {
        let a = device_token("acct-1");
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(a, device_token("acct-1"), "stable across calls");
        assert_ne!(a, device_token("acct-2"), "and scoped to the account");
        let host = crate::journal::device_name().to_lowercase();
        assert!(!host.is_empty() && !a.contains(&host), "the hostname is not in the token");
    }

    #[test]
    fn a_refusal_never_carries_a_path_or_a_body() {
        let e = SyncError::Io("C:\\Users\\someone\\Knowlu\\Vault".to_string());
        assert_eq!(e.label(), "the vault could not be read");
        // The LABEL is what a line prints; `Display` is what a report may carry, and it is allowed
        // the cause. What matters is that the two are different functions and the step prints the
        // first — `run_lines_with` is where that is asserted (Task 7).
        assert!(!e.label().contains("Users"));
    }
}
```

- [ ] **Step 4: The manifest.** In `engine/Cargo.toml`, delete the `base64 = "0.22"` / `ring = "0.17"` block and its comment, and add, in alphabetical place after `serde_yaml_ng`:

```toml
# Knowlu C3′ (cloud design §5.5 as amended 2026-09-17): SHA-256 for the sync content hash — the
# idempotence key of a pushed journal record, which the server re-derives from the body. Already
# resolved in this workspace's Cargo.lock, so this is one direct edge and no new crate to audit and
# no new version to resolve. `sha1` beside it is a different thing and stays: it is
# `ids::derived_id`, a note-identity contract with every vault that exists.
sha2 = "0.10"
```

Then `cargo build --workspace` once, so `Cargo.lock`'s `knowlu-engine` dependency list is rewritten with `ring` and `base64` out and `sha2` in. **Inspect the lockfile diff**: exactly one package's `dependencies` array changes, and no `[[package]]` block is added or removed. If one is, stop — `sha2` was not already resolved and the argument above is wrong.

- [ ] **Step 5: Delete the two Deno vector files.**

```bash
git rm cloud/supabase/functions/_shared/sync_vectors.json cloud/supabase/functions/_shared/sync_envelope_test.ts
```

`sync_rows.ts` and `sync_rows_test.ts` stay and are reshaped by Task 3.

- [ ] **Step 6: Run everything.**

Run: `cargo test -p knowlu-engine --test dependency_boundary`, then `cargo test --workspace` (0 warnings, the accepted `.rsrc` line only), then the whole-tree `deno test`.
Expected: green. `engine/tests/fixtures/` is clean.

- [ ] **Step 7: Commit.**

```bash
git add engine/src/sync.rs engine/Cargo.toml Cargo.lock engine/tests/dependency_boundary.rs cloud/supabase/functions/_shared/sync_vectors.json cloud/supabase/functions/_shared/sync_envelope_test.ts
git commit -F .git-commit-msg.txt   # "engine: the sync envelope retires — the account holds the vault in plain text (C3' Task 2)"
```

---

### Task 3: `POST /sync-push`, and what a row may be

The first endpoint and the validators it stands on. It verifies the entitlement, refuses anything that is not a bounded, shaped row, **re-derives every record's hash from its own body**, checks the account is under its ceiling, and upserts. It never logs a row and never tells the caller anything about another device.

**Files:**
- Modify: `cloud/supabase/functions/_shared/sync_rows.ts`, `cloud/supabase/functions/_shared/sync_rows_test.ts`
- Create: `cloud/supabase/functions/_shared/sync_db.ts`, `cloud/supabase/functions/sync-push/{handler.ts,handler_test.ts,index.ts}`
- **Hand-off this task needs:** **H2** (`config.toml`'s two entries), applied by the controller beside this task

**Interfaces:**
- Consumes: C1's `requireActiveEntitlement`, `_shared/db.ts`, `_shared/http.ts`; Task 1's tables.
- Produces:
  - `sync_rows.ts`: `MAX_ROWS` (500), `MAX_RECORD_BYTES` (16384), `MAX_NOTE_BYTES` (131072), `NOTE_PATH_RE`, `isDeviceToken`, `isHash`, `isNotePath`, `bytes(s)`, `checkRecord(raw, device, accountId)`, `checkNote(raw, device, accountId)`
  - `sync_db.ts`: `READ_LAG_SECONDS` (10), `saveRecords`, `saveNotes`, `bytesUsed`, `ceiling`, `readRecords`, `readNotes`
  - `POST /sync-push` → `200 {"records": n, "notes": m, "bytes_used": b, "bytes_ceiling": c}`; 400 on a malformed row, an unknown field, a bad path or a hash that is not its body's; 401/402 from the gate — **thrown**, so a handler test awaits the rejection; 405 on the wrong method; **413** when the batch would carry the account past its ceiling.

- [ ] **Step 1: Write the failing validator tests** — `cloud/supabase/functions/_shared/sync_rows_test.ts`, rewritten:

```ts
import { assert, assertEquals, assertThrows } from "@std/assert";
import { checkNote, checkRecord, isNotePath, MAX_NOTE_BYTES, MAX_RECORD_BYTES } from "./sync_rows.ts";

const DEVICE = "0123456789abcdef";
const HASH = "a".repeat(64);
const ACCOUNT = "acct-1";
const RECORD = '{"actor":"quinn","device":"LAPTOP","id":"task_0000000001","op":"set","path":"tasks/x.md","ts":"2026-09-17T10:00:00.000Z","via":"dashboard"}';

Deno.test("a well-formed record becomes the row sync_records takes, and nothing else", () => {
  const row = checkRecord({ hash: HASH, body: RECORD }, DEVICE, ACCOUNT);
  assertEquals(Object.keys(row).sort(), ["account_id", "body", "device", "record_hash"]);
  assertEquals(row.account_id, ACCOUNT);
  assertEquals(row.record_hash, HASH);
});

Deno.test("`keep` cannot be sent — the server decides it", () => {
  // The amendment's improvement over the sealed design: the row is readable, so `keep` is a
  // generated column computed from the record. A client that sends one is telling the server
  // something the server already knows better, and the honest answer is a refusal, not a silent drop.
  const e = assertThrows(() => checkRecord({ hash: HASH, body: RECORD, keep: true }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a record whose body is not a journal record is a 400", () => {
  for (const body of ["", "null", "[]", '"a string"', "not json at all", '{"op":"set"}', '{"actor":"quinn"}']) {
    const e = assertThrows(() => checkRecord({ hash: HASH, body }, DEVICE, ACCOUNT), undefined, undefined, `${body} was accepted`) as Response;
    assertEquals(e.status, 400);
  }
});

Deno.test("a record over the byte cap is refused, and the cap is BYTES not characters", () => {
  // 16384 `é` is 16384 characters and 32768 bytes; the column's `octet_length` check would refuse
  // it after the validator let it through, and PostgREST's 400 says nothing a student could act on.
  const big = `{"actor":"quinn","op":"set","pad":"${"é".repeat(9000)}"}`;
  assert(big.length < MAX_RECORD_BYTES, "the test vector must be short in CHARACTERS to be a test");
  const e = assertThrows(() => checkRecord({ hash: HASH, body: big }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a note path is one of the six folders, markdown, and cannot climb out", () => {
  for (const ok of ["tasks/x.md", "courses/cs-100.md", "archive/a-b.md", "info/x.md", "issues/i.md", "approvals/amend-1.md"]) {
    assert(isNotePath(ok), ok);
  }
  for (const bad of [
    "state/journal/2026-09-17.jsonl", "config/ingest.yaml", "tasks/../../etc/hosts", "../tasks/x.md",
    "tasks//x.md", "tasks/x.txt", "tasks\\x.md", "/tasks/x.md", "tasks/", "", "TASKS/x.md",
  ]) {
    assert(!isNotePath(bad), `${bad} was accepted`);
  }
});

Deno.test("a tombstone carries no bytes and a live note carries them", () => {
  const dead = checkNote({ path: "tasks/x.md", deleted: true }, DEVICE, ACCOUNT);
  assertEquals(dead.deleted, true);
  assertEquals(dead.body, null);
  const live = checkNote({ path: "tasks/x.md", body: "---\nid: task_0000000001\n---\n" }, DEVICE, ACCOUNT);
  assertEquals(live.deleted, false);
  assert(typeof live.body === "string");
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", deleted: true, body: "x" }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("a note over the byte cap is refused", () => {
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", body: "x".repeat(MAX_NOTE_BYTES + 1) }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});

Deno.test("an unknown field beside the body is a 400 that names it", () => {
  // The single way a patched client could turn this store into something else is by sending a field
  // beside the body and hoping we write it.
  const e = assertThrows(() => checkNote({ path: "tasks/x.md", body: "x", title: "CS 100 HW 1" }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/_shared/sync_rows_test.ts`
Expected: FAIL to import — `isNotePath` and `MAX_RECORD_BYTES` do not exist yet, and `checkRecord` still takes a ciphertext.

- [ ] **Step 3: Rewrite `sync_rows.ts`.** The `noExtras` rule and the throw-a-`Response` convention are kept verbatim from the superseded version; the shapes change:

```ts
/**
 * What a pushed row may be. **Every field is bounded and shaped** — that is the whole module.
 *
 * The device has already built these; refusing again here is not distrust of the device, it is the
 * only place the rule holds for a client somebody else wrote. Two refusals this file makes that the
 * device does not: **an unknown key is a 400**, because a field beside the body is the one way a
 * patched client could turn this store into something it is not; and **a path is checked**, because
 * the path is the note's primary key now and a restore is what would write it.
 */
import { fail } from "./http.ts";

/** The server's own batch cap, and the device's page size. Matches C1's `/telemetry`. */
export const MAX_ROWS = 500;
/** `sync_records.body`'s `octet_length` check. BYTES, not characters — see `bytes()`. */
export const MAX_RECORD_BYTES = 16384;
/** `sync_notes.body`'s `octet_length` check. */
export const MAX_NOTE_BYTES = 131072;

const DEVICE_RE = /^[0-9a-f]{16}$/;
const HASH_RE = /^[0-9a-f]{64}$/;
/** The same rule `engine/src/sync.rs::is_note_path` enforces and the same one the column checks:
 * one of `ids::NOTE_FOLDERS`, markdown, no `..` segment and no empty segment. */
export const NOTE_PATH_RE = /^(tasks|approvals|archive|courses|issues|info)\/[A-Za-z0-9._ /-]{1,300}\.md$/;

export function isDeviceToken(x: unknown): boolean {
  return typeof x === "string" && DEVICE_RE.test(x);
}
export function isHash(x: unknown): boolean {
  return typeof x === "string" && HASH_RE.test(x);
}
export function isNotePath(x: unknown): boolean {
  return typeof x === "string" && NOTE_PATH_RE.test(x) && !/(^|\/)\.\.(\/|$)/.test(x) && !x.includes("//");
}
/** Postgres's `octet_length` on the device's side of the wire. `String.length` is UTF-16 units and
 * would let a vault of accented Spanish past a cap the column then refuses. */
export function bytes(s: string): number {
  return new TextEncoder().encode(s).length;
}

export interface RecordIn { hash: string; body: string }
export interface NoteIn { path: string; deleted?: boolean; body?: string }

function noExtras(row: Record<string, unknown>, allowed: string[], what: string): void {
  for (const key of Object.keys(row)) {
    if (!allowed.includes(key)) throw fail(400, `a ${what} carries an unknown field ${JSON.stringify(key)}`);
  }
}

/** One pushed journal record → the row `sync_records` takes. **Throws a `Response`** on anything else. */
export function checkRecord(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a record is not an object");
  const row = raw as Record<string, unknown>;
  // `keep` is NOT allowed: it is a generated column decided from the record itself.
  noExtras(row, ["hash", "body"], "record");
  if (!isHash(row.hash)) throw fail(400, "a record has no usable hash");
  if (typeof row.body !== "string" || row.body.length === 0) throw fail(400, "a record has no body");
  if (bytes(row.body) > MAX_RECORD_BYTES) {
    throw fail(400, `a record's body is over ${MAX_RECORD_BYTES} bytes`);
  }
  // Parsed here so the table's generated `keep` column never meets something it cannot cast, and so
  // a client that sends a blob of text instead of a record is told which of the two it did.
  let parsed: unknown;
  try {
    parsed = JSON.parse(row.body);
  } catch {
    throw fail(400, "a record's body is not JSON");
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw fail(400, "a record's body is not a journal record");
  }
  const rec = parsed as Record<string, unknown>;
  for (const field of ["op", "actor"]) {
    if (typeof rec[field] !== "string" || (rec[field] as string).length === 0) {
      throw fail(400, `a record's body has no ${field}`);
    }
  }
  return { account_id: accountId, device, record_hash: row.hash as string, body: row.body as string };
}

/** One pushed note → the row `sync_notes` takes. A tombstone carries no bytes; a live note carries them. */
export function checkNote(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a note is not an object");
  const row = raw as Record<string, unknown>;
  noExtras(row, ["path", "deleted", "body"], "note");
  if (!isNotePath(row.path)) throw fail(400, "a note has no usable path");
  const deleted = row.deleted === true;
  if (deleted) {
    if (row.body !== undefined) throw fail(400, "a deleted note carries no bytes");
    return { account_id: accountId, path: row.path as string, device, deleted: true, body: null };
  }
  if (typeof row.body !== "string" || row.body.length === 0) throw fail(400, "a note has no body");
  if (bytes(row.body) > MAX_NOTE_BYTES) throw fail(400, `a note's body is over ${MAX_NOTE_BYTES} bytes`);
  return { account_id: accountId, path: row.path as string, device, deleted: false, body: row.body as string };
}
```

- [ ] **Step 4: Write `sync_db.ts`** — C1's five helpers and **no raw fetch**:

```ts
/**
 * The only database access C3′ adds. Five calls over C1's `Rest`, every one scoped by
 * `account_id=eq.<id>` — the service role bypasses RLS, so that predicate IS the access control,
 * and `sync_rows_test.ts`'s scan (Task 12) refuses a query in this file without it.
 */
import { Rest, restSelect, restUpsert } from "./db.ts";

/**
 * How far behind `now` a read stops. **Two desktops cannot strand a row between them.**
 * `seq` is taken at INSERT and becomes visible at COMMIT, so a row can become visible with a `seq`
 * lower than one a previous pull already returned; a cursor that stepped past it would never fetch
 * it again. Ten seconds is far longer than a batch insert takes and far shorter than a slot.
 */
export const READ_LAG_SECONDS = 10;

export async function saveRecords(rest: Rest, rows: unknown[]): Promise<void> {
  if (rows.length === 0) return;
  // `sync_records_once` is `(account_id, record_hash)`: a batch re-sent after a dropped connection
  // lands on the rows it landed on the first time.
  await restUpsert(rest, "sync_records", rows, "account_id,record_hash");
}

export async function saveNotes(rest: Rest, rows: unknown[]): Promise<void> {
  if (rows.length === 0) return;
  await restUpsert(rest, "sync_notes", rows, "account_id,path");
}

export async function bytesUsed(rest: Rest, accountId: string): Promise<number> {
  const rows = await restSelect<{ bytes: number }>(rest, "sync_usage", `select=bytes&account_id=eq.${accountId}`);
  return rows[0]?.bytes ?? 0;
}

export async function ceiling(rest: Rest): Promise<number> {
  const rows = await restSelect<{ ceiling: number }>(rest, "sync_limits", "select=ceiling&limit=1");
  return Number(rows[0]?.ceiling ?? 0);
}

export async function readRecords(
  rest: Rest, accountId: string, after: number, limit: number, now: Date,
): Promise<unknown[]> {
  const cut = new Date(now.getTime() - READ_LAG_SECONDS * 1000).toISOString();
  return await restSelect(
    rest,
    "sync_records",
    `select=seq,device,record_hash,body&account_id=eq.${accountId}&seq=gt.${after}` +
      `&received_at=lt.${cut}&order=seq.asc&limit=${limit}`,
  );
}

export async function readNotes(
  rest: Rest, accountId: string, after: number, limit: number, now: Date,
): Promise<unknown[]> {
  const cut = new Date(now.getTime() - READ_LAG_SECONDS * 1000).toISOString();
  return await restSelect(
    rest,
    "sync_notes",
    `select=rev,device,path,deleted,body&account_id=eq.${accountId}&rev=gt.${after}` +
      `&updated_at=lt.${cut}&order=rev.asc&limit=${limit}`,
  );
}
```

- [ ] **Step 5: Write the failing handler tests** — `cloud/supabase/functions/sync-push/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const DEVICE = "0123456789abcdef";
const REC = (n: number) =>
  `{"actor":"quinn","device":"LAPTOP","id":"task_000000000${n}","op":"set","path":"tasks/x.md","ts":"2026-09-17T10:0${n}:00.000Z","via":"dashboard"}`;

/** The one hash the server trusts is the one it computes. */
async function sha256Hex(s: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

const OK = () => Promise.resolve({ account_id: "acct-1" });

function deps(over: Partial<Parameters<typeof handle>[1]> = {}) {
  return {
    requireEntitled: OK,
    bytesUsed: () => Promise.resolve(0),
    ceiling: () => Promise.resolve(1_000_000),
    saveRecords: () => Promise.resolve(),
    saveNotes: () => Promise.resolve(),
    ...over,
  };
}

function push(body: Record<string, unknown>): Request {
  return new Request("http://127.0.0.1/sync-push", {
    method: "POST",
    headers: { authorization: "Bearer t", "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

/** **Every refusal in this codebase is a THROWN `Response`** — `_shared/http.ts`'s `fail` throws and
 * `requireActiveEntitlement` rejects, and `index.ts`'s `asResponse` is what turns either into the
 * reply. A handler test therefore awaits the rejection, exactly as C1's
 * `telemetry/handler_test.ts:79` does; a bare `await handle(…)` would reject the test instead of
 * failing an assertion. */
function refusal(req: Request, d: Parameters<typeof handle>[1]): Promise<Response> {
  return handle(req, d).catch((e) => e as Response);
}

Deno.test("a batch of records and notes is stored and counted", async () => {
  let records: unknown[] = [];
  let notes: unknown[] = [];
  const res = await handle(
    push({
      device: DEVICE,
      records: [{ hash: await sha256Hex(REC(1)), body: REC(1) }, { hash: await sha256Hex(REC(2)), body: REC(2) }],
      notes: [{ path: "tasks/x.md", body: "---\nid: task_0000000001\n---\n" }, { path: "tasks/y.md", deleted: true }],
    }),
    deps({
      saveRecords: (r) => { records = r; return Promise.resolve(); },
      saveNotes: (n) => { notes = n; return Promise.resolve(); },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { records: 2, notes: 2, bytes_used: 0, bytes_ceiling: 1_000_000 });
  assertEquals(records.length, 2);
  assertEquals((records[0] as Record<string, unknown>).account_id, "acct-1");
  assertEquals((notes[1] as Record<string, unknown>).deleted, true);
});

Deno.test("a record whose hash is not its body's is a 400, and nothing is stored", async () => {
  // **The check the sealed design could not make.** A hash the client asserts is an idempotence key
  // a client can aim: two different records under one hash, and the second silently never lands.
  // The server can read the body now, so it computes the key itself.
  let stored = 0;
  const res = await refusal(
    push({ device: DEVICE, records: [{ hash: await sha256Hex(REC(1)), body: REC(2) }] }),
    deps({ saveRecords: () => { stored += 1; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 400);
  assert((await res.json()).error.includes("hash"), "the message names what is wrong");
  assertEquals(stored, 0);
});

Deno.test("a duplicate row inside one batch becomes one write", async () => {
  // `sync_records_once` catches it across retries; this catches it within a batch, before the
  // upsert — PostgREST refuses a batch that names one conflict target twice, so this is not an
  // optimisation, it is what keeps a legitimate double-send from becoming a 400.
  let records: unknown[] = [];
  const h = await sha256Hex(REC(1));
  const res = await handle(
    push({ device: DEVICE, records: [{ hash: h, body: REC(1) }, { hash: h, body: REC(1) }] }),
    deps({ saveRecords: (r) => { records = r; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 200);
  assertEquals((await res.json()).records, 1);
  assertEquals(records.length, 1);
});

Deno.test("a row with a plaintext field beside the body is a 400 that names the field", async () => {
  const res = await refusal(
    push({ device: DEVICE, notes: [{ path: "tasks/x.md", body: "x", title: "CS 100 HW 1" }] }),
    deps(),
  );
  assertEquals(res.status, 400);
  assert((await res.json()).error.includes("title"));
});

Deno.test("a device token that is not a device token is a 400", async () => {
  assertEquals((await refusal(push({ device: "nope", records: [] }), deps())).status, 400);
  assertEquals((await refusal(push({ records: [] }), deps())).status, 400);
});

Deno.test("a batch that would pass the ceiling is a 413 and stores nothing", async () => {
  let stored = 0;
  const res = await refusal(
    push({ device: DEVICE, records: [{ hash: await sha256Hex(REC(1)), body: REC(1) }] }),
    deps({
      bytesUsed: () => Promise.resolve(999_999),
      ceiling: () => Promise.resolve(1_000_000),
      saveRecords: () => { stored += 1; return Promise.resolve(); },
    }),
  );
  assertEquals(res.status, 413);
  assertEquals(stored, 0);
});

Deno.test("more than MAX_ROWS of either kind is a 400, not a silent truncation", async () => {
  const many = Array.from({ length: 501 }, (_, i) => ({ path: `tasks/x${i}.md`, deleted: true }));
  assertEquals((await refusal(push({ device: DEVICE, notes: many }), deps())).status, 400);
});

Deno.test("no entitlement and no session are the gate's, thrown, and nothing is stored", async () => {
  for (const status of [401, 402]) {
    let stored = 0;
    const res = await refusal(
      push({ device: DEVICE, records: [] }),
      deps({
        requireEntitled: () => Promise.reject(new Response(JSON.stringify({ error: "x" }), { status })),
        saveRecords: () => { stored += 1; return Promise.resolve(); },
      }),
    );
    assertEquals(res.status, status);
    assertEquals(stored, 0);
  }
});

Deno.test("the wrong method is a 405", async () => {
  const res = await handle(new Request("http://127.0.0.1/sync-push", { method: "GET" }), deps());
  assertEquals(res.status, 405);
});
```

- [ ] **Step 6: Run them and watch them fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/sync-push/`
Expected: FAIL to resolve `./handler.ts`.

- [ ] **Step 7: Write the handler and its index.** `handler.ts`:

```ts
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { checkNote, checkRecord, isDeviceToken, MAX_ROWS } from "../_shared/sync_rows.ts";

export interface PushDeps {
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  bytesUsed: (accountId: string) => Promise<number>;
  ceiling: () => Promise<number>;
  saveRecords: (rows: unknown[]) => Promise<void>;
  saveNotes: (rows: unknown[]) => Promise<void>;
}

interface Body { device?: unknown; records?: unknown; notes?: unknown }

async function sha256Hex(s: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function listOf(x: unknown, what: string): unknown[] {
  if (x === undefined) return [];
  if (!Array.isArray(x)) throw fail(400, `${what} must be a list`);
  if (x.length > MAX_ROWS) throw fail(400, `${what}: at most ${MAX_ROWS} a call`);
  return x;
}

export async function handle(req: Request, deps: PushDeps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const { account_id } = await deps.requireEntitled(req);
  const body = await readJson<Body>(req);
  if (!isDeviceToken(body.device)) throw fail(400, "a push carries an opaque device token");
  const device = body.device as string;

  // **Validate, hash-check and de-duplicate before anything is written.** A batch is all or nothing:
  // a student whose last row is malformed must not end up with a half-stored push and a cursor that
  // has moved past the rest of it.
  const records = new Map<string, Record<string, unknown>>();
  for (const raw of listOf(body.records, "records")) {
    const row = checkRecord(raw, device, account_id);
    const actual = await sha256Hex(row.body as string);
    if (actual !== row.record_hash) {
      throw fail(400, "a record's hash is not the sha256 of its body");
    }
    records.set(actual, row);
  }
  const notes = new Map<string, Record<string, unknown>>();
  for (const raw of listOf(body.notes, "notes")) {
    const row = checkNote(raw, device, account_id);
    notes.set(row.path as string, row);
  }

  const [used, cap] = await Promise.all([deps.bytesUsed(account_id), deps.ceiling()]);
  const adding = [...records.values(), ...notes.values()]
    .reduce((n, r) => n + (typeof r.body === "string" ? new TextEncoder().encode(r.body).length : 0), 0);
  if (cap > 0 && used + adding > cap) {
    // Not a failure of the device and not something a retry fixes: the device turns this into one
    // named line and one Good-to-know item, and the records stay in the journal.
    throw fail(413, "this account's copy is at its size limit");
  }

  await deps.saveRecords([...records.values()]);
  await deps.saveNotes([...notes.values()]);
  return json(200, { records: records.size, notes: notes.size, bytes_used: used, bytes_ceiling: cap });
}
```

`index.ts` follows C1's shape exactly — `Deno.serve`, `restFromEnv()`, `requireActiveEntitlement`, and `try { … } catch (e) { return asResponse(e); }`.

- [ ] **Step 8: Record what C3′ hands on.** In the task report, one paragraph: **`exportAll` in `cloud/supabase/functions/account/index.ts` should now include the two sync tables**, because the folder export is struck (ruling 2) and `GET /account/export` is the only way a student takes their data elsewhere. The shape is `sync_notes` rows as `{path, body}` and `sync_records` rows as raw JSON lines, so the export is a vault a person could reconstruct. **Not built here** — it is a change to the body of C1's own export function and it deserves its own test and its own review — and it is named in *What is NOT in this plan* and in Task 12's production list.

- [ ] **Step 9: Run, then commit.**

Run: the C3′ files, then the whole tree. Green.

```bash
git add cloud/supabase/functions/_shared/sync_rows.ts cloud/supabase/functions/_shared/sync_rows_test.ts cloud/supabase/functions/_shared/sync_db.ts cloud/supabase/functions/sync-push/
git commit -F .git-commit-msg.txt   # "cloud: POST /sync-push — bounded rows, a hash the server re-derives, and the account's ceiling (C3' Task 3)"
```

---

### Task 4: `GET /sync-pull`

The second endpoint, and the smaller one. Two cursors, one page, and a `more` flag so a desktop that has been away for a term catches up over several calls instead of asking for everything at once.

**Files:**
- Create: `cloud/supabase/functions/sync-pull/{handler.ts,handler_test.ts,index.ts}`
- **Hand-off this task needs:** **H2**, applied at Task 3 — nothing further here

**Interfaces:**
- Consumes: Task 3's `sync_db.ts` (`readRecords`, `readNotes`, `READ_LAG_SECONDS`); C1's gate and `http.ts`.
- Produces: `GET /sync-pull?records_after=<n>&notes_after=<n>&limit=<n>` → `200 {"records": [{seq, device, record_hash, body}], "notes": [{rev, device, path, deleted, body}], "record_cursor": n, "note_cursor": m, "more": bool}`; the gate's 401/402 **thrown**; 405 returned. `MAX_PAGE` is exported for the tests and for the device's page size.

- [ ] **Step 1: Ask Quinn (P4)**, in one message, when this task is reached: *`/sync-pull` returns rows this device itself pushed, and the device drops them by content hash. For a one-desktop account that is one wasted page per slot right after a push. Accept it (the hash filter has to exist anyway, for a re-install), or add `device=neq.<token>` on the server and let the service learn which desktop is asking? Recommendation: accept.* Record the answer in the task report; the plan is built to accept.

- [ ] **Step 2: Write the failing test** — `cloud/supabase/functions/sync-pull/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { handle, MAX_PAGE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const rec = (seq: number) => ({ seq, device: "0123456789abcdef", record_hash: "a".repeat(64), body: '{"actor":"quinn","op":"set"}' });
const note = (rev: number) => ({ rev, device: "0123456789abcdef", path: `tasks/x${rev}.md`, deleted: false, body: "---\n---\n" });

/** A fixed clock, so the ten-second read lag is a value a test can see rather than a race. */
const NOW = new Date("2026-09-17T12:00:00.000Z");

function pull(query: string) {
  return new Request(`http://127.0.0.1/sync-pull${query}`, { headers: { authorization: "Bearer t" } });
}

function refusal(req: Request, d: Parameters<typeof handle>[1]): Promise<Response> {
  return handle(req, d).catch((e) => e as Response);
}

Deno.test("a page comes back in cursor order with the cursor it ends on", async () => {
  const res = await handle(pull("?records_after=0&notes_after=0"), {
    requireEntitled: OK,
    readRecords: () => Promise.resolve([rec(7), rec(9)]),
    readNotes: () => Promise.resolve([note(3)]),
    now: () => NOW,
  });
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(body.records.map((r: { seq: number }) => r.seq), [7, 9]);
  assertEquals(body.record_cursor, 9);
  assertEquals(body.note_cursor, 3);
  assertEquals(body.more, false);
});

Deno.test("an empty page holds the cursors where they were", async () => {
  // The device must not rewind. A page with nothing in it answers with the cursor it was given, so a
  // slot that finds nothing new advances nothing and asks the same question next time.
  const res = await handle(pull("?records_after=41&notes_after=17"), {
    requireEntitled: OK, readRecords: () => Promise.resolve([]), readNotes: () => Promise.resolve([]), now: () => NOW,
  });
  const body = await res.json();
  assertEquals([body.record_cursor, body.note_cursor, body.more], [41, 17, false]);
});

Deno.test("a full page says there is more", async () => {
  const many = Array.from({ length: MAX_PAGE }, (_, i) => rec(i + 1));
  const res = await handle(pull(`?records_after=0&notes_after=0&limit=${MAX_PAGE}`), {
    requireEntitled: OK, readRecords: () => Promise.resolve(many), readNotes: () => Promise.resolve([]), now: () => NOW,
  });
  const body = await res.json();
  assertEquals(body.more, true);
  assertEquals(body.record_cursor, MAX_PAGE);
});

Deno.test("the caller's limit is clamped, never trusted", async () => {
  // PostgREST caps a select at `max_rows` (1000) whether or not the caller asked, so a limit above
  // the cap would silently produce a short page that reads as "no more" and strand every row past it.
  for (const q of ["?limit=99999", "?limit=0", "?limit=-3", "?limit=lots"]) {
    let asked = -1;
    await handle(pull(q), {
      requireEntitled: OK,
      readRecords: (_a, _b, limit) => { asked = limit; return Promise.resolve([]); },
      readNotes: () => Promise.resolve([]),
      now: () => NOW,
    });
    assertEquals(asked, MAX_PAGE, q);
  }
});

Deno.test("a cursor that is not a number starts from the beginning rather than erroring", async () => {
  let asked = -1;
  await handle(pull("?records_after=yesterday"), {
    requireEntitled: OK,
    readRecords: (_a, after) => { asked = after; return Promise.resolve([]); },
    readNotes: () => Promise.resolve([]),
    now: () => NOW,
  });
  assertEquals(asked, 0);
});

Deno.test("a restore is this endpoint from zero, and nothing else", async () => {
  // Exit gate item 7: there is no third function. A desktop with an empty vault asks the same
  // question with `records_after=0` and pages to the end.
  let asked = -1;
  await handle(pull("?records_after=0&notes_after=0"), {
    requireEntitled: OK,
    readRecords: (_a, after) => { asked = after; return Promise.resolve([]); },
    readNotes: () => Promise.resolve([]),
    now: () => NOW,
  });
  assertEquals(asked, 0);
});

Deno.test("the clock the read lag uses is the handler's, and it reaches both readers", async () => {
  // I3, inherited: both reads must share one window, or a note can be returned from behind a lag a
  // record was already read past.
  const seen: Date[] = [];
  await handle(pull("?records_after=0"), {
    requireEntitled: OK,
    readRecords: (_a, _b, _c, now) => { seen.push(now); return Promise.resolve([]); },
    readNotes: (_a, _b, _c, now) => { seen.push(now); return Promise.resolve([]); },
    now: () => NOW,
  });
  assertEquals(seen.length, 2);
  assertEquals(seen[0].getTime(), NOW.getTime());
  assertEquals(seen[1].getTime(), NOW.getTime());
});

Deno.test("no entitlement and no session are the gate's, thrown, and nothing is read", async () => {
  for (const status of [401, 402]) {
    let read = 0;
    const res = await refusal(pull("?records_after=0"), {
      requireEntitled: () => Promise.reject(new Response(JSON.stringify({ error: "x" }), { status })),
      readRecords: () => { read += 1; return Promise.resolve([]); },
      readNotes: () => { read += 1; return Promise.resolve([]); },
      now: () => NOW,
    });
    assertEquals(res.status, status);
    assertEquals(read, 0);
  }
});

Deno.test("the wrong method is a 405", async () => {
  const res = await handle(new Request("http://127.0.0.1/sync-pull", { method: "POST" }), {
    requireEntitled: OK, readRecords: () => Promise.resolve([]), readNotes: () => Promise.resolve([]), now: () => NOW,
  });
  assertEquals(res.status, 405);
});
```

- [ ] **Step 3: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/sync-pull/`
Expected: FAIL to resolve `./handler.ts`.

- [ ] **Step 4: Write the handler.**

```ts
import { json, methodNotAllowed } from "../_shared/http.ts";

/** One page. The same number the device uses, and under PostgREST's own `max_rows` (1000) so a
 * short page always means "that is all", never "the server cut you off". */
export const MAX_PAGE = 500;

export interface PullDeps {
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  readRecords: (accountId: string, after: number, limit: number, now: Date) => Promise<unknown[]>;
  readNotes: (accountId: string, after: number, limit: number, now: Date) => Promise<unknown[]>;
  now: () => Date;
}

function cursor(params: URLSearchParams, key: string): number {
  const n = Number(params.get(key));
  return Number.isSafeInteger(n) && n > 0 ? n : 0;
}

export async function handle(req: Request, deps: PullDeps): Promise<Response> {
  if (req.method !== "GET") return methodNotAllowed(["GET"]);
  const { account_id } = await deps.requireEntitled(req);
  const params = new URL(req.url).searchParams;
  const asked = Number(params.get("limit"));
  const limit = Number.isSafeInteger(asked) && asked > 0 ? Math.min(asked, MAX_PAGE) : MAX_PAGE;
  const recordsAfter = cursor(params, "records_after");
  const notesAfter = cursor(params, "notes_after");
  // **One clock for both reads** (review I3): two `new Date()` calls would put the two windows a
  // millisecond apart, which is exactly the gap the lag exists to close.
  const now = deps.now();
  const [records, notes] = await Promise.all([
    deps.readRecords(account_id, recordsAfter, limit, now),
    deps.readNotes(account_id, notesAfter, limit, now),
  ]);
  const last = <T extends Record<string, unknown>>(rows: unknown[], key: string, fallback: number): number => {
    const row = rows.at(-1) as T | undefined;
    const v = row ? Number(row[key]) : NaN;
    return Number.isSafeInteger(v) ? v : fallback;
  };
  return json(200, {
    records,
    notes,
    record_cursor: last(records, "seq", recordsAfter),
    note_cursor: last(notes, "rev", notesAfter),
    more: records.length >= limit || notes.length >= limit,
  });
}
```

`index.ts` is `sync-push`'s, with `readRecords`/`readNotes` from `sync_db.ts` and `now: () => new Date()`.

- [ ] **Step 5 (controller): deploy both functions to staging and smoke them.**

```
supabase functions deploy sync-push sync-pull --project-ref brvhgbihxevrudqpulcm
```

Then, with a staging session minted by the controller (never a subagent — `HANDOFF.md` §5): `POST /sync-push` with one record and one note → 200 and the two counts; `GET /sync-pull?records_after=0&notes_after=0` **twice, ten seconds apart** — the first page is **empty** (the read lag) and the second carries the row. **An implementer or controller who sees only the second pull has not tested the lag**; both observations go in the report. Then `POST /sync-push` with no bearer → 401, and with a bad row → 400.

- [ ] **Step 6: Commit.**

```bash
git add cloud/supabase/functions/sync-pull/
git commit -F .git-commit-msg.txt   # "cloud: GET /sync-pull — two cursors, one page, and a read lag two desktops cannot fall through (C3' Task 4)"
```

---
### Task 5: The cursor, and what this device sends

The push half of the device. Nothing here opens a socket except the one loopback test at the end; everything else is a pure function of the vault and the cursor.

**Files:**
- Modify: `engine/src/sync.rs` (the cursor and push sections)
- Create: `engine/tests/sync_contract.rs`

**Interfaces:**
- Consumes: Task 2's `sha256_hex`, `device_token`, `MAX_RECORD_BYTES`, `MAX_NOTE_BYTES`, `SyncError`; C2's `cloudmodel::{CloudClient, CloudError}`; `journal::Journal`; `ids::NOTE_FOLDERS`; `ledger::dumps_value`; `pystr::read_text`.
- Produces:
  - `sync::PAGE: usize` (500) and `sync::CURSOR_FILE: &str` (`"state/sync-cursor.json"`)
  - `sync::Cursor { record_cursor: i64, note_cursor: i64, pushed_through: String, boundary: Vec<String>, notes: BTreeMap<String, String> }`, `Cursor::default()`, `sync::load_cursor(&Path) -> Cursor`, `sync::save_cursor(&Path, &Cursor) -> Result<(), SyncError>`
  - `sync::is_note_path(&Path, &str) -> bool`, `sync::note_paths(&Path) -> Vec<String>`
  - `sync::PushBatch { device: String, records: Vec<Value>, notes: Vec<Value>, warnings: Vec<String> }`
  - `sync::build_push(&Path, &Cursor, &str, &mut Journal) -> (PushBatch, Cursor)`
  - `sync::push(&CloudClient, &PushBatch) -> Result<(usize, usize), CloudError>`

- [ ] **Step 1: Write the failing contract tests** — `engine/tests/sync_contract.rs`. The loopback harness is C2's `engine/tests/cloud_contract.rs`, copied verbatim (a second harness would be a second thing to keep true): a `TcpListener` on `127.0.0.1:0`, a thread that reads one request and answers from a scripted list, and `requests()` joining that thread. The header says why that is not egress. Then:

```rust
#[test]
fn a_first_push_carries_every_record_and_every_note_once() {
    let dir = fixture("first-push");            // a copy of tests/fixtures/vault-s1 in a temp dir
    let mut journal = Journal::new(&dir);
    let (batch, next) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(batch.warnings.is_empty(), "{:?}", batch.warnings);
    assert_eq!(batch.device.len(), 16);
    assert_eq!(batch.notes.len(), sync::note_paths(&dir).len(), "every note on disk");
    assert!(!batch.records.is_empty(), "the fixture has a journal");
    // The SECOND build with the cursor the first returned sends nothing.
    let (again, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    assert_eq!((again.records.len(), again.notes.len()), (0, 0), "a push is idempotent against its own cursor");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_records_hash_is_the_sha256_of_the_canonical_bytes_the_server_will_see() {
    // The server re-derives this and refuses a mismatch (Task 3). The bytes it hashes are the ones
    // in `body`, so the two computations agree only if `body` IS `dumps_value`'s output — which is
    // why nothing here ever reaches for `serde_json::to_string`.
    let dir = fixture("hash");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    for row in &batch.records {
        let body = row["body"].as_str().expect("a body");
        assert_eq!(row["hash"].as_str().expect("a hash"), sync::sha256_hex(body.as_bytes()));
        let parsed: serde_json::Value = serde_json::from_str(body).expect("a record parses");
        assert_eq!(knowlu_engine::ledger::dumps_value(&parsed), body, "the body is canonical, not re-serialised");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_note_that_has_not_changed_is_not_pushed_again_and_one_that_moved_is_a_tombstone() {
    let dir = fixture("tombstone");
    let mut journal = Journal::new(&dir);
    let (_, cursor) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let gone = sync::note_paths(&dir).into_iter().next().expect("a note");
    std::fs::remove_file(dir.join(&gone)).expect("remove one note");
    let (batch, _) = sync::build_push(&dir, &cursor, "acct-1", &mut journal);
    assert_eq!(batch.notes.len(), 1, "{:?}", batch.notes);
    assert_eq!(batch.notes[0]["path"].as_str(), Some(gone.as_str()));
    assert_eq!(batch.notes[0]["deleted"], serde_json::Value::Bool(true));
    assert!(batch.notes[0].get("body").is_none(), "a tombstone carries no bytes");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_or_a_note_over_the_cap_is_named_and_left_behind_never_truncated() {
    let dir = fixture("oversize");
    let big = format!("---\nid: task_0000000001\n---\n{}", "x".repeat(sync::MAX_NOTE_BYTES));
    knowlu_engine::pystr::write_text(&dir.join("tasks").join("huge.md"), &big).expect("write");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(!batch.notes.iter().any(|n| n["path"] == "tasks/huge.md"), "the oversize note is not sent");
    assert!(batch.warnings.iter().any(|w| w.contains("tasks/huge.md") && w.contains("too large")), "{:?}", batch.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_page_is_at_most_PAGE_and_the_cursor_does_not_skip_the_rest() {
    // A term of catching up must not be one enormous request, and it must not silently drop the
    // tail either: the cursor stops exactly where the page stopped.
    let dir = fixture("page");
    let mut journal = Journal::new(&dir);
    for n in 0..(sync::PAGE + 7) {
        let mut spec = knowlu_engine::journal::NewRecord::new("set", "tasks/x.md", "quinn", "dashboard");
        spec.field = Some("importance");
        spec.new = serde_json::json!(n as i64 % 5);
        let mut rec = knowlu_engine::journal::make_record(spec).expect("a record");
        journal.append(&mut rec).expect("append");
    }
    journal.invalidate();
    let (batch, next) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert_eq!(batch.records.len(), sync::PAGE);
    let (rest, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    assert!(!rest.records.is_empty(), "the tail is still owed");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_push_carries_no_token_no_hostname_and_no_app_data_path() {
    // The data-minimisation assertion, kept from the sealed design because it is still true and
    // still worth failing on: the bearer is a header `CloudClient` adds and scrubs from errors, the
    // hostname is inside the record where the student can read it and not in a column, and nothing
    // about this machine's app data travels at all.
    let dir = fixture("minimal");
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    let wire = knowlu_engine::ledger::dumps_value(&serde_json::json!({
        "device": batch.device, "records": batch.records, "notes": batch.notes
    }));
    for forbidden in ["Bearer", "access_token", "LOCALAPPDATA", "AppData", "knowlu/", "sync-key"] {
        assert!(!wire.contains(forbidden), "the wire carries {forbidden}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_push_posts_to_sync_push_and_reads_the_two_counts() {
    let mut server = loopback(vec![(200, r#"{"records":2,"notes":1,"bytes_used":10,"bytes_ceiling":20}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], warnings: vec![] };
    assert_eq!(sync::push(&client, &batch).expect("a 200"), (2, 1));
    let sent = server.requests();
    assert!(sent[0].starts_with("POST /sync-push "), "{}", sent[0]);
    // Lower case: `ureq` writes header names as given and C2's own contract test learned this the
    // hard way — do not "fix" it back to `Authorization`.
    assert!(sent[0].contains("authorization: Bearer jwt-not-a-secret"), "{}", sent[0]);
}

#[test]
fn a_413_is_a_named_refusal_and_not_a_transport_failure() {
    let mut server = loopback(vec![(413, r#"{"error":"this account's copy is at its size limit"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], warnings: vec![] };
    match sync::push(&client, &batch) {
        Err(knowlu_engine::cloudmodel::CloudError::Status { code, .. }) => assert_eq!(code, 413),
        other => panic!("{other:?}"),
    }
    let _ = server.requests();
}
```

- [ ] **Step 2: Run them and watch them fail.**

Run: `cargo test -p knowlu-engine --test sync_contract`
Expected: FAIL to compile — `sync::Cursor`, `sync::build_push`, `sync::note_paths`, `sync::PAGE` and `sync::push` do not exist.

- [ ] **Step 3: Write the cursor section.**

```rust
/// One page, both ways. The same number `sync_rows.ts::MAX_ROWS` and `sync-pull`'s `MAX_PAGE` use,
/// and under PostgREST's own `max_rows` (1000), so a short page always means "that is all".
pub const PAGE: usize = 500;

/// Generated, device-local, and **never synced**: it holds two integers, one timestamp and a map of
/// note path → content hash, every one of which is already in the vault in plainer form.
pub const CURSOR_FILE: &str = "state/sync-cursor.json";

/// What this device has already seen and already sent.
///
/// `pushed_through` plus `boundary` is the pair that makes "where did I stop" exact. The journal is
/// `ts`-ordered and several records can share a `ts` to the millisecond, so a cursor that stored
/// only a timestamp would either re-send the whole millisecond every run or skip the half of it that
/// landed after the read. `boundary` holds the hashes of the records at exactly `pushed_through`;
/// the next run reads from that timestamp inclusive and drops the ones it already sent.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    #[serde(default)] pub record_cursor: i64,
    #[serde(default)] pub note_cursor: i64,
    #[serde(default)] pub pushed_through: String,
    #[serde(default)] pub boundary: Vec<String>,
    #[serde(default)] pub notes: std::collections::BTreeMap<String, String>,
}

/// A missing or unreadable cursor is a **fresh** cursor, never an error: the worst it costs is one
/// full re-push, every row of which is idempotent on its own hash, and the alternative — a slot that
/// fails because a generated file was hand-edited — is the failure mode this product does not have.
pub fn load_cursor(vault: &Path) -> Cursor {
    crate::pystr::read_text(&vault.join(CURSOR_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Written through `ledger::dumps_value` and `pystr::write_text`, like every other file this crate
/// puts in a vault.
pub fn save_cursor(vault: &Path, cursor: &Cursor) -> Result<(), SyncError> {
    let value = serde_json::to_value(cursor).map_err(|e| SyncError::Io(e.to_string()))?;
    let path = vault.join(CURSOR_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SyncError::Io(e.to_string()))?;
    }
    crate::pystr::write_text(&path, &crate::ledger::dumps_value(&value)).map_err(|e| SyncError::Io(e.to_string()))
}

/// Inside the vault, under one of `ids::NOTE_FOLDERS`, markdown, and no segment that climbs out.
///
/// **One implementation, three callers** (review I7): the push builder, the applier and the
/// restorer. `sync_rows.ts::NOTE_PATH_RE` and `sync_notes.path`'s own check are the same rule on the
/// other two sides of the wire, and `is_note_path_and_the_servers_regex_agree` pins them together.
pub fn is_note_path(vault: &Path, rel: &str) -> bool {
    if rel.is_empty() || rel.contains('\\') || rel.contains("//") || !rel.ends_with(".md") {
        return false;
    }
    if rel.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return false;
    }
    let Some(folder) = rel.split('/').next() else { return false };
    if !crate::ids::NOTE_FOLDERS.contains(&folder) || rel.split('/').count() < 2 {
        return false;
    }
    crate::ids::inside_vault(vault, &vault.join(rel)).is_ok()
}

/// Every note in the vault, vault-relative and POSIX-separated, sorted.
///
/// **Flat, like every other pass in this engine.** `ids::scan_notes` and `backup::BACKUP_FOLDERS`
/// treat the six folders as flat and nothing in this product has ever produced a nested note;
/// `is_note_path` accepts any depth because a *pulled* path must be checked whatever it is, but
/// making this side recursive alone would push a file nothing else in the engine can see. Deferred,
/// and recorded in *Deferred minors*.
pub fn note_paths(vault: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for folder in crate::ids::NOTE_FOLDERS {
        let Ok(entries) = std::fs::read_dir(vault.join(folder)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|x| x == "md") == Some(true) && path.is_file() {
                out.push(format!("{folder}/{}", path.file_name().unwrap_or_default().to_string_lossy()));
            }
        }
    }
    out.sort();
    out
}
```

- [ ] **Step 4: Write the push builder.**

```rust
/// What one push carries. `warnings` are lines the run prints; they are never sent.
#[derive(Debug, Clone, Default)]
pub struct PushBatch {
    pub device: String,
    pub records: Vec<Value>,
    pub notes: Vec<Value>,
    pub warnings: Vec<String>,
}

/// Everything written since the cursor, and the cursor that follows it.
///
/// **Two independent halves.** Records come from the journal, in `ts` order, from `pushed_through`
/// inclusive with `boundary` subtracted. Notes come from the folder: every path whose text hashes
/// differently from what the cursor remembers, plus a tombstone for every path the cursor remembers
/// and the folder no longer has. The note half is deliberately state-based rather than journal-based
/// — a note edited by hand in an editor has no record, and the mirror has to carry it anyway.
///
/// **The returned cursor is only saved by the caller after the push succeeds.** A cursor advanced
/// over a batch the service never received is the one bug that loses a record for good.
pub fn build_push(vault: &Path, cursor: &Cursor, account_id: &str, journal: &mut Journal) -> (PushBatch, Cursor) {
    let mut batch = PushBatch { device: device_token(account_id), ..Default::default() };
    let mut next = cursor.clone();

    let since = if cursor.pushed_through.is_empty() { None } else { Some(cursor.pushed_through.as_str()) };
    let already: std::collections::BTreeSet<&String> = cursor.boundary.iter().collect();
    for record in journal.read(since, None) {
        if batch.records.len() >= PAGE { break; }
        let body = crate::ledger::dumps_value(&Value::Object(record.clone()));
        let hash = sha256_hex(body.as_bytes());
        if already.contains(&hash) { continue; }
        if body.len() > MAX_RECORD_BYTES {
            batch.warnings.push(format!("sync: one journal record is too large to send ({} bytes); it stays in the journal", body.len()));
            continue;
        }
        let ts = record.get("ts").and_then(Value::as_str).unwrap_or_default().to_string();
        if ts != next.pushed_through {
            next.pushed_through = ts;
            next.boundary.clear();
        }
        next.boundary.push(hash.clone());
        batch.records.push(serde_json::json!({ "hash": hash, "body": body }));
    }

    let on_disk = note_paths(vault);
    let mut seen = std::collections::BTreeSet::new();
    for rel in &on_disk {
        if batch.notes.len() >= PAGE { break; }
        seen.insert(rel.clone());
        let Ok(text) = crate::pystr::read_text(&vault.join(rel)) else {
            batch.warnings.push(format!("sync: {rel} could not be read; it stays on this machine"));
            continue;
        };
        if text.len() > MAX_NOTE_BYTES {
            batch.warnings.push(format!("sync: {rel} is too large to send ({} bytes); it stays on this machine", text.len()));
            continue;
        }
        let hash = sha256_hex(text.as_bytes());
        if next.notes.get(rel) == Some(&hash) { continue; }
        next.notes.insert(rel.clone(), hash);
        batch.notes.push(serde_json::json!({ "path": rel, "body": text }));
    }
    for rel in cursor.notes.keys() {
        if batch.notes.len() >= PAGE { break; }
        if seen.contains(rel) { continue; }
        next.notes.remove(rel);
        batch.notes.push(serde_json::json!({ "path": rel, "deleted": true }));
    }
    (batch, next)
}

/// The one POST. `CloudClient::post` already serialises through `dumps_value`, carries the bearer
/// and the anon key, and scrubs the bearer from every error string it produces.
pub fn push(client: &crate::cloudmodel::CloudClient, batch: &PushBatch) -> Result<(usize, usize), crate::cloudmodel::CloudError> {
    let body = serde_json::json!({ "device": batch.device, "records": batch.records, "notes": batch.notes });
    let reply = client.post("/sync-push", &body)?;
    let count = |key: &str| reply.get(key).and_then(Value::as_u64).unwrap_or(0) as usize;
    Ok((count("records"), count("notes")))
}
```

- [ ] **Step 5: Add the cross-language path test** to `sync_contract.rs`, so the three implementations of one rule cannot drift:

```rust
#[test]
fn is_note_path_and_the_servers_regex_agree() {
    // The rule lives in three places by necessity — Rust, TypeScript and a column check — so it is
    // pinned from one side rather than trusted three times. The TS source is read, not imported.
    let ts = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("cloud").join("supabase")
            .join("functions").join("_shared").join("sync_rows.ts"),
    ).expect("sync_rows.ts");
    assert!(ts.contains("(tasks|approvals|archive|courses|issues|info)"), "the server's folder list is the engine's");
    let vault = std::env::temp_dir();
    for ok in ["tasks/x.md", "courses/cs-100.md", "info/a.md"] { assert!(sync::is_note_path(&vault, ok), "{ok}"); }
    for bad in ["state/journal/2026-09-17.jsonl", "config/ingest.yaml", "tasks/../../x.md", "tasks//x.md",
                "tasks/x.txt", "tasks\\x.md", "/tasks/x.md", "tasks", ""] {
        assert!(!sync::is_note_path(&vault, bad), "{bad}");
    }
}
```

- [ ] **Step 6: Run, then commit.**

Run: `cargo test -p knowlu-engine --test sync_contract`, then `cargo test --workspace` at 0 warnings. `git status --porcelain --untracked-files=all engine/tests/fixtures/` → empty.

```bash
git add engine/src/sync.rs engine/tests/sync_contract.rs
git commit -F .git-commit-msg.txt   # "engine: the sync cursor and what this device sends (C3' Task 5)"
```

---

### Task 6: The pull, `reconcile`, and the amend card

The half the whole feature is for. A record another desktop wrote is appended to this device's journal **verbatim**, and the note-level effect is settled by the pure `reconcile::resolve` that has been sitting unused since plan 2 — with the roles reversed, which is the one subtle thing in this task and is spelled out twice below.

**Files:**
- Modify: `engine/src/sync.rs` (the pull and apply sections)
- Test: `engine/tests/sync_contract.rs` (eleven new cases)

**Interfaces:**
- Consumes: `reconcile::resolve`, `write::{write_literals, create, delete, move_note, propose_amendment, find_pending_amendment, WriteContext, WriteOpts}`, `approvals::{AMENDABLE_FIELDS, AMENDABLE_FOLDERS}`, `models::split_frontmatter`, `ids::{read_meta, inside_vault, is_id, NOTE_FOLDERS}`, `journal::{Journal, OPS, VIAS}`, Task 5's `is_note_path`, `sha256_hex`.
- Produces:
  - `sync::ACTOR: &str` = `"agent:knowlu.sync"`
  - `sync::PulledNote { device: String, path: String, text: Option<String> }`
  - `sync::Pulled { records: Vec<(String, Record)>, notes: Vec<PulledNote>, record_cursor: i64, note_cursor: i64, more: bool, warnings: Vec<String> }`
  - `sync::pulled_from_reply(&Value) -> Result<Pulled, SyncError>`
  - `sync::pull(&CloudClient, i64, i64) -> Result<Pulled, SyncError>`
  - `sync::record_is_well_formed(&Record) -> Result<(), &'static str>`
  - `sync::ApplyReport { records, notes_written, applied, cards, superseded, moved, refused, warnings }`
  - `sync::apply(&Path, &Pulled, &WriteContext, &mut Journal, jiff::civil::Date) -> ApplyReport`

**The role reversal, stated once and repeated in the code.** `reconcile::resolve` was written for git, where `checkout --ours` had already put *upstream's* file on disk and the job was to re-apply *this* device's edits onto it. Here the file on disk is **this device's**. So the arguments are handed over the other way round:

| `resolve`'s parameter | git's world | C3′'s world |
|---|---|---|
| `upstream_meta` | the file after taking upstream | **the local note as it stands on disk** |
| `upstream_records` | the other device's records | **this device's own records since the base** |
| `local_records` | this device's records | **the foreign records that just came down** |
| `upstream_mtime_ts` | upstream's commit time | **the local file's mtime, as a journal `ts`** |

Read the function with that substitution and it does exactly the right thing: `chains` becomes the set of fields the *other* desktop changed; a field whose on-disk value equals that chain's first `old` means this device never touched it, so the foreign value applies cleanly and **no supersede record is produced**; anything else is a two-sided change, `wins()` decides by later `ts`, and a supersede record names the loser. **A field with a supersede record is this plan's definition of a conflict**, and that is what becomes a card.

- [ ] **Step 1: Write the failing tests** — eleven cases appended to `engine/tests/sync_contract.rs`. The load-bearing six, in full; the other five are the same shape and are named in the step so an implementer cannot quietly drop one.

```rust
/// A foreign `set` record: another desktop, another `seq`, a chosen `ts`.
fn foreign_set(id: &str, path: &str, field: &str, old: serde_json::Value, new: serde_json::Value, ts: &str) -> knowlu_engine::ledger::Record {
    let mut spec = knowlu_engine::journal::NewRecord::new("set", path, "quinn", "dashboard");
    spec.id = Some(id);
    spec.field = Some(field);
    spec.old = old;
    spec.new = new;
    spec.ts = Some(ts.to_string());
    spec.device = Some("OtherDesktop".to_string());
    knowlu_engine::journal::make_record(spec).expect("a record")
}

fn pulled(records: Vec<knowlu_engine::ledger::Record>, notes: Vec<sync::PulledNote>) -> sync::Pulled {
    sync::Pulled {
        records: records.into_iter().map(|r| ("fedcba9876543210".to_string(), r)).collect(),
        notes, record_cursor: 1, note_cursor: 1, more: false, warnings: Vec::new(),
    }
}

#[test]
fn a_field_only_the_other_desktop_touched_applies_cleanly_and_files_no_card() {
    let dir = fixture("clean");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.applied, report.cards, report.superseded), (1, 0, 0), "{report:?}");
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_foreign_record_is_appended_verbatim_with_its_own_device_and_ts() {
    // The whole reason a pull and a restore do not go through `write`: these records already exist,
    // and rewriting their `ts` or their `device` would destroy the attribution judge-once reads.
    let dir = fixture("verbatim");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    journal.invalidate();
    let kept = journal.read(None, None).into_iter()
        .find(|r| r.get("device").and_then(|v| v.as_str()) == Some("OtherDesktop")).expect("the foreign record");
    assert_eq!(kept.get("ts").and_then(|v| v.as_str()), Some("2026-09-17T10:00:00.000Z"));
    assert_eq!(kept.get("actor").and_then(|v| v.as_str()), Some("quinn"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_record_this_device_already_has_is_not_applied_twice() {
    let dir = fixture("dedupe");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let first = sync::apply(&dir, &pulled(vec![rec.clone()], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(first.records, 1);
    let second = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((second.records, second.applied, second.cards), (0, 0, 0), "{second:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_field_both_desktops_moved_becomes_one_amend_card_and_not_a_silent_merge() {
    // VISION, and §5.5's own sentence. The card is the ordinary `kind: amend` card the deck already
    // answers, through the ordinary `propose_amendment` path, so the fifteen-a-day cap applies.
    let dir = fixture("conflict");
    let mut journal = Journal::new(&dir);
    let mine = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::write_literals(&dir, "tasks/cs-100-hw-01.md", &[("importance".to_string(), "4".to_string())], &mine, &mut journal, &Default::default()).expect("my edit");
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2036-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.cards, 1, "{report:?}");
    // The note keeps what this device had until the card is answered: never a silent merge.
    let meta = knowlu_engine::ids::read_meta(&dir.join("tasks").join("cs-100-hw-01.md")).expect("the note");
    assert_eq!(knowlu_engine::yaml::get(&meta, "importance").and_then(knowlu_engine::yaml::i64_of), Some(4));
    let card = std::fs::read_dir(dir.join("approvals")).expect("approvals").flatten().map(|e| e.path())
        .find(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("amend-")).unwrap_or(false)).expect("one amend card");
    let text = knowlu_engine::pystr::read_text(&card).expect("the card");
    assert!(text.contains("kind: amend") && text.contains("from: 4") && text.contains("to: 5"), "{text}");
    // A second pull of the same field set must not mint a second card.
    let again = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(6), "2036-09-17T11:00:00.000Z");
    let second = sync::apply(&dir, &pulled(vec![again], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(second.cards, 0, "an identical pending proposal already IS the re-proposal");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_record_that_is_not_a_record_never_reaches_the_ledger() {
    // Review I7, inherited. The rows are plaintext now, which makes this MORE important rather than
    // less: a malformed record used to fail to decrypt, and now it arrives looking like data.
    let dir = fixture("malformed");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let good = foreign_set("task_0000000001", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(3), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let mut bad = Vec::new();
    for (key, value) in [
        ("op", serde_json::json!("rm -rf")), ("via", serde_json::json!("telepathy")),
        ("ts", serde_json::json!("last Tuesday")), ("actor", serde_json::Value::Null),
        ("id", serde_json::json!("../../etc/hosts")), ("path", serde_json::json!("config/cloud.yaml")),
    ] {
        let mut r = good.clone();
        r.insert(key.to_string(), value);
        bad.push(r);
    }
    let mut all = bad.clone();
    all.push(good);
    let report = sync::apply(&dir, &pulled(all, vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.refused, bad.len(), "{report:?}");
    assert_eq!(report.records, 1, "the good one still lands");
    journal.invalidate();
    let text = knowlu_engine::ledger::dumps_value(&serde_json::json!(journal.read(None, None)));
    assert!(!text.contains("rm -rf") && !text.contains("telepathy"), "a refused record reached the ledger");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pulled_note_for_a_path_this_device_has_never_seen_arrives_whole() {
    // The second-desktop case: a note created on the other machine has no local file to reconcile
    // against, so its text is written as it stands — and a path outside the six folders is refused
    // before anything is written.
    let dir = fixture("newnote");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let body = "---\nid: task_0000009999\nstatus: open\n---\n\nFrom the other desktop.\n";
    let report = sync::apply(&dir, &pulled(vec![], vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/from-elsewhere.md".into(), text: Some(body.to_string()) },
        sync::PulledNote { device: "fedcba9876543210".into(), path: "config/cloud.yaml".into(), text: Some("api_base: evil\n".into()) },
    ]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!(report.notes_written, 1, "{report:?}");
    assert_eq!(report.refused, 1);
    assert_eq!(knowlu_engine::pystr::read_text(&dir.join("tasks").join("from-elsewhere.md")).expect("the note"), body);
    assert!(!knowlu_engine::pystr::read_text(&dir.join("config").join("cloud.yaml")).unwrap_or_default().contains("evil"));
    let _ = std::fs::remove_dir_all(&dir);
}
```

The other five, each one paragraph of the same shape: `a_conflict_this_device_won_writes_nothing_and_files_no_card` (a foreign write older than this device's leaves the note alone, produces one supersede record and no card); `a_conflict_on_an_unamendable_field_takes_reconciles_rule_and_says_so` (a field outside `AMENDABLE_FIELDS` — the later `ts` wins, the supersede record stands, one named warning names the note and the field); `a_pulled_tombstone_settles_the_note_rather_than_unlinking_it` (`text: None` goes through `write::delete`, never `fs::remove_file`); `a_pulled_move_may_not_escape_the_vault` (a `move` record whose destination fails `is_note_path` is refused before `write::move_note` sees it); and `a_pull_reads_the_two_cursors_and_the_more_flag_from_the_reply` (a loopback `GET /sync-pull?records_after=…&notes_after=…&limit=500` and the parsed `Pulled`).

- [ ] **Step 2: Run them and watch them fail.** `cargo test -p knowlu-engine --test sync_contract` → FAIL to compile on `sync::apply`, `sync::Pulled`, `sync::PulledNote`, `sync::ACTOR`.

- [ ] **Step 3: Write the guards and the reader.**

```rust
/// Who a pulled write is attributed to on THIS machine's side of a reconciliation. The foreign
/// records keep their own actor; this is the actor of the local effect — the supersede record, the
/// amend card, the note write — so the Runs view and the journal both say a sync did it.
pub const ACTOR: &str = "agent:knowlu.sync";

/// Is this a journal record at all? **Six checks, and every one of them is a shape a malformed or
/// hostile row could otherwise slip through into `state/journal/`** (review I7).
pub fn record_is_well_formed(record: &Record) -> Result<(), &'static str> {
    let s = |k: &str| record.get(k).and_then(Value::as_str).unwrap_or_default();
    if !crate::journal::OPS.contains(&s("op")) { return Err("unknown op"); }
    if !crate::journal::VIAS.contains(&s("via")) { return Err("unknown via"); }
    if s("ts").parse::<jiff::Timestamp>().is_err() { return Err("unparseable ts"); }
    if s("actor").is_empty() { return Err("no actor"); }
    if s("device").is_empty() { return Err("no device"); }
    let id = s("id");
    if !id.is_empty() && !crate::ids::is_id(id) { return Err("not an id"); }
    Ok(())
}

/// The reply, parsed. **Every field the device trusts is re-derived from the row's own body**
/// (Task 2's carried obligation (a) from the sealed design's review): the record is parsed out of
/// `body` and its hash recomputed, so a row whose `record_hash` disagrees with its bytes is a
/// warning and a skip rather than a record with a borrowed identity.
pub fn pulled_from_reply(reply: &Value) -> Result<Pulled, SyncError> {
    let mut out = Pulled { record_cursor: reply.get("record_cursor").and_then(Value::as_i64).unwrap_or(0),
                           note_cursor: reply.get("note_cursor").and_then(Value::as_i64).unwrap_or(0),
                           more: reply.get("more").and_then(Value::as_bool).unwrap_or(false),
                           ..Default::default() };
    for row in reply.get("records").and_then(Value::as_array).cloned().unwrap_or_default() {
        let device = row.get("device").and_then(Value::as_str).unwrap_or_default().to_string();
        let body = row.get("body").and_then(Value::as_str).unwrap_or_default();
        if row.get("record_hash").and_then(Value::as_str) != Some(sha256_hex(body.as_bytes()).as_str()) {
            out.warnings.push("sync: one pulled record did not match its own hash and was skipped".to_string());
            continue;
        }
        match serde_json::from_str::<Value>(body) {
            Ok(Value::Object(map)) => out.records.push((device, map)),
            _ => out.warnings.push("sync: one pulled record was not a record and was skipped".to_string()),
        }
    }
    for row in reply.get("notes").and_then(Value::as_array).cloned().unwrap_or_default() {
        let path = row.get("path").and_then(Value::as_str).unwrap_or_default().to_string();
        let text = if row.get("deleted").and_then(Value::as_bool).unwrap_or(false) {
            None
        } else {
            Some(row.get("body").and_then(Value::as_str).unwrap_or_default().to_string())
        };
        out.notes.push(PulledNote { device: row.get("device").and_then(Value::as_str).unwrap_or_default().to_string(), path, text });
    }
    Ok(out)
}

/// One `GET`. The device asks for both cursors in one call so the two windows share a clock on the
/// server (`READ_LAG_SECONDS`).
pub fn pull(client: &crate::cloudmodel::CloudClient, records_after: i64, notes_after: i64) -> Result<Pulled, SyncError> {
    let path = format!("/sync-pull?records_after={records_after}&notes_after={notes_after}&limit={PAGE}");
    let reply = client.get(&path).map_err(|e| SyncError::Service(e.to_string()))?;
    pulled_from_reply(&reply)
}
```

- [ ] **Step 4: Write `apply`.** Its shape, in order, with the reasons in the code:

1. Build `known`, the set of `sha256_hex(dumps_value(record))` for every record already in the local journal. A pulled record already in it is not applied twice, whichever desktop pushed it.
2. For each pulled record: `record_is_well_formed`, then `is_note_path` on its `path` (and, for a `move`, on its destination) — a failure is `refused += 1` and one warning, never an append.
3. Append the survivors to the ledger **verbatim** through `JsonlLedger::append`, grouped by their own `ts`'s UTC day, so a record made on 15 September lands in `state/journal/2026-09-15.jsonl` and not in today's file.
4. Group the appended records by note id. For each note: read the local note's `meta` and mtime, read this device's own records for that id since the foreign records' earliest `ts`, and call `reconcile::resolve` **with the roles reversed** exactly as the table above says.
5. `resolution.apply` goes through `write::write_literals` under `ACTOR`; `resolution.supersede` records are appended; a field with a supersede record whose folder is in `AMENDABLE_FOLDERS` and whose field is in `AMENDABLE_FIELDS` becomes a `write::propose_amendment` **unless `write::find_pending_amendment` already has one for that (note, field)**; anything else is one named warning.
6. Pulled notes: a path this device has no file for is written with `pystr::write_text` (it is a note another desktop created; there is nothing to reconcile); a path it does have is **left alone** — its frontmatter is settled by the records above and its body is never merged; `text: None` is `write::delete`.

- [ ] **Step 5: Run, then commit.**

```bash
git add engine/src/sync.rs engine/tests/sync_contract.rs
git commit -F .git-commit-msg.txt   # "engine: the pull, reconcile with the roles reversed, and the amend card (C3' Task 6)"
```

---

### Task 7: `knowlu-engine sync`, the slot, and the named skips

The command. Everything above becomes one subcommand that always exits 0 and says, in one short line each, exactly what happened or why nothing did.

**Files:**
- Modify: `engine/src/sync.rs` (the command section)
- Test: `engine/tests/sync_contract.rs` (five new cases), `app/tests/scheduler.rs`, `app/tests/commands.rs`
- **Hand-offs this task needs:** **H4a** (`engine/src/main.rs`'s `Sync` subcommand), **H8a** (`scheduler.rs`'s slot step), **H9a** (`ConsoleState.sync` and `commands::sync_inner`) — all applied by the controller beside this task

**Interfaces:**
- Consumes: `cloudmodel::{load, resolve}`, Tasks 5 and 6.
- Produces:
  - `sync::Direction { Pull, Push, Both }` with `Direction::parse(&str) -> Option<Direction>`
  - `sync::Totals { pulled_records, pulled_notes, applied, cards, superseded, pushed_records, pushed_notes, more, skipped: Option<String>, errors: Vec<String> }`
  - `sync::SyncStatus { ok: bool, at: Option<String>, lines: Vec<String>, last_error: Option<String> }` — `Default`, `Clone`, `Serialize`, and `SyncStatus::of(&Totals, Vec<String>) -> SyncStatus`
  - `sync::run_lines_with(&Path, Direction, &str, Option<&str>) -> (i32, Vec<String>, Totals)` — **the return code is always 0**
  - `sync::run_lines(&Path, Direction, &str, Option<&str>) -> (i32, Vec<String>)`
  - `sync::is_configured(&Path) -> bool`

- [ ] **Step 1: Write the failing tests** — five cases appended to `engine/tests/sync_contract.rs`:

```rust
#[test]
fn a_vault_with_no_account_says_so_and_exits_zero() {
    let dir = fixture("noaccount");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0);
    assert_eq!(lines, vec!["sync (skipped: no account)".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_vault_with_an_account_and_no_session_says_exactly_that() {
    // **The check order is the message** (review I4, inherited): `load` → `resolve`. The common
    // answer on a machine with a `cloud.yaml` and no credential is "no session", and a student who
    // has signed out must not read a sentence about their subscription. Reversing the order makes
    // this assertion impossible to write, which is how the review found it the first time.
    let dir = fixture("nosession");
    std::fs::write(
        dir.join("config").join("cloud.yaml"),
        "api_base: 'https://example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/c3-no-such-profile/session'\naccount_id: 'acct-1'\n",
    ).expect("cloud.yaml");
    let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    assert_eq!(code, 0);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("sync (skipped: no session"), "{lines:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_refusal_is_a_named_line_and_never_a_non_zero_exit() {
    // The property `judge` has and the one this step must have: a non-zero exit sets
    // `RunSummary.engine_ok = false`, which paints the tray amber and puts the slot into retry
    // backoff twice a day forever.
    let dir = fixture("exitzero");
    for yaml in ["", "api_base: 'https://example.invalid'\n", "not: yaml: at: all\n"] {
        std::fs::write(dir.join("config").join("cloud.yaml"), yaml).expect("cloud.yaml");
        let (code, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
        assert_eq!(code, 0, "{yaml:?} -> {lines:?}");
        assert!(!lines.is_empty(), "{yaml:?} said nothing");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_line_never_carries_a_vault_path_a_bearer_or_a_hostname() {
    let dir = fixture("quiet-lines");
    let (_, lines) = sync::run_lines(&dir, sync::Direction::Both, "cli", None);
    let joined = lines.join("\n");
    for forbidden in ["Bearer", &dir.to_string_lossy().to_string(), &knowlu_engine::journal::device_name()] {
        assert!(!joined.contains(forbidden.as_ref() as &str), "{joined}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_status_the_page_reads_is_built_from_the_totals_and_never_from_a_line() {
    let totals = sync::Totals { pushed_records: 3, applied: 1, cards: 1, ..Default::default() };
    let s = sync::SyncStatus::of(&totals, vec!["sync: 3 up, 1 applied, 1 card".to_string()]);
    assert!(s.ok && s.last_error.is_none());
    assert!(s.at.is_some(), "a run that happened is stamped");
    let failed = sync::SyncStatus::of(&sync::Totals { errors: vec!["no network".into()], ..Default::default() }, vec![]);
    assert!(!failed.ok);
    assert_eq!(failed.last_error.as_deref(), Some("no network"));
    // A skip is not an error: the tray must not go amber because a student is signed out.
    let skipped = sync::SyncStatus::of(&sync::Totals { skipped: Some("no account".into()), ..Default::default() }, vec![]);
    assert!(skipped.ok && skipped.last_error.is_none());
}
```

and, in `app/tests/scheduler.rs`:

```rust
#[test]
fn sync_is_the_slots_first_step() {
    let v = scratch("sync-first");
    let exe = Path::new("knowlu-engine.exe");
    let steps = slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no entitlement)"));
    assert_eq!(steps[0].1[0], "sync", "{steps:?}");
    assert_eq!(steps[1].1[0], "coursework", "and nothing was displaced");
    assert!(steps.iter().any(|(_, a)| a[0] == "rank"), "{steps:?}");
    // One row in the Runs view, not two: `run_slot_inner` names a step by `args[0]`.
    assert_eq!(steps.iter().filter(|(_, a)| a[0] == "sync").count(), 1);
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 2: Run them and watch them fail.** `cargo test -p knowlu-engine --test sync_contract` and `cargo test -p knowlu --test scheduler` → FAIL: `sync::run_lines` and `sync::SyncStatus` do not exist; `steps[0].1[0]` is `"coursework"`.

- [ ] **Step 3: Write the command section.** `run_lines_with`, in order, and the order is the message (review I4):

1. `cloudmodel::load(vault)` → `None` is `sync (skipped: no account)`, `Totals { skipped: Some("no account") }`, exit 0. This is the answer for every vault that has never signed in, including all three fixtures, which is why the oracles cannot move.
2. `cloudmodel::resolve(vault)` → `Err(Unavailable::NoSession(why))` is `sync (skipped: no session — sign in again)`, exit 0.
3. Load the cursor. **Pull first** when the direction includes it: `pull`, then `apply`, then save the cursor with the returned `record_cursor`/`note_cursor`. The pull is the half that has to precede `rank`.
4. **Push second**: `build_push`, `push`, and **save the cursor only if the push returned 200** — a cursor advanced over a batch the service never received is the one bug that loses a record for good.
5. Every `CloudError` becomes one line through `CloudError::label()` plus its `Display`, is recorded in `totals.errors`, and **is not a skip**: `sync (no network)` reads differently from `sync (skipped: no account)` because they are different facts. Exit 0 either way.
6. `more: true` adds `sync: more to come — the next slot continues` so a term of catching up is visible rather than silent.

`run_lines` is `run_lines_with` with the totals dropped. `is_configured(vault)` is `cloudmodel::load(vault).is_some()` and is what `state::quit_flush` asks before pushing.

- [ ] **Step 4: Apply the three hand-offs** (**H4a**, **H8a**, **H9a**) and re-run `cargo test --workspace`. The slot's first step is `sync`; the console's *Sync now* runs the engine instead of git; `ConsoleState.sync` is filled.

- [ ] **Step 5: Commit.**

```bash
git add engine/src/sync.rs engine/tests/sync_contract.rs app/tests/scheduler.rs app/tests/commands.rs
git commit -F .git-commit-msg.txt   # "engine: knowlu-engine sync — first in the slot, always exit 0, one line each (C3' Task 7)"
```

---

### Task 8: The engine gates itself on entitlement

Ruling 3, and the one task in this plan that is not about sync at all. Today the app decides whether a cloud step runs; from here the engine decides too, from the same cache and the same 72-hour grace, so a copy of `knowlu-engine.exe` with no account behind it ranks a hand-made folder and nothing else.

**Files:**
- Create: `engine/src/entitle.rs`, `engine/tests/entitlement_gate.rs`
- **Hand-offs this task needs:** **H3a** (`engine/src/lib.rs`'s `pub mod entitle;`, **compile-blocking**), **H4b** (`engine/src/main.rs`'s gate) — both applied by the controller beside this task

**Interfaces:**
- Consumes: `cloudmodel::load` (for `session_credential_target`), `jiff::Timestamp`.
- Produces:
  - `entitle::GRACE_SECONDS: i64` (`72 * 60 * 60`)
  - `entitle::Cached { status: String, checked_at: String }` (the two fields the decision reads; the other two are ignored on the way in)
  - `entitle::profile_id(&str) -> Option<&str>`
  - `entitle::app_data_root_in(&Path) -> PathBuf` and `entitle::cache_path_in(&Path, &str) -> PathBuf`
  - `entitle::decide(Option<&Cached>, jiff::Timestamp) -> bool`
  - `entitle::gate(&Path) -> Option<String>` — `Some(the named line)` when the step must not run

- [ ] **Step 1: Ask Quinn (P5)**, in one message, when this task is reached — the text is in the preconditions table. Record the answer in the task report; the plan is built to (a), the four cloud steps.

- [ ] **Step 2: Write the failing test** — `engine/tests/entitlement_gate.rs`:

```rust
//! Ruling 3 of the cloud design's amendment of 2026-09-17, from the outside.
//!
//! No test here reads a real profile: `entitle`'s two `_in` functions take the app-data root, and
//! every case passes a temp directory. No test here touches Credential Manager — the gate is a file
//! and a clock, deliberately, because an entitlement check that needed a credential would be a
//! second thing to fail on a machine that is merely signed out.
use std::path::{Path, PathBuf};

use knowlu_engine::entitle;

fn cache(root: &Path, profile: &str, status: &str, checked_at: &str) {
    let dir = root.join("knowlu").join("profiles").join(profile);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("entitlement.json"),
        format!(r#"{{"status":"{status}","current_period_end":null,"plan":"monthly","checked_at":"{checked_at}"}}"#),
    ).expect("write");
}

#[test]
fn a_vault_with_no_account_is_not_gated_at_all() {
    // "an orphaned binary ranks a hand-made folder and nothing else" — the hand-made folder is the
    // one with no `config/cloud.yaml`, and nothing about it changes.
    let dir = temp("nogate");
    assert_eq!(entitle::gate(&dir), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fresh_check_lets_the_step_run_and_a_stale_one_does_not() {
    let now: jiff::Timestamp = "2026-09-17T12:00:00Z".parse().unwrap();
    let fresh = entitle::Cached { status: "active".into(), checked_at: "2026-09-17T06:00:00Z".into() };
    let stale = entitle::Cached { status: "active".into(), checked_at: "2026-09-13T06:00:00Z".into() };
    assert!(entitle::decide(Some(&fresh), now));
    assert!(!entitle::decide(Some(&stale), now), "past 72 hours the grace is spent");
    assert!(!entitle::decide(None, now), "no cache is not entitlement");
    for status in ["past_due", "canceled", "none", ""] {
        let c = entitle::Cached { status: status.into(), checked_at: "2026-09-17T06:00:00Z".into() };
        assert!(!entitle::decide(Some(&c), now), "{status}");
    }
    assert!(entitle::decide(Some(&entitle::Cached { status: "trialing".into(), checked_at: "2026-09-17T06:00:00Z".into() }), now));
}

#[test]
fn a_clock_an_hour_fast_is_tolerated_and_a_clock_a_year_fast_is_not() {
    // `app/src/account.rs::decide`'s own fix round 1 item 1, copied because copying it is the point:
    // a machine a few minutes ahead must not disentitle itself forever, and a machine a year ahead
    // is a clock that moved, not a licence.
    let now: jiff::Timestamp = "2026-09-17T12:00:00Z".parse().unwrap();
    assert!(entitle::decide(Some(&entitle::Cached { status: "active".into(), checked_at: "2026-09-17T12:30:00Z".into() }), now));
    assert!(!entitle::decide(Some(&entitle::Cached { status: "active".into(), checked_at: "2027-09-17T12:00:00Z".into() }), now));
}

#[test]
fn the_engines_grace_is_the_apps_grace_and_the_path_is_the_apps_path() {
    // **Two constants in two crates, pinned from one side.** The app writes the file; the engine
    // reads it; a grace that drifted would mean the page said "your subscription lapsed" on a
    // different day from the one the slots stopped. Read as text, not linked: `knowlu-engine` does
    // not depend on `knowlu` and must not start.
    let app = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("app").join("src").join("account.rs"),
    ).expect("app/src/account.rs");
    assert!(app.contains("from_secs(72 * 60 * 60)"), "the app's grace moved; move the engine's with it");
    assert_eq!(entitle::GRACE_SECONDS, 72 * 60 * 60);
    assert!(app.contains(r#"data_dir.join("entitlement.json")"#), "the app's cache filename moved");
    assert!(app.contains(r#"base.join("knowlu")"#), "the app's data root moved");
}

#[test]
fn the_profile_id_comes_out_of_the_credential_target_and_nowhere_else() {
    assert_eq!(entitle::profile_id("knowlu/profile_0a1b2c3d4e/session"), Some("profile_0a1b2c3d4e"));
    for bad in ["", "knowlu/session", "knowlu//session", "profile_1", "knowlu/p/extra/session"] {
        assert_eq!(entitle::profile_id(bad), None, "{bad}");
    }
}

#[test]
fn a_gated_vault_past_the_grace_gets_one_named_line_and_that_line_is_all_the_caller_needs() {
    let root = temp("gated");
    let vault = root.join("vault");
    std::fs::create_dir_all(vault.join("config")).expect("mkdir");
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        "api_base: 'https://example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_0a1b2c3d4e/session'\naccount_id: 'acct-1'\n",
    ).expect("cloud.yaml");
    cache(&root, "profile_0a1b2c3d4e", "canceled", "2026-09-17T06:00:00Z");
    let line = entitle::gate_in(&root, &vault, "2026-09-17T12:00:00Z".parse().unwrap()).expect("gated");
    assert_eq!(line, "skipped: no entitlement");
    // And the line is a word, never a path or a status code from Stripe.
    assert!(!line.contains("profile_") && !line.contains("canceled"));
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 3: Run it and watch it fail.** `cargo test -p knowlu-engine --test entitlement_gate` → FAIL to compile: there is no `entitle` module.

- [ ] **Step 4: Write `engine/src/entitle.rs`.**

```rust
//! The entitlement gate (cloud design, amendment 2026-09-17, ruling 3).
//!
//! **Why the engine and not only the app.** Quinn's reason, on the record: the attacker is a person
//! repurposing this code to get its full utility without paying, or a competitor shipping a free
//! copy. With the fetch sequence, the parsers, the rules and every judgment server-side and
//! entitlement-gated, an orphaned binary ranks a hand-made folder and nothing else. The app's gate
//! is a UI decision; this one is the binary's.
//!
//! **It reads the app's cache and never the network.** `GET /entitlement` is the app's call, made at
//! launch and every six hours; an engine that made it too would race the app's own session refresh
//! and could spend a token the app is about to rotate. So this module is a file and a clock: the
//! same file, the same 72-hour grace, the same arithmetic — pinned to `app/src/account.rs` by a test
//! that reads both, because two constants that must agree and cannot be linked have to be checked.
//!
//! **A gate is a named line at exit 0, never a failure.** A student whose card expired is not a
//! machine with a problem, and retrying fixes nothing; `main.rs` prints the line and returns
//! SUCCESS, so the tray stays calm and the Runs view says what happened.
//!
//! **What is not gated:** `surface` and `write` (the console reads and edits through them — gating
//! either freezes the window rather than the subscription), `runs`, `info`, `issues`,
//! `coursework-discover`, and — pending precondition P5 — `rank`.

use std::path::{Path, PathBuf};

/// Spec §5.1, and `app/src/account.rs::GRACE`. Seventy-two hours, so a weekend of bad wifi never
/// stops a slot for a student who is paying.
pub const GRACE_SECONDS: i64 = 72 * 60 * 60;

/// The two fields of the app's cache this decision reads. `current_period_end` and `plan` are in the
/// file and are deliberately not here: a decision that read them would be a second implementation of
/// Stripe's own rules on the wrong side of the wire.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Cached {
    pub status: String,
    pub checked_at: String,
}

/// `knowlu/<profile_id>/session` → `profile_id`. Exactly three segments, all non-empty: a target of
/// another shape is a vault this engine does not understand, and guessing would mean reading some
/// other profile's cache.
pub fn profile_id(target: &str) -> Option<&str> {
    let mut parts = target.split('/');
    let (Some(first), Some(id), Some(last), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if first != "knowlu" || id.is_empty() || last != "session" { return None; }
    Some(id)
}

/// `app/src/state.rs::app_data_root_in`'s path, and the seam every test uses.
pub fn app_data_root_in(base: &Path) -> PathBuf { base.join("knowlu") }

pub fn cache_path_in(base: &Path, profile: &str) -> PathBuf {
    app_data_root_in(base).join("profiles").join(profile).join("entitlement.json")
}

/// `app/src/account.rs::decide`'s arithmetic, for the two states this side needs.
pub fn decide(cache: Option<&Cached>, now: jiff::Timestamp) -> bool {
    let Some(c) = cache else { return false };
    if c.status != "active" && c.status != "trialing" { return false; }
    let Ok(checked) = c.checked_at.parse::<jiff::Timestamp>() else { return false };
    let mut age = now.as_second() - checked.as_second();
    // A clock a few minutes fast must not disentitle someone forever — each refresh would otherwise
    // write another "future" `checked_at` and the state would never recover on its own.
    if age < 0 && -age <= 3600 { age = 0; }
    age >= 0 && age <= GRACE_SECONDS
}

/// The gate, with its two inputs injected. `None` means "run".
pub fn gate_in(base: &Path, vault: &Path, now: jiff::Timestamp) -> Option<String> {
    // A vault with no account is not gated: it is a hand-made folder, it has no cloud step to run,
    // and every fixture vault in this repository is one.
    let cfg = crate::cloudmodel::load(vault)?;
    let profile = profile_id(&cfg.session_credential_target)?;
    let cached = std::fs::read_to_string(cache_path_in(base, profile))
        .ok()
        .and_then(|t| serde_json::from_str::<Cached>(&t).ok());
    if decide(cached.as_ref(), now) { return None; }
    Some("skipped: no entitlement".to_string())
}

/// The production entry point. `LOCALAPPDATA` unset is **not** a refusal: an engine run outside a
/// user session has no app data to read and gating it would turn a missing environment variable into
/// a licence check, which is a different thing from the one ruling 3 asked for.
pub fn gate(vault: &Path) -> Option<String> {
    let base = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty())?;
    gate_in(Path::new(&base), vault, jiff::Timestamp::now())
}
```

**`main.rs` composes the line**, so the word `sync` or `judge` is the caller's, not this module's: hand-off **H4b** prints `format!("{} ({})", name_of(&cli.command), line)` — `sync (skipped: no entitlement)`, `judge (skipped: no entitlement)` — which is the same shape `JudgePlan::Skip` already produces and the same words `CloudError::label()` uses, so a student reads one sentence whichever step hit it first.

- [ ] **Step 5: Apply H3a and H4b**, then run `cargo test --workspace`. The three fixture vaults have no `config/cloud.yaml`, so `oracle.rs` and `surface_oracle.rs` are untouched; assert that in the task report rather than assuming it.

- [ ] **Step 6: Commit.**

```bash
git add engine/src/entitle.rs engine/tests/entitlement_gate.rs
git commit -F .git-commit-msg.txt   # "engine: a slot's cloud steps do not run past the 72-hour grace (C3' Task 8, ruling 3)"
```

---
### Task 9: Restore — a vault's journal replayed into an empty folder, and the wizard that does it

The test the whole feature is judged by, and the one recorded exception to "every note write goes through `write`". Under ruling 2 this is also the *only* restore path there is: there is no code to type and no link to click — a student signs in on a new desktop and Finish fills the folder.

**Files:**
- Modify: `engine/src/sync.rs` (the restore section), `app/static/console.js`, `app/static/index.html`, `app/tests/onboarding.rs`, `app/tests/static_assets.rs`
- Create: `engine/tests/sync_replay.rs`
- **Hand-off this task needs:** **H11a** (`app/src/onboarding.rs`'s fill-from-the-account block), applied by the controller beside this task

**Interfaces:**
- Consumes: Task 6's `Pulled`, `pull`, `pulled_from_reply`, `is_note_path` and `record_is_well_formed`; `ledger::JsonlLedger`; `pystr::write_text`; `cli::run` and `PINNED_FIXTURE_DATE` (the rank comparison); `cloudmodel::resolve`.
- Produces:
  - `sync::Restored { notes: usize, records: usize, empty: bool, warnings: Vec<String> }` (`Default`, `Serialize`)
  - `sync::unexpected_notes(&Path, &[String]) -> Vec<String>`
  - `sync::materialise(&Path, &Pulled) -> Restored`
  - `sync::restore(&Path, &Pulled, &[String]) -> Result<Restored, SyncError>` — the third argument is the **seed allowlist**
  - `sync::restore_all(&Path, &CloudClient, &[String]) -> Result<Restored, SyncError>` — pages `/sync-pull` from zero to the end
  - `sync::restore_into(&Path) -> Result<Restored, String>` — the wizard's one call: resolve, compute the allowlist from `note_paths`, page to the end

- [ ] **Step 1: Ask Quinn (P3)**, in one message, when this task is reached — the text is in the preconditions table. The cloud half is settled by the amendment; what is open is the *local `Backups` mirror* link on the picker. Record the answer; the plan is built to yes (step 6).

- [ ] **Step 2: Write the failing round-trip test** — `engine/tests/sync_replay.rs`:

```rust
//! The round trip, end to end, with no network: a fixture vault's notes and journal are built into a
//! push, fed straight back as a pull, materialised into an empty folder, and compared.
//!
//! **The notes are compared byte for byte.** The journal is compared as a SET OF RECORDS rather than
//! as file bytes, because `JsonlLedger::append` writes `pystr::NEWLINE` (CRLF on Windows) and the
//! fixture's own day files are whatever they are — and because the records, not the bytes, are what
//! the journal is (`ledger.read` sorts them into one order whatever order they arrived in).
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use knowlu_engine::journal::Journal;
use knowlu_engine::sync::{self, Cursor};

/// Everything the source vault would push, read straight back into a `Pulled` — the wire without the
/// wire. The two endpoints are under test in `sync_contract.rs`; this test is about the replay.
fn round_trip(source: &Path) -> sync::Pulled {
    let mut journal = Journal::new(source);
    let (batch, _) = sync::build_push(source, &Cursor::default(), "acct-1", &mut journal);
    assert!(batch.warnings.is_empty(), "{:?}", batch.warnings);
    let body = serde_json::json!({
        "records": batch.records.iter().enumerate().map(|(i, r)| serde_json::json!({
            "seq": i + 1, "device": batch.device, "record_hash": r["hash"], "body": r["body"]
        })).collect::<Vec<_>>(),
        "notes": batch.notes.iter().enumerate().map(|(i, n)| serde_json::json!({
            "rev": i + 1, "device": batch.device, "path": n["path"],
            "deleted": n.get("deleted").cloned().unwrap_or(serde_json::Value::Bool(false)),
            "body": n.get("body").cloned().unwrap_or(serde_json::Value::Null)
        })).collect::<Vec<_>>(),
        "record_cursor": batch.records.len(), "note_cursor": batch.notes.len(), "more": false
    });
    sync::pulled_from_reply(&body).expect("every row reads back")
}

fn records_of(vault: &Path) -> BTreeSet<String> {
    Journal::new(vault).read(None, None).into_iter()
        .map(|r| knowlu_engine::ledger::dumps_value(&serde_json::Value::Object(r))).collect()
}

fn notes_of(vault: &Path) -> Vec<(String, Vec<u8>)> {
    sync::note_paths(vault).into_iter()
        .map(|rel| (rel.clone(), std::fs::read(vault.join(&rel)).expect("read a note"))).collect()
}

#[test]
fn a_fixture_vaults_notes_and_journal_replay_into_an_empty_folder() {
    let source = temp("source");
    copy_tree(Path::new("tests/fixtures/vault-full"), &source);
    let dest = temp("dest");
    std::fs::create_dir_all(&dest).expect("an empty folder");

    let page = round_trip(&source);
    let report = sync::restore(&dest, &page, &[]).expect("restore into an empty folder");

    let before = notes_of(&source);
    let after = notes_of(&dest);
    assert_eq!(before.len(), after.len(), "note count");
    assert_eq!(report.notes, before.len());
    assert!(!report.empty);
    for ((rel_a, bytes_a), (rel_b, bytes_b)) in before.iter().zip(after.iter()) {
        assert_eq!(rel_a, rel_b);
        assert_eq!(bytes_a, bytes_b, "{rel_a} differs after a restore");
    }
    assert_eq!(records_of(&source), records_of(&dest), "every record, as a record");
    let _ = std::fs::remove_dir_all(&source);
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn a_restored_vault_ranks_the_same_day_as_the_one_it_came_from() {
    // Review B3, inherited and unchanged: `state/calendar.md`, `state/events.md` and
    // `state/events-seen.md` are `rank` INPUTS a restore does not carry (a restore carries notes and
    // records), and `golden-today-full.md` proves both reach the rendered day — its line 3 says
    // "2.25h calendar" and its line 5 says "Events: 3 in today's digest". So the comparison copies
    // those three beside `config/` and `profile/`, and the assertion is the whole rendered file.
    let source = temp("rank-source");
    copy_tree(Path::new("tests/fixtures/vault-full"), &source);
    let dest = temp("rank-dest");
    std::fs::create_dir_all(&dest).expect("an empty folder");
    sync::restore(&dest, &round_trip(&source), &[]).expect("restore");
    for rel in ["config", "profile"] { copy_tree(&source.join(rel), &dest.join(rel)); }
    for rel in ["state/calendar.md", "state/events.md", "state/events-seen.md"] {
        let from = source.join(rel);
        if from.exists() {
            std::fs::create_dir_all(dest.join("state")).expect("state");
            std::fs::copy(&from, dest.join(rel)).expect("copy an input a restore does not carry");
        }
    }
    for v in [&source, &dest] {
        knowlu_engine::cli::run(v, Some(knowlu_engine::PINNED_FIXTURE_DATE), "manual", None).expect("rank");
    }
    assert_eq!(
        knowlu_engine::pystr::read_text(&source.join("state").join("today.md")).expect("source today"),
        knowlu_engine::pystr::read_text(&dest.join("state").join("today.md")).expect("dest today"),
        "a restored vault renders the same day",
    );
    let _ = std::fs::remove_dir_all(&source);
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn a_restore_refuses_a_note_it_did_not_put_there_and_tolerates_exactly_the_seeds() {
    // Review B1, inherited. The wizard's vault is seconds old and `scaffold::seed_writes` has
    // already written into it, so a restore that demanded an empty folder could never run from its
    // only caller. The allowlist is what the caller has just written, computed at the moment it runs.
    let dest = temp("allowlist");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    knowlu_engine::pystr::write_text(&dest.join("tasks").join("get-to-know-knowlu.md"), "---\nid: task_0000000001\n---\n").expect("seed");
    knowlu_engine::pystr::write_text(&dest.join("tasks").join("someone-elses.md"), "---\nid: task_0000000002\n---\n").expect("stray");
    let page = sync::Pulled::default();
    let err = sync::restore(&dest, &page, &["tasks/get-to-know-knowlu.md".to_string()]).expect_err("a stray note is a refusal");
    assert!(format!("{err}").contains("someone-elses"), "{err}");
    assert!(sync::restore(&dest, &page, &sync::note_paths(&dest)).is_ok(), "everything on disk tolerated is a restore that runs");
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn a_restored_note_overwrites_the_seed_at_the_same_path_and_an_empty_copy_is_not_a_failure() {
    let dest = temp("overwrite");
    std::fs::create_dir_all(dest.join("tasks")).expect("mkdir");
    let seed = dest.join("tasks").join("get-to-know-knowlu.md");
    knowlu_engine::pystr::write_text(&seed, "---\nid: task_0000000001\n---\nthe seed\n").expect("seed");
    let mine = "---\nid: task_0000000009\n---\nfrom the other desktop\n";
    let page = sync::Pulled {
        notes: vec![sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/get-to-know-knowlu.md".into(), text: Some(mine.into()) }],
        ..Default::default()
    };
    let report = sync::restore(&dest, &page, &sync::note_paths(&dest)).expect("restore over the seed");
    assert_eq!(knowlu_engine::pystr::read_text(&seed).expect("the note"), mine);
    assert!(!report.empty);
    let nothing = sync::restore(&dest, &sync::Pulled::default(), &sync::note_paths(&dest)).expect("an empty copy");
    assert!(nothing.empty, "an empty account copy is a fact, not an error");
    let _ = std::fs::remove_dir_all(&dest);
}
```

- [ ] **Step 3: Run them and watch them fail.** `cargo test -p knowlu-engine --test sync_replay` → FAIL to compile: `sync::restore`, `sync::Restored`, `sync::unexpected_notes`.

- [ ] **Step 4: Write the restore section.** One `materialise` and three callers around it (review I10):

- `unexpected_notes(vault, tolerate)` returns every path under `ids::NOTE_FOLDERS` that is not in `tolerate`, sorted.
- `restore(dest, page, tolerate)` refuses with `SyncError::Io` naming the first unexpected note, then calls `materialise`.
- `materialise(dest, page)` writes each pulled note's text with `pystr::write_text` after `is_note_path` (creating the parent, so a pulled `courses/` path works on a vault that has none), skips a tombstone, appends each well-formed record to `state/journal/<its own ts's UTC day>.jsonl` with `JsonlLedger::append`, and counts what it did. **`empty` is `notes == 0 && records == 0`.**
- `restore_all(dest, client, tolerate)` does the allowlist check once, then pages `/sync-pull` from zero, calling `materialise` for each page and summing.
- `restore_into(dest)` is the wizard's one call: `cloudmodel::resolve(dest)`, `note_paths(dest)` as the allowlist, `restore_all`. **A network failure is not an `Err`** — it is `Ok(Restored { empty: true, warnings: [..] })`, because a student making their first vault on a hotel Wi-Fi should get a vault and a first slot that fills it, not a refusal (hand-off H11a's comment says the same thing from the other side).

**The one recorded exception, in the code where it happens:**

```rust
// **This is the one place in the crate that writes a note without `write`**, and the fidelity
// ledger argues it. Every record here ALREADY EXISTS and carries the `ts`, the `device` and the
// `seq` of the machine that made it. Putting them through `write` would fabricate a second
// `create` record, stamped with THIS device's name and TODAY's `ts`, for a note created three
// months ago on another laptop — and `journal::human_set`, which is what judge-once reads, would
// then answer with the restore instead of with the student's own decision. The invariant exists to
// make attribution complete; re-journalling a restore is the one way to break it while appearing
// to obey it. The bound is the allowlist above: this function never runs into a folder holding a
// note its caller did not name.
```

- [ ] **Step 5: Apply H11a**, then add `app/tests/onboarding.rs`'s case: a wizard finish whose account copy is empty **keeps the vault** and the envelope carries `restored.empty == true`; a finish whose `restore_into` returns `Err` removes the folder it made and returns `ok: false`. The finish panel's copy in `console.js` gains one line, pinned by `app/tests/static_assets.rs`:

```rust
#[test]
fn the_finish_panel_says_when_the_account_had_nothing_to_restore() {
    let js = read("console.js");
    assert!(js.contains("your account had no vault yet"), "the empty-copy sentence is the page's, not an error");
}
```

- [ ] **Step 6 (P3): the picker's *Restore from a backup folder…* link.** `app/static/index.html`'s picker nav gains one button between the two it has:

```html
<button class="b" id="pick-restore-backup">Restore from a backup folder…</button>
```

and `console.js` wires it to `pick_folder` then `restore_vault`, which have both existed since C1 with no caller. **This is the local `Backups\` mirror and nothing to do with the account** — the account's copy arrives by signing in — and the button's own title says so in one sentence. `app/tests/static_assets.rs` pins the button, its handler and the sentence.

- [ ] **Step 7: Run, then commit.** `cargo test --workspace` at 0 warnings; `git status --porcelain --untracked-files=all engine/tests/fixtures/` → empty (the replay copies a fixture, never writes into one).

```bash
git add engine/src/sync.rs engine/tests/sync_replay.rs app/static/console.js app/static/index.html app/tests/onboarding.rs app/tests/static_assets.rs
git commit -F .git-commit-msg.txt   # "engine+app: signing in on a new desktop fills the mirror from the account (C3' Task 9)"
```

---
### Task 10: Git leaves the product

**Precondition P2** is a one-line go, asked when this task is reached. §4.4 has said this since the spec was signed: *"What the app stops doing: git commit/push/rebase (`history.rs`)"*, and §4.1: *"The vault is not a git repository and the app never assumes git exists."*

**Files:**
- Delete: `engine/src/history.rs`
- Modify: `engine/tests/no_console.rs`, `app/static/console.js`, `app/tests/commands.rs`
- Create: `app/tests/no_git.rs`
- **Hand-offs this task needs:** **H3b** (the `lib.rs` removal), **H8b** (`scheduler.rs`), **H9b** (`state.rs` and `commands.rs`), **H10** (`app/src/main.rs`'s startup thread). **All four are compile-blocking together and are the one non-buildable intermediate state in this plan**: the controller applies them as one commit each, back to back, immediately after the implementer's, and the branch is green again at the end of the four. No checkpoint is implied and there is no round trip to wait for.

**Interfaces:**
- Produces: nothing new. This task removes.

- [ ] **Step 1: Write the failing test** — `app/tests/no_git.rs`:

```rust
//! Git is not part of this product (cloud design §4.1, §4.4).
//!
//! **There is no `git2` and there never was.** `history.rs` drove the `git` EXECUTABLE, through
//! `git_with("git", …)` → `Command::new(program)`, so a dependency-graph check alone would have
//! passed on every day the app was still spawning a process every sixty seconds. And the app never
//! wrote `Command::new("git")` either: it called into the engine. So this scans for the names the
//! app actually used — `history::`, `refresh_history`, `refresh_head`, `git_sha` — plus the spawn
//! and the crates, because an assertion that was never true of anything passes vacuously.
use std::path::Path;

fn sources(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).flatten() {
        let path = entry.path();
        if path.extension().map(|x| x == "rs") == Some(true) {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            out.push((name, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    assert!(out.len() >= 10, "the scan found suspiciously few source files in {}", dir.display());
    out
}

#[test]
fn no_git_process_is_spawned_for_a_vault() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut scanned = 0;
    for (name, text) in sources(&src) {
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        for forbidden in ["Command::new(\"git\")", "history::", "refresh_history", "refresh_head", "git_sha"] {
            assert!(!code.contains(forbidden), "app/src/{name} still names {forbidden}");
        }
        scanned += 1;
    }
    assert!(scanned >= 14, "the scan found suspiciously few app source files: {scanned}");
}

#[test]
fn the_engines_git_transport_is_gone_too() {
    // `history.rs` spawned through `git_with("git", …)`, not `Command::new("git")`, so the name to
    // look for on the engine side is the helper — the one that would come back if anyone re-added
    // the module. `runs::git_sha` stays and is named here so its survival is deliberate: a run
    // record's `sha` field is a contract with existing vaults and already answers `None` on a
    // folder that is not a repository, which is every vault the app creates.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("engine").join("src");
    for (name, text) in sources(&src) {
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!code.contains("git_with"), "engine/src/{name} still drives the git executable");
        assert!(!code.contains("SyncLock"), "engine/src/{name} still holds history.rs's lock");
    }
    let runs = std::fs::read_to_string(src.join("runs.rs")).expect("runs.rs");
    assert!(runs.contains("pub fn git_sha"), "runs::git_sha stays: a run record's `sha` is a contract");
}

#[test]
fn the_history_module_is_gone_and_nothing_declares_it() {
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("engine");
    assert!(!engine.join("src").join("history.rs").exists(), "engine/src/history.rs is still here");
    let lib = std::fs::read_to_string(engine.join("src").join("lib.rs")).expect("lib.rs");
    assert!(!lib.contains("pub mod history"), "lib.rs still declares it");
    // `childproc` outlives it: `runs::git_sha` and `runtime.rs` both spawn children, and a GUI
    // application's child must never flash a console window.
    assert!(lib.contains("pub mod childproc"), "childproc must stay");
}

#[test]
fn no_git_library_is_in_any_manifest() {
    for (name, manifest) in [
        ("app/Cargo.toml", include_str!("../Cargo.toml")),
        ("Cargo.toml", include_str!("../../Cargo.toml")),
        ("engine/Cargo.toml", include_str!("../../engine/Cargo.toml")),
    ] {
        for forbidden in ["git2", "libgit2", "gix", "gitoxide"] {
            assert!(!manifest.contains(forbidden), "`{forbidden}` must not be a dependency ({name}): git is not part of this product");
        }
    }
}

#[test]
fn the_page_says_nothing_about_a_repository() {
    let js = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("static").join("console.js")).expect("console.js");
    for word in ["is_repo", "has_remote", "auto-sync", "auto_sync", "pending push", "engine_newer", "vault_head"] {
        assert!(!js.contains(word), "the page still renders {word}, which no longer exists");
    }
}

#[test]
fn the_local_snapshot_mirror_is_untouched() {
    // §5.5: "the local snapshot tick stays". The account's copy answers "I lost the laptop"; the
    // mirror answers "I deleted the file five minutes ago". They are different products.
    let backup = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("engine").join("src").join("backup.rs"),
    ).expect("engine/src/backup.rs");
    assert!(backup.contains("pub fn"), "backup.rs is still here and still a module");
}
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `cargo test -p knowlu --test no_git`
Expected: FAIL on the first three — `app/src/state.rs still names refresh_history`, `engine/src/history.rs is still here`, and the page's words.

- [ ] **Step 3: Ask Quinn (P2)**, in one message: *`engine/src/history.rs` is 707 lines and the last of plan 2's sync work; §4.4 says the app stops doing git and §4.1 says a vault is not a repository. C3′ replaces it. Deleting it is not a reversal, but re-adding it would be a rewrite, so: go?* Record the answer in the task report.

- [ ] **Step 4: Delete the module and the page's git words.**

```bash
git rm engine/src/history.rs
```

In `app/static/console.js`, `renderSyncLine`'s first six lines of body — the comment block naming repo/remote and the five branches from `if (!s.is_repo)` through the `else { … "synced" … }` — are replaced by the block below, and the one further line `if (t.engine_newer) { … }` is deleted. **Everything from `if (b.last_error)` to the closing brace is otherwise unchanged**: the backup, the missed-slot and the last-slot halves of the line are not this stream's.

```javascript
  // C3', Task 10: the topline's cloud-copy/backup/scheduler line. `t.sync` is the engine's
  // `SyncStatus` (state.rs fills it from `sync::run_lines_with`), never computed here. A vault with
  // no account says so calmly: it is a state, not a fault, and the wizard is where it changes.
  function renderSyncLine(state) {
    var t = state.topline, s = t.sync || {}, b = t.backup || {}, bits = [];
    if (s.last_error) { bits.push('<span class="amber">' + h(String(s.last_error).split("\n")[0]) + "</span>"); }
    else if (s.at) { bits.push('<span class="calm">in step with your account</span>'); }
    else { bits.push('<span class="calm">not synced yet</span>'); }
```

- [ ] **Step 5: `engine/tests/no_console.rs`'s floor.** `with_spawns >= 3` becomes `>= 2`, and the assertion message says why: `history.rs` was one of the three engine files that spawned a child process, and it is gone; `runs::git_sha` and `runtime.rs` are the two that remain, and a floor that could never fail would be worse than no floor.

- [ ] **Step 6: Apply the four hand-offs** (**H3b**, **H8b**, **H9b**, **H10**) and rebuild. `cargo test --workspace` at 0 warnings; `app/tests/commands.rs` loses its `vault_head`/`auto_sync` expectations and gains one that the topline's `sync` object is the engine's.

- [ ] **Step 7: Commit.**

```bash
git add engine/src/history.rs engine/src/lib.rs engine/tests/no_console.rs app/tests/no_git.rs app/tests/commands.rs app/static/console.js
git commit -F .git-commit-msg.txt   # "engine+app: git leaves the product — history.rs deleted, the page's repo words with it (C3' Task 10)"
```

(`engine/src/lib.rs` appears in that `git add` only because `git rm` staged the deletion beside it; the **declaration** line is hand-off H3b's own commit. If the two collide, the hand-off wins and the implementer reports it.)

---
### Task 11: The privacy sentences, and the two capability URLs leave the vault

Two removals from the vault and one rewrite of the published promise. The rewrite is the half that must not lag the release: until it lands, the live policy says the opposite of what the product now does.

**Files:**
- Modify: `site/privacy.html`, `app/static/console.js` (the `PRIVACY` var), `engine/tests/site.rs` (its `PRIVACY` const, plus one new case), `app/tests/static_assets.rs`, `app/tests/scaffold.rs`, `app/tests/scheduler.rs`, `engine/tests/sync_contract.rs`
- **Hand-offs this task needs:** **H11b** (`scaffold::ingest_yaml` and one doc sentence in `app/src/lms_link.rs`), **H12** (`site/index.html`'s copy of the sentence); **H5**, **H7** and **H8c** are **verified, not edited**

**Interfaces:**
- Consumes: C2's `/ingest-ics`, `/ingest-calendar?name=personal`, its `cloud:<name>` routing in `cli.rs:241-250`, its `cloud_ics_failure_with_no_local_url` in `ingest.rs`, and `scheduler::ingest_included`.
- Produces: a vault whose `config/ingest.yaml` carries `ics_url: ''` and `- name: personal` / `ics_url: 'cloud:personal'` whenever it has an account.

- [ ] **Step 1: Write the failing tests** — `app/tests/scaffold.rs`:

```rust
#[test]
fn a_vault_with_an_account_carries_no_capability_url() {
    // C3', and §9's Alabama SPII line: a capability URL is a credential in all but name, and the one
    // place it belongs is the account, encrypted, where `PUT /account/sources` already puts it.
    let dest = temp("no-capability-url");
    let mut plan = plan_for(&dest);
    plan.account_id = "acc-1".into();   // `plan_for`'s own default; named here because it is the point
    plan.ics_url = Some("https://lms.example.invalid/feed/secret-capability.ics".into());
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(!yaml.contains("secret-capability"), "{yaml}");
    assert!(!yaml.contains("private-abc"), "{yaml}");
    assert!(yaml.contains("ics_url: ''"), "the key stays, empty, so `ingest` still parses it: {yaml}");
    assert!(yaml.contains("- name: personal\n    ics_url: 'cloud:personal'"), "{yaml}");
}

#[test]
fn a_vault_with_no_account_still_carries_its_own_urls() {
    // A friend who has not signed in still has nowhere else to keep the feed. Nothing about that
    // path changes, and the fallback in `ingest.rs` is what reads it.
    let dest = temp("keeps-its-urls");
    let mut plan = plan_for(&dest);
    plan.account_id = String::new();
    plan.ics_url = Some("https://lms.example.invalid/feed/secret-capability.ics".into());
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(yaml.contains("secret-capability"), "{yaml}");
    assert!(yaml.contains("private-abc"), "{yaml}");
}
```

(`temp` and `plan_for` are the file's own helpers; `plan_for` already sets `account_id: "acc-1"`, which is why the second test clears it explicitly.)

and `app/tests/scheduler.rs`, which **asserts the C2 behaviour this task depends on rather than assuming it** — the superseded plan was going to add a fourth `IcsState` for exactly this and C2's A-2 fix made it unnecessary:

```rust
#[test]
fn a_cloud_vault_runs_ingest_with_no_url_in_the_vault_at_all() {
    let v = scratch("cloud-ingest");
    std::fs::write(v.join("config").join("ingest.yaml"), "timezone: America/Chicago\nics_url: ''\n").expect("ingest.yaml");
    assert_eq!(ics_state(&v), IcsState::NoUrl);
    assert!(!ingest_included(&v), "with no account and no url the step is left out, as it always was");
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://x.example.invalid/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/p/session'\naccount_id: 'acct-1'\n",
    ).expect("cloud.yaml");
    assert!(ingest_included(&v), "an account is a feed, wherever the URL lives");
    let steps = slot_argv(&v, Path::new("knowlu-engine.exe"), &JudgePlan::Skip("judge (skipped: no entitlement)"));
    assert_eq!(steps[0].1[0], "sync", "sync runs first");
    assert_eq!(steps.iter().filter(|(_, a)| a[0] == "ingest").count(), 1, "{steps:?}");
    let _ = std::fs::remove_dir_all(&v);
}
```

and `engine/tests/sync_contract.rs`, for **H7**'s verification — the engine's own half, asserted from the outside:

```rust
#[test]
fn a_cloud_vault_with_no_url_does_not_exit_one_just_for_having_no_url() {
    // C2's A-1 fix already moved the blank-`ics_url` refusal after the cloud attempt, so a cloud
    // vault reaches the service before it can fail. This test is the proof C3' relies on it rather
    // than on the change the superseded plan was going to make (hand-off H7, verified not edited).
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("ingest.rs"),
    ).expect("ingest.rs");
    let cloud_at = src.find("let cloud = crate::cloudmodel::resolve(vault).ok();").expect("the cloud resolve");
    let refusal_at = src.find(r#"return (1, vec!["ingest: no ics_url configured".to_string()]);"#).expect("the refusal");
    assert!(cloud_at < refusal_at, "the empty-URL refusal must stay AFTER the cloud attempt");
    assert!(src.contains("no LMS feed on this account — skipped"), "a 404 is a named skip at exit 0");
}
```

- [ ] **Step 2: Run them and watch them fail.** `cargo test -p knowlu --test scaffold` → FAIL: the yaml still carries `secret-capability`. The scheduler and `sync_contract` cases should **pass immediately** — say so in the report, because a test that was green before the change is evidence about C2, not about this task.

- [ ] **Step 3 (P1): the four privacy sentences and the bullet.** Ask Quinn with the drafts below; they go to the lawyer with the C1 packet's P5 list before merge.

**The sentence (four copies, character for character).** Today:

> Your vault stays on this machine. Knowlu's servers hold your account, the judgments they make for you, and what you correct; they never hold the text of your notes, and nothing here is ever sold or shared.

Drafted replacement:

> Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on shows the same day; we store them encrypted, we never sell or share them, and deleting your account deletes them.

It is one sentence, carries no double quote (it is a JavaScript string literal in `console.js`), and says the three things ruling 2 named: **stored to keep the desktops in step, encrypted at rest, deleted with the account.**

**The bullet at `site/privacy.html:54`.** Today it begins *"**No note bodies stored.**"* and pre-commits the design of a switch this plan does not build. Drafted replacement:

> **Your notes are stored, and here is exactly what that means.** Knowlu keeps your tasks, your notes and the journal of your own edits in your account, so that every computer you sign in on opens on the same day. They are encrypted at rest, they are never sold or shared, they are never used to train anyone's model, and *Delete my account* deletes them. What travels to a model when a judgment is made is still only what the judgment needs: the note's title and up to the first 1,200 characters of its text, your stated preferences and grade weights, and for an email its subject, its sender and up to the first 1,200 characters of the message — for the length of that one call, and written down by neither side.

- [ ] **Step 4: Move all four copies together.** `site/privacy.html:13` and `:54`, `app/static/console.js:1371`'s `var PRIVACY`, `engine/tests/site.rs:15`'s `const PRIVACY` — and `site/index.html:16` through hand-off **H12**, applied in its own commit beside this one. `app/tests/static_assets.rs::the_wizards_privacy_sentence_is_the_sites_privacy_sentence` finds the line by the literal `Your vault stays on this machine`, so **that finder moves too**, to `Your tasks and notes live in`. Three tests hold the four copies in step and all four move together:

```rust
#[test]
fn the_privacy_page_describes_the_account_vault_and_no_switch() {
    // The bullet said, in published words, that note bodies are never stored and that if that ever
    // changed it would be "a separate switch, off by default, with its own screen". Ruling 2 changed
    // it and there is no switch: the copy is what an account IS. A page still promising the switch
    // would be the one thing in this stream that cannot be fixed after the fact.
    let privacy = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
    assert!(!privacy.contains("No note bodies"), "the old bullet is still published");
    assert!(!privacy.contains("separate switch"), "the page still promises a switch nobody built");
    assert!(privacy.contains("encrypted at rest"), "and the new promise is spelled out");
    assert!(privacy.contains("Delete my account"), "with the way out named");
}
```

- [ ] **Step 5: Apply H11b and H12**, then run `cargo test --workspace` and `python scripts/wizard-check.py`.

- [ ] **Step 6: Record the two verified hand-offs.** The task report says plainly that **H5**, **H7** and **H8c** were verified and not edited, and that the superseded plan's H3 and H7c were made redundant by C2's own final-review fixes (A-1 and A-2) rather than dropped. A reader of both plans must not be left wondering which change went missing.

- [ ] **Step 7: Commit.**

```bash
git add site/privacy.html app/static/console.js engine/tests/site.rs app/tests/static_assets.rs app/tests/scaffold.rs app/tests/scheduler.rs engine/tests/sync_contract.rs
git commit -F .git-commit-msg.txt   # "site+app: the account holds the notes, and the two capability URLs leave the vault (C3' Task 11)"
```

`site/index.html` is **not** staged here: it is C1's file and hand-off H12, applied by the controller in its own commit beside this one. `engine/tests/site.rs` **is** staged — it is C3′'s.

---

### Task 12: Close

- [ ] **Step 1: The whole gate, twice.** `cargo test --workspace` at 0 warnings — record the pass count and the exact `warnings: N accepted (.rsrc), N tallies, N other` line, which must end in `0 other`. Then `deno check` / `deno lint` / `deno test` over `cloud/supabase/` **with C2's full flag set**, recording each count. Then `git ls-files --eol cloud/ engine/ app/ site/ docs/` and confirm every `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and `.md` is `i/lf`, every `.ps1` is `i/crlf`, and every fixture is `attr/-text`. Then `.\scripts\ci\eol-check.ps1`. Then `python scripts/wizard-check.py` → `ok` and `python scripts/settings-check.py` (hand-off **H13**, verified).

- [ ] **Step 2: The frozen references.** `git status --porcelain --untracked-files=all engine/tests/fixtures/` → **empty**. The eight Python-written references and the three Rust-generated surface references are untouched; `oracle.rs` and `surface_oracle.rs` pass unchanged, and the reason is structural: none of the three fixture vaults has a `config/cloud.yaml`, so `sync` skips them and `entitle::gate` never fires on them.

- [ ] **Step 3: The account-scoping scan — C3′'s own, in C3′'s own file.** Added to `cloud/supabase/functions/_shared/sync_rows_test.ts`, which C3′ owns. It scans `sync_db.ts` — the only file in this stream that queries anything — so that a `restSelect`, `restUpsert` or `restDelete` naming a sync table **without** `account_id=eq.` fails the suite. The service role bypasses RLS; this scan is the backstop and it is cheap. **And it asserts zero raw fetches**, which the superseded plan could not: with the key generation gone there is no compare-and-set, so every call goes through C1's helpers.

```ts
Deno.test("every sync query is scoped to one account, and nothing bypasses C1's helpers", async () => {
  const src = await Deno.readTextFile(new URL("./sync_db.ts", import.meta.url));
  for (const table of ["sync_records", "sync_notes", "sync_usage"]) {
    for (const call of src.matchAll(new RegExp(`"${table}",\\s*\`([^\`]*)\``, "g"))) {
      assert(call[1].includes("account_id=eq."), `${table}: a query without account_id=eq.: ${call[1]}`);
    }
  }
  assertEquals([...src.matchAll(/rest\.fetch\(/g)].length, 0, "no raw fetch: C1's helpers are the only path");
  assert(!src.includes("sync_generation"), "the key generation is gone");
});
```

- [ ] **Step 4: The ownership check.** `git diff --name-only main...c3-sync` → confirm every path is inside C3′'s ownership as *Global Constraints* lists it, plus the hand-off files as their own commits and `cloud/supabase/migrations/migrations_test.ts` under R-C3-exec-4. **An overlap is a stop, not a rebase**: report it to the controller.

- [ ] **Step 5: The hand-off list.** One section in the task report: **fifteen entries**, each with the exact code from *Controller hand-offs*, the task it landed beside, its commit sha, and what would have broken without it — in the order they were applied: **H1** (Task 1), **H2** (Task 3), **H4a**, **H8a**, **H9a** (Task 7), **H3a** and **H4b** (Task 8, H3a compile-blocking), **H11a** (Task 9), **H3b**, **H8b**, **H9b**, **H10** (Task 10, all four compile-blocking together), **H11b** and **H12** (Task 11), **H5**, **H7**, **H8c** (Task 11, verified), **H13** and **H15** (Task 12, verified), **H14** (Task 12). State plainly which were compile-blocking, and confirm that **no task was reported green with a hand-off it named unapplied**.

- [ ] **Step 6: Docs (hand-off H14).** `CLAUDE.md` gains, in the engine-command list after `judge`:

```
- `sync --vault <v> [--direction pull|push|both] [--via <via>] [--run-id <id>]` — the account's copy
  of the vault: new journal records and changed note text up, another desktop's writes down and
  applied through `write`. **Always exits 0**: no account, no session, no entitlement and no network
  are normal outcomes. The account is the source of truth and the folder is its mirror (cloud design,
  amendment 2026-09-17, ruling 2); the service can read what it stores, says so on the privacy page,
  and deletes it with the account.
```

and a sentence beside it: *"**The engine gates itself** (ruling 3): `coursework`, `ingest`, `judge` and `sync` do not run past the 72-hour entitlement grace the app caches — `engine/src/entitle.rs` reads `%LOCALAPPDATA%\knowlu\profiles\<id>\entitlement.json` and the refusal is a named line at exit 0. `rank`, `surface` and `write` are never gated."* Four edits in the app section: `coursework → ingest → judge → rank` becomes **`sync → coursework → ingest → judge → rank`**; "Seven mutate notes" becomes **eight**, naming `sync` (it applies another desktop's writes through `write` and can file an amend card) and leaving `backup_now` as the one that moves the vault without writing a note; the `history.rs` mention goes; and the Tauri-command paragraph keeps **43 / 30 / 62** with the recount date moved and one clause saying **C3′ added none**. `app/README.md`'s module table loses `history.rs` and its counts do not move. `HANDOFF.md` gains a `▶ C3′ DONE <date>` block. **`VISION.md` needs no change** — commitment 2 and the Sync and Mobile rows were amended in the amendment's own commit.

- [ ] **Step 7: What production still needs**, in the `HANDOFF.md` block:
  - the three migrations pushed to `knowlu-prod` **with `--include-all`** (R-C3-exec-5), and `select public.sync_ceiling_bytes()` read back;
  - the two functions deployed there, and the `knowlu-sync-prune` cron job confirmed active;
  - **the privacy policy and the wizard's `PRIVACY` sentence republished** — P1, and the one item that must not lag the release;
  - **the lawyer's read** of the new stored-notes paragraph, with the C1 packet's P5 list;
  - **`exportAll` should carry the two sync tables.** With the folder export struck (ruling 2), `GET /account/export` is the only way a student takes their data elsewhere; C3′ did not widen C1's export function and Task 3 step 8 recorded the shape;
  - **`sync_usage` is a counter maintained by triggers**: a hand-run bulk `delete from sync_records` leaves it wrong; the repair is `update public.sync_usage set bytes = 0` and the next push rebuilds it;
  - **the ceiling and the retention window are P3's answered values**, not measured ones — revisit after the first ten accounts have a term of history;
  - **`/sync-pull` returns a desktop's own rows** and the device filters them by content hash (P4, answer (a)). A `device=neq.` predicate is the first lever if the pull ever costs anything;
  - **a second desktop has never been run.** Every test here is one machine playing both parts, and that is honest but it is not the same thing. The amend-card path is what to watch, and `READ_LAG_SECONDS` guards the one hazard only two desktops produce — a `seq` taken before a commit that lands after a pull — and has never been exercised by two real pushes either;
  - **`apply` hashes the whole local journal** to build its `known` set, and `build_push` reads the ledger again in the same run. Fine for a first-year vault (50,000 records is about 100 ms of SHA-256, twice a day); worth measuring before it is fine for a fourth-year one;
  - **`sync::restore_into` computes its allowlist from `note_paths` at the moment it runs**, which is exactly right for a vault seconds old and would be exactly wrong for a vault that is not. It is called from one place; keep it that way;
  - **P5's open half** if Quinn has not ruled: whether `rank` joins the gate.

- [ ] **Step 8: Commit.**

```bash
git add HANDOFF.md CLAUDE.md app/README.md docs/plans/2026-09-17-c3-account-vault-plan.md cloud/supabase/functions/_shared/sync_rows_test.ts
git commit -F .git-commit-msg.txt   # "docs: C3' closed — the account holds the vault, the engine gates itself, and git leaves the product (C3' Task 12)"
```

---
## Exit gate

1. **`POST /sync-push` and `GET /sync-pull` are deployed on staging**, both behind C1's `requireActiveEntitlement`, both answering their designed refusals (401 with no bearer, 402 with no subscription, 400 on a malformed row or a bad path, 405 on the wrong method, 413 past the ceiling). **Every endpoint path is the hyphenated function name** — Supabase routes `/functions/v1/<function-name>` — and the loopback contract test asserts the same spelling the deploy uses.
2. **The account's copy is three tables and every row belongs to one account.** `20260912000300_sync_plaintext.sql` is applied to staging with `--include-all`; `sync_generation` is gone and is gone from the purge list; RLS is on with select-only policies; the bodies are bounded in **bytes** on all three sides of the wire (`octet_length` in the column, `TextEncoder` in the validator, `str::len` on the device) and `the rows hold the student's own text, bounded in BYTES` passes.
3. **`knowlu-engine sync` always exits 0** on every one of: no `config/cloud.yaml`, no session, no network, 402, 413, 429, 5xx, timeout, an unparseable `cloud.yaml` — each as a distinct named line on stdout. A vault with no account reports **exactly** `sync (skipped: no account)`, because the checks run `load` → `resolve` in that order (review I4), and the day still ranks.
4. **A pushed row cannot lie about its own identity.** The server re-derives `sha256(body)` and refuses a mismatch; the device re-derives it again on the way back in (`pulled_from_reply`) and skips a row whose hash and bytes disagree. `a record whose hash is not its body's is a 400, and nothing is stored` and `a_records_hash_is_the_sha256_of_the_canonical_bytes_the_server_will_see` both pass, and the bytes in question are `ledger::dumps_value`'s on both machines.
5. **`keep` is the server's, not the client's.** `sync_records.keep` is a stored generated column over the record's own `op` and `actor`; `sync_rows.ts` refuses a push that carries one; `sync_prune` deletes only `and not keep`. `retention never deletes a record a human wrote, and the SERVER is what decides that` passes, and judge-once survives a restore for the reason P3 was asked.
6. **A vault's notes and journal replay into an empty folder**: every note byte for byte at the same path, every journal record present as a record, and `a_restored_vault_ranks_the_same_day_as_the_one_it_came_from` green (it copies `state/{calendar,events,events-seen}.md` alongside `config/` and `profile/`, because those three are `rank` **inputs** a restore does not carry and `golden-today-full.md` proves both reach the rendered day). A restore refuses any note its caller did not name in the seed allowlist, **and the wizard's own path passes exactly what is on disk**, so it runs rather than refusing itself.
7. **A restore is `/sync-pull` from zero**, paged to the end. There is no third endpoint, no restore code to type, no picker link for the account's copy, and no second code path that writes a note file except the two in `sync.rs` that say so in their own doc comments.
8. **Nothing malformed reaches `state/journal/`.** Every pulled record is checked before it is appended — `op` in `journal::OPS`, `via` in `journal::VIAS`, a parseable `ts`, an `actor`, a `device`, an `id` matching `ids::ID_RE` when present, and a `path` that passes `is_note_path` — and a `move`'s destination is checked the same way before `write::move_note` sees it. `a_pulled_record_that_is_not_a_record_never_reaches_the_ledger` and `a_pulled_move_may_not_escape_the_vault` pass, and `is_note_path_and_the_servers_regex_agree` pins the one rule across Rust, TypeScript and the column check.
9. **A field both desktops moved is a `kind: amend` card**, filed through `write::propose_amendment` so the fifteen-a-day cap applies; a second pull of the same field set does not mint a second card; the note keeps this device's value until the card is answered; and a conflict on a field no card could ever apply takes reconcile's rule, keeps its supersede record and says so in one named line. **A foreign record is journalled verbatim** — its own `ts`, `device` and `actor` — and a record this device already has is not applied twice.
10. **Two desktops cannot strand a row between them.** `/sync-pull` reads only rows that settled at least `READ_LAG_SECONDS` ago, so a cursor cannot step past a push that has taken its `seq` and not yet committed; `the clock the read lag uses is the handler's, and it reaches both readers` pins the wiring and Task 4's smoke step observes **both** pulls — the empty one and the one with the row.
11. **The engine gates itself** (ruling 3). `coursework`, `ingest`, `judge` and `sync` on a vault with an account whose cached entitlement is absent, inactive or older than 72 hours print one named line and exit 0; a vault with **no** `config/cloud.yaml` is not gated at all; `surface`, `write`, `runs`, `info`, `issues` and `coursework-discover` are never gated; and `the_engines_grace_is_the_apps_grace_and_the_path_is_the_apps_path` pins the two constants that cannot be linked. **P5's answer is recorded** — whether `rank` joins them.
12. **Git is gone.** `engine/src/history.rs` does not exist, `lib.rs` does not declare it, no file under `app/src/` names `history::`, `refresh_history`, `refresh_head` or `git_sha` — **including `app/src/main.rs`'s startup thread, which H10 deletes** — no file under `engine/src/` names `git_with` or `SyncLock`, `runs::git_sha` survives and is asserted to, no manifest names `git2`/`gix`, and the page renders none of `is_repo`, `has_remote`, `auto_sync`, `pending push`, `engine_newer` or `vault_head`. **`engine/src/backup.rs` is byte-identical to `main`.**
13. **The envelope is gone and cannot come back.** `engine/Cargo.toml` names neither `ring` nor `base64`; `sha2` is the one hash edge and is justified in the test that names it; `engine/src/sync.rs` names no key, no recovery code, no HKDF and no AEAD; `sync_vectors.json` and `sync_envelope_test.ts` are deleted. **This stream adds no Credential Manager entry and no Tauri command**, and the counts stay 43 / 30 / 62.
14. **The two capability URLs are out of the vault** for any vault that has an account: `ics_url: ''` and `- name: personal` / `ics_url: 'cloud:personal'`, with `ingest` still exiting **0** and naming the reason when the service answers 404, and a vault with no account unchanged. `a_cloud_vault_runs_ingest_with_no_url_in_the_vault_at_all` passes.
15. **The published promise is true.** All four copies of the privacy sentence say that the service stores the student's tasks and notes to keep their desktops in step, encrypted at rest, deleted with the account; `site/privacy.html` no longer carries `No note bodies` or the switch it promised; `the_wizards_privacy_sentence_is_the_sites_privacy_sentence`, `the_site_is_plain_html_and_carries_the_privacy_sentence_on_both_pages` and `the_privacy_page_describes_the_account_vault_and_no_switch` all pass; **and Quinn and the lawyer have read it (P1)**. This item alone is a merge blocker.
16. `cargo test --workspace` is green at **0 warnings**; `oracle.rs` and `surface_oracle.rs` pass **unchanged**; `deno check`, `deno lint` and `deno test` over `cloud/supabase/` are green **with C2's full flag set**; no test opens a socket that is not `127.0.0.1`; every hand-off **H1, H2, H3a, H3b, H4a, H4b, H5, H7, H8a, H8b, H8c, H9a, H9b, H10, H11a, H11b, H12, H13, H14, H15** is listed in the final report with exact code and the task it landed beside, and **no commit on `c3-sync` touches a file outside C3′'s ownership** (`git diff --name-only main...c3-sync` proves it).

## What is NOT in this plan

- **Server-originated writes down `/sync-pull`.** Spec §5.5's "Down" bullet lists judgment fields, Gmail-derived notes, event verdicts and rule proposals; **C2 already delivers all four by pull, inside the slot step that asks for them**, journalled through `write` under `agent:knowlu.<kind>` with judge-once intact. The superseded plan's structural reason for not re-routing them (a server-originated row could not be sealed) is gone with the envelope; what decides it now is that re-routing would re-open every C2 test for no behaviour change and would give the service a second way to write a note — one that skips the per-step reporting a student reads in the Runs view. Revisit in C5. Fidelity ledger, row `§5.5 Down`.
- **Supabase Storage.** Two Postgres tables instead, argued in the fidelity ledger. If a vault ever grows past what a `text` column wants to hold, Storage is the move and nothing about the wire changes.
- **Widening `GET /account/export`.** It should carry the two sync tables now that the folder export is struck, and Task 3 step 8 records the shape — but it is a change to the body of C1's own export function, it deserves its own test and its own review, and this stream's hand-off list is deliberately lists-not-bodies. Named in Task 12's production block so it is the next thing somebody picks up.
- **The relay fetch (ruling 4).** `engine/src/{coursework,zybooks,vhl}.rs` are untouched; the host allow-list, the placeholder substitution and the step protocol are **C5's spec and plan**. Task 11's change to the vault's `ics_url` is about a capability URL, not about the fetch sequence, and the two must not be read into each other.
- **Removing the local llama.cpp runtime — C4.** `engine/src/runtime.rs` and `app/src/inference.rs` are left exactly as they are; nothing in this plan extends them.
- **The class-(c) opt-in and its export — C4.** The account's copy is not telemetry and is not training data: it is the student's own vault, held so their desktops agree. Named here so nobody reads the two features into each other, and so the privacy copy of Task 11 is not mistaken for a consent.
- **Real-time sync.** One pull and one push per slot, plus the console's *Sync now* button. A push the moment a note changes would mean a network call on every committed edit and a second scheduler; twice a day plus a button is what §5.5 asks for, and the local snapshot mirror covers the minutes in between.
- **A second desktop, actually run.** Every test here is one machine playing both parts, and that is honest but it is not the same thing. Recorded in Task 12's production block as the first thing to watch.
- **Conflict resolution beyond `reconcile`.** No three-way text merge and no body reconciliation: a note's **body** is never merged — the file that exists locally keeps its body, and a body written on another desktop arrives only for a note this device has never seen. Frontmatter is what the system ranks on and frontmatter is what reconcile settles.
- **Selective sync.** All notes or none. A per-folder switch is a settings screen and a second set of rules for what a restore means, for a product where the whole vault is a few megabytes.
- **Deleting the account's copy on its own.** *Delete my account* removes it and already does (hand-off H1). A second delete path for the same rows would be a second thing to keep correct, and there is no switch for it to hang from any more.
- **`runs::git_sha`.** It fills the `sha` field of a run record, that field is a contract with existing vaults, and on a folder that is not a repository it already answers `None`. Its only app-side caller goes; the function stays. C4 or later may retire the field, with a migration note.
- **A picker link for the account's copy** — ruling 2 replaced it with the wizard's ordinary path. If **P3** is answered *no*, the local-mirror link goes too and this section gains: *the local-mirror restore is deferred; `onboarding::restore_vault` and `restore_vault_in` stay in the crate, registered in the wizard window's handler list, with no caller.*
- **Web and mobile, and a server-run engine.** Ruling 1 closed the parity question as *no* and ruling 6 parks the rest. Nothing here builds toward either, and nothing here forecloses the one door ruling 1 left open.
- **The settings panel's Google connect/disconnect row, the "source went quiet" surface, the console's visual redesign, a second campus, production go-live.** Task 12 records production's list.

### Where this plan narrows the spec, and the recommendation

Three places, all fidelity-ledger rows.

**One. §5.5's "Down" bullet.** The largest narrowing, argued above. **Recommendation: as written**, and revisit in C5 when the relay gives the service a reason to write a note the device did not ask for.

**Two. §5.5's Storage.** Postgres instead, for one access path, RLS, a cursor that is an identity column and a purge list C1 already maintains. **Recommendation: as written**, and revisit only if a vault's corpus stops fitting comfortably in `text`.

**Three. Ruling 3's "refuses to run a slot" is implemented as the four cloud steps, not five.** §5.1 — unmarked by the amendment, therefore standing — promises that past the grace "the slots keep ranking". **Recommendation: as written, and it is precondition P5.** Answer (b) is one line in hand-off H4b plus an amendment to §5.1 in the same commit.

## Deferred minors

Inherited from `docs/reports/2026-09-14-c3-sync-plan-review.md`, with the ones the amendment retired marked so nobody goes looking for them.

| # | Minor | Why it is deferred, or gone |
|---|---|---|
| Review M9 | `note_paths` is flat, so a note at `tasks/sub/x.md` is never pushed, while `apply` and `restore` both `create_dir_all(parent)` for a pulled path | **Half-fixed, half-deferred**, unchanged from the superseded plan. The write side is explicit — `is_note_path` accepts any depth, and the `create_dir_all` is what makes a pulled `courses/` path work on a vault that has none. What is deferred is making the read side recursive: `ids::scan_notes`, `backup::BACKUP_FOLDERS` and every existing engine pass treat the six folders as flat, and a nested note is a shape this product has never produced. Making `note_paths` recursive alone would push a file nothing else in the engine can see. |
| Review M11 | `apply` hashes the whole local journal for its `known` set and `build_push` reads the ledger again in the same run | Fine for a first-year vault — 50,000 records is about 100 ms of SHA-256, twice a day. The fix is a cached hash set in the cursor, which is a second thing to keep true; **recorded in Task 12's production list** with the real numbers so it is measured before it is optimised. |
| Review M13 | The recovery code carries no checksum | **Gone.** Ruling 2 removed the recovery code. |
| Review M20 | The `no_console.rs` floor change | Not a defect: the review confirmed the arithmetic, and it is the same arithmetic here (`>= 3` becomes `>= 2` when `history.rs` goes). Listed so the next reader does not re-derive it. |
| M-a | `/sync-pull` returns rows this desktop pushed, and the device filters them by content hash | **Precondition P4**, with the recommendation to accept: the hash filter has to exist anyway for the re-install case, so a server-side `device=neq.` would be a second mechanism for a saving of one page per slot. |
| M-b | `sync_usage` is a trigger-maintained counter, so a hand-run bulk delete leaves it wrong | The alternative is `sum(octet_length(body))` on the hot path, which is O(rows) per push. The repair is one `update` and the next push rebuilds; it is written down in Task 12 rather than engineered around. The one bulk delete this stream performs — Task 1's `drop table` — resets the counter in the same migration, because `drop table` fires no row triggers. |
| M-c | A note's **body** is never merged | Frontmatter is what the system ranks on and what `reconcile` was written for; a three-way text merge of prose is a different product. A body written on another desktop still arrives whole for a note this device has never seen, which is the case that matters for a second desktop and for a restore. |
| M-d | The commit trailer hard-codes a model name | **Ruled not a defect** (R-C2-6): the trailers name the model and session that *execute* the plan, and the literal in Global Constraints is the writing session's. |
| New | The device sends its whole note set on a first push, one page of 500 at a time | A vault of 500 notes is one page and about a megabyte; a fourth-year vault is two or three pages across two or three slots, each idempotent. A first push that streamed would be a second protocol for a case that resolves itself in a day. |
