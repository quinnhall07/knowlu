# Knowlu — the independent app — design

**Status:** design of record, 2026-09-04. Brainstormed with Quinn section by section; every
section below was approved in conversation before this file was written. Where this document is
silent, `docs/superpowers/specs/2026-09-02-console-on-rust-design.md` (the console spec) stands,
and behind it S2 and the parent design. **Where they conflict, this document wins** (the most
recent ruling wins — Quinn, 2026-08-29).

**Name:** the product is **Knowlu**, spelled exactly so — capital K, lowercase elsewhere;
`knowlu` as a binary, crate, folder and identifier name (Quinn, 2026-09-04). `quinn-ops` remains
the name of Quinn's personal deployment and of this repository until §7's stage 3.

**Direction:** *"I want to start phasing out the old system while we keep building the app. I want
this app to be fully independent. No more Python, no more Obsidian."* — Quinn, 2026-09-04. And,
in the same conversation: the routine's judgment work moves to **local models** per the product
plan §2.4; storage is **one device, local history, backup to a folder the user owns**;
distribution is **direct download with the built-in updater**, signed with **Azure Trusted
Signing**.

---

## 1. Purpose and what changes

quinn-ops today is four cooperating pieces: a Python engine run by a PowerShell script under Task
Scheduler on the laptop, the same Python engine run by a Claude Code cloud routine that also
performs the judgment steps, Obsidian as the editor, approval surface and git auto-committer, and
GitHub as the transport that ties them together. The Rust port is at parity and mid-cutover; the
console (plan 1) renders the day read-only.

Knowlu is the end state where **one application owns everything**: it is the only process, the
only writer, the only scheduler and the only committer. Python leaves on the cutover schedule.
Obsidian leaves when the console can write. The routine leaves when the app's local models do
its judgments. GitHub leaves with the routine. Nothing in this document requires a hosted
service of ours; the free tier's rule that everything runs with zero cloud calls (VISION) is
what this design delivers for the personal deployment too.

### What this document carries unchanged

- The vault format: markdown plus YAML frontmatter, the journal as the record, single-line
  surgery, `engine.write` (now `src/write.rs`) as the only note writer, judge-once.
- The console spec's architecture: two crates, `commands.rs` computes nothing, the read model
  never writes, `State.schema = 1`, the frozen surface references and their regeneration rule.
- The cutover plan, every task and gate. **This design does not move G2, does not touch
  `$mode`, and does not edit the cloud routine.** §2 sequences everything after those.
- The product and market documents' rulings (§12 of each), VISION's standing rules, and the
  "no new single-user assumptions" rule.

### What changes

| Console spec said | This document says | Why |
|---|---|---|
| Decision 16: no launch at login | The app starts at login and lives in the tray (§3) | The app is now the scheduler |
| Decision 19: the runner rebuilds both crates; topline says `engine newer than console` | No runner and no build step; one bundle, one version (§3, §6) | The runner is gone; releases carry both binaries |
| Decision 20: the console does not run the passes | The app's slot runs the whole pass (§3) | Same reason |
| §8 sync coexists with Obsidian Git | The app is the only committer; git is a transport that leaves with the remote (§4) | Obsidian is gone; the journal is the history |
| §16 plan 3 holds the Runs view | The Runs view moves into plan 2 (§9) | `runner-log.md` stops existing when the script does |
| Product name `quinn-ops`, identifier `ops.quinn.console` | **Knowlu**, staged rename (§7) | Quinn, 2026-09-04 |

---

## 2. The phase-out order

Three hosted pieces come out in a forced order, because each one delivers the next: the routine
pushes its work through GitHub, and GitHub is the only reason the app calls git with a remote.

| Phase | When | What lands | What leaves |
|---|---|---|---|
| 0 | now → G2 (~2026-09-11) | Plan 2 built in a worktree against **scratch copies**: writes, deck, interaction events, history + backup, the scheduler behind its config key, the Runs view, rename stage 1 | nothing yet |
| 1 | at G2 | the console switched to the **live vault** | Obsidian closed; Obsidian Git stopped |
| 2 | harness out (~2026-09-18) | scheduler key flipped to `app`; coursework, ingest, rank run from the tray | `scripts/local-run.ps1`, the scheduled task, the venv, rename stage 2 |
| 3 | after | local extraction model takes enrichment, then event verdicts, then Gmail | routine steps 3, 5, 4 starve (§5.1); the routine is turned off; the GitHub remote removed; rename stage 3 with the vault split |
| 4 | after | profiles, onboarding, signed installer, updater, the site | nothing; the first friends install |

