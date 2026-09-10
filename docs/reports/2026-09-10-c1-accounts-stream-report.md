# C1 stream report — Task 21 (Close)

Worktree: `C:\Users\danie\GitHub\knowlu\.claude\worktrees\c1-accounts`, branch `c1-accounts`,
HEAD `5e93b06` (unchanged — the gate passed first time, so no `R-C1-58` commit was needed).
Untracked-only in the worktree: `supabase/.temp/**`, the Supabase CLI's local link cache — not
part of the repo, not part of any diff, left alone.

## Step 1 — the full gate

All commands run exactly as the brief gives them, from the worktree root.

### `cargo build --workspace`

```
   Compiling knowlu-engine v0.1.0
   Compiling knowlu v0.1.0
warning: linker stderr: ...ld.exe: .rsrc merge failure: multiple non-default manifests
warning: `knowlu` (bin "knowlu") generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.73s
```

One warning, the accepted `.rsrc` line. Nothing else.

### `cargo test --workspace`

App crate (`target\debug\deps\...`):

| binary | result |
|---|---|
| `knowlu` (lib) | 0 tests |
| `knowlu` (main) | 0 tests |
| `tests/account.rs` | 27 passed |
| `tests/commands.rs` | 24 passed |
| `tests/credentials.rs` | 2 passed |
| `tests/inference.rs` | 19 passed |
| `tests/lms_link.rs` | 20 passed |
| `tests/no_console.rs` | 1 passed |
| `tests/onboarding.rs` | 29 passed |
| `tests/profiles.rs` | 11 passed |
| `tests/report.rs` | 14 passed |
| `tests/scaffold.rs` | 21 passed |
| `tests/scheduler.rs` | 23 passed, **1 ignored** |
| `tests/static_assets.rs` | 41 passed |
| `tests/telemetry.rs` | 9 passed |
| `tests/updates.rs` | 8 passed |

Engine crate:

| binary | result |
|---|---|
| `knowlu_engine` (lib) | 873 passed, **3 ignored** |
| `knowlu_engine` (main) | 4 passed |
| `tests/dependency_boundary.rs` | 3 passed |
| `tests/no_console.rs` | 1 passed |
| `tests/oracle.rs` | 3 passed |
| `tests/site.rs` | 1 passed |
| `tests/starvation.rs` | 4 passed |
| `tests/surface_oracle.rs` | 4 passed |
| `tests/uievents_isolation.rs` | 1 passed |
| `tests/workflows.rs` | 4 passed |
| doc-tests (both crates) | 0 tests |

