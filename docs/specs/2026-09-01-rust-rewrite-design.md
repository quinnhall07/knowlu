# Rust rewrite — porting quinn-ops from Python to Rust

**Date:** 2026-09-01
**Status:** DRAFT — awaiting Quinn's review. Nothing has been ported.
**Decision:** Quinn, 2026-09-01: *"I want to write this app in Rust"* → whole system; the page stays
plain HTML/CSS/JS.
**REVISED the same day** against `docs/superpowers/notes/2026-09-01-product-and-business-plan.md`
("the plan"), which reframes quinn-ops as a product and supplies a stronger justification for this
port than the original draft had: shipping the Python engine standalone needs a bundled interpreter,
+20–50 MB against a sub-50 MB target (plan §5.2). **The plan's §12 rulings dissolved this spec's one
go/no-go blocker** — see §3, which now says the opposite of what it said this morning.

Read alongside `CLAUDE.md` (every architecture invariant binds here) and
`docs/superpowers/specs/2026-08-11-personal-ops-system-design.md` (the parent design, which is
language-neutral and survives this change unaltered).

---

## 1. Purpose

Replace the Python engine with a Rust one, preserving behaviour exactly, then continue the S2/S3
programme on the Rust codebase.

### What changes

`engine/` — 29 modules, 6,354 lines — and `tests/` — 39 files, 9,330 lines, 686 tests. Plus
`scripts/migrate_s1.py`. **15,684 lines of Python.**

### What does not change

- **The vault.** `tasks/*.md` and every other note keep their exact frontmatter schema. This is a
  reimplementation of the code that reads and writes the vault, not a data migration. No note is
  touched by the port itself.
- **`config/*.yaml`** — five files, data not code, byte-unchanged.
- **`scripts/local-run.ps1`** — PowerShell, and it stays PowerShell. One line changes: the
  interpreter path becomes a binary path.
- **The page.** `app/static/` ships as designed — plain HTML/CSS/JS, vendored fonts, no build step
  (Quinn, 2026-09-01). The mockup and `docs/surface/anatomy.md` remain implementable as drawn.
  **The shell around it does change: Tauri replaces pywebview** (plan §5.2 — OS-native webview, no
  bundled Chromium, ~5–10 MB). That retires S2's `app/window.py` design and its
  `requirements-desktop.txt`, but not one line of the page itself. S2a's Task 15 is the only task in
  the six plans that dies outright rather than needing translation.
- **Every architecture invariant in `CLAUDE.md`.** Journal-first writes, single-line frontmatter
  surgery, judge-once, the 15/day approvals budget, must-do-first rendering, `merge=union` state
  files. The port is not licence to revisit any of them.

### What this costs — stated once, plainly

- **The six S2/S3 plans are invalidated as written.** 20,172 lines, 127 tasks, 657 TDD steps, three
  review rounds. Their *specs*, *decisions* and *reconciliation contracts* survive — those are
  design, not code — but every one of the 657 steps quotes Python and must be rewritten.
- **686 passing tests go to zero** until they are ported. During the port the Python suite remains
  the definition of correct.
- **S2 and S3 slip behind the port.** §9 explains why they cannot be built during it.

---

## 2. The oracle — the reason this port is tractable

`tests/test_today_unchanged.py` copies a checked-in fixture vault, runs the engine at a pinned date
(`2026-08-28`), and asserts `state/today.md` is byte-identical to `tests/fixtures/golden-today-s1.md`
with only the wall-clock footer stripped. It does this on **two** vaults — `vault-s1/` and
`vault-s1-migrated/`.

This is the single most valuable asset in the repo for a cross-language port, and it dictates the
whole strategy:

> **The Rust engine is correct when it produces the same bytes as the Python engine, from the same
> vault, at the same pinned date.**

That is a mechanical, unambiguous acceptance test that needs no human judgment and cannot be argued
with. It converts "did we port 6,354 lines faithfully?" — unanswerable — into a diff.

**Consequence, and it is the most important sentence in this spec:** the oracle only works while the
Rust build is a *behaviour-preserving* port. The moment a feature that deliberately changes
`today.md` lands — S3a's binder removes a Must-do line; S3b's per-event move changes the digest
count — "correct" stops meaning "identical" and the oracle is gone. **Port to parity first. Add
features second.** (§9.)