Two constraints shape phases 0 and 1:

1. **The console must not write the live vault before G2.** The cutover rule is that Rust writes
   scratch only until the switch. `write` is byte-verified by `scripts/diff-engines-notes.ps1`,
   so the risk is low, but the rule is the rule. Plan 2 is built and tested against copies; the
   switch to live is one launch argument at G2.
2. **The GitHub remote stays until the routine is off.** The no-remote end state cannot arrive
   while the routine still delivers through GitHub. The history module pulls and pushes through
   phase 3 and the remote is removed as the last act of retiring the routine.

And one that shapes phase 2: **the scheduler must not take the live vault before the harness
comes out.** Cutover Tasks 8 and 9 edit `local-run.ps1` and the week is measured on that
script's runs; replacing it mid-week restarts the count.

---

## 3. The tray scheduler

**Process shape.** Knowlu is a tray application. Closing the window hides it; the process stays
resident with a tray icon whose menu is *Open · Run now · Pause scheduling · Copy diagnostics ·
Quit*. Quit is the only thing that stops the scheduler, and the next launch reports how many
slots were missed while it was quit. The app registers itself to start at login through Tauri's
autostart plugin — an opt-in toggle in settings that onboarding turns on by default. The
single-instance plugin already guarantees one process per machine.

**What a slot does**, in order: pull (while a remote exists) → coursework → rank → commit by
name → push (while a remote exists) → backup tick (§4). **No build step**: the app runs the
engine it shipped with. Each engine step runs the sibling `knowlu.exe` (today
`quinn-ops.exe`) as a **child process** rather than calling the engine crate in-process, so an
engine panic produces a FAIL run line instead of taking the window down, and the run records
keep their exact shape (`start`/`step`/`end`, full warning text). The scheduler writes nothing
to the vault itself; the engine's own `runs` module records the run.

**When a slot runs.** Slots, grace and the device key come from the profile's runner config
(`config/runners.yaml` today). A tick every 60 s asks one question: *is there a slot earlier than
now with no run record for it today?* If so, run it. Because the answer is derived from
`state/runs/` on disk, wake from sleep, a restart and a crash all catch up identically — the
behaviour Task Scheduler's `StartWhenAvailable` gives today. Inside grace is on time; past
grace is recorded late; **no slot is ever skipped**. One run at a time: an in-process guard plus
the existing crashed-run detection (a `start` without an `end` past grace).

**The switch.** A new key in the runner config, `scheduler: script | app`, default `script`.
The app's scheduler is inert on `script`. Flipping it to `app` is one commit at phase 2, after
which the script and the scheduled task are removed. The **one-active-runner rule stands**: the
device key still names which machine runs, so a second install pointed at a shared vault never
double-runs.

**Failure visibility.** The topline shows the last run's result and the next slot; the tray
icon changes on a FAIL or a missed slot; the Runs view (moved into plan 2) lists run records with
their full warning text. `runner-log.md` is not written by the app: it was a rendering of the
`end` record, and the Runs view is now that rendering.

---

## 4. Storage and backup

**The vault stays files.** Markdown with frontmatter, the journal beside it, one folder per
profile. Nothing moves into a database. The journal is already the complete record of every
write, which is what keeps this section small.

**The journal is the history; git is a transport.** Git stays exactly as long as the remote
does, driven by the app as the console spec §8 describes: commit by name never `-A`, pull,
push, conflicts through `reconcile::resolve`, the advisory lock `state/.sync.lock`, only-ahead
means push never rebase. When the routine is gone and the remote comes out in phase 3, git
leaves the app with it. Local history after that is **the journal plus the backup snapshots**,
which together answer "what changed and when" and "put it back". Keeping git as a permanent
history store was rejected: it would mean bundling a ~40 MB git into a friend's installer for a
job the journal already does. Quinn's own machine may keep its repository indefinitely; the app
simply stops needing one.

**Backup is a mirror plus dated snapshots in a folder the user owns.** Onboarding asks for a
backup location (a OneDrive folder, an external drive, a network share) and the app writes two
things under `<backup>/<profile-id>/`:

- `vault/` — a **live mirror**, files copied on content change with temp-and-rename so a crash
  never leaves a half-written note. **Nothing is ever deleted from the mirror**: a note removed
  from the vault moves to the mirror's own `archive/`.