**0 failed anywhere.** Ignored: 1 (app, `scheduler.rs::run_slot_end_to_end`, by design) + 3
(engine: two `events.rs` traps, `runtime.rs`'s real-runtime smoke test) = **4**, matching
`CLAUDE.md`'s "Four tests are `#[ignore]` by design." **`app/tests/onboarding.rs`'s real-engine
test passed on this run — the `R-C1-58` placeholder-sidecar flake did not occur.** No test file
was touched, no commit was made.

Warnings: same accepted `.rsrc` line, plus cargo's own "generated 1 warning" duplicate-tally
lines (one from the build cache reuse, one marked `(1 duplicate)`) — no other warning text
anywhere in either log.

### Deno — `cloud/supabase/`

`deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`: **124 passed, 0
failed** (1s), across 16 test files — `_shared/{auth,crypto,db,entitlement,http,scrub,stripe}_test.ts`
(4/4/2/5/7/8/4), `functions/{account,billing-checkout,billing-jobs,billing-portal,entitlement,
issues,stripe-webhook,telemetry}/handler_test.ts` (19/4/11/4/4/10/13/18), `migrations_test.ts` (7).

`deno lint --config cloud/supabase/deno.json cloud/supabase/`: **Checked 39 files** — clean.

`deno fmt --check --config cloud/supabase/deno.json cloud/supabase/`: **Checked 49 files** —
clean.

### eol

`git ls-files --eol cloud site app | Select-String -Pattern "i/lf" -NotMatch` returned only:

```
i/-text w/-text attr/text=auto eol=lf   app/icons/icon.ico
i/-text w/-text attr/text=auto eol=lf   app/static/fonts/instrument-sans-{400,500,600,700}.woff2
i/-text w/-text attr/text=auto eol=lf   app/static/fonts/jetbrains-mono-{400,500,600}.woff2
i/none  w/none  attr/text=auto eol=lf   site/releases/.gitkeep
```

All eight are binary (`-text`, correctly excluded from LF normalization) or an empty placeholder
file with no content to carry an eol (`i/none`/`w/none`). No `.ts`, `.sql`, `.rs`, `.html`, `.css`,
`.js` or `.md` file under `cloud`, `site` or `app` is anything but `i/lf`. No `.ps1` file exists
under those three paths, so the `i/lf w/crlf` exception never comes up here.

**Gate: green, first time, at 0 warnings beyond the one accepted line.**

## Step 2 — the recount

Read `app/src/main.rs`'s two `generate_handler!` lists by hand (lines 107 and 186).

**Vault-less shell (line 107) — 27 names:**
`onboarding::{launch_state, pick_folder, pick_file, adopt_vault, open_profile, create_vault,
restore_vault, apply_profile_settings, store_credentials, retarget_credentials,
finish_onboarding, discover_coursework, timezone_for_state, campus_search}` (14) +
`account::{sign_up, sign_in, send_magic_link, verify_email_code, sign_out, open_policy,
entitlement_now, open_checkout}` (8) + `lms_link::{open_lms_window, capture_calendar_link,
capture_courses, paste_calendar_link, close_lms_window}` (5) = **14 + 8 + 5 = 27**.

**Console window (line 186) — 43 names:**
`commands::{state, note, mark_seen, ui_event, set_fields, create_task, delete_note, decide,
close_info, open_issue, resolve_issue, sync, backup_now, get_settings, set_settings,
set_profile_name, copy_diagnostics, copy_text, settings_context, switch_profile,
check_for_updates, install_update, inference_status, install_inference_file,
install_inference_download, remove_inference_model}` (26) + `onboarding::{launch_state,
pick_folder, pick_file}` (3) + `account::{sign_up, sign_in, send_magic_link, verify_email_code,
sign_out, open_policy, entitlement_now, open_checkout, account_status, open_portal,
attach_account, delete_my_data}` (12) + `report::{report_preview, report_send}` (2) = **26 + 3 +
12 + 2 = 43**.

**Overlap between the two lists:** the 3 `onboarding` names in the console list are all inside
the 14 in the vault-less list; the 8 `account` names in the vault-less list are all inside the 12
in the console list. Overlap = 3 + 8 = **11**.

**Distinct:** 27 + 43 − 11 = **59**, matching 26 (`commands.rs`) + 14 (`onboarding.rs`) + 12
(`account.rs`) + 5 (`lms_link.rs`) + 2 (`report.rs`) = 26+14+12+5+2 = **59**.

**Confirmed by hand: 27 / 43, 59 distinct.** The plan's own arithmetic (Amendment 2, "Counts moved
to 27 / 43 / 59") is correct as it stands on the branch; nothing to correct. These are the numbers
H5 and H8 must carry at merge.

## Step 3 — disjointness

`git -C <worktree> diff --name-only main...c1-accounts` — 84 paths. Checked each against
`c1-global-constraints.md`'s ownership line: `cloud/supabase/**` (except C2's
`functions/{judge-*,ingest-*,events,gmail-*}` and `_shared/judge_*.ts` — none touched),
`app/src/{onboarding,profiles,scaffold,credentials,scheduler}.rs`, the new
`app/src/{account,lms_link,report,telemetry}.rs`, `app/static/**`, `app/tests/**`, `site/**`, and
the plan file.

**Cleanly inside ownership (79 paths):**
- `app/src/{account,lms_link,onboarding,report,scaffold,scheduler,telemetry}.rs` (7)
- `app/static/{console.css,console.js,index.html}` (3)
- `app/tests/{account,lms_link,onboarding,report,scaffold,scheduler,static_assets,telemetry}.rs` (8)
- `cloud/supabase/**` — `.gitignore`, `README.md`, `config.toml`, `deno.json`,
  `migrations_test.ts`, `templates/magic_link.html`, all 7 `migrations/2026091000*.sql` files, and
  every `functions/{_shared,account,billing-checkout,billing-jobs,billing-portal,entitlement,
  issues,stripe-webhook,telemetry}/*.ts` (55 files total under `cloud/supabase/`) — none of them
  `judge-*`, `ingest-*`, `events`, `gmail-*` or `_shared/judge_*.ts` (6 paths)
- `site/{index,privacy,signed-in,subscribed,terms}.html`, `site/site.css` (6)