---

## 3. The cloud runner — RESOLVED, and it is not a blocker

**This section said the opposite eight hours ago.** The original draft made "can Rust run in the
Claude Code cloud sandbox?" the go/no-go for the whole project, and proposed three ways to solve it.
The plan's §12 ruling 3 removes the question rather than answering it:

> **The Claude Code cloud routine is a dead end.** It is a personal-deployment artefact with no place
> in a product — 42,000 students cannot each be given a Claude Code routine. A local scheduler plus a
> cloud inference API replaces it (plan §2.4, §3.5).

So **the Rust engine never targets that sandbox, and no static-musl binary is committed to the
vault.** Both of the ugly options are dropped. Concretely:

- **The Python engine keeps running the cloud routine, frozen, throughout the port.** It is not
  ported, not modified, and not cut over. It is a dependency with a scheduled end.
- **The port targets Quinn's desktop and laptop only** — `x86_64-pc-windows-gnu` — until the local
  scheduler exists.
- **Retiring the routine is its own milestone, after cutover**, not part of this port. Until then the
  live automation is untouched and at no risk from any of this work.
- **`§11`'s first failure mode is deleted.** It was the largest risk in the spec and it no longer
  exists.

This also removes the last reason the port needed to start with a decision from Quinn. **It can begin
immediately.**

### What the routine still constrains

One thing survives: the routine's command surface is the *contract* the Rust CLI eventually has to
satisfy, so §4's subcommand mapping stays as designed. For reference, the routine shells out to:

| Invocation | Step |
|---|---|
| `python3 -c "from engine.runs import start_run, git_sha; …"` | open the run record |
| `python3 -m engine.write --via cloud-routine --run-id $RUN_ID` (`create`/`set`/…) | every note edit |
| `python3 -m engine.ingest --vault . --via cloud-routine --run-id $RUN_ID` | Blackboard |
| `python3 -m engine.runs step $RUN_ID` | per-step records |
| `python3 -m engine.eventfeed` | event sources |
| `python3 -m engine.cli --vault . --runner cloud --run-id $RUN_ID` | rank, approvals, close run |

Every one of those becomes a Rust subcommand eventually (§4). None of them has to, to ship this port.

### The one hazard that replaces it

The routine is now a **frozen Python dependency running live, twice a day, against the same vault the
Rust engine is being developed against.** That is a smaller risk than the one it replaced, but it is
not zero:

- **Never let both engines write the same vault.** Rust development runs against *copies*
  (`tests/fixtures/`, or a scratch clone), never against the working tree, until cutover. The
  dual-run in §8 is explicit about which engine writes.
- **The routine keeps mutating `state/` and `tasks/` while the port proceeds**, so a golden file
  derived from the live vault would drift under you. This is exactly why the oracle uses **checked-in
  fixture vaults at a pinned date** and not the live one.

---

## 4. Architecture: one crate, not a workspace

The Python import graph contains **four true cycles**:

```
ingest → write → ingest
ids → ingest → write → ids
coursework → vhl → coursework
coursework → zybooks → coursework
```

Python tolerates these through function-local imports. **Cargo rejects circular dependencies between
crates but permits them freely between modules inside one crate.** So:

> **`engine/` becomes ONE crate — `quinn-ops` — with one `mod` per current Python module.**

A workspace of per-module crates would hit all four cycles and force a refactor of the write path
mid-port, which is precisely the kind of unforced behaviour change the oracle exists to catch and
that would make a failed diff ambiguous. Breaking those cycles may well be worth doing *later*; it
must not be entangled with the port.

Binaries mirror today's `python -m` entry points so the routine's command surface changes as little
as possible:

| Today | After |
|---|---|
| `python3 -m engine.cli` | `quinn-ops rank` |
| `python3 -m engine.write set …` | `quinn-ops write set …` |
| `python3 -m engine.ingest` | `quinn-ops ingest` |
| `python3 -m engine.runs step` | `quinn-ops runs step` |
| `python3 -c "…start_run…"` | `quinn-ops runs start` |

One binary with subcommands, not seven. The inline `python3 -c` in the routine prompt becomes
`runs start` — a subcommand S3c's Task 17 was already planning to add for unrelated reasons.

