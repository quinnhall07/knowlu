# C3′ (the account vault) — Task 12's stream report

Task 12's report: the account-scoping scan, file ownership, the hand-off list, H14's text (as written before the merges; HANDOFF.md carries the version applied on 2026-09-25), the production needs, and the parked-findings appendix. Preserved verbatim from the worktree workspace (`.superpowers/sdd/2026-09-17-c3-account-vault-plan/`, git-ignored) when the stream was closed on 2026-09-25, before the worktree and its branch were deleted.

---

# Task 12: Close — report

Executed against BASE `b0898ce` (branch `c3-sync`, worktree `.claude/worktrees/c3-sync`). No
subagents were used; every command below was run directly by this session.

## Step 1: The whole gate, twice

**Run 1** (`RUSTFLAGS="-D warnings" CARGO_TERM_COLOR=never cargo test --workspace --no-fail-fast`):
exit 0. 1403 passed, 0 failed, 4 ignored (by design, unchanged: the two `events.rs` traps, the
`runtime.rs` real-runtime smoke test, `app/tests/scheduler.rs::run_slot_end_to_end`).
`warnings: 1 accepted (.rsrc), 2 tallies, 0 other`.

**Run 2** (same command, same flags): exit 0. 1403 passed, 0 failed, 4 ignored.
`warnings: 1 accepted (.rsrc), 2 tallies, 0 other`. Identical to run 1.

`oracle.rs`: 3 passed (`migrated_vault_renders_identically`, `today_renders_identically_to_golden`,
`full_vault_renders_identically`). `surface_oracle.rs`: 4 passed (`surface_refuses_an_unknown_view_with_exit_2`,
`surface_today_s1_migrated_is_frozen`, `surface_today_s1_is_frozen`, `surface_today_full_is_frozen`).
Both unchanged from their pre-Task-12 shape.

