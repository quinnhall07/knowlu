# Wave B: the long onboarding and one school sign-in (lanes O3 and O4): build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: T0 done and its amendments reviewed (PASS with minors, closed in §14.43; on `main` as
`cb3b661`); S-A phase 1 PASS (§13), so T2 (phase 2, WebView2) is next; F10's review gaps in §9
A10 assigned before F11 (§14.44); a removed course's feed items left out by ingest, new task F6c
(§14.45); shared fragments, the pin form and one source of pairs (§14.46); the kept grades count and class
times on removal (§14.47); F10 review r2's owners and F5b's three retirements (§14.48); an unreadable file never resumes past
the school (§14.49); a failed first grades and a gate that fails closed (§14.50).** T0 ran 2026-10-24 on `feat/o4-school-sign-in` at `1d3c467`. T0's
record is in §13 (dated 2026-10-24) and its amendments in §14.42: seven premises did not hold as
written, and each is amended where it lands (the lanes not yet merged, `grades` and course notes,
the entitlement-refused sync, the feed host, the sync hold, `mode_device`, a `campus.yaml` race).
None reopens a ruling.

Quinn ruled Q-B3, Q-B10, Q-B11, Q-B14 and the direction (5b,
Departures) on 2026-10-23 (§5.0). Q-B15 to Q-B19 stand as decided by the controller; Q-B20, new
at T0, is decided by the controller and Q-B21 ruled by Quinn, both on 2026-10-24 (§5, §14.43). The plan was
reviewed and passed in round 8, and its minors are closed. Round 8 (`docs/reports/2026-10-23-wave-b-plan-review-r8.md`, 2026-10-23)
passed with 0 critical, 0 important and 3 minor findings plus one note, closed in §14.40 with no
further review round. Rounds 1-7 failed and are answered in §14.1-§14.39 (round 1 in §14.1-§14.10,
2 in §14.11-§14.17, 3 in §14.18-§14.22, 4 in §14.23-§14.27, 5 in §14.28-§14.34, 6 in §14.35-§14.38,
7 in §14.39), each also in the sections and tasks it names. Round 5 added a **reuse audit** (§13.1):
every existing function, command, target, lock and seam the plan reuses, checked against the code
for its signature, the credential it reads and its other callers. No finding reopened a ruling.
Nothing is built until its spikes (§3) pass.
§5's questions are all ruled or decided (§5.0; Q-B20 and Q-B21 in §5).

**Date:** 2026-10-23. **Lanes:** O3 and O4, wave B of the onboarding redesign.

**Spec:** `docs/specs/2026-10-23-onboarding-redesign-draft.md` (ruled by Quinn, 2026-10-23; the
filename keeps `-draft`). This plan carries §1 (the long onboarding), §2 (one school sign-in), the
part of §4.2 that moves the registrar into the school step, the reveal's count that lane O1 deferred
(§6.3, O1's Q-B), and §8's measure. The rulings it builds:
- **Direction (Quinn):** one long onboarding where every login happens, each step opening with why it
  helps and ending with proof; the main page appears only after everything has run once.
- **Q1:** one kept school sign-in (option C), built after spike S-A passes.
- **Q2:** payment right after the school step's proof (option B); no entitlement gate changes.
- **Q7:** the reveal waits up to 3 minutes after "Your week", then names what is still running;
  backfills are never waited for.
- **Q9:** one "Forget school sign-in"; the profile goes with its last use, and always on sign-out or
  account deletion.
- **Q12:** optional sources may be skipped, each with its cost named; account, payment, "Your week"
  and the school sign-in may not (the school sign-in falls back to the feed link).
- **Q13:** the vault is created at the account step; onboarding is a full-screen mode of the app.

**Lane names follow spec §9:** **O3 is the long onboarding** (§1) and **O4 is one school sign-in**
(§2). The brief that commissioned this plan named them the other way round; the spec's table wins.
To avoid any doubt, every mention below carries its descriptive name.

**The look is not in this plan.** Five UI directions are being drawn under
`docs/design/2026-10-23-directions/`, and Quinn chooses one. Every screen here is a **design slot**:
its look, layout and copy are the chosen direction's. What this plan fixes is the **flow contract**
each slot must honour (§4): the steps and their order, what runs in the background, the proof data
each step shows, the skip costs, the resume state, the reveal's 3-minute cap, payment after the school
step's proof, the vault at the account step, and the full-screen onboarding mode. Quinn reads every
"why" line before it ships (spec §1.10).

---

## Goal

A new student installs Knowlu, signs in once to their account and **once** to their school, pays
after seeing their own counts, connects the rest with one consent or login each, confirms their
week, and first sees the main page when it already knows what's next. Concretely:
1. Steps 1-11 of spec §1.3 run as one flow, in order, each with why, ask and proof.
2. At a school whose curated row says `shared_sign_in`, one visible sign-in serves the LMS feed link,
   the course list, the registrar, the grades capture and the school mailbox; later reads are hidden
   and silent until the school's session expires.
3. Background work starts as each ask is answered and never blocks the next step; a rail shows each
   source's state to the end.
4. Quitting at any step and relaunching resumes at that step and re-runs unfinished work.
5. The reveal opens when everything required has run once, or 3 minutes after "Your week", naming
   what is still coming and what was skipped.
6. After the reveal, on a fresh scratch profile at the founder's school, the main page shows no task
   past its window, already submitted, from a past term, or estimated above a stated ceiling for its
   kind (spec §8's measure; the late window itself is O6's, wave C).

## Architecture

- **O4, one school sign-in.** A new `app/src/school_session.rs` owns `<profile data>\school-session`
  (path guard, patient delete, the record of which features use it). The curated campus row gains
  `shared_sign_in`. The grades, registrar, LMS-capture and mail windows ask `school_session` which
  profile to open; at a shared school it is `school-session`, otherwise each keeps today's. Every
  window on the shared folder carries one browser-arguments constant (WebView2's one-environment
  rule, S-B), which **never carries a debug port** (§14, I5). Sign-out and account deletion delete it
  (`app/src/account.rs`, **contract list**).
  Settings shows one row; the privacy page names it.
- **O3, the long onboarding.** The vault-less shell keeps only the picker, welcome and the account
  step; step 2 creates the vault with defaults and relaunches into the console in **onboarding
  mode** (no tray, no scheduler, no autostart change) until the reveal. **Every config file is
  written once, create-new, at the first step that knows its whole content** (§4.9; the
  `config/actor.yaml` precedent), so nothing is ever rewritten wholesale. A new
  `app/src/onboarding_state.rs` holds `onboarding.json` (step, skips, job states; no student content)
  and `onboarding-answers.json` (the answers the config jobs need, kept across a quit and deleted at
  the reveal, §4.5) beside `settings.json`; a feed link not yet on the account waits in Credential
  Manager (§4.11). A new `app/src/onboarding_jobs.rs` runs the background work: a serial queue for
  windows and engine steps, in slot order, and a local lane for the seconds-long config and account
  jobs (§4.3). The reveal gate is a pure function in its own `app/src/onboarding_gate.rs` (F9). A new read-only engine command computes the
  step-3 proof from raw captures and the reveal's tidy counts from the vault. The page's onboarding
  screens live in their own static files, filled by the chosen direction.
- **Nothing in `engine/src/entitle.rs` changes** (Q2 ruled B). The only contract-list files touched
  are `app/src/account.rs` (O4's sign-out, deletion and `PRIVACY_VERSION`; O3's one call to
  `onboarding_state::on_sign_out` and two credential names for deletion, F2c), by `contract-engineer`.

**Code read for this plan:** this worktree, `.claude/worktrees/o1` (branch `feat/o1-turned-in-work`,
`origin/main` plus O1's commits). The planner had no shell; every `file:line` is from that tree, and
T0 re-reads each on the lane branches. The spec's own citations came from an older worktree and are
not reused where this read found them moved.

### What the code read found (facts the spec did not have)

- **L6 and L5a have merged.** `app/src/inference.rs` and `engine/src/runtime.rs` are gone, and
  `registrar.rs` is on the branch, so `app/src/main.rs` is no longer held by L6 (T0 confirms).
- **Sign-out does not delete the kept Blackboard profile today.** `account::sign_out_with`
  (`account.rs:753-784`) deletes only mail profiles (`mail_session::forget_all`); `lms-session` goes
  only with account deletion's `remove_dir_all(data_dir)` (`account.rs:1672-1674`). Spec §2.5's
  "Today" column says sign-out deletes every kept profile; it does not (§5, Q-B9).
- **The console's first slot would fire at step 2.** `scheduler::spawn` runs a slot at launch when
  `state/today.md` is absent and the vault is app-scheduled on its own device (`scheduler.rs:1155-1163`
  here; `spawn` at `:1134` on `1d3c467`, T0), which a step-2 vault is. Onboarding mode must
  not call `scheduler::spawn` (P-3, §3). **`spawn` does three other things onboarding mode then
  skips** (T0, 2026-10-24): it primes `Scheduler.mode_device` (`:1154`; otherwise the `Default`
  placeholder `(Script, true)`, `:146`, which `commands::attach_scheduler` reports on every poll,
  `commands.rs:59-71`), so F4 primes it itself; it takes and clears `quit_at` and computes
  `startup_missed` (`:1137-1149`); and it starts the housekeeping thread (`:1229`), which re-primes
  `mode_device`, runs the debounced backup, prunes logs and checks for updates. The last two are
  benign (§13, 2026-10-24).
- **The vault's config is written once, at creation, from the whole wizard plan.** `create_vault_in`
  takes a `WizardPlan` (`onboarding.rs:621`) and `scaffold::build_into` writes `config/campus.yaml`,
  `events.yaml`, `ingest.yaml` (feed, `course_map`, coursework mappings, calendars) and `runners.yaml`
  in one staged rename (`scaffold.rs:733-770`). A vault made at step 2 has none of the school's
  answers yet, so each later file is created at its own step (§4.9). `build_into` also refuses an
  unknown campus preset (`campus_yaml`, `scaffold.rs:140-141`), so step 2 must not write
  `campus.yaml` or `events.yaml` at all (§4.9, F3). Config files never sync (`sync` carries only
  `ids::NOTE_FOLDERS`), so a restore or a second device brings notes, never config.
- **Storing the feed link on the account needs an entitlement.** `PUT /account/sources` checks it
  first (`cloud/supabase/functions/account/handler_test.ts:284`). Under Q2's order, step 3 captures the
  link before payment, so it must wait somewhere until step 4 (§5, Q-B2).
- **A feed the account cannot take is never lost today** (review r2, I1). `create_vault_in` retries a
  validated, unstored feed once and, failing that, writes it into the vault
  (`restore_capability_url`, `onboarding.rs:744-773`, R-C3'-exec-40); a rejected paste is never sent
  to the account and stays in the vault (R-C3'-exec-41). §4.11 keeps both rules.
- **A down network reads as an empty restore** (review r2, I5). `sync::restore_into` reports
  network-down as an empty result with a warning and `ok: false`, not an error (`onboarding.rs:788-802`;
  `sync::Restored`), so "nothing to restore" and "could not ask" differ only in `ok` (§4.9).
- **Step 10's proposals read `ingest.yaml` synchronously** (review r2, I4).
  `week::commitment_proposals_inner` runs `knowlu-engine commitments --json` (`week.rs:54`), which
  fetches the calendars `config/ingest.yaml` names (`cli.rs:977`), so `ingest.yaml` must exist before
  step 10 asks (§4.3's local lane).
- **A slot cannot be stopped between steps today** (review r2, I3). `run_slot_inner`'s loop runs every
  engine child in turn (`scheduler.rs:1060-1068`), each bounded only by `CHILD_TIMEOUT`, 20 minutes
  (`scheduler.rs:28`). A child's output is piped to the app, so it cannot outlive the process. F8f adds
  a stop checked before each step.
- **Leaving the process is the caller's job** (review r4, I-1). `onboarding::relaunch` only spawns the
  new process, which waits on `--after-pid` (`onboarding.rs:391-412`); `account::sign_out` deletes and
  returns JSON, ending nothing (`account.rs:710-729`, `:783`); the console page never calls it today.
  So every onboarding exit is its own command that drains, acts, relaunches where it should, and exits
  (§4.10, F2b).
- **A skipped `sync` still exits 0 under the name `sync`** (review r4, I-2). `run_slot_with` names a
  step by `args[0]` with the child's exit code (`scheduler.rs:1060-1067`); the skip is recorded only in
  `state/sync-status.json` (`totals.skipped`, `engine/src/sync.rs:2524-2554`), which
  `state::refresh_sync` reads into `cs.sync` at the slot's end (`scheduler.rs:1116`). §4.3 reads it
  there.
- **`slot_argv` names nothing it leaves out** (review r4, I-3). It omits `grades` without a bundle and
  `judge` for `JudgePlan::Skip` silently (`scheduler.rs:530-532`, `:550`); the named skips, the
  runner-log lines, the privacy hold and the session refresh are all `run_slot_with`'s
  (`:806-812`, `:840`, `:955-956`, `:981-995`, `:1033-1057`). So `slot.partial` runs through
  `run_slot_with` too (§4.3).
- **The mail window carries its own browser arguments** (`MAIL_BROWSER_ARGS`, `mail.rs:534-535`:
  wry's defaults plus one more disabled feature), plus a debug port in a debug build **only when
  `KNOWLU_MAIL_DEBUG_PORT` is set** (`debug_port.rs:9`, `mail.rs:566-572`). The grades window carries
  wry's defaults (`debug_port::WRY_DEFAULT_ARGS`, `debug_port.rs:13`; `grades.rs:580-587`). The LMS and
  registrar window is incognito on a temp folder (`lms_link.rs:173-184`). All three differ, which is
  what S-B tests.
- **The mail seams Q9 needs exist.** `mail_disconnect` (`mail.rs:782`) and `mail_keep_session`
  (`mail.rs:809`) delegate to `mail::commands::disconnect_with` and `keep_session_with`; the folder
  deletes are `mail_session::forget(data_dir, address_id)` and `forget_all` (`mail_session.rs:373`,
  `:388`), whose guard accepts only `mail-setup` and `mail-session-<id>`, so it can never delete the
  shared folder (S4c).
- **`state::Settings` cannot gain a field safely** (no serde defaults; a parse failure resets
  settings, `onboarding.rs:279-285`). So `onboarding.json` and `school.json` are files beside
  `settings.json`, the `grades.json` and `mail.json` precedent.
- **The interaction vocabulary is closed.** `uievents::ACTIONS` holds 11 actions and `record` refuses
  any other (`engine/src/uievents.rs:11`, `:43-44`), so per-step drop-off telemetry needs a new action
  (§5, Q-B10).
- **The step-3 join already exists.** O1 added `ingest::gradable_id` and `ingest::gradebook_done`
  (`engine/src/ingest.rs:572`, `:580`), pure, which S-F's turned-in count can reuse.
- **The shell's account commands read the pending session, which step 2 empties** (review r5, I-1
  and I-2). `entitlement_now`, `open_checkout`, `google_connect_url` and `google_connected` read only
  `PENDING_TARGET` (`account.rs:1195-1200`, `:1611-1616`, `:2111-2151`); `move_session` copies the
  session to the profile and deletes the source (`:143-157`). The vault-target versions exist for
  three of the four: `refresh_entitlement` (`:994`), `google_status` and `google_connect`
  (`:2163-2210`). Checkout has none, and no command re-attaches a session to a vault that has
  `cloud.yaml` (`attach_in` refuses one). F2g and F2h add the two missing paths.
- **A cloud vault's slot always runs `coursework` and `ingest`** (§13.1). `slot_argv` pushes
  `coursework` unconditionally and `ingest` for every cloud vault, the feed living in the account
  (`scheduler.rs:529`, `:538`). So a slot before `ingest.yaml` exists must leave both out itself
  (`SlotScope::BeforeConfig`, §4.3).
- **`grades` writes nothing without course notes** (T0, 2026-10-24). The engine's `grades` matches
  the bundle's courses to the vault's course notes (`grades.rs:405-415`) and skips every course it
  cannot match, so on a vault with none it writes no grade note (`:538-541`). So `BeforeConfig` leaves
  `grades` out too, and the first grades run only after `school.courses` (§4.3).
- **An entitlement-refused sync is a skip** (T0). `sync::record_gated_skip` writes `ok: false`
  **with** `skipped: "no entitlement"` (`sync.rs:2462-2475`), so §4.3 reads it as "neither".
- **The account never returns a stored feed's host** (T0). `GET /account` selects only `kind` and
  `added_at` from the sources (`cloud/supabase/functions/account/index.ts:192`), so §4.7 suggests no
  school from the account.
- **A closed mail-backfill batch starts a whole slot** (review r5, I-4): `real_env`'s `on_closed`
  spawns `scheduler::run_slot` (`mail/live.rs:118-123`). F8m routes it to the queue in onboarding
  mode.
- **Window events run on the event loop** (review r5, I-5), so nothing that waits may run in
  `on_window_event` (`main.rs:187-193`); F4 hands the close to a thread.

---

## 1. VISION and the two overriding rules

- **"Onboarding asks for sign-ins, nothing else."** Every ask in §4 is a sign-in, one consent,
  payment, or a confirm of what Knowlu found (the coursework mapping of R-OB-1, the rows of "Your
  week"). No profile name, timezone or slot time is asked: step 2 fills them with defaults, editable
  later in Settings (spec §1.9 A).
- **Success 5, "it already knows what's next" in the first session.** The main page is first shown
  after the reveal, and the reveal waits for the first full slot (§4.3).
- **"Adding a source is one action."** One school sign-in feeds up to five reads (O4).
- **Commitment 5.** The registrar inside the school step is an authoritative source and writes as the
  student (R24), as today; the school step's "why" line says so. Turned-in work is O1's rule, unchanged.
- **Commitment 1, `rank` never calls a model.** The new engine command (§4.4) is arithmetic over files;
  no model, no network.
- **Portal credentials never leave the device.** `school-session` is a WebView2 profile under the
  profile's app data, DPAPI-protected cookies, no password, never synced, never in the vault.
  `onboarding.json` and `onboarding-answers.json` hold no secret: no feed link, no cookie, no login.
  A feed link awaiting the account waits in Credential Manager, on this device only (§4.11, Q-B2).
- **Data minimization.** Before payment, the vault receives nothing from the school step (§4.2,
  step 3), not even `campus.yaml` (§4.9): raw captures stay in the profile's app data, as the grades
  capture already does.
- **"Never rewrite a vault file wholesale."** Each config file is created once, create-new, and an
  existing one is kept, never replaced (§4.9). No exception is planned, so `CLAUDE.md` needs no change
  for it (§5, Q-B1).
- **Rule 1, no single-user assumptions.** Whether a school shares one sign-in is the curated row's
  `shared_sign_in`, data and never code. Nothing names a school, a person or an account; spikes record
  structure and counts only. Defaults (vault name "Knowlu" with a suffix, Windows' timezone, 12:00
  and 18:00) are the same for every student.
- **Rule 2, frozen references.** No fixture changes. `surface`'s read model gains no key: the reveal's
  counts come from the new read-only command, so `surface-today-*.json` stay byte-identical (F7a's test
  runs `surface_oracle` unedited). The eight Python references are untouched: no command they cover
  changes its output for their inputs.
- **No free tier (cloud design D2).** Before payment, only device-side reads run; every gated engine
  step and cloud judgment waits for an entitlement, so a student who leaves at step 4 has cost
  nothing in the cloud (Q2).

## 2. Global constraints

- `cargo build --workspace` and `cargo test --workspace` from the root, dev profile, **0 warnings**
  (the one accepted `.rsrc` line). TDD: each `test-writer` task's tests compile and fail at run time
  (minimal `pub` stubs with `todo!("<task id>")`, O1's red-state rule), then the implementer task
  makes them pass.
- **Contract list:** the only file touched is `app/src/account.rs`, and only by `contract-engineer`
  with `contract-reviewer` reviewing that diff (S5, S7, F2c). `app/src/credentials.rs`, `wincred.rs`,
  `entitle.rs`, `sync.rs`, `write.rs` and every fixture are used, never edited. A task that finds it
  needs one stops and reports.
- **No synthetic input, no live vault.** Tests use scratch vaults under the temp directory and fake
  window seams; nothing opens a real WebView2 window in `cargo test`. Tests never leave the machine.
- **Credential Manager tests** hold the file's `CREDMAN_LOCK`, use generated ids and clean up with a
  `Drop` guard (`app/tests/account.rs`).
- **No new Tauri capability.** Every sign-in window has no capability grant, as today
  (`app/capabilities/default.json` names only `main`); a test keeps pinning it.
- **`app/src/commands.rs` computes nothing.** New counts are the engine's (F7a, F7c); orchestration (the job
  queue, the reveal gate) lives in its own module, as the scheduler's does.
- **Shared files.** `app/static/console.js` is one queue (HANDOFF §2): the onboarding screens go in new
  static files (F11a-F11e) and `console.js` takes one slot to remove the old wizard and add the hook
  (F10, static files only). `app/src/main.rs` is touched by F4 (the mode), F5 and F5b (the lists) and
  S6r (two registrations), and by nothing else; every list edit is `mechanical`'s and recounted.
- **Task size.** No implementer or console-ui task edits more than about 80 lines of one file's code;
  larger work is split (§14, I7).
- **Line endings:** LF in every file this plan edits; `*.ps1` stay CRLF.
- **Every live step is the controller's** (§3, F15, S10): a dev build against staging, a scratch profile,
  Quinn at the machine only to sign in or answer the school's MFA (Okta at the founder's school),
  structure and counts only, and the scratch
  profile removed afterwards (profile, vault, credentials, autostart).

---

## 3. Spikes first (the controller, live; the planner's code reads recorded)

Nothing that depends on a spike is built until it passes (spec §12). A spike that fails goes back to
Quinn with its finding, never a quiet redesign (spec §10). Every live spike follows these rules:
- **Where:** a scratch profile made for the spike (`scripts\scratch-vault.ps1` where a vault is
  needed), at the founder's school, a dev build against **staging**, never a live vault or prod.
- **Who does what:** the controller launches processes, reads files and records; Quinn types only
  the school sign-in, the answer to the school's MFA (Okta at the founder's school) and, where a step needs it, the Stripe test checkout in the
  browser. No synthetic keyboard or mouse input; screenshots by window handle only.
- **No debug port near a school page** (§14, I5). The proof harness's remote-debugging port may be
  attached to the app's main window only. No window on a school folder ever carries one, and if the
  harness's mechanism would reach every WebView2 environment in the process (an environment variable
  does), the harness is not used in a spike that opens a school window. No school page is ever read
  through CDP.
- **What is written down:** counts, seconds, HTTP status codes, host kinds ("the LMS host", "the
  identity provider", "the mailbox host"), and which surface asked for MFA and how many times.
  **Never** a URL path, a cookie name or value, a title, a course name, an address, an id or a score.
- **Afterwards:** the scratch profile, its vault, its credentials and any autostart entry are
  removed; any spike instrument (below) is reverted and kept only in the session's scratchpad.
- **Instruments.** S-A phase 2, S-B, S-F's live half and S-G need code the app does not have: windows
  opened on one shared folder, the validation-fetched feed saved for counting (§14, I6), and each
  hidden window's host kind logged from Rust. That code is a **throwaway patch**, built by the main
  session in a scratch worktree, kept in the scratchpad as the email lane's proof hunks were, **never
  committed** and never pushed. It logs only what the line above allows, and anything it saves is
  deleted in the same session.

### S-A: one school sign-in across the LMS, Outlook and the registrar

**Question (spec §12).** At the founder's school, does one visible sign-in on one profile let the LMS
(`users/me` 200), Outlook (a signed-in mailbox page), the registrar (its rows call answering with
rows) and, if reachable, Simple Syllabus come up **silently** later? How many MFA prompts in all?

**Phase 1, no code (Edge, a scratch user-data folder).** The controller starts Microsoft Edge with a
new, empty `--user-data-dir` under the scratchpad and the LMS host's start page (a process launch with
a URL, not input). Quinn signs in and answers the school's MFA (Okta at the founder's school). The controller then starts the same Edge profile at
the school mailbox's start page, then at the registrar's start page, then at Simple Syllabus if the
school has one. For each, record: signed in without a prompt (yes or no), prompts shown (password,
MFA), seconds to a signed-in page. Then close Edge, wait 10 minutes, and repeat the three opens
(session survival across a restart). Delete the folder afterwards.
- *Why Edge first:* it answers the structural question (does one identity-provider session cover the
  three hosts?) in ten minutes with no code. Its caveat: Edge may use Windows' own account sign-in,
  which WebView2 does not by default, so a pass here is necessary, not sufficient.

**Phase 2, the spike build (WebView2).** With the throwaway patch: one visible window on a scratch
`school-session` folder at the LMS host; Quinn signs in. Then, in turn and **hidden**: the grades read
(`users/me` and one gradebook call), the registrar's rows call, and the mail window at the school
mailbox (its title reaching the mailbox, no setup run). Record per read: silent (yes or no), seconds,
HTTP status of the read, and whether the hidden window ever needed to be shown. The patch also lists,
from Rust, the **top-level hosts the mail window navigates through** on its way to the mailbox (the
identity provider, any MFA host): the notes record their kinds and number only, and the host names go
straight into the founder's school's curated row as `sign_in_hosts` (S2), public institutional names
in code, never in a note (Q-B14, review r2 M1).

**Pass.** Phase 2 shows the LMS, the registrar and the school mailbox each signed in silently after
one visible sign-in, with one MFA prompt in all. Then the founder's school's row gets
`shared_sign_in: true` (S2's data line).

**Results that change the plan.**
- **Two of three share** (say Outlook asks again under conditional access): `shared_sign_in` becomes a
  per-surface list rather than a bool (S2), the school mailbox keeps `mail-session-<id>`, and step 5
  asks one sign-in. Goes to Quinn with the finding.
- **None share silently** (MFA per app): O4 stops; the finding goes to Quinn (spec §2.3's option B is
  the fallback to discuss). O3 can still be built, with each read keeping today's window and
  prompting inside onboarding.
- **Phase 1 passes and phase 2 fails:** WebView2-specific; the finding (which read, which status) goes
  to the planner with S-B's results before any O4 task is dispatched.

### S-B: can those windows share one WebView2 data folder?

**Question.** WebView2 lets windows share a user-data folder only when they use the same environment
options. Today the mail window sets `MAIL_BROWSER_ARGS`, the grades window runs on wry's defaults
(`WRY_DEFAULT_ARGS`, one disabled feature fewer), and the LMS and registrar window is incognito on a
temp folder. Can they share one folder, and must the LMS capture stop being incognito? The spike
build sets `KNOWLU_MAIL_DEBUG_PORT` nowhere: no window here carries a debug port.

**What the controller runs (the same spike build and session as S-A phase 2):**
1. Open the grades window on the folder with wry's default arguments, close it, then open the mail
   window on the same folder with `MAIL_BROWSER_ARGS`. Then the reverse. Record: built (yes or no),
   the error text's HRESULT name if not.
2. All with **one** argument string (`MAIL_BROWSER_ARGS`, no debug port): open, read, close, in turn.
   Record whether each read still works (grades `users/me`, the registrar's rows call, the mail
   window's title reaching the mailbox, read from Rust).
3. Two windows open on the folder **at once** (hidden grades read while the hidden mail window sits
   open). Record whether the second opens and both read.
4. Close a window and open the next on the folder within one second (WebView2 releases a folder a
   moment after the window goes). Record failures and the wait that avoids them.
5. The LMS capture's cookie read (`cookies_for_url` on the feed host) on a **non-incognito** window on
   the folder: the feed link and the course list come back as they do today.
6. Restart the app; open a hidden window on the folder: still signed in (persistence on disk).

**Pass.** Step 2 works for every read; step 5 works; step 6 is signed in. Then S4a-S4c build one
`SCHOOL_BROWSER_ARGS` constant equal to `MAIL_BROWSER_ARGS`, **never with a debug port in any build**,
used by every window on the school folder, and the LMS capture runs non-incognito on it. A debug
build's mail debug port stays available only to a mail window on its own `mail-session-<id>` folder.

**Results that change the plan.**
- **Step 1 fails, step 2 passes** (the expected case): as the pass says; no change.
- **Step 2 fails for a non-mail read** (`MAIL_BROWSER_ARGS` breaks grades or the registrar): spec
  §2.7's fallback. The school mailbox stays on `mail-session-<id>` (one more sign-in at step 5); the
  shared folder serves the LMS capture, course list, registrar and grades only. Goes to Quinn.
- **Step 3 fails:** no change in design (one window at a time on the folder is already the rule);
  `school_session`'s lock (S2) becomes load-bearing and its test is required, not optional.
- **Step 4 needs a wait:** that lock holds the folder for the measured wait after each close.
- **Step 5 fails:** the LMS capture keeps its incognito window; the student signs in twice at step 3
  (LMS capture, then the shared profile). Goes to Quinn: Q1's "one sign-in" is not met.

### S-E: real first-run timings

**Question.** How long does each step and each background job take, so the copy ("about 10 minutes")
and Q7's 3-minute cap can be checked against reality (spec §1.3, §1.5)?

**What the controller runs.** Today's build (no patch), a fresh scratch profile at the founder's
school, against staging, with Quinn at the machine for sign-ins. Time, in seconds, from the moment
each ask is shown:
- each sign-in: the LMS window to a captured link; the course list; the registrar to rows; the grades
  sign-in to a bundle; the school mail setup to forwarding on; the canary's arrival; the Google consent
  round trip (Gmail and Calendar);
- each zyBooks and VHL discovery;
- the first full slot, step by step, from the run record: `sync`, `coursework`, `grades`, `ingest`,
  `judge` (with the number of tasks judged), `rank`; and the mail step's first pass;
- the payment round trip with Stripe's test card (Quinn types it) or a staging tester entitlement.
Record counts beside times (tasks judged, events read), never content. Also record the time from the
moment `ingest.yaml` would be created (the end of step 8, §4.9) to the end of `ingest` and `judge`
over the feed's tasks, since under write-once those steps cannot start earlier. Also record the
**longest single stage** of the first full slot (review r2 I3, review r3 M1: the session and
entitlement refreshes, the grades capture, the mail pass, each engine step, backup, telemetry): it
bounds how long Open Knowlu, Finish later, close or Sign out can wait at a stage boundary (§4.10).

**Results that change the plan.**
- **The longest single stage exceeds 60 seconds** (most likely `judge` on a first vault): the number
  goes to the planner and Quinn before F8f. The options are a shorter per-step bound for onboarding's
  slot steps (the step is then killed as `CHILD_TIMEOUT` already kills, and re-run by the first-run
  slot), or the wait as designed with its seconds shown.
- **The first full slot plus the mail canary usually exceeds 3 minutes:** the queue already runs
  `sync` right after payment, `grades` (with `coursework`, `ingest` and `judge`) in the `NoRank` slot
  once `config.ingest` is `done` (T0, 2026-10-24), and the canary in step 5's background. If the wait after step
  10 still exceeds 3 minutes, the number goes to Quinn, with the one exception that would shorten it
  (§5, Q-B1: `ingest.yaml` created at step 4 and extended by line at steps 6-8). The ruled cap stands
  until Quinn says otherwise.
- **The student's own time exceeds 15 minutes:** the step that costs most goes to Quinn with the
  number; the "about 10 minutes" line is rewritten from the measurement (the direction's copy).
- **A sign-in takes longer than the window's limit** (grades' `SIGN_IN_LIMIT`): the limit is raised
  for onboarding's visible sign-in in S4d, with the measured number.

### S-F: the step-3 counts before payment

**Question.** Can step 3's proof (classes, class times, upcoming assignments, already turned in,
grades) be computed on the device from raw captures, before payment and before any note is written?

**The planner's code read (this plan, re-confirmed at T0).** Yes, from four captures, none of which
needs an entitlement:
- **Classes:** the course list `lms_link::capture_courses` reads (`lms_link.rs:706`), after O0's
  current-term filter (`fix/current-term-courses`, not yet on this branch).
- **Upcoming assignments:** the feed the LMS capture already fetches on the device to validate the
  link (`lms_link::summarise`, `lms_link.rs:326`; R-C1-42 counted 156 events that way). The fetched
  text is handed to the engine as a temp file, never kept.
- **Already turned in:** the grades bundle (`grades::bundle_path`, `grades.rs:686`) joined to the
  feed's UIDs by O1's `ingest::gradable_id` and `ingest::gradebook_done`, by id only.
- **Grades for N courses:** the same bundle (courses with at least one graded item column).
- **Class times:** the registrar capture's matched rows, counted by the engine without writing
  (`commitments` today writes only with `--confirm`; the count needs a read-only path, F7a).
- **Syllabi found:** only with L5 (wave C, O5); absent in wave B.
The engine computes all of them in one read-only command (F7a's `--preview`); the app only passes
file paths.

**What the controller runs (in the S-E session).** Today's build never puts the feed on disk
(`summarise` counts it in memory), so the **throwaway patch gains one step** (§14, I6): after the
validation fetch, it writes the feed text to a file in the scratch profile's `tmp\` folder and logs
the path's existence only. Before payment, the controller runs F7a's command on that file, the course
list and the grades bundle if F7a is built, else counts by a short script that prints counts only
(items due after now; items whose UID's gradable id joins a done grade row; courses with a graded
column). The file is deleted in the same session, and no line of it is read by a person. After the
first full slot, count the vault: live Blackboard tasks plus
`imported-done` and `imported-past` archive notes, grade notes by course, confirmed class
commitments. **Pass:** every proof number equals what the vault then holds, or differs only by a
stated structural reason (e.g. an item the feed carries twice).

**Results that change the plan.** If the feed cannot be fetched on the device at this school (the
cloud fetches it for an account vault), the upcoming and turned-in counts move after payment and
step 3's proof shows classes, class times and grades only. Q2 stands; the thinner proof goes to Quinn.

### S-G: do hidden windows keep working while the student is elsewhere?

**Question.** Step 3's reads run in hidden windows while the student is in the browser paying (step
4) and while the onboarding window changes steps. Chromium throttles hidden and background pages; an
identity provider's redirect chain often relies on a script that submits a form. Do the reads
finish? (The M365 backfill spec's decision 19 met the same question for the mail window.)

**What the controller runs (the S-A phase 2 build, in the S-E session).** After the visible school
sign-in, start the hidden reads (grades, registrar, then the mail window at the mailbox), then:
(a) Quinn brings the browser to the front (their own click) and stays there 3 minutes; (b) Knowlu's
window minimised; (c) the onboarding page navigating between two steps while a read runs. Record per
read and case: finished (yes or no), seconds, and, **from Rust only** (§14, I5), the host kind the
hidden window's URL sat on each second (`WebviewWindow::url`, reduced by the patch to "identity
provider", "LMS host", "registrar host", "mailbox host" or "other"). A read that stays on the identity
provider while hidden is the stall this spike looks for. No school page is read through CDP, and no
`document.visibilityState` is read from a school page.

**Results that change the plan.**
- **All finish:** no change.
- **A read stalls:** `SCHOOL_BROWSER_ARGS` (S-B) gains `--disable-renderer-backgrounding
  --disable-background-timer-throttling --disable-backgrounding-occluded-windows`, in release builds
  too, and the case is re-run. That changes every window on the school folder at once, so it is
  recorded for L1 (whose decision 19 assumed the mail window had a folder of its own).
- **Still stalls:** the hidden window's controller is kept "visible" while the native window stays
  hidden (decision 19's step W3), re-run.
- **Still stalls:** payment opens only after step 3's reads finish. That lengthens the student's wait
  and goes to Quinn before F8b is built.

### P-3: can the console start in a full-screen onboarding mode? (planner's code read)

**Answer: yes, with five changes to `run_console`, none on the contract list.** From `app/src/main.rs`:
1. **The scheduler.** `scheduler::spawn` (`main.rs:178`) starts the tick and, for a vault with no
   `state/today.md`, runs a slot at once (`scheduler.rs:1155-1163`). A step-2 vault has none, so
   onboarding mode must not call `spawn`. The first full slot is run by the job queue through
   `scheduler::run_slot_inner` (`scheduler.rs:731`), which the `--run-slot-once` path already calls
   without the tick (`main.rs:36-46`).
2. **The tray.** `tray::build` (`main.rs:168`): not called in onboarding mode, and so the window's
   close (`main.rs:190-191`, hide to tray) must end the process instead, with onboarding's state saved.
3. **Autostart.** `main.rs:169-171` enables or disables per settings at every launch; in onboarding
   mode neither call is made, and step 2's default writes `autostart` as the student's later choice
   (on), applied at the reveal's relaunch.
4. **The mail launch sweep** (`mail::live::on_start`, `main.rs:175`) still runs: it only cleans and
   resumes, and a setup interrupted by a quit at step 5 needs it.
5. **The relaunch.** Step 2 ends with `onboarding::relaunch_with(vault)` (`onboarding.rs:415`), the
   call Finish makes today; the reveal's "Open Knowlu" ends with the same call, so the normal console
   (tray, scheduler, autostart) starts fresh. No in-process switch from one mode to the other.
   `relaunch_with` ends this process, so nothing may still be running when it is called: §4.10 says
   how a running job is let finish and how unfinished ones are handed to the normal console.
The mode is decided before the builder from `onboarding.json` in the profile's data folder
(`profiles::profile_dir`, `main.rs:142`): present and unfinished means onboarding mode. **The
controller re-confirms these five on the lane branch at T0;** a sixth difference found there goes to
the planner before F4.

---

## 4. The flow contract

This section is what every design direction must honour. The look and words are the direction's; the
data, order, states and rules are these.

### 4.1 Rules for every step

- **Three parts.** *Why* (one line, shown before the ask), *the ask* (one sign-in, consent, payment or
  confirm), *proof* (counts the student can check). A step may show "reading…" in place of its proof
  and let the student go on; the proof then fills in on the rail.
- **States a step renders:** `intro` (why, before the ask), `asking` (a sign-in window or browser is
  open; the step says where to look), `reading` (its jobs run), `proof` (its numbers), `failed` (a
  named reason and its fix, never a bare error), `skipped` (its cost, §4.2), `already-done` (from a
  restored account or this device, showing the proof it has).
- **The rail.** Visible on every step from step 3 to the reveal: one row per source with its state
  (`reading`, its proof in one line, `failed` with its fix, `skipped`, `not yet`). Its data is
  `onboarding_status`'s `jobs` (§4.5).
- **Forward only, with two exits.** Back is allowed to any finished step for viewing; a finished ask
  is not re-run except by its own "Try again" on a failure. Two exits on every screen: **Finish
  later** (`onboarding_quit`: drains, saves and quits; the next launch resumes) and **Sign out**
  (`onboarding_sign_out`: drains, then the account's sign-out, which deletes kept sessions (O4) and
  rewinds setup to step 2, then reopens the picker; §4.10). No screen calls `account::sign_out`,
  `switch_profile` or `delete_my_data` (F11t).
- **Window.** Onboarding fills the main window: maximised, no console navigation, no tray. Not
  exclusive full-screen and never always-on-top, so that sign-in windows and the browser's checkout
  come in front of it (§5, Q-B7).
- **Sign-in windows stay separate windows,** each with no capability grant, as today. The onboarding
  page never shows a school's or Google's page inside itself.
- **One visible window at a time** asks the student for anything; every hidden read waits its turn on
  the school folder (`school_session`'s lock, S2).

### 4.2 The steps

Order is fixed. "Jobs" are the background work the step's answer starts (§4.3 names them). "Proof"
lists the fields the slot receives; a field the source cannot give is `null` and the slot omits it.
"Resume" is where a quit during the step lands next launch.

**Step 1, Welcome (shell).** Ask: none. Proof: none. Skip: n/a. Jobs: none. Resume: step 1.

**Step 2, Your account (shell, then relaunch).** Ask: Continue with Google (name and email only) or
an emailed code; 18+ and terms (today's sign-in, `account::google_sign_in`, `send_magic_link`,
`verify_email_code`). On success the shell creates the vault with defaults (F3), runs today's restore
from the account, records `account_settled: true` in `onboarding.json` only when that restore
reached the account (`Restored.ok`, whether or not it was empty, §4.9), and relaunches into
onboarding mode. Proof: `signed_in_as` (the account's email). Skip: no. Jobs: none. Resume: a quit
before the vault exists restarts at step 2 (a pending session is reused if still valid); after it,
at step 3. **Step 2 again, inside onboarding mode** (review r4, I-1): when the profile's vault exists
but it holds no session (after Sign out, or a session deleted elsewhere), onboarding mode opens at
step 2's ask without the vault creation: the same sign-in commands, which save to the pending target
as they do today, then **`onboarding_attach_session(account_id)`** (F2g, review r5 I-2), which moves
the session onto this profile's target only when the pending session, the sign-in's answer and
`config/cloud.yaml` name the same account, then the resume step. Another account gets a named
refusal and the way back to the picker, never a merge, and **its pending session is deleted from
this machine** (a local delete, never a revoke, which would sign that account out everywhere;
review r6, I-1), so no other account's token stays on the machine. No path for this exists today: `attach_in` refuses a
vault that has `cloud.yaml`, and `move_session` overwrites its target unconditionally.

**Step 3, Your school (onboarding mode).** Ask: pick the school (`campus_search`), then **one**
sign-in in the school window. Curated and `shared_sign_in`: the window opens on `school-session`
(O4). Curated, not shared: today's windows, one per read. Uncurated, or a failed sign-in: the
**feed-link paste** fallback (`onboarding_paste_link`, F8l: validate only, never the shell's
`paste_calendar_link`, which stores at once under the pending session; review r6, I-2). Jobs: `school.capture` (feed link and its
validation fetch, course list, registrar rows where the row has a registrar, grades bundle where
grades are available), then `school.preview` (F7a's counts). **Nothing is written to the vault
before payment** (§1), `campus.yaml` included: until `school.apply`, the capture code takes the
school's curated row from `onboarding.json`'s `school.unitid`, never from the vault. What the capture
found that later jobs need (the course list, the registrar rows) is saved at once to
`onboarding-answers.json` (§4.5), and a captured or pasted feed link to its pending credential
(§4.11), so a quit after step 3 never needs the school window again. The student may
go Back and pick another school freely until payment. **C1's page rules, carried (F10's review,
2026-10-24; §9 A10):**
- **"My school isn't listed" (R-OB-4).** Beside the school search, a free-text school name. It
  leads to the feed-link paste fallback (`school.path = paste`); the typed name is the `school`
  label with no unitid, and `school.apply` writes `campus.yaml` for it as `campus_config_yaml`
  does today for a school with no unitid (`curated: false`, the `none` events preset; F6a's test).
- **The capture trigger (R-C1-42).** Where step 3 opens today's windows (not shared), the capture
  runs on the student's **second press** ("I'm signed in"), never when the window opens, and never
  while the window is on the identity provider's page (mid-MFA): the press is refused there with a
  line, as today. On the shared folder (S4d) the trigger is the signed-in test (`users/me` 200),
  which an identity-provider page never passes.
- **A campus login never outlives its screen (R-C1-40, the page's half).** Leaving step 3 by Next,
  Back, Finish later or Sign out calls `close_lms_window` for every non-shared window step 3
  opened. Windows on `school-session` are S4d's, closed by it.
- **Every captured course can be removed (R-C1-51, the page's half).** Enrolment comes back
  unfiltered (past terms, organisations, TA roles), so the school proof lists the captured courses,
  after O0's current-term filter where it applies, each with Remove. A removal is an
  `onboarding_answer(3, …)` that marks the row removed in `onboarding-answers.json` (F2d); removed
  rows are never made notes by `school.courses`, never enter `config.ingest`'s `course_map`, and
  **their feed items are never imported**: `ingest.yaml` carries their pairs as `course_exclude`
  (F6b), and ingest leaves out an item only they claim (F6c; 2026-10-24, §14.45). The proof's
  counts are totalled in Rust from `school.preview`'s match-set buckets the same way, so they equal
  what the vault will hold, with no second fetch (F7a, F8b); `grades_courses` counts kept courses
  marked `graded`, and **a removal never removes a class time**, since the registrar holds only the
  current term (§14.47). The list is
  editable until payment; after it, `school.courses` has run and a course is removed in the
  console, as today.

Proof: `classes`, `class_times` (registrar-matched, or `null`),
`assignments_upcoming`, `turned_in`, `grades_courses`, `syllabi_found` (`null` until O5). Skip: no;
the paste is the only way round, and its cost is named (no grades, no registrar class times, no
turned-in filter for LMS work, school email needs its own sign-in). Resume: at step 3; with a kept
school session the captures re-run hidden and silent; otherwise the student signs in again.

**Step 4, Payment.** Shown after step 3's proof, or after its `failed` or paste outcome. Ask: Stripe
checkout in the browser (**`onboarding_checkout`**), polled by **`onboarding_entitlement`** (F2h,
review r5 I-1: both read the vault's own session through `config/cloud.yaml`, because step 2 moved
it off the pending target that `open_checkout` and `entitlement_now` read); a tester or any
account already entitled sees "nothing to pay" and continues. **A `comp` account never sees the
trial's terms** (F10's review): no "no free plan after the trial" line, no checkout button; only
"nothing to pay" and Next. Proof: `entitlement` (`trialing`,
`active` or `comp`) and `trial_end`. Skip: no. Jobs on entitlement, in the local lane (§4.3):
`school.store` (the pending feed link to the account, `PUT /account/sources`, §4.11),
`school.apply` (creates `campus.yaml` and `events.yaml` once, §4.9), `school.courses` (the course
notes through `write`, once the account is settled, §4.9), `registrar.confirm` (R24's write, as the
student, after `school.courses`); in the serial queue, `slot.partial` (`sync` only; `grades`,
`coursework`, `ingest` and `judge` follow `config.ingest`, since `grades` writes nothing before the
course notes exist; §4.3, T0 2026-10-24). Resume: at step 4; the entitlement is re-asked first (today's
re-ask on entering the panel); the course list and feed link come from §4.5's and §4.11's stores,
never from a new capture.

**Step 5, School email.** Ask: none where the school sign-in covers the mailbox (setup runs on
`school-session`, the window shown only if the mailbox asks for something); otherwise one sign-in in
the mail window (today's `mail_setup_start` and its progress). Jobs: `mail.setup`, `mail.canary` (up
to 3 minutes), `mail.first_pass`. Proof: `forwarding` (`on`, `guided`, `off`), `canary_seconds`,
`to_review` (cards from the first pass), `older_mail` (the backfill's state, never waited for).
Skip: yes; cost "date changes and new work announced by email are missed", and beside the skip the
mail spec's risk-12 line, kept from the deleted wizard test: "If your school's rules don't allow
forwarding your school email, skip this." Resume: at step 5;
`mail::live::on_start` resumes an interrupted setup as today.

**Step 6, Google: Gmail and Calendar (needs O2).** *T0 (2026-10-24): O2 has no branch yet, so F11b
first builds this step on today's separate consents (one `google_connect` per scope), and F11f adds
the one consent and the forwarding fallback below once O2 merges (§8).* Ask: **one** Google consent for `gmail.readonly`
and `calendar.readonly` together on the sign-in client, while the account count of Gmail-permission
holders is under 100 (O2's server count); otherwise, or on decline or a failed first read, Gmail
forwarding through the mail window; or Skip. The consent is opened and polled with the
**vault-target** commands `account::google_connect` and `google_status` (`account.rs:2198-2210`,
through `console_session`), never the shell's `google_connect_url` and `google_connected`, which
read the pending target (§13.1). Jobs: `google.gmail_first_read` (`judge`'s Gmail pull,
once entitled; or `mail.canary` for forwarding), `google.calendar_read` (through `ingest`, so after
`config.ingest`); the calendar answer itself (a flag, never a URL) is saved in
`onboarding-answers.json` until `ingest.yaml` is created. Proof: `gmail` (`granted`, `forwarding`, `skipped`),
`gmail_items` (things to do from Gmail), `calendar` (`granted`, `skipped`), `events_this_week`.
Skip: yes; cost "club, job and internship tasks from Gmail are missed; your own events are not
counted as busy time". Resume: at step 6; a grant already on the account is `already-done`.
**The Google testing disclosure** (kept from the deleted wizard test, F10's review): while Google
reviews Knowlu, step 6 shows Settings' `GOOGLE_TESTING` sentence (`console.js:2195`, "While Google
reviews Knowlu, this works only for invited testers, and the connection needs renewing about once a
week.") beside the consent, the same text from one source, never a copy.

**Step 7, Another calendar.** Shown only when step 6 granted no calendar. Ask: paste an iCal link
(validated on the device as today, through `onboarding_paste_link`, F8l) or Skip. The link goes first to its pending credential (§4.11).
Jobs: `calendar.store` (to the account, as today, local lane), `calendar.read` (through `ingest`,
after `config.ingest`). Proof: `events_this_week`. Skip: yes; cost
as step 6's calendar half.

**Step 8, Homework sites.** Ask: per site (zyBooks, VHL; those step 3's courses suggest first, all
offered) a login stored in Credential Manager (`store_credentials`), then discovery
(`discover_coursework`) and **a confirm of the course each book or section belongs to**, pre-filled
with the suggestion (R-OB-1; §5, Q-B8). Each confirmed row is saved to `onboarding-answers.json` as
it is confirmed. Jobs: `coursework.discover`; then, on leaving step 8 (or skipping it),
**`config.ingest`** in the local lane: `config/ingest.yaml` created once from steps 3, 6, 7 and 8's
saved answers (§4.9), after `school.courses` and once both feed stores have settled (§4.11); then
`slot.partial` (`sync`, `coursework`, `grades`, `ingest`, `judge`) in the serial queue. Proof per
site: `books_or_sections`, `assignments`, `done`. Skip: yes, per site; cost "that site's due dates
are missing unless the LMS feed carries them". **C1's page rules, carried (F10's review):**
- **Discovery in flight disables Next** (R-C1-55): while `discover_coursework` runs for any site,
  Next (and Skip for that site) is disabled, with the "reading…" state of §4.1.
- **A blank row is never sent as ignored** (R-OB-1): a book or section row whose course the
  student left blank is sent as unanswered and asked again; only an explicit "not a class" goes
  into `zybooks_ignore`.
- **No invented slug** (R-OB-1): a row's course is sent as the captured course's key from the
  answers file; its slug comes only from `school.courses`'s recorded slug (§4.5), never derived on
  the page from the LMS id or the name.
- **No password field but ours**: the zyBooks and VHL login fields are the only password inputs in
  onboarding, pinned by id (F11t).

**Step 9, Syllabi (wave C, O5).** In wave B this step is **absent**: not shown, not counted as
skipped, and not on the rail. O5 fills it with L5's reads under this same contract (proof `read`,
`of`, `exams`, `weights`, `late_rules`; skip per course; cost "no exam dates beyond the feed, no
weights, no late rule for that course").

**Step 10, Your week.** Ask: confirm class times (registrar rows already written as the student show
as set, each with Edit), and any proposed meeting from the calendar series, through today's confirm
screen commands (`week::your_week`, `commitment_proposals`, `commitments_confirm`); a row may be
left blank. **The proposals are asked for only once `config.ingest` is `done`** (review r2, I4):
until then that part of the step reads "reading your calendars…" and the class-time rows can be
confirmed meanwhile; a failed `config.ingest` shows its fix there. In wave C, O5 adds syllabus rows here. Jobs on confirm: `slot.first_full` (§4.3). Proof:
`confirmed`, `left_blank`. Skip: no as a screen; a blank row comes back as asks over the next days
(today's rule). Resume: at step 10, rows as last saved.

**Step 11, The reveal.** Opens per §4.6. Shows: the week (the Schedule view's data), what's next (the
first item of Today and its reason), free time where the read model gives it, **what Knowlu tidied**
(`imported_past`, `imported_done`, `effort_estimated`, from F7c), **still coming** (jobs not finished,
each named), **not connected** (each skip with its cost and its Settings button). One button, **Open
Knowlu**: stops the work in flight at its next safe point (for a slot, the next engine-step
boundary), writes `finished` with the hand-over list, enables autostart per settings, and relaunches
into the normal console (§4.10). The main page then shows what the reveal
promised, and its "still coming" line until each handed-over job is done.

### 4.3 Background jobs: the queue and its order

- **One serial queue per profile** (`onboarding_jobs.rs`, F8a-F8h) for every job that runs an engine
  child or a window. Jobs run one at a time, in the order their asks were answered, except as the
  rules below require. Engine steps never overlap (the sync run lock already refuses a second); hidden
  windows on the school folder never overlap (`school_session`'s lock).
- **A local lane beside it** (review r2, I4) for the seconds-long jobs that open no window and start
  no engine child: `school.store`, `calendar.store`, `school.apply`, `school.courses`,
  `registrar.confirm` and `config.ingest`. The lane is itself serial and never waits behind the
  queue's `mail.canary` (up to 3 minutes), `mail.first_pass` or `coursework.discover`; a job in it
  waits only for its own inputs (below). Creating a config file needs no hold: no engine step reads
  that file until the job that creates it is `done`.
- **Note writes and `sync` take turns, and wait rather than refuse** (review r3, I-B). The hold
  against a sync is taken by two callers only (corrected at T0, 2026-10-24): `week::run_confirm`
  (`week.rs:103-117`: `vault_io`, then `state/sync.lock`) and `registrar::spawn` (private,
  `registrar.rs:96-104`, the same order through `week::hold_sync_lock`). The console's
  ordinary note writes through `commands::mutate` take `vault_io` only, never `sync.lock`. The hold
  **refuses** when a sync holds the lock, and a slot's `sync` that meets it is skipped at exit 0.
  The lane's note jobs take `run_confirm`'s hold (F8h, unchanged). **Never nested** (the planner,
  at T0): `vault_io` is a `std::sync::Mutex`, which is not re-entrant, and `hold_sync_lock` opens a
  second handle whose `try_lock` refuses even inside one process (`week.rs:122-130`). So a lane job
  takes the two itself (with `hold_sync_lock`, `pub(crate)`) only around an engine child it starts;
  around a path that takes them itself, `registrar.confirm`'s `registrar::spawn`, it takes neither
  and retries that call on its "a sync is running" refusal, by the same 250 ms and 30-second rule. So inside onboarding, one in-process **notes gate** (in `onboarding_jobs.rs`)
  orders them: a lane job that writes a note (`school.courses`, `registrar.confirm`) holds the gate
  for its run, and the queue holds it around every slot job (`slot.partial`, `slot.first_full`). Each
  side **blocks** on the gate, never refuses. With the gate held, the lane job then takes
  `run_confirm`'s hold as today; if `sync.lock` is still held (another process), it retries every
  250 ms for up to 30 seconds before it is `failed` with its fix, and **between tries it holds
  neither `vault_io` nor `sync.lock`** (review r4, M2): each try takes `vault_io` then the lock, and on
  refusal drops both before it sleeps, so the page's own writes and telemetry are never blocked by the
  wait. The lock order is the gate, then `vault_io`, then `sync.lock`, everywhere. Which side goes first at payment
  follows the inputs: with `account_settled` false, `slot.partial` (whose `sync` settles it) runs
  first and the note jobs after; with it true, the note jobs are ready first and the slot waits for
  them, seconds.
- **A skipped `sync` is not a sync** (review r3 I-B; the signal corrected in review r4, I-2). The
  slot's step is always named `sync` with exit 0, skipped or not, so "ran" is read from the **status
  record**, never the step: after the slot, `cs.sync` (filled by `state::refresh_sync` from
  `state/sync-status.json`) must show an `at` at or after the slot's **`RunSummary.started`** (both
  `journal::now_ts`, UTC to the millisecond, so the strings compare; review r5, M2) with no
  `skipped` reason. Three readings (review r5, M2; the second and third corrected at T0,
  2026-10-24):
  - fresh, no `skipped`, `ok`: "first sync" is `done`; `account_settled` is set **only if the
    cursor also shows `pulled_to_end`** (below);
  - fresh, no `skipped`, `ok: false` (an error: transport, the account service, a cursor that could
    not be saved): "first sync" is `failed`, with the status's `last_error` mapped to its fix, and
    `account_settled` is not set (a later slot's fresh `ok` run turns both into the first reading);
  - `skipped` set (no account, no session, **and the engine's entitlement refusal**, which
    `sync::record_gated_skip` writes as `ok: false` with `skipped: "no entitlement"`,
    `sync.rs:2462-2475`), or no fresh stamp at all: neither. The entitlement case cannot arise while
    the hold works (a `sync` is `held` until `Entitled`); if the entitlement lapses mid-onboarding it
    stays `pending` behind the hold. **A sync that met a held `sync.lock` writes no status**
    (`sync.rs:2524-2528`), so only the stale-stamp rule catches it; the item stays `pending` for the
    next slot.
- **First grades, from the step row** (F9 review r1, §14.50). The gate's "first grades" reads the
  slot's bare `grades` row (the engine child's step, with its real exit code): exit 0 marks it
  (F9's `first_grades_marked`), a non-zero exit makes it `failed` (F9's `first_grades_failed`,
  mapped by F8g), and a `(skipped:` or `(not yet:` row leaves it `pending`. **The same blind spot as
  `sync`'s, noted, not fixed:** the engine can also skip at exit 0 under the bare name `grades`
  (for example `grades (skipped: the vault is busy…)` printed by the child itself), which would mark
  first grades with nothing written. In onboarding this is barely reachable, because the queue is
  serial and holds the notes gate around every slot, so nothing else holds the vault while its
  `grades` runs. Unlike `sync`, `grades` has no status record to read instead; if it ever shows up,
  the fix is a status line from the engine, not a guess from the step name.
- **Named jobs, each with a state:** `pending`, `running`, `done` (with its proof numbers), `failed`
  (a closed-set reason and its fix), `skipped` (the student's), `held` (waiting for the entitlement).
  **The closed set's first written entry** (§14.50; owned by F8g, as constants in
  `onboarding_jobs.rs`, so F9's caller, F11's screens and F1 share one spelling):
  `grades_read_failed`, with the fix "Knowlu couldn't read your grades this time. It will try again
  on its next run, or press Try again now. If it keeps failing, reconnect your school in Settings
  after setup." **The second** (§14.51; F8a2): `job_crashed`, with the fix "This step closed
  Knowlu unexpectedly twice, so Knowlu has paused it. Press Try again to run it once more." (the
  queue already names an unwinding panic `job_crashed`, `REASON_JOB_CRASHED`; F8a2 adds the fix
  beside it). Every other `failed` reason this plan names (for example "waiting to reach your
  account", the sync status's `last_error`) joins the same constants as its task builds it.
  States are written to `onboarding.json` as they change, so a quit loses at most the running job.
- **Device-side before payment, everything else after.** `school.capture` and `school.preview` need
  no entitlement and run at once. Every job that writes the vault, stores on the account or runs a
  gated engine step (`coursework`, `ingest`, `grades`, `judge`, `sync`) is `held` until the vault's
  entitlement is `Entitled`; it is never run to fail. **The hold reads the vault's own session**
  (review r5, I-1): its test is `scheduler::entitlement_state(cs)` (the cached entitlement within
  the grace, `scheduler.rs:291`), refreshed through `account::refresh_entitlement(vault, data_dir)`
  (`account.rs:994`, which reads `cloud.yaml`'s target and saves the cache), by step 4's poll
  (`onboarding_entitlement`) and once when the queue starts after a relaunch. Never
  `entitlement_now`, which reads the pending target step 2 emptied.
- **Slot order inside the queue (O1's rule).** Within any run of engine steps: `sync → coursework →
  grades → ingest → judge`, and **`rank` only in `slot.first_full`**. So the vault's first `ingest`
  always finds the grade notes (O1's first-sight `imported-done` needs `state/today.md` absent and
  `grades/` present), and `ingest::is_first_run` holds until the full slot.
- **Config before its readers.** `ingest`, `coursework`, every calendar read and step 10's
  proposals wait for `config.ingest` (§4.9); `grades` waits for `school.apply` **and**
  `school.courses` (T0, 2026-10-24: it reads course notes, `grades.rs:405-415`, and writes nothing
  without them), so it runs first in the `NoRank` slot, which waits for `config.ingest` (itself after
  `school.courses`) and, for the capture's `lms`, for `school.apply`; `registrar.confirm` waits for
  `school.courses`; `school.courses` waits for `account_settled` (§4.9); `config.ingest` waits for
  `school.courses` and for `school.store` and `calendar.store` to settle (§4.11). T0 confirmed that
  `sync` and `judge`'s Gmail pull run correctly with no `ingest.yaml`.
- **A `school.apply` that lands mid-slot** (T0). A slot reads `campus.yaml`'s row once, at its start
  (`scheduler.rs:732`), and the grades capture reads `lms` from that row later (`:945`). A
  `school.apply` that creates `campus.yaml` between the two leaves that slot's grades capture a named
  skip (`grades (skipped: …)`), and the next slot reads the file. The queue's rule above keeps this
  out of `NoRank` and `slot.first_full`; it can arise only from a `school.apply` retried by "Try
  again" while a slot runs, and costs one slot's grades.
- **`account_settled`** is true when step 2's restore reached the account, or when a later `sync`
  reports in `state/sync-status.json` that it ran and reached the account (the rule above; never its
  exit code, which is always 0) **and** the sync cursor shows the account's copy read to its end:
  `pulled_to_end` in `state/sync-cursor.json` (`sync::load_cursor`, `sync.rs:238`; the field at
  `:227`, set sticky at `:2618-2620` when a pull returns `more: false`). A fresh `ok` sync whose pull
  still had pages left has not seen every restored course note, so `school.courses` could duplicate
  one (§14.15). Read only: no `sync.rs` edit. Until then `school.courses` is `pending` with the reason
  "waiting to reach your account".
  **A `sync` pulls one page per run** (`sync.rs:2598-2620`; "more to come — the next slot continues",
  `:2668-2670`), and the only slot onboarding runs before `school.courses` is `BeforeConfig`. So,
  without a rule, an account with more than one page (a restore that failed at step 2 on a down
  network, then a large account) would leave `school.courses`, `config.ingest` and the `NoRank` slot
  waiting for ever. **The rule** (T0, 2026-10-24; Q-B20, decided by the controller after the T0
  review's M1). A page is at most 500 records (`MAX_PAGE`,
  `cloud/supabase/functions/sync-pull/handler.ts:5`), so a fixed 10 runs would read only about
  5,000; the chain is bounded by progress instead:
  - **Continue** while the cursor moves: after a `BeforeConfig` slot whose sync reads fresh, no
    skip, `ok`, with `pulled_to_end` false **and** `record_cursor` or `note_cursor` greater than at
    the previous reading, the queue enqueues the next `slot.partial` (`BeforeConfig` again) at once.
  - **Stop** on a fresh `ok` run where neither cursor moved: `school.courses` is `failed`
    ("couldn't finish reading your account") with "Try again".
  - **Between re-runs:** a *skipped* or stale reading (no session, a held `sync.lock`, no fresh
    stamp) is not progress and not a stop: it counts toward the backstop, leaves the previous
    cursor as the comparison, and enqueues one more `BeforeConfig`; two such readings in a row stop
    the chain as above. A *failed* reading (`ok: false`, no skip) stops it at once: "first sync" is
    `failed` with its fix and `school.courses` `failed` with "Try again".
  - **Backstop:** 50 `BeforeConfig` runs in one chain (every run counts, whatever its reading);
    the 51st is not enqueued and `school.courses` is `failed` with "Try again".
  - **Try again** on `school.courses` while `account_settled` is false resets the count and
    enqueues one `BeforeConfig` `slot.partial`, which starts a new chain; once settled, it re-runs
    `school.courses` itself.
  F8e owns it; F1 test 16 covers it.
- **`slot.partial`** (after payment, and again after `config.ingest`): **the same slot as any other,
  with the steps its scope leaves out named** (review r4 I-3; the layer corrected in review r5 I-3).
  F8s adds `scheduler::run_slot_inner_scoped(cs, sch, Some(app), late, scope)`, which builds the
  `GradesSeam` and `MailSeam` from the `AppHandle` exactly as `run_slot_inner` does today
  (`scheduler.rs:731-762`), then calls the scoped body `run_slot_scoped(…, &seam, &mail, scope)`.
  `run_slot_inner` and `run_slot_with` become one-line wrappers passing `Full`, so none of today's
  35 call sites of the three slot functions changes (7 in `app/src`, 28 in `app/tests/scheduler.rs`;
  review r5, M4). The queue passes
  **onboarding's own `AppHandle`**, so the grades capture and the mail pass exist; with no tray,
  `tray::set_state` does nothing (`tray.rs:150-151`). Three scopes (§13.1, row "slot_argv"):
  - `BeforeConfig`, the run after payment and before `config.ingest` is `done`: leaves out
    `coursework`, `grades`, `ingest`, `judge` and `rank`, so its one engine step is `sync`. It must:
    `slot_argv` always includes `coursework` and, on a cloud vault, `ingest` (`scheduler.rs:529`,
    `:538`), so nothing today would leave them out for a missing `ingest.yaml`. **`grades` is left
    out whole** (T0, 2026-10-24): the grades capture stage and the engine step, as one row
    `grades (not yet: your courses are not in yet)`, because the engine step writes nothing before
    the course notes exist (`grades.rs:405-415`, `:538-541`) and a captured bundle would only be
    deleted unread (F8s deletes it on every path);
  - `NoRank`, the run after `config.ingest` (and `school.apply`): leaves out `rank` only, so it runs
    the vault's first `grades`, with the capture, before its first `ingest`;
  - `Full`: today's slot.
  Each step a scope leaves out is named at exit 0 as `<step> (not yet: <why>)` (for example
  `rank (not yet: the first full slot ranks)`). **These rows do not join the runner log** (review
  r5, M3): the filter at `scheduler.rs:1036-1037` matches `(skipped:` forms only, so a scope's
  rows stay in the summary and out of `state/runner-log.md`, a vault file, which onboarding's
  phases have no reason to fill. Everything else is the slot's own: the session refresh, the
  entitlement and privacy holds, the grades capture and its named skip (in `NoRank` and `Full`), the mail pass, backup,
  telemetry, their runner-log lines and the stop checks.
- **`slot.first_full`** (on step 10's confirm): `run_slot_inner_scoped(…, Some(app), …, Full)`,
  which runs the slot's own mail pass when a mailbox is connected (no second mail step; review r3,
  M1). Its end is the "Effort and ranking" item of §4.6. Both slots honour the stop before each
  stage (F8s, F8g, §4.10); a slot stopped before `rank` leaves `state/today.md` absent, so
  `ingest::is_first_run` still holds for whoever runs the rest.
- **No slot starts outside the queue in onboarding mode** (review r5, I-4). `scheduler::run_slot`
  has four callers (`tray.rs:49`, `scheduler.rs:1162`, `:1206`, `mail/live.rs:121`). The tray and
  `scheduler::spawn` do not exist in onboarding mode; the fourth, a closed mail-backfill batch's
  `on_closed`, **tells the queue instead** (F8m): no slot is started, and the batch is pulled by
  the next slot the queue runs (`judge`'s pull in `NoRank` or `slot.first_full`). A slot started
  there would rank before `config.ingest` (ending `is_first_run`), take `sync.lock` outside the notes
  gate, escape the drain, and hold `sch.running`, so the queue's own slot would get the stale
  `sch.last` back (`scheduler.rs:782-786`). **A batch that closes after `slot.first_full` has run**
  (review r6, M3) has no queued slot left to pull it: it waits for the first scheduled slot after
  the reveal (the normal console's, at the next slot time), which is acceptable because backfills are
  never waited for (§4.6). The rail's and the reveal's `older_mail` line says so in that case (the
  words are the direction's; the meaning is "being read; what it finds appears after Knowlu's next
  scheduled run"), from the queue's `mail_batch_closed()` record (F8m).
- **Idempotent on resume.** On relaunch, every `running` job becomes `pending` and runs again: captures
  re-read, engine steps skip what they have seen, mail setup resumes by its own records.
- **Failures never stop the flow.** A failed job shows its fix on the rail and at the reveal, and has
  a "Try again"; the student may go on.

### 4.4 The read-only engine command (F7a, F7c)

`knowlu-engine onboarding --vault <v> --json` with exactly one mode:
- `--preview --feed <ics file> --courses <json file> [--grades <bundle>] [--registrar <file>]`:
  step 3's counts (`classes`, `class_times`, `assignments_upcoming`, `turned_in`, `grades_courses`;
  also per course key as `by_course`, F10's review)
  from the raw captures, counting only what is due after now in the vault's zone, and joining
  turned-in items by O1's id rule only.
- `--tidied`: the reveal's counts from the vault: `imported_past` and `imported_done` (archive notes by
  `archived_reason`), `effort_estimated` (live tasks whose effort an agent set), and per-source live
  task counts (`live_by_source`) for the rail's proofs of steps 6 and 8.
It **never writes**, makes no network call, is **not gated** (it reaches no cloud), always exits 0
(an unreadable input is a named `null` field and a named line in `errors`), and its output is
deterministic (keys sorted, `ledger::dumps_value`). It is a new entry in `CLAUDE.md`'s command
contracts (§9, Quinn's).

### 4.5 `onboarding.json` and the page's commands

- **The file:** `<profile data>\onboarding.json`, beside `settings.json`, never in the vault, never
  synced. Fields: `version`, `step`, `finished` (bool), `school` (unitid, label, `path`: `shared`,
  `separate` or `paste`), `skips` (step and source ids), `jobs` (name → state, proof numbers,
  reason), `account_settled`, `step10_confirmed_at`, `started_at`, `handover` (items, each with its
  clearing signal, §4.10). **No secret, no URL, no cookie, no title.**
- **The answers file** (review r2, I2): `<profile data>\onboarding-answers.json`, beside it, never in
  the vault, never synced. It keeps **every answer a later job needs**, written the moment the answer
  is given, so a quit at any step loses none:

  | Answer | From | Needed by |
  |---|---|---|
  | the course list (code, name, label, the LMS's course key, and per row `removed` and `school.preview`'s counts: R-C1-51, F10's review) | step 3's capture, or the paste fallback's typed list; step 3's Remove; `school.preview` | `school.courses`, `config.ingest`'s `course_map`, step 3's proof totals (kept rows only) |
  | the registrar rows | step 3's capture | `registrar.confirm` |
  | each course's slug, after `school.courses` (a restored note's slug where one matched, §4.9) | `school.courses` | `config.ingest`'s `course_map`, step 8's suggestions |
  | `google_calendar` (a flag) | step 6 | `config.ingest` |
  | each feed's state: `validated`, `stored`, `rejected` (never the link) | steps 3 and 7, `school.store`, `calendar.store` | `config.ingest` (§4.11) |
  | the confirmed book and section rows, `zybooks_ignore` | step 8, per row | `config.ingest` |

  It holds course names and class times, which the vault holds anyway after payment, and no URL,
  cookie, login or grade. It is written by temp file and rename (app data), deleted when onboarding
  finishes (after `config.ingest` and `registrar.confirm` are `done`), and deleted by sign-out and
  account deletion (F2c, F2e). An unreadable answers file is never read as "no answers": the step whose
  answers it held is resumed and asked again (a re-capture or the confirm rows), and no config job
  runs until it is rewritten. **An absent answers file past step 3 is treated the same way** (review
  r3, I-C): it means the answers were deleted or never saved, never that there were none.
- **Sign-out during onboarding rewinds to step 2** (review r3 I-C; the step corrected in review r4,
  I-1). Sign-out deletes the answers file and the pending feed links (§4.11), the school folder (S5)
  and the session. So, in the same call, `onboarding.json` is rewritten to **step 2** with every job
  `pending`, the feed states, `account_settled` and `school` cleared (`school` added in the T0
  review, M5; §4.7), and the skips kept. The next open of this
  profile lands on step 2's sign-in inside onboarding mode (§4.2), then step 3, which asks again what
  the deleted files held: the school sign-in and capture, then steps 6-8's confirms (a grant already
  on the account shows `already-done`; stored homework logins, which sign-out keeps, pre-fill step 8).
  Jobs whose output is already in the vault re-run idempotently: create-new finds their files and
  notes and keeps them (§4.9). `finished: true` is never touched by sign-out. Independently of the
  file, **a profile with no session always resumes at step 2** (F2a's resume rule), so a session lost
  any other way also lands on a sign-in, never on step 3 with nothing to pay with. The function is
  `onboarding_state::on_sign_out(profile_dir, profile_id)` (F2e), called from
  `account::sign_out_clean` (F2c).
- **No save after the rewind** (review r4, I-1). `onboarding_sign_out` drains the queue and the lane,
  makes their last save, then **freezes** the queue's persistence (any later save is refused and
  logged), and only then signs out; the process exits right after. So the live queue can never write
  its in-memory step and jobs over the rewound file. **The freeze lives on the queue's own saver**
  (F8f), never in F2a's file save (review r5, M1): `on_sign_out`'s rewind writes through F2a
  directly, after the freeze, and must land.
- **Unreadable file:** never read as "finished". Onboarding resumes at the earliest step not proven by
  what the account and this device hold (§4.7), and the file is rewritten from that. **Never past
  step 3** (§14.49): the school is never proven and the rewritten file has none, so the resume is
  step 2 (no session) or step 3's school confirm; after it, proven steps pass through as
  `already-done`.
- **Absent file on a registered profile:** a normal launch (every install before this lane).
- **Commands (console list, onboarding mode):** `onboarding_status` (the file plus live job states),
  `onboarding_answer(step, answer)` (records the answer and enqueues its jobs), `onboarding_skip(step,
  source)`, `onboarding_retry(job)`, `onboarding_finish` (the reveal's button), and the two exits
  (review r4, I-1): `onboarding_quit` (Finish later, and the window's close) and
  `onboarding_sign_out` (§4.10). The three account commands of review r5 (I-1, I-2), each reading
  the vault's own session through `config/cloud.yaml`: `onboarding_attach_session(account_id)` (step
  2 again, F2g), `onboarding_entitlement` and `onboarding_checkout(plan)` (step 4, F2h). Plus the
  moved commands of F5. Each returns the `{ok, error, …}` envelope; none computes a count. Every one
  that waits (a drain, a network call, Credential Manager) is `#[tauri::command(async)]`, so none
  runs on the event loop that the sign-in, capture and mail windows need (review r5, I-5).
- **The picker** lists a profile whose `onboarding.json` is unfinished as "setup not finished", and
  opening it resumes onboarding (`launch_state` gains a per-profile flag; `profiles.json` is unchanged).

### 4.6 The reveal gate (F9, a pure function)

Inputs: the jobs' states, the skips, `step10_confirmed_at`, now. **Required before the reveal opens**
(spec §1.5, Quinn's list, each only if its source is connected): first `sync`; first `coursework`;
first `ingest`; first `grades`; class times (step 10 confirmed); mail (forwarding verified by the
canary **and** the first mail pass delivered); Gmail (the first read, or the canary for forwarding);
effort and ranking (`slot.first_full` done). Syllabus reads join the list in O5. **"First grades"
is marked only by a slot whose `grades` ran** (T0, 2026-10-24): `NoRank`, or `slot.first_full` if
`NoRank` was stopped; a `BeforeConfig` slot's `grades (not yet: …)` row never marks it, and a
`grades (skipped: …)` row (no bundle, no LMS, a `campus.yaml` that landed mid-slot) leaves it
pending for the next slot. So the order of the gate's items is first `sync`, then `grades`,
`coursework` and `ingest` together in `NoRank`, then effort and ranking.

**Opens** when every required item is `done`, `failed` or `skipped`, **or** when now is 3 minutes after
`step10_confirmed_at`, whichever is first. **Never waits for** a backfill (Gmail forwarding's 120 days,
Microsoft 365's older mail). **Still coming** lists every required item not yet `done`, plus every
backfill in progress; **not connected** lists the skips, each with its cost. A `failed` item is named
with its fix, never hidden. The gate is re-evaluated each time `onboarding_status` is read.

### 4.7 Restored, adopted and second-device installs

At step 2 the account may already hold a vault (C3′'s restore) and this device may already hold
sessions. Each step checks, before it asks, what proves it done. **Config never syncs, so a restored
vault has no `campus.yaml`** (review r2, M5): its school is never "proven". **The account does not
say which school either** (T0, 2026-10-24): `GET /account` returns a stored feed's `kind` and
`added_at` only (`cloud/supabase/functions/account/index.ts:192`), never its host, and this plan
makes no cloud change. So step 3 on a restored install suggests a school only from what **this
device** keeps: `onboarding.json`'s `school` (unitid and label, §4.5) from an earlier onboarding of
this profile, as a confirm; with none, step 3 asks for the school as a fresh install does. **On a
new device, step 3 always asks for the school** (T0 review M5): the profile is new, so it has no
`onboarding.json`, and nothing else on the device or the account names the school. **Sign-out
clears `school`** with the rest of the rewind (§4.5): the answers, the pending links, the school
folder and the session go, and the school choice goes with them, so the rewound file names nothing
the next sign-in has not confirmed. So the stored-school arm arises only when this profile resumes
an onboarding it left without signing out (a quit, Finish later, a crash) after step 3 saved the
school. F1 test 2 covers both arms. Its proof
is the restored course
notes' count; this device still needs the school sign-in where the row has grades or a registrar
(sessions are per device), and `school.apply` creates `campus.yaml` after the confirm. The other
checks: payment (entitled), school email
(an active forwarding for the school mailbox), Google (the account's grant), homework sites (a stored
login on this device and the mapping in the vault), "Your week" (class commitments for every current
course). A proven step shows `already-done` with its proof and is passed through. Portal logins and
school sessions are per device, so a second desktop still signs in to the school and the homework
sites (spec §1.11).

### 4.8 What each design slot receives (for the directions)

Every step's slot gets: its `why` key (the copy is the direction's; Quinn reads it), its state from
§4.1, its proof fields from §4.2, its skip cost key, and the rail. The reveal's slot gets §4.2 step
11's blocks. A direction may merge two steps on one screen only if each keeps its own why, ask, proof
and skip; it may not reorder steps, hide a skip's cost, or add an ask.

### 4.9 Every config file is written once (review r1, Critical 1)

Each file the app writes under `config/` is created **once, create-new, at the first step that knows
its whole content**, as `journal::create_actor_file` writes `config/actor.yaml` (`journal.rs:213`).
An existing file is **kept, never replaced**: no wholesale rewrite exists anywhere in onboarding.

| File | Created by | When | Its whole content is known because |
|---|---|---|---|
| `.gitignore`, `config/planning.yaml`, `week_template.yaml`, `actor.yaml`, `runners.yaml`, `cloud.yaml` | F3, inside today's staged create | step 2 | all defaults (slots, Windows' zone, device) or the account |
| `config/campus.yaml`, `config/events.yaml` | `school.apply` (F6a) | after payment | the school is picked and its path settled (curated: its row and preset; uncurated: the LMS kind from the window or the paste, events preset `none`) |
| `config/ingest.yaml` | `config.ingest` (F6b) | on leaving step 8, after `school.courses` and both feed stores settle | each feed per §4.11 (`ics_url: ''` when the account holds it; the link itself when the account could not take it or the paste was rejected), `course_map` (step 3's kept courses and `school.courses`' slugs, R-C1-48's pairs), `course_exclude` (the removed courses' pairs, only when one was removed; §14.45), calendars (steps 6, 7), books, sections and `zybooks_ignore` (step 8), all from `onboarding-answers.json` |
| `courses/*.md` | `school.courses` (F6a) | after payment, once `account_settled` | notes, created through `write` as the student, as today's seeds are |

- **How "create-new" behaves** (review r2, M4). The text is built and parse-checked first (for
  `ingest.yaml`, through `coursework::load_coursework_config`, as `build_into` does) and written to a
  temp file in the same `config/` folder (`.<name>.tmp-<pid>`). It is then put in place with
  **`std::fs::hard_link(temp, final)`**, which fails with `AlreadyExists` when `final` exists and never
  replaces it (`CreateHardLinkW`); the temp file is then removed, and a stray one from a crash is
  removed on the next run of the job. `std::fs::rename` is never used for this, because on Windows it
  replaces an existing file. **At run time,** when the hard link fails for any reason but
  `AlreadyExists` (a FAT, exFAT or network volume answers "not supported"), the helper falls back to
  `MoveFileExW` **without** `MOVEFILE_REPLACE_EXISTING`, which also fails rather than replace (review
  r3, M2); only if both fail is the job `failed`, with the volume named in its fix. If the file exists: when its bytes
  equal what would be written (a resumed job), the job is `done`; otherwise the existing file is kept,
  the job is `done` with the note "kept your existing settings", and nothing is overwritten.
- **Why no restored vault is overwritten.** Config never syncs (`sync` carries only
  `ids::NOTE_FOLDERS`), so a C3′ restore or a second device brings notes, never config; any config
  file already in the folder was made on this device and is kept by the rule above.
- **Why no course note is duplicated** (review r2, I5). A down network at step 2 reads as an empty
  restore, so "the account has no notes" is known only when `account_settled` is true (§4.3).
  `school.courses` therefore waits for it, then creates a note only where none exists for that course
  (by its path, its `source_uid` or its code), and records each course's slug in the answers file: a
  restored note's own slug where one matched, the new slug otherwise. `config.ingest`'s `course_map`
  uses those slugs, so it names the restored notes (F6a's and F6b's tests). If the account is never
  reached, `school.courses` stays `pending` with its reason and Try again; it never guesses.
- **What it costs.** `ingest`, `coursework` and the calendar reads start only after step 8, not after
  payment. Steps 5-8 take a few minutes, so the reveal may wait longer; S-E measures it (§3).
- **Exceptions: none planned,** so `CLAUDE.md` needs no change for this (§5, Q-B1). The one candidate,
  only if S-E shows the wait breaking the 3-minute cap, is `ingest.yaml` created at payment and then
  extended at steps 6-8 by adding list items in place. That would need Quinn's ruling and a
  `CLAUDE.md` row (§9, A17, conditional).
- **After the reveal,** changes go through today's Settings paths, unchanged by this plan.

### 4.10 Quitting, finishing and handing over (review r1, Important 2)

Every way out of onboarding ends this process (below), an engine child's output is piped to it, and
an engine child or a hidden window must never be cut off mid-write. So every way out stops the work in
flight **at its next safe point**, never at the end of everything queued (review r2, I3):

| Job in flight | Its safe point | What happens to the rest |
|---|---|---|
| `slot.partial`, `slot.first_full` | the next **stage boundary** of `run_slot_scoped` (review r3, M1): every stage it names with `steps.start` (session, entitlement, privacy, the grades capture, the mail pass, each engine child, backup, telemetry) checks the stop before it starts; the running stage ends within its own bound (an engine child's `CHILD_TIMEOUT`, the capture's and mail pass's own limits; S-E measures the longest) | the stages not started are not run, recorded as not run and never as a failure; the slot is `pending`. **The grades bundle is deleted on every path** (review r4, M1): a stop skips the remaining stages, never the bundle's delete, which runs whatever happened, as it does today after the engine loop |
| a hidden read on the school folder | the window closes (a read is re-runnable; nothing it saw is kept) | the read is `pending` |
| `mail.setup` | the end of its current recorded step (its own records make it resumable) | the rest is `mail::live::on_start`'s, as today |
| `mail.canary`, `mail.first_pass` | at once: waiting for mail writes nothing | the wait continues through the mail lane's own paths |
| a local-lane job | its end (seconds; no network wait beyond the account call's own timeout); one still waiting for the notes gate (§4.3) has not started and does not start | none, or the waiting job stays `pending` |

**The stop flag** (review r3, I-A) is one `AtomicBool` on `Scheduler`. Onboarding's drain sets it;
**nothing ever clears it**, not the scheduler at a slot's start and not the queue: it lives for the
process, and every way out of onboarding ends the process by its own command (below). So a stop
set while the queue is between dequeuing a slot and starting it still stops that slot before its first
stage. The normal console never sets it. **Two flags, one call** (F8f review I-1, §14.51): the
queue keeps its own stop (`Inner.stop`, which `Queue::request_stop` sets) and the slot checks
`sch.stop`; F2b's live `ExitSeams::request_stop` sets **`sch.stop` first, then the queue's**, in one
call, so a slot in flight sees the stop at its next stage boundary. Neither is ever cleared.

**The drain has a limit** (F8f review I-2, §14.51): `DRAIN_LIMIT`, **45 seconds**, for every
serial-queue job that writes nothing to the vault: the hidden school reads, `school.preview`,
`mail.setup`'s current step, the Google reads and `coursework.discover` (the canary and the first
mail pass are already abandoned at once). At the limit each of those still running is **abandoned**
the way the canary is today (recorded `pending`, exactly as a crash leaves it; its late return
changes nothing), and the exit goes on; the next launch re-runs it. **Two kinds are never
abandoned:** a slot (its engine children write the vault, and abandoning it would leave a child
running after this process ends) and a local-lane job (seconds of config, account and
`registrar.confirm` writes). Each ends at its safe point within its own bound (a slot's running
stage, at worst one `CHILD_TIMEOUT`). `quit_flush`'s 10-second cap runs **after** the drain, never
alongside it, so the exit takes at most 45 + 10 seconds with no slot running, and the running
stage's end plus 10 seconds with one. **The page's words** (F11a; F2b's progress answer gives the
job and whether it is a slot):
- from the start: "Finishing <job>…";
- from 15 seconds, for a job the limit covers: "Still finishing <job>. Knowlu will close within a
  minute and pick up the rest next time.";
- from 15 seconds, for a slot: "Saving what Knowlu has read so far. This can take a few minutes,
  and Knowlu closes by itself when it's done."

**The three exits, each one command in F2b** (review r4, I-1; `relaunch` only spawns, so each command
ends the process itself):

| Exit | Command | In order |
|---|---|---|
| Finish later; the main window's close (F4 routes it here, off the event loop) | `onboarding_quit` | set the stop; drain to the safe points; save; **today's quit path**: stamp `settings.quit_at`, then `state::quit_flush` (push and back up, capped at 10 s), exiting in its `then` |
| Sign out | `onboarding_sign_out` | set the stop; drain; save; **freeze** saves (§4.5); `account::sign_out` (its clean runs `on_sign_out`, rewinding to step 2); `onboarding::relaunch(["--pick"])`; exit. **No push**: the session is about to go, and the vault stays, so the next sign-in's first slot pushes |
| Open Knowlu | `onboarding_finish` | refuse while the gate is closed or a lane job is not `done`; set the stop; drain; write `finished` and `handover`; delete the answers file; enable autostart per settings; `onboarding::relaunch_with(vault)`; then today's quit path as `switch_profile` runs it (the spawn before the flush, so the child waits on our pid while it runs) |

- **Why today's quit path** (§13.1, row `quit_flush`). Every quit today, the tray's and
  `switch_profile`'s, stamps `quit_at` (which `startup_missed` counts from) and pushes and backs up
  under a 10-second cap (`tray.rs:105-121`, `commands.rs:669-686`). The plan's exits skipped it, so
  the course notes written at payment would wait for the next slot to leave the device. The drain
  runs first, so the flush's `vault_io` meets no job.
- **One exit at a time.** A second exit while one is draining (a second click on the window's close,
  Finish later after Sign out) is answered `{ok: true, exiting: true}` and does nothing (one
  `AtomicBool` in F2b). With the drain's limit (above) the first exit always ends, so the second
  close is never the only way out.

- **A relaunch that fails never leaves a live, stopped process** (review r4, M5). If `relaunch` or
  `relaunch_with` returns an error (`current_exe`, the spawn), the command does not stay running with
  every Try again dead: it returns `{ok: false, restart: true}`, the page shows one screen ("Knowlu
  needs to restart: open it again", the words the direction's) whose only button calls
  `onboarding_quit`, which exits. The state written before the relaunch stands, so the next manual
  open lands where it should (the normal console after finish, the picker after sign-out).
- **No other console exit is reachable in onboarding mode** (review r4, M6). `switch_profile` refuses
  in onboarding mode with a named line ("finish or leave setup first"; F4b), and no onboarding screen
  calls `switch_profile` or `delete_my_data` (F11t pins both; Settings is not reachable in onboarding
  mode). Account deletion stays the normal console's, after the reveal.
- The page shows "finishing <job>…" while a command drains (the words above). A job interrupted
  anyway (a crash) is `pending` on the next launch, **unless it was running at two launches in a
  row** (§14.51, the crash-loop guard; F8a2): then it is `failed` with `job_crashed`, so a job that
  takes the app down every time it runs cannot do it at every launch (release builds are
  `panic = "abort"`, and the queue resumes a pending job first).
- **Open Knowlu needs the local lane done.** `school.apply`, `school.courses`, `registrar.confirm`
  and `config.ingest` (and through it both feed stores, settled per §4.11) are seconds of local work.
  Open Knowlu is enabled only when all are `done`; a failure shows its fix and Try again. One case
  waits longer by design: `school.courses` waiting to reach the account (§4.9), shown with Try again
  and Finish later.
- **The hand-over.** At Open Knowlu, every required item of §4.6 not yet `done` is written to
  `onboarding.json` as `handover`, each **with its own clearing signal** (review r2, M5), with
  `finished: true`. The normal console then finishes them by its usual paths, and onboarding runs
  nothing after the relaunch:

  | Item | Finished by | Cleared when |
  |---|---|---|
  | the slot (`slot.partial`'s rest, `slot.first_full`) | `scheduler::spawn`'s first-run slot, since a slot stopped before `rank` leaves no `state/today.md` (App mode and `device_ok` hold for a step-2 vault; T0 confirms), or the next slot | the first slot after the reveal ends, whatever its outcome |
  | the Gmail first read, a calendar read | that slot's `judge` and `ingest` | the same slot's end |
  | an open mail setup, the canary, the first mail pass | `mail::live::on_start` and the mail lane | forwarding verified (the canary), the first pass delivered, or the mailbox disconnected, read by F8s2 at a normal slot's mail stage (§14.51: verified from the service's address row, which `mail.json` does not hold; delivered and disconnected from `mail.json`); never by the slot's end |

- **The main page's "still coming" line** reads `handover` and drops each item at its own signal. A
  failure then shows where that path's failures show (the Runs view, the issues panel, the mail row).

### 4.11 A feed link waiting for the account (review r2, I1)

The LMS feed link (step 3) and the other calendar's link (step 7) are bearer links. Each is kept
until the account or the vault holds it, and **never silently lost** (R-C3'-exec-40):
- **Where it waits.** In Credential Manager, `knowlu/<profile_id>/pending-lms_ics` and
  `pending-calendar_ics` (the app's `knowlu/<profile_id>/<source>` shape, written through
  `credentials::write`, unedited), from the moment it is captured or pasted. Never in
  `onboarding.json`, the answers file or the vault before `config.ingest`. It survives a quit, so a
  resume after payment needs no school window, at any school.
- **Only `school.store` and `calendar.store` send it** (review r6, I-2). Today's capture and paste
  commands store a validated link at once under the pending session (`lms_link::finish`,
  `lms_link.rs:843-847`), which step 2 emptied, and which, being one per machine, may hold another
  sign-in parked there (whose account would then receive this student's link). So onboarding mode
  never registers them: F8l's `onboarding_capture_link` and `onboarding_paste_link` validate through
  `finish_with` with a store seam that sends nothing, then write the pending credential, and
  `school.capture`'s background read uses the same seam (`capture_link_only`, F8la; review r7 I-1).
- **`school.store` / `calendar.store`** (local lane, entitled): send a **validated** link to the account
  (`PUT /account/sources`), retrying once as `create_vault_in` does. On success the feed's state is
  `stored` and the pending credential is deleted. A link the paste **rejected** is never sent
  (R-C3'-exec-41); its state is `rejected`. A store that fails twice is `failed` ("kept on this
  computer"), which settles it for `config.ingest`.
- **`config.ingest`** waits until each feed is `stored`, `rejected` or `failed`, then writes
  `ics_url: ''` (or the calendar's empty entry) for a stored feed and **the link itself** for a
  rejected or failed one, exactly as today's `build_into` plus `restore_capability_url` would leave the
  file; it then deletes the pending credential. The rail and the reveal name a link kept on this
  computer (proof field `feed_kept_on_device`, per feed; the words are the direction's), never a bare
  `''`. How a kept link reaches the account later is today's path, unchanged.
- **Deleted also** by sign-out (F2e's `on_sign_out`, which also rewinds onboarding to step 2, §4.5)
  and account deletion (F2c), and by the student's Back to another school before payment. An
  abandoned setup keeps the waiting link until one of those; the privacy page says so (S7).

---

## 5. Questions: ruled and decided

### 5.0 Rulings (Quinn, 2026-10-23, after the plan passed review r8)

| Question | Ruling | Where it lands |
|---|---|---|
| Q-B3 | **Yes.** Add the read-only `knowlu-engine onboarding` command, with a `CLAUDE.md` line for it. The line is drafted here (§9, A11). `CLAUDE.md` itself is edited by the task that adds the command (F7a), in its own commit | §4.4, F7a, A11 |
| Q-B10 | **Yes.** Add the `onboarding_step` event, gated on the privacy page naming it first | F7b, S7 |
| Q-B11 | **O4 merges first, then O3, with no release tag in between.** Amendment A18 is recorded | §11, A18 |
| Q-B14 | **The allow list, with a fallback to the mailbox's own `mail-session-<id>` folder** on a refused host, as recommended. The deny-list alternative is closed | S3, S4c |
| Design direction | **5b, Departures, as the spine, with the grafts listed in `docs/design/2026-10-23-directions/README.md`** ("Grafts" and "Round 3"). Build from `5b-departures-board/index.html`, `screens.json` and `BRIEF.md` | F10, F11a-F11f, S6b |

- **Decided by the controller, standing:** Q-B15 to Q-B19, as written below. Quinn re-ruled none of
  them. **Q-B20** (new at T0) was decided by the controller on 2026-10-24 (below).
- **Q-B21, ruled by Quinn on 2026-10-24:** wave B may ship with two Google consents in step 6
  (Calendar, then Gmail) until O2 and F11f combine them (below).
- **Q-B1** was not in this ruling set. It stands at its recommendation, "no exception": every config
  file is created once, and `CLAUDE.md` needs no change. A17 stays unused.
- **Q-B2, Q-B4 to Q-B9, Q-B12 and Q-B13** are the controller's, at the recommendations below.
- **Separate from wave B:** Quinn also ruled an evening ranking rule. After a set hour, items due
  tonight rank first. It is drafted as its own spec note,
  `docs/specs/2026-10-23-evening-ranking-rule-draft.md`, and **no wave B task builds it**.

The questions below are kept as they were argued; each one's ruling or decision is the table above.

- **Q-B1 (Quinn, reframed after review r1): is any exception to "written once" needed?**
  *Recommend: none.* Every config file is created once, create-new, at the first step that knows its
  whole content, and an existing one is kept (§4.9). No wholesale rewrite exists, a restored or
  second-device vault is never overwritten, and `CLAUDE.md` needs no change. *The price:* `ingest`,
  `coursework` and the calendar reads start after step 8 rather than after payment. *The only
  candidate exception,* to bring if S-E shows the 3-minute cap usually broken: `ingest.yaml` created at
  payment and extended at steps 6-8 by adding list items in place (the `restore_capability_url`
  shape). It would need Quinn's ruling and a `CLAUDE.md` row (A17). *Withdrawn:* the round-1
  recommendation (re-render whole while unfinished), which could overwrite a restored vault's config
  with no journal record (review r1, Critical 1).
- **Q-B2 (revised after review r2, I1): where a feed link waits for the account.** *Recommend:*
  Credential Manager, `knowlu/<profile_id>/pending-lms_ics` and `pending-calendar_ics`, from capture
  or paste until the account holds the link or `config.ingest` writes it into `ingest.yaml`; deleted
  then, and by sign-out and deletion (§4.11). *Why it changed:* round 1's "memory only" lost the link
  on a quit after payment (the job resumed with nothing to store), needed step 3's window again to
  recover it, and at a school without `shared_sign_in` that meant an unplanned second sign-in; and it
  had no fallback when the store failed, so `ics_url: ''` could be written over nothing. *Cost:* two
  new transient target names, listed in `docs/reference/app.md` (S8), deleted by S5's sign-out and
  deletion. *Alternative:* a file in app data; refused, because the link is a bearer secret and the
  app keeps secrets only in Credential Manager.
- **Q-B3 (Quinn, a `CLAUDE.md` line): the new read-only engine command.** *Recommend:* add
  `onboarding` (§4.4) and its sentence to "Command and app contracts" ("`onboarding` never writes,
  reaches no network and always exits 0"). *Why not `surface`:* a new key in the read model changes
  `surface-today-*.json`, which then need a regeneration commit.
- **Q-B4: existing installs under O4.** *Recommend:* a feature moves to `school-session` only once the
  folder exists (the student signed in through it: at onboarding, or from Settings' "Connect"); until
  then each keeps today's folder. When a feature moves, its old folder (`lms-session`, or the school
  mailbox's `mail-session-<id>`) is forgotten through its own guard. No folder is renamed or moved.
- **Q-B5: a "keep school sign-in" switch?** *Recommend:* no new switch; Forget only (Q9). After Forget,
  the next sign-in keeps again. The mail lane's saved-sign-in switch still decides whether the school
  mailbox *uses* the folder (spec §2.5).
- **Q-B6: which agent edits the window modules (S4a-S4d).** *Recommend:* `implementer` as the spec
  says, with `test-writer` first, and `contract-reviewer` (not only `reviewer`) reviewing each diff: a cookie
  read for the wrong host leaks a live session, CLAUDE.md's first test for Opus-level care.
- **Q-B7: the window during onboarding.** *Recommend:* maximised, not exclusive full-screen, never
  always-on-top, so the school window, Google's consent and Stripe's checkout come in front. The
  direction designs within that.
- **Q-B8: the coursework mapping confirm (R-OB-1) at step 8.** *Recommend:* keep it, as pre-filled
  confirm rows on the homework step (a confirm, not a form; "ask only to confirm").
- **Q-B9: sign-out and today's `lms-session`.** *Recommend:* S5 makes sign-out delete every kept school
  profile, `lms-session` included where it still exists, which is what spec §2.5 says sign-out does.
  Today it does not (§ "What the code read found"); the privacy page's sentence is checked in S7.
- **Q-B10 (Quinn): drop-off by step.** *Recommend:* one new `uievents` action, `onboarding_step`, with
  the step's id as the object, recorded on entering each step (interaction class (a)); the server's
  telemetry accepts it only if its own vocabulary allows (cloud-engineer checks; if a migration is
  needed, defer to the pilot). Spec §1.11 says the pilot measures this. **The privacy page must
  already say it** (review r1 minor): the page reviewer in S7 checks whether its telemetry sentence
  covers "which onboarding step was reached"; if not, S7 adds that sentence, and since O4 merges
  first (Q-B11) the page carries it before F7b's action exists. If O3 were to merge first, F7b waits
  for the page.
- **Q-B11: merge and release order. A departure from spec §9, stated (review r1 minor).** Spec §9
  lists O4 as depending on O3. *Recommend:* merge O4 first anyway, then O3, with **no release tag
  between the two**, so they still ship together as §9 requires. *Why the dependency can be reversed:*
  §9's dependency is about *value* (O3 alone lengthens onboarding and saves no sign-in), not about
  code. O4's code needs nothing O3 builds: its windows, Forget, sign-out and Settings row work in
  today's console. O3's school step does need O4's `school_session::profile_for` and lock (F8b). So
  in code the dependency runs from O3 to O4, and merging O4 first is the order the code needs.
  *Alternative:* one integration branch merged once. It follows §9 to the letter, but it makes one
  very large PR and gives up O4's separate contract-list review. Quinn may prefer §9's wording; if
  so, the two branches merge back to back in one sitting.
- **Q-B12: timezone.** *Recommend:* keep Windows' zone from step 2 (R-OB-4's state suggestion is
  dropped as a default: the computer's clock is better evidence of where the student is), and show it
  on "Your week" with Edit. No ask.
- **Q-B13: what counts as a "use" of the school folder.** *Recommend:* three recorded uses in
  `<profile data>\school.json`: `grades` (connected), `school_mail` (forwarding on and the saved
  sign-in on), `registrar` (the school has a registrar row and it has been read). The LMS capture is
  transient. The folder is deleted when the last use clears, and always by Forget, sign-out and
  deletion. `school_mail` clears on the school mailbox's Disconnect (after forwarding is turned off
  through the session) and when the saved mail sign-in switch goes off (spec §2.5; S4c).
- **Q-B14 (revised after review r2, M1): a navigation guard on the mail window.** *Recommend: an
  allow list.* On the shared folder, the mail window's top-level navigation is allowed only to the
  mail provider's hosts (the set `popup_allowed` already trusts) and the curated row's
  `sign_in_hosts` (the identity provider and MFA hosts S-A phase 2 lists, §3); every other host is
  refused, a pure predicate. Its popup filter stays as it is; `init.js` is not edited (L1 puts it on
  `contract-engineer`). *Why it changed:* round 1's deny list (LMS and registrar hosts) still let the
  mail window, which carries `INIT_SCRIPT`, open on any other school host (Simple Syllabus, a second
  SIS) with that host's cookies. Round 1 refused an allow list because identity-provider hosts differ
  by school; but the shared folder exists **only at a curated row with `shared_sign_in`**, whose hosts
  the spike has just observed, so the list is known exactly where it is needed. **A refused host never
  dead-ends** (review r3, M3): `sign_in_hosts` comes from one sign-in on one device, and a classmate
  may meet a host it lacks (Microsoft's first-time MFA registration, a conditional-access page). On a
  refused navigation the mail window on the shared folder closes, and the school mailbox falls back
  to **its own `mail-session-<id>` folder** (today's path, with its own window and sign-in), records
  no `school_mail` use, and logs the refused host's kind. *Cost:* that student signs in to the mailbox
  once more, and the row is updated from the logged kind when a pattern shows. *Alternative, for Quinn
  only:* keep round 1's deny list and record the remaining exposure as an accepted risk.
- **Q-B15 (new, review r2 I2): course names in app data before payment.** *Recommend:* yes, in
  `onboarding-answers.json` (§4.5). Spec §1.4 B forbids writing a **note** before payment; the answers
  file is not in the vault, never syncs, holds no secret or grade, and is deleted at the reveal,
  at sign-out and at account deletion. **An abandoned setup keeps it** (and any waiting feed link,
  §4.11) until one of those (review r3, M6); the privacy page names both, with that retention (S7).
  Without it a quit at step 4 needs step 3's window again (I1's case b). *Alternative:* re-capture on
  resume, which brings back the second sign-in at a school without `shared_sign_in`. *Not chosen:* an
  age-based sweep of an abandoned file; it would turn a resume weeks later into a silent re-ask.
- **Q-B16 (new, review r2 I3 and I5): what Open Knowlu may wait for.** *Recommend:* only the safe point
  of the job in flight (one slot stage at most, §4.10) and the local lane, including `school.courses`
  waiting to reach the account. If S-E's longest stage exceeds 60 seconds, that number comes back for a
  shorter onboarding bound (§3, S-E). *Why not hand `school.courses` over too:* the normal console has
  no path that creates course notes from a capture, and a course note made before the account is
  reached can duplicate a restored one.
- **Q-B17 (new, review r4 I-1): where a signed-out setup asks for sign-in.** *Recommend:* inside
  onboarding mode, as step 2 again (the sign-in ask without the vault creation, §4.2), for any
  unfinished profile with no session; sign-out rewinds the file to step 2. *Why:* the student stays
  in the one full-screen flow, and the rule also catches a session lost any other way. *Alternative:*
  route such a profile to the shell's sign-in, then relaunch into onboarding; it reuses the shell's
  screen as it is, but adds a third launch path to `LaunchMode` and a shell-to-onboarding hand-off.
  Either way the account must be the vault's own; another account is refused by name. *Review r5
  (I-2) found no attach path exists for either option*, so F2g builds it (`onboarding_attach_session`);
  the shell alternative would need the same function.
- **Q-B18 (new, review r5's audit): do onboarding's exits run today's quit flush?** *Recommend: yes*
  for Finish later, the window's close and Open Knowlu (stamp `quit_at`, push and back up, 10-second
  cap, as the tray's Quit and `switch_profile` do); **no** for Sign out, whose session is about to be
  deleted. *The price:* up to 10 more seconds on a quit with a slow account service. *Alternative:*
  exit without it, as rounds 1-4 planned; nothing is lost (the notes are on disk and the next slot
  pushes them), but `quit_at` stays unstamped and the backup waits.
- **Q-B19 (new, review r5's audit): which steps does the slot before `ingest.yaml` run?** *Recommend:*
  `sync` and `grades` (with the mail pass, backup and telemetry), leaving `coursework`, `ingest`,
  `judge` and `rank` out by name (`SlotScope::BeforeConfig`, §4.3). *Why:* today's argv always runs
  `coursework` and, on a cloud vault, `ingest`, whatever `ingest.yaml` holds, and step 4's text already
  promised they follow `config.ingest`. *Alternative:* let T0 show each runs cleanly with no
  `ingest.yaml` and keep `NoRank` only; that runs `ingest` before the `course_map` exists.
  **Amended at T0 (2026-10-24, the controller's call):** `BeforeConfig` leaves `grades` out too
  (capture and step, one `grades (not yet: …)` row), so its one engine step is `sync`; `grades`
  reads course notes and writes nothing before `school.courses` (§4.3).
- **Q-B20 (new at T0, 2026-10-24; for the controller): how many `BeforeConfig` slots may run while
  the account is still being read?** *Recommend:* re-run `slot.partial` (`BeforeConfig`) at once
  after each fresh `ok` sync whose cursor is short of `pulled_to_end`, up to 10 in a row, then
  `school.courses` is `failed` with "Try again" (§4.3). *Why:* a sync pulls one page per run, and
  `account_settled` now waits for the end of the account's copy, so without a re-run a large account
  whose step-2 restore failed would stall onboarding until the 3-minute cap and past it. *Price:*
  each re-run is a whole slot (session refresh, mail pass, backup, telemetry), seconds each.
  *Alternative:* a `sync`-only job outside the slot, which needs a second path into the engine child
  and the notes gate; or leave `pulled_to_end` out and accept a duplicate course note on a
  multi-page account. **Decided by the controller (2026-10-24, T0 review M1):** not a fixed 10,
  since a page is at most 500 records (`sync-pull/handler.ts:5`) and 10 runs read about 5,000. The
  chain continues while the cursor moves, stops on a run where it did not, and has a backstop of
  50 runs; skipped and failed readings and Try again are as §4.3 states.
- **Q-B21 (new after T0's review, M4): may wave B ship with step 6 asking for Google consent
  twice?** If wave B ships before O2, F11b's step 6 asks once for Calendar and once for Gmail; F11f
  combines them after O2. **Ruled by Quinn (2026-10-24): yes.** Ship with two Google consents in
  step 6 until O2 and F11f combine them.

---

## 6. Conflict map

| File | Change | Task | Shared with |
|---|---|---|---|
| `app/src/school_session.rs` (new) | the folder, guard, patient delete, uses, the lock (S2); onboarding's visible school sign-in (S4d); `school_status` and `school_forget` beside the module they serve (S6a) | S2, S4d, S6a | none |
| `app/src/scaffold.rs` | `Curated.shared_sign_in` and `sign_in_hosts`; the founder's school's values (after S-A); step 2 without campus or events files; create-new writers (§4.9) | S2, F3, F6a, F6b | L5 (none expected) |
| `app/src/grades.rs` | profile choice; one args constant; Forget at a shared school | S4a | L5, O5 |
| `app/src/registrar.rs`, `lms_link.rs` | profile choice; non-incognito on the shared folder (S4b); `capture_calendar_link` split to take its finish, plus `validate_only` and `capture_link_only` (F8la, after S4b merges, before F8b) | S4b, F8la | L5 (`capture_syllabi`), O5 |
| `app/src/mail.rs`, `mail_session.rs`, `mail/commands.rs` | the school mailbox's folder; the navigation allow list; Disconnect and the saved-sign-in switch clear the use | S4c | L1 (R-series), O2 |
| `app/src/account.rs` (**contract list**) | sign-out and deletion delete school folders (S5); `sign_out`'s clean closure calls `onboarding_state::on_sign_out`, and `delete_local_data` deletes the two pending credentials (F2c, O3, after O4 merges); `PRIVACY_VERSION` (S7) | S5, S7, F2c | none in flight |
| `app/src/onboarding.rs` | step-2 vault, `account_settled`, picker flag, relaunch | F3 | none |
| `app/src/onboarding_state.rs` (new) | state, resume, skips, `LaunchMode` (F2a); answers file and pending names (F2d); `on_sign_out` (F2e); commands, finish, hand-over (F2b); the session re-attach (F2g); the vault-target entitlement and checkout (F2h) | F2a, F2d, F2e, F2b, F2g, F2h | none |
| `app/src/mail/live.rs` | `on_closed` tells the onboarding queue instead of starting a slot, in onboarding mode only | F8m | L1, O2 (one closure) |
| `app/src/onboarding_jobs.rs` (new) | queue core, runners, order and local lane, notes gate, stop, drain and freeze, the link commands | F8a-F8h, F8lb | none |
| `app/src/onboarding_gate.rs` (new) | the reveal gate, a pure function (review r8's note) | F9 | none |
| `app/src/main.rs` | onboarding mode; the window's close prevented and handed to a thread running `onboarding_quit`'s inner; moved and new commands | F4, F5, F5b, S6r | every lane adding a command: recount |
| `app/src/commands.rs` | `switch_profile` refuses in onboarding mode | F4b | console lanes (one function) |
| `app/src/scheduler.rs` | `run_slot_inner_scoped` and `run_slot_scoped` with `SlotScope::{Full, NoRank, BeforeConfig}` (wrappers keep all 35 call sites); a stop flag, never cleared, checked before each stage; the bundle delete on every path; one call at a slot's end that clears the slot's `handover` items; `prime_mode_device` (T0) (together about sixty lines); otherwise the queue only calls its public functions | F8s | O1 (merged first) |
| `engine/src/onboarding.rs` (new), `engine/src/cli.rs` | the read-only command: `--preview` and the `cli.rs` entry (F7a), `--tidied` (F7c) | F7a, F7c | L5 (`cli.rs`) |
| `engine/src/uievents.rs` | one action, only if Q-B10 is ruled yes | F7b | none |
| `engine/src/ingest.rs`, `engine/tests/ingest_exclude.rs` (new) | `course_exclude` read and applied at attribution (§14.45) | F6c | O1 (merged), L5 if it edits ingest |
| `app/static/onboarding.{js,css}` (new) | the screens | F11a-F11e | none |
| `app/static/index.html`, `console.js` | the old wizard removed; the hook (F10); `renderFirstRun`'s `(not yet:` reading, one line (F11a, after F10) | F10, F11a | `console.js` queue |
| `app/tests/*` (new files per module), `app/tests/static_assets.rs`, `scripts/wizard-check.py` | tests | S1, S3, S6t, F1, F7a, F7c, F11t, F12 | console lanes (static tests) |
| `site/privacy.html`, `engine/tests/site.rs` | the school sign-in sentences | S7 | L1 (its page row) |
| docs (§9) | references and amendment drafts | S8, F13 | docs-keeper queue |

---

## 7. Tasks at a glance

| Task | What | Agent | Why that agent | Depends on |
|---|---|---|---|---|
| T0 | Branches, state of other lanes, line re-check, P-3 and S-F reads re-confirmed | main session | git, drift judgment | O1 merged |
| T1 | S-A phase 1 (Edge) | main session | live, Quinn at the machine | T0 |
| T2 | Spike build; S-A phase 2 and S-B | main session | live; a throwaway patch never committed | T1 passes |
| T3 | S-E, S-F (live half) and S-G in one session | main session | live, timed | T2 |
| **O4, one school sign-in** | | | | |
| S1 | Failing tests: `school_session` | `test-writer` | the failing test, specified | T2 |
| S2 | `school_session.rs`; `shared_sign_in`; the founder's row | `implementer` | specified, test-checked, off the contract list | S1 |
| S3 | Failing tests: profile choice, one args constant with no debug port, request host equals queried host, mail popups and the navigation allow list, the mail rows of spec §2.5, the lock | `test-writer` | as S1 | S2 |
| S4a | `grades.rs` on the shared folder; Forget at a shared school | `implementer`, reviewed by `contract-reviewer` | specified; a wrong host leaks a session (Q-B6) | S3, T3 (S-G) |
| S4b | `lms_link.rs` and `registrar.rs` on the shared folder | `implementer`, reviewed by `contract-reviewer` | as S4a | S3, T3 |
| S4c | Mail: the school mailbox's folder, the navigation allow list, Disconnect and the saved-sign-in switch | `implementer`, reviewed by `contract-reviewer` | as S4a; Q9's last-use rule | S3, T3 |
| S4d | Onboarding's one visible school sign-in | `implementer`, reviewed by `contract-reviewer` | as S4a | S4a, S4b |
| S5 | Sign-out and deletion delete the school folders | `contract-engineer`, then `contract-reviewer` | `account.rs` is on the contract list | S2 |
| S6t | Failing tests: `school_status`, `school_forget`, the row's static pin | `test-writer` | test first (review r1 minor) | S4a-S4c |
| S6a | `school_status` and `school_forget` (Rust) | `implementer` | specified, test-checked (review r1, I8) | S6t, S5 |
| S6r | Register the two commands | `mechanical` | a two-name list edit, recounted | S6a |
| S6b | The Settings row | `console-ui` | `app/static` | S6a; direction ruled: 5b (`m10-settings` in `5b-departures-board/`) |
| S7 | Privacy page and `PRIVACY_VERSION` | `contract-engineer` (the constant), `reviewer` (page text), Quinn reads | `account.rs`; the page is Quinn's | S4a-S4c |
| S8 | References and O4's amendment drafts | `docs-keeper` | docs | S4a-S7 |
| S9 | Review | `reviewer`; `contract-reviewer` for S4a-S4d, S5, S7 | before a push | S2-S8 |
| S10 | Live proof | main session | live | S9, Quinn's go |
| **O3, the long onboarding** | | | | |
| F1 | Failing tests: state, answers, resume, sign-out rewind, hand-over, gate, queue order, local lane and notes gate, feeds, `LaunchMode`, the scheduler's stop | `test-writer` | the failing tests | T0 |
| F2a | `onboarding_state.rs`: `onboarding.json`, resume derivation, skips, `LaunchMode` | `implementer` | specified | F1 |
| F2d | `onboarding_state.rs`: the answers file, the pending-credential names, the absent and unreadable rules | `implementer` | specified (review r3, M4) | F2a |
| F2e | `onboarding_state::on_sign_out`: delete answers and pending links, rewind to step 2 | `implementer` | specified, tested in F1 (review r3 I-C, r4 I-1) | F2d |
| F2b | `onboarding_state.rs`: the commands and the three exits (quit, sign-out, finish), each draining and ending the process; `stop_both` and the progress answer (§14.51) | `implementer` | specified | F2a, F2d, F2e (the file's chain, review r7 M2), F8f, F8f2 |
| F2c | `account.rs`: `pub fn sign_out_clean(dir, id, api_base)` (today's clean plus `on_sign_out`; the base a parameter so tests stay local), used by `sign_out` with `api_base()`; the two pending names in `delete_local_data`'s credential loop | `contract-engineer`, then `contract-reviewer` | `account.rs` is on the contract list | F2e, S5 merged |
| F2g | `onboarding_state.rs`: `onboarding_attach_session`, step 2 again's re-attach (three-way account match, move on a match, a local delete of the pending session and a refusal on a mismatch) | `implementer`, reviewed by `contract-reviewer` | specified and tested over seams, off the contract list; a wrong move attaches another account's token to the vault (review r5, I-2) | F2b (chained after it in the same file, review r6 M2) |
| F2h | `onboarding_state.rs`: `onboarding_entitlement` and `onboarding_checkout`, the vault-target versions of `entitlement_now` and `open_checkout` | `implementer`, reviewed by `contract-reviewer` | specified, public `account.rs` functions only, tested on a `127.0.0.1:0` listener (review r5, I-1) | F2g (the same file's chain: F2a, F2d, F2e, F2b, F2g, F2h; review r6 M2) |
| F3 | Step-2 vault with defaults, without campus or events files; `account_settled`; picker flag; the name picked in Rust past profiles and folders on disk; the old flow's errors rewritten; F10's interim name loop deleted (F10's review) | `test-writer` then `implementer` | writes a vault; tests first; the static change is a deletion only | F2a, F10 |
| F4 | Onboarding mode in `run_console`; the window's close prevented, then `onboarding_quit`'s inner on its own thread; `mode_device` primed (T0); one update check at launch (T0 review M3); `LaunchMode` and the `Queue` managed (§14.51) | `implementer` | P-3's five changes; the close off the event loop (review r5, I-5) | F2a, F2b, F8s (`prime_mode_device`, `spawn_update_check`), T0 |
| F4b | `switch_profile` refuses in onboarding mode | `implementer` | specified, tested in F1 (review r4, M6) | F2a |
| F5 | Moved and new commands registered | `mechanical` | a list edit, recounted, pinned | F4, F2b, F2g, F2h, F8d, F8lb |
| F5b | Shell list: the retired panels' commands, after F10; five move to the console's list and are never deleted (F10's review) | `mechanical` | `main.rs` is never F10's (review r1, I8) | F10, F5 |
| F6a | `school.apply` (`campus.yaml`, `events.yaml`) and `school.courses` (course notes), create-new | `test-writer` then `implementer` | vault bytes; tests first | F3 |
| F6b | `config.ingest`: `ingest.yaml` once, create-new, feeds per §4.11; `course_exclude` for removed courses (§14.45) | `test-writer` then `implementer` | as F6a | F6a |
| F6c | `ingest.rs`: read `course_exclude`; an item only a removed course claims is not imported (§14.45); the uid pass bounded for pin-form fragments (§14.46) | `test-writer` then `implementer`; `reviewer` with the oracle runs | engine, every vault's import; off the contract list, but the frozen references ride on ingest | T0 (before F15; independent of F6a and F6b) |
| F7a | `knowlu-engine onboarding --preview` and its `cli.rs` entry; `by_course` by code and match set, through `ingest::match_course` (§14.45) | `test-writer` then `implementer` | engine, deterministic, off the contract list | T0 |
| F7c | `knowlu-engine onboarding --tidied` | `test-writer` then `implementer` | as F7a | F7a |
| F7b | One `uievents` action (only if Q-B10 is yes) | `implementer` | a closed vocabulary; its page sentence first | F7a, S7 |
| F8a | The queue's core: states, persistence, the serial thread, holds | `implementer` | orchestration over fakes | F1, F2a |
| F8e | Order rules and the local lane: config before readers, slot order, `account_settled` from the sync status, step 10's gate | `implementer` | as F8a | F8a |
| F8h | The notes gate and the lane's `sync.lock` retry (holding nothing between tries) | `implementer` | split out of F8e (review r4, M4) | F8e |
| F8f | Stop and drain at safe points; the save freeze; hand-over items | `implementer` | as F8a | F8a, F1 |
| F8f2 | The drain's 45-second limit for jobs that write no vault bytes; the progress snapshot (§14.51) | `implementer` | as F8a; its test (F1's 30) written first in the task | F8f (or folded into F8f's fix round) |
| F8a2 | The crash-loop guard: `JobRecord.crashes`, failed with `job_crashed` at a second running start (§14.51) | `implementer` | as F8a; `STATE_VERSION` stays 1; its test (F1's 31) written first | F8f2 (the file's chain) |
| F8s2 | `scheduler.rs`: the normal console clears `handover` (the slot item at a run slot's end, the mail items at the mail stage, §14.51) | `implementer` | off the contract list; its test (F1's 32) written first | F8f merged, F8s |
| F8s | `scheduler.rs`: `run_slot_inner_scoped` (the seams from `Some(app)`) and `run_slot_scoped` with `SlotScope::{Full, NoRank, BeforeConfig}`; the stop before each stage (never cleared); the bundle deleted on every path; the slot's hand-over clearing; `prime_mode_device` (T0) and `spawn_update_check` (T0 review M3) | `implementer` | `scheduler.rs` is off the contract list; its tests are F1's 15, 18 and 27 (review r3 M5, r4 I-3, M1, r5 I-3; T0) | F1 |
| F8b | Runners: the school step (capture, answers, pending feed, preview) | `implementer`, reviewed by `contract-reviewer` (its capture handles the bearer link; review r7, I-1) | specified | F8a, F7a, S4d, F8la |
| F8c | Runners: the local lane (stores, apply, courses, registrar, config) | `implementer` | specified | F8e, F6a, F6b |
| F8g | Runners: the partial (`BeforeConfig`, then `NoRank`) and full slots, each with onboarding's `AppHandle` (held by the runner); a queue stop returns before the slot (§14.51) | `implementer` | specified | F8e, F8f, F8h, F8s |
| F8d | Runners: mail, Google, calendars, coursework | `implementer` | specified | F8a |
| F8m | `mail/live.rs`: in onboarding mode a closed backfill batch tells the queue, never starts a slot | `implementer` | specified, about ten lines, tested in F1 (review r5, I-4) | F8a |
| F8la | `lms_link.rs`: `capture_calendar_link_with`, `validate_only` and `capture_link_only`, the validate-only seam | `implementer`, reviewed by `contract-reviewer` | a bearer link stored under the wrong session reaches another account (review r6 I-2, r7 I-1) | F1, S4b merged |
| F8lb | `onboarding_capture_link` and `onboarding_paste_link` over F8la's seam; the link to its pending credential | `implementer`, reviewed by `contract-reviewer` | as F8la | F8la, F8b, F8c, F8g, F8d (last in `onboarding_jobs.rs`'s chain), F2d |
| F9 | The reveal gate, in its own `onboarding_gate.rs` | `implementer` | a pure function, tested in F1; its own file, so no chain (review r8's note) | F1 |
| F10 | Shell steps 1-2 in the ruled direction (5b); old wizard removed (static files only) | `console-ui` | `app/static`, the `console.js` queue | direction ruled (5b, 2026-10-23) |
| F11t | Failing static tests for the onboarding screens | `test-writer` | test first (review r1 minor) | F2b |
| F11a | The step frame and the rail | `console-ui` | `app/static` | F11t, F10 |
| F11e1 | Step 3's school choice ("isn't listed" included), sign-in and capture rules (second press, close on leaving); step 2's sign-in again | `console-ui` | `app/static` | F11a, F8b, F2g, F8lb (review r7 M1) |
| F11e2 | Step 3's proof with the removable course list; step 4, its `comp` state included | `console-ui` | `app/static` | F11e1, F8c, F8g, F2d, F2h, F7a (`by_course`) |
| F11b | Steps 5, 6 and 7; step 6 on today's separate consents (`google_connect` with each scope, `google_status`) | `console-ui` | `app/static` | F11a, F8d, F8lb (step 7's paste; review r7 M1). No longer O2 (T0, 2026-10-24: O2 has no branch) |
| F11f | Step 6's combined Gmail-and-Calendar ask and the forwarding fallback, over O2's cloud half | `console-ui` | `app/static`; O2's contract only | F11b, O2 merged; may follow wave B (§8) |
| F11c | Steps 8 and 10 | `console-ui` | `app/static` | F11a, F8d |
| F11d | The reveal and the main page's "still coming" line | `console-ui` | `app/static` | F11a, F9 |
| F12 | `scripts/wizard-check.py` rewritten | `test-writer` | the walk-through is the page's test | F11a-F11e |
| F13 | References, HANDOFF, O3's amendment drafts | `docs-keeper` | docs | F2a-F12 |
| F14 | Review | `reviewer` | before a push | F2a-F13 |
| F15 | Live proof, spec §8's measure | main session | live | F14, S10, Quinn's go |

S-tasks and F-tasks touch different files except `main.rs` (F4, F5, F5b, S6r: one branch each, in
merge order, recounted) and `scaffold.rs` (S2 adds a field; F3, F6a and F6b add functions; O4 merges
first). Every implementer and console-ui task is sized to edit about 80 lines of code or fewer in any
one file; a task that finds it needs more stops and asks for a split.

## T0. Branches and checks (main session)

*Run 2026-10-24 by the main session on `feat/o4-school-sign-in`, cut from `origin/main` `1d3c467`
with O1 merged. Results in §13 (2026-10-24) and §14.42.*

- [x] After O1 merges: `feat/o4-school-sign-in` and `feat/o3-long-onboarding`, each from `origin/main`.
  Commit this plan and the spec on the first, if `main` lacks them. *O4's branch is cut; the plan is
  on `main`. O3's branch is cut from `main` after O4 merges (A18).*
- [x] Record the state of: L6 (merged? then `main.rs` is free), L5 (merged? then O5 can follow
  directly), O0's `fix/current-term-courses` (needed by S-F's class count), O2 (needed before F11b's
  step 6), L8 (testers' "nothing to pay"), L1 (not started; it rebases on O4). *L6, L8, O0 (#63)
  and #19 are merged. **L5, O2 and L1 are not merged and have no branch**: O5 cannot follow
  directly (§8), and F11b builds step 6 on today's separate consents, the combined ask moving to
  F11f after O2 (§7, §8).*
- [x] Re-read every `file:line` this plan cites on the branch; record drift as dated lines in §13.
  *The code is byte-identical to `49b0b43`; three lines drifted by one, now corrected.*
- [x] Re-confirm P-3's five changes (§3) and S-F's read; a sixth finding goes to the planner.
  *The five hold. The sixth: onboarding mode leaves `Scheduler.mode_device` at its placeholder;
  F4 now primes it (§13, §14.42). S-F's code read is re-read live in T3.*
- [x] Confirm `PUT /account/sources` still refuses without an entitlement (Q-B2's premise).
  *Holds (`handler.ts:126`, `entitlement.ts:89`).*
- [x] Confirm that `sync`, `grades` and `judge`'s Gmail pull run correctly on a vault with no
  `config/ingest.yaml`, and that onboarding mode never reaches a reader of `campus.yaml` or
  `events.yaml` before `school.apply` (§4.3, §4.9). Any that does not goes to the planner before F8c.
  *`sync`, `judge` and Gmail hold, and no engine file reads `campus.yaml`. **`grades` does not**: it
  needs course notes, so `BeforeConfig` leaves it out (§4.3). The app's slot reads `campus.yaml`'s
  row at its start (`scheduler.rs:732`), a named skip before `school.apply`; the mid-slot case is
  in §4.3.*
- [x] Confirm how the scheduler bounds a running step (its timeout), which §4.10's wait relies on.
  *`CHILD_TIMEOUT`, 20 minutes, with `kill_tree` (`scheduler.rs:597-611`).*
- [x] For round 2's answers, name in §13: the hold a console note write takes against a running `sync`
  (§4.3's local lane); the record a `sync` leaves that says it reached the account (`account_settled`);
  whether `grades` reads course notes (§4.3); whether the account returns a stored feed's host to the
  device (§4.7's suggestion); that `std::fs::hard_link` refuses an existing target on the vault's
  volume (§4.9); and that `scheduler::spawn`'s first-run slot fires for a step-2 vault after a
  relaunch from onboarding (App mode, `device_ok`; §4.10). Any that does not hold goes to the planner.
  *Named in §13 (2026-10-24). Four did not hold as written and are amended: only `run_confirm` and
  `registrar::spawn` take the sync hold; an entitlement-refused sync is a skip, and `account_settled`
  also needs `pulled_to_end`; `grades` reads course notes; the account returns no host. `hard_link`
  and the first-run slot hold.*
- [x] For round 3's answers: that `Scheduler` is managed state in onboarding mode even though
  `scheduler::spawn` is not called (the stop flag lives on it). *Answered by review r4 from the code,
  so no longer T0's:* Sign out does **not** relaunch or end anything (`account.rs:710-729`; now
  `onboarding_sign_out`'s job, §4.10), and a skipped sync leaves **no** signal in its step name, only in
  `state/sync-status.json` (now read there, §4.3). *Holds: managed unconditionally in `run_console`
  (`main.rs:161`).*
- [x] For round 4's answers, as corrected by review r5 (I-2) and §13.1: **confirm no re-attach path
  exists** on the lane branch (today `attach_in` refuses a vault with `cloud.yaml` and the sign-in
  commands save only to the pending target), so F2g's is the only one; if a lane has added one since,
  it goes to the planner before F2g. *Answered by §13.1 from the code:* the three sign-in commands
  are on the console's list (`main.rs:194`); `switch_profile` has no inner function (F4b adds one);
  `cs.sync` carries `at`, `ok` and `skipped` (`sync.rs:2408-2417`). *Holds: no re-attach path.*
- [ ] **The ruled direction's files** (Quinn, 2026-10-23). `docs/design/2026-10-23-directions/` is
  in the main checkout (`C:\Users\danie\GitHub\knowlu\docs\design\2026-10-23-directions`), untracked
  there on 2026-10-23 and absent from this worktree. Before F10, F11a-F11e or S6b is dispatched,
  commit at least `README.md` and `5b-departures-board/` to `main` (docs only), or give each brief
  the main checkout's path, so the console-ui agents read the same files. *Not in T0's report of
  2026-10-24; still open, and due before F10, F11a-F11e or S6b is dispatched.*
- [x] For round 5's answers: re-run §13.1's reuse audit on the lane branches (each row's `file:line`,
  signature, target and callers) and record drift in §13; recount `run_slot_inner` and
  `run_slot_with`'s call sites (35 here, with `run_slot`'s); confirm `scheduler::run_slot` still has exactly the four
  callers §4.3 names, and that nothing else in `app/src` starts a slot. *All 54 rows hold on
  `feat/o4-school-sign-in`, the `SyncStatus` row as now corrected; 35 call sites; four callers;
  `main.rs:42`'s `--run-slot-once` is the only other slot starter, outside the console process.*

## T1, T2, T3. The spikes (main session)

As §3 says, in that order: T1 is S-A phase 1; T2 builds the throwaway instrument and runs S-A phase 2
and S-B; T3 runs S-E with S-F's live half and S-G in one session. Each records its result as dated
rows in §13 (counts, seconds, status codes, host kinds only) and its verdict. **A failed spike stops
the tasks that depend on it** (S-A, S-B: every S-task; S-G: S4a-S4d and F8b; S-F: F7a's `--preview`
fields) and goes to
Quinn with the finding. S-E's numbers go to Quinn with the copy check and to the directions' authors.

## S1. Failing tests: `school_session` (test-writer)

**Brief:** this section, §1, §2, §5's Q-B4, Q-B13, spec §2.4-§2.5. Tests in `app/tests/school_session.rs`
(new), on scratch folders under the temp directory, with minimal `pub` stubs:
1. `session_dir(data_dir)` is exactly `<data_dir>\school-session`; `is_session_dir` refuses a
   grandchild, a sibling, another profile's folder, a relative path and any `..` (the guard shape of
   `grades::is_session_dir` and `mail_session::is_session_dir`).
2. `forget` deletes the folder patiently (it survives one failed attempt while a file is held open,
   then goes), refuses any path its guard refuses, and reports a delete it could not finish.
3. Uses: `add_use(Grades)`, `add_use(SchoolMail)`, `add_use(Registrar)` persist in `school.json`;
   `clear_use` of the last use deletes the folder; clearing one of two keeps it; `forget` clears all
   and deletes. An unreadable `school.json` is an error, never "no uses" (the `mail.json` rule).
4. `profile_for(feature, row, data_dir)`: a curated row with `shared_sign_in` gives `school-session`
   for every feature; without it, each feature's own folder (today's); an uncurated school, the
   feature's own folder; and, per Q-B4, an existing install with no `school-session` yet keeps its
   old folder until the folder exists.
5. The lock: a second `try_take` while one holder is live is refused; dropping the holder frees it.
Acceptance: each test compiles and fails at run time; every other test passes; 0 warnings. Size: S.

## S2. `school_session.rs` and the campus field (implementer)

**Brief:** this section, S1's tests, spec §2.4.
- [ ] `app/src/school_session.rs`: the folder, its guard, `forget` (the patient wipe of `grades.rs`
  and `mail_session.rs`, 12 tries at 250 ms, with S-B's measured wait if longer), `school.json`
  (uses; one write path that reloads under a process-wide lock, `MailState::update`'s shape), the
  window lock (`Capturing`'s shape, `grades.rs:493`), `profile_for`.
- [ ] `scaffold::Curated` gains `shared_sign_in` (a bool, or a per-surface list if S-A found partial
  sharing). Every row is `false` except the founder's school's, set `true` **only if T2 recorded S-A
  as passed**, with a doc comment citing the spike's dated row (no school is named in the comment).
- [ ] `scaffold::Curated` gains `sign_in_hosts` (a list of host names, empty on every row but a shared
  one): the founder's school's row gets the hosts S-A phase 2 listed (§3), which the mail window's
  navigation allow list reads (Q-B14). A row with `shared_sign_in` and no `sign_in_hosts` is refused
  by a test over `CAMPUSES`.
- [ ] Module doc: what the folder is, who uses it, when it goes (Q9), that it is never synced, never
  in the vault and holds no password.
Acceptance: S1's tests pass; all else unedited and green. Size: S.

## S3. Failing tests: the windows on the shared folder (test-writer)

**Brief:** this section, spec §2.4's bullets, S-B's recorded result. Tests at the pure seams each
module already has (builders' argument functions, path choices), never a real window:
1. For each of grades, registrar, LMS capture and the school mailbox, at a shared school, the window
   is built on `school_session::session_dir`; at a non-shared school, on today's folder; the LMS
   capture is non-incognito only on the shared folder (incognito, temp folder, as today, otherwise).
2. Every window on the shared folder takes **one** arguments constant (`SCHOOL_BROWSER_ARGS`, equal to
   `MAIL_BROWSER_ARGS`, or as S-B and S-G recorded); no window on it builds without it. **No debug
   port, in any build** (review r1, I5): the constant contains no `remote-debugging` argument, and the
   argument function takes the debug port **as a parameter** (`Option<u16>`, read from
   `KNOWLU_MAIL_DEBUG_PORT` only at its one call site): for a window on the shared folder it returns
   the constant whatever port it is passed (the test passes `Some(9222)`), and only a mail window on
   its own `mail-session-<id>` folder adds the port. **No test sets or reads the environment
   variable** (review r2, M3: it is process-wide, races parallel tests, and `set_var` is `unsafe` in
   edition 2024).
3. **Cookies go only where they were read** (review r1, I4). Each reader reads cookies for its own
   host only (grades: the LMS host; registrar: the registrar host; LMS capture: the feed host, or the
   window's host only when that is the feed host, R-C1-41). **Every request a reader sends goes to the
   exact host whose cookies it read.** Here the **queried host** is the host of the URL the reader
   passes to the jar query (`cookies_for_url`'s argument), never a returned cookie's `Domain`
   attribute (review r2, M2): the query can return the identity provider's parent-domain cookies,
   whose `Domain` differs, and that is exactly why the request must go to the queried host. A test
   drives each reader through a fake jar and a fake HTTP agent that record `(queried host, request
   host)` pairs, and fails on any pair that differs or any queried host not on the reader's list;
   the fake jar returns a parent-domain cookie too, so a reader that keyed on `Domain` fails.
4. **Scripts and popups stay per window** (review r1, I4): only the mail window carries
   `INIT_SCRIPT`; no other window on the folder sets an initialization script or a new-window handler
   that opens anything. The mail window's popup filter (`popup_allowed`) still refuses every URL that
   is not the provider's, including the LMS and registrar hosts. On the shared folder its new
   navigation allow list (Q-B14, review r2 M1) allows the provider's hosts and the row's
   `sign_in_hosts` only, and refuses the LMS host, the registrar host, a syllabus host, another host
   under the school's domain and an unrelated host. Both are pure predicates, tested with those hosts.
   **The fallback** (review r3, M3): a refused navigation in the school mailbox's setup on the shared
   folder yields the decision "fall back to own folder" (a pure function over the refusal and the
   row), after which the mailbox's folder choice is `mail-session-<id>` and no `school_mail` use is
   recorded; it never yields "needs sign-in". No window label appears in
   `app/capabilities/default.json`.
5. **One window at a time on the folder:** a hidden read while another holds `school_session`'s lock
   answers `Busy` and opens nothing.
6. **A use is recorded** when a feature first reads through the shared folder, and cleared by that
   feature's own disconnect (grades' Forget becomes "Forget school sign-in" at a shared school: it
   calls `school_session::forget`).
7. **Spec §2.5's mail rows** (review r1, I3), through the seams `mail::commands::disconnect_with` and
   `keep_session_with` with fakes:
   - (a) Disconnecting the school mailbox at a shared school turns forwarding off **using the shared
     session first**, then clears the `school_mail` use. It never calls `mail_session::forget` on the
     shared folder (whose guard refuses it anyway). The folder stays while grades use it and goes when
     `school_mail` was the last use.
   - (b) The saved-sign-in switch off deletes every personal mailbox's `mail-session-<id>` and clears
     the school mailbox's `school_mail` use. The shared folder is deleted only when no use remains.
   - (c) At a non-shared school, both behave exactly as today (existing tests unedited).
8. **"Needs sign-in" after Forget:** with the folder gone, grades refresh, the registrar read and
   school-mail repair each answer their named "needs sign-in" outcome and open no hidden window.
Acceptance: as S1's. Size: M.

## S4a-S4d. The windows on the shared folder (implementer; contract-reviewer reviews each diff)

**Common brief:** this section, S3's tests, spec §2.4-§2.5, S-B's and S-G's records. Each window on
the shared folder chooses its folder through `school_session::profile_for`, builds with
`SCHOOL_BROWSER_ARGS` (plus any arguments S-G required), and holds `school_session`'s lock for its
life. Each task edits one module's window code, about 80 lines or fewer, and lists any existing test
it changes, with why. **Acceptance for each:** its S3 tests pass; every other test passes; 0 warnings;
`contract-reviewer` passes the diff (Q-B6).
- **S4a, `grades.rs`.** `open_visible`, `open_hidden` and the jar read on the shared folder; `forget`
  at a shared school calls `school_session::forget` (Forget school sign-in) and records the use on
  connect. S3 tests 1-3, 5, 6, 8 (grades). Size: S.
- **S4b, `lms_link.rs` and `registrar.rs`.** `open_window_at` builds non-incognito on the shared
  folder and keeps today's incognito temp folder otherwise. The wipe on `Destroyed` and the sweep
  stay for the incognito folder only, and the shared folder is refused by `lms_link::is_session_dir`.
  The registrar records its use. S3 tests 1-3, 5, 8 (registrar). Size: S to M.
- **S4c, mail.** `mail.rs` `open` takes the shared folder for the school mailbox at a shared school,
  with `SCHOOL_BROWSER_ARGS` and no debug port, and adds the navigation allow list (Q-B14) with its
  fall-back to the mailbox's own `mail-session-<id>` folder on a refused host (review r3, M3). In
  `mail/commands.rs`, `disconnect_with` and `keep_session_with` clear the `school_mail` use, per S3
  test 7. `mail_session.rs` is unchanged except where `profile_for` replaces its folder choice for the
  school mailbox. S3 tests 1-4, 7, 8 (mail). Size: M.
- **S4d, `school_session.rs`: onboarding's one visible sign-in.** One window on the shared folder at
  the LMS host. When it reaches a signed-in LMS (`grades::signed_in`'s test, `users/me` 200 for the
  LMS host only), it closes, and F8b queues the hidden reads. A window the student closes first is
  `signed-out`, never an error. Its test (in `app/tests/school_session.rs`) drives the decision with
  fakes of the window URL and `users/me`. Size: S.

## S5. Sign-out and deletion (contract-engineer, then contract-reviewer)

**Brief:** this section, spec §2.5, Q-B9, `account.rs`'s own doc comments for `sign_out_with` and
`delete_local_data`. Tests first in `app/tests/account.rs` (the file's `CREDMAN_LOCK`, generated ids,
`Drop` cleanup), then the code:
1. `sign_out_with`: under the same window hold it takes for the mail window, closes any window on the
   school folder, then forgets `school-session` and any `lms-session` (patient deletes through each
   module's guard), before the credential is deleted; a folder left on disk is reported in the
   envelope (`school_profiles_left`, as `mail_profiles_left` is) and retried at launch.
2. `delete_local_data`: the school folders go patiently before `remove_dir_all(data_dir)`, as the mail
   profiles do.
3. Order and envelope otherwise unchanged; every existing account test passes unedited.
Acceptance: the new tests pass; `contract-reviewer` passes the diff. Size: S.

## S6t, S6a, S6r, S6b. Settings: one school sign-in row

- **S6t (test-writer), first.** In `app/tests/school_session.rs`: `school_status` reports the uses,
  whether the folder exists and the school's label, and never a path, cookie or host. `school_forget`
  closes the windows on the folder (fake), forgets the folder and clears every use. In
  `app/tests/static_assets.rs`: the row's handler exists and calls `school_forget` only after a
  confirm that lists the uses. Red until S6a and S6b. Size: S.
- **S6a (implementer).** The two commands in `school_session.rs`, beside the module they serve, as
  thin wrappers over S2's functions and S4a's window close (review r1, I8). Size: S.
- **S6r (mechanical).** Both names into the console's `generate_handler!` list; recount; pin. Size: XS.
- **S6b (console-ui).** The row: "School sign-in: kept on this computer, used for <uses>. Forget.",
  with a confirm that lists what will need a sign-in again (spec §2.5), and "Connect" when the folder
  is absent at a shared school. At a shared school it replaces grades' "Forget Blackboard sign-in".
  The look and words are the chosen direction's; the uses come from `school_status`. Acceptance: S6t's
  static test passes; a fixture render shows the row. Size: S.

## S7. The privacy page (contract-engineer for the constant; reviewer; Quinn reads)

- [ ] `site/privacy.html`: the kept school sign-in, what uses it, that Forget, sign-out and deletion
  remove it, and that personal mailboxes keep their own (spec §2.7, §11). Drafted by `docs-keeper`
  from spec §2.4-§2.5; `reviewer` checks each sentence against S2-S5's code; **Quinn reads the page.**
- [ ] The telemetry sentence: the page reviewer checks whether it covers "which onboarding step was
  reached" (Q-B10). If Q-B10 is ruled yes and the sentence does not cover it, this page adds it, so
  the page names it before F7b's action exists.
- [ ] **Onboarding's on-device data** (review r3, M6), one sentence or two, for O3's behaviour, which
  ships in the same release (no tag between the merges, Q-B11), so the page and its version move
  once: during setup, a class-feed or calendar link not yet saved to the account waits in Windows
  Credential Manager on this computer, and the course names and class times found at the school step
  wait in a file in Knowlu's app data; both are deleted when setup finishes, at sign-out and when the
  account is deleted, and a setup left unfinished keeps them until then. `reviewer` checks the
  sentence against §4.5 and §4.11 when F2a-F2e land; if O3 slips to a later release, this sentence
  moves to F13 with its own version bump.
- [ ] `PRIVACY_VERSION` moves to the page's date (`account.rs`, `contract-engineer`), so L4's
  re-consent screen asks every existing account; `engine/tests/site.rs` pins the new sentences.
Acceptance: the privacy guard passes; Quinn's read recorded in §13. Size: S.

## S8, S9, S10. Docs, review, live proof

- **S8 (docs-keeper):** `docs/reference/app.md` (app data: `school-session`, `school.json`; the windows),
  `cloud/supabase/README.md` untouched, HANDOFF's O4 row; drafts of O4's amendments (§9 rows A1-A6)
  for Quinn's signature. Acceptance: eol check clean; no source in the diff.
- **S9 (reviewer; contract-reviewer for S4a-S4d, S5, S7):** the lane against this plan and spec §2;
  report `docs/reports/2026-10-2X-o4-school-sign-in-review.md`.
- **S10 (main session, Quinn's go):** a dev build against staging on a scratch profile at the founder's
  school, using today's wizard (O3 not yet merged), with no debug port set:
  - one sign-in on the shared folder from Settings' "Connect";
  - grades refresh, the registrar read and school mail setup each silent (count prompts: one);
  - disconnecting the school mailbox keeps the folder while grades use it;
  - Forget lists the uses and deletes the folder;
  - **each of** the next grades refresh, the registrar read and a school-mail repair then reads
    "needs sign-in" (review r1 minor);
  - after one more sign-in, sign-out removes the folder.
  Counts, status codes and host kinds only; the profile removed afterwards.

## F1. Failing tests: state, resume, queue order, gate (test-writer)

**Brief:** §4.3, §4.5, §4.6, §4.7, spec §1.5-§1.7. New files `app/tests/onboarding_state.rs` and
`app/tests/onboarding_jobs.rs`, with minimal `pub` stubs. No window, no network: the queue takes its
job runners as a trait so tests pass fakes.
1. **The two files:** round trip of every field of `onboarding.json` and `onboarding-answers.json`;
   neither holds a URL, cookie or login (a test scans each serialised text for `http`, `://`, `webcal`
   and the credential target prefix, with a captured feed link present in the run); `onboarding.json`
   holds no course name either; both are written beside `settings.json`; an unreadable answers file
   is an error, never "no answers" (§4.5).
2. **Resume:** an unfinished file reopens at its `step`; `running` jobs come back `pending`; an
   unreadable file resumes at the earliest step §4.7's checks do not prove (driven by fakes of each
   check), never as finished, **and never past step 3, because the school is never proven** (F2a
   review I-2, ruled in §14.49): with a session present, every rung of the ladder resumes at **3**
   with `school: SchoolOffer::Ask`, whatever the other checks prove (the restored, entitled,
   mailbox, Google, login and week rungs included); with no session it resumes at 2, as now. After
   the school is confirmed, the next step is the earliest one past 3 the checks do not prove, each
   proven step passed through as `already-done` (with every check proven, step 10); an absent file on a registered profile is a normal launch. **The
   school at step 3 on a restored install** (T0 review M5, §4.7): with fakes of a restore that
   brought course notes, (a) a new profile with no `onboarding.json` reaches step 3 with no school
   suggested and the school asked for, as on a fresh install, and the fake account is never asked
   for a school; (b) a file left by a quit after step 3 saved `school` offers that school as a
   confirm and assumes nothing (`school.apply` does not run until it is confirmed); (c) a file
   rewound by `on_sign_out` has no `school` and behaves as (a).
3. **Holds:** with the entitlement fake answering "not entitled", every vault-writing or gated job is
   `held` and never started; `school.capture` and `school.preview` run; after "entitled", the held
   jobs run in the order they were enqueued.
4. **Slot order:** whatever order the asks were answered in, the engine steps the fakes record are
   `sync`, `coursework`, `grades`, `ingest`, `judge`; `rank` appears only in `slot.first_full`, after
   step 10's confirm, and never before the first `ingest`. A `slot.partial` dequeued before
   `config.ingest` is `done` is handed `BeforeConfig`, and one after it `NoRank` (the fake records the
   scope it was given; review r5's audit, Q-B19).
5. **Serial:** two jobs never run at once (a fake that fails if entered twice).
6. **The gate** (pure): opens when every required item is done, failed or skipped; opens at exactly
   3:00 after `step10_confirmed_at` with the rest listed as still coming; never waits for a backfill;
   a failed item is listed with its reason; an unconnected source is not required; skips are listed
   with their cost keys.
7. **Skips:** account, payment, the school step and "Your week" refuse a skip; every other step takes
   one and records it.
8. **Config before its readers (§4.3, §4.9):** no fake `ingest`, `coursework` or calendar read starts
   before `config.ingest` is `done`; no `grades` (capture or engine step) before `school.apply` and
   `school.courses` are both `done`, so none in a `BeforeConfig` slot (T0); no `registrar.confirm` before
   `school.courses`; no `school.courses` before `account_settled`; no `config.ingest` before
   `school.courses` and both feed stores have settled.
9. **Quitting and finishing (§4.10, review r1 I2, review r2 I3 and M5):**
   - Finish later, close or Sign out during a running fake job stops new starts in both the queue and
     the local lane and waits only for that job's safe point, then saves; the next load shows it
     `done`, or `pending` if it was stopped.
   - `onboarding_finish` at the cap with a fake `slot.first_full` mid-way (its `ingest` running) waits
     for `ingest` to end and starts neither `judge` nor `rank`; the slot is `pending`, and
     `handover` names it. A fake `mail.canary` running at finish is abandoned at once.
   - `onboarding_finish` is refused while any local-lane job is not `done`.
   - On finish, every required item not `done` is written to `handover` with its clearing signal and
     `finished: true`.
   - Slot items are cleared by the first slot's end (a fake slot-end call); a mail item is **not**
     cleared by it, only by its own fake `mail.json` signal or a disconnect.
10. **`LaunchMode` (review r1 minor, for F4):** absent file gives normal; unfinished gives onboarding;
    finished, with or without `handover`, gives normal; unreadable gives onboarding, per test 2.
    **No session** (review r4, I-1): an unfinished file at any step on a profile whose session
    credential is absent (a fake) resumes at step 2's sign-in inside onboarding mode, never at the
    file's step; after the fake sign-in and a fake attach that succeeds it resumes at the file's step;
    a refused attach (test 21's mismatch) leaves the resume at step 2.
11. **The local lane (review r2, I4):** with a fake `mail.canary` holding the serial queue for its full
    3 minutes (a fake clock), leaving step 8 makes `config.ingest` run and finish without waiting for
    it; `onboarding_status` reports step 10's proposals as not ready while `config.ingest` is
    `pending` or `running` and ready once it is `done`; no lane job ever starts an engine child or a
    window (a fake that fails if one does).
12. **Answers survive a quit (review r2, I2):** run steps 3-8 through the fakes once without a quit,
    and once with a quit and reload at each of steps 4, 6 and 8 (after step 8's confirms, before
    leaving it); the text `config.ingest` hands F6b's builder is byte-equal in every run, and its
    `course_map` is never empty when step 3 found courses. **Removed courses** (F10's review,
    R-C1-51): with one captured course removed at step 3 (and a quit and reload after the removal),
    that course is in no `school.courses` create, not in `course_map`, and not in step 3's totals,
    which equal the sum of the saved `by_course` buckets whose code set is empty or names a kept
    course; a re-capture clears the mark. **Its items never become tasks** (2026-10-24, §14.45): the
    text `config.ingest` hands F6b carries the removed course's pairs under `course_exclude` and in
    no `course_map` entry, and a bucket that names only the removed code is out of the totals (the
    engine half, that ingest imports none of those items, is F6c's first test). **One source of
    pairs** (§14.46): after a step-3 label is edited, each course's `pairs` in the fake
    `--preview` call's `--courses` are byte-equal to that course's lines in `config.ingest`'s text
    (`course_map`, or `course_exclude` less any fragment a kept course shares), and a fragment a kept course also contributes is never in
    `course_exclude`. **Grades and class times on removal** (§14.47): with a fake preview whose
    single-course buckets carry `graded: true` for two courses, removing one of them makes step 3's
    `grades_courses` 1, and removing a `graded: false` course leaves it unchanged; with every
    bucket `graded: null`, `grades_courses` is `null`; and step 3's `class_times` is the same
    before and after any removal, and every registrar row still reaches `registrar.confirm`.
13. **The feed is never silently lost (review r2, I1):** with a fake account store, (a) a store that
    fails twice leaves the feed `failed` and `config.ingest`'s text carries the link, with
    `feed_kept_on_device` set; (b) a quit after payment and before `school.store`, then a reload,
    stores the link from the pending-credential fake with no school window opened (a fake that fails
    if one is); (c) a rejected paste is never sent to the store and its link is in the text; (d) a
    stored feed gives `ics_url: ''` and the pending-credential fake is deleted; (e) Back to another
    school before payment deletes it.
14. **No duplicate course note (review r2, I5):** with step 2's fake restore answering `ok: false`,
    `school.courses` stays `pending` ("waiting to reach your account") through payment; a fake `sync`
    status reporting the account reached, carrying two course notes, settles it; `school.courses`
    then creates notes only for the courses with none, and the answers file records the restored
    notes' slugs for those two.
15. **The scheduler's stop** (review r3 I-A and M1; moved here from F8f, review r3 M5), in
    `app/tests/scheduler.rs` through `run_slot_scoped`'s seams (the same `GradesSeam` and `MailSeam`
    `run_slot_with` takes, the file's `LOCALAPPDATA` guard):
    - the stop set **before** the slot: no stage starts (no session refresh, no grades capture, no
      mail pass, no engine child, no backup, no telemetry); the summary names the stages not run;
      `engine_ok` stays true; no run-log line is added;
    - the stop set from a fake grades capture that writes a bundle (so, mid-slot): the capture
      finishes, the mail pass and every engine child do not start, and **the bundle file is gone**
      when the slot returns (review r4, M1);
    - the stop is still set after the slot returns (nothing cleared it), and a second slot started
      on the same `Scheduler` also runs no stage;
    - with the stop never set, the slot runs exactly as today (existing tests unedited).
16. **Notes gate and `sync` (review r3, I-B):** at payment, with `account_settled` true, the fakes
    `school.courses` and `registrar.confirm` (lane) and `slot.partial`'s `sync` (queue) become ready
    together. Their runs never overlap (a fake that fails if both are inside at once); neither job is
    `failed`; `sync` is not skipped. With `account_settled` false, `sync` runs first and the note
    jobs after it. Separately (review r4, I-2: the fake is a **status record**, not a step name), a
    fake slot whose step reads `sync` exit 0 but whose `state/sync-status.json` carries
    `totals.skipped` ("another sync is running"; then "no session") leaves the gate's "first sync"
    not done and `account_settled` false; a status stamped before the slot's `RunSummary.started` (a
    stale run, which is also what a lock-held sync leaves, since it writes none) does the same; a
    fresh status with no skip and `ok` settles both **when the fake cursor has `pulled_to_end`**; a
    fresh status with no skip and `ok: false` makes "first sync" `failed` with its fix and leaves
    `account_settled` false (review r5, M2). **T0's cases (2026-10-24):** a fresh status with
    `ok: false` and `skipped: "no entitlement"` (the shape `record_gated_skip` writes) is "neither":
    "first sync" not done, not `failed`, `account_settled` false; a fresh `ok` status with
    `pulled_to_end` false marks "first sync" `done`, leaves `account_settled` false and, when the
    fake cursor moved, enqueues one more `BeforeConfig` `slot.partial` (Q-B20, as decided after the
    T0 review): a fresh `ok` run with the cursor unmoved stops the chain and fails `school.courses`
    with "Try again"; one skipped reading between moving runs enqueues one more and keeps the
    previous cursor as the comparison, two in a row stop the chain; a failed reading stops it at
    once; a fake cursor that moves on every run stops at exactly 50 runs; "Try again" while
    unsettled resets the count and enqueues exactly one `BeforeConfig` slot; the cursor is read,
    never written. A lane job retrying on a held
    `sync.lock` holds neither `vault_io` nor the lock between tries (a fake `vault_io` user gets it
    within one retry interval; review r4, M2). `registrar.confirm`'s fake is entered with
    `vault_io` free (it fails if held) and refuses with `week::SYNC_RUNNING` itself, the constant,
    never a typed copy of its text; the retry treats that, and only that, as "try again" (T0 review
    M2).
17. **Sign-out mid-onboarding (review r3 I-C, review r4 I-1):**
    - `onboarding_sign_out` at step 8, with a fake `mail.canary` waiting, `school.capture` done, a
      feed `validated` and its pending-credential fake set: the stop is set, the canary abandoned, the
      last save made, saves frozen, the fake sign-out called, the fake relaunch called with `--pick`,
      and the fake exit called, in that order;
    - `on_sign_out` (inside the fake sign-out) deletes the answers file and the pending fake and
      rewrites `onboarding.json` to **step 2** with every job `pending`, the feed states,
      `account_settled` and `school` cleared, skips kept;
    - **no save lands after the rewind**: a save the fake queue attempts after the freeze is refused,
      and the file read back is the rewound one;
    - signing in again (test 10's fake) and resuming lands at step 3, and no config job runs before
      step 3's capture saves new answers;
    - an `onboarding.json` at step 8 with **no** answers file (deleted out of band) resumes at step 3
      the same way, and a finished `onboarding.json` is unchanged by `on_sign_out`.
18. **`slot.partial` through `run_slot_scoped` (review r4 I-3, r5 I-3 and M3),** in
    `app/tests/scheduler.rs`, at the seam layer (fake `GradesSeam` and `MailSeam`, a fake engine exe
    that records its argv, as the file's tests do today):
    - `NoRank`, with a capture seam that returns no bundle: the summary carries a named
      `grades (skipped: …)` row and the runner log gains its line as it does for a full slot; `rank`
      is not in the recorded argv and is the row `rank (not yet: the first full slot ranks)`, with
      **no runner-log line** for it and no `state/today.md` written;
    - `BeforeConfig` on a cloud vault with no `ingest.yaml` (amended at T0, 2026-10-24): the
      recorded argv is `sync` only; the capture seam is **never called** (a fake that fails if it
      is) and no bundle is written; `coursework`, `grades`, `ingest`, `judge` and `rank` are each a
      `(not yet: …)` row with no runner-log line, `grades` once (its capture and step together);
    - with a capture seam present and a mail seam with a due mailbox, both run under `NoRank` exactly
      as under `Full` (the seams are the scope's, not dropped by it);
    - with a privacy update pending, the held steps are named exactly as in a full slot.
19. **A failed relaunch (review r4, M5):** with the fake relaunch returning an error,
    `onboarding_finish` and `onboarding_sign_out` each return `{ok: false, restart: true}` with the
    state they wrote kept (`finished: true`; the rewind), start no job, and the following
    `onboarding_quit` calls the fake exit.
20. **No other exit in onboarding mode (review r4, M6):** `switch_profile`'s inner refuses with its
    named line when the console is in onboarding mode and writes nothing (`quit_at` unchanged); in the
    normal console it behaves as today (existing tests unedited).
21. **The re-attach (review r5, I-2; F2g),** in `app/tests/onboarding_state.rs`, under a file-scoped
    `CREDMAN_LOCK`, generated ids and targets, `Drop` cleanup; the pending target is a **generated
    test target** passed in (as `attach_in`'s tests pass theirs), never the machine's
    `knowlu/pending/session`; `cloud.yaml`'s `api_base` is the test's own `account::api_base()` value,
    so no environment is changed and no request is made:
    - pending, sign-in answer and `cloud.yaml` naming one account: the session is at `cloud.yaml`'s
      target, the pending target is empty, the forget seam is not called;
    - pending naming another account: the profile's target is unchanged (empty, or the earlier
      session byte-equal), the forget seam is called once, and the answer is the named refusal with
      `wrong_account: true`;
    - the sign-in's answer naming an account the pending session does not (a stale pending session):
      the same refusal, nothing moved;
    - no pending session, or `cloud.yaml` unreadable: a named refusal, nothing moved, and with an
      unreadable `cloud.yaml` the forget seam is called (no session is left behind a refusal);
    - **the forget is local** (review r6, I-1): run once with the command's real seam over the
      generated pending target, a refusal leaves that credential gone and the profile's untouched;
      a source pin asserts F2g's code names neither `sign_out_with` nor `/logout`.
22. **The vault-target entitlement and checkout (review r5, I-1; F2h),** in the same file, with a
    `127.0.0.1:0` listener and `KNOWLU_API_BASE` pointed at it by the guard `app/tests/account.rs`
    already uses (`cloud_config` refuses a `cloud.yaml` naming another host than `api_base()`,
    `account.rs:870`):
    - with a session at `cloud.yaml`'s (generated) target, the listener sees that session's bearer
      on `GET /entitlement`, and on `POST /account/consent` then `POST /billing-checkout`, in that
      order, for checkout; a source pin asserts `onboarding_state.rs` names `PENDING_TARGET` only
      inside F2g's command wrapper;
    - an `active` or `trialing` reply saves the cache, so `scheduler::entitlement_state` reads
      `Entitled`; a refusal leaves the cache as it was and answers `ok: false` with the service's
      sentence; no session at the profile's target is a named error with no request made;
    - the checkout core returns the URL and opens nothing (the command adds `open_in_browser`).
23. **No slot outside the queue (review r5, I-4; F8m):** the pure decision `on_batch_closed(mode)`
    answers "tell the queue" in onboarding mode and "run a slot" in the normal console; with a fake
    queue, a closed batch in onboarding mode enqueues no job and starts no slot, and the next slot
    the queue runs is unchanged in order (the batch is pulled by its `judge`).
24. **The window's close (review r5, I-5; F4, F2b):** `onboarding_quit`'s inner, called from a
    spawned thread with a fake drain that waits on a channel, returns control to the caller at once
    (the close handler's own call does not block); a second call while the first drains answers
    `exiting: true` and starts no second drain; the fake exit is called once.
25. **Today's quit path (§13.1, Q-B18):** `onboarding_quit` and `onboarding_finish` call the fake
    flush after the drain and before the fake exit, with `quit_at` stamped; `onboarding_sign_out`
    calls no flush.
26. **Onboarding never stores a link at capture or paste (review r6, I-2; F8l):** with
    `KNOWLU_API_BASE` pointed at a `127.0.0.1:0` listener (the existing guard) and a fake fetch
    serving a valid ICS, `onboarding_paste_link` answers `ok: true, stored: false` with its note,
    writes the generated pending credential, and the listener receives **no request**; a link that
    fails validation answers `ok: false` and is kept as `rejected`; source pins (review r7 I-1,
    narrowed in review r8 M1 and M2):
    - `onboarding_jobs.rs` and `onboarding_state.rs` name none of `lms_link::finish(`,
      `capture_calendar_link(` and `paste_calendar_link(`, so F8b's `school.capture` cannot reach the
      storing capture either;
    - `store_source(` appears in them only inside F8c's store runner, whose call names
      `session_credential_target`, and **never beside `PENDING_TARGET`**; `PENDING_TARGET` appears
      in `onboarding_jobs.rs` nowhere, and in `onboarding_state.rs` only inside F2g;
    - over `lms_link.rs` (review r8, M3): `capture_link_only`'s body names `validate_only` and does
      not pass `finish` to `capture_calendar_link_with`;
    - the console's handler list holds neither `capture_calendar_link` nor `paste_calendar_link`
      (F5's pin).
27. **Onboarding mode primes the scheduler's cached mode (T0, 2026-10-24; F4),** in
    `app/tests/scheduler.rs`: a `Scheduler::default()` reports `(Script, true)` through
    `commands::attach_scheduler` (today's placeholder, pinned so the test fails if it changes
    unnoticed); after F4's priming call on a step-2 vault (App mode, this device), the envelope's
    `topline.scheduler` reads `mode: "app"`, `device_ok: true`; and a source pin over `main.rs`
    asserts the onboarding arm calls `scheduler::prime_mode_device` (F8s) before the window is built,
    and calls `scheduler::spawn_update_check` exactly once, and names neither `install_staged` nor
    `hold_for_install`; a second pin over `scheduler.rs` asserts the housekeeping tick calls
    `spawn_update_check` too, and that `check_and_stage_blocking` appears only inside it (T0 review
    M3). Also in test 6 (the
    gate): a `grades (not yet: …)` row never marks "first grades", and a `grades (skipped: …)` row
    leaves it pending.
28. **First grades from the slot's rows (F9 review r1, §14.50; F8g)**, in `app/tests/onboarding_jobs.rs`
    with fake slot summaries: a bare `grades` row at exit 1 makes `first.grades`
    `Failed { reason, fix }` equal to the two constants, and F9's output lists it with that fix and
    does not count it as done; a later fake slot with `grades` at exit 0 makes it `Done`; a
    `grades (skipped: …)` or `grades (not yet: …)` row leaves the state unchanged; "Try again" on
    it enqueues exactly one slot. **`onboarding_status` never hands F9 an empty list it did not
    read** (F9 review r1, the implementer's ruling 4; F2b): with the jobs read failing (an
    unreadable queue state, through a fake), `onboarding_status` answers `{ok: false, error}` and
    no gate output at all, never `{ok: true}` with an empty item list, which would open the gate.

Tests 29-32 (§14.51) came after F1 merged, so **each owner writes its test first, in its own task**,
failing before the code and passing after; the numbering keeps one list.

29. **One call sets both stops (F8f review I-1; F2b, F8g)**, in `app/tests/onboarding_jobs.rs`: with a
    `Scheduler::default()` and a queue over a fake runner whose `slot.first_full` waits on a gate and
    then loops "stage by stage" checking `sch.stop` as `run_slot_scoped` does, F2b's
    `stop_both(&sch, &queue)` (the one function its live `ExitSeams::request_stop` calls) sets
    `sch.stop` and the queue's stop; the fake slot starts no stage after the call and returns
    `Stopped`, so the job reads `pending`. A source pin over `onboarding_state.rs`: the live seam's
    `request_stop` calls `stop_both` and nothing else calls `Queue::request_stop` there. F8g's half:
    the real slot runner, handed a queue stop already set, returns `Stopped` without calling
    `run_slot_inner_scoped` (a fake slot function counting calls).
30. **The drain ends (F8f review I-2; F8f2)**, in `onboarding_jobs.rs`'s tests: with
    `drain_within(limit)` at 200 ms, a fake `school.capture` that never heeds the stop is abandoned
    at the limit (the drain returns within the limit plus a margin; the job reads `pending`; its late
    `Done` changes nothing, as the canary's test asserts today); a fake `slot.partial` that ignores
    the stop for 600 ms and a fake local-lane `school.apply` that runs 600 ms are both waited for
    past the limit, never abandoned; `DRAIN_LIMIT` is 45 seconds; and the progress answer names the
    running job and `slot: true` for a slot.
31. **A job that crashes twice is failed (F8a review r2 I-2; F8a2)**, in `onboarding_jobs.rs`'s tests,
    with no panic needed: an `onboarding.json` holding `school.capture` `running` with no crash count
    is started, and the job reads `pending` with a count of 1 (and its file saved); saved `running`
    again with that count and started, it reads `failed` with `job_crashed` and does not run (the fake
    runner records no call); a job that then ends normally reads a count of 0; Try again on a
    `job_crashed` job sets the count to 1, so the next crash fails it at once; a file written before
    the field (no key) loads at version 1 as a count of 0, and a count of 0 is not written, so such a
    file's bytes are unchanged by a load and save.
32. **The normal console clears the slot's hand-over (F8s follow-up; F8s2)**, in
    `app/tests/scheduler.rs`, over the fake engine the existing slot tests use: a finished
    `onboarding.json` with one slot item and one mail item, after a slot that runs, keeps only the
    mail item and is saved; byte-identical after a slot refused in script mode, a slot refused on
    another device and a slot refused because one is already running; with no `onboarding.json`,
    none exists after the slot; an unfinished or unreadable `onboarding.json` is byte-identical
    after. The mail half over the pure `mail_signals(rows, &mail_state)`: a row `confirmed` or
    `active` gives `CanaryVerified`; a row `revoked`, or the address in `turned_off`, gives
    `Disconnected`; a `last_reconcile` entry gives `FirstPassDelivered`; with a source pin that the
    slot's mail stage calls it beside its address list.
Tests 21, 22 and 26 touch the real Credential Manager, so the test-writer's sign-off rule for such tests
applies (the controller gives it at dispatch); every other test here uses fakes.
Acceptance: as S1's. Size: M (test files; the cap is for implementer and console-ui tasks).

## F2a, F2d, F2e, F2b, F2c. `onboarding_state.rs` and its deletes

Split so that each edits about 80 lines or fewer of one file (review r2 I6, review r3 M4).
- **F2a (implementer), `onboarding.json`.** F1's tests 1 (its `onboarding.json` half), 2, 7 and 10;
  §4.5, §4.7. The file's struct, load and save (temp file and rename: app data, never a vault file),
  resume derivation from §4.7's checks, the unreadable-file rule, the no-session rule (resume at step
  2's sign-in whatever the file's step; review r4, I-1), skip rules and `LaunchMode`. **The school
  is never a rung** (§14.49): `earliest_unproven` stops at 3 whenever the state it resumes has no
  `school`, which an unreadable file's rewrite never has, so an unreadable file resumes at 2 (no
  session) or 3; the steps past 3 are then derived after the school's confirm, proven ones passed
  through as `already-done`. With a session present, a file step of 2 or below resumes at 3 (F2a
  review I-1, the controller's ruling). Size: S (about
  80 lines; if the no-session rule pushes past, it moves to F2d).
- **F2d (implementer), the answers file and the pending names.** F1's test 1 (its answers half) and
  test 12's save path; §4.5, §4.11. The answers struct, its load and save, the rule that an absent or
  unreadable answers file past step 3 resumes the step it served, and the two pending-credential names
  (constants), written, read and deleted through `credentials::write`, `exists` and `delete`,
  unedited. The course row carries `removed` (F10's review, R-C1-51), set by step 3's Remove through
  `onboarding_answer` and cleared by a re-capture, and the per-course counts `school.preview`
  saves; a `kept_courses()` accessor is the only way `school.courses`, `config.ingest` and
  `onboarding_status`'s step-3 totals read the list. Size: S (about 65 lines).
- **F2e (implementer), `on_sign_out(profile_dir, profile_id)`.** F1's test 17; §4.5's rewind. Deletes
  the answers file and both pending credentials where they exist, then, for an unfinished
  `onboarding.json`, rewrites it at **step 2** with every job `pending` and the feed states,
  `account_settled` and `school` cleared (`school`: T0 review M5, §4.7); a finished file is left alone; every failure is returned, never
  panicked, so the sign-out goes on. Size: S (about 35 lines).
- **F2b (implementer), the commands.** F1's test 9's finish bullets and tests 12, 17's command half
  and 19; §4.5, §4.10. The `onboarding_status`, `onboarding_answer` (saves each answer to the answers
  file before enqueuing its jobs), `onboarding_skip` and `onboarding_retry` commands (registered in
  F5), each a thin wrapper. **`onboarding_status` fails closed** (F9 review r1; §14.50; F1 test
  28's second half): when the jobs cannot be read, it answers `{ok: false, error}` with no gate
  output, never an empty item list, because F9 opens the gate on an empty list; and the three exits of §4.10's table, each an inner function over seams
  (drain, sign-out, relaunch, exit) so F1 drives them with fakes, and a `#[tauri::command]` wrapper
  passing the real ones:
  - `onboarding_quit`: stop, drain, save, stamp `quit_at`, `state::quit_flush` (10 s) exiting in its
    `then` (§4.10, Q-B18);
  - `onboarding_sign_out`: stop, drain, save, freeze saves (F8f's flag on the queue's saver),
    `account::sign_out(app, Some(profile_id))`, `onboarding::relaunch(["--pick"])`, exit; no flush;
  - `onboarding_finish`: refuse while the gate is closed or a lane job is not `done`; stop, drain,
    write `finished` and `handover`, delete the answers file, autostart per settings,
    `onboarding::relaunch_with`, then the flush as `switch_profile` orders it, exiting in its `then`.
  All three are `#[tauri::command(async)]`, and each is guarded by one `AtomicBool`: a second exit
  while one drains answers `{ok: true, exiting: true}` and does nothing (F1 test 24). The inner of
  `onboarding_quit` is also what F4's close handler runs on its own thread. A relaunch error returns
  `{ok: false, restart: true}` and exits nothing until `onboarding_quit` (§4.10, review r4 M5). Size: S
  (about 80 lines; if past, the three exits move to their own piece, F2f, the same agent).
  **§14.51's additions** (F1 test 29, which F2b writes first; about 15 more lines, so the F2f split
  above is likely):
  - **One call sets both stops.** `pub fn stop_both(sch: &Scheduler, queue: &Queue)` sets
    `sch.stop`, then `queue.request_stop()`; the live `ExitSeams::request_stop` is that call and
    nothing else, so a slot in flight (which checks `sch.stop`) and the queue (which checks its own)
    both see it.
  - **The drain never runs on the main thread.** The live `drain` seam is reached only from the three
    `#[tauri::command(async)]` wrappers and F4's spawned close thread; no sync command or event
    handler calls it (F1 test 24 gains a pin: all three wrappers carry `async`).
  - **The progress answer** for the page's words (§4.10): while an exit drains, `onboarding_status`
    answers `exiting: {job, slot, since}` from the queue's snapshot (F8f2), so the page picks its
    line; the page keeps no limit of its own.
  - **Step 3 over an unreadable answers file** (F2d review r1, I-2): step 3's capture writes a fresh
    answers file from what it captured, replacing the unreadable one, never merging into it and never
    refusing. F1 test 12 gains the case (an unreadable answers file, then a step-3 answer, gives a
    readable file holding only the new capture). F8b's `school.capture` writes through the same F2d
    path, so it inherits the rule.
- **F2c (contract-engineer, then contract-reviewer; after S5 has merged).** Tests first in
  `app/tests/account.rs` (its `CREDMAN_LOCK`, generated ids, `Drop` cleanup), then `account.rs`, kept
  to two small edits (the reviewer's suggestions, reviews r3 and r4 M3):
  - **`pub fn sign_out_clean(dir: &Path, id: &str, api_base: &str)`** (new), holding what
    `sign_out`'s `clean` closure does today (the mail `clean_unfinished` with
    `sign_out_mail_client(api_base, …)`) plus one call to `onboarding_state::on_sign_out(dir, id)`,
    whose failure never stops the sign-out, as the mail clean's does not; `sign_out`'s closure becomes
    `|dir| sign_out_clean(dir, &id, &api_base())`. **The base is a parameter** (review r5, M5), as
    `sign_out_mail_client`'s already is, so the tests point it at their own `127.0.0.1:0` listener and
    no clean can reach the real service even with a session credential present. The tests call
    `sign_out_clean` itself, so they reach the edit (a test that called `sign_out_with` with its own
    closure would pass with the wiring missing);
  - `delete_local_data`'s credential loop (`for source in ["zybooks", "vhl"]`) gains the two pending
    names. The answers file needs no line there: `remove_dir_all(data_dir)` already removes it.
  Tests: `sign_out_clean` on a profile folder with both pending credentials set (generated ids)
  removes them and rewinds a step-8 `onboarding.json` to step 2; with a session credential present
  and the base at a local listener, the clean's requests reach only that listener; with no session
  credential the mail clean is skipped and the rest still runs; a deletion removes both credentials.
  Every existing account test passes unedited. Size: XS (about 20 lines).
- **F2g (implementer, reviewed by contract-reviewer), `onboarding_attach_session`** (review r5, I-2).
  F1's test 21. In `onboarding_state.rs`, over public `account.rs` functions only:
  - an inner `attach_session_in(vault, pending_target, expected_account_id, forget_pending)`: load the
    pending session (`account::load_session`), read `account::cloud_config(vault)`, and require the
    three account ids to agree (the pending session's, the sign-in command's answer, `cloud.yaml`'s);
  - on agreement, `account::move_session(pending_target, &cfg.session_credential_target)` (the target
    the vault names, which every reader uses), then `account::refresh_entitlement` best effort, as
    `attach_account` does after its attach;
  - on any disagreement, an unreadable `cloud.yaml` or a failed move, call `forget_pending` and
    return the named refusal ("this setup belongs to another Knowlu account: sign in with that one,
    or choose another profile") with `wrong_account` where it applies. No email or id enters the
    sentence;
  - the command `onboarding_attach_session(cs, account_id)` passes `account::PENDING_TARGET` and a
    `forget_pending` that is **a local delete only**, `credentials::delete(account::PENDING_TARGET)`
    (review r6, I-1). `credentials::delete` returns `Err` when no credential is there
    (`CredDeleteW`'s not-found, `credentials.rs:60-65`); **that result is ignored**, so test 21's
    no-pending case still answers the named refusal, never a delete error (review r7, M4). Any other
    delete error is logged by name, and the refusal still stands. **Never `sign_out_with`, and no revoke of any kind:** its
    `/logout` carries no `scope` (`account.rs:776-781`), which GoTrue reads as `global`, revoking
    every session the account holds. On a shared PC the wrong account is often a sibling's, whose
    finished profile (and other desktops) would lose their session; on an unreadable `cloud.yaml` it
    would be the student's own. The token still leaves this machine with the credential. F1 test 21
    drives the real local delete over its generated target. Size: S (about 35 lines).
- **F2h (implementer, reviewed by contract-reviewer), the vault-target entitlement and checkout**
  (review r5, I-1). F1's test 22. In `onboarding_state.rs`:
  - `onboarding_entitlement(cs)`: `account::refresh_entitlement(&cs.vault, &cs.data_dir)`, answered
    as `entitlement_now`'s envelope (`status`, `plan`, `current_period_end`), and, on `Entitled`, a
    wake for F8a's hold;
  - `onboarding_checkout(cs, plan)`: the `open_portal` shape (`account.rs:1626-1636`) with
    `checkout_url_at` in place of the portal POST: `cloud_config`, `auth_base`,
    `valid_access_token_at` on `cloud.yaml`'s target, `checkout_url_at` (which retries the consent
    first), then `open_in_browser`. The URL-returning half is its own function so the test opens
    nothing.
  No `account.rs` edit: every function used is public (`now_unix` is not, so the clock is
  `jiff::Timestamp::now().as_second()`, as `google_connect_url` reads it). Size: S (about 35 lines).

## F3. Step 2: the vault with defaults (test-writer, then implementer)

**Brief:** §4.2 step 2, §4.9's first row, spec §1.9 A, P-3. Tests in `app/tests/onboarding.rs`:
1. A new vault from the defaults: name "Knowlu" (a second profile "Knowlu 2", then 3; the existing
   `dest_for` refusals hold), Windows' zone, slots `12:00` and `18:00`, `autostart: true` in settings,
   `config/cloud.yaml` naming the profile's session target, the session moved off the pending target
   (today's order in `create_vault_in`), and an unfinished `onboarding.json` at step 3. **And
   `config/runners.yaml`'s `local` entry says `scheduler: app` with this device** (`runners_yaml`,
   `scaffold.rs:653-667`), so `scheduler::mode` and `device_ok` pass: `run_slot_with` refuses every
   onboarding slot otherwise (`scheduler.rs:775-780`; §13.1).
2. **No `config/campus.yaml`, `config/events.yaml` or `config/ingest.yaml` exists** after step 2, and
   the step-2 path never calls `scaffold::campus_yaml` (review r1 minor: an unknown preset fails
   `build_into`). There is no "no school yet" preset: the files are absent until §4.9 creates them.
3. A restore from the account's copy (C3′) still restores through the same call, writes none of those
   three files, and `onboarding.json` is written at the step §4.7 derives.
4. `launch_state`'s profiles carry `setup_unfinished` from each profile's `onboarding.json`.
5. **`account_settled`** (review r2, I5): true after a restore that reached the account, empty or not
   (a local listener on `127.0.0.1:0` serving the account's answer); false after a restore whose
   account could not be reached (a port bound then closed, so the connection is refused), with the
   vault kept, as today.
6. **The name is Rust's** (F10's review, 2026-10-24). Step 2 asks no name: Rust picks "Knowlu",
   then "Knowlu 2", "Knowlu 3", stepping past every name that is **either** a registered profile
   **or** a folder already on disk at the vault's destination (a leftover folder with no profile
   included), so `dest_for`'s "already exists" refusal is never reached on the step-2 path. Cases:
   a registered "Knowlu" gives "Knowlu 2"; an unregistered `Knowlu` folder on disk gives
   "Knowlu 2"; both "Knowlu" and a "Knowlu 2" folder give "Knowlu 3"; no folder is ever written
   into or renamed.
7. **The old flow's errors are rewritten** for the new one: `onboarding.rs`'s "press Finish again"
   and "pick another name" sentences have no step to point to (step 2 has no Finish and no name
   field). On the step-2 path a failure that today says either reads as a named error with step 2's
   own retry ("Try again"), and a test asserts neither old sentence is reachable from it. The adopt
   and restore-from-backup paths keep their own wording.
Then the code in `onboarding.rs` and `scaffold.rs`: a step-2 variant of `build_into` that omits the
three files, with `onboarding.json` written last, after the rename; the name picker (test 6); the
rewritten errors (test 7). **F10's interim page loop**, which steps the name on the page until the
create succeeds, is removed in the same task (its static test with it), since Rust now picks the
name; F3's report lists the removal. This makes F3 touch `app/static` once, a deletion of F10's
loop only. Today's `create_vault_in` stays for
the adopt and restore-from-backup paths. Size: S (about 60 lines in `onboarding.rs`, about 30 in
`scaffold.rs`; review r3, M4), plus about 25 in `onboarding.rs` for the name and the errors (F10's
review); past 80 in `onboarding.rs`, the name picker moves to its own piece (F3b), after F3.

## F4. Onboarding mode (implementer)

**Brief:** P-3's five changes (§3), §4.1's window rule. In `run_console`: read `onboarding.json`
before the builder; in onboarding mode skip `tray::build`, the autostart calls and
`scheduler::spawn`, keep `mail::live::on_start`, keep `manage(Scheduler)`, `manage(Updates)` and
`manage(LmsSession)` (the commands and the stop flag read them), manage F8a's queue (which F8m's
`on_closed` looks for), make the main window's close end the process (state saved, after §4.10's
drain), and maximise the window. The mode comes from F2a's `LaunchMode`, whose tests F1 wrote first
(test 10). **The close never waits on the event loop** (review r5, I-5): `on_window_event` runs on
the main thread, and the drain waits for stages (the grades capture, the mail pass, the school reads)
that need that loop to open and close their windows, for up to an engine child's 20 minutes. So, for
`main` in onboarding mode, `CloseRequested` calls `api.prevent_close()`, spawns a thread running
`onboarding_quit`'s inner (F2b, which shows "finishing <job>…" and exits the process itself), and
returns at once; a second close meets F2b's guard. The normal console's hide-to-tray arm and the
`lms_link` window's `Destroyed` arm are unchanged. Size: S (about 40 lines in `main.rs`; about 45
with the two calls below).

**Two managed values the guards need** (§14.51). F4b's `switch_profile` refusal reads `LaunchMode`
from managed state and F8m's `on_closed` looks for the `Queue`; both are inert until F4 manages
them, so the onboarding arm calls `.manage(LaunchMode::Onboarding)` and `.manage(queue)` (the normal
arm manages `LaunchMode::Normal` and no queue). F1 test 27's `main.rs` pin gains both. **An
unreadable `onboarding.json` (T0 review M5) is rewritten by the queue's start**: F4 passes the
state F2a derived for it (§14.49: no `school`, so step 2 or 3) to `Queue::start`, whose first
persist writes it; F4 writes no file of its own.

**What skipping `spawn` also skips** (T0, 2026-10-24; F1 test 27). `scheduler::spawn` is where
`Scheduler.mode_device` is first primed (`scheduler.rs:1154`) and where the housekeeping thread that
re-primes it every minute starts (`:1229`, `:1255`). Without it the cache keeps its `Default`
placeholder `(Script, true)` (`:146`), and `commands::attach_scheduler` puts that into every
envelope's `topline.scheduler` (`commands.rs:59-71`). So the onboarding arm calls
`scheduler::prime_mode_device(&cs, &sch)` (F8s: the one line `spawn` runs at `:1154`, moved into a
`pub fn` that `spawn` then calls, no behaviour change) once, after managing `Scheduler` and before
building the window. A step-2 vault is App mode on this device from birth (F3 test 1), so one
priming holds for the whole of onboarding. **The rest is benign, and F4 adds nothing for it:**
`spawn`'s `quit_at` take-and-clear and `startup_missed` (`:1137-1149`) run at the first normal launch
after the reveal, when `quit_at` is the finish's own stamp (Q-B18), so no slot reads as missed; the
housekeeping thread's debounced backup, log prune, entitlement refresh (`:1269`) and update check wait
for that launch too, and each onboarding slot runs its own backup and entitlement refresh.

**One update check at an onboarding-mode launch** (T0 review M3, the controller's call). Today the
check runs only from the housekeeping thread, at its first tick and daily (`scheduler.rs:1284-1310`),
and from Check now (`commands.rs:729`), so a student who stays in onboarding would never see a new
version. `main.rs` is the binary over the `knowlu` library and `commands::updates_dir` is
`pub(crate)` (`commands.rs:702`), so the tick cannot be copied into `main.rs`, and should not be. So
F8s moves the tick's spawned body (`scheduler.rs:1285-1310`: `try_state::<updates::Updates>()`,
`commands::updates_dir()`, `updates::check_and_stage_blocking`, the recording, the single-flight
silence) into `pub fn scheduler::spawn_update_check(app: &AppHandle)`, which the housekeeping tick
then calls (no behaviour change), and the onboarding arm calls it **once**, after managing
`Updates`. `updates.rs` (contract list) is not edited. It never blocks the window and never installs: onboarding has no install path
(`install_update` and `hold_for_install` stay uncalled, F11t), so a staged update is offered by the
normal console, as today. *Open for the controller:* the check stages but cannot install, so a student
who never finishes onboarding still runs the old version; installing from onboarding would need
`hold_for_install` and a drain like §4.10's, which this plan does not add. F1 test 27 gains a
source pin: the onboarding arm calls `scheduler::spawn_update_check` once. Size, with the priming:
S (about 45 lines in `main.rs`).

**F4b (implementer), `switch_profile` refuses in onboarding mode** (review r4, M6). F1's test 20. In
`app/src/commands.rs`: if `switch_profile` has no inner function, add one taking the launch mode
(the wrapper reads it from managed state), and in onboarding mode return the named refusal "finish
or leave setup first" before anything is written. `delete_my_data` (in `account.rs`, contract list)
is not edited: no onboarding screen can call it (F11t), and Settings is unreachable in onboarding
mode. Size: XS (about 15 lines).

## F5. Command lists (mechanical)

Into the console's `generate_handler!` list: `onboarding::campus_search`,
`onboarding::store_credentials` (**not** `timezone_for_state`: no page calls it and Q-B12 retired
its suggestion; F5b, §14.48),
`onboarding::discover_coursework`, `lms_link::open_lms_window`, `capture_courses`,
`close_lms_window`, F8l's `onboarding_capture_link` and `onboarding_paste_link` (**never**
`lms_link::capture_calendar_link` or `paste_calendar_link`, which store under the pending session;
review r6, I-2), and F2b's seven commands
(`onboarding_status`, `onboarding_answer`, `onboarding_skip`, `onboarding_retry`, `onboarding_finish`,
`onboarding_quit`, `onboarding_sign_out`; review r4, I-1), and F2g's and F2h's three
(`onboarding_attach_session`, `onboarding_entitlement`, `onboarding_checkout`; review r5, I-1 and
I-2). The step-2 sign-in commands (`account::google_sign_in`, `send_magic_link`,
`verify_email_code`) are already on the console's list (`main.rs:194`; §13.1), so none is added.
Recount both lists by script and pin the new names in the test that pins the lists. Size: XS.

**F5b (mechanical, after F10).** In the shell's list, remove only the commands F10 left with no
caller in the shell's page (each named in F10's report); recount and pin. F10 itself never edits
`main.rs` (review r1, I8). **Which go and which move** (F10's review, 2026-10-24):
- **Moved, never deleted:** `campus_search`, `store_credentials`, `discover_coursework`,
  `open_lms_window` and `close_lms_window`. Steps 3 and 8 call them in onboarding mode, so they are
  on the console's list by F5 (F5b runs after F5 and checks that they are) and only then leave the
  shell's list. Their functions and tests stay.
- **May go from the shell's list:** `google_connect_url`, `google_connected`,
  `paste_calendar_link` and `capture_calendar_link`, the shell's pending-target commands, which no
  onboarding screen may call (F11t). Their functions stay unless a later task removes them with
  their tests: `capture_calendar_link_with` keeps today's `finish` arm (F8la).
- **Retired from the shell's list, never moved** (F10 review r2, Minor 1; §14.48):
  - `account::open_external` (shell list, `main.rs:107`): its one caller was the old wizard's
    Google button; every Google consent now opens from Rust (`google_sign_in`, `google_connect`),
    and the console list must still not name it (`app/tests/handler_lists.rs:50-51`). The function
    stays in `account.rs` (contract list, not a `mechanical` edit); its removal is a later
    contract-engineer cleanup, out of wave B.
  - `onboarding::retarget_credentials` (shell list, `main.rs:107`): it existed because the old
    wizard stored the coursework logins before the vault's name was final (R-P4a-23). Now the vault
    and its name exist from step 2 (F3) and step 8 stores the logins against that vault, so no
    caller remains. Off the list; the function and its test (`app/tests/onboarding.rs:417-441`)
    stay until F13's follow-up list names their deletion.
  - `onboarding::timezone_for_state`: no page has ever called it, and Q-B12 retired R-OB-4's
    timezone suggestion (the zone is Windows'). It joins neither list (F5 no longer adds it) and
    leaves the shell's; the function and its tests stay on the same follow-up.
  F5b's pin asserts all three are on neither list, and `static_assets.rs:2994-2995`'s command list
  is updated to match.
Size: XS.

## F6a, F6b, F6c. The answers into the vault, each file once (test-writer, then implementer)

**Brief:** §4.9, §4.2 steps 3-8, Q-B1 as recommended (no exception; if Quinn rules otherwise, these
two tasks go back to the planner). Tests in `app/tests/onboarding.rs`, then the code in `scaffold.rs`:
- **F6a, `school.apply` and `school.courses`.** One create-new helper (§4.9: temp file in `config/`,
  then `std::fs::hard_link`, never `rename`, falling back at run time to `MoveFileExW` without
  `MOVEFILE_REPLACE_EXISTING` when the hard link fails for any reason but `AlreadyExists`).
  `school.apply` creates `campus.yaml` and `events.yaml`
  through it; `school.courses` creates the course notes through `write` as the student (journal
  first) and records each course's slug in the answers file. Tests:
  - each file's bytes equal what today's `build_into` writes for the same school (the same answers
    sent through both paths, compared byte for byte); an uncurated school gets events preset `none`;
    **a school typed as "isn't listed"** (R-OB-4, F10's review: a name, no unitid) gets the bytes
    `campus_config_yaml` writes for that choice today (`curated: false`) and preset `none`;
  - `school.courses` creates notes for `kept_courses()` only; a removed row gets none (R-C1-51);
  - a file already present with other bytes is kept, unchanged, and the job reports "kept your
    existing settings"; present with equal bytes, the job is simply `done` (a resume);
  - **a file created between the existence check and the put-in-place** (the test creates it after
    the temp file is written, through a seam) is kept, and the helper reports it existing, never
    replacing it;
  - a failure part-way leaves no partial file at the final name, and a stray temp file is removed on
    the next run;
  - **the run-time fallback** (review r3, M2): with the hard-link step forced to fail "not supported"
    (a seam), the file is put in place by the fallback with the same bytes; with a file already at
    the final name, the fallback also refuses and the file is kept; with both forced to fail, the job
    is `failed` with a fix that names the volume;
  - **restored notes** (review r2, I5): a course note already present (a C3′ restore) is not touched,
    no second one is made for it (matched by path, `source_uid` or code), and the answers file's slug
    for that course is the restored note's own slug.
  Size: S (about 75 lines in `scaffold.rs`).
- **F6b, `config.ingest`.** It creates `ingest.yaml` once from the answers file, through F6a's helper.
  Tests:
  - its bytes equal `build_into`'s for the same answers;
  - `coursework::load_coursework_config` parses it with no warning, checked before the create.
    **That function takes a vault and reads `<vault>/config/ingest.yaml`, answering empty when the
    file is absent** (`engine/src/coursework.rs:134-137`; §13.1), so it cannot check a temp file in
    place, and a check pointed at the wrong path passes silently. F6b writes the text into a scratch
    root under the profile's app data (`tmp\ingest-check-<pid>\config\ingest.yaml`), checks that
    root, as `build_into` checks its staged root (`scaffold.rs:762-766`), and deletes it. A test feeds
    a malformed text and asserts the job is `failed` and no file is created, which proves the check
    read the text;
  - it keeps an existing file, and keeps one created mid-way, as F6a's tests;
  - **F6a's partial-failure test** (review r2, M4): no partial file at the final name, the stray temp
    removed next run;
  - **the feeds** (§4.11, review r2 I1): `ics_url: ''` when the feed is `stored`; the link itself when
    it is `failed` or `rejected` (and then the bytes equal today's `build_into` followed by
    `restore_capability_url` for that link); the same for the other calendar;
  - **`course_map` names restored notes** (review r2, I5): with two restored course notes, the map's
    slugs for those courses are the restored notes' slugs;
  - a skipped step 6, 7 or 8 leaves its section empty, exactly as today's wizard writes it;
  - **`course_exclude`** (2026-10-24, §14.45): with a course removed at step 3, its pairs (its
    code fragments and its gradebook uid pins, built exactly as `course_map`'s are) are written as
    one single-line flow sequence `course_exclude: [...]` through `yaml_scalar`, and appear in no
    `course_map` entry; with nothing removed the key is **absent**, so the bytes still equal
    `build_into`'s (the first test above holds unchanged). **Removal never touches class times**
    (§14.47, the controller's G5): `course_exclude` governs feed items only; the registrar's rows
    are the current term's alone and reach the vault through `registrar.confirm` whatever was
    removed, so F6b writes nothing for them and needs no code for this;
  - **a fragment a kept course also contributes is never excluded** (§14.46, F7c review r2): a
    lecture and its lab both contribute `CS 100`-style human codes (`course_fragments`,
    `scaffold.rs:420-429`); with the lab removed and the lecture kept, the shared fragment stays in
    `course_map` and is absent from `course_exclude`, and only the lab's own code goes there. A
    test with two enrolments sharing a human code, one removed, asserts exactly that;
  - **uid pins** (§14.46): each course's gradebook pins, in the one pin form (the column id as the
    grade note keeps it in `source_uid`, `_<digits>_<digits>`), are written as `course_map`
    entries for kept courses and in `course_exclude` for removed ones, after that course's code
    fragments. With no grades bundle there are no pins and the bytes equal `build_into`'s; with
    pins, they are `build_into`'s plus the pin lines, in the answers file's order (a test).
  Size: S (about 65 lines in `scaffold.rs`).
- **F6c (test-writer, then implementer), ingest drops a removed course's items** (2026-10-24, the
  controller's call after the F7c review; §14.45). `engine/src/ingest.rs`, off the contract list.
  Where `course_map` is read (`ingest.rs:1251-1254`), also read `course_exclude` as fragments,
  empty when absent. At attribution (`:883`, and the institutional-noise test at `:788`): an item
  is matched with `match_course` over `course_map` **first**; only when that is `None` is it
  matched over the excluded fragments, and a hit there means the item is **not imported**: no
  task, no update, no archive on its account. A kept course always wins an item both claim. The run
  names the count in one line ("N item(s) from courses you removed were left out"), and no other
  reader of `ingest.yaml` changes. **The uid pass is bounded for pins** (§14.46, F7c review r2):
  today pass (a) of `match_course_fields` (`ingest.rs:275-279`) is a case-sensitive substring test
  in list order, so a pin `_71_1` also matches `…GradableItem-_71_10` and `…-_171_1`. For a
  fragment of the pin form (`_<digits>_<digits>`, exactly), pass (a) now requires the match to be
  **bounded**: not preceded by an ASCII letter, digit or `_`, and not followed by an ASCII digit.
  Every other fragment keeps today's substring test, and pass (b) is unchanged. The change is in the
  shared rule, so `judge`'s tier 1 and F7a's preview get it too, which is the point of sharing it. Tests, inline in `ingest.rs`'s `mod tests` on synthetic codes:
  - **a removed course's item never becomes a task** (F1 test 12's spirit, end to end through
    `sync_tasks_with_grades`): an item whose summary names only an excluded fragment, and one pinned
    only by an excluded uid, create no note and no journal record;
  - an item matching a kept fragment and an excluded one is created under the kept slug;
  - an unmatched item is still created course-less (`:883`'s rule unchanged);
  - a task already in the vault whose item is now excluded is left untouched (no update, no
    archive);
  - **the bounded pin** (§14.46): with pins `_71_1 -> a-101` and `_71_10 -> b-202` in either order,
    `GradableItem-_71_10` goes to `b-202` and `GradableItem-_71_1` to `a-101`; a pin `_71_1` alone
    does not claim `GradableItem-_171_1` or `GradableItem-_71_10`; a pin at the very end of the uid
    and one followed by a non-digit (`-_71_1@x`) both match; `a_uid_pin_beats_everything_and_is_case_sensitive`
    (`ingest.rs:1505`) passes unedited; a non-pin fragment that is a substring of the uid still
    matches as today;
  - with no `course_exclude` key, every existing ingest test and `oracle.rs` and
    `surface_oracle.rs` pass unedited (no frozen reference changes, Rule 2);
  - `commitments`, `enrich`, `cloudmodel` and `coursework` read an `ingest.yaml` carrying the key
    with no warning and the same result as without it (each ignores the key), and likewise one
    whose `course_map` carries pin-form keys (§14.46: `commitments` reads `course_map`'s keys as
    sources, `commitments.rs:710-769`, and a pin must add no claimant or warning there), in a new
    `engine/tests/ingest_exclude.rs` through each reader's public entry; a reader with none is named
    in the report for the reviewer rather than edited.
  Size: S (about 50 lines in `ingest.rs` with the bounded pin). Reviewed by `reviewer`, with the
  oracle runs shown: `oracle.rs` and `surface_oracle.rs` unedited and green, which shows no frozen
  reference relied on a pin's prefix match. If either goes red on the bounded pin, F6c stops and the
  finding goes to the planner; the reference is never regenerated (Rule 2).

## F7a, F7c. `knowlu-engine onboarding` (test-writer, then implementer)

Split by mode so that each edits about 80 lines or fewer of `engine/src/onboarding.rs` (review r2,
I6). **Common brief:** §4.4, S-F's record. Tests in `engine/src/onboarding.rs`'s `mod tests` and
`engine/tests/onboarding_cmd.rs`, on inline captures (an inline ICS, a `json!` course list and a
`json!` v2 grades bundle as O1's tests build them), never fixture files. Shared tests, written in
F7a and extended to `--tidied` in F7c:
- writes nothing: the vault's bytes and its journal are identical before and after; no socket is
  opened (the command takes no cloud config);
- exit 0 on a missing or unreadable input, with the field `null` and a line in `errors`;
- same input, same bytes (run twice, compare);
- `engine/tests/surface_oracle.rs` passes unedited.
- **F7a, `--preview` and the `cli.rs` entry.** Its own tests: upcoming items only (due after now in
  the vault's zone); turned-in items joined by `gradable_id` and `gradebook_done` only, never by
  title; a bundle with an Overall column counts no extra item; a missing `--grades` gives
  `turned_in: null` and `grades_courses: null`; exactly one mode, or a named usage line at exit 0;
  **`by_course`** (F10's review, R-C1-51; amended 2026-10-24, §14.45): the same counts per course,
  so the app can total kept rows without a second run.
  - **Keyed by the LMS course `code`** (F7c review, Important 2), falling back to the slug, then the
    label, only when `code` is empty; a past-term shell and the current shell of one course are two
    keys, never one.
  - **Attribution is ingest's own rule, never a second matcher** (F7c review, Important 3): F7a
    takes each course's `(fragment, key)` pairs from `--courses`'s `pairs` (§14.46: the very pairs
    F8b saved and F6b writes, codes and gradebook uid pins alike; its own derivation remains only
    for a list without `pairs`, which no app path sends) and calls `ingest::match_course`, the shared `match_course_fields` behind it
    (`ingest.rs:258-305`); no prefix rule, course-order precedence or bundle fallback of its own.
    The bundle's legitimate role is ingest's uid pin, built as a pair.
  - **Buckets by match set** (§14.45, so a removal is totalled as ingest will import it): each item
    is matched against each course's pairs **one course at a time** with `ingest::match_course`,
    and counted once under the sorted set of codes that matched it: usually one code (that course's
    own bucket), several for an item two courses both claim (a past and a current shell), or none
    (`unmatched`). The buckets sum to the totals. The app (F2d's `onboarding_status`) totals the
    proof as every bucket whose set is empty or names at least one kept code, which is exactly what
    F6c's ingest imports.
  - **`graded` on each single-course bucket** (§14.47, the controller's call): every course in
    `--courses` has its own single-code bucket, present even when all its counts are zero, and that
    bucket carries `graded: true` when the grades bundle has a graded item column for the course,
    `false` when the bundle has none for it, and `null` when no bundle was given. Multi-code buckets
    carry no `graded`. The app's kept `grades_courses` is the number of kept single-course buckets
    with `graded: true` (and `null` when every bucket is `null`); the total `grades_courses` equals
    the count over all single-course buckets.
  - **Class times are never removed** (§14.47, the controller's G5): the registrar holds only the
    current term, so every class time is counted in every total and imported whatever the student
    removes; `class_times` stays a total for the proof, and its buckets (by match set, F7c review r2
    Important 1) are for display only, never subtracted on a removal.
  - Tests: an item two kept courses' pairs both match sits in one two-code bucket and counts once;
    removing one of the two keeps it; removing both drops it; **class times** (F7c review r2,
    Important 1; §14.47): two shells carrying one registrar code and one meeting give one two-code
    `class_times` bucket, and the kept `class_times` total is unchanged after removing either shell
    or both; **`graded`**: a bundle grading one of two courses gives `true` and `false`, no bundle
    gives `null` on both, and a course with no feed items still has its bucket; with `pairs`
    given, an edited label changes no bucket (§14.46); "Quiz covering SYN 120"-style
    summaries and bracketed section ids land where `ingest::match_course` puts them (one case
    each, with synthetic codes); the unmatched item is in `unmatched` and in the totals.
  The new module, its JSON output through `ledger::dumps_value`, and the `cli.rs` entry. **And
  `CLAUDE.md`'s line** (Q-B3 ruled yes, 2026-10-23): §9 A11's drafted text, added under "Command and
  app contracts" in the commit that adds the command, and in no other. Size: S (about 70 lines, plus
  about 10 in `cli.rs` and the one `CLAUDE.md` line).
- **F7c, `--tidied`.** Its own tests: archive notes counted by `archived_reason`, live tasks by
  `created_by` (`live_by_source`), and `effort_estimated` counting tasks whose effort an `agent:`
  actor set (`provenance::is_agent`). Size: S (about 50 lines).

**F7b (implementer; Q-B10 ruled yes, 2026-10-23; built only after S7's privacy page names the
event).** One action,
`onboarding_step`, in `uievents::ACTIONS`, with a test that it records and that an unknown action is
still refused; the server's acceptance is checked by `cloud-engineer` first. Size: XS.

## F8a-F8h, F8s. The job queue, and the scheduler's half (implementer)

`app/src/onboarding_jobs.rs` (F8a-F8h) and `app/src/scheduler.rs` (F8s), split so that each piece
edits about 80 lines or fewer of one file (review r1 I7, review r2 I6, review r4 M4). No count is
computed in any of them. Runners are a trait; F1's tests drive every piece with fakes.
- **F8a, the core.** Job states, persistence through F2a, the serial thread, the entitlement hold.
  The hold's test is `scheduler::entitlement_state(cs) == Entitled`, re-read when F2h's
  `onboarding_entitlement` wakes it and once at the queue's start after a relaunch, where a cache
  short of `Entitled` gets one `account::refresh_entitlement` first (§4.3; review r5, I-1). It never
  reads the pending target. **The real `Runner` holds onboarding's `AppHandle`** for F8g (F8a's
  ruling 3, corrected 2026-10-24, §14.51); the queue holds none. F1's tests 3 and 5. Size: S (about
  70 lines). Merged.
- **F8a2, the crash-loop guard** (F8a review r2, I-2; §14.51; `implementer`, after F8f2 in
  `onboarding_jobs.rs`'s chain; F1 test 31, written first). Release builds are `panic = "abort"`, so
  a job that takes the process down every time it runs would do it at every launch, because
  `Queue::start` turns a `running` job back to `pending` and the queue runs it first. So:
  - `JobRecord` (F2a's, in `onboarding_state.rs`) gains `crashes: u8` with
    `#[serde(default, skip_serializing_if = "is_zero")]`. **`STATE_VERSION` stays 1**: an old file
    lacks the key and reads as 0, a count of 0 is never written (so today's files keep their bytes),
    and the struct has no `deny_unknown_fields`, so nothing older refuses the key.
  - `Queue::start`: a job found `running` gets `crashes += 1`; below 2 it is `pending`, at 2 it is
    `failed` with `REASON_JOB_CRASHED` and its fix (§4.3's second closed-set entry) and is not run.
  - `work`'s `running` insert keeps the record's `crashes` (today it replaces the whole record with
    `..Default::default()`, which would reset the count before the crash it is counting); every
    normal end (done, failed, skipped, stopped) sets it to 0; an abandoned job keeps it.
  - Try again on a `job_crashed` job sets `crashes` to 1 and `pending`, so one more crash fails it
    at the next start, not two.
  Student-facing: the fix sentence of §4.3 on the job's row, and its Try again (F11's existing
  failed state; no new screen). Size: XS (about 20 lines over the two files).
- **F8e, order and the local lane.** The second, serial lane for §4.3's local jobs; the
  config-before-readers rules; `account_settled` and "first sync" read from `cs.sync` after
  `state::refresh_sync` (§4.3's three readings: an `at` at or after the summary's
  `RunSummary.started`, no `skipped`, and `ok` for `done`, `ok: false` for `failed`; any `skipped`,
  `"no entitlement"` included, for neither; review r4 I-2, r5 M2, T0); `account_settled` also needs
  the cursor's `pulled_to_end`, read with `sync::load_cursor` and never written, and short of it the
  next `BeforeConfig` slot is enqueued while the cursor moves, with a backstop of 50 (§4.3, Q-B20;
  T0, 2026-10-24); `grades`
  after `school.apply` and `school.courses`; which scope each `slot.partial` gets (F8g); slot order; and step
  10's proposals flag in
  `onboarding_status`. F1's tests 4, 8, 11, 14's ordering and 16's status half. Size: S (about 75
  lines with T0's re-run rule; past 80, the `account_settled` reading moves to its own piece).
  **What is already built** (§14.51): the second, serial lane is F8a's (`LANE`, its own thread), and
  `confirm_step10` exists, so F8e adds neither; what remains is the rules above and **step 10's gate
  refinement** (the proposals flag in `onboarding_status` read from `config.ingest`'s state, F1 test
  11). F8e's brief should start from F8a's merged lane and name each rule's test, not rebuild them.
- **F8h, the notes gate** (split out now, review r4 M4). The in-process gate both sides block on, the
  lock order (gate, `vault_io`, `sync.lock`), and the lane's retry on `sync.lock` every 250 ms for up
  to 30 seconds, **taking and dropping `vault_io` and the lock on each try** so neither is held
  while it sleeps (review r4, M2). **Never around a path that takes them itself** (T0, §4.3):
  `registrar.confirm`'s write goes through `registrar::spawn`, which locks `vault_io` and
  `sync.lock` itself, so the retry wraps that call and its refusal, holding neither (a nested
  `vault_io` deadlocks; a nested `sync.lock` refuses). **The refusal is matched on the constant,
  never a copied string** (T0 review M2): `week::SYNC_RUNNING` (`week.rs:101`, "a sync is running;
  try again in a moment"), which `hold_sync_lock` returns for both callers, and the retry compares
  the error with it. **It is already `pub`** (F1 made it so in `8cc0caf`; corrected 2026-10-24,
  §14.45), so F8h does not touch `week.rs`. F1's test 16's gate and retry halves; F1 test 16 also runs `registrar.confirm`'s fake through
  the retry, the fake refusing with `week::SYNC_RUNNING` itself, and fails if `vault_io` is held on
  entry. Size: S (about 45 lines).
- **F8f, stop, drain, freeze and hand-over** (in `onboarding_jobs.rs`). Setting the stop, each job
  kind's safe point (§4.10's table), the wait, the **save freeze** that `onboarding_sign_out` sets
  after its last save (a later save is refused and logged; review r4, I-1), and the hand-over items
  and their clearing signals. **The freeze is a flag on the queue's own saver**, the one path the
  queue and the lane persist through; F2a's file save has no such flag, so F2e's rewind, which writes
  through F2a after the freeze, always lands (review r5, M1; F1 test 17 asserts both halves). F1's
  tests 9 and 17's freeze half. Size: S (about 60 lines).
- **F8f2, the drain's limit** (F8f review I-2; §14.51; `implementer`, after F8f in
  `onboarding_jobs.rs`'s chain, or folded into F8f's fix round if it is still open, the
  controller's call; F1 test 30, written first). Today `drain` waits with no bound, so a job that
  misses the stop leaves the page on "finishing…" for ever and a second close does nothing (F2b's
  guard). `pub const DRAIN_LIMIT: Duration` = 45 s, and `drain` becomes `drain_within(DRAIN_LIMIT)`
  (the parameter is for the test). At the limit, every running job on the `ABANDON_AT_LIMIT` list
  (the serial queue's jobs that write nothing to the vault: `school.capture`, `school.preview`,
  `mail.setup`, `google.gmail_first_read`, `google.calendar_read`, `coursework.discover`) is
  abandoned through the path `ABANDON_AT_ONCE` already uses, and the drain goes on waiting only for
  a slot or a lane job, which are never on the list (§4.10). A crash leaves the same `pending`, so
  nothing is lost that a crash would not lose. The snapshot gains the running job, whether it is a
  slot, and when the drain began, for F2b's progress answer. Composition: the drain returns, then
  F2b's `quit_flush` (10 s) runs; the two never overlap. Size: XS (about 20 lines).
- **F8s, the scheduler's half** (in `scheduler.rs`; tests are F1's 15 and 18, written first by
  `test-writer`, review r3 M5).
  - The stop flag: an `AtomicBool` on `Scheduler`, **never cleared** by anything (review r3, I-A).
  - **Two scoped functions, one per layer** (review r4 I-3; the layer corrected in review r5 I-3):
    `run_slot_scoped(cs, sch, tray_app, late, grades, mail, scope)` takes today's `run_slot_with`
    body, and `run_slot_inner_scoped(cs, sch, tray_app, late, scope)` takes today's
    `run_slot_inner` body (the `GradesSeam` and `MailSeam` built from `tray_app`) and calls it.
    `run_slot_with` and `run_slot_inner` become one-line wrappers passing `Full`, so none of the 35
    call sites of the three slot functions (28 in `app/tests/scheduler.rs`; in `app/src`,
    `main.rs:42`'s `--run-slot-once`, `run_slot` and `run_slot_inner` themselves, and `run_slot`'s
    four callers) changes (review r5, M4).
  - `SlotScope::{Full, NoRank, BeforeConfig}` (§4.3, Q-B19): `NoRank` drops `rank` from
    `slot_argv`'s output; `BeforeConfig` drops `coursework`, `grades`, `ingest`, `judge` and `rank`,
    and **does not start the grades capture stage at all** (T0, 2026-10-24: one row
    `grades (not yet: your courses are not in yet)` stands for both; F1 test 18). Each
    dropped step is the row `<step> (not yet: <why>)` at exit 0, pushed where today's named skips
    are, and **outside the runner-log filter** (`scheduler.rs:1036-1037` matches `(skipped:` forms
    only; review r5, M3), so no `state/runner-log.md` line is written for it.
  - The stop is checked before **every stage** the body starts with `steps.start` (session,
    entitlement, privacy, grades, mail, each engine child, backup, telemetry; review r3, M1); a
    stopped slot's unstarted stages are recorded as not run, never as a failure, and add no run-log
    line. **The grades bundle's delete runs on every path**, a stop included (review r4, M1): the
    stop skips stages, never that cleanup.
  - The slot's end clears `handover`'s slot items.
  - `pub fn prime_mode_device(cs, sch)` (T0; F1 test 27): `spawn`'s line at `:1154` moved into a
    function, which `spawn` then calls; F4 calls it in onboarding mode. About 4 lines.
  - `pub fn spawn_update_check(app)` (T0 review M3; F1 test 27): the housekeeping tick's update
    check (`:1285-1310`) moved into a function, which the tick then calls; F4 calls it once. A move,
    about 6 new lines.
  Size: S (about 66 lines). Merged; its "slot's end clears `handover`'s slot items" was not built
  (F8f's `slot_ended` was a stub then), so it moves to F8s2.
- **F8s2, the normal console clears the hand-over** (F8s follow-up; §14.51; `implementer`, in
  `scheduler.rs`, after F8f has merged `slot_ended` and `mail_signal`; F1 test 32, written first).
  - **The slot item.** At the end of `run_slot_scoped`'s body, on the paths where the slot ran
    (whatever its outcome), load `onboarding.json` from the profile's folder through F2a's load:
    absent or unreadable, do nothing and write nothing (the scheduler never creates or repairs it);
    unfinished, do nothing (the queue owns it until the reveal); finished with `handover` items,
    call `slot_ended()` and save through F2a's save when it changed anything. Never on a refused slot
    (script mode, another device, a slot already running): those return before the body, and the
    call sits after them.
  - **The mail items** are read at the slot's mail stage, where it already lists the account's
    addresses (`scheduler.rs:828`), never at the slot's end: a pure
    `mail_signals(rows: &[Value], mail: &MailState) -> Vec<MailSignal>` (a row `confirmed` or
    `active` is `CanaryVerified`, as `mail::runner` reads a verified forward at `runner.rs:820`; a
    row `revoked` or an address in `mail.json`'s `turned_off` is `Disconnected`; a mailbox with a
    `last_reconcile` entry is `FirstPassDelivered`), then `mail_signal` for each and one save, under
    the same finished-file rule. **`mail.json` holds no verified flag**: the canary's result lives
    on the service's address row, which is why the signal is read beside the address list
    (§4.10's table, corrected). The implementer confirms that `last_reconcile` is written only after
    a pass delivered; if it is not, it stops and reports, and the delivered signal waits for a
    ruling.
  - The "still coming" line drops each item as it clears (F11's, reading `handover`; no new words).
  Size: S (about 35 lines).
- **F8b, the school step's runners.** `school.capture`: S4d's visible sign-in, then the hidden reads
  through `grades::capture_hidden`, the LMS capture (`lms_link::capture_link_only`, F8la's
  validate-only seam, never `capture_calendar_link` or `lms_link::finish`; review r7, I-1) and the
  registrar's capture, each holding
  `school_session`'s lock. The course list and registrar rows go to the answers file at once; a
  captured or pasted link to its pending credential (§4.11). The validation-fetched feed is written
  to the profile's `tmp\` for F7a and deleted after `school.preview`. `school.preview`: F7a's
  `--preview`. **Removable rows** (F10's review, R-C1-51): F7a's counts are also reported per course
  key, saved in the answers file (the match-set buckets, keyed by code; §14.45), so the feed file is
  still deleted after the one preview; `onboarding_status` (Rust, never the page) totals the proof
  as every bucket whose set is empty or names a kept code, which is what F6c's ingest will import,
  so a Remove changes the proof with no second fetch or preview. **Two exceptions** (§14.47): kept
  `grades_courses` is the count of kept single-course buckets with `graded: true` (`null` when no
  bundle was given), and `class_times` is never reduced by a removal, since the registrar holds only
  the current term and every class time is imported whatever is removed. F8b also saves each course's pairs
  (code fragments, gradebook uid pins) as F6b needs them for `course_map` and `course_exclude`.
  **One source of pairs** (§14.46, F7c review r2 Minor 1): F8b builds each course's pairs once, at
  capture, with `scaffold::course_fragments` (made `pub(crate)` if it is not) and the bundle's
  column ids in the pin form, saves them in the answers file, and **passes the same saved pairs to
  `onboarding --preview` inside `--courses`** (a `pairs` array per course). A label the student
  typed or edited at step 3 is display only and never re-derives a fragment, so the proof, the
  `course_map` and the `course_exclude` all use identical pairs. F1 test 12 asserts the pairs in
  `--courses` equal those in `config.ingest`'s text, byte for byte, after an edited label. Depends on S-G's record. Size: S
  (about 75 lines).
- **F8c, the local lane's runners.** `school.store` and `calendar.store` (the account call, one retry,
  the feed states of §4.11, the pending credential deleted on success). **The call** (review r8, M1)
  is `lms_link::store_source(&cfg.api_base, &cfg.session_credential_target, kind, &link)` with
  `cfg = account::cloud_config(&cs.vault)`: the vault's own session, as `attach_in` stores with the
  profile's target (`account.rs:1031`), never `PENDING_TARGET`. It sits in one runner function, the
  only place test 26's pin allows `store_source(`; `school.apply`,
  `school.courses` and `config.ingest` (calls into F6a and F6b, the pending credentials deleted after
  `config.ingest`); `registrar.confirm` (R24's existing write path). F1's tests 13 and 14. Size: S
  (about 70 lines).
- **F8g, the slot runners.** Both hold the notes gate for their run (§4.3), and after each, F8e reads
  `cs.sync` for "first sync" and `account_settled`.
  - **First grades** (F9 review r1, §14.50; F1 test 28): after each slot, F8g reads the summary's
    `(name, exit)` rows with F9's `first_grades_marked` and `first_grades_failed`. Marked makes
    `first.grades` `Done`; failed (and not marked) makes it `Failed { reason: GRADES_READ_FAILED,
    fix: GRADES_READ_FAILED_FIX }`, the closed-set constants of §4.3; neither leaves it as it was.
    A later slot whose `grades` exits 0 turns a `Failed` into `Done`, and "Try again" on the item
    enqueues the next `NoRank` (or, after step 10, `slot.first_full`) slot. At finish, a failed
    first grades goes into `handover` like any unfinished item, cleared by the first slot whose
    `grades` exits 0.
  - Both call `scheduler::run_slot_inner_scoped(&cs, &sch, Some(&app), false, scope)` with the
    queue's own `AppHandle` (review r5, I-3), so the grades capture and the mail pass exist; with no
    tray, `tray::set_state` does nothing (`tray.rs:150-151`).
  - `slot.partial`: `BeforeConfig` while `config.ingest` is not `done`, `NoRank` once it and
    `school.apply` are (§4.3, Q-B19; `school.apply` added at T0), so its named skips, runner-log
    lines, privacy hold, session refresh and stop checks are the slot's own; no child loop of its
    own. A `BeforeConfig` run may repeat while the account is still being read (F8e, Q-B20).
  - `slot.first_full`: `Full`, which already runs the slot's mail pass before its engine steps
    (`scheduler.rs:966`), so **no second mail step** is added (review r3, M1).
  - A summary with `reason` set (a refusal: script mode, another device) is the job's `failed` with
    that reason, never `done`; F3's test 1 makes it unreachable for a step-2 vault.
  - **The runner holds the `AppHandle`** (F8a's ruling 3; §14.51), and reaches `Scheduler` through
    it. **A stop already set on the queue** (the `stop` the runner is handed) returns `Stopped`
    before `run_slot_inner_scoped` is called; once the slot runs, it sees `sch.stop`, which F2b's
    `stop_both` sets (§4.10's two flags). The slot call is an inner taking the slot function, so F1
    test 29's half passes a fake.
  Size: S (about 60 lines with the first-grades mapping and its constants, §14.50, and the stop check).
- **F8d, the remaining runners.** `mail.setup`, `mail.canary` and `mail.first_pass` (the mail
  commands' inner functions); `google.gmail_first_read` and `google.calendar_read`;
  `coursework.discover`. Size: S (about 70 lines).
- **F8m, the backfill hook** (in `app/src/mail/live.rs`; review r5, I-4). F1's test 23. A pure
  `on_batch_closed(mode) -> BatchClosed::{RunSlot, TellQueue}`, and in `real_env`'s `on_closed`: when
  the app manages F8a's queue (onboarding mode), call its `mail_batch_closed()`, which records the
  fact for the rail and starts nothing (the next queued slot that runs `judge`, `NoRank` or
  `slot.first_full`, pulls the batch);
  otherwise today's `scheduler::run_slot(&app, false)` on its own thread, unchanged. `start_setup` and
  `on_start`, the two builders of `real_env`, are not edited. Size: XS (about 15 lines).
- **F8l (F8la, F8lb), the link commands that only validate** (review r6, I-2; `implementer`, reviewed by
  `contract-reviewer`, since a bearer link sent to the wrong account leaks a student's classes). F1's
  test 26.
  Split in two (review r7, I-1), because F8b's `school.capture` needs the seam:
  - **F8la, before F8b** (in `lms_link.rs`, after S4b; about 20 lines):
    - `capture_calendar_link`'s body becomes
      `capture_calendar_link_with(app, unitid, finish: &dyn Fn(&str, &str) -> Value)`, and the
      shell's command passes today's `finish`, unchanged;
    - `pub fn validate_only(kind, url, fetch) -> Value`: `finish_with` with a store seam that sends
      nothing and answers `Err("kept on this computer until payment")`, so `stored` is `false` and the
      `note` says why;
    - `pub fn capture_link_only(app, unitid) -> Value`: `capture_calendar_link_with` with
      `validate_only` and today's fetch. **F8b's `school.capture` calls this**, never
      `capture_calendar_link`.
  - **F8lb, last in `onboarding_jobs.rs`'s chain** (about 25 lines): `onboarding_capture_link(app,
    unitid)` over `capture_link_only`, and `onboarding_paste_link(kind, url)` over `validate_only`.
    After the answer, each writes the link to its pending credential (F2d) and the feed's state:
    `validated`, or `rejected` for a paste that failed validation, whose link is also kept (§4.11).
    A failed capture writes nothing. Each is a wrapper over an inner taking the fetch and the profile
    id, so F1 test 26 passes a fake fetch and a generated id.
  - F8lb's two commands name none of `store_source`, `lms_link::finish(`, `capture_calendar_link(`,
    `paste_calendar_link(` and `PENDING_TARGET` (test 26's pin, review r7 M3, scoped to these two
    commands in review r8 M2; F2g elsewhere in `onboarding_state.rs` must name `PENDING_TARGET`).
  - **A source assertion over `lms_link.rs`** (review r8, M3; in test 26): `capture_link_only`'s body
    names `validate_only` and never hands `finish` to `capture_calendar_link_with`.

## F9. The reveal gate (implementer)

F1's test 6: a pure function over F2a's state and a clock, called by `onboarding_status` and
`onboarding_finish`. The cap is one constant (180 s), cited to Q7. "First sync" counts only a `sync`
that ran (§4.3). **In its own file, `app/src/onboarding_gate.rs`** (review r8's note), so it runs
beside the `onboarding_jobs.rs` chain without sharing a file; `onboarding_jobs.rs` and F2b call it.
Size: S (about 50 lines).

## F10. Shell steps 1-2; the old wizard retired (console-ui)

**The direction is ruled** (Quinn, 2026-10-23): 5b, Departures, as the spine, with the grafts in
`docs/design/2026-10-23-directions/README.md`. The build sources are in
`docs/design/2026-10-23-directions/5b-departures-board/`: `index.html` (the prototype; any screen at
`index.html#<id>`), `screens.json` (the screen ids) and `BRIEF.md` (the build contract, its "Round 3"
section included). For F10 these are `o0-welcome`, `o1-account`, `o1c-code`, `o1d-signed-in` and
`x5-picker`. The prototype's data is invented; nothing of it is copied as data. Steps 1 and 2 in that
direction, in the shell; the
picker keeps its restore-from-backup link and the adopt path. The old panels 3-10 and their
`console.js` code go (the queue slot), with their static tests rewritten or deleted and listed with
why. **Static files only:** F10 never edits `main.rs`; its report names any shell command left with
no caller, for F5b. C1's rulings on those panels are carried or retired per §9's row A10. Size: M.

## F11t, F11a-F11f. The onboarding screens and the rail

**F11t (test-writer), first (review r1 minor).** Failing static tests in a new
`app/tests/onboarding_static.rs` that pin:
- each step's ids and handlers;
- that no screen calls a command outside §4.5's list and F5's, which include `onboarding_quit` and
  `onboarding_sign_out` (review r4, I-1);
- that Finish later calls `onboarding_quit` and Sign out calls `onboarding_sign_out`, and that no
  onboarding screen calls `account::sign_out`, `switch_profile` or `delete_my_data` (review r4, M6);
- **that no onboarding screen calls a command that reads the pending target or starts work outside
  the queue** (review r5 I-1 and I-2, §13.1): `entitlement_now`, `open_checkout`, `attach_account`,
  `google_connect_url`, `google_connected`, `sync`, `backup_now`, `check_for_updates` and
  `install_update`, and (review r6, I-2) `capture_calendar_link` and `paste_calendar_link`; step 4
  calls `onboarding_checkout` and `onboarding_entitlement`, step 2 again calls
  `onboarding_attach_session` after its sign-in, step 6 calls `google_connect` and `google_status`,
  and steps 3 and 7 call `onboarding_capture_link` and `onboarding_paste_link`;
- that no onboarding screen renders a slot's step rows (`RunSummary.steps`, `renderFirstRun`); the
  rail reads `onboarding_status`'s jobs only (review r6, M1). Separately, F11a teaches
  `renderFirstRun`'s reader (`console.js:1542`) to class a `(not yet:` row as skipped, never done,
  and a static test pins that;
- that a `{restart: true}` answer shows the restart screen, whose only button calls `onboarding_quit`
  (review r4, M5);
- that the skip control is absent on steps 2, 3, 4 and 10, and that every skip shows its cost key;
- that the reveal's button is disabled while `onboarding_status` says a local-lane job is not `done`;
- that step 10 asks for proposals only when `onboarding_status` says they are ready (§4.2);
- that the main page's "still coming" line reads `handover`;
- **C1's rules F10 retired from the shell, carried here** (F10's review, 2026-10-24; §9 A10):
  - R-C1-42: step 3's window-open handler calls no capture command; the capture is called only
    from the second press's handler, and a refusal on the identity provider's page is shown;
  - R-C1-40: every exit from step 3 (Next, Back, Finish later, Sign out) calls `close_lms_window`
    for a non-shared window;
  - R-C1-51: step 3's proof renders every captured course with a Remove control that calls
    `onboarding_answer`, and the proof's counts come from `onboarding_status`, never computed;
  - R-OB-4: step 3 has the "my school isn't listed" free-text field, leading to the paste
    fallback;
  - R-C1-55: step 8's Next and the site's Skip are disabled while `discover_coursework` is in
    flight;
  - R-OB-1: step 8 never sends a blank row as ignored (only the explicit "not a class" control
    adds to `zybooks_ignore`), and no handler builds a slug from an LMS id or a name;
  - **no password field but ours, by id**: F10's whole-page "no password field" check becomes an
    allow-list of the zyBooks and VHL login fields' ids when F11c adds them, as
    `static_assets.rs:516-535` does today; any other `type="password"` fails. **This test is never
    deleted**, only its allow-list extended;
  - **credentials leave page memory the moment the write returns** (spec §5, `clearCredentialFields`;
    F10 review r2, Important 2; §14.48): step 8's handler that calls `store_credentials` clears the
    zyBooks or VHL username and password fields in **both** arms of the result (ok and refused),
    before anything else it does with the answer, by calling `clearCredentialFields` (or one
    onboarding function of that name); no credential value is assigned to anything that outlives
    the handler (no module-level variable, no saved state, no `onboarding_answer` payload), and the
    fields are read only inside that handler. The pin reads the handler's text for the call in each
    arm and scans `onboarding.js` for a password field's value assigned outside it;
  - step 4 on a `comp` entitlement shows "nothing to pay" and no trial terms ("no free plan after
    the trial" is absent from that state's render);
  - step 5's skip shows the risk-12 line ("If your school's rules don't allow forwarding your
    school email, skip this.");
  - step 6 shows `GOOGLE_TESTING` beside the consent, by reference to the one constant.
Size: S (about 30 more lines for these pins).

**Common brief (console-ui):** §4 whole, and the ruled direction: **5b, Departures**
(`docs/design/2026-10-23-directions/5b-departures-board/`: `index.html`, `screens.json`, `BRIEF.md`),
with the README's grafts:
- the wait as a countable "Building your week" list, with the 3-minute cap (`w1-wait`, `w1-cap`);
- "Found at your school", with its reason, on the rail;
- the proof's "left out and why" rows.
Each piece takes its screens by id from `screens.json`. The flow contract (§4) wins wherever a
prototype screen differs from it. The README's open items stay open: the 11 pm order is a separate
ranking draft, not this lane's, and the reveal's weekend rows fall below the fold. The prototype's
invented data is never copied as data. New files `app/static/onboarding.js`
and `onboarding.css`, loaded by `index.html` when `launch_state` or `onboarding_status` says onboarding
mode. Every screen reads `onboarding_status`, calls only §4.5's and F5's commands, and computes no
count. Each piece edits about 80 lines or fewer of code and adds fixture renders (the console's
`?fixture=` path) for its steps' states. Acceptance for each: its share of F11t's tests passes.
- **F11a:** the step frame (why, ask, proof, the states of §4.1, Finish later, Sign out, the
  "finishing <job>…" state of §4.10, with its two 15-second lines picked from F2b's
  `exiting: {job, slot, since}` and an F11t pin on each line's text; §14.51) and the rail; and one line in `console.js`'s `renderFirstRun`
  (`:1542`) so a `(not yet:` row reads as skipped (review r6, M1). **The shots hook** (F10 review r2,
  Important 3; §14.48): `scripts/console-shots.py:91` calls `KNOWLU_SHOTS.startWizard`, which F10
  removed, so the shots run aborts (not a CI gate). F11a adds `KNOWLU_SHOTS.startOnboarding(step)`,
  which opens the step frame at a given step against the fixture state, with a static pin; F12
  repoints the script at it (the script is F12's, beside `wizard-check.py`). Size: S (about 70 lines of `onboarding.js`, plus
  CSS).
- **F11e (two pieces, F11e1 and F11e2; "F11e" elsewhere means both):** steps 3 and 4, including the restored install's school suggestion (§4.7) and the feed
  kept on this device (§4.11), step 4 through `onboarding_checkout` and `onboarding_entitlement`; and
  step 2's sign-in again inside onboarding mode (§4.2, review r4 I-1), reusing the shell's step-2
  sign-in markup where the direction allows, then `onboarding_attach_session` with the sign-in's
  `account_id`, whose `wrong_account` refusal shows its sentence and the way to the picker
  (`onboarding_sign_out`). **C1's page rules at step 3 and 4** (F10's review, §4.2): the "my school
  isn't listed" field (R-OB-4); the second-press capture, never on open or on the identity
  provider's page (R-C1-42); `close_lms_window` on every exit for non-shared windows (R-C1-40); the
  removable course list at the proof (R-C1-51); and step 4's `comp` state, "nothing to pay" with no
  trial terms. Size: M; **split** (F10's review): **F11e1** steps 3's school choice, the sign-in
  and its capture rules, and step 2 again; **F11e2** step 3's proof with the removable list, and
  step 4. Each about 80 lines.
- **F11b:** steps 5, 6 and 7; step 6 on today's separate consents, one vault-target
  `google_connect` per scope (`calendar`, `gmail`), polled by `google_status` (§4.2; amended at T0,
  2026-10-24, because O2 has no branch). Step 5's skip carries the risk-12 line, and step 6 shows
  the `GOOGLE_TESTING` disclosure beside the consent, referencing the one constant (F10's review,
  §4.2). Size: S (about 80 lines; if past, step 7 moves to its own piece).
- **F11f (after O2 merges):** step 6's one consent for both scopes and the forwarding fallback
  through the mail window, over O2's cloud half and its holder count, in the same design slot; F11t
  gains its pins in the same task. Size: S (about 50 lines).
- **F11c:** steps 8 and 10, with step 10's proposals waiting on `config.ingest` (§4.2). Step 10's
  confirm is today's `commitments_confirm`, which refuses by name while a `sync` child holds
  `sync.lock` ("a sync is running; try again in a moment", `week.rs:101`); the step shows that
  sentence and its Try again, as the console does (§13.1). **Step 8's C1 rules** (F10's review,
  §4.2): Next and the site's Skip disabled while discovery is in flight (R-C1-55); a blank row is
  never sent as ignored, and a row's course is sent as its key, never a slug made on the page
  (R-OB-1); the zyBooks and VHL password fields added to F11t's allow-list by id, and both login
  fields cleared the moment `store_credentials` returns, ok or refused (spec §5,
  `clearCredentialFields`; §14.48). Size: S
  (about 80 lines; the same rule, step 10 moving out).
- **F11d:** the reveal, and the main page's "still coming" line from `handover`, each item dropped at
  its own signal; the `older_mail` line's "after the next scheduled run" case when a batch closed
  after `slot.first_full` (§4.3, review r6 M3; the rail's own row is F11a's). Size: S (about 60 lines).

## F12. The walk-through (test-writer)

`scripts/wizard-check.py` rewritten to walk steps 1-11 against fixtures (no network, no school),
including a quit and resume at steps 3, 4 and 10, a skip at 5 and 8, the reveal with one item
still coming, and Open Knowlu handing it over; and (F10's review) one course removed at step 3's
proof, one walk through "my school isn't listed" to the paste, and a `comp` account's step 4. Run
in the repository's `.wv/` venv. **Its pass is the O3 merge gate** (§8). **Also the shots script**
(F10 review r2, Important 3; §14.48): `scripts/console-shots.py:91`'s `KNOWLU_SHOTS.startWizard`
call becomes F11a's `KNOWLU_SHOTS.startOnboarding(step)`, one shot per shown step, run once to show
it no longer aborts. Size: M.

## F13, F14, F15. Docs, review, live proof

- **F13 (docs-keeper):** `docs/reference/app.md` (onboarding mode, `onboarding.json` and
  `onboarding-answers.json`, the two pending feed credential targets and when each is deleted, the
  queue and its local lane, the scheduler's stop, the commands, the picker flag), `docs/reference/engine-commands.md` (`onboarding`), `app/README.md`,
  HANDOFF's O3 row; drafts of O3's amendments (§9 rows A7-A13) for Quinn's signature. **Stale text
  found by F10's review r2** (§14.48): `app/README.md:293` (`startWizard`, "seven panels") is
  F13's. The `account.rs:2100` doc comment (`#wiz-google`) is on the contract list, so docs-keeper
  does not edit it: it goes to the next contract-engineer task that touches `account.rs` (F2c or
  S5, whichever lands later), named in F13's report. F13's report also lists the follow-up
  deletions F5b leaves: `open_external`, `retarget_credentials` and `timezone_for_state`, with
  their tests.
- **F14 (reviewer):** the lane against this plan and spec §1; report
  `docs/reports/2026-10-2X-o3-long-onboarding-review.md`.
- **F15 (main session, Quinn's go, after S10):** a dev build with both lanes, against staging, on a
  fresh scratch profile at the founder's school, with Quinn signing in. Record: seconds per step and
  the reveal's wait; the number of school prompts (one); step 3's proof against the vault after the
  first slot (S-F's pass rule); a quit and relaunch at steps 3, 4 and 8 (each resumes, no second school
  prompt with the kept session); a skip at step 7 named at the reveal; the reveal at or before 3
  minutes; Open Knowlu pressed with an item still coming, which the first slot after the reveal then
  finishes; the config files created once each (their creation times, never rewritten); no debug
  port on any school window; and **spec §8's measure**: no live task past due before the vault's first day, already
  submitted, from a past term, or estimated above the ceiling Quinn states for its kind (the late
  window itself waits for O6). Counts only; the scratch profile removed afterwards.

---

## 8. Where the other lanes fit

- **O1 (turned-in work, wave A):** merges first. Its slot order and first-sight rules are what §4.3's
  order protects; its deferred visible count (O1's Q-B) is F7c's `imported_done` on the reveal.
- **O0 (fixes):** the current-term filter must be on `main` before T3 (S-F's class count) and F15.
- **O2 (Gmail permission first, wave A):** its cloud half (the combined Gmail-and-Calendar ask on the
  one client; the count of permission holders) and its forwarding fallback must be on `main` before
  step 6 makes the combined ask. **At T0 (2026-10-24) O2 is not merged and has no branch**, so F11b
  builds step 6 on today's separate consents (`account::google_connect` with each scope, polled by
  `google_status`, both through the vault's session), and F11f adds the combined ask and the
  forwarding fallback once O2 merges. If wave B ships first, its step 6 is the separate-consent form
  (Quinn ruled this acceptable, Q-B21, 2026-10-24),
  and the ledger's §1.3 Q8 row is met by F11f, not F11b. Proof keys are unchanged (`gmail` is
  `granted` or `skipped` until F11f adds `forwarding`).
- **L1 (Microsoft 365 older mail):** after O1, O3 and O4 (Q10); at T0 (2026-10-24) not started, no
  branch, as planned. It runs on `school-session` where the
  row is shared. Its plan needs two amendments before it starts: its decision 19's browser arguments
  go into `SCHOOL_BROWSER_ARGS` (the mail window no longer has a folder of its own at a shared school,
  and S-G may already have added them), and its `init.js` closure stays on the mail window only (S3's
  test 4). Its privacy row follows S7's page version.
- **L5, then O5 and O6 (wave C):** L5 is not merged and, at T0 (2026-10-24), has no branch, so O5
  cannot follow wave B directly: it waits for L5 to be cut, built and merged, and step 9 stays absent
  meanwhile (§4.2). Nothing in wave B waits for either. O5 fills step 9, adds `syllabi_found` to step 3's
  proof and syllabus rows to step 10, and joins the syllabus reads to the reveal gate's list, all
  under §4's contract. O6's late window is outside onboarding.
- **The merge gate for O3** (F10's review, 2026-10-24). F10 retires the old wizard's panels 3-10,
  so a `feat/o3-long-onboarding` with F10 but without F11a-F11e leaves onboarding with no school,
  payment or mail step. **`feat/o3-long-onboarding` never merges to `main` before F12's
  walk-through passes** on it (steps 1-11 against fixtures, F12), whatever else is green; task
  branches (`feat/o3-f10` and the rest) merge into the lane branch, never into `main`. F14 checks
  the gate before its PASS.
- **L8 (testers):** merged (T0). An entitled tester's step 4 reads "nothing to pay"; no code here
  beyond reading the entitlement through F2h's `onboarding_entitlement` (the vault's session; never
  `entitlement_now`, §4.3).

## 9. Rule-file and signed-spec amendments this plan needs

Each is drafted by the docs task named, by text and never by line number, and **lands only with
Quinn's signature** (signed texts) or **Quinn's word** (rule files), in the lane that changes the
behaviour, before it merges.

| # | Document | Change | Ruling | Draft | Lane |
|---|---|---|---|---|---|
| A1 | Cloud design, Amendment 2026-09-29 ruling 12 | the discard-everything captures and "the registrar never shares this profile" become one kept school profile per identity | Q1, Q9 | S8 | O4 |
| A2 | Cloud design, Amendment 2026-09-30 | "one WebView2 profile per connected mailbox" becomes per identity; deletion as spec §2.5 | Q1, Q9 | S8 | O4 |
| A3 | `VISION.md`, pilot paragraph | "the kept Blackboard sign-in and the saved mail sign-in" becomes the kept school sign-in and the saved mail sign-ins | Q1 | S8 | O4 |
| A4 | Email spec D15, §4.13, §7's privacy sentences | profiles and deletes per identity | Q1, Q9 | S8 | O4 |
| A5 | Grades spec §4, §11 | `lms-session` becomes a use of `school-session`; Forget school sign-in | Q1, Q9 | S8 | O4 |
| A6 | Commitment model phase 3 spec D1, its plan's R4-a, the close-out plan, HANDOFF §3 | "the registrar never shares the kept Blackboard profile" retired; the registrar runs inside the school step | Q1, Q13 | S8 | O4 |
| A7 | Cloud design §4.2 and §11a's "keep nothing" rows | the long onboarding's steps, payment's place, the reveal | Q2, Q7, Q12, Q13 | F13 | O3 |
| A8 | Email spec D6, §4.2 step 6 | school mail is set up inside onboarding, not at the console's first open | Q13 | F13 | O3 |
| A9 | Gmail connect spec | step 6's place; one consent for Gmail and Calendar (O2 drafts its half) | Q4, Q8 | F13 | O3 |
| A10 | C1, C1b, C1c specs; C1's rulings R-C1-40, -41, -42, -47, -48, -51, -55 and R-OB-1, -2, -4 | each carried, with the task that carries it (corrected after F10's review, 2026-10-24, which found the page halves unassigned): **R-C1-40/41** the incognito fallback (S4b) and, the page's half, `close_lms_window` on leaving step 3 (F11e1); **R-C1-42** the second-press capture trigger, never on open or on the identity provider's page (F11e1, F11t); **R-C1-47/48** in F6a, F6b; **R-C1-51** the data half in F6a, F6b and the page's half, the removable course list at step 3's proof (F2d, F8b, F7a's `by_course`, F11e2, F11t); **R-C1-55** the casing pins and "discovery in flight disables Next" (F11c, F11t); **R-OB-1** at step 8, with "a blank row is never ignored" and "no invented slug" (F11c, F11t); **R-OB-2** at step 3; **R-OB-4** the school search and its "isn't listed" free-text fallback (F11e1, F6a). Retired: R-OB-4's timezone suggestion only (Q-B12). Also carried from tests F10 deleted: mail risk 12 at step 5's skip and `GOOGLE_TESTING` at step 6 (F11b), and the password check as an allow-list by id (F11c, F11t; never deleted); and, from the C1 spec's §5, **credentials leave page memory the moment the write returns** (`clearCredentialFields`, at step 8's zyBooks and VHL fields: F11c, F11t's pin; F10 review r2, §14.48) | Q2, Q12, Q13 | F13 | O3 |
| A11 | `CLAUDE.md`, "Command and app contracts" | the `onboarding` command never writes, reaches no network, exits 0 (Q-B3); onboarding mode runs no scheduler until the reveal. **Nothing about config writes:** §4.9 needs no rule change. **Ruled yes (Quinn, 2026-10-23).** The line, drafted here and added to `CLAUDE.md` by F7a in the commit that adds the command: "`onboarding` (`--preview`, `--tidied`) never writes, reaches no network, is never gated and always exits 0; an unreadable input is a named `null`. Onboarding mode runs no scheduler, tray or autostart until the reveal." | Q-B3 | this plan (text); F7a lands it | O3 (ruled) |
| A17 | `CLAUDE.md`, engine invariants ("never rewrite a vault file wholesale") | **only if** Quinn rules Q-B1's candidate exception: `ingest.yaml` extended in place during onboarding | Q-B1 | F13 | O3 (Quinn's word), conditional |
| A18 | The onboarding spec, §9's dependency line | O4 merges first, O3 second, no tag between; they still ship together. **Ruled (Quinn, 2026-10-23, Q-B11)** | Q-B11 | F13 | both (ruled) |
| A12 | `docs/reference/app.md`, `HANDOFF.md` §2 | the lanes, the profiles, onboarding mode | all | S8, F13 | both |
| A13 | The onboarding spec, §2.1 and §2.5 "Today" | a dated note: sign-out did not delete `lms-session` before O4 | fact | S8 | O4 |
| A14 | `site/privacy.html`, `PRIVACY_VERSION` | the kept school sign-in and its uses | Q1 | S7 | O4 |
| A15 | L1's plan (`2026-10-18-c3-m365-backfill-plan.md`) and the M365 redesign's decision 19 | arguments on the shared folder (§8) | Q1, Q10 | L1's planner | L1 |
| A16 | Syllabus spec §3.1, §15.2; L5's command placement | deferred to O5 (wave C) | Q1, Q13 | O5 | O5 |

---

## 10. Fidelity ledger

| Spec | Requirement | Task | Proof |
|---|---|---|---|
| §1.2 | Every step: why, ask, proof; "reading…" lets the student go on | F11a-F11e | §4.1 states; F11t; F15 |
| §1.3 | Eleven steps in the ruled order; step 9 absent until O5 | F2a, F11a-F11e | F12's walk; §4.2 |
| §1.3, Q8 | One Google consent for Gmail and Calendar at step 6 | F11f (O2's cloud half; F11b builds the separate-consent form until O2 merges, T0) | F11t; F15 after F11f |
| §1.4, Q2 | Payment right after step 3's proof; no gate change | F2a, F8a, F11e | F1 test 3; `entitle.rs` absent from the diff (F14) |
| §1.4 B | Step 3's counts from raw captures, before any note is written | F7a, F8b | S-F (T3, with the patch's saved feed); F7a's tests; F1 test 3 |
| §1.5 | Background work starts on each answer and never blocks | F8a-F8h | F1 tests 3-5, 8, 11; S-G (T3) |
| `CLAUDE.md` "left out and named" (r4 I-3) | `slot.partial` is the slot itself minus `rank`: every step it leaves out is named, with its runner-log line | F8s, F8g | F1 test 18 |
| `CLAUDE.md` "left out and named" (r5 I-3, M3; §13.1) | The scoped slots carry the grades and mail seams (onboarding's `AppHandle`); before `ingest.yaml` no `coursework`, `grades` (capture or step; T0), `ingest` or `judge` runs; a scope's rows are named `(not yet: …)` and kept out of the runner log | F8s, F8g | F1 test 18 |
| §1.11 (T0, Q-B20) | `account_settled` waits until the account's copy is read to its end (`pulled_to_end`, read only); a multi-page account re-runs `BeforeConfig`, bounded, never stalls; an entitlement-refused sync is never the first sync | F8e, F8g | F1 test 16's T0 cases |
| §1.9, Q13 (T0) | The onboarding console reports the vault's real scheduler mode, never the `Default` placeholder | F4, F8s | F1 test 27 |
| §1.4, Q2 (r5 I-1) | Checkout, the entitlement poll and the queue's hold read the vault's own session, never the pending target step 2 emptied | F2h, F8a, F11e | F1 tests 3, 22; F11t |
| §1.6 (r5 I-2) | Signing in again attaches only the vault's own account; a mismatch moves nothing and leaves no other account's session on the machine | F2g, F11e | F1 tests 10, 21; F11t |
| §1.5 (r5 I-4) | No slot starts outside the queue in onboarding mode; a closed backfill batch waits for the queue's next `judge` | F8m | F1 test 23 |
| §1.6 (r5 I-5) | The window's close never blocks the event loop; one exit at a time | F4, F2b | F1 test 24 |
| §1.6 (§13.1, Q-B18) | Finish later, the close and Open Knowlu take today's quit path (`quit_at`, push, backup, 10 s); Sign out does not | F2b | F1 test 25 |
| §1.9 (§13.1) | `ingest.yaml`'s parse check reads the text it is about to create | F6b | F6b's malformed-text test |
| §1.6 (r6 I-1) | A refused re-attach deletes the pending session on this machine only; no account is signed out anywhere | F2g | F1 test 21's local-forget case and source pin |
| §4.11, R-C3'-exec-40/41 (r6 I-2, r7 I-1) | In onboarding a captured or pasted link is validated only and sent to the account by `school.store` or `calendar.store` alone, never under the pending session, on the background capture too | F8la, F8b, F8lb, F5, F11t | F1 test 26 and its source pin over both files; F11t; F5's list pin |
| `CLAUDE.md` "left out and named" (r6 M1) | A scope's `(not yet:` row never reads as done on any screen | F11a, F11t | F11t's pins |
| §1.5 (r2 I4) | Local config and account jobs never wait behind a window or an engine step; step 10's proposals wait for `ingest.yaml` | F8e, F8c, F11c | F1 test 11; F11t |
| §1.5 (r3 I-B) | A lane note write and a `sync` take turns and wait, never refuse; a skipped `sync` is never the first sync | F8e, F8g, F9 | F1 test 16 |
| §1.5, Q7 | The reveal waits for the required list, at most 3 minutes after step 10; backfills never | F9 | F1 test 6; F15 |
| §1.5 | A failed source is named with its fix | F8a, F9, F11a-F11e | F1 test 6; F11 fixture renders |
| §1.5, §1.8 | What is still running at Open Knowlu is finished by the normal console and shown until done, each item cleared by its own signal | F2b, F8f, F11d | F1 test 9; F12; F15 |
| §1.5, Q7 (r2 I3, r3 I-A, M1) | Open Knowlu waits only for the job in flight's safe point (one slot stage at most), never the whole slot; the stop is never cleared | F8f, F8g | F1 tests 9, 15; S-E's longest stage |
| §1.6 | `onboarding.json` beside `settings.json`; resume at the step; unfinished jobs re-run | F2a, F8a | F1 tests 1-2; F12; F15 |
| §1.6 (r2 I2) | Every answer a later job needs survives a quit; a resumed run writes the same `ingest.yaml` | F2d, F2b, F8b | F1 tests 1, 12 |
| §1.6 (r3 I-C, r4 I-1) | Sign-out mid-onboarding drains, freezes saves, rewinds to step 2 and reopens the picker; a profile with no session resumes at step 2's sign-in; an absent answers file past step 3 is re-asked, never read as empty | F2a, F2b, F2d, F2e, F2c | F1 tests 10, 17; F2c's tests |
| §1.6 (r4 I-1, M5, M6) | Every exit is one onboarding command that drains and ends the process, a failed relaunch included; no other console exit is reachable | F2b, F4, F4b, F11t | F1 tests 9, 17, 19, 20; F11t |
| §1.6 | A quit never cuts a running job off | F2b, F4, F8f | F1 test 9 |
| §1.6 | Scheduler and autostart only at the reveal | F4 | P-3; F1 test 10; F15 |
| §1.6 | Leaving at payment returns to payment, entitlement re-asked, nothing re-captured | F2a, F11e | F1 test 13(b); F12; F15 |
| §1.7, Q12 | Skips with costs; four steps unskippable; the paste fallback | F2a, F11a-F11c, F11e | F1 test 7; F11t |
| §1.7 | Every skip named at the reveal and in Settings, never nudged again | F9, F11d | F1 test 6; F13's reference |
| §1.8 | The reveal: week, next, free time, tidied, still coming, Open Knowlu | F7c, F9, F11d | F7c's tests; F15 |
| §1.9, Q13 | Vault at step 2 with defaults; full-screen mode of the console | F3, F4 | F3 tests 1-2; P-3; F15 |
| §1.9 A | An abandoned vault is listed "setup not finished" | F3 | F3 test 4 |
| §1.9, `CLAUDE.md` | No vault file rewritten wholesale: each config file created once, by hard link or `MoveFileExW` without replace, never `rename`; a restored vault never overwritten | F3, F6a, F6b | F3 tests 2-3; F6a and F6b's tests, the mid-way, partial-failure and run-time fallback cases included; F15 |
| R-C3'-exec-40/41 (r2 I1) | A feed the account cannot take is kept in `ingest.yaml`, never lost; a rejected paste is never sent; the link survives a quit on this device only | F2d, F2c, F6b, F8c | F1 test 13; F6b's feed tests; F2c's tests |
| §2.7 (r3 M6) | The privacy page names the waiting feed link and the answers file, and that an abandoned setup keeps them until finish, sign-out or deletion | S7 | the guard; Quinn's read |
| §1.11 (r2 I5) | A restore that could not reach the account never leads to duplicate course notes; `course_map` names restored notes | F3, F6a, F6b, F8e | F3 test 5; F1 test 14; F6a and F6b's restored-note tests |
| §1.11 | Restored or adopted installs skip proven steps; a restored install's school is suggested only from this device's own `onboarding.json`, never from the account, never assumed (T0) | F2a, F11e | F1 test 2; F3 test 3 |
| §1.11 | C1's rulings carried or retired | F10, F13 | §9 row A10 |
| §2.3, Q1 | One kept school profile at a shared school, after S-A | S2, S4a-S4d | S-A (T1, T2); S3; S10 |
| §2.4 | Under the profile's app data; guard; no password; never synced | S2 | S1 tests 1-2 |
| §2.4 | `shared_sign_in` on the curated row, data not code | S2 | S1 test 4; S2's row and its spike citation |
| §2.4 | Hidden later reads, one window at a time | S4a-S4c, F8b | S3 test 5; S1 test 5 |
| §2.4 | Cookies sent only to the host they were read for (the queried host); scripts and popups per window; the mail window kept to its allow list, falling back to its own folder on a refused host; no capability | S2, S4a-S4c | S3 tests 3-4; S2's `CAMPUSES` test |
| §2.4 | The incognito capture stays as the fallback | S4b | S3 test 1 |
| §2.5, Q9 | Forget school sign-in; last use deletes; sign-out and deletion always | S2, S5, S6a, S6b | S1 test 3; S5 tests; S6t; S10 |
| §2.5, Q9 | Disconnecting the school mailbox, and the saved-sign-in switch off, clear the use; the folder goes with its last use | S4c | S3 test 7; S10 |
| §2.5 | After Forget, grades, registrar and school-mail repair each need a sign-in | S4a-S4c | S3 test 8; S10 |
| §2.7 | Environment options match across the folder (S-B), or the mailbox falls back | S4a-S4c | S-B (T2); S3 test 2 |
| (r1 I5) | No debug port on any window on the school folder, in any build | S4a-S4c | S3 test 2; F15 |
| §2.7 | The privacy page names the kept school sign-in; version moves | S7 | the guard; Quinn's read |
| §4.2 item 1 | The registrar runs inside the school step, writes as the student, editable on step 10 | F8b, F8c, F11c | F15 |
| §6.3, O1 Q-B | "N already turned in were archived" shown once at the reveal | F7c, F11d | F7c's tests; F15 |
| §6.2, O1 | Grades before ingest; `rank` only in the full slot; a stopped slot leaves `is_first_run` true | F8e, F8f | F1 tests 4, 8, 9 |
| §8 | No past-window, submitted, past-term or oversized task after the reveal | F15 | F15's measure |
| §11 | Each signed text amended in its lane before merge | S8, F13 | §9; Quinn's signatures |
| Rule 2 | No frozen reference changes | F7a, F7c | the shared `surface_oracle` test; F14 |
| (plan) | Only `account.rs` from the contract list, by contract-engineer | S5, S7, F2c | S9; F14 |
| §9 A10, C1's page rules (F10's review) | R-C1-40's close on leaving, R-C1-42's second-press trigger, R-C1-51's removable courses, R-C1-55's disabled Next, R-OB-1's blank-row and slug rules, R-OB-4's "isn't listed", the risk-12 line, `GOOGLE_TESTING`, the password allow-list, the `comp` state | F11e1, F11e2, F11b, F11c; F2d, F8b, F7a (the course list) | F11t's C1 pins; F1 test 12 (removal survives a quit); F7a's `by_course` test; F12 |
| R-C1-51 (2026-10-24, §14.45, §14.46) | A removed course hides its feed items: ingest imports nothing only a removed course claims, a kept course wins an item both claim, a shared fragment is never excluded, a pin claims only its own column, and step 3's totals equal what the vault will hold; attribution is ingest's one shared rule over one set of pairs | F6b, F6c, F7a, F8b | F6c's tests (the bounded pin included); F6b's `course_exclude` and shared-fragment tests; F7a's match-set tests; F1 test 12 |
| §1.9 (F10's review) | Rust names the vault past profiles and folders on disk; the old flow's errors are gone from the step-2 path | F3 | F3 tests 6-7 |
| (plan; F10's review) | `feat/o3-long-onboarding` never merges to `main` before F12's walk-through passes | F12, F14 | §8's merge gate; F14 checks it |

## 11. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before T1: **passed in round 8**, whose minors are closed
   in §14.40 with no further round. Rounds 1-7 failed and are answered in §14.
2. **Quinn:** **ruled 2026-10-23** (§5.0): Q-B3 yes, Q-B10 yes, Q-B11 O4 then O3 with no tag
   between, Q-B14 the allow list with its fallback, and the direction (5b, Departures, with the
   README's grafts). Q-B1 stands at "no exception needed". Still Quinn's: each spike's verdict when
   one fails, every "why" line, the privacy page, and §9's signatures.
3. **Controller:** each spike's record before the tasks that depend on it (§7); Q-B2, Q-B4 to Q-B9,
   Q-B12 to Q-B20 (Q-B20 decided 2026-10-24).
4. **Quinn reads:** every "why" line (F11a-F11e), the privacy page (S7), and signs §9's amendments.
5. **S9, then F14** before each push; each lane PR merges on green CI and its reviewer's PASS
   (`contract-reviewer` for `account.rs`); **no release tag between the O4 and O3 merges.**
6. **Quinn's go** for S10 and F15.

## 12. Out of scope

- The look, layout and words of every screen (the chosen direction; Quinn reads the words).
- The syllabus step and its reads (O5), the late window (O6), Microsoft 365's older mail (L1).
- Gmail permission's cloud half and the 100-holder count (O2); verification and CASA (Launch, Q5).
- A second desktop's sync of sessions or portal logins (they stay per device).
- Changing `entitle.rs` or any entitlement state for onboarding (Q2 ruled B).
- Moving or renaming an existing `lms-session` or mail profile (Q-B4).

## 13. Fidelity notes

*(T0's drift check, the spikes' records, review rounds and answers go here, dated, newest first.)*

### 13.1 Reuse audit (round 5, 2026-10-23, planner, this tree)

Every existing function, command, credential target, lock and seam this plan reuses or wraps,
opened and checked for its signature and side effects, the credential or target it reads, and its
other callers. **"No"** rows did not hold as the plan was written and are fixed where named; **"Holds,
tightened"** rows held but a brief now says what the code requires. T0 re-runs this table on the lane
branches. Tally (corrected in review r6, M4): 54 rows; 12 "No" (7 of them review r5's I-1 to I-5,
4 found by this audit, 1 by review r6's I-2), 7 "Holds, tightened" (the `sign_out_with` row
rewritten for review r6's I-1, and `renderFirstRun`'s reader added for its M1), 35 "Holds".
**T0's re-run (2026-10-24, `feat/o4-school-sign-in` at `1d3c467`):** every row's `file:line`,
signature, target and callers held, but the `SyncStatus` row's reading was wrong (corrected in
place, still "Holds, tightened"), and two rows are added: `Cursor.pulled_to_end` ("Holds") and
`Scheduler.mode_device` ("No"). Tally now 56 rows: 13 "No", 7 "Holds, tightened", 36 "Holds".

| Function, command or seam | file:line | What the plan assumes | Holds? |
|---|---|---|---|
| `account::entitlement_now` | `account.rs:1195` | step 4's poll and the queue's hold | **No**: reads `PENDING_TARGET` only. Fixed: F2h, F8a (I-1) |
| `account::open_checkout` | `account.rs:1611` | step 4's checkout | **No**: `PENDING_TARGET`. Fixed: F2h's `onboarding_checkout` (I-1) |
| `account::refresh_entitlement` | `account.rs:994` | reads the vault's target, saves the cache | Holds: `cloud_config` → `session_credential_target`; also called by `attach_account`, the slot (`scheduler.rs:898`) and housekeeping (`:1269`) |
| `scheduler::entitlement_state` | `scheduler.rs:291` | the hold's test | Holds: `load_cache` and `decide` within the grace; no network |
| `account::checkout_url_at` | `account.rs:1292` | consent retry, then checkout, for any token | Holds; its one caller today is `open_checkout` |
| `account::open_portal` | `account.rs:1626` | the shape F2h copies | Holds: vault target through `cloud_config` |
| `account::cloud_config` | `account.rs:855` | names the vault's account and target | Holds, tightened: refuses a `cloud.yaml` whose host differs from `api_base()` (`:870`), so F1 test 22 points `KNOWLU_API_BASE` at its listener with `app/tests/account.rs`'s guard; test 21 writes `api_base()`'s own value |
| `account::move_session` | `account.rs:143` | moves the pending session onto the profile | **No** as reused: overwrites `to` unconditionally. Fixed: F2g compares the accounts first (I-2) |
| `account::load_session` | `account.rs:128` | the pending session's account id | Holds: `(username = account id, Session)` |
| `google_sign_in`, `send_magic_link`, `verify_email_code` | `account.rs:1579`, `:601`, `:614` | step 2 again's sign-in, on the console list | Holds: save to `PENDING_TARGET`, answer `account_id`, listed at `main.rs:194` |
| `account::attach_account`, `attach_in` | `account.rs:1143`, `:1019` | the re-attach T0 was to name | **No**: refuses a vault with `cloud.yaml`. Fixed: F2g; F11t pins `attach_account` uncalled (I-2) |
| `account::sign_out_with(None, …)` | `account.rs:753` | (round 5: F2g's forget on a mismatch) | Holds, tightened (corrected in review r6, I-1): its `/logout` carries no `scope` (`:776-781`), which GoTrue reads as `global`, so it would sign the other account out on every device. F2g no longer calls it; its forget is a local `credentials::delete(PENDING_TARGET)` |
| `lms_link::finish`, `store_source` (via `capture_calendar_link`, `paste_calendar_link`) | `lms_link.rs:843-847`, `:801`, `:397`, `:883` | step 3's capture and paste and step 7's paste validate the link, which waits in its pending credential until `school.store` | **No** (review r6, I-2): `finish` sends a validated link to the account at once, under `PENDING_TARGET`, which step 2 emptied, or which may hold another sign-in parked on this machine. Fixed: F8la's `validate_only` and `capture_link_only` (`finish_with` with a store seam that sends nothing), used by F8b's `school.capture` and F8lb's two commands; F5 registers those, never the shell's pair (review r7, I-1) |
| `account::sign_out` | `account.rs:710` | `onboarding_sign_out` calls it; it ends nothing | Holds: closes mail windows only for this console's profile (`:715`); revokes via `api_base()` |
| `account::sign_out_mail_client` | `account.rs:813` | F2c's clean | Holds, tightened: takes `api_base`; F2c's `sign_out_clean` now passes it through (M5) |
| `account::delete_local_data` | `account.rs:1664` | removes app data and, after F2c, the pending links | Holds: `(vault, data_dir, backup_root, profile_id, app_root, session_target)`; logins loop at `:1696` |
| `google_connect_url`, `google_connected` | `account.rs:2111`, `:2137` | (step 6 named no command) | **No** for onboarding: `PENDING_TARGET`. Fixed: step 6 uses `google_connect`, `google_status`; F11t pins the pair uncalled (audit) |
| `google_connect`, `google_status` | `account.rs:2198-2210` | step 6's consent and poll | Holds: `console_session` reads `cloud.yaml`'s target (`:2163`) |
| `account::valid_access_token_at` | `account.rs:516` | F2h's token | Holds: one in-process refresh lock, blocking |
| `scheduler::run_slot_inner` | `scheduler.rs:731` | the slot with its seams | **No** as the scope sat: seams only from `Some(app)`. Fixed: `run_slot_inner_scoped`, F8g passes `Some(&app)` (I-3) |
| `scheduler::run_slot_with` | `scheduler.rs:774` | refuses unless App and device; stale `last` while running | Holds, tightened: F3 test 1 asserts `runners.yaml`; F8m keeps every other slot out; F8g reads a `reason` as `failed` |
| `scheduler::slot_argv` | `scheduler.rs:520` | a slot before `ingest.yaml` names `coursework` and `ingest` as skipped | **No**: both always run on a cloud vault (`:529`, `:538`). Fixed: `BeforeConfig` (§4.3, F8s, Q-B19; audit) |
| runner-log skip filter | `scheduler.rs:1036-1037` | scope rows | Holds: no `rank (` form; F8s's `(not yet:` rows stay out (M3) |
| `RunSummary.started` | `scheduler.rs:793` | "sync ran" compares to it | Holds: `now_ts(None)`, the format of `SyncStatus.at` (M2) |
| `SyncStatus` | `engine/src/sync.rs:2408` | `ok`, `at`, `skipped` | Holds, tightened (corrected at T0, 2026-10-24): an entitlement-gated sync is `ok: false` **with** `skipped: "no entitlement"` (`record_gated_skip`, `sync.rs:2462-2475`), so it reads as "neither", not `failed`; round 5's reading was wrong |
| `Cursor.pulled_to_end`, `sync::load_cursor` (T0) | `engine/src/sync.rs:227`, `:238`; set `:2618-2620` | `account_settled` | Holds: sticky once a pull returns `more: false`; one page per sync run, so F8e re-runs `BeforeConfig` (Q-B20); read only |
| `Scheduler.mode_device` (T0) | `scheduler.rs:68`, `:146`; primed `:1154`, `:1255`; read `commands.rs:59-71` | (unstated: kept right in onboarding mode) | **No**: primed only by `spawn` and its housekeeping thread, so onboarding mode would report `(Script, true)`. Fixed: F8s's `prime_mode_device`, called by F4 (F1 test 27) |
| `state::refresh_sync` | `state.rs:174` | fills `cs.sync` at the slot's end | Holds (`scheduler.rs:1116`) |
| `scheduler::run_slot` callers | `tray.rs:49`; `scheduler.rs:1162`, `:1206`; `mail/live.rs:121` | none fires in onboarding mode | **No** for `mail/live.rs:121`. Fixed: F8m (I-4). The tray and `spawn` do not exist there |
| `mail::live::real_env`, `start_setup`, `on_start` | `mail/live.rs:111`, `:130`, `:146` | kept in onboarding mode | Holds but for `on_closed` (row above) |
| `tray::set_state` | `tray.rs:150` | harmless with no tray | Holds: `tray_by_id("main")` is none |
| `tray::build`'s menu | `tray.rs:21` | absent in onboarding mode | Holds: its Run now and Quit never exist |
| `main` window `CloseRequested` | `main.rs:187-193` | routed to `onboarding_quit` | **No**: the handler runs on the event loop. Fixed: F4 prevents the close and hands it to a thread (I-5) |
| `commands::switch_profile` | `commands.rs:669` | refuses through an inner | Holds: no inner today, F4b adds it; it stamps `quit_at` and runs `quit_flush` |
| `state::quit_flush` | `state.rs:330` | (the exits said "save; exit") | **No**: every quit today stamps `quit_at` and pushes and backs up. Fixed: §4.10, F2b, Q-B18 (audit) |
| `onboarding::relaunch`, `relaunch_with` | `onboarding.rs:404`, `:415` | spawn only | Holds |
| `--pick` | `main.rs:62-63` | sign-out reopens the picker | Holds |
| `profiles::resolve_launch` | `profiles.rs:231` | `run_console` then reads `LaunchMode` | Holds: the mode is never decided here |
| `week::run_confirm`, `hold_sync_lock` | `week.rs:111`, `:122` | F8h's retry; step 10's confirm | Holds, tightened: `hold_sync_lock` is `pub(crate)`; step 10's confirm can meet "a sync is running" (F11c) |
| `week::commitment_proposals_inner` | `week.rs:54` | reads `ingest.yaml`'s calendars | Holds |
| `coursework::load_coursework_config` | `engine/src/coursework.rs:134` | parse-checks the text before the create | **No**: takes a vault, reads its `ingest.yaml`, empty when absent. Fixed: F6b's scratch root and malformed-text test (audit) |
| `journal::create_actor_file` | `engine/src/journal.rs:213` | the precedent only | Holds (not called) |
| `sync::restore_into`, `Restored.ok` | `engine/src/sync.rs:1545`, `:938` | step 2's restore | Holds |
| `ingest::is_first_run` | `engine/src/ingest.rs:493` | true until `state/today.md` | Holds: only `rank` writes it, which two scopes leave out |
| `uievents::ACTIONS` | `engine/src/uievents.rs:11` | closed at 11 | Holds |
| `grades::capture_hidden`, `open_visible` | `grades.rs:669`, `:574` | S4a moves them to the shared folder | Holds: both use `session_dir(data_dir)` today; `Capturing` is one per process |
| `grades::forget`, `is_session_dir` | `grades.rs:288`, `:279` | never deletes the shared folder | Holds: exact `lms-session` only, `-> bool` |
| `grades::campus_of` | `grades.rs:329` | the slot reads `campus.yaml` at its start | Holds: before `school.apply` the step is a named skip; a `school.apply` landing mid-slot is one too (§4.3, T0) |
| `WindowHost::open_window` (`mail::open_visible`, `open_hidden`) | `mail/live.rs:68-74` | take a profile path | Holds (S4c passes `school-session`) |
| `scheduler::mail_pass`, `MailWindow::try_take` | `scheduler.rs:443` | the slot's mail pass | Holds: a held window is a named skip |
| `lms_link::LmsSession` | `main.rs:167` | managed in the console | Holds |
| `renderFirstRun`'s row reader (review r6, M1) | `app/static/console.js:1542` | a scope's `(not yet: …)` row is not shown as done | Holds, tightened: the reader marks only `(skipped:` as skipped, so an exit-0 `(not yet:` row would read as done. F11a teaches it `(not yet:` as skipped, and F11t pins that no onboarding screen renders slot rows |
| `Scheduler`, `Updates` managed state | `main.rs:161`, `:164` | kept in onboarding mode | Holds: F4 keeps both and drops only the tray, autostart and `spawn` |
| `updates::hold_for_install` | via `tray.rs:90` | never in onboarding | Holds: sets `running`; reachable from `install_update` (F11t pins it uncalled) and the tray |
| `credentials::write`, `exists`, `delete` | `credentials.rs:17-60` | any target | Holds |
| `scaffold::runners_yaml`, `debug_port` | `scaffold.rs:656`; `debug_port.rs:9` | App mode and device from birth; a port only with the env var | Holds |

- **2026-10-24, T1: S-A phase 1, first half (main session; Quinn at the machine).** A fresh Edge
  profile on a new, empty `--user-data-dir`, at the founder's school. **New fact:** the school's MFA
  is **Okta, not Duo** (Quinn and this spike); the plan's wording is corrected. Results, in host
  kinds and counts only:
  - the LMS: a password and a "stay signed in?" prompt, **no MFA prompt**; signed in;
  - Outlook (the school mailbox), opened next on the same profile: signed in at once, **no prompt**;
  - the registrar, opened next: signed in in about 1 second, **no prompt**.
  Prompts in all: one password, one "stay signed in?", zero MFA. Simple Syllabus: not recorded.
  **Restart check (2026-10-24):** Edge was closed (forced), and after 10 minutes the same profile
  reopened at all three; all three came up signed in with no prompt (Quinn). **S-A phase 1: PASS.**
  Edge may lean on the Windows account, so phase 2 (WebView2, T2) is still the deciding test.
- **2026-10-24, T0 part B: citation drift (main session, `feat/o4-school-sign-in` at `1d3c467`).**
  The app and engine code this plan cites is byte-identical to `49b0b43`, so all 54 rows of §13.1
  held as cited, and of about 75 other citations three were off by one, now corrected in place:
  `scheduler.rs:783-786` is `:782-786` (§4.3, the stale `sch.last`); `sync.rs:2523-2553` is
  `:2524-2554` (the skipped-sync record); review r1's `spawn` at `:1129` is `:1134` (the code-read
  facts and the r1 line below). `run_slot_inner`, `run_slot_with` and
  `run_slot` have 35 call sites; `scheduler::run_slot` has exactly the four callers §4.3 names;
  `main.rs:42`'s `--run-slot-once` is the only other slot starter, and runs in its own process,
  never in onboarding mode.
- **2026-10-24, T0 part A: premises that hold (main session).** L6, L8, O0 (#63) and #19 are merged.
  `PUT /account/sources` refuses without an entitlement (`cloud/supabase/functions/account/handler.ts:126`,
  `entitlement.ts:89`). `sync` and `judge`'s Gmail pull run correctly with no `config/ingest.yaml`,
  and no engine file reads `campus.yaml`. `CHILD_TIMEOUT` is 20 minutes, the child killed by
  `kill_tree` (`scheduler.rs:597-611`). The first-run slot fires after a relaunch from onboarding
  (`device_name` matches; `needs_first_run` tests `state/today.md`, `scheduler.rs:271`). No re-attach
  path exists, so F2g's is the only one. `Scheduler` is managed unconditionally in `run_console`
  (`main.rs:161`). `std::fs::hard_link` onto an existing target on NTFS fails with
  `ERROR_ALREADY_EXISTS` ("Cannot create a file when that file already exists") and leaves the
  target untouched (§4.9).
- **2026-10-24, T0 part A: premises that did not hold, each amended (main session; the planner's
  edits, §14.42).**
  1. **L5, O2 and L1 are not merged and have no branch.** O5 cannot follow wave B directly (§8);
     F11b builds step 6 on today's separate consents and the new F11f adds O2's combined ask after
     O2 merges (§4.2 step 6, §7, §8, the ledger's §1.3 Q8 row).
  2. **`grades` reads course notes** (`grades.rs:405-415`) and with none skips every course and
     writes no grade note (`:538-541`). The controller's call: `BeforeConfig` leaves `grades` out,
     capture and step, as `grades (not yet: …)`; `grades` runs first in `NoRank`, after
     `school.courses` and `school.apply` (§4.3, §4.6, F8s, F8e, F8g, F1 tests 8 and 18, Q-B19).
  3. **An entitlement-refused sync is `skipped: "no entitlement"`** with `ok: false`
     (`sync.rs:2462-2475`), not an unskipped `ok: false`. §4.3's readings and §13.1's `SyncStatus`
     row are corrected: it reads as "neither". The controller's call: `account_settled` also needs
     the cursor's sticky `pulled_to_end` (`sync.rs:227`, `:2618-2620`), read only. The planner adds,
     because a sync pulls one page per run, the bounded `BeforeConfig` re-run (Q-B20, for the
     controller; §4.3, F8e, F1 test 16).
  4. **The account never returns a stored feed's host** (`account/index.ts:192` selects
     `kind,added_at`). The controller's call: §4.7's read-back suggestion is dropped, and the school
     a restored install is offered comes only from this device's `onboarding.json`. No cloud change.
  5. **The hold against a sync is taken only by `week::run_confirm` and `registrar::spawn`;**
     `commands::mutate` takes `vault_io` only. §4.3's wording is corrected; the lane's jobs were
     right as written.
  6. **Onboarding mode leaves `Scheduler.mode_device` at its placeholder** (`scheduler.rs:146`,
     primed only at `:1154` and `:1255`, both inside `spawn`), and `attach_scheduler` reports it on
     every poll. F4 now calls F8s's `prime_mode_device` (F1 test 27). `spawn`'s `quit_at` clear and
     `startup_missed` (`:1137-1149`) and the housekeeping thread are skipped too; benign (F4).
  7. **Minor:** `campus.yaml`'s row is read at slot start (`scheduler.rs:732`) and `lms` at the
     grades step (`:945`), so a `school.apply` between them gives one named skip; noted in §4.3.
  Not reported by T0: the ruled direction's files (T0's open box, before F10) and S-F's code read
  (re-read live in T3).
- **2026-10-23, review r4's code reads (planner, this tree).** `onboarding::relaunch` spawns the new
  process with `--after-pid` and returns (`onboarding.rs:391-412`); `relaunch_with` is a wrapper
  (`:415`). `account::sign_out` builds its `close` and `clean` closures and returns
  `sign_out_with`'s JSON (`account.rs:710-729`). `run_slot_with`'s callers are one in `scheduler.rs`
  (`:762`) and seven in `app/tests/scheduler.rs`, which is why `run_slot_scoped` keeps it as a
  wrapper. `state::refresh_sync` is `state.rs:174`, called at the slot's end (`scheduler.rs:1116`).
  `switch_profile` is `commands.rs:669` and `delete_my_data` is `account.rs:1735`.
- **2026-10-23, review r3's code reads (planner, this tree).** `run_confirm` takes `vault_io` then
  `state/sync.lock` and refuses by name when a sync holds it (`week.rs:100-117`); a slot's sync that
  meets the lock is skipped at exit 0. `run_slot_with` (`scheduler.rs:774`) starts its stages with
  `steps.start`: session `:841`, entitlement `:897`, privacy `:923`, grades `:944`, mail `:966`, the
  engine children in the loop at `:1060-1068`, backup `:1084`, telemetry `:1104`; the mail pass runs
  before the engine steps. `account::sign_out` builds a `clean` closure and passes it to
  `sign_out_with` (`account.rs:728`), which calls it under the mail hold before the credential is
  deleted (`:757-767`).

- **2026-10-23, review r2's code reads (planner, this tree).** `scheduler::CHILD_TIMEOUT` is 20 minutes
  (`scheduler.rs:28`); `run_slot_inner`'s step loop is `scheduler.rs:1060-1068` with no stop between
  steps, and a child's stdout and stderr are piped (`run_child`, `:653-655`). `create_vault_in`'s
  unstorable-feed fallback is `onboarding.rs:744-773`; `restore_into`'s network-down arm is described at
  `:794-797`, and `sync::Restored` carries `ok` and `empty` (`sync.rs:938-944`). `restore_into` refuses
  a vault that is not fresh (`sync.rs:1546-1548`), which is why §4.9 waits for a `sync` rather than a
  second restore. `week::commitment_proposals_inner` is `week.rs:54`. `credentials::write`, `exists`
  and `delete` take any target (`credentials.rs:17-60`), so §4.11 edits no contract-list file but
  `account.rs` (F2c); `delete_local_data` deletes only `zybooks` and `vhl` logins today
  (`account.rs:1696-1698`). `journal::create_actor_file` opens with `create_new` (`journal.rs:222`).

- **2026-10-23, the planner's code read (this plan's).** On `feat/o1-turned-in-work`: L6 and L5a are
  merged; `account::sign_out_with` deletes mail profiles only (Q-B9); `scheduler::spawn` runs a slot
  at launch for a vault with no `state/today.md` (P-3); `PUT /account/sources` is entitlement-gated
  (Q-B2); the windows' arguments differ three ways (S-B); `uievents::ACTIONS` is closed at 11
  (Q-B10); O1's `gradable_id` and `gradebook_done` are public (S-F). P-3's answer and S-F's code-read
  answer are in §3; T0 re-confirms both.
- **2026-10-23, review r1's drift (reviewer, against `02e9e73`).** All of the planner's findings held.
  Line drifts: `scheduler::spawn` at `:1129` (`:1134` on `1d3c467`, T0), and its first-run slot needs App mode and `device_ok`;
  `run_slot_inner` at `:726`; `slot_argv` at `:515`. The grades window carries wry's default
  arguments (`debug_port::WRY_DEFAULT_ARGS`), not none. The mail window's debug port is added only
  when `KNOWLU_MAIL_DEBUG_PORT` is set. The text above is corrected; T0 re-reads every line on the
  lane branches.

## 14. Responses to the plan reviews (2026-10-23)

**Round 1** (§14.1-§14.10). Report: `docs/reports/2026-10-23-wave-b-plan-review-r1.md` (filed from the
controller's summary; its head says so). Each finding, with the controller's direction where it gave
one. Where round 2 changed an answer (Q-B2's memory-only link, Q-B14's deny list, §4.10's wait for
the whole job), §14.11-§14.17 say so; the round-1 text is kept as written.

### 14.1 Critical 1: a whole re-render could overwrite a restored vault's config

**Answered by the controller's direction: write once, create-new.** §4.9 is new. Each config file is
created once, with an exclusive create, at the first step that knows its whole content, as
`config/actor.yaml` is. An existing file is kept, never replaced, so neither a C3′ restore nor a second
device is ever overwritten. Config never syncs, which the plan now states. Step 2 writes no
`campus.yaml`, `events.yaml` or `ingest.yaml` (F3 tests 2-3). `school.apply` (F6a) creates the first
two and the course notes after payment, and `config.ingest` (F6b) creates `ingest.yaml` on leaving
step 8.

**Files that cannot be written once: none.** The cost is that `ingest`, `coursework` and the calendar
reads wait for the end of step 8 (§4.3, S-E's added measure). Q-B1 is reframed: is any exception
needed? The recommendation is none, so no `CLAUDE.md` change is needed (A11 notes it). The one
candidate exception is recorded as conditional A17, to be brought only if S-E shows the cap broken.
The round-1 re-render is withdrawn.

### 14.2 Important 2: quitting mid-job

§4.10 is new: every way out stops new starts and waits for the running job within that job's own
bound. Open Knowlu needs `school.apply` and `config.ingest` done, then writes `handover`. The normal
console finishes the handed-over items through its usual paths (the first-run slot,
`mail::live::on_start`), and the main page shows them until the first slot ends. Tests: F1 tests 9
and 10. Tasks: F2, F4, F8a, F11d. T0 confirms the scheduler's step bound.

### 14.3 Important 3: spec §2.5's mail rows

S3 test 7 (Disconnect, the saved-sign-in switch, the non-shared case unchanged) and S4c, through
`mail::commands::disconnect_with` and `keep_session_with`. Disconnect clears the `school_mail` use
and never calls `mail_session::forget` on the shared folder (whose guard refuses it anyway, now
stated in "What the code read found"). The folder goes when the last use clears (Q9), and S10 checks
it.

### 14.4 Important 4: cookie isolation

S3 test 3 now records `(cookie host, request host)` pairs and fails on any mismatch, because the
cookie read can return the identity provider's parent-domain cookies. S3 test 4 adds the mail
window's popup filter (unchanged, tested against the LMS and registrar hosts) and a new navigation
guard on the shared folder (Q-B14; a deny list in round 1, an allow list since round 2, §14.16). `init.js` is not
edited.

### 14.5 Important 5: debug port; CDP on school pages

`SCHOOL_BROWSER_ARGS` never carries a debug port, in any build, pinned by S3 test 2 with
`KNOWLU_MAIL_DEBUG_PORT` set. The debug port remains only for a mail window on its own folder. §3's
rules forbid a debug port near any school window and keep the harness off when its mechanism would
reach every environment. S-G now measures from Rust only: outcome, seconds and the host kind the
hidden window's URL sat on. No CDP, and no `document.visibilityState` read from a school page.

### 14.6 Important 6: S-F's live half could not run

The throwaway patch gains a step (§3, Instruments; S-F): it writes the validation-fetched feed to
the scratch profile's `tmp\`. The controller counts it with F7 or a counts-only script and deletes it
in the same session. In the product, F8b writes it for F7 and deletes it after `school.preview`.

### 14.7 Important 7: task size

- F8 is split into F8a-F8d, F11 into F11t and F11a-F11d, S4 into S4a-S4d, and F6 into F6a and F6b.
- §2 and §7 state the cap: about 80 lines of code per file per task, or stop and ask for a split.

### 14.8 Important 8: assignment gaps

- S6's Rust (`school_status`, `school_forget`) is S6a, `implementer`, in `school_session.rs` beside the
  module it serves. S6t (`test-writer`) comes first; S6r (`mechanical`) registers the commands; S6b
  (`console-ui`) is the page.
- F10 is static files only. Shell-list removals are F5b (`mechanical`), after F10.

### 14.9 Minors

- **Test-first for `LaunchMode`, S6 and F11:** F1 test 10, S6t, F11t.
- **Merge order:** Q-B11 now states the departure from spec §9 and why (in code, O3 depends on O4),
  with A18 conditional on Quinn's acceptance.
- **Spike logging:** §3 limits records to counts, seconds, status codes and host kinds. Never a URL
  path or a cookie name.
- **Telemetry and the page:** Q-B10 and S7 make the page name the step event before F7b adds it. O4
  merges first, so the page is there in time.
- **Unknown campus preset:** step 2 writes no `campus.yaml` or `events.yaml` at all, so no "no school
  yet" preset is needed. F3 test 2 pins that `campus_yaml` is never called at step 2, and uncurated
  schools get `none` at `school.apply` (F6a).
- **"Needs sign-in" after Forget:** S3 test 8, and S10 checks grades, the registrar and school-mail
  repair.

### 14.10 What did not change

Every ruling (Q1, Q2, Q7, Q8, Q9, Q12, Q13), the step order, the reveal's cap, payment's place, the
contract-list boundary (only `account.rs`, by `contract-engineer`) and the passes the reviewer
recorded.

### Round 2 (§14.11-§14.17)

Report: `docs/reports/2026-10-23-wave-b-plan-review-r2.md` (filed from the review content in the
coordinator's message). 0 critical, 6 important, 5 minor. The holds (write-once, config never syncs,
the `none` preset, S3 test 7 and S4c, I5, I6, I8) are unchanged.

### 14.11 I1: a failed `school.store` lost the LMS feed silently

**Fixed as the reviewer proposed, plus a place for the link to wait.**
- New §4.11. A validated link is sent to the account by `school.store` (one retry, as
  `create_vault_in` does today). A link the account cannot take, or a rejected paste, is written into
  `ingest.yaml` by `config.ingest`, exactly as `build_into` plus `restore_capability_url` leave it
  today (R-C3'-exec-40/41). `ics_url: ''` is written only for a feed the account holds.
- `config.ingest` waits for both feed stores to settle, and Open Knowlu waits for `config.ingest`, so
  Open Knowlu waits for the store or its fallback.
- **Case (b), a quit after payment:** the link now waits in Credential Manager
  (`knowlu/<profile_id>/pending-lms_ics`, `pending-calendar_ics`) from capture to store, so a resume
  needs no school window at any school. Q-B2 changed from "memory only" to this.
- F2c deletes both targets on sign-out and deletion (`account.rs`, `contract-engineer`).
- Tests: F1 test 13 (a)-(e), F6b's feed tests, F2c. Ledger row "R-C3'-exec-40/41".

### 14.12 I2: the answers `config.ingest` needs did not survive a quit

**Fixed.**
- §4.5 adds `onboarding-answers.json`, with a table naming each answer, where it comes from and which
  job needs it: the course list, registrar rows, course slugs, the calendar flag, the feed states,
  and the book and section rows. Each is written the moment it is given.
- `onboarding.json` keeps no student content; the Architecture line that said it held "answers" is
  corrected.
- The answers file is deleted at finish, and by sign-out and deletion (F2c). An unreadable answers
  file re-asks its step and never runs a config job.
- Course names in app data before payment are Q-B15: recommended yes, since spec §1.4 B forbids notes,
  not app data.
- Tests: F1 test 12 (quit at steps 4, 6 and 8; byte-equal text, `course_map` never empty) and F1 test 1.

### 14.13 I3: Open Knowlu waited for the whole first slot

**Fixed as the reviewer proposed.**
- §4.10 now stops each job kind at its own safe point (a table). A slot stops at the next engine-step
  boundary: F8f adds a stop that `run_slot_inner` checks before each child (about fifteen lines in
  `scheduler.rs`).
- A slot stopped before `rank` leaves `state/today.md` absent, so `scheduler::spawn`'s first-run slot
  runs the rest after the relaunch and `ingest::is_first_run` still holds. T0 confirms App mode and
  `device_ok` for that path.
- `mail.canary` and `mail.first_pass` are abandoned at once (waiting writes nothing); `mail.setup`
  stops at its current recorded step.
- The wait left is one engine step, at most `CHILD_TIMEOUT`. S-E now records the longest single step;
  over 60 seconds goes to Quinn with two options (§3; Q-B16).
- Finish later, close and Sign out use the same safe points.
- Tests: F1 test 9 (finish mid-slot starts neither `judge` nor `rank`) and an `app/tests/scheduler.rs`
  test of the stop.

### 14.14 I4: `config.ingest` could sit behind the serial queue at step 10

**Fixed as the reviewer proposed.**
- §4.3 adds a local lane, itself serial, for `school.store`, `calendar.store`, `school.apply`,
  `school.courses`, `registrar.confirm` and `config.ingest`. They never wait behind a window or an
  engine step.
- A lane job that writes a note takes the console's existing hold against a running `sync`.
- Step 10 asks for proposals only once `config.ingest` is `done`; its class-time rows stay usable
  meanwhile (§4.2).
- Tests: F1 test 11 and F11t's step-10 pin.

### 14.15 I5: an empty restore on a down network could duplicate course notes

**Fixed as the reviewer proposed.**
- Step 2 records `account_settled` only when the restore reached the account (`Restored.ok`); a later
  `sync` can also settle it, read from its status record and not its exit code.
- Course notes moved from `school.apply` into a new `school.courses` job that waits for
  `account_settled`. It creates a note only where none matches (by path, `source_uid` or code) and
  records each course's slug: the restored note's own where one matched.
- `config.ingest`'s `course_map` uses those slugs.
- If the account is never reached, `school.courses` waits with Try again. It never guesses, and Open
  Knowlu waits for it (Q-B16).
- Tests: F3 test 5, F1 test 14, F6a's restored-notes test and F6b's `course_map` test.

### 14.16 I6 and the minors

- **I6, task size.**
  - F2 split into F2a, F2b and F2c.
  - F7 into F7a (`--preview` and `cli.rs`) and F7c (`--tidied`).
  - F8a into F8a (core), F8e (order and the local lane) and F8f (stop and drain), with F8c's slot
    runners moved to a new F8g.
  - F11a into F11a (the frame and rail) and F11e (steps 3 and 4).
  - Every implementer and console-ui task now carries a line estimate of about 80 or fewer, and F11b
    and F11c say which step moves out if one grows past it.
- **M1:** Q-B14 is now an allow list. The provider's hosts plus the curated row's new `sign_in_hosts`,
  which S-A phase 2 lists and S2 records in the row (never in a note). The allow list is possible
  because the shared folder exists only at a curated `shared_sign_in` row. S3 test 4 now refuses a
  syllabus host and another school-domain host. Quinn's alternative is to keep the deny list as an
  accepted risk.
- **M2:** S3 test 3 now says "queried host": the host of the URL passed to `cookies_for_url`, never a
  cookie's `Domain`. The fake jar returns a parent-domain cookie, so a reader keyed on `Domain` fails.
- **M3:** S3 test 2 passes the debug port as a parameter, and no test touches
  `KNOWLU_MAIL_DEBUG_PORT`.
- **M4:** §4.9 names the method: `std::fs::hard_link` from a temp file in `config/`, never `rename`,
  with `MoveFileExW` without replace as T0's alternative. F6a tests a file created mid-way, and F6b
  has F6a's partial-failure test.
- **M5:** two fixes.
  - §4.7: a restored vault has no `campus.yaml`, so its school is suggested from the account's stored
    feed's host and confirmed, never treated as proven.
  - §4.10: each hand-over item clears by its own signal; mail items clear from `mail.json`, never at a
    slot's end. F1 test 9 pins it.

### 14.17 What did not change in round 2

- Every ruling, the step order, the 3-minute cap, payment's place, write-once with no exception, and
  O4 merging before O3 with no tag between them.
- The contract-list boundary is unchanged: `account.rs` only, by `contract-engineer`. It is now
  touched by S5, S7 and F2c.

### Round 3 (§14.18-§14.22)

Report: `docs/reports/2026-10-23-wave-b-plan-review-r3.md` (filed by the controller). 0 critical,
3 important, 6 minor; the three importants came from round 2's own fixes. The checked-and-holding
list (credential names, spec §1.4 B, routing, local tests, frozen references, single-user, vault
writes, the ledger) is unchanged. Each answer below takes the reviewer's suggestion where it gave one.

### 14.18 I-A: the stop flag was cleared at slot start

- §4.10 now defines the stop as one `AtomicBool` on `Scheduler` that **nothing clears**. Onboarding's
  drain sets it; every way out of onboarding ends the process, so it never needs resetting; the normal
  console never sets it.
- The race the reviewer named (dequeue, then stop, then a clear at slot start) cannot happen: a stop
  set before the slot stops it before its first stage.
- F1 test 15 pins it: the stop set before the slot starts no stage, it is still set after the slot,
  and a second slot on the same `Scheduler` also runs nothing.
- T0 confirms that Sign out relaunches into the shell and that `Scheduler` is managed in onboarding
  mode.

### 14.19 I-B: the lane's note writes collided with `slot.partial`'s `sync`

- §4.3 adds a **notes gate** in `onboarding_jobs.rs`. Lane note jobs (`school.courses`,
  `registrar.confirm`) and slot jobs each hold it for their run, and each side **blocks** rather than
  refuses.
- Behind the gate, a lane job still takes `run_confirm`'s hold, retrying on `sync.lock` for up to 30
  seconds before it fails with its fix. That covers another process only.
- Which side goes first follows `account_settled`: when false, the sync runs first, because it is
  what settles it.
- **A skipped `sync` is not a sync** (the reviewer's suggestion). Neither the gate's "first sync" nor
  `account_settled` counts a step named `sync (skipped: …)`.
- F1 test 16 starts the two together and checks they never overlap, that neither fails, and that the
  sync is not skipped. It also checks that a skipped sync settles nothing.

### 14.20 I-C: Sign out left `onboarding.json` pointing at deleted answers

- §4.5 adds two rules:
  - Sign-out rewinds an unfinished `onboarding.json` to step 3, with every job `pending` and the feed
    states and `account_settled` cleared. A finished file is untouched.
  - An **absent** answers file past step 3 is handled like an unreadable one: its step is re-asked,
    never read as "no answers".
- New F2e implements `onboarding_state::on_sign_out`: it deletes the answers file and both pending
  links, then rewinds.
- F2c's `account.rs` diff is two edits, per the reviewer's suggestion:
  - one call to `on_sign_out` from `sign_out`'s existing `clean_unfinished` closure;
  - the two pending names added to `delete_local_data`'s credential loop. Its
    `remove_dir_all(data_dir)` already removes the answers file.
- F1 test 17 signs out at step 8, signs in again and resumes at step 3, with no config job before new
  answers exist. It also covers an answers file deleted out of band, and checks that a finished file
  is unchanged.

### 14.21 Minors

- **M1, the stop between engine steps only, and a double mail step.**
  - The stop is checked before every stage `run_slot_with` starts: session, entitlement, privacy,
    grades, mail, each engine child, backup and telemetry. Q-B16, S-E and the ledger now say "one slot
    stage at most".
  - F8g no longer adds a mail step after `run_slot_inner`, which already runs one before the engine
    steps.
  - `slot.partial` runs its own child loop with the same check.
- **M2, hard links at run time.** §4.9 and F6a fall back at run time to `MoveFileExW` without
  `MOVEFILE_REPLACE_EXISTING` when the hard link fails for any reason other than `AlreadyExists`. If
  both fail, the job is `failed` with a fix that names the volume. F6a tests the fallback through a
  seam.
- **M3, a refused host in the mail window.** It no longer dead-ends. The mail window on the shared
  folder closes, and the school mailbox falls back to its own `mail-session-<id>` folder (today's
  path), records no `school_mail` use and logs the host's kind. This is in Q-B14 and S4c, and S3 test 4
  pins the decision.
- **M4, line estimates.** F2a is split into F2a (`onboarding.json`, about 75 lines) and F2d (the
  answers file and pending names, about 60). F2b, F2e, F3, F4 and F9 now carry estimates. F8e is
  about 80, and if the gate pushes it past that, the gate becomes its own piece.
- **M5, test order.** The scheduler stop test moves from F8f to F1 (test 15, `test-writer`).
- **M6, privacy.** S7 gains a sentence naming the waiting feed link in Credential Manager and the
  answers file, with their retention: deleted at finish, sign-out and deletion, and kept by an
  abandoned setup until then. Q-B15 records the choice and declines an age-based sweep. The sentence
  ships with O4's page version because no tag falls between the two merges; if O3 slipped to a later
  release, it would move to F13 with its own version bump.

### 14.22 What did not change in round 3

- Every ruling, the step order, the 3-minute cap, payment's place, write-once with no exception, and
  the merge order.
- The local lane, the answers file and the pending links stay as round 2 set them, with round 3's
  guards added.
- The contract-list boundary: `account.rs` only, by `contract-engineer` (S5, S7, F2c), and F2c's
  diff is now about ten lines.

### Round 4 (§14.23-§14.27)

Report: `docs/reports/2026-10-23-wave-b-plan-review-r4.md` (filed by the controller). 0 critical,
3 important, 6 minor. Every fix below is the reviewer's own unless noted, checked against the code
(§13's round-4 reads). The reviewer's "checked and holding" list stands: the `MoveFileExW` fallback,
F2c's routing, frozen references, no single-user assumptions, and test order.

### 14.23 I-1: Sign out and Finish later had no command, drain or relaunch

- **Three exit commands in F2b** (§4.10's new table), each over seams so F1 drives them with fakes.
  `relaunch` only spawns, so each command ends the process itself:
  - `onboarding_quit` (Finish later, and the window's close via F4): stop, drain, save, exit.
  - `onboarding_sign_out`: stop, drain, save, then **freeze saves**; then `account::sign_out`,
    `relaunch(["--pick"])`, exit.
  - `onboarding_finish`: as before, now ending in an explicit exit.
- **The freeze** (F8f) means the live queue can never write its in-memory step over the rewound file.
- **Sign-out rewinds to step 2**, not 3 (§4.5). Onboarding mode gains step 2's sign-in without the
  vault creation (§4.2, F11e).
- **A profile with no session always resumes at step 2** (F2a), so a session lost any other way also
  lands on a sign-in. The account must be the vault's own.
- The two commands join F5's list and F11t's allowed list. F11t pins that Finish later and Sign out
  call them.
- F1 test 17 now drives the whole command and shows that no save lands after the rewind. F1 test 10
  covers the no-session rule.
- Q-B17 records the choice between step 2 in onboarding mode and the shell's sign-in.
- T0's round-3 checks the reviewer answered from the code are marked answered. T0 now names the
  re-sign-in path step 2 reuses.

### 14.24 I-2: a skipped sync is always named `sync`, exit 0

- §4.3 now reads "ran" from the **status record**: `cs.sync`, filled from `state/sync-status.json`
  by `state::refresh_sync` at the slot's end.
  - It must show a run stamped at or after the slot's start, with no `skipped` reason.
  - `account_settled` also needs `ok`.
- F1 test 16's fake is now a status record: skipped, stale, and fresh with `ok`. F8e implements it.

### 14.25 I-3: `slot.partial`'s own child loop named nothing

- `slot.partial` now goes through the slot itself. F8s adds `run_slot_scoped(…, SlotScope)`;
  `run_slot_with` and `run_slot_inner` become wrappers passing `Full`, so none of the eight existing
  call sites changes.
- `NoRank` names the step it drops: `rank (skipped: the first full slot ranks)`.
- So `slot.partial` gets the slot's own machinery: the named `grades`, `ingest` and `judge` skips,
  the runner-log lines, the privacy hold, the session refresh and the stop checks.
- F1 test 18: with no bundle there is a named `grades (skipped: …)` row and its log line; `rank` is
  named and no `state/today.md` is written.

### 14.26 Minors

- **M1, the grades bundle on a stop.** The bundle's delete runs on every path: a stop skips stages,
  never that cleanup (§4.10, F8s). F1 test 15 asserts the file is gone.
- **M2, holding `vault_io` while retrying.** Each try on `sync.lock` takes and drops `vault_io` and
  the lock, and neither is held while sleeping (§4.3, F8h). F1 test 16 checks a `vault_io` user
  gets it within one interval.
- **M3, F2c's test could not reach its edit.**
  - F2c adds `pub fn account::sign_out_clean(dir, id)`: today's mail clean plus `on_sign_out`.
    `sign_out`'s closure calls it.
  - The tests call `sign_out_clean` itself, under `CREDMAN_LOCK` with generated ids and `Drop`
    cleanup.
- **M4, F8e too big.** The notes gate is split out now as F8h (about 45 lines), leaving F8e at about
  65. F8f's scheduler half is also split, as F8s (about 45).
- **M5, a failed relaunch.** It returns `{ok: false, restart: true}`. The restart screen's only
  button calls `onboarding_quit`, which exits. The state already written stands (§4.10). Covered by
  F1 test 19 and F11t.
- **M6, other console exits.**
  - New F4b makes `switch_profile` refuse in onboarding mode.
  - F11t pins that no onboarding screen calls `switch_profile`, `delete_my_data` or
    `account::sign_out`.
  - `delete_my_data` (in `account.rs`) is not edited, because nothing in onboarding mode can reach it.
  - F1 test 20.

### 14.27 What did not change in round 4

- Every ruling, the step order, the 3-minute cap, payment's place, write-once with no exception, the
  merge order, the local lane, the answers file and the pending links.
- The contract-list boundary: `account.rs` only, by `contract-engineer` (S5, S7, F2c). F2c's diff is
  now about twenty lines, adding `sign_out_clean`.

### Round 5 (§14.28-§14.34)

Report: `docs/reports/2026-10-23-wave-b-plan-review-r5.md` (filed by the controller). 0 critical,
5 important, 5 minor. Every fix is the reviewer's own unless noted, checked against the code first.
At the controller's direction this round also ran a **reuse audit** (§13.1): 52 rows, of which 11 did
not hold (the seven behind I-1 to I-5, and four new, §14.33) and 6 held with a brief tightened.

### 14.28 I-1: payment and the hold read a session step 2 had moved

- New **F2h** (`implementer`, reviewed by `contract-reviewer`), in `onboarding_state.rs`:
  - `onboarding_entitlement` is `account::refresh_entitlement(vault, data_dir)`, which already reads
    `cloud.yaml`'s target and saves the cache;
  - `onboarding_checkout` is `open_portal`'s shape with `checkout_url_at` in place of the portal call.
  - No `account.rs` edit: every function used is public.
- **F8a's hold** tests `scheduler::entitlement_state(cs)`, woken by the poll and refreshed once at the
  queue's start after a relaunch (§4.3).
- Step 4 names the two commands (§4.2). F11e calls them, F5 registers them, and F11t pins that no
  screen calls `entitlement_now` or `open_checkout`.
- F1 test 22 runs on a `127.0.0.1:0` listener with the existing `KNOWLU_API_BASE` guard, because
  `cloud_config` refuses any other host.

### 14.29 I-2: "step 2 again" had no Rust task and no path to reuse

- New **F2g**, `onboarding_attach_session(account_id)` (`implementer`, reviewed by
  `contract-reviewer`).
  - Its inner takes the pending target as a parameter, as `attach_in` does, so tests never touch the
    machine's real `knowlu/pending/session`.
  - It needs three account ids to agree: the pending session's, the sign-in's answer, and
    `cloud.yaml`'s. The sign-in's answer is the third check, so a stale pending session left by
    someone else is never moved.
  - On agreement it calls `move_session` to the target `cloud.yaml` names.
  - On any refusal it calls a forget seam, so no other account's token stays on the machine.
    *Corrected in round 6 (§14.35):* the seam is a local `credentials::delete`, never
    `sign_out_with`, whose unscoped `/logout` revokes the account everywhere.
- T0's check is now "confirm none exists" (§T0). F1 test 21 covers the cases. F11e shows the
  refusal and the way to the picker. Q-B17 notes that the shell alternative would need the same
  function.

### 14.30 I-3: the scope sat one layer too low

- **F8s now adds two scoped functions:**
  - `run_slot_inner_scoped(cs, sch, tray_app, late, scope)` builds the seams from the handle;
  - `run_slot_scoped` is the body.
- **F8g** passes the queue's own `AppHandle` to both slots. With no tray, `tray::set_state` is a
  no-op.
- F1 tests 15 and 18 stay at the seam layer. Test 18 now also shows the seams run under `NoRank`.

### 14.31 I-4: a closed backfill batch started a slot outside the queue

- New **F8m** (`implementer`, about fifteen lines in `mail/live.rs`). In onboarding mode, `on_closed`
  tells the queue and starts nothing; the next queued slot that runs `judge` pulls the batch.
- §4.3 names all four `run_slot` callers and why none fires in onboarding mode. F1 test 23. T0
  recounts the callers.

### 14.32 I-5: the close would have drained on the event loop

- **F4:** `api.prevent_close()`, then `onboarding_quit`'s inner on a spawned thread, returning at once.
- **F2b's three exits** are `(async)` commands guarded by one `AtomicBool`, so a second close does
  nothing.
- F1 test 24.

### 14.33 Minors, and what the audit found

- **M1, the freeze.** It is a flag on the queue's saver (F8f), never F2a's save, so F2e's rewind
  lands (§4.5).
- **M2, "sync ran":**
  - it compares against `RunSummary.started`;
  - a fresh `ok: false` (an error, or the engine's entitlement refusal) is `failed` with its fix;
  - a lock-held sync writes no status, so the stale-stamp rule catches it (§4.3, F1 test 16).
- **M3, the runner log.** Scope rows are named `<step> (not yet: <why>)`, which the filter at
  `scheduler.rs:1036-1037` does not match, so no runner-log line is written (§4.3, F8s, test 18).
  `rank`'s row was `rank (skipped: …)` in round 4 and is renamed to match.
- **M4, call sites.** There are 35 calls of `run_slot`, `run_slot_inner` and `run_slot_with` (28 in
  `app/tests/scheduler.rs`, 7 in `app/src`; 38 matches less the three definitions), all unchanged by the
  wrappers. §14.25's "eight" was wrong.
- **M5, F2c's tests.** `sign_out_clean` takes `api_base` as a parameter, as `sign_out_mail_client`
  already does. Its tests point it at a local listener (F2c).
- **The audit's four new findings:**
  1. **Step 6** named no command, and the shell's Google pair reads the pending target. Step 6 now
     uses the vault-target `google_connect` and `google_status`, and F11t pins the pair as uncalled.
  2. **`slot_argv`** always runs `coursework`, and on a cloud vault `ingest`. So the slot before
     `ingest.yaml` exists is `BeforeConfig` (Q-B19), which leaves out `coursework`, `ingest`, `judge`
     and `rank` by name.
  3. **Today's quit path.** The exits skipped `quit_at` and the 10-second push and backup. Finish
     later, the close and Open Knowlu now take it; Sign out does not (Q-B18, F1 test 25).
  4. **`load_coursework_config`** reads `<vault>/config/ingest.yaml` and answers empty when it is
     absent, so F6b checks inside a scratch root and tests a malformed text.
- **Tightened where the code required:**
  - F3 test 1 asserts `runners.yaml` gives App mode and this device;
  - F8g treats a refused summary as `failed`;
  - F11c shows step 10's "a sync is running" refusal and its Try again;
  - F1 tests 21-22 carry the Credential Manager sign-off note.

### 14.34 What did not change in round 5

- Every ruling, the step order, the 3-minute cap, payment's place, write-once with no exception, the
  merge order, the local lane, the notes gate, the answers file and the pending links.
- The contract-list boundary is still `account.rs` only, by `contract-engineer` (S5, S7, F2c). F2g
  and F2h call only public `account.rs` functions. They go to `contract-reviewer` because a wrong move
  or target would attach or leak a session.

### Round 6 (§14.35-§14.38)

Report: `docs/reports/2026-10-23-wave-b-plan-review-r6.md` (filed by the controller). 0 critical,
2 important, 4 minor. All ten round-5 findings are closed, and the reviewer's spot-check of §13.1
confirmed 26 of 26 rows. Each fix below is the reviewer's own, checked against the code first
(`account.rs:776-781`, `lms_link.rs:397-434`, `:801-885`, `console.js:1531-1544`).

### 14.35 I-1: F2g's forget would sign the other account out everywhere

- F2g's `forget_pending` is now a **local delete only**, `credentials::delete(account::PENDING_TARGET)`,
  with no revoke. `sign_out_with`'s `/logout` carries no `scope`, which GoTrue reads as `global`: on a
  shared PC it would have signed a sibling's account out of their finished profile and every other
  desktop, and on an unreadable `cloud.yaml` the student's own. The token still leaves the machine.
- §4.2 step 2 again, F2g, §13.1's `sign_out_with` row and §14.29 say so. F1 test 21 adds a case that
  runs the real local delete over a generated target, plus a source pin that F2g names neither
  `sign_out_with` nor `/logout`. No `account.rs` edit.

### 14.36 I-2: the capture and paste commands stored the link under the pending session

- Today's `capture_calendar_link` and `paste_calendar_link` end in `lms_link::finish`, which sends
  the validated link straight to the account under `PENDING_TARGET` (`lms_link.rs:843-847`). By
  step 3 that target is empty, or holds another sign-in parked on this machine, whose account would
  then receive the student's bearer link.
- New **F8l** (`implementer`, reviewed by `contract-reviewer`): `onboarding_capture_link` and
  `onboarding_paste_link` validate through `finish_with` with a store seam that sends nothing
  (`stored: false`, with a note), then write the pending credential. `capture_calendar_link`'s body
  is split to take its finish (about ten lines in `lms_link.rs`, after S4b); the shell's commands
  are unchanged.
- F5 registers the onboarding pair and never the shell's pair. F8b's capture uses F8l's seam.
  §4.2 steps 3 and 7 and §4.11 name the new commands. F11t pins that no screen calls the shell's
  pair. F1 test 26: a local listener receives no request, and source pins back this up.
- §13.1 gains the `lms_link::finish` and `store_source` row ("No").

### 14.37 Minors

- **M1, `(not yet:` read as done.** §13.1 names the reader (`console.js:1542`). F11a teaches it
  `(not yet:` as skipped (one line, so `console.js` joins F11a in §6), and F11t pins both that and
  that no onboarding screen renders slot rows.
- **M2, file-disjointness.** F2g and F2h are now chained after F2b in `onboarding_state.rs` (F2a,
  F2d, F2e, F2b, F2g, F2h). F8l comes last in `onboarding_jobs.rs`'s chain.
- **M3, a batch after `slot.first_full`.** §4.3 says such a batch waits for the first scheduled slot
  after the reveal. The rail's and the reveal's `older_mail` line says so (F11a, F11d).
- **M4, the tally.** §13.1 now has 54 rows: 12 "No", 7 "Holds, tightened" and 35 "Holds". The
  reviewer's count was 53/12; the 54th row is `renderFirstRun`'s reader, added for M1.

### 14.38 What did not change in round 6

- Every ruling, the step order, the scopes, the exits, the notes gate, the answers file and the
  pending links. The contract-list boundary is still `account.rs` only (S5, S7, F2c). No open
  question changed.

### 14.39 Round 7 (`docs/reports/2026-10-23-wave-b-plan-review-r7.md`)

0 critical, 1 important, 4 minor; every round-6 finding confirmed closed. The reviewer's fixes:
- **I-1, F8b was built before the seam it needs.** F8l is split:
  - **F8la** comes before F8b, in `lms_link.rs`. It adds `capture_calendar_link_with`,
    `validate_only` and `capture_link_only`.
  - **F8lb** comes last in `onboarding_jobs.rs`'s chain and adds the two commands.
  - F8b depends on F8la and calls `capture_link_only`. It is now reviewed by `contract-reviewer`.
  - Test 26's source pin now covers `onboarding_jobs.rs` and `onboarding_state.rs`, and adds
    `capture_calendar_link(` and `paste_calendar_link(`.
  - The r6 I-2 ledger row now names F8la, F8b and F8lb, and §13.1's row and §4.11 name the seam.
- **M1.** F11e and F11b depend on F8lb.
- **M2.** F2b depends on F2e, so the `onboarding_state.rs` chain is enforced.
- **M3.** F8l's pin text uses `lms_link::finish(`, as test 26 does.
- **M4.** F2g ignores `credentials::delete`'s not-found result, so the no-pending case answers the
  named refusal. Any other delete error is logged, and the refusal stands.

Nothing else changed: no ruling, no open question, and no contract-list file beyond `account.rs`.

### 14.40 Round 8 (`docs/reports/2026-10-23-wave-b-plan-review-r8.md`): PASS, minors closed

0 critical, 0 important, 3 minor, one note. Every round-7 finding was confirmed closed. Closed here,
with no further round (the controller's direction):
- **M1, the pin forbade F8c's correct call.**
  - F8c's store is now named: `lms_link::store_source(&cfg.api_base, &cfg.session_credential_target,
    …)`, the vault's own session, in one runner function.
  - Test 26's pin is narrowed to what is actually wrong. `store_source(` may appear only in that
    runner, and never beside `PENDING_TARGET`. `PENDING_TARGET` appears nowhere in
    `onboarding_jobs.rs`, and in `onboarding_state.rs` only inside F2g.
- **M2.** The F8l pin text's ban on `PENDING_TARGET` is scoped to F8lb's two commands, so F2g still
  passes.
- **M3.** A source assertion over `lms_link.rs`: `capture_link_only` names `validate_only` and never
  passes `finish` to `capture_calendar_link_with`. It is stated in F8la and in test 26.
- **Note.** F9 moves to its own file, `app/src/onboarding_gate.rs` (§6, §7), so it no longer runs
  beside the `onboarding_jobs.rs` chain in the same file.

Status: reviewed, PASS (r8). The plan now waits on Quinn's rulings (Q-B3, Q-B10, Q-B11, the Q-B14
alternative, the direction) and on T0.

### 14.41 Quinn's rulings (2026-10-23): ruled, ready for T0

Recorded in §5.0. The task edits each ruling needs:
- **Q-B3.** §9 A11 now carries the `CLAUDE.md` line's text; F7a adds it in the command's own commit.
- **Q-B10.** F7b is no longer conditional. It still waits for S7's privacy page to name the event.
- **Q-B11.** A18 is recorded as ruled.
- **Q-B14.** The allow list, with its fallback (S3, S4c, unchanged).
- **The direction.** F10, the F11 common brief and S6b point at `5b-departures-board/`'s
  `index.html`, `screens.json` and `BRIEF.md`, with the README's grafts. A new T0 item makes those
  files reachable from the lane branches, since they are untracked in the main checkout.
- **Q-B15 to Q-B19** stand as the controller decided. Q-B1 stands at "no exception".
- **The evening ranking rule** is out of wave B. Its draft note is
  `docs/specs/2026-10-23-evening-ranking-rule-draft.md`.

### 14.42 T0 (2026-10-24): what did not hold, and the amendments

T0 ran on `feat/o4-school-sign-in` at `1d3c467`; its record is in §13 (2026-10-24). Part B found
three one-line drifts, corrected in place. Part A found seven premises that did not hold, amended
as follows (docs only; no ruling reopened):
1. **Lanes not merged** (L5, O2, L1; no branches). §8 says O5 waits for L5; F11b no longer depends
   on O2 and builds step 6 on today's separate consents; a new task **F11f** (`console-ui`, after O2
   merges, possibly after wave B) adds the combined ask and the forwarding fallback; the §1.3 Q8
   ledger row moves to F11f. *Trade-off for Quinn and the controller:* if wave B ships before O2,
   step 6 asks twice (Calendar, then Gmail) rather than once.
2. **`grades` needs course notes.** `BeforeConfig` now leaves out `grades`, the capture stage with
   the engine step, as one `(not yet: …)` row, so a bundle is never captured only to be deleted;
   `NoRank` waits for `school.apply` as well as `config.ingest`, and runs the vault's first
   `grades` before its first `ingest`, as O1's order wants. The gate's "first grades" is marked
   only by a slot whose `grades` ran (§4.6). §4.2 step 4, S-E's note, F8s, F8e, F8g, F1 tests 8 and
   18 and Q-B19 are amended. The step-3 proof's grade count is unaffected: it comes from the raw
   bundle via F7a, not from the engine step.
3. **The entitlement-refused sync is a skip.** It reads as "neither" (§4.3; §13.1's row corrected),
   which the hold makes unreachable in practice. `account_settled` now also needs `pulled_to_end`,
   read only. **New, for the controller (Q-B20):** a sync pulls one page per run, so the queue
   re-runs `BeforeConfig` while the cursor is short of the end, up to 10 in a row, then fails
   `school.courses` with "Try again". Without it, a multi-page account whose step-2 restore failed
   would stall onboarding. F8e, F8g, F1 test 16 and the ledger carry it.
4. **No host from the account.** §4.7's suggestion comes only from this device's `onboarding.json`;
   otherwise step 3 asks as on a fresh install. No cloud change; the §1.11 ledger row is amended.
5. **The sync hold's takers.** §4.3 now names `run_confirm` and `registrar::spawn` only, and says
   `commands::mutate` takes `vault_io` only. The lane's jobs are unchanged, with one rule the
   planner added while reading `registrar.rs:96-104`: the lane never takes `vault_io` or `sync.lock`
   around `registrar::spawn`, which takes both itself (a nested `std` mutex deadlocks; a second
   `try_lock` handle refuses), and retries that call on its refusal instead (§4.3, F8h, F1 test 16).
6. **`mode_device` in onboarding mode.** F8s adds `prime_mode_device` (`spawn`'s own line, moved
   into a function), F4 calls it once, and F1 test 27 pins it. The rest of what skipping `spawn`
   skips (`quit_at`, `startup_missed`, housekeeping) is recorded in F4 as benign. §13.1 gains this
   row and a `pulled_to_end` row (56 rows: 13 "No", 7 "Holds, tightened", 36 "Holds").
7. **`campus.yaml` mid-slot.** §4.3 notes the named skip, and that the queue's `NoRank` rule keeps
   it to a retried `school.apply`.

Still open from T0: the ruled direction's files on the lane branches (before F10, F11a-F11f or
S6b), and Q-B20. Next: T1.

### 14.43 The T0 amendments' review (2026-10-24): PASS with minors, closed

The review passed; the plan went to `main` as `cb3b661`. The controller's calls on its minors, and
two new facts, each edited where it lands:
- **M1 (Q-B20 decided).** The `BeforeConfig` re-run is bounded by progress, not by 10: it continues
  while `record_cursor` or `note_cursor` moved, stops on a fresh `ok` run where neither did, and has
  a backstop of 50 runs, because a page is at most 500 records (`MAX_PAGE`,
  `cloud/supabase/functions/sync-pull/handler.ts:5`) and 10 runs would read about 5,000. Between
  re-runs, a skipped or stale reading counts toward the backstop, is not progress, and enqueues one
  more run (two in a row stop the chain); a failed reading stops it at once. "Try again" on
  `school.courses` while unsettled resets the count and enqueues one `BeforeConfig` slot (§4.3, Q-B20,
  F8e, F1 test 16).
- **M2.** F8h matches `registrar::spawn`'s refusal on `week::SYNC_RUNNING` (`week.rs:101`, already
  `pub` since F1's `8cc0caf`, so no `week.rs` edit; corrected in §14.45), never on a copied string;
  test 16's fake refuses with the same constant.
- **M3.** One update check at an onboarding-mode launch. The check lives only in the housekeeping
  tick (`scheduler.rs:1284`) and Check now (`commands.rs:729`), and `commands::updates_dir` is
  `pub(crate)`, out of `main.rs`'s reach; so F8s moves the tick's body into
  `scheduler::spawn_update_check(app)`, the tick calls it, and F4 calls it once (F1 test 27's pins).
  **Open for the controller:** onboarding has no install path, so the check stages an update that
  only the normal console can install; a student who never finishes onboarding still runs the old
  version.
- **M4 (Q-B21, ruled by Quinn 2026-10-24).** If wave B ships before O2, step 6 asks for Google
  consent twice (Calendar, then Gmail) until O2 and F11f combine them. Quinn: ship it that way (§5).
- **M5.** §4.7 now says plainly that on a new device step 3 always asks for the school. Sign-out
  clears `school` from `onboarding.json` with the rest of the rewind (§4.5, F2e, F1 test 17), so a
  stored school is offered only on a resume without a sign-out. F1 test 2 covers the no-school arm,
  the stored-school arm and the rewound file.
- **Okta, not Duo.** The school's MFA is Okta at the founder's school (Quinn, and S-A phase 1); §2
  and §3 now say "the school's MFA (Okta at the founder's school)".
- **S-A phase 1** (first half) is recorded in §13: on a fresh Edge profile, the LMS asked for a
  password and "stay signed in?" with no MFA prompt; Outlook then came up at once with no prompt; the
  registrar in about 1 second with no prompt. The 10-minute restart check passed (all three signed in); S-A phase 1 PASS.

### 14.44 F10's review (8292301 on `feat/o3-f10`): §9 A10's unassigned C1 rules

F10's review found that §9's row A10 claimed C1's rulings "carried" while their page halves, and
three rules held only by tests F10 deleted, had no later task. Each now has one, before F11 is
dispatched (§4.2 steps 3-8, the tasks named, A10 rewritten with its carrier per ruling):
- **R-C1-40, page half:** `close_lms_window` on every exit from step 3 for non-shared windows
  (F11e1, F11t).
- **R-C1-42:** the capture on the second press only, never on open, never on the identity
  provider's page; on the shared folder the signed-in test is the trigger (F11e1, F11t).
- **R-C1-51, UI half:** chosen as a removable list rather than a filter, since nothing on the
  device can tell an organisation or a TA role from a class with certainty. Every captured course is
  listed at step 3's proof with Remove; `removed` lives on the answers file's course row (F2d);
  `school.courses` and `course_map` use `kept_courses()` only (F6a); and so that the page computes
  no count and the feed is not kept, F7a's preview also reports `by_course`, and Rust totals the
  kept rows (F7a, F8b, F11e2; F1 test 12, F6a's test). **F11e splits** into F11e1 and F11e2 to stay
  under the size cap.
- **R-C1-55:** discovery in flight disables Next (F11c, F11t).
- **R-OB-1:** a blank row is never sent as ignored; no slug is made on the page (F11c, F11t).
- **R-OB-4:** "my school isn't listed" at step 3, to the paste fallback; `school.apply` writes the
  no-unitid school as today (F11e1, F6a's test).
- **From the deleted tests:** mail risk 12's line at step 5's skip and `GOOGLE_TESTING` at step 6
  (F11b, F11t); the password check, now an allow-list by id that F11c extends and that is never
  deleted (F11t).
- **F5b:** five commands move to the console's list and are never deleted; four of the shell's
  pending-target commands may go from the shell's list.
- **F3:** Rust picks "Knowlu", "Knowlu 2", … past profiles and folders on disk; the old flow's
  "press Finish again" and "pick another name" errors are rewritten; F10's interim name loop is
  deleted (F3 tests 6-7; F3 now depends on F10).
- **Payment:** a `comp` account sees "nothing to pay", never the trial's terms (F11e2, F11t).
- **Merge gate:** `feat/o3-long-onboarding` never merges to `main` before F12's walk-through passes
  (§8; F12 walks the new cases; F14 checks it).
Two choices were the planner's and are open to the controller: the removable list over a filter, and
`by_course` (an engine output change in F7a) over keeping the feed file until payment.

### 14.45 A removed course's feed items, and F8h's constant (2026-10-24)

From the F7c review (`docs/reports/2026-10-24-wave-b-f7c-review-r1.md`, on `feat/o3-f7a`): its
"outside this diff" note and Important findings 2 and 3, with the controller's calls.
1. **A removed course hid nothing.** R-C1-51 kept a removed course out of `course_map`, but ingest
   still imported its feed items as course-less tasks (`ingest.rs:883` keeps an unmatched item), so
   the removal hid nothing and step 3's kept totals under-counted the vault. **The controller's
   call:** an item ingest's shared rule would attribute to a removed course is not imported.
   - **Where the list travels.** The handoff that carries `course_map` is `config/ingest.yaml`,
     written once by `config.ingest` (F6b's builder, run by F8c), not by F8b. So **F6b** writes the
     removed courses' pairs as `course_exclude` (absent when nothing was removed, so today's bytes
     hold), and F8b only saves each course's pairs in the answers file. A separate key rather than
     marked `course_map` entries, because `commitments`, `enrich`, `cloudmodel` and `judge`'s tier 1
     all read `course_map` too (`commitments.rs:769`, `cloudmodel.rs:340-351`, `ingest.rs:258-269`),
     and none of them should see a removed course.
   - **The ingest side is new task F6c** (`ingest.rs`, off the contract list; test-writer then
     implementer, `reviewer` with the oracle runs), its first test being that a removed course's item
     never becomes a task; F1 test 12 carries the app half.
   - **One refinement the call needed, and the smallest alternative to a per-course bucket.** A
     plain "first match over the whole list, then drop if that course was removed" would, for a
     past-term shell and the current shell of one course that both claim an item, drop the live
     course's item whenever the old shell is removed, the very case removal exists for. So ingest
     matches over `course_map` first and over `course_exclude` only when nothing kept claims the
     item: **a kept course always wins.** A static per-course bucket cannot total that once the
     removal set changes, so F7a's buckets are keyed by the **set** of codes that each claim the
     item (matched one course at a time with `ingest::match_course`): one code in the usual case,
     which is that course's own bucket as the controller asked, two for a shared item, none for
     `unmatched`. The app totals every bucket that is empty or names a kept code, which equals what
     F6c imports. If the controller prefers strict per-course buckets, the price is that a shared
     item is miscounted when one of its courses is removed.
2. **`by_course` keys on the LMS course `code`** (review Important 2), falling back to the slug and
   then the label only when `code` is empty (F7a).
3. **One attribution rule** (review Important 3): F7a and F7c call ingest's shared rule
   (`ingest::match_course`, over `match_course_fields`), with pairs built as F6b builds
   `course_map`, uid pins included; no prefix rule, order precedence or bundle fallback of their own
   (F7a).
4. **F8h does not touch `week.rs`.** `week::SYNC_RUNNING` is already `pub` (F1, `8cc0caf`); F8h and
   §14.43's M2 are corrected.
Tasks changed: F6b (the key and its test), F6c (new), F7a (`by_course`), F8b (saves each course's
pairs; totals by match set), F8h, F1 test 12; §4.2 step 3, §4.9's `ingest.yaml` row, §6, §7 and §10.
**What removal never touches** (added 2026-10-24, the controller's G5; §14.47): class times. The
registrar holds only the current term, so every class time is counted in every total and imported
whatever the student removes; `course_exclude` and the kept totals govern feed items (and, by
`graded`, the grades count) only.

### 14.46 Shared fragments, the pin form, one source of pairs (2026-10-24)

From the F7c review, round 2 (`docs/reports/2026-10-24-wave-b-f7c-review-r2.md`, "Outside this
diff" and Minor 1), with the planner's choices:
1. **A fragment two courses share, one of them removed.** `course_fragments` gives a lecture and its
   lab the same human code. F6b now puts a fragment in `course_exclude` only when no kept course
   also contributes it (a test with two enrolments, one removed). F6c's kept-first match already
   made such an entry harmless, so this is defence in depth and keeps the file honest.
2. **The pin form.** A gradebook uid pin's fragment is the column id exactly as the grade note
   keeps it in `source_uid` (`_<digits>_<digits>`), never with the `GradableItem-` prefix and never
   partial. It goes in `course_map` for a kept course and `course_exclude` for a removed one (F6b).
   To stop `_71_1` claiming `-_71_10` or `-_171_1`, F6c makes ingest's uid pass **bounded for
   pin-form fragments only** (no ASCII letter, digit or `_` before, no digit after); every other
   fragment keeps the substring test, `ingest.rs:1505`'s pin test passes unedited, and the oracles
   must stay unedited and green, or F6c stops. Because the rule is shared, `judge`'s tier 1 and F7a
   get the bound too. F6c's reader tests add that a pin key in `course_map` gives `commitments` no
   new claimant. *Open for the controller:* this changes the shared rule in every vault, not only
   onboarding's; the alternative (bounding only `course_exclude`'s pins) would leave the collision in
   `course_map` and attribute kept and removed items by different rules.
3. **One source of pairs (Minor 1): the first option.** F8b builds each course's pairs once at
   capture (`course_fragments` and the bundle's column ids), saves them, and passes them to
   `onboarding --preview` in `--courses`; F7a uses them and no longer re-derives from the label. An
   edited label is display only. F1 test 12 checks the pairs are byte-equal in `--courses` and in
   `config.ingest`'s text. Pinning `lms_link`'s label to `course_fragments` was not chosen, because
   a typed label (the paste fallback) has no `lms_link` origin to pin.
4. **Also from round 2** (its Important 1, for F7a's next round): class times are bucketed by match
   set like feed items (F7a's tests).
Tasks changed: F6b (shared-fragment rule; pin lines; tests), F6c (bounded pin pass; tests; reader
test for pins; oracle stop rule), F7a (pairs from `--courses`; class times by match set), F8b (one
source of pairs, passed to `--preview`), F1 test 12 (pairs byte-equal).

### 14.47 The grades count and class times on removal (2026-10-24, the controller's calls)

1. **Per-course grades.** Step 3's `grades_courses` ("grades for N courses") could not be totalled
   over kept rows, because `by_course`'s buckets carried no grades signal. Now every course in
   `--courses` has its own single-code bucket (present even with zero counts), carrying
   `graded: true` (the bundle has a graded item column for it), `false` (the bundle has none for
   it) or `null` (no bundle); multi-code buckets carry none. The kept `grades_courses` counts kept
   single-course buckets with `graded: true`, and is `null` when every bucket is `null` (F7a, F8b,
   F1 test 12).
2. **Class times are never removed** (G5). The registrar holds only the current term, so every
   class time is counted in every total and imported whatever the student removes; no code change,
   since the proof already counts an unattributed class time in every total. This **supersedes
   §14.46 item 4's purpose**: F7a still buckets class times by match set (the r2 review's fix), but
   only for display; the kept `class_times` total never falls on a removal, and F7a's two-shell test
   now asserts the total is unchanged after removing either shell or both. Recorded beside §14.45
   (its closing note), in F6b (`course_exclude` governs feed items only) and in F8b (the two
   exceptions to the bucket rule).
Tasks changed: F7a (`graded` on single-course buckets, every course a bucket, class-time buckets for
display only, tests), F8b (kept `grades_courses` and `class_times` rules), F6b (a note that removal
never touches class times; no code), F1 test 12 (grades and class-time expectations).

### 14.48 F10's review, round 2: three owners and a list (2026-10-24)

From `docs/reports/2026-10-24-wave-b-f10-review-r2.md` (on `feat/o3-f10`; F10 passed with the
controller's fix). Each gap now has a task:
1. **Credentials leave page memory when the write returns** (Important 2). The C1 spec's §5 rule
   (`clearCredentialFields`) lost its owner when F10 retired the old login panel. F11c clears the
   zyBooks and VHL fields in both arms of `store_credentials`' result, and F11t pins it: the clear
   in each arm, and no password value assigned outside the handler. §9 A10 names it.
2. **The shots script** (Important 3). `scripts/console-shots.py:91` calls the removed
   `KNOWLU_SHOTS.startWizard`. Split by domain: **F11a** adds the page hook
   `KNOWLU_SHOTS.startOnboarding(step)` (with a static pin), and **F12** repoints the script, which
   sits beside the `wizard-check.py` it already rewrites. Until F12, the shots run aborts; it is not
   a CI gate.
3. **F5b's list** (Minor 1), each decided:
   - `open_external`: **retired** from the shell's list (its only caller was the old wizard's
     Google button; every consent now opens from Rust). Kept off the console's list, as
     `handler_lists.rs` already pins.
   - `retarget_credentials`: **retired** from the shell's list (R-P4a-23's reason, a vault name
     changed after the logins were stored, is gone: the vault and name exist from step 2).
   - `timezone_for_state`: **retired** from both lists (never called; Q-B12 retired its
     suggestion). F5's text no longer adds it to the console's list.
   None moves; none is kept for a caller. Their functions and tests stay for now (`open_external`
   is in `account.rs`, on the contract list), and F13's report lists their deletion as a follow-up.
4. **For F13** (Minor 4): `app/README.md:293` is F13's. The `account.rs:2100` doc comment is on the
   contract list, so it goes to the next contract-engineer task touching `account.rs` (F2c or S5),
   not to docs-keeper.
Tasks changed: F5 (no `timezone_for_state`), F5b (three retirements and their pin), F11a (the shots
hook), F11c (credential clearing), F11t (its pin), F12 (the shots script), F13 (stale README line;
the routed `account.rs` comment; the follow-up deletions), §9 A10.

### 14.49 An unreadable `onboarding.json` never resumes past the school (2026-10-24)

From the F2a review r1 (`docs/reports/2026-10-24-wave-b-f2a-review-r1.md`, I-2, on
`feat/o3-f2a`). The unreadable-file ladder (`earliest_unproven`, `onboarding_state.rs:420`) treated
step 3 as proven by a restore, but §4.7 says the school is never proven, and the rewritten file has
no `school`. A restored, entitled account with a corrupt file would resume past step 3 with
`school: None`, so `school.apply` could never run.
- **Chosen: (a), the ladder stops at 3 whenever the state has no `school`.** Since an unreadable
  file's rewrite never has one, an unreadable file always resumes at 2 (no session) or 3. It is the
  simplest rule, it is honest (nothing proves the school), and its cost is one confirm on a rare
  path: after the school is confirmed, every later step the checks prove is passed through as
  `already-done` (§4.7's existing rule), so the student re-answers nothing else.
- **Not chosen:** (b), a screen rule that asks the school first at any step with no school, splits
  the resume rule between F2a and F11 and leaves the file's step wrong; (c), proving step 3 by an
  existing `campus.yaml`, makes the resume read a vault config file, a second source for the school
  beside `onboarding.json`, for a corrupt-file case only. (c)'s suggestion (offer that school as the
  confirm) could be added later without changing (a)'s rule.
- **F1 test 2's assertion change** (`app/tests/onboarding_state.rs:236-245`): in `ladder`, the
  no-session rung keeps `2`, and **every other rung expects `3`**:
  `(Checks::at_least_signed_in(), 3)` is unchanged, and `(Checks::restored(3), 4)`, the entitled
  rung's `5`, the mailbox rung's `6`, the Google rung's `8`, the login rung's `10` and the
  everything-proven rung's `10` all become `3`; each of those rungs also asserts
  `r.school == SchoolOffer::Ask`. A new tail asserts the advance: on the everything-proven checks,
  confirming the school moves to step 10 with steps 4-8 `already-done`; on `Checks::restored(3)`
  alone, to step 4.
- **Tasks changed:** F2a (the rule in `earliest_unproven`; its brief), F1 test 2 (the ladder and the
  new tail), §4.5's unreadable-file bullet. F2a's I-1 ruling (a session present and a file step of
  2 or below resumes at 3) is recorded in F2a's brief too.

### 14.50 A failed first grades, its reason, and a gate that fails closed (2026-10-24)

From the F9 review r1 (`docs/reports/2026-10-24-wave-b-f9-review-r1.md`, on `feat/o3-f9`), after
the controller's ruling that F9 keeps `first_grades_marked` (a bare `grades` row at exit 0) and adds
`first_grades_failed` (a bare `grades` row at a non-zero exit):
1. **The caller's mapping goes to F8g**, not F2b, because F8g already reads each slot's result into
   the gate's items (`first sync` and `account_settled` after each slot). After each slot: marked
   gives `first.grades` `Done`; failed and not marked gives `Failed { reason, fix }`; neither leaves
   it as it was; a later exit-0 `grades` turns `Failed` into `Done`; "Try again" enqueues the next
   `NoRank` or `slot.first_full`; at finish a failed first grades goes to `handover` like any
   unfinished item. **The closed set gains its first written entry** (§4.3), as constants in
   `onboarding_jobs.rs`: reason `grades_read_failed`, fix "Knowlu couldn't read your grades this
   time. It will try again on its next run, or press Try again now. If it keeps failing, reconnect
   your school in Settings after setup." Each part is something the student can do or can count
   on: an automatic retry, a button, and a Settings action that exists after the reveal (the
   school row, S6b; Settings is unreachable during onboarding, hence "after setup"). F1 test 28.
2. **M3, noted beside §4.3's sync note:** the engine can skip at exit 0 under the bare name
   `grades` (the vault busy), which would mark first grades with nothing written. Barely reachable
   in onboarding, where the queue is serial and holds the notes gate around every slot; no fix in
   wave B, and if it appears, the fix is an engine status line, not a guess from the step name.
3. **`onboarding_status` fails closed (F2b):** a jobs read that fails is `{ok: false, error}` with
   no gate output, never an empty item list, which F9 would open on. F1 test 28's second half.
Tasks changed: F8g (the first-grades mapping and the closed-set constants), F2b (`onboarding_status`
fails closed), F1 (new test 28), §4.3 (the first-grades note and the closed set's first entry).

### 14.51 Two stops, a bounded drain, a crash-loop guard, the console's hand-over (2026-10-24)

From the F8f review (I-1, I-2), the F8a review r2 (I-2) and the F8s follow-up, checked against the
code on `feat/o3-long-onboarding` (`onboarding_jobs.rs`, `onboarding_state.rs`, `scheduler.rs`,
`mail_session.rs`, `mail/commands.rs`). Each item has an owner, a test the owner writes first (F1
merged before them, so the tests are numbered 29-32 in F1's list but written in their tasks) and the
words where the student sees anything.
1. **The slots never saw the queue's stop.** `Queue::request_stop` sets `Inner.stop`, which the
   runner is handed; `run_slot_scoped` checks `sch.stop`; nothing joined them, so Finish later during
   `slot.first_full` ran every stage. **Ruling: F2b joins them**, with
   `stop_both(sch, queue)` (sets `sch.stop`, then the queue's), the one body of the live
   `ExitSeams::request_stop`; every exit already starts with that seam (§4.10's table), so no other
   path needs it. A `Deps` field was rejected: `Scheduler.stop` is a field of managed state, not an
   `Arc`, so a `Deps` field would change `Scheduler` or carry a closure over the `AppHandle` for one
   store. **F8g adds the belt:** its slot runner returns `Stopped` before calling the slot when the
   queue's stop is already set. Test 29. No words (the page's drain lines, item 2).
2. **The drain had no limit.** **Ruling: 45 seconds (`DRAIN_LIMIT`), F8f2** (or F8f's fix round,
   the controller's call), for the serial queue's jobs that write nothing to the vault
   (`school.capture`, `school.preview`, `mail.setup`, the Google reads, `coursework.discover`),
   abandoned at the limit through the canary's existing path, so each reads `pending` as a crash
   leaves it. **A slot and a local-lane job are never abandoned**: a slot's engine child would
   outlive this process and write the vault with nobody watching, and a lane job is seconds; each
   ends at its safe point within its own bound (worst, one `CHILD_TIMEOUT`). 45 seconds is a
   chosen figure, not a measured one: long enough for a hidden read or a preview to reach its safe
   point on a slow connection, short enough that a student waits once. S-E's timings and F15's live
   proof check it, and it is one constant to change. `quit_flush`'s 10 s runs
   after the drain returns, never alongside: at most 55 s with no slot running. The page (F11a, from
   F2b's `exiting: {job, slot, since}`): "Finishing <job>…"; from 15 s, "Still finishing <job>.
   Knowlu will close within a minute and pick up the rest next time."; for a slot from 15 s,
   "Saving what Knowlu has read so far. This can take a few minutes, and Knowlu closes by itself
   when it's done." A second close still does nothing, which is now safe because the first always
   ends. Test 30.
3. **The crash-loop guard.** **Ruling: F8a2** (after F8f2 in `onboarding_jobs.rs`; the field in
   F2a's `onboarding_state.rs`). `JobRecord.crashes: u8`, `#[serde(default, skip_serializing_if =
   "is_zero")]`, so **`STATE_VERSION` stays 1**: old files read 0, a 0 is never written (today's
   bytes unchanged), and nothing denies unknown fields. A job found `running` at start counts one;
   at two it is `failed` with `job_crashed` (`REASON_JOB_CRASHED`, which the queue already uses for
   an unwinding panic) and is not run. `work`'s `running` insert must keep the count (today it
   resets the whole record); every normal end clears it; Try again sets it to 1. The fix, §4.3's
   second closed-set entry: "This step closed Knowlu unexpectedly twice, so Knowlu has paused it.
   Press Try again to run it once more." Test 31.
4. **The console's hand-over clearing was never built** (F8s merged without it; `slot_ended` was a
   stub). **Ruling: new task F8s2** (`implementer`, `scheduler.rs`, after F8f merges). The slot item
   clears at the end of a slot that ran, for a finished `onboarding.json` only; refused slots,
   absent, unfinished and unreadable files write nothing. **The mail signal is read at the slot's
   mail stage beside its address list** (`scheduler.rs:828`), not at the slot's end, through a pure
   `mail_signals(rows, &MailState)`: `mail.json` has no verified flag (`MailState`,
   `mail_session.rs:87-166`), so `CanaryVerified` comes from the service's row status (`confirmed`
   or `active`, as `mail/runner.rs:820` reads it), `Disconnected` from a `revoked` row or
   `turned_off`, `FirstPassDelivered` from `last_reconcile` (the implementer confirms its meaning or
   stops). §4.10's hand-over table is corrected. Test 32. No new words: the "still coming" line
   drops the item.
5. **Notes placed in the briefs:**
   - **F2b:** the drain runs only from the three `async` commands and F4's thread (test 24's pin);
     step 3's capture replaces an unreadable answers file (F2d review r1, I-2; a test 12 case).
   - **F8e:** the second lane (F8a) and `confirm_step10` are built; only the rules and step 10's
     gate refinement remain.
   - **F8g:** the runner holds the `AppHandle` (F8a's ruling 3); F8a's brief is corrected.
   - **F4:** manages `LaunchMode` and the `Queue` (F4b's and F8m's guards are inert without them;
     a test 27 pin); an unreadable `onboarding.json` is rewritten by passing F2a's derived state to
     `Queue::start` (T0 review M5).
   - **Open, owner unassigned: the `America/Chicago` fallback** (`onboarding.rs:155` in
     `launch_state`, `:867` in the step-2 create) when Windows reports no IANA name. It is the
     founder's own zone hard-coded for every student, so it reads as a rule-1 bug. Options: (a) F3
     takes it, and when Windows names no zone, step 2 writes the zone from the school's state
     (`scaffold::STATE_TZ`) at step 3's confirm instead, which needs §4.9 to allow the zone to be
     written then rather than at step 2; (b) a fixed-offset zone from the system's current offset,
     wrong across a DST change; (c) keep it and show the zone in Settings for correction.
     Recommendation: (a), owned by F3, with a test that a machine reporting no IANA name never gets
     `America/Chicago` from the step-2 path. The controller's call; nothing in wave B blocks on it.
Tasks changed: §4.3 (the second closed-set entry), §4.10 (two flags, one call; the drain's limit and
its words; the crash guard; the hand-over table's mail row), F1 (tests 29-32, each its owner's), F2b
(`stop_both`, the progress answer, the main-thread rule, the unreadable answers rule), F4 (two
managed values, the rewrite), F8a (the `AppHandle` corrected) and new F8a2, F8e (what is built),
F8f2 (new), F8g (the runner's stop check), F8s (the clearing moved) and new F8s2, F11a (the drain
lines), §7 (rows F2b, F4, F8f2, F8a2, F8s2, F8g).