---

## 5. Crate budget

The engine's dependency rule — stdlib + PyYAML + tzdata — exists so the cloud runner never drags a
heavy tree into its environment. The Rust equivalent is a **short, justified, pinned** list.

| Need | Python today | Crate | Note |
|---|---|---|---|
| YAML read | `yaml` (17 uses) | `serde_yaml_ng` + `serde` | `serde_yaml` is archived upstream. Read-only: edits are single-line surgery (`update_frontmatter_fields`), never a round-trip. **CORRECTED 2026-09-02 (wave 7 Task 15):** this row originally said no YAML *emitter* was needed. Three `safe_dump` calls in `engine/` *produce* YAML — `provenance.judgment_literal` (flow, one line), and `info.open_info` / `issues.open_issue`, which build a **new** note's frontmatter in block style. No crate emits PyYAML's bytes, so `src/yamlemit.rs` is a transcription of PyYAML's emitter, measured against a frozen 188-case corpus. The invariant that survives is *no note is ever parsed and re-dumped*. |
| Dates + tz | `datetime` (25), `zoneinfo` (8) | `jiff` | Bundles tzdb, so it replaces the `tzdata` dependency that exists purely because Windows ships no system tzdb. `chrono` + `chrono-tz` is the fallback. |
| Regex | `re` (13 imports, 38 literals) | `regex` | One lookaround must be hand-coded — §6.3. |
| JSON | `json` (8) | `serde_json` | Journal and run records are JSONL. |
| CLI | `argparse` (7) | `clap` (derive) | |
| HTTP | `urllib.request` (4), `http.cookiejar` (1) | `ureq` + `rustls`/`ring` | Blocking, small. **`rustls`, not OpenSSL** — no C toolchain, which is what lets the GNU host toolchain work without Visual Studio. |
| Hashing | `hashlib` (1) | **`sha1`** | **CORRECTED 2026-09-01:** this table said `sha2`. The engine's single `hashlib` call is `hashlib.sha1` in `ids.py:32`, and `derived_id` must reproduce that digest exactly or every path-derived id changes. `sha2` was a dependency referenced by nothing and has been removed. |
| Randomness | `secrets` (1) | `getrandom` (0.2) | `secrets.token_hex(5)` in `new_id`. Pinned to 0.2 because `ureq` already brings that version — a bare `cargo add` would put a second copy of `getrandom` in the tree for no benefit. |
| Windows creds | `ctypes` (2) | `windows` (`Win32_Security_Credentials`) | `cfg(windows)`-gated; the cloud build must not need it. |
| HTML unescape | `html` (1) | `html-escape` | |
| Local HTTP server (S2) | `http.server` | `tiny_http`, or hand-rolled `TcpListener` | Decide at S2. Hand-rolling matches today's stdlib-only spirit. |

Everything else — `pathlib`, `dataclasses`, `itertools`, `os`, `sys`, `subprocess`, `platform`,
`secrets`, `typing` — is std.

**Not in the engine crate:** `tauri` is a dependency of the *shell*, when S2 resumes — it never
enters `quinn-ops` itself. This is the same boundary `CLAUDE.md` already enforces between `engine/`
and `app/window.py`, and it exists for the same reason: the engine must stay linkable from a
scheduler, a test harness and eventually a mobile target, none of which want a GUI toolkit.

**Release profile — now a shipping requirement, not an optimisation.** The plan's §5.2 targets a
sub-50 MB app and calls under 20 MB frictionless, and the compiled engine is budgeted at 1–5 MB of
that. So from wave 0, `Cargo.toml` carries:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Plan §5.3: audit with `cargo-bloat` before assuming anything — one or two crates usually dominate.
That audit belongs at the end of wave 7, when the real dependency set exists, not before.

---

## 6. The five hard ports

Everything else is mechanical. These five are where a faithful port is genuinely difficult, and
each has already cost this project a live debugging session once.

1. **Single-line frontmatter surgery.** `CLAUDE.md`: *never rewrite a vault file wholesale.*
   `apply_frontmatter_fields_to_text` edits one line and leaves every other byte — including
   comments, ordering and odd spacing — untouched. A serde round-trip would silently reformat the
   whole vault on first write. **This must stay line-based, and it must be ported before anything
   that writes.**