- `snapshots/YYYY-MM-DD.zip` — one dated snapshot a day, **30 kept**, for point-in-time restore.

Triggers are sync's: after every slot, 30 s after the last console write, on quit.

**Never in the backup:** credentials. They live in Windows Credential Manager and the vault
never contains them. Quinn's Blackboard token and calendar URL sit in the vault's config today;
the profiles milestone (plan 4) moves them to the keychain. Until then they ride in the backup,
to storage that is the user's own.

**Failure is amber, never blocking.** An unplugged drive or an offline share shows
`backup 3 days behind` in the topline and on the tray icon. **Restore** is an onboarding option:
point at a backup folder and its mirror becomes the vault.

**The door to a second device.** The mirror layout is the folder a second install would replay
from (product plan §4: sync replays the journal over storage the user already trusts). Option 2
— two devices through a shared folder — is a *reader* over this layout later, not a second
writer. Nothing here forecloses it.

---

## 5. Retiring the routine onto local models

### 5.1 Starving, not switching

Each of the routine's judgment steps acts only on work nobody has done: enrichment (step 3) on
tasks still flagged `needs_enrichment: true`; events (step 5) on entries in `state/events.md`
without a `verdict:`; Gmail (step 4) on message ids not yet in `state/ingest-seen.md`. **The app
does not switch steps off. It does the work first, at its own slots, and the routine finds
nothing left each time it wakes.** When the routine has done nothing for a week, it is turned
off once, at claude.ai/code/routines. The routine's prompt and config are **never edited during
the transition** — decision 13 and the RemoteTrigger wholesale-update trap stay out of the path.

### 5.2 One judgment seam, three tiers

A `judge` module in the engine crate answers every judgment the same way:

1. **Heuristics** — deterministic, in the engine's own terms (course map fragments, uid pins,
   vendor effort, `config/planning.yaml` defaults).
2. **Promoted rules** — a rule table learned from past model outputs (§5.4).
3. **The model** — only when tiers 1–2 report low confidence, and only if a model is installed.

Every judgment is written through `write` as a **judged** field by an agent actor
(`knowlu/enrich`, `knowlu/events`, `knowlu/gmail`), so judge-once holds: a field Quinn set in the
console is never overwritten, and the app's later opinion arrives as a `kind: amend` card in the
deck — exactly the loop the routine's judgments follow today. The engine stays deterministic
in the sense that matters: **the judge writes fields into notes; `rank` never calls a model.**

> **Amended 2026-09-07 (plan 3a, Task 2).** The actor names above — `knowlu/enrich`,
> `knowlu/events`, `knowlu/gmail` — are a defect and are replaced by **`agent:knowlu.enrich`,
> `agent:knowlu.events`, `agent:knowlu.gmail`**. `provenance::is_agent` is
> `actor.starts_with("agent:")` and nothing else, so an actor spelled `knowlu/enrich` is not an
> agent: `write_literals` would skip judge-once entirely and write no `judgment:` block, and a
> field Quinn had set by hand would be silently overwritten by the model. The new names mirror the
> routine's own `agent:routine.enrich`, which is what the journal already holds.

### 5.3 The runtime and the models

One local inference runtime: **llama.cpp, shipped as a second Tauri sidecar beside the engine**
(`bundle.externalBin`), chosen because it is the mature path for grammar-constrained decoding,
which the product plan (§2.3) requires on every model call without exception.