**Outside the literal ownership list, but each a documented, already-applied controller
hand-off or plan-directed exception — not a silent violation, reported per instructions rather
than fixed:**

1. **`engine/src/{main,vhl,coursework}.rs`** — hand-off **H10**, the read-only
   `coursework-discover` subcommand. `engine/**` is expressly "not this stream's to edit," but H10
   was one of the plan's five *mid-stream controller applications* (R-C1-11: H1 → H9a → H10 → H9b
   → H11), reviewed twice (`h10-review.md`, `h10-fix1-review.md`) and reported
   (`h10-report.md`) before landing on the branch. Task facts for this close confirm "H10 is
   applied." No other engine file changed; no fixture regenerated.
2. **`engine/tests/site.rs`** — one constant (`PRIVACY`) updated to the sentence Task 17/19 put in
   `site/privacy.html`, so the existing cross-check test
   (`the_wizards_privacy_sentence_is_the_sites_privacy_sentence`) keeps proving the wizard's
   one-sentence promise matches the site's own — an exit-gate item (§14). This was explicit,
   reviewed plan text (Task 17 brief step 1 and step 6a; named across four review rounds), not
   scope creep: the alternative was landing that test permanently red.
3. **`scripts/campuses-from-ipeds.ps1`, `app/campuses.json`** — hand-off **H11**, applied
   mid-stream (compile-blocking: `onboarding.rs`'s `include_str!("../campuses.json")` will not
   compile without the asset). `progress.md` records "H11 applied on the branch at the 14b/14c
   gap." `app/campuses.json` sits beside `app/src/`, not literally under `app/static/`, but is
   Task 14c's data file for `onboarding::campus_search`, generated (not hand-written) from the
   public-domain IPEDS file per H11's own recipe.
4. **`app/Cargo.toml`, `Cargo.lock`** — companion edits, not their own domain: `app/Cargo.toml`
   adds `regex` (pinned to the version `engine/Cargo.toml` already carries — no new crate in the
   graph) for `report.rs`'s scrubber (Task 16, R-C1-46/C1+I1) and two more `windows` feature flags
   for `report.rs`'s `os_build` (Task 16 fix round 1, M2); both are documented inline with the
   ruling that required them. `Cargo.lock` is cargo's own regeneration from those two additions,
   not a hand edit.