2. **The unreadable-note fallback.** `existing_by_uid` parses YAML and, *on failure*, falls back to
   a regex over the raw text so an unreadable note still registers its `source_uid` and does not
   resurrect as a duplicate. Rust's instinct is to propagate the error. The fallback is
   load-bearing and must be ported as a fallback, not as a `?`.

3. **The one lookaround.** `engine/ingest.py:114` builds
   `(?<![A-Za-z0-9])` + `re.escape(fragment)` + `(?![A-Za-z0-9])` — the course-fragment matcher that
   decides whether an institutional Blackboard event belongs to a course. Rust's `regex` has no
   lookaround. It is a literal-substring search with alphanumeric boundary checks: **hand-code it in
   ~10 lines; do not add `fancy-regex`.** Exactly one of 38 regex literals is affected.

4. **The coursework auth trio** — spec `2026-08-25-coursework-ingest-design.md` §7.2, which must be
   re-read before this wave. zyBooks **403s every request without a `User-Agent`** (a total outage,
   not a degradation). VHL login is CAS and needs the one-time `lt` ticket from the form, or the
   POST fails *silently* with HTTP 200 returning the login page. The dashboard is on
   `m3a.vhlcentral.com` at a URL discovered from `data-schools-payload`, not constructible. `ureq`'s
   cookie jar must hold the CAS session across the redirect chain. **And: an empty parse is failure,
   never an empty semester.**

5. **Windows Credential Manager.** `engine/wincred.py` is 60 lines of `ctypes` against `CredReadW`.
   The `windows` crate covers it, but this is the one module with no cross-platform meaning: it must
   compile away cleanly under `cfg(not(windows))` so the Linux cloud build does not need it.
   Passwords must never enter the repo — the tree auto-pushes.

---

## 7. Port waves

Dependency-ordered. Leaves first, oracle as early as possible. Each wave ends green: its Rust tests
pass and the Python suite is untouched and still passing.

| # | Wave | Modules | Lines | Ends when |
|---|---|---|---|---|
| 0 | Scaffold | — | — | `cargo test` runs; the oracle harness exists and fails loudly |
| 1 | Leaves | `models` `ledger` `planning` `provenance` `weekcal` `wincred` `eventledger` | 671 | No engine imports; pure data + parsing |
| 2 | Write path | `journal` `ids` `ingest` `write` | 1,117 | Journal-first writes and frontmatter surgery are byte-faithful |
| 3 | **Rank + render** | `scheduling` `ranking` `approvals` `render` `passes` `reconcile` | 1,604 | **THE ORACLE GOES GREEN** — byte-identical `today.md` on both fixture vaults |
| 4 | Calendar | `calfeed` | 353 | ICS + RRULE expansion matches |
| 5 | Events | `events` `eventfilter` `eventroster` `eventfeed` `eventemit` | 967 | Digest and roster render identically |
| 6 | Coursework | `coursework` `zybooks` `vhl` | 939 | Against the checked-in fixtures, then one supervised live run |
| 7 | CLI + orchestration | `cli` `runs` `info` `issues` | 703 | Every subcommand has a home; run records match |
| 8 | Cutover | — | — | §8 |

**Wave 3 is the milestone that matters.** It is only 3,392 lines in — 53% of the engine — and at
that point the port is either demonstrably faithful or demonstrably not. Everything before it is
speculative; everything after it is verified as it lands. If this project is going to fail, it fails
at wave 3, cheaply.

`approvals.py` (958 lines) is the largest single module and sits inside the oracle wave because
`render` imports it. It is also the module with the most live scar tissue — the amendment hardening
report and the budget build report both concern it. Budget accordingly.

---

## 8. Cutover

Not a big bang. The Python engine stays authoritative until every gate below is green.

> **Plan written 2026-09-02:** `docs/superpowers/plans/2026-09-02-rust-cutover-plan.md`. Two
> amendments it records: **step 3 below is void** — under §3 and decision 13 the cloud routine
> stays on frozen Python and is retired by the local-scheduler milestone, so cutover makes no
> `RemoteTrigger` call at all; and **the local runner moves to the laptop before it moves to
> Rust** (Quinn, 2026-09-02: the laptop is becoming the main device), so that each change has
> one possible cause. Step 4's month is moot while the routine needs `engine/`.