> **Amended 2026-09-07 (Quinn).** This section first said "through its Rust binding". A spike
> before plan 3 found that `llama-cpp-2` cannot be linked on this product's
> `stable-x86_64-pc-windows-gnu` toolchain without forking a dependency:
> `docs/superpowers/reports/2026-09-07-llama-cpp-on-gnu-spike.md`. The bindings themselves are
> fine and ggml compiles; llama.cpp's vendored `cpp-httplib` calls `CreateFile2`, which this
> mingw-w64 does not declare, and it is built regardless of `LLAMA_BUILD_SERVER`,
> `LLAMA_BUILD_TOOLS`, `LLAMA_BUILD_EXAMPLES` and `LLAMA_CURL` all being OFF.
> The deciding argument is ownership rather than that failure: **linking makes us own llama.cpp's
> build, a sidecar lets us consume its releases.** Linking would put four undiscoverable
> prerequisites on anyone who builds this repo (LLVM for `libclang`, three
> `BINDGEN_EXTRA_CLANG_ARGS` include paths, a build directory short enough for `MAX_PATH`, and the
> patched dependency), re-run on every upstream bump, and would pull a large C++ build into the
> crate that the Python-versus-Rust dual-run harness shares. The sidecar reuses machinery this app
> already ships and proved in the 2026-09-07 release rehearsal, and grammar-constrained decoding
> is available across the process boundary. Its costs are accepted and belong to plan 3: a process
> lifecycle; the choice between a loopback server and one process per call; verifying a binary we
> did not build; and version skew. Moving the whole product to the MSVC toolchain was considered
> and rejected — it would revalidate the entire Rust port, both oracles and the dual-run harness,
> to accommodate one dependency.
>
> **Refined by plan 3a (2026-09-07), ruling R-P3a-1.** The process boundary stands; the *bundling*
> does not. `bundle.externalBin` copies the runtime into the installer, and the runtime is optional
> (the free tier runs with no model at all), versions far faster than this app, and is the same
> kind of thing as the models, which this section already downloads after install. So it is
> acquired into the app-data root instead, and — ruling R-P3a-2 — the app compiles in a table of
> supported upstream release digests so that **there is no unverified path to executing a runtime
> binary**. The earlier draft of this paragraph justified the choice by installer size, on a figure
> with no source; the repo's own number is the product plan's `+20-50 MB` (line 235 of
> `2026-09-01-product-and-business-plan.md`), and size is no longer the reason.
>
> **Refined again 2026-09-07 (plan 3a, Tasks 1 and 3), on the shape of the call itself.** The
> runtime is **one process per judgment, not a loopback server**, and the binary is
> **`llama-cli.exe`**. Task 1's spike
> (`docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`) measured both shapes against
> release `b10840` over a thirty-item batch: `SERVER load=1.11s total=43.39s median=1.38s` against
> `PERCALL total=95.72s median=3.16s`. The server was faster on every measure, but the plan's
> pre-committed rule only gave it the added design — a port, a health check, a lifecycle — at
> **3×** or a per-call median over 5s, and it managed **2.15×**. The ratio only approaches 2.21× as
> the batch grows, because the 1.11s load cost is already too small to amortise further: the lever
> is **model size, not batch size** — at ~2 GB the per-call cost would be ~4.4s against the
> server's roughly unchanged 1.45s, which is 3.05× and flips the decision, so a future model-size
> increase is the re-run trigger, not a calendar date. This product ships the 1–4 B Q4 model this
> section already specifies, so: no port, no health check, no lifecycle, and about fifty seconds
> more per twice-daily background pass. `src/runtime.rs`'s `RUNTIME_EXE` is the single place
> `llama-cli.exe` is spelled; `app/src/inference.rs` reaches for that same constant
> (`quinn_ops::runtime::RUNTIME_EXE`) rather than a copy of its own, so the two cannot drift apart.
>
> **Plan 3 must also weigh a split the original wording hid:** classifying events and mail is a
> single embedding forward pass with no generation and no grammar, while only enrichment needs a
> generative model with constrained output. The two halves need not share a runtime, and the
> lighter half may not need llama.cpp at all. Two models, downloaded after install from R2 into the **profile's
app-data directory, never into the vault or the backup**:

| Model | Size | Used for |
|---|---|---|
| Embedding model (nomic-embed class) | ~0.3 GB | classification, relevance, dedup |
| Extraction model, 1–4 B parameters, Q4 | ~2 GB | five-field extraction under a grammar |

CPU inference is sufficient at these sizes. **The app runs with no model present**: tier 3
reports `model not installed` as a normal outcome and the item stays unenriched and visible,
which is the free tier's zero-cloud rule made concrete. The model download is a settings action
and an onboarding offer, never automatic.

### 5.4 The rule-promotion loop

Every model call is logged — inputs **by id**, the output, the confidence — to the profile's
app-data directory (`judgments/YYYY-MM-DD.jsonl`), not the vault, so neither the backup nor a
future second device carries raw content. A rule that reproduces the model's output across
repeated cases is proposed for promotion as an approval card; once approved it enters tier 2 and
the model stops being called for that pattern. The routine's own judgments, already in the
journal as agent-set fields, are the seed corpus.

### 5.5 The order, by size of the step