5. **`app/src/main.rs`** — wires the four new modules and both `generate_handler!` lists. The task
   facts for this close state H3 (main.rs's handler lists and `use` line) is "fully applied on the
   branch" (`8279ca9` then Task 18) — a documented acceleration of what would otherwise be a
   merge-time hand-off, verified correct in Step 2 above.

None of the five touches C0's files (`.github/**`, `scripts/release.ps1`, the signing scripts) or
C2's reserved surface (`judge-*`/`ingest-*`/`events`/`gmail-*` functions, `_shared/judge_*.ts`,
`CloudModel`, the eval suite, the rule table, `engine/src/ingest.rs`'s R-OB-3 guard). Every one is
either a formally reviewed, already-landed hand-off the plan itself names, or a mechanical
consequence of one.

**Verdict: clean.** No overlap with C0's or C2's files. The five items above are worth the
controller's eyes at merge (nothing to rebase or remove — the brief is explicit that an overlap is
a stop, not a rebase, and none of these five is a stop: each is either pre-approved plan text or
cargo's own bookkeeping).

## Step 4 — the controller's merge list

Per hand-off, what's already applied vs. what the controller does at merge, and where its text
lives (`close-merge-handoffs.md` collects the verbatim text — cited here rather than re-derived):

- **H1** — pre-flight stub modules (`app/src/lib.rs` + four stubs). **Applied already**, before
  Task 10. Nothing at merge.
- **H9a / H9b** — the two partial `generate_handler!` additions (three commands at Task 13 step 3,
  two more at Task 14b step 3a). **Applied already**, superseded by H3's full lists. Nothing at
  merge.
- **H3** — the two `generate_handler!` lists and the `use` line in `app/src/main.rs`. **Fully
  applied on the branch** (`8279ca9`, then Task 18) and hand-recounted correct in Step 2 (27 / 43,
  59 distinct). Nothing at merge.
- **H10** — `coursework-discover` (`engine/src/{main,vhl,coursework}.rs`). **Applied already**,
  mid-stream, reviewed twice. Nothing at merge.
- **H11** — `scripts/campuses-from-ipeds.ps1` + `app/campuses.json`. **Applied already**,
  mid-stream (compile-blocking). Nothing at merge.
- **H7** — `ci.yml`'s `cloud` job (R-C1-23). **Done** — verified identical on `main` and
  `c1-accounts` (`.github/workflows/ci.yml` lines 56–71, no diff between the branches). Nothing
  at merge.
- **H4 — `app/src/tray.rs`, two menu items.** Not on the branch (`.github`/`app/src/tray.rs` is
  the controller's). Text: `close-merge-handoffs.md` §"H4" — add the `vault-folder` and `report`
  `MenuItem`s, widen the `Menu::with_items` call, and the two `on_menu_event` arms, verbatim.
  *Delete my data* stays deliberately out of the tray (it's a settings-row confirmation flow,
  Task 17).
- **H5 — `app/README.md` recount.** Not on the branch. Text: `close-merge-handoffs.md` §"H5" —
  replace the "Thirty-two commands" paragraph with the 59/26/14/12/5/2 breakdown and the two
  window bullets (43 console / 27 vault-less). The exact numbers match Step 2's hand recount
  above — no correction needed to the hand-off text itself.
- **H6 — `scripts/wizard-check.py` corrections.** Not on the branch (`scripts/` is the
  controller's). Text: `close-merge-handoffs.md` §"H6" — five corrections (H6-1 through H6-5):
  the step-5 second-press capture and "12 events" wording, `open_lms_window`'s fake no longer
  returning `session_dir`, both capture fakes carrying `kind`/`note`, the optional course-code
  assertion, and H6-5's `ageAttested` camelCase fix (Tauri v2 lower-camel-cases argument keys).
  Task 17's report §5 and its fix round are where these were found and written up; nothing in
  `scripts/` was touched by this stream.
- **H8 — `CLAUDE.md`'s two edits.** Not on the branch. (1) The two `generate_handler!` counts →
  **27** and **43**, **59** distinct — confirmed correct in Step 2. (2) The engine's commands list
  gains `coursework-discover [--vault <v>] [--zybooks-target <t>] [--vhl-target <t>]` (read-only,
  always exits 0) — already implemented (H10) and always exits 0, confirmed by
  `h10-report.md`'s manual smoke test.
- **Also at the close** (from `close-merge-handoffs.md`'s trailing list, not a lettered hand-off):
  - `cloud/supabase/templates/magic_link.html` — reword "will tell you the same code" (Task 19
    m8), then `supabase config push` on staging (and prod at rollout).
  - `billing-jobs/handler.ts`'s reminder mail — align "Cancel subscription" with the app's actual
    "Manage subscription" button wording (Task 19 concern 4).
  - `migrations/20260910000100_accounts.sql` — comment-only fix: the consents table's deletion
    comment should say it nulls `ip` too, not only `account_id` (R-C1-56).

## Step 5 — the HANDOFF block

The template's Google-verification line and the Stripe-prod line needed honest filling rather
than invented specifics, per the task's facts (Google's verification is not submitted; the two
project names, not keys; Stripe live mode is prepared but not yet flipped on). Task 13's outcome
is **B**. The controller pastes this block into `HANDOFF.md`:

```markdown
▶ **C1 DONE 2026-09-10:** accounts, entitlement and the new wizard. Supabase `knowlu-staging` and
`knowlu-prod` are live; Stripe is in test mode on staging with the monthly and academic-year
prices, the 7-day trial and the June–August pause, and live mode is prepared on prod (prices,
portal, webhook) pending Quinn's fresh live secret key. `GET /entitlement` is cached on the device
with a 72-hour grace and a slot with no subscription says `judge (skipped: no entitlement)` and
stays green. The wizard is nine panels with no folder question; an install from before C1 is
adopted in place. Telemetry (a) and (b) go up at each slot; issue reports are previewed and
scrubbed before they are sent. The privacy policy and the terms are written and committed to
`site/`, not yet live at `knowlu.com`. **Google's restricted-scope verification has not been
submitted** — P4 waits for the site to actually be live (a verified authorised domain and a live
privacy URL are both required by the consent screen); Gmail stays testing-mode only (100 named
test users, 7-day token expiry) until it lands. LMS link capture: outcome **B** from the Task 13
spike (the sign-in window's cookies fetch the feed URL in one GET and the course list in another).
```

Steps 6–7 (the plan's status line, the close commit) are the controller's, on `main`.