1. **Dual-run.** After wave 7, `scripts/local-run.ps1` runs both engines against the same vault —
   Python writes, Rust writes to a scratch copy — and diffs `today.md`, `state/runs/` and the
   journal. **One clean week, both scheduled runs a day,** before anything switches.
2. **Local first.** The local runner cuts over before the cloud one. It is deterministic, it
   involves no Claude, and a failure there is visible in `state/runner-log.md` within six hours and
   costs nothing irreversible.
3. **Cloud last, and only after §3 is settled.** The routine prompt changes are a `RemoteTrigger`
   `update` — full `job_config`, `outcomes` absent, independent `get` afterwards.
4. **Keep the Python engine in the tree** for one month after cutover, unimported. It is the only
   way to re-derive a golden file if a bug is found later, and it costs nothing but disk.
5. **`scripts/migrate_s1.py` is not ported.** It ran once, on 2026-08-29; the vault carries its
   records. Re-running it on a migrated vault is already forbidden by `CLAUDE.md`. It stays as a
   Python artefact of a completed event.

---

## 9. What happens to S2 and S3

**They wait for cutover.** This is a consequence of §2 and it is not negotiable without giving up
the oracle:

- S3a's binder **deliberately** removes a Must-do line from `today.md`.
- S3b's per-event move **deliberately** changes the digest count line.

Both are planned re-baselines of the golden file. During the port, a changed golden file is
indistinguishable from a bad port. Building either while porting means every subsequent diff must be
argued rather than checked, which discards the one thing making a 15,684-line port safe.

What survives, and should be re-read rather than rewritten:

- Both specs (`2026-08-31-s2-surface-design.md`, `-s3-data-quality-design.md`) — design, not code.
- `2026-08-31-plan-reconciliation-contracts.md` — the ownership table, the shipping order, the
  edit-reason rules and the task-split feature are all language-neutral. **The signatures become
  Rust signatures; the contracts they encode do not change.**
- `docs/surface/` and the chosen mockup — the page is unaffected (Quinn, 2026-09-01).
- The six plans' *task decomposition* is largely reusable; their 657 TDD *steps* are not.

The shipping order `S2a → S3a → S3c → S2b → S3b → S2c` and its reasoning (contracts §7) still hold,
and still apply — after cutover.

**S2a is the one partial exception worth considering.** It is read-only, it never writes `today.md`,
and its golden guard is green throughout by design. It could in principle be built in Rust against
the Rust engine as soon as wave 3 is green. Recommendation: **do not** — it also depends on
`surface.py` extracting `_status_word`, `_due_move` and `capacity_breakdown` out of `render.py`,
which is a behaviour-preserving refactor of the exact module the oracle watches. Land it after
cutover, where a red guard means what it has always meant.

---

## 10. Testing

- **Port the tests, do not re-derive them.** 9,330 lines encode behaviour nobody remembers deciding
  — the `_proposal_weight` `events-digest` branch, the `23:59` placeholder, the seven skipped
  institutional-event shapes. A test rewritten from the implementation tests nothing.
- **Same names.** `tests/test_approvals.py` → `tests/approvals.rs`, function names preserved, so a
  coverage gap is a `grep` rather than an audit.
- **Fixtures are shared verbatim.** `tests/fixtures/` — the two vaults, `blackboard.ics`, `gcal.ics`,
  `zybooks-assignments.json`, `vhl-dashboard.html`, `golden-today-s1.md` — is consumed unchanged by
  both suites. It is the contract between them.
- **The oracle runs on every wave from 3 onward**, not just at the end.
- **A cross-engine differ** (`scripts/diff-engines.ps1`) is wave 0 work, not wave 8 work: point both
  engines at a vault copy and diff every generated file. It is what makes §8's dual-run week cheap.

---

## 11. Failure modes