1. **Enrichment first.** Course attribution already comes from the course map and uid pins;
   vendor coursework arrives with effort. What remains is effort and importance for a
   Blackboard or Gmail task from its title and description — a five-field extraction under a
   grammar.
2. **Event verdicts second.** Embeddings of each event against `profile/interests.md` and the
   past verdicts in `state/events.md`; a per-user classifier that retrains in milliseconds.
3. **Gmail last.** The app holds a **read-only** Gmail token in Credential Manager; the tier
   classifier runs on embeddings; the extraction model fills the task's fields for the
   clear-task tier only; the other tiers file proposals as the routine does. Calendar-event
   approvals keep their executor shape: the same OAuth client, **write scope requested only when
   the user approves the first calendar event** (VISION: rare writes behind explicit approval).
4. **The failure push** (routine step 10) becomes a tray notification.

### 5.6 Data minimisation

Judgment logs hold ids, field values and confidences — never message bodies, never event
descriptions. Embeddings are computed and discarded; only the classifier's weights persist, in
app data. This is VISION's "collect the least that answers the question" applied to the one
place raw personal content is touched.

---

## 6. Distribution

**One release, three artefacts.** A tagged version produces a signed NSIS installer, a signed
update bundle, and `latest.json`. All three go to a static site on **Cloudflare Pages**, which
also serves the download page. Models are separate objects on **R2** (market doc §7.4), fetched
after install, so the installer stays under the 50 MB target.

**Signing.** **Azure Trusted Signing** signs the console exe and the installer through
`signtool` in the release script, against the certificate profile Quinn creates. The script runs
on Quinn's laptop under Quinn's own Azure login; **no signing secret ever enters the repo**.
Tauri's updater carries a second, independent signature: a keypair generated once, public half
in the app config, **private half in Credential Manager** beside the coursework passwords. An
update that fails either signature is refused and reported, never installed.

**Updating.** The app checks the manifest on launch and once a day while resident, downloads in
the background, and offers *restart to update* in the topline and the tray menu. **Never
mid-run**: a slot in progress finishes first. Engine and console ship as one bundle at one
version, so `engine newer than console` stops being a reachable state.

**Installing.** The installer bundles the WebView2 bootstrapper and registers autostart. First
run is onboarding: vault folder; backup folder, or restore from one; the LMS ICS URL; coursework
logins entered straight into Credential Manager; timezone and slots. Everything stays on the
machine.

**Feedback.** *Copy diagnostics* in the tray menu puts the version, the last three run lines,
the last error and the slot config on the clipboard — no note content.

**Release location and the repo.** Releases never come from this repository, which holds the
vault and a live token. Until the code is split out, the release script uploads artefacts from
the laptop to Pages directly. The public code repository, named `knowlu`, arrives with the vault
split (§7, stage 3).

**Versioning.** One semantic version in the app config, `0.1.0` for the first friend build. The
engine's build SHA stays in the run records for debugging.

---

## 7. Naming: Knowlu, in three stages

The name touches ~80 files. They split into what a user sees, what the running system depends
on, and what is data; each group has its own safe moment.

| Stage | When | Renamed |
|---|---|---|
| 1 | plan 2 | everything a user sees: window title, tray, `productName`, installer name, the console exe → `knowlu.exe`, the app identifier, spec and plan titles, VISION, README, product docs |
| 2 | phase 2 (script runner gone) | the engine crate and binary, the clap name, the user-agent strings that identify the product to zyBooks, VHL and the UA event hosts, test and script references, the credential targets `quinn-ops/zybooks|vhl` → `knowlu/…` (Quinn re-stores them from a non-Claude terminal, at the same time as the owed rotation) |
| 3 | phase 3 (routine off) | `QUINN_OPS_DEVICE` (the routine sets it to `cloud` on every run — read both names until then), then the GitHub repository and the local folder, together with the vault split |
| never | | journal actors, run records, ledger lines, the eight frozen references — history and Python compatibility, not branding |

**The app identifier** is reverse-DNS and should match a domain Quinn holds; the Pages site and
the Trusted Signing publisher want the same domain. **Settled 2026-09-05: Quinn holds `knowlu.com`,
so the identifier is `com.knowlu.desktop`**, set by plan 4a Task 9 before the first installer
existed (it had been `app.knowlu.desktop`). It is permanent from that point: the NSIS uninstall key,
the autostart registry entry and the window-state file are all keyed by it.