**Cloud, with C2's full flag set** (per Global Constraints' "whole tree" command, scoped to
`cloud/supabase/` — no `cloud/eval/`, which is C2's and outside this plan):
- `deno check --config cloud/supabase/deno.json` over all 118 `.ts` files under `cloud/supabase/`
  (functions/**, migrations/*, the two root test files): all check clean.
- `deno lint --config cloud/supabase/deno.json cloud/supabase/`: `Checked 120 files`, clean.
- `deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/`:
  `ok | 423 passed | 0 failed (8s)`.

**EOL scan** (`git ls-files --eol cloud/ engine/ app/ site/ docs/`, 412 files): every `.md` (70),
`.rs` (92), `.css` (2), `.js` (1), `.html` (7), `.ts` (126), `.sql` (24) is `i/lf`. No `.ps1` file
exists under these five roots (they live under `scripts/`), so there is nothing to check there in
this scan. All 64 files under `engine/tests/fixtures/` carry `attr/-text` (confirmed also with
`git check-attr text -- engine/tests/fixtures/golden-today-s1.md` → `text: unset`). Zero violations.

**`.\scripts\ci\eol-check.ps1`**: `eol contract holds over 440 files`.

**`python scripts/wizard-check.py`**: `ok`, exit 0.
**`python scripts/settings-check.py`**: `ok`, exit 0.
(Hand-off **H13**, verified — nothing to edit, per the plan's own text.)

## Step 2: The frozen references, and the backup mirror

- `git status --porcelain --untracked-files=all engine/tests/fixtures/` → empty output, exit 0.
  No task in this close touched a fixture.
- `git diff --quiet main -- engine/src/backup.rs` → exit 0. `engine/src/backup.rs` is byte-identical
  to `main` (local `main` at `642a749`, which is ahead of `origin/main` at `312d833` with docs-only
  commits; the merge-base of `main` and `c3-sync` is `5aacf65`). This is the check exit-gate item 12
  names in place of a `pub fn` scan (review M4).
- The eight Python-written fixtures and the three Rust-generated surface references are therefore
  untouched, and `oracle.rs`/`surface_oracle.rs` pass unchanged (Step 1). None of the three fixture
  vaults carries a `config/cloud.yaml`, so `sync` skips them and `entitle::gate` never fires on them
  — structurally, not by luck.

## Step 3: The account-scoping scan

Added verbatim (exactly the brief's code block) to the end of
`cloud/supabase/functions/_shared/sync_rows_test.ts` — the file C3′ already owns:

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

Diff is purely additive (`+11` lines, `+0` removed), LF-only (0 CR bytes checked with Python), and
lints clean. Run alone: `13 passed | 0 failed`. What it actually catches, walked against
`cloud/supabase/functions/_shared/sync_db.ts`:

- `readRecords`/`readNotes` (`restSelect`, `"sync_records"`/`"sync_notes"`, a backtick query
  template) — matched, and each template contains `account_id=eq.${accountId}` literally, so the
  assertion passes.
- `bytesUsed` (`restSelect`, `"sync_usage"`, a one-line backtick template) — matched, same result.
- `saveRecords`/`saveNotes` (`restUpsert`, `"sync_records"`/`"sync_notes"`, then a `rows` **variable**,
  not a backtick literal) — the regex requires a backtick immediately after the comma, so these two
  calls are **not** matched at all. This is not a gap in enforcement: `restUpsert` takes no query
  string to scope — every row it writes already carries its own `account_id` field (set by the
  caller before the array reaches `saveRecords`/`saveNotes`), and the scan's job, per the brief's own
  text, is queries built as PostgREST filter strings. Noted here so the next reader does not read the
  silence as a hole.
- `ceiling()` (`restSelect`, `"sync_limits"`, no `account_id` at all) is correctly **exempt**: the
  scan's table list is `["sync_records", "sync_notes", "sync_usage"]`, not `sync_limits`, which is a
  global row with no account column (ledger m11: "`sync_db.ts:2` says five calls (six;
  `ceiling()` global — Task 12's scan must exempt `sync_limits`)"). Confirmed exempt as required.
- `assertEquals([...src.matchAll(/rest\.fetch\(/g)].length, 0, …)` — 0 matches; `sync_db.ts` never
  calls `rest.fetch` directly, only `db.ts`'s `restSelect`/`restUpsert` wrappers.
- `!src.includes("sync_generation")` — true; the key generation left with the envelope at Task 2.

This is the file's 13th test (12 existed before it).

## Step 4: The ownership check

`git diff --name-only main...c3-sync` (three-dot, against local `main` at `642a749`) lists **74**
paths. `c3-sync` was rebased onto `c1b-sign-in` at `773f529` (R-C3′-exec-0) and later took a
`--no-ff` merge of `main` at `8e6d8e2`, so this diff is not C3′'s diff alone — it is everything that
separates `main` from a branch stacked on an unmerged prerequisite (PR #9, C1b). Every one of the 74
paths was checked against *who actually touched it*, using `git log 773f529..c3-sync --oneline --
<path>` (commits unique to c3-sync since the rebase point) filtered further against the merge commit
`8e6d8e2` and the one main-side commit it carried, `0fb375b` ("test: pin \"today\" for
decide_inner's approval-expiry test instead of the real clock" — reachable from `8e6d8e2^2`, not
`^1`: it is main's own commit, not C3′'s).

**Result: every path is accounted for, and there is no unlisted overlap.** Seven buckets:

1. **38 files — inside C3′'s explicit ownership** (Global Constraints' bullet list, verbatim):
   `Cargo.lock`, `engine/Cargo.toml`, `engine/src/sync.rs`, `engine/src/entitle.rs`, the deletion of
   `engine/src/history.rs`, all 6 of `engine/tests/{dependency_boundary,entitlement_gate,no_console,
   site,sync_contract,sync_replay}.rs`, all 8 of `app/tests/{commands,no_console,no_git,onboarding,
   report,scaffold,scheduler,static_assets}.rs`, all 3 of `app/static/{console.css,console.js,
   index.html}`, `site/privacy.html`, this plan file, all 6 of
   `cloud/supabase/functions/{sync-pull,sync-push}/{handler.ts,handler_test.ts,index.ts}`, all 3 of
   `cloud/supabase/functions/_shared/{sync_db.ts,sync_rows.ts,sync_rows_test.ts}`, all 4 of
   `cloud/supabase/migrations/20260912000{100,200,300,400}_*.sql`, and
   `cloud/supabase/migrations_sync_test.ts`.
2. **14 files — a "shared file with a single owner" (HANDOFF §2), touched only via a named hand-off
   commit (or that hand-off's own fix-round continuing the same feature, reviewed as part of that
   task):** `app/src/{account,commands,lms_link,main,onboarding,report,scaffold,scheduler,state,
   tray}.rs`, `engine/src/{lib,main}.rs`, `cloud/supabase/functions/account/index.ts`. Every touching
   commit traces to H16, H4a/H8a/H9a, H3b/H8b/H9b/H10, H11a, H11b (+ its amendment), H1a (Task 2),
   H3a, H4b, or H1/H5 — see Step 5 for the exact commit list per file.
3. **2 files — a controller ruling explicitly reclassifying how a hand-off half lands**, not a silent
   edit: `cloud/supabase/migrations/migrations_test.ts` (corpus-pin bumps, R-C3-exec-4 — the named
   exception Global Constraints itself carves out) and `cloud/supabase/config.toml` (R-C3′-exec-6:
   `[functions.sync-push]` landed in Task 3's own commit `72b03d4`, `[functions.sync-pull]` in Task
   4's own commit `067fe52`, because `config_toml_test.ts` fails on a section with no directory and a
   directory with no section — H2's controller-applied form would leave the tree red between the two
   tasks).
4. **2 files — ordinary engine modules with no cross-stream ownership claim at all**: `engine/src/
   approvals.rs` and `engine/src/write.rs`, extended by Task 6 (`ff716e3`) and Task 6b (`520eda1`).
   Neither file appears in HANDOFF §2's "shared, single owner" list or in any stream's exclusive-
   ownership row; C1's and C2's exclusive-ownership rows have themselves retired now that both are
   merged (Global Constraints: "C1 and C2 are merged, so their exclusive-ownership rows retire").
5. **1 file — a minor, flagged for the controller**: `scripts/wizard-check.py` is in HANDOFF §2's
   single-owner list (`scripts/*.py`), and Task 9's own fix round 1 (`caec555`, M3) extended it by 39
   lines to cover the new "Restore from a backup folder…" name field — not via a separate named
   hand-off. Functionally this is smoke-test coverage tracking a UI change in the same task that made
   the change (the same relationship `app/tests/**`, fully C3′-owned, has to every other task), and
   H13 (Step 1) treats it as **verified**, not untouched. It is not a silent edit — it is named in
   Task 9's own report — but it is the one file in this diff that does not fit either "C3′-owned" or
   "hand-off-covered" cleanly. Reported here per the brief's instruction; judged non-blocking.
6. **2 files — docs-only, cross-cutting, low-risk**: `docs/plans/2026-09-14-c3-sync-plan.md` (the
   superseded predecessor plan; one comment fix during Task 1's review, `212ef07`) and
   `docs/plans/2026-09-17-c1b-sign-in-plan.md` (a controller trailer-template correction touching
   three plans at once, `8c11d3b`, plus the `8e6d8e2` merge).
7. **15 files — inherited from `c1b-sign-in`, not touched by any C3′ commit at all** (zero commits in
   `773f529..c3-sync` for each): `CLAUDE.md`, `app/Cargo.toml`, `app/tests/account.rs`,
   `cloud/supabase/functions/_shared/config_toml_test.ts`, `cloud/supabase/functions/account/
   {handler.ts,handler_test.ts}`, `cloud/supabase/functions/billing-checkout/{handler.ts,
   handler_test.ts,index.ts}`, `cloud/supabase/migrations/{20260917000100_oauth_consent.sql,
   20260922000100_trim_user_metadata.sql,20260922000200_trim_user_metadata_grant.sql}`,
   `cloud/supabase/migrations_test.ts` (a different file from bucket 3's
   `migrations/migrations_test.ts`), `docs/specs/2026-09-17-c1b-sign-in-design.md`,
   `site/signed-in.html`. These are C1b's own commits, present in this diff only because `c3-sync` is
   stacked on `c1b-sign-in`, not yet merged to `main` (merge order: PR #9 first).

38 + 14 + 2 + 2 + 1 + 2 + 15 = 74. Verified programmatically: every path in the diff appears in
exactly one bucket, and every bucket's files appear in the diff (no phantom entries).

**No overlap to stop for.** The one item worth the controller's attention is bucket 5
(`scripts/wizard-check.py`); everything else is either C3′'s own, hand-off-covered, a ruled
exception, unrestricted, or someone else's already-reviewed work riding along on the stack.

## Step 5: The hand-off list — twenty-one entries

Applied order. "Compile-blocking" means the tree would not build with the hand-off missing but its
task's other commits present. "Applied by" follows the plan's own rule (preamble, review I6): H1
(Task 1, before R-C3′-exec-23 existed) was the controller's own commit; from Task 7 on
(R-C3′-exec-23), the **implementer** applies each hand-off verbatim as its own named commit (or
folded into the same commit when compile-blocking made that unavoidable), and the review checks it
line for line against the plan text.

### H1 — `cloud/supabase/functions/account/index.ts`, the purge list (Task 1)
Applied by: **controller**, `0779b0c` (comment wording corrected to the plan's verbatim block at
`85ea281`, Task 1's own review fix, ruling R-C3′-exec-5). Exact code (the plan's three names,
replacing four):
```ts
            "sync_records",
            "sync_notes",
            "sync_usage",
```
Without it: Task 1's own purge test stays red — a name that no longer exists (`sync_generation`)
made every account deletion 404 from PostgREST.

### H2 — `cloud/supabase/config.toml` (Task 3, then Task 4)
Applied by: **implementer, split across two of its own task commits** under ruling R-C3′-exec-6 (not
a separate hand-off commit at all — `config_toml_test.ts` fails on a directory with no `[functions.…]`
section and on a section with no directory, so a single controller commit would leave the tree red
between Task 3 and Task 4). `[functions.sync-push]` landed in `72b03d4` (Task 3); `[functions.sync-pull]`
in `067fe52` (Task 4):
```toml
[functions.sync-push]
verify_jwt = false
[functions.sync-pull]
verify_jwt = false
```
Without it: neither function deploys — `verify_jwt` defaults true, and Supabase's gateway would
answer its own 401 shape before `requireActiveEntitlement` ever ran, breaking the contract
`CloudError::label()` depends on.

### H4a — `engine/src/main.rs`, the `Sync` subcommand (Task 7)
Applied by: **implementer**, `a650877` (bundled with H8a and H9a in one commit, per R-C3′-exec-23,
after Task 7's own commit `f03c84b`). Adds `sync` to the CLI's `use` list, a `Command::Sync { vault,
direction, via, run_id }` variant, and the `match` arm that calls `sync::run_lines` and always
returns `ExitCode::SUCCESS`. Without it: there is no `knowlu-engine sync` command at all — nothing
for the scheduler or the console's *Sync now* to invoke.

### H8a — `app/src/scheduler.rs`, the slot's first step (Task 7)
Applied by: **implementer**, `a650877` (same commit as H4a/H9a). `slot_argv` gains a `sync` step
ahead of `coursework`:
```rust
let mut steps = vec![(exe.to_path_buf(), vec!["sync".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
steps.push((exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
```
Without it: a slot never pulls another desktop's writes before ranking, and §5.5's "pull precedes
rank" promise is unmet.

### H9a — `app/src/state.rs` and `app/src/commands.rs`, the sync status (Task 7)
Applied by: **implementer**, `a650877` (same commit). A **replacement**, not an addition (review
C1): rewrites `state::run_sync`'s body and return type from `knowlu_engine::history::SyncOutcome` to
`knowlu_engine::sync::SyncStatus`, in place — and in the **same commit** removes all three of its
git-shaped callers (`scheduler::sync_step` and both its call sites in `run_slot_inner`; the two
debounced `state::run_sync(&cs)` calls in `spawn`'s housekeeping loop and the `cs.auto_sync`
condition beside the surviving one; `quit_flush`'s git branch, rewritten to push-only). Adds
`ConsoleState.sync: Mutex<knowlu_engine::sync::SyncStatus>` and rewrites `commands::sync_inner`.
Without it: `error[E0428]` (two `run_sync`s) the moment H3a/H3b's module changes land, or — if
applied as a bare addition instead of a replacement — a caller left calling a function whose
signature or existence changed out from under it for the three tasks between Task 7 and Task 10
(review C1/R1, in mirror image of a design the first review draft would have produced).

### H3a — `engine/src/lib.rs`, the addition (Task 8, compile-blocking)
Applied by: **implementer**, `5f76903` (folded into the commit that creates `entitle.rs` and
`entitlement_gate.rs`, because it is compile-blocking — a module referenced before it is declared).
```rust
pub mod entitle;
```
Without it: `engine/src/entitle.rs` exists on disk but nothing in the crate can `use` it —
`error[E0433]`.

### H4b — `engine/src/main.rs`, the entitlement gate (Task 8)
Applied by: **implementer**, `7721c47` (its own commit, after `5f76903`). Adds `entitle` to the CLI's
`use` list; `gated_vault(&Command) -> Option<&PathBuf>` (matches `Coursework`/`Ingest`/`Judge`/`Sync`,
deliberately not `Rank` — precondition P5); the gate check immediately before `match cli.command`,
composing the line itself (`println!("{} ({reason})", name_of(&cli.command));`); and `name_of`.
Without it: `coursework`/`ingest`/`judge`/`sync` keep running past a lapsed or absent entitlement —
ruling 3 of the cloud design's amendment is simply not implemented.

### H11a — `app/src/onboarding.rs`, the wizard fills the mirror from the account (Task 9)
Applied by: **implementer**, `50a750d` (reordered by the controller from `b76070a`/`1b1e0c3` via
`git rebase --onto` + `cherry-pick` — I1's finding that the hand-off didn't build alone because it
called `sync::restore_into` from the task's own commit; the resulting tree is byte-identical to the
pre-reorder state, just in build order). In `create_vault_in`, after `move_session` and before
`finish_or_roll_back`:
```rust
let restored = match knowlu_engine::sync::restore_into(&dest) {
    Ok(r) => r,
    Err(e) => {
        let _ = std::fs::remove_dir_all(&dest);
        return json!({ "ok": false, "error": e, "profile": Value::Null });
    }
};
```
and one key on the returned envelope: `out["restored"] = json!({ "notes": …, "records": …, "empty": … });`.
Without it: a student signing in on a second desktop gets only the wizard's own nine-panel seeds —
their real vault never comes down. `WizardPlan` gains no field (there is no route to flag; ruling 2
made restoring "what Finish does").

### H3b, H8b, H9b, H10 — the git removal (Task 10, all four compile-blocking together)
Applied by: **implementer**, all four in one named commit `074ca28` ("hand-off H3b/H8b/H9b/H10"),
after Task 10's own red commit `517adc3` (`history.rs` deleted, `app/tests/no_git.rs` added — the
plan's one deliberately non-buildable intermediate state). This is the plan's own instruction (they
"are compile-blocking together, the plan's one non-buildable intermediate state"), and the report
records `517adc3` as red by design, `074ca28` as green.

- **H3b** — `engine/src/lib.rs`: deletes `pub mod childproc; pub mod history;` (with the git-sync
  doc comment above them) and replaces it with `pub mod childproc;` alone, with a new comment
  explaining `childproc` outlives `history`. Without it: `engine/src/history.rs` is gone from disk
  but `lib.rs` still declares the module — `error[E0583]`.
- **H8b** — `app/src/scheduler.rs`, git out: deletes the `state::refresh_history(cs);` call before
  the (already-H9a-replaced) sync step and the trailing `state::refresh_head(cs); state::refresh_history(cs);`
  pair. (`fn sync_step` and its two call sites were already gone — H9a's job, at Task 7 — so this
  hand-off is only the `history`-shaped remainder, three `refresh_*` calls.) Without it:
  `refresh_history`/`refresh_head` calls survive referencing functions H9b deletes in the same
  commit — `error[E0425]`.
- **H9b** — `app/src/state.rs` and `app/src/commands.rs`, git out: removes the `HistoryStatus` import,
  the `history`/`head_sha`/`auto_sync` fields and their initialisers, and `refresh_history`/
  `refresh_head` themselves (state.rs); removes the three git-derived topline keys
  (`vault_head`, `engine_newer`, and the old `cs.history`-sourced `topline.sync` line, replaced by
  `cs.sync`) and the `auto_sync` topline key (commands.rs). `run_sync` is untouched here — H9a already
  replaced it at Task 7. Without it: `state.rs` and `commands.rs` reference fields/types that no
  longer exist once `history.rs` is gone — multiple `error[E0609]`/`error[E0433]`.
- **H10** — `app/src/main.rs`, the startup thread: deletes the `std::thread::spawn` block in
  `run_console`'s `setup` that called `refresh_head`/`refresh_history` on a background thread at
  launch. Without it: the startup thread calls two functions that no longer exist —
  `error[E0425]`, and (per exit-gate item 12) `no_git_process_is_spawned_for_a_vault`'s scan of every
  file under `app/src/` would still find a dead reference even if the build somehow tolerated it.
  H10 also records that the two `generate_handler!` lists are untouched — C3′ adds no Tauri command,
  confirmed again in this report's Step 6 recount (42/29/61, unchanged).

Six additional deviations the implementer recorded and the reviewer accepted (tray.rs and report.rs
reading `cs.history.last_error` moved to `cs.sync`; a second `refresh_*` call site the hand-off text
did not enumerate) are in `task-10-report.md`; none contradicts the hand-off's own text, they extend
it to call sites the hand-off's prose named by intent rather than by line number.

### H11b — `app/src/scaffold.rs` and `app/src/lms_link.rs` (Task 11), plus its amendment
Applied by: **implementer**, `66bb8f6` (the hand-off) and `1e169fc` ("hand-off H11b amendment",
Task 11's own fix round 1, I1). `66bb8f6`: in `scaffold::ingest_yaml`, the LMS `ics_url` line becomes
account-aware (`ics_url: ''` when `p.account_id` is non-empty; the old direct write only when it is
empty), and the personal-calendar entry becomes `- name: personal\n    ics_url: 'cloud:personal'\n`
on an account vault instead of writing the pasted address; `lms_link.rs` gets one doc-sentence
replacement ("the account is the only writer of this URL from C3′ on"). `1e169fc` (the amendment,
review I1: a failed account save of a pasted calendar link would otherwise lose it for good) adds a
per-feed "stored" flag threaded from the paste into the wizard plan, retries only the feeds that were
not stored at Finish, and falls back to writing the URL into the vault if the retry also fails — so a
transient network error never silently drops a student's calendar. Without either commit: a
capability URL — a credential in all but name — sits in plain text in a vault that has an account
(§9's Alabama SPII line), or (without the amendment) a legitimate paste is lost on a flaky Finish.

### H12 — `site/index.html`, the second copy of the privacy sentence (Task 11)
Applied by: **implementer**, `6632bd1`. The plan names this as the controller's file
(`site/index.html` is C1's), but records "the controller applies this one" as a plan-level
instruction that Task 11's implementer executed directly under R-C3′-exec-23's now-standing
precedent (the same precedent every hand-off from Task 7 on used) — replaces the sentence at
`site/index.html:16` with the identical new sentence Task 11 wrote into `site/privacy.html:13`,
character for character. Without it: `engine/tests/site.rs`'s
`the_site_is_plain_html_and_carries_the_privacy_sentence_on_both_pages` fails — the download page and
the privacy page would say two different things about what the service stores.

### H5, H7, H8c — verified, no edit (Task 11)
No commit exists for any of the three; each is a confirmation that earlier work (C2's own fixes)
already does what the superseded plan's hand-off would have done, recorded in `task-11-report.md`:
- **H5** (`engine/src/cli.rs`) — C2's own H4 already routes `cloud:<name>` to
  `/ingest-calendar?name=<name>`; nothing new needed once the vault carries `cloud:personal` instead
  of an address.
- **H7** (`engine/src/ingest.rs`) — C2's final-review fix A-1 already moved the blank-`ics_url`
  refusal after the cloud attempt, so a cloud vault with no local URL exits 0 with a named skip. The
  superseded plan's own H3 is redundant, not dropped.
- **H8c** (`app/src/scheduler.rs`, `ics_state`) — C2's A-2 fix (`ingest_included`) already makes a
  cloud vault run `ingest` regardless of `ics_state`; no fourth `IcsState` variant is added.

Without verifying these three, Task 11 (and this close) would either duplicate work C2 already did or
miss that it was already done.

### H13 — `scripts/wizard-check.py` and `scripts/settings-check.py` (Task 12, verified)
No edit — the point of this hand-off is that C3′ adds no Tauri command, no wizard route and no
settings row, so both scripts already describe the app correctly. Verified in this close's Step 1:
both print `ok`, exit 0.

### H15 — `.github/workflows/ci.yml` (Task 12, verified, C0's file)
No edit. Verified: `main`'s `cloud` job already runs
`deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/`
— exactly the command this close's Step 1 used, over the whole `cloud/supabase/` tree with no
directory list to fall out of date. `deno check` globs every `.ts` under `functions/`; `deno lint`
walks `cloud/supabase/` whole. No path-list edit is needed for C3′'s new files.

### H16 — `app/src/account.rs`, `PRIVACY_VERSION` (Task 11, ruling R-C3′-exec-35)
Applied by: **implementer**, `3894693` ("hand-off H16"). One line:
```rust
-pub const PRIVACY_VERSION: &str = "2026-09-17";
+pub const PRIVACY_VERSION: &str = "2026-09-24";
```
`app/tests/static_assets.rs`'s `the_privacy_version_constant_is_the_published_pages_date` reads the
constant live against `site/privacy.html`'s own Effective-date line, so no test literal needed to
move with it. Without it: the constant and the page's own printed date disagree, and the test that
pins them together fails. This is the hand-off added beyond the brief's twenty (ruling R-C3′-exec-35,
2026-09-24: storing note text is a new privacy-page collection category, so the version moves, and
the page now promises to ask before a materially different use begins — see Step 7(a)).

### H14 — `CLAUDE.md`, `HANDOFF.md`, `README.md`, `app/README.md`, `VISION.md` (Task 12)
Written here (Step 6), applied by the **controller** as its own commit immediately after this task's
Step 8 commit — never `git add`ed by this session. See Step 6 for the full replacement text.

**Count: 21** (H1, H2, H3a, H3b, H4a, H4b, H5, H7, H8a, H8b, H8c, H9a, H9b, H10, H11a, H11b + its
amendment, H12, H13, H14, H15, H16 — 20 named slots plus H16, with H11b's amendment folded into
H11b's own entry rather than a 22nd bullet, per the task's own framing). **There is no H6**: the
superseded plan's numbering is kept where a hand-off survived it, and the gap is deliberate (review
M12). **Compile-blocking:** H3a (alone) and H3b+H8b+H9b+H10 (together) are the only ones; every other
hand-off's absence would leave a red *test*, not a red *build*. **No task was ever reported green
with a hand-off it named unapplied** — confirmed by reading every task's own "DONE" ledger line
against its hand-off list above: Task 1 names H1 and reports it applied at `0779b0c` before going
green; Task 3 names H2's first half and Task 4 its second, each applied in-task before that task's
own review closed; Task 7 names H4a/H8a/H9a and reports `a650877` before its review starts; Task 8
names H3a/H4b and reports `5f76903`/`7721c47`; Task 9 names H11a and reports `50a750d`; Task 10 names
all four of H3b/H8b/H9b/H10 and reports `074ca28`, with `517adc3` explicitly logged as the plan's one
intended red intermediate rather than a green claim over red code; Task 11 names H11b/H12/H16 and
reports `66bb8f6`/`6632bd1`/`3894693` (plus the amendment `1e169fc` in its own fix round), and
verifies H5/H7/H8c without claiming an edit for them.

## Step 6: H14 — the docs text

Written here; **not applied** to `CLAUDE.md`, `HANDOFF.md`, `README.md`, `app/README.md` or
`VISION.md` by this session (none of the five was `git add`ed). The controller applies this section
verbatim as H14's own commit, immediately after this task's Step 8 commit, and the two are reviewed
together.

### CLAUDE.md — insert after the `judge` bullet, before `surface`, in "The engine's commands"

```
- `sync --vault <v> [--direction pull|push|both] [--via <via>] [--run-id <id>]` — the account's copy
  of the vault: new journal records and changed note text up, another desktop's writes down and
  applied through `write`. **Always exits 0**: no account, no session, no entitlement and no network
  are normal outcomes. The account is the source of truth and the folder is its mirror (cloud design,
  amendment 2026-09-17, ruling 2); the service can read what it stores, says so on the privacy page,
  and deletes it with the account.
```

and, as its own sentence immediately after (matching the plan's exact wording):

```
**The engine gates itself** (ruling 3): `coursework`, `ingest`, `judge` and `sync` do not run past the
72-hour entitlement grace the app caches — `engine/src/entitle.rs` reads
`%LOCALAPPDATA%\knowlu\profiles\<id>\entitlement.json` and the refusal is a named line at exit 0.
`rank`, `surface` and `write` are never gated.
```

### CLAUDE.md — four edits in "Knowlu (the app)"

**1. The slot chain.** Current: `` A slot is `coursework → ingest → judge → rank` (`scheduler::slot_argv`) ``.
Replace with:

```
- A slot is `sync → coursework → ingest → judge → rank` (`scheduler::slot_argv`), each the sibling
  `knowlu-engine.exe` as a child process (`KNOWLU_ENGINE_EXE` overrides). Steps are left out and
  named — `ingest (skipped: no ics_url)`, `judge (skipped: no runtime)` / `(skipped: no model)`,
  `sync (skipped: no account)` / `(skipped: no entitlement)` / `(skipped: another sync is running)` —
  never run-and-failed: a non-zero step means retry backoff and an amber tray. The scheduler is inert
  unless the vault's `config/runners.yaml` `local` entry says `scheduler: app` for this `device:`; a
  wizard-created vault carries both from birth.
```

**2. "Seven mutate notes" becomes eight.** Current paragraph's tail (`Tauri commands, recounted…`
through `Recount before quoting a number.`) becomes:

```
- `app/src/commands.rs` computes nothing itself; every vault write goes through the engine's `write`
  with `console_ctx()` (`via: "dashboard"`). **Tauri commands, recounted 2026-09-24 (C3′ Task 12, by
  script, over the two `generate_handler!` lists in `app/src/main.rs`; C3′ added none)**: the console
  window registers **42**, the vault-less picker/wizard window **29** (+3 from C2's hand-off H9 phase
  (a) — `account::google_connect_url`, `account::google_connected`, `account::open_external`; C1b's
  H1 removed `account::sign_up` and `account::sign_in` with the password and added
  `account::google_sign_in` to both lists) — **61** distinct. Commands live beside the module they
  serve (`commands.rs`, `onboarding.rs`, `account.rs`, `report.rs`, `lms_link.rs`), never all in one
  file. **Eight** mutate notes
  (`set_fields`, `create_task`, `delete_note`, `decide`, `close_info`, `open_issue`,
  `resolve_issue`, `sync` — the last applies another desktop's writes through `write` and can file a
  `kind: amend` card); `backup_now` is the one command that moves the vault without writing a note;
  `ui_event` writes the `state/events-ui/` ledger; everything else touches app data,
  `profiles.json`, the clipboard, the process or the updater — never a note. Recount before quoting a
  number.
```

**3. The `history.rs` mention.** The plan's text anticipated a `history.rs` reference in this section
of `CLAUDE.md` that would need to go. **It is not present in the current file** — C1b's own H3
(`66edd13`) already recounted the Tauri paragraph and left no `history.rs` mention behind, and no
other C3′-relevant text in this section names it. Nothing to remove; recorded so the next reader does
not go looking for it.

**4. Recount date.** Folded into edit 2 above (the paragraph's own recount clause).

### HANDOFF.md — a new `▶ C3′ DONE 2026-09-24` block

Inserted in §1 ("State, in facts"), in the same style and density as the existing `▶ C2 MERGED`/
`▶ C0`/`▶ C1 DONE` blocks:

```
▶ **C3′ DONE 2026-09-24 (twelve tasks executed and reviewed on branch `c3-sync`, worktree
`.claude/worktrees/c3-sync`, stacked on `c1b-sign-in` at `773f529` per R-C3′-exec-0; not yet merged —
merge order is PR #9 (C1b) → C1c → C3′.)** Git leaves the product: `engine/src/history.rs` is
deleted, `app/src/main.rs`'s startup thread with it, and the account is now the source of truth for a
student's vault — `knowlu-engine sync` (new; always exits 0) pulls another desktop's journal records
and note text down and applies them through `write`, and pushes this device's new records and changed
note text up, once a slot (first, ahead of `coursework`), once on the console's *Sync now*, and once
(push only) on quit. Two Postgres tables hold the account's copy in **plain text** — `sync_records`,
`sync_notes`, plus a `sync_usage` byte counter — under RLS, keyed by `account_id`, with a `keep`
generated column the **server** decides; the sealed-envelope design (a device-held AES-256-GCM key, a
recovery code) that the superseded 2026-09-14 plan built retired in Task 2, by the same 2026-09-17
amendment that moved sync itself off git. **Staging carries all four `20260912*` migrations**
(`…000100_sync`, `…000200_sync_usage_prune` — both already live before C3′ started — plus C3′'s own
`…000300_sync_plaintext` and `…000400_sync_note_path_check`, the second a forward-only fix for a
Postgres regex repetition cap `000300`'s own check tripped on staging) **and both functions**,
`POST /sync-push` / `GET /sync-pull`, deployed and smoke-tested with real rows (pushed, pulled after
the ten-second read lag, cleaned up). The entitlement gate is new too:
`coursework`/`ingest`/`judge`/`sync` refuse to run past the 72-hour grace the app already caches
(`engine/src/entitle.rs`, ruling 3) — a named line at exit 0, never a failure; `rank` does not join
them (precondition P5, built to the fallback "(a)": the day keeps ranking past the grace). A second
desktop is real: signing in during onboarding restores the account's copy into the wizard's own
seeded vault (`app/src/onboarding.rs`, hand-off H11a), byte-identical seed detection keeps a
still-untouched wizard placeholder from overwriting real history and vice versa, and a failed restore
rolls back to the pristine scaffold rather than reporting a half-filled vault as empty. The two
capability URLs (the LMS feed, the personal calendar) leave the vault for any account: `ics_url: ''`
and `cloud:personal`, with `ingest` unchanged for a vault with no account (hand-off H11b, plus an
amendment that retries a failed account save of a pasted feed at Finish before ever writing the URL
to disk as a fallback). The privacy page names the new collection honestly — the service stores task
and note text, encrypted at rest, journal kept 400 days except human-written records, purged with the
account, never trained on — and `PRIVACY_VERSION` moved to **2026-09-24** (hand-off H16); the page now
promises to ask before a materially different use begins, which is a release gate (Task 12's
production-needs list, item (a)). **The loop that got here:** Task 6 (the pull-side apply/reconcile
logic) took five fix rounds plus a breaker ruling plus its own Task 6b before a rejected sync card
stopped stranding two desktops on different values; Task 9 (restore) took four fix rounds before a
seed could be told from a hand-edited note by content hash rather than heuristics. Both are the
correctness-critical halves of this stream and both were escalated to a fresh implementer once, per
the plan's own escalation rule. **What production still needs, and what the code does not yet cover
(read in full before the pilot):** Task 12's report,
`.superpowers/sdd/2026-09-17-c3-account-vault-plan/task-12-report.md` — the prod migration/function
push, the privacy re-ask headcount, the export widening, and a list of edge cases (a second desktop
has literally never been run; a stale sync-card note from a pre-fix build could still be accepted on
pull; three-desktop conflict ordering) that are honest, bounded and none of them silent data loss.
```

### README.md — one addition

The Layout section's engine command list (`rank, ingest, coursework, judge, surface, write, info,
issues, runs`) becomes `rank, ingest, coursework, judge, sync, surface, write, info, issues, runs` —
the only change; nothing else in `README.md` names git, history or a remote, so there is nothing else
to retire there.

### app/README.md

**The `history.rs` row.** The plan's H14 text says "the `history.rs` row goes" from the module
table. **There is no such row in the current file** — `app/`'s module table (`## What is in here`)
never listed a `src/history.rs` (there never was one: the deleted file is `engine/src/history.rs`,
and the app only ever *called* it through `state.rs`/`scheduler.rs`, which already have their own
rows describing today's shape). Nothing to remove; recorded so the next reader does not go looking
for a row that was never there.

**A `src/sync.rs`-free note**, added to the `src/state.rs` row (which already mentions the `sync`
field) so a reader of this crate's own module table does not go looking for a sync module here:
append this sentence to the `src/state.rs` row: *"There is no `src/sync.rs` in this crate — the sync
logic is `engine/src/sync.rs`'s; this file only holds the `Mutex<knowlu_engine::sync::SyncStatus>`
the topline reads and the `run_sync` wrapper that calls into it under `vault_io`."*

**The two false git claims in "Never point this at the live vault"** (retiring the wording Task 10's
and Task 11's reviews flagged, carried to this close). Current:

```
`%LOCALAPPDATA%\knowlu\scratch\<stamp>`, turns that copy into its own local-only git repo (no
remote — the console's history writes never touch the real vault's remote), and prints the launch
line.
```
becomes:
```
`%LOCALAPPDATA%\knowlu\scratch\<stamp>`, turns that copy into its own local-only git repo (no
remote — a dev convenience for diffing what one run changed, unrelated to the account sync C3′ added;
nothing in the product itself touches git any more), and prints the launch line.
```

Current:
```
console writes notes, drives `git`, and now also runs scheduled slots inside the vault it's
pointed at.
```
becomes:
```
console writes notes, syncs them to the account, and runs scheduled slots inside the vault it's
pointed at.
```

**Not touched, per the plan's own instruction:** the Tauri-command counts in this file (`Sixty-two…
twenty-six in src/commands.rs, fourteen in src/onboarding.rs, fifteen in src/account.rs, five in
src/lms_link.rs and two in src/report.rs`) are already stale relative to `CLAUDE.md`'s own,
independently-updated 42/29/61 (a C1b-era drift, not a C3′-era one), and H14 says plainly "its counts
do not move" — fixing that drift is a separate hand-off for whoever owns it next, not this one.

### VISION.md — no change

Confirmed: commitment 2 and the Sync/Mobile rows were amended on 2026-09-17, in the amendment's own
commit, before this plan was written from it. This plan implements them as written; nothing in
`VISION.md` disagrees with what landed.

### `docs/surface/anatomy.md` — the git sync line, and what replaces it

Not one of H14's five files, but named explicitly by the orchestrating brief ("retire the claims Task
10 and Task 11's reviews flagged: `docs/surface/anatomy.md`'s git sync line"). Three spots, all in
the same document, all describing the same now-dead git-based sync line.

**§3.1, the sync/backup/scheduler-line paragraph** (currently reads `ConsoleState`'s cached
`HistoryStatus` (`src/history.rs`) and describes `engine_newer` comparing a build SHA against
`ConsoleState.head_sha`). Replace with:

```
- **The sync/backup/scheduler line (Knowlu plan 1, Task 15; rebuilt on the account, C3′ Task 7) rides
  under the topline, on `state.topline.{sync,backup,startup_missed,last_slot,scheduler}`.** Unlike
  every other field this document names, **these are not written by the engine's `surface::topline`**
  — `app/src/commands.rs`'s `build_state_value` copies `topline.sync` / `topline.backup` /
  `topline.startup_missed` straight from `ConsoleState`'s cached `knowlu_engine::sync::SyncStatus`
  (`cs.sync`, filled by `state::run_sync` and by the slot's own `sync` step through
  `state/sync-status.json`) and `BackupStatus` (`src/backup.rs`), and `attach_scheduler` copies
  `topline.last_slot` / `topline.scheduler` from the running `Scheduler` on every poll (spec §3.1:
  "nothing in `commands.rs` computes" — this is copying, not computing). **`topline.auto_sync`,
  `topline.engine_newer` and `topline.vault_head` are gone** (C3′ Task 10, hand-off H9b): they
  compared the vault's git HEAD against this build and reported whether auto-sync was on, and a vault
  has not been a git repository since git left the product. The console's own build stays —
  `topline.console_build` — the diagnostics blob and the issue report both still name it.
- **`startup_missed`** is the scheduler's count of slots that were due while the app was not
  running (Task 12 [of plan 1]) — the console's own answer to "was I asleep for a run."
```

**§4.7, "Sync, backup, the scheduler" — the whole "Repo/sync" bullet list.** Currently five bullets
keyed on `is_repo`/`conflicted`/`has_remote`/`ahead`/`auto_sync`, none of which exist any more.
Replace the **`Repo/sync (state.topline.sync, checked in this order):`** block (the five bullets from
`no is_repo` through `otherwise → synced`) with:

```
**Sync (`state.topline.sync`, an `engine::sync::SyncStatus`, checked in this order):**
- `last_error` set → its first line (amber).
- else `skipped` set → `sync skipped — <reason>` — amber for `no entitlement` and `no session` (the
  two a student can act on: subscribe again, sign back in), calm for `no account` and `another sync
  is running` (ordinary, expected states, C3′ Task 7 review R-C3′-exec-38/-39).
- else `at` set (a sync has completed) → `in step with your account` (calm).
- otherwise (no sync has ever run) → `not synced yet` (calm).

There is no `conflicted`, `ahead`, `is_repo` or `has_remote` any more (C3′ Task 10, git left the
product): a conflicting write is a `kind: amend` card in the deck, not a merge state on the topline,
and there is nothing here to be "ahead" of.
```

**The "Opt-in to git, not automatic" paragraph** (§4.6-adjacent, about `state/events-ui/` and
`commit_ui_events: true`). Currently says `config/planning.yaml`'s `commit_ui_events: true` makes
`history`'s `run_sync` stage the ledger with `git add -f`. `history` no longer exists, and — found by
this close's own search, beyond what any task's review flagged — `engine::uievents::commit_opt_in`
(the function that reads the `commit_ui_events` key) is now **unreachable**: nothing calls it, because
the git-staging code that used to be its only caller is deleted. Replace with:

```
**The opt-in is inert.** `state/events-ui/` is still git-ignored by default, and
`config/planning.yaml`'s `commit_ui_events: true` still parses (`engine::uievents::commit_opt_in`
reads it), but nothing calls that function any more: the git-staging code it fed
(`history::run_sync`'s `git add -f`) left with git itself (C3′ Task 10). The flag is dead
configuration, not a lie the page tells — no test asserts a behaviour for it, and setting it in a
vault today changes nothing. **Never read by the engine's ranking path either way** —
`tests/uievents_isolation.rs` proves it — determinism is untouched. (Found during Task 12's close;
`commit_opt_in`'s removal is recorded in this report's production-needs list rather than done here,
since it is a code change to a file outside this task's docs scope.)
```

## Step 7: What production still needs

Written for `HANDOFF.md`'s block above and for this report. The controller runs every `supabase`
command below — this session ran none (rule: "You never run a `supabase` CLI command").

### From the brief, corrected where the branch has moved past it

- **The migrations, corrected to four, not three.** The brief's Step 7 was written before
  `20260912000400_sync_note_path_check.sql` existed (R-C3′-exec-10, added mid-Task-4 as a
  forward-only fix for a Postgres DUPMAX repetition-count failure `…000300` tripped on staging).
  Production needs all **four** — `…000100_sync`, `…000200_sync_usage_prune`, `…000300_sync_plaintext`,
  `…000400_sync_note_path_check` — pushed with `db push --include-all` (the 2026-09-12 stamp sorts
  before migrations already on prod from other streams, and without the flag the CLI skips it
  silently), then `select public.sync_ceiling_bytes()` read back to confirm the ceiling function
  exists and answers.
- **The two functions**, `sync-push` and `sync-pull`, deployed to prod (`--use-api`), and the
  `knowlu-sync-prune` cron job confirmed active on prod's `cron.job` table.
- **The privacy policy and the wizard's `PRIVACY` sentence republished** — P1, and the one item that
  must not lag the release: all four copies (`site/privacy.html`, `site/index.html`,
  `app/static/console.js`'s `PRIVACY`, `engine/tests/site.rs`'s `PRIVACY`) already agree on this
  branch; what "republished" means for production is `site/` actually deploying to `knowlu.com`
  through the next tagged release, not a further code change.
- **The lawyer's read of the new stored-notes paragraph**, with the C1 packet's P5 list (the same
  packet that already asked about the legal entity's name, the `support@`/`security@` mailboxes,
  Alabama governing law, no arbitration clause). Quinn ruled 2026-09-24 that the merge is **not**
  gated on this (P1 decided by delegation), but a lawyer's read before the **paid launch** stays
  advisable and was noted to Quinn once (ruling R-C3′-exec-35's own closing line).
- **`GET /account/export` should carry the two sync tables.** With the folder export struck (ruling
  2), it is now the only way a student takes their data elsewhere; C3′ deliberately did not widen
  C1's export function (a change to the body of someone else's function deserves its own review), and
  Task 3 step 8 recorded the shape it should take.
- **`sync_usage` is a trigger-maintained counter.** A hand-run bulk `delete from sync_records` leaves
  it wrong (the one bulk delete this stream performs, Task 1's `drop table`, resets it correctly
  because `drop table` fires no row triggers — only a *future* manual delete is the hazard). Repair:
  `update public.sync_usage set bytes = 0`; the next push rebuilds it.
- **The ceiling (200 MiB) and the retention window (400 days)** are P3's answered values, not measured
  ones. Revisit once the first ten accounts have a term of history.
- **`/sync-pull` returns a desktop's own rows too**, filtered by content hash on the device (P4,
  answer (a) — accepted because the hash filter has to exist anyway for the re-install case). A
  `device=neq.` predicate on the server is the first lever if the pull ever costs enough to matter.
- **A second desktop has never actually been run.** Every test in this stream is one machine playing
  both parts — honest, but not the same thing. Watch the amend-card path first; `READ_LAG_SECONDS`
  (ten seconds) guards the one hazard only two real desktops produce (a `seq` taken before a commit
  that lands after a pull) and has never been exercised by two real pushes either.
- **`apply` hashes the whole local journal** to build its `known` set, and `build_push` reads the
  ledger again in the same run — fine for a first-year vault (about 100 ms of SHA-256, twice a day),
  worth measuring before it is fine for a fourth-year one.
- **`sync::restore_into` computes its allowlist from `note_paths` at the moment it runs** — exactly
  right for a vault seconds old, exactly wrong for one that is not. It is called from one place only;
  keep it that way.
- **P5's open half, if Quinn has not ruled:** whether `rank` joins the entitlement gate. Built to the
  fallback "(a)" — `rank` keeps ranking past the grace, per §5.1's standing promise that "the slots
  keep ranking."

### The five items this close's brief could not know

- **(a) The privacy re-ask gate (ruling R-C3′-exec-35, Task 11 review M6).** `PRIVACY_VERSION` moved
  to `2026-09-24` (hand-off H16) and the page now promises to ask before a materially different use
  begins. Before C3′ reaches production: **count the prod accounts whose accepted `privacy_version` is
  older than `2026-09-24`.** If any exist, a re-ask screen must ship before this stream does — the
  consent route (`POST /account/consent`) never updates an existing account's accepted version, by
  design (it would silently backdate consent). If none exist (the expected case before the pilot —
  production has no real sign-ups from C3′-era code yet), the version bump alone is enough.
- **(b) C3′'s migrations and functions to prod**, per the corrected list above (four migrations,
  `sync-push`/`sync-pull`).
- **(c) C1c's `ingest-coursework` redeploy to prod.** It is already on staging (C1c's own stream);
  C3′ neither owns nor touches it, but it is part of the same production-parity push this report's
  migrations/functions items are, and should not be forgotten because it belongs to a different
  stream's ledger.
- **(d) C1c's `20260923000100` first-day-cap migration to prod.** Same reasoning as (c) — a C1c
  artefact, named here because production parity is a checklist item spanning streams, not a single
  plan's job.
- **(e) Merge order and its two concrete hooks.** PR #9 (C1b) merges first, then C1c, then C3′.
  C3′'s merge lands **on top of** C1c's `scheduler.rs` `SlotSteps` recorder (C1c Task 8, R-C1c-8): the
  sync step this stream added to `slot_argv` goes through that recorder, not a bare `steps.push` — the
  version of H8a that ships is the one that composes with C1c's recorder, not the one this branch's
  own `a650877` wrote against C1c-less `scheduler.rs` (a merge-time reconciliation the controller
  already anticipated in the ledger, 2026-09-23). The first-run table already knows `sync` as a step
  name. `cloud/supabase/migrations/migrations_test.ts`'s function-count pin resolves to the **union**
  count once all three streams' migrations are on one branch — not a number any one stream's plan can
  state in advance.

### Appendix — accumulated review findings "parked for Task 12", by ruling

Every one of these was explicitly deferred to this task by name in the ledger. None is fixed by this
close (the brief's eight steps do not ask for code changes beyond Step 3's test); all are catalogued
here so they are not lost. **One is flagged separately below because it is not settled design debt —
it is a ruled fix wave that the branch does not yet contain.**

**⚠ R-C3′-exec-41 — NOT YET APPLIED as of this branch's HEAD (`b0898ce`, identical to this task's
BASE).** Ruled in response to Task 11 re-review 1 (N1–N5 and a nit) as "C3′'s FINAL FIX WAVE," but no
commit after `b0898ce` implements it: (N1) a per-feed "stored" flag so Finish retries only feeds not
already stored, and only a validated link is ever sent to the account; (N2) the 402 message's "until
it reaches your account" wording, corrected to "if it still can't"; (N3) a personal-calendar
retry-success test; (N4/N5) two more wording additions to the privacy page (§6 (vi)/(vii), the
computer name and the retention scope); (the nit) `find(...).expect(...)` at
`sync_contract.rs:2686`. This task's brief does not name it as one of Task 12's steps, and this
session's mandate does not include implementing it — it is reported here, unresolved, for the
controller's decision: fold it into the H14/close commit's sibling work, or dispatch it as Task 11's
own fix round 2 before merge.

**From Task 6/6b's reviews (R-C3′-exec-19, -21, -22; `task-6-rereview-final.md`, `task-6b-review.md`):**
- N17: a late, older write from a third desktop can apply cleanly and settle a card under a false
  "resolved" warning (needs three desktops and late delivery to reach).
- N15 / N15b: a judge-once card and a sync card can both be live on one field — two visible cards, no
  wrong write, but `write::find_pending_amendment` (write.rs:336) ignores `created_by`, so a pending
  sync card suppresses the judge's own re-proposal.
- Whether an approved or rejected sync card's resulting value should count as "human-set" for
  judge-once purposes (ruling R-C3′-exec-22 left it open, consistent with the approve path's existing
  behaviour).
- F1: `apply`'s note pass should refuse a live note row `sync_card_note` recognises, named.
- F2: `SyncCards::find` should also require `kind == amend` or an `approvals/` path (a latent trap,
  not yet a bug).
- m1/m2 (Task 6b review): N23/N24 have no direct contract test yet; `approvals.rs` reaching into
  `crate::sync::ACTOR` is a layering nit, confirmed harmless.

**From Task 7's reviews (R-C3′-exec-26):**
- R2-1: a server-supplied refusal reason can still carry control or bidi characters (NUL, ESC,
  U+202E, U+200B) through `judge::one_line` — best fixed in `one_line` itself, since every
  server string passes through it.
- R2-2: a failed slot status save reaches only the slot log, not the Runs view.
- R2-3: a failure to open the sync lock file writes the status without holding the lock.

**From Task 8's reviews (R-C3′-exec-28):**
- N1: `entitle.rs`'s registry-path match is looser than the app's exact-spelling key
  (`profile_` + 10 lowercase hex) — recipe: compare `as_os_str()`, or reuse `ids::derived_id("profile", …)`.
- N2: the Runs view is built from run records, not `runner-log.md`, so a gated step still shows
  `coursework 0` rather than the skip line main.rs's own comment claims reaches it.
- N3: the spawned entitlement-gate test doesn't assert `sync-status.json`'s absence for the other
  three gated commands, and its output check is a loose substring match.
- N4: a stale `state.rs` line reference in a comment; "lowercase hex" should read case-insensitive.

**From Task 9's reviews (R-C3′-exec-31, -37):**
- The ledger's `JsonlLedger::append` fsyncs on every call, unconditionally, and is shared by every
  write path in the crate — a batched append (one fsync per restore page rather than per record)
  would help a fourth-year vault's restore; recorded here and on C0's own perf list.
- E3's twin, on the failed-restore path only: `apply` can archive a seed when a tombstone arrives
  without checking whether the account already holds an archived copy at that path, and the next
  push then sends the seed over it. Recipe: the same byte-identical-seed check E3 uses, applied as a
  narrow exception in `apply` right after the case-only-rename check.
- E4c/E5 (Task 9, R-C3′-exec-34): prune `seed-hashes.json` entries whose paths no longer exist or no
  longer match; a foreign `move` record should check the note's `id` at the old path before relocating
  it (a pre-existing gap from Task 6, low severity).

**From Task 10's review (M2's "others", M3) — found by this close, not by any earlier review:**
- `engine/src/judgelog.rs:5`'s doc comment still reads `` `history::sync` never commits it `` — a
  stale reference to the deleted module. `app/tests/no_git.rs`'s scanner only covers `app/src/`, so an
  engine-side doc comment like this one was never in its reach. Not fixed here (outside this task's
  docs scope, and a code-comment edit in a file this task does not otherwise touch); flagged for
  whoever next edits `judgelog.rs`.
- `engine::uievents::commit_opt_in` is dead code (M3): nothing calls it now that the git-staging path
  it fed is gone. `docs/surface/anatomy.md`'s replacement text (Step 6) says so plainly rather than
  removing the function; an actual removal is a small, safe follow-up for whoever next touches
  `uievents.rs`.

## Step 8: Commit

```
git add docs/plans/2026-09-17-c3-account-vault-plan.md cloud/supabase/functions/_shared/sync_rows_test.ts
git commit -F <message file>
```

Committed as **`b519da9`** — "cloud+docs: the account-scoping scan, and this plan marked DONE (C3'
Task 12)". 2 files changed, 12 insertions, 1 deletion: the 11-line test addition (Step 3) and the
one-line Status-line replacement (the plan file's header paragraph, marking it DONE 2026-09-24,
naming the worktree, the rebase point, the ledger, both escalations, and — plainly, not hidden — that
R-C3′-exec-41's fix wave is not yet applied). `CLAUDE.md`, `HANDOFF.md`, `README.md` and
`app/README.md` were not `git add`ed. The tree is clean afterward except this untracked report file.

Re-ran the full gate once more after the plan-file edit and before committing (a third full run,
beyond Step 1's required two, because the plan file's Status line is prose only and touches no code —
still confirmed): 1403 passed / 0 failed / 4 ignored, `warnings: 1 accepted (.rsrc), 2 tallies, 0
other`.

## Summary

| Step | Result |
|---|---|
| 1 | Gate green twice: 1403/0/4, `warnings: 1 accepted (.rsrc), 2 tallies, 0 other`. Cloud: check/lint clean, 423/0 tests. EOL scan clean over 412 files (no `.ps1` under the five roots); `eol-check.ps1` holds over 440. `wizard-check.py`/`settings-check.py`: `ok` (H13 verified). |
| 2 | Fixtures untouched; `engine/src/backup.rs` byte-identical to `main`; `oracle.rs`/`surface_oracle.rs` unchanged. |
| 3 | The account-scoping scan added to `sync_rows_test.ts`, verbatim; 13/13. |
| 4 | 74-path ownership diff, all seven buckets accounted for; one minor flagged (`scripts/wizard-check.py`, not hand-off-covered but functionally test coverage); no unlisted overlap. |
| 5 | 21 hand-offs listed with exact code, task, commit, compile-blocking status and applied-by; no task reported green with an unapplied hand-off. |
| 6 | H14's text written for `CLAUDE.md`, `HANDOFF.md`, `README.md`, `app/README.md`, `VISION.md` (no change), plus `docs/surface/anatomy.md`'s git sync line — none applied by this session. |
| 7 | Production-needs list: the brief's items (migrations corrected to four), the five items this brief could not know (a)–(e), and an appendix of every review finding parked for this close, with R-C3′-exec-41 flagged as **not yet applied**. |
| 8 | Committed `b519da9`. |

**Concerns for the controller, in order of weight:**
1. **R-C3′-exec-41 (Task 11 re-review 1's fix wave) is ruled but not on the branch.** `c3-sync`'s
   HEAD before this task's own commit was `b0898ce`, identical to this task's BASE — nothing after
   Task 11's last commit (`b0898ce`) implements N1–N5 or the nit. This close's brief does not name it
   as a Task 12 step, so it was not implemented here; it is reported, not silently dropped or silently
   fixed.
2. `scripts/wizard-check.py` (bucket 5, Step 4) is a "shared, single-owner" file per HANDOFF §2 that
   Task 9 extended directly rather than through a named hand-off. Judged non-blocking (it is smoke-test
   coverage tracking the same task's own UI change), but it is the one file in the ownership diff that
   does not fit cleanly into "C3′-owned" or "hand-off-covered."
3. The appendix's roughly two dozen "parked for Task 12" review findings are real, ruled, and none is
   fixed by this close — they are catalogued, per the brief's own Step 7 pattern (record, don't fix).
4. `app/README.md`'s Tauri-command counts are already stale relative to `CLAUDE.md`'s (a C1b-era
   drift), and per the plan's own H14 text this close does not touch them.

**One test line:** `cargo test --workspace --no-fail-fast` (RUSTFLAGS=-D warnings) — **1403 passed, 0
failed, 4 ignored, warnings: 1 accepted (.rsrc), 2 tallies, 0 other** — run three times across this
close, identical every time.