| Mode | Consequence | Guard |
|---|---|---|
| ~~Rust cannot run in the cloud sandbox~~ | **DELETED 2026-09-01** — the routine is a dead end and Rust never targets that sandbox (§3) | — |
| Both engines write the live vault at once | Interleaved writes, an unattributable journal, and a corrupted golden baseline | §3 — Rust runs against fixtures and scratch copies only, until §8's dual-run |
| serde round-trips a note on write | The whole vault silently reformats on first write; Obsidian Git pushes it within 5 minutes | §6.1 — line surgery, plus a test asserting an unrelated byte never moves |
| Empty parse read as empty semester | A dead zyBooks/VHL session blanks real coursework | §6.4 — already the Python rule; port it as a rule, not an accident |
| Golden diff argued instead of fixed | The oracle stops meaning anything | §9 — no feature work during the port |
| Single-user assumptions baked into the port | Every one must be undone before a friend can run it — and VISION.md now forbids adding them | §12 #15 — profiles land right after cutover, and the port introduces no *new* hardcoding |
| Port drifts into refactor | Diffs become unattributable | §4 — one crate, cycles preserved, no cleanup during the port |
| Python deleted too early | No way to re-derive a golden file | §8.4 — keep it a month |

---

## 12. Decisions this spec makes

| # | Decision |
|---|---|
| 1 | **One crate**, `quinn-ops`, one `mod` per Python module. The four import cycles are preserved, not refactored (§4). |
| 2 | **One binary with subcommands**, mirroring today's `python -m` surface; the routine's inline `python3 -c` becomes `runs start` (§4). |
| 3 | **The golden `today.md` is the acceptance test** for the whole port, and it fires at wave 3 — 53% in (§2, §7). |
| 4 | **No feature work during the port.** S2 and S3 resume after cutover (§9). |
| 5 | **No note is ever re-dumped.** Edits stay single-line surgery; serde is read-only (§5, §6.1). **Amended 2026-09-02, Quinn's decision for Task 15:** the crate carries one deliberate emitter, `yamlemit`, for the three places Python *produces* YAML — the `judgment:` literal and the frontmatter of a new `info/` or `issues/` note. It transcribes PyYAML rather than pulling a crate, because its bytes are the oracle. |
| 6 | **`rustls`, never OpenSSL**, so no C toolchain is required and the GNU host toolchain suffices (§5). |
| 7 | **GNU host toolchain on Windows** — no Visual Studio Build Tools (not installed; multi-GB). MSVC addable later via `rustup` if a crate ever demands it. |
| 8 | **Tests are ported, not rewritten**, keeping names and sharing `tests/fixtures/` verbatim (§10). |
| 9 | **Cutover is local-then-cloud, after one clean dual-run week** (§8). |
| 10 | **`migrate_s1.py` is not ported** — it ran once and must never run again (§8.5). |
| 11 | **The Python engine stays in the tree for one month** after cutover (§8.4). |
| 12 | **The vault, `config/`, `local-run.ps1` and the page are unchanged** by the port (§1). |
| 13 | **The cloud routine is not ported and not cut over.** It stays on frozen Python until a local scheduler replaces it, which is a separate post-cutover milestone (§3, plan §12 ruling 3). |
| 14 | **Tauri replaces pywebview** as the shell when S2 resumes. S2a Task 15 is retired; the page is untouched (§1, plan §5.2). |
| 15 | **The port adds no new single-user assumptions**, but does not generalise the existing ones either. Per-user profiles are the milestone immediately after cutover — see §14 Q1 for why not during. |
| 16 | **The release profile is fixed from wave 0** (§5), because compiled engine size is a product budget line, not a preference. |
| 17 | **Parity first, profiles after cutover** (Quinn, 2026-09-01). §14 Q1. |
| 18 | **The oracle is verified, not assumed.** Python 3.14.7, `.venv` built, **686 passed / 0 failed** on 2026-09-01 — `test_today_unchanged.py` included, so `golden-today-s1.md` provably reproduces from the fixture vault today. |

---

## 13. What is NOT decided here

Deliberately left to the plan, or to Quinn:

- ~~Whether the sandbox builds Rust or runs a committed binary~~ — **resolved by deletion** (§3).
  Neither. The question no longer exists.
- `jiff` vs `chrono` + `chrono-tz` (§5). Recommendation is `jiff`; either is defensible, and the
  decision is reversible inside wave 1.