**Two stage-2 items were done early, by plan 4a** (2026-09-06), and are struck from the table above:
the **app-data root** — `%LOCALAPPDATA%\quinn-ops` → `knowlu`, a move rather than a fresh start, now
`profiles.json` plus `profiles\<id>\` — and the **identifier**. Plan 2 Task 12's steps 2 and 3
become checks. The credential targets stay in stage 2, because they wait on Quinn's rotation; note
that what plan 4a's app *writes* for a new profile is `knowlu/<profile_id>/<source>`, and Quinn's
own `quinn-ops/…` entries are untouched by it.

**The repository rename** is safe on GitHub's side — the old URL redirects clone, fetch and push
indefinitely — but the cloud routine's environment is bound to the repository and its behaviour
under a rename is unknown. Hence stage 3: after the routine is off, nothing live can lose its
binding. The local folder waits for the same moment: the scheduled task, the Start-menu shortcut,
the worktree and the session memory path all embed it, and the dual-run week is measured on that
task.

---

## 8. Testing

The two golden guards stay: `today.md` byte-identical across both engines on both fixture
vaults, and the three surface references regenerated only with a reviewed diff. Each new module
gets a seam so its tests need no real clock, network, model or remote:

- **Scheduler** — a fake clock and a run-records directory. Tests plant records and assert which
  slot fires; that a missed slot catches up; that inside grace is on time and past grace is
  late; that two ticks never start two runs; that `scheduler: script` is inert.
- **Backup** — run against the three fixture vaults, the mirror compared with the vault byte for
  byte by a Rust comparator written in plan 2 (the rule of `scripts/compare-vaults.ps1`: bytes
  first, whole tree); a planted half-written file proves
  temp-and-rename; a missing target proves amber, not failure; the archive proves nothing is
  deleted.
- **History and sync** — a temporary bare repository as the remote, conflict cases from the
  `reconcile` tests (zero callers since wave 3 — this is their first).
- **Judge** — the model behind a trait; tests use a scripted fake. Heuristics and rule promotion
  are deterministic and tested like the engine. One `#[ignore]` smoke test runs the real runtime
  only when a model file is present, named in its attribute like traps 4 and 5.
- **Installer and updater** — manual, on a clean Windows VM **with Smart App Control on**, once
  per release, from a checklist in `docs/runners/`.

Throughout: `cargo test` at 0 warnings; the app crate at zero *new* warnings; both dual-run
scripts clean after every engine change until the harness comes out; `engine/` untouched and
Python at 687 until the cutover's exit gate.

---

## 9. Plans, in order

Each ends with working software Quinn uses; the first two are gated by the cutover calendar.

1. **Knowlu foundation for independence** (console plan 2, widened): writes, deck with
   in-process execution, interaction events, history + backup, the scheduler behind
   `scheduler: script`, the Runs view, rename stage 1. Built in a worktree against scratch
   copies; switched to the live vault at G2. Engine touches: the one bundled read-model change
   from plan 1's final review, `src/sync.rs` → history, `src/backup.rs`, `src/schedule.rs` —
   all read-only with respect to what `rank` and `write` write.
2. **The runner leaves**: `scheduler: app`, script and scheduled task removed, the venv gone,
   rename stage 2, the remaining views (Decisions, Good to know, Issues), the gauge counts and
   coursework run records that waited for the cutover.
3. **Judgment comes home**: `src/judge.rs`, the runtime, model download, enrichment, events,
   Gmail; the routine turned off; the remote removed; rename stage 3 with the vault split.
4. **Friends**: profiles, onboarding, installer, signing, updater, the site.

**This order was amended once, on 2026-09-05: plan 4 split, and its shell half ran before plan 3.**
Quinn's call, argued in `docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md` (the
friends-shell design), which carries the whole of plan 4a — profiles, onboarding, the settings
panel, credentials, the installer/updater/release script and the site — and executed on
2026-09-06. Plan 4a is runner-independent: nothing in it was gated on the cutover clock, on G2 or
on plan 2 Part B, so running it first cost the queue nothing and got a friend-installable build
into existence a plan earlier. **Plan 4b** — what the shell half deferred (the wizard's
*Reconfigure…*, telemetry once there is a policy and a backend, widening the pilot) — follows plan
3. Plan 4a also landed the one engine change its own scope forced: `quinn-ops ingest`, without
which a wizard that collects an LMS calendar URL would have collected a URL nothing fetches.

---

## 10. Decisions this document makes

| # | Decision | Source |
|---|---|---|
| 1 | The product is **Knowlu**, spelled exactly so | Quinn 2026-09-04 |
| 2 | Python leaves on the cutover schedule; **nothing here moves G2 or `$mode`** | this doc §2 |
| 3 | The routine stays until the app replaces each step, by **starving** it, never by editing it | Quinn 2026-09-04, §5.1 |
| 4 | The routine's judgments move to **local models** (product plan §2.4), never to a cloud API | Quinn 2026-09-04 |
| 5 | **The app is the scheduler**, resident in the tray, autostart on; engine steps as child processes | Quinn 2026-09-04, §3 |
| 6 | `scheduler: script \| app` in the runner config; flipped at phase 2 | this doc §3 |
| 7 | **One device, local history, backup to a folder the user owns** | Quinn 2026-09-04, §4 |
| 8 | The journal is the history; **git is a transport and leaves with the remote** | this doc §4 |
| 9 | Backup = live mirror (never deletes) + 30 daily zip snapshots; amber on failure | this doc §4 |
| 10 | The console writes the live vault **only from G2** | this doc §2 |
| 11 | One judge seam, three tiers; every judgment a judged write by an agent actor; `rank` never calls a model | this doc §5.2 |
| 12 | llama.cpp as a **separate process**, downloaded and hash-verified into app data, not bundled and not linked (amended 2026-09-07 twice — see §5.3 and the spike report; was "via Rust binding", then "a second Tauri sidecar"); the app runs with no model | this doc §5.3 |
| 13 | Judgment logs hold ids and fields only, in app data | this doc §5.6 |
| 14 | **Direct download + Tauri updater**, Cloudflare Pages/R2, **Azure Trusted Signing** | Quinn 2026-09-04, §6 |
| 15 | Updater private key in Credential Manager; no signing secret in the repo | this doc §6 |
| 16 | Never update mid-run; one bundle, one version | this doc §6 |
| 17 | Rename in three stages; history and references never renamed | this doc §7 |
| 18 | Repo and folder rename only after the routine is off, with the vault split | this doc §7 |
| 19 | Runs view moves into plan 2 | this doc §3 |
| 20 | Four plans in the order of §9 | this doc §9 |

---

## 11. Open items — none block plan 2

| Item | Owner | Needed by |
|---|---|---|
| Azure Trusted Signing: account, individual identity validation, certificate profile | Quinn (lead time: days to weeks — **started 2026-09-04**) | plan 4 |
| Domain (`knowlu.com` / `knowlu.app`) → app identifier, Pages site, publisher | Quinn | plan 2 stage-1 rename records the identifier |
| Cloudflare account (Pages + R2) | Quinn | plan 3 (models), plan 4 (releases) |
| Gmail OAuth client registration (read-only scope; calendar write scope on first approval) | Quinn | plan 3, Gmail step |
| Laptop hardware for local inference (RAM; CPU is assumed sufficient at these sizes) | Quinn to report | plan 3 model choice |
| Credential re-store under `knowlu/…` together with the owed password rotation | Quinn, non-Claude terminal | phase 2 |
| G2 | Quinn | phase 1 |

---

## 12. Rulings preserved (fidelity)

Every plan under this document carries a ledger row per line below, and reviewers check them per
task.

- No new single-user assumptions (2026-09-01). Profiles, device key, backup folder, model
  directory are all per-profile.
- The cloud routine is not touched by cutover (decision 13); this document extends it: **not
  touched by the transition either** — starved, then turned off once.
- Every note write goes through `write`; journal first, single-line surgery second; no note is
  ever parsed and re-dumped.
- Judge once, re-propose freely: every app judgment is a judged write; amend cards for
  re-judgements.
- Never regenerate the eight Python-written references; the three Rust references only with a
  reviewed diff.
- `$mode` stays `python-live` until G2; Rust writes scratch only until then.
- Credentials in the OS keychain; never in the repo, a log, a backup rule, or a model prompt.
- Failures visible; silence never ambiguous (tray icon, topline, Runs view).
- quinn-ops only ever reads; rare writes behind explicit approval (calendar write scope).
- Read-only surface: `commands.rs` computes nothing; the read model never writes.
- Desktop safety: no synthetic input; screenshots by window handle only.
- The repo is not made public as it stands; the public `knowlu` repository is code only, after
  rotation and history rewrite.