- The S2 HTTP server crate (§5) — not needed until S2 resumes, and Tauri may make it unnecessary
  entirely if the shell talks to the engine in-process rather than over loopback. **Worth revisiting:
  the loopback API existed to cross a Python/GUI boundary that Rust does not have.**
- Whether to break the four import cycles *after* cutover. Probably yes; explicitly not now.
- The on-disk shape of a per-user profile (§14 Q1) — deliberately not designed here, because
  designing it during the port is what §14 Q1 argues against.

---

## 14. Open questions for Quinn

*Q1 and Q2 of the original draft are gone — §3 dissolved both. Q3 (do S2/S3 wait?) was answered by
Quinn choosing the parity port as the next milestone. What remains:*

1. ~~**Profiles pull against parity — which gives?**~~ **RESOLVED — Quinn, 2026-09-01: "Parity
   first."** Per-user profiles are the milestone immediately *after* cutover, not during the port.
   The reasoning that stands behind it: a profile abstraction moves the vault and config paths the
   oracle holds fixed, so introducing one mid-port gives every golden-file diff two possible causes
   instead of one. **Consequence to hold to:** if a friend needs to run this before the port
   finishes, the answer is to give them the *Python* system with a profile layer — not to relax
   parity.
2. **Is behaviour parity truly the goal?** This spec forbids fixing anything during the port,
   including known-wrong behaviour. If there is something you actively want changed, it is cheaper to
   change it in Python *before* wave 3 freezes the golden file than to carry it through the port and
   change it after. Live candidates: the `blount` source, the Localist/Engage 10-event ceiling, and
   the runner-log WARN truncation (HANDOFF docket 7).
3. **When does the local scheduler replace the cloud routine?** §3 makes it a post-cutover milestone
   with no date. It is also the last thing standing between the personal system and the product
   architecture, and it is what finally makes the Gmail/events/calendar path something a second user
   could have. Worth sequencing explicitly rather than letting it drift.
4. **What does a friend actually need to see in the demo?** It determines whether the post-cutover
   profile milestone is "a second vault path and a settings file" or "onboarding, source connection,
   and a credential store." The plan's §9.3 sets the bar — *"it already knows what I should work on
   today" in the first session* — which is a high bar for someone whose Blackboard the system has
   never seen.

---

## 15. Hand-offs

- **Next artefact:** a plan per wave, in `docs/superpowers/plans/`, following the house TDD format.
  Waves 0–3 should be one plan — they stand or fall together at the oracle. **With §3 resolved,
  nothing blocks writing and executing it**; the toolchain is installed (Rust 1.98.0,
  `stable-x86_64-pc-windows-gnu`, no Visual Studio required).
- **Post-cutover milestones, in order:** ~~per-user profiles + onboarding (§14 Q1, Q4) → local
  scheduler retiring the cloud routine (§14 Q3) → S2/S3 on Rust (§9) → Tauri shell.~~
  **REORDERED — Quinn, 2026-09-02** ("I accept the reorder"), because the goal is a demoable app
  for friends soonest: **Tauri spike → the console on Rust (S2a/S2b translated; the shell links
  the engine as a library, Tauri IPC replaces the loopback API) → per-user profiles + onboarding
  + entitlement seam + installer.** The local scheduler that retires the cloud routine drops off
  the friend path entirely: a friend's build is the free tier (market doc §8.4 — coursework,
  morning view, ranking, capacity, zero cloud calls), which the deterministic engine already is
  without the routine. Quinn keeps the routine for himself until that milestone comes round.
  S3 still follows the contracts' order after S2a. Nothing in the new order forecloses the old
  milestones. Plan: `docs/superpowers/plans/2026-09-02-rust-cutover-plan.md` Task 10 docket.
- **`docs/HANDOFF.md`** must gain a banner recording this decision, that the six S2/S3 plans are
  suspended rather than abandoned, and why.
- **Time-sensitive and unaffected by any of this:** CS 100 Project 2 posts to zyBooks on
  **Mon 2026-09-07**, and `tasks/cs-100-project-2.md` … `-5.md` carry no `source_uid`. The dry-run
  must be run daily from **Fri 2026-09-04**, and the hand-stamp applied when the id appears. Until
  the port cuts over that is the **Python** engine's job, and it needs a working `.venv` on
  whichever machine runs it.
