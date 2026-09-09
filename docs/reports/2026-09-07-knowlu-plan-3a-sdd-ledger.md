# Knowlu plan 3a - preserved SDD ledger

The plan's SDD workspace is git-ignored scratch and is deleted when the plan closes. This file is
its durable record: the controller ledger with all rulings, then the whole-branch final review and
the scoped re-review of the final fix wave, as appendices.

Plan: `docs/superpowers/plans/2026-09-07-knowlu-judge-and-enrichment-plan.md`
Spec: `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` section 5
Branch: `worktree-knowlu-plan-3a`, 27 commits, base `main` at 02cc688.

---

# SDD ledger â€” plan: docs/superpowers/plans/2026-09-07-knowlu-judge-and-enrichment-plan.md

**Spec:** `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` Â§5 (Â§5.3 as amended
2026-09-07), Â§8, Â§10 decisions 4, 11, 12.
**Worktree:** `C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1`
**Branch:** `worktree-knowlu-plan-3a`, created off `main` at **02cc688** (= `origin/main`, 0/0).
**Workspace:** `.superpowers/sdd/2026-09-07-knowlu-judge-and-enrichment-plan/`
**Started:** 2026-09-07.

Twelve tasks. The plan was written from a brief, independently reviewed (5 blocking, 9 should-fix,
10 minor) and revised; the revision list is `.superpowers/sdd/plan3a-revision-1.md` and the writer
brief is `.superpowers/sdd/plan3a-writer-brief.md`. Both predate this workspace and are kept.

---

## Pre-flight conflict scan (controller, before Task 1)

**Method.** Every task's `Files:` and `Interfaces:` block was extracted and compared pairwise; then
each of the revision's 24 findings was re-checked by name against the revised plan text, because a
fix is the likeliest source of a new inconsistency. The independent review had already verified the
signatures, the `propose_amendment` port against `engine/write.py`, the actor-name correction, the
three spike branches, determinism, judge-once, provenance and the deliberate 8-before-9 ordering;
those are not re-litigated here.

### Rows: pairs of tasks that share a file or an interface

| Producer â†’ consumer | What passes | Found |
|---|---|---|
| T1 â†’ T3 | the report's `Outcome:` and `Flags:` lines | agrees; T3 step 1 reads both, and T1 step 8 enumerates every place outcome B reaches (T3, T4 step 5, T7, T8) |
| T1 â†’ T8 | release tag, asset name, byte size, SHA-256 â†’ `SUPPORTED_RUNTIMES` | agrees; T1 step 1 records all four, T12 step 8 refuses to close on an empty table unless outcome C |
| T2 â†’ T4 | `src/judge.rs`; `Item, Heuristics, Verdict, ModelError, clip, one_line, MAX_BODY_CHARS, MAX_REASON_CHARS` | agrees â€” every name T4 consumes is in T2's Produces list, constants included |
| T2 â†’ T7 | `Item, Heuristics, Outcome, Missing, Model, NoRules, judge_task, one_line` | agrees |
| T3 â†’ T4 | `src/runtime.rs`; `Server::complete` | agrees; T4 adds `impl judge::Model for Server` only |
| T3 â†’ T7 | `resolve, Server, LOAD_TIMEOUT, CALL_TIMEOUT` | agrees |
| T3 â†’ T8 | `quinn_ops::runtime::RUNTIME_EXE` | agrees, and it is referenced as the constant (not the literal) at 3682, 3811, 4016, 4027, 4267, 4275 â€” which is what makes outcome B free for T8 (S6) |
| T5 â†’ T7 | `judgelog::{entry_for, record}` | agrees |
| T6 â†’ T7 | the `proposed` branch inside `write_literals`; `ProposeNotPorted` removed | agrees â€” T7 consumes `write_literals`, not `propose_amendment`, so the re-proposal is reached through the existing call, exactly as Python does |
| T7 â†’ T9 | the `quinn-ops judge` command and its flags | agrees: T7's clap arm takes `--vault --via --run-id --runtime --model --log-dir --limit` (3555â€“3607); T9's `slot_argv` emits `judge --vault â€¦ --via local-runner --runtime â€¦ --model â€¦ --log-dir â€¦` (4708â€“4711) |
| T8 â†’ T9 | `inference::{runtime_exe, model_file, judgments_dir}` | agrees, **with an asymmetry worth naming**: `runtime_dir`/`models_dir`/`runtime_exe`/`model_file` take the install-wide `root`; `judgments_dir` takes the **profile's** `data_dir` (R13). T8 documents it at the function (4006â€“4008) and T9 asserts it (`!a.log_dir.starts_with(&root)`, 4604). No action. |
| T8 â†’ T10 | `status, Status, Half, install_from_file, install_from_manifest, install_model_from_file, remove_model, models_dir, runtime_dir, MANIFEST_URL` | agrees; T10 is barred from calling `fetch_manifest`/`download_to`/`install_runtime_from_zip` directly (M7 + R-P3a-2) |
| T9 â†’ T10 | `commands::inference_root` | agrees; ordering 9â†’10 is correct |
| T2, T3, T5, T7 | all add one `pub mod` to `src/lib.rs` | additive, sequential on one branch â€” no conflict |
| T9, T10 | both modify `app/src/commands.rs` | additive (T9 adds `inference_root`, T10 adds five commands), sequential â€” no conflict |
| T6, T11 | both read journal records; no shared file | none |
| T11 | `scripts/starvation.ps1`, `tests/starvation.rs` â€” both new, no other task touches them | none |
| T12 | docs only; no task's code file | none |

### Rows: each task against itself, and against the revision

| Task | Checked | Found |
|---|---|---|
| T1 | outcome rule pre-committed before measurement; step 8 enumerates B's blast radius; step 9 carries the full replacement | consistent |
| T2 | Produces list vs the names T4 and T7 consume | consistent |
| T3 | **B1** â€” the real-runtime smoke test was moved out of T3 into T4 step 5; T3 step 8 now expects "**2 ignored**" (1733) and T4 step 7 names the third | fixed, verified |
| T4 | **B2** â€” `prompt_for` clips at the point of use, not only in `Heuristics::load` | fixed, verified |
| T5 | log holds ids and field values only; `no_note_text_ever_reaches_a_log_line` | consistent |
| T6 | **B3** â€” the new dual-run cases are `w13`/`w14`/`w15` (2861â€“2863), appended after `w12`, and all three are in the success list (2869); no collision with the existing `w5`/`w6`/`w7`. **B4** â€” the amend-suffix mask `amend-([A-Za-z0-9_.-]+)-[0-9a-f]{6}` is present (2845) and the header says so (2825). **S9** â€” `--evidence` is on `w14` (2862) | fixed, verified |
| T7 | **S8** â€” `literals_for` renders through `write::to_literal` with a `serde_yaml_ng::Number`, never `format!` (3366â€“3385) | fixed, verified |
| T8 | **S6** â€” every assertion is against `quinn_ops::runtime::RUNTIME_EXE`; **R-P3a-2** â€” `install_runtime_from_zip` = check then extract, and it is the only thing production calls | fixed, verified |
| T9 | **S3** â€” `judge_state_in(root, cs)` exists with `judge_state` as the thin wrapper, and the tests point at a temp root (4588â€“4604, 4676â€“4688) | fixed, verified |
| T10 | commands marshal only; the composition is in `inference.rs` (M7) | consistent |
| T11 | read-only; exit 0 always | consistent |
| T12 | **B5/R-P3a-1** â€” no "100â€“200 MB" survives anywhere in the plan; the size figures are the product plan's +20â€“50 MB plus T1's measured bytes | fixed, verified |

**Verdict: the scan is clean.** No conflict required a ruling before execution. The one thing worth
carrying into a dispatch is the `judgments_dir` argument asymmetry, which is noted in T8's and T9's
briefs already.

---

## Rulings

**R-3a-1 â€” Outcome B stands: one process per call.** The spike measured `SERVER load=1.11s
total=43.39s median=1.38s p90=1.90s` against `PERCALL total=95.72s median=3.16s p90=3.95s`. The
pre-committed rule gives A only at â‰¥3Ã— or a per-call median over 5s; the ratio is 2.15Ã— and the
median 3.16s, so B. **Why I did not override it toward the expected answer:** the measurement ran
the configuration Â§5.3 actually specifies (a 1â€“2 B Q4 model), the server's number was flattered by
`cache_prompt` which per-call cannot have, and B deletes an HTTP server, ephemeral-port
allocation, health-check polling, a single-flight latch and a `Drop` impl from the engine in
exchange for ~51s on a background pass that runs twice a day. **What it costs if wrong:** the
judge pass takes about twice as long; re-adding a server later is purely additive, because
everything downstream talks to `judge::Model`.
Correcting the report's own forward-looking claim, which was backwards: the ratio *converges* to
3.19/1.45 â‰ˆ 2.21Ã— as the batch grows and never crosses 3Ã—, because the 1.11s load is already too
small to amortise further. The lever is **model size** â€” at ~2 GB, per-call's item cost would be
~4.4s against the server's ~1.45s, which is 3.05Ã— and flips the outcome. Committed as 5d61291,
with a re-run trigger written into the report: if the shipped model passes ~1.5 GB, measure again.

**R-3a-2 â€” `--single-turn`, not `-no-cnv`.** The flag in the plan's pasted `PerCall` code does not
exist on the measured release (`b10840` rejects it outright). Task 3 uses the long form
`--single-turn`. Not a runtime probe and not pinning an older release: `SUPPORTED_RUNTIMES` is what
decides which releases may be executed at all, so the flag only has to hold for the releases in
that table, and every later addition is checked against it. Recorded in the report beside the code.
Cost if wrong: `llama-cli` refuses its arguments and every judgment returns `ModelError::Failed` â€”
loud, immediate, and caught by Task 4's `#[ignore]`d smoke test the first time anyone runs it.

**R-3a-3 â€” `SUPPORTED_RUNTIMES` is seeded with, and Task 12 cites, these measured values.** Tag
`b10840`; asset `llama-b10840-bin-win-cpu-x64.zip`; **18,417,566 bytes** (17.6 MiB â€” well under the
product plan's +20â€“50 MB, which is the only other size figure this plan may state, per R-P3a-1);
SHA-256 `7063dfc6b874e7eee0ddf601bdf8e70e6f4a3d708926641ffad046ec51e8e30b`. Task 8 must not
hard-code `â€¦/releases/latest/download/â€¦` anywhere: that release is a nightly *pointer* carrying no
binaries, and the asset name embeds the build number.

**R-3a-4 â€” commit messages go in the SDD workspace, never `.git/`.** This is a worktree, so `.git`
is a gitlink *file* and every task's `-F .git/COMMIT_MSG_*` step is unwritable. Use
`.superpowers/sdd/2026-09-07-knowlu-judge-and-enrichment-plan/COMMIT_MSG`, which is git-ignored.
Carry this in every dispatch; the plan's text is wrong in twelve places and it is not worth a
commit to fix each.

**R-3a-5 â€” Task 1's review is mine, not a dispatched reviewer's.** The deliverable is one report
with no code, and I had to read it in full to carry its numbers into Tasks 3 and 8 regardless. The
review found one substantive defect (the batch-size claim) and one unresolved decision left to a
later implementer (the flag); both are fixed in 5d61291. Cost if wrong: a second opinion on a
docs-only commit, which the whole-branch final review will see anyway.

**R-3a-6 â€” `PerCall::complete` must drain stdout on a thread.** The plan's outcome-B code polls
`try_wait` to completion and only then reads the child's stdout. On Windows `Stdio::piped()` is an
anonymous pipe with a ~4 KB default buffer; under outcome B `llama-cli` echoes the prompt as well
as the completion, and this plan's own bound allows a prompt just under 6 KB. The child would
block in `WriteFile`, the parent would poll until `CALL_TIMEOUT`, and **every judgment would fail
by timeout** â€” on the real runtime only, which no `#[ignore]`d test would have caught before
Quinn's first live slot. Task 3 spawns a reader thread that reads stdout to end while the parent
polls the deadline, and joins it after the child exits or is killed. Cost if wrong: a thread per
judgment on a twice-daily background pass, which is nothing.

**R-3a-7 (SUPERSEDED, kept for the record) â€” "`parse_reply` anchors on the LAST `{`".** The problem
it addressed is real: under outcome B stdout carries the echoed prompt before the completion, and a
note body may contain a brace, so the plan's `find('{')` would parse the prompt.

**R-3a-7a â€” the correction, which is what Task 4 is dispatched with.** `rfind('{')` is *also*
wrong, and reading `GRAMMAR` before writing the dispatch is what caught it: `string ::= "\"" ([^"\\]
| "\\" ["\\/bfnrt])* "\""` permits braces **inside string values**, so a model answering
`"importance_reason": "worth {5} points"` defeats it. My "the object is flat so the answer is the
last `{`" reasoning confused *nesting* with *brace characters*. Neither anchor works, because the
premise is wrong: `parse_reply` is no longer handed a clean response body at all.
So: **anchor on nothing.** Try each `{` from the left, parse one complete JSON value from there
with `serde_json::Deserializer` (which ignores trailing text, covering anything the runtime prints
afterwards), and accept the first object carrying both `effort_hours` and `importance` â€” so a note
body containing a valid `{"a": 1}` is skipped rather than mistaken for an answer. Four tests named
in `task-4-outcome-b.md`, including one for the brace-in-a-string case that killed the superseded
ruling. Cost if wrong: a parse failure reported loudly, one item skipped, never a bad write.

**R-3a-8 â€” a course note counts as known even with no `## Grade weights` section (review M1).**
`knows_course` tested `weights.contains_key`, and `weights` was only populated for notes with a
non-empty weights section. The friends wizard writes `course_map: {}` and an **empty** `courses/`
(`app/src/scaffold.rs:73,137`), so on every friend's vault `knows_course` was false for every slug
and `judge_task` silently discarded every course the model got right â€” no `why`, no log line,
`Outcome` still `Answered`. Insert unconditionally; the key set means "courses this vault has notes
for" and an empty value means "no weights section". Task 4's `prompt_for` already skips empty
weights, so nothing downstream moves. Cost if wrong: a course note with no weights contributes an
empty string to a map, which nothing reads.

**R-3a-9 â€” the weights map is keyed by the note's own `slug:`, falling back to the file stem
(review m5).** Only load-bearing *because* of R-3a-8: the key set now decides `knows_course`, and
task notes carry the slug, not the filename, so a stem that disagrees with `slug:` would drop that
course's attributions exactly as M1 did. `Heuristics::load` stays infallible â€” unparseable
frontmatter falls back to the stem.

**R-3a-10 â€” `one_line` agrees with `write::single_line_problem`, not with Python (review M2).** It
split on Rust's `char::is_whitespace`, which excludes U+001Câ€“U+001F; `single_line_problem` measures
lines with `pystr::splitlines`, which includes them. So its documented invariant was false and both
tests written to prove it were unreachable. Fixed by splitting on `pystr::is_python_space`. **The
reason matters more than the fix:** `judge.rs` has no Python counterpart, so Python fidelity is not
a goal here â€” which is why I ruled *against* the same reviewer's m3 suggestion to swap the three
`str::trim()` calls for `pystr::strip`. Agreeing with our own writer is a requirement; matching
Python's whitespace set is cargo-cult. The doc comment also stops claiming protection against a
leading `---`, which it never had and does not need: Task 7 writes through `write::to_literal`,
which quotes.

**R-3a-11 â€” tier 1's haystack stays `item.title` alone (review m4).** Tier 1 and the ICS ingest
share the *rule* (`match_course_fields`) but not the *haystack*: `match_course` passes SUMMARY and
CATEGORIES joined. Widening tier 1 to the body would let an incidental mention ("see also CS 100")
attribute a course, which is worse than missing one â€” and tier 1 only runs the map when
`item.course` is already empty, so it sees exactly the items the ingest failed on. Doc fixed to say
the rule is shared and the haystack is not; no behaviour change.

Also ruled without a number, all "no change": **m3** (see R-3a-10), **m8** (`NoRules` tautology â€”
kept as a contract pin for 3b), **m11** (temp dir leaks on a panicking assertion â€” matches a dozen
existing tests, not drift), and the reviewer's stated design preference to split
`Outcome::LowConfidence` into separate variants for "unsure" and "broken" (Task 8's `entry_for`
match arms depend on the current shape; the `why` string carries the distinction).

**R-3a-12 â€” no task runs an engine command against the worktree root.** Task 3's implementer, while
debugging shell quoting, ran `engine.info open` twice with the default vault (`.`) and created two
stray `info/` notes plus journal lines **inside this worktree**. It noticed, deleted them, restored
the journal, and said so unprompted; I verified independently â€” `git status` clean, `dac8fc8`
contains only `src/lib.rs` and `src/runtime.rs`, and `git diff --name-only main...HEAD` names five
files, none of them vault data. **The rule as written did not forbid it**: "scratch copies only,
never the live vault" reads as being about `C:\Users\danie\GitHub\quinn-ops`, and this worktree is
not that â€” but it *is* a full checkout of the vault, so a committed stray note would have merged
into Quinn's real one. From Task 4 on, every dispatch says: never invoke an engine binary or
`python -m engine.*` with `--vault .` or any path inside the worktree; copy a fixture vault to a
temp directory and use that. Cost if wrong: nothing here was lost, and the incident is the reason
the rule now exists.

**R-3a-13 â€” a runtime that fails after spawning must say so, in its own words (review M1 + m5).**
Three independent paths all produced `Ok("")`: the exit status bound to `_` and dropped, a reader
doing `let _ = read_to_string` (which leaves the buffer *unchanged* on invalid UTF-8), and `stderr`
to null. Every realistic post-spawn failure on a friend's laptop â€” a runtime unzipped without its
sibling `ggml-*.dll`, a corrupt `.gguf`, an OOM load, the invalid-flag case R-3a-2 describes â€” would
have reached Task 4 as "the model gave a bad reply", twice a day, with no diagnostic anywhere on the
machine. **I went further than the reviewer's one-arm fix and captured stderr on a second reader
thread**, because the judge always exits 0 by design, so that error string is the only diagnostic
that will ever exist. Both streams read as bytes, capped, `from_utf8_lossy` (the prompt echo carries
arbitrary note text and a cap can slice a multi-byte character); non-zero exit *and* a clean exit
with no output are both errors carrying the stderr tail. Cost if wrong: a second thread per
judgment, and the second pipe must be drained for its whole life or it reintroduces R-3a-6's
deadlock on the other stream â€” which the fix brief says twice.

**R-3a-14 â€” the grammar file is unique per call (review M2).** It was `knowlu-gbnf-<pid>/judge.gbnf`
for every call in the process. `PerCall` is `Sync` and the server's single-flight mutex went with
the server, so the first time anything judges two items in parallel, one call's delete races the
other's child opening it â€” and *before* R-3a-13 that returned `Ok("")`. Two defects composing into a
silently wrong answer is the reason this is fixed now rather than when concurrency arrives. The
directory is deliberately **not** removed: that would race a concurrent call writing into it.

**R-3a-15 â€” `KNOWLU_LLAMA_SERVER` becomes `KNOWLU_RUNTIME`, reversing my own amendment.** I told
Task 3 to keep `resolve` "exactly as the brief writes it", and the implementer correctly did. But I
wrote that before the spike returned: the variable now points at `llama-cli.exe` in a module that
has no server and never will, and it is user-facing enough to reach a settings row and a README. The
review is right that this is the cheap moment â€” Task 4's brief hard-codes it twice and the plan four
more times, all of it text I still control. Carried into Task 4's dispatch; Task 12 sweeps the plan.

**R-3a-16 â€” no engine-side guard against an implied `--vault .`.** The reviewer suggested a
sentinel file or a wrapper refusing an implied vault. Rejected: a refusal added to `--vault`
resolution is a behaviour change to a command `scripts\diff-engines*.ps1` compares against Python
byte for byte, and Python has no such guard, so it would break parity; and a sentinel nobody has is
not a guard. R-3a-12 plus every dispatch carrying it is the control.

**R-3a-17 â€” the pipe-buffer test's deadline is `CALL_TIMEOUT`, and the 5-second version was my
error.** The first reviewer raised the 30 s deadline as *preference, not a defect*; I promoted it to
a fix on an unchecked assumption ("~150 ms when passing"). The scoped re-review **measured** it:
1.03â€“1.23 s idle, and **3 of 3 failures under 16 spinners on 16 cores, 3 of 3 under 32**, panicking
with `Failed("llama-cli: timed out")` â€” the exact message a genuine undrained-pipe regression
produces. A raw child took 39.6 s once under that load. So I had converted the one test whose job is
to detect that defect into one where a busy laptop and the defect are indistinguishable. **The
general lesson, worth more than the fix:** on a test that fails *by hanging*, the deadline is not a
bound on correctness â€” it is the failure path, and tightening it trades a slow failure for a lying
one. Cost of the fix: a real regression now takes 120 s to surface instead of 5. That is the right
trade and the comment in the test says so, because the next reader will want to shorten it too.

**R-3a-18 â€” a reader that stops reading is the bug this module exists to avoid, at any threshold
(re-review N3).** The 1 MiB cap stopped *draining*, not just storing, so a child writing more than a
megabyte would block in `WriteFile` again and be reported as a timeout â€” R-3a-6's failure
reintroduced higher up. Unreachable today at `-n 256`; fixed anyway, because removing the class
costs one line (`io::copy` into `io::sink` past the cap) and the instance-by-instance alternative is
how this defect got in twice already. N2 and N4 are the same shape: both readers joined on every
path, and the stderr tail reaching the error on the two paths that dropped it â€” including the
timeout path, where a stalled model load is most likely and llama.cpp's load progress is exactly
what a debugger would want.

**R-3a-19 â€” `parse_reply` scans candidates left to right, and that direction is load-bearing.**
The review's one substantive Minor was that the scan is quadratic in principle, safe only because
upstream clipping keeps inputs small. I ruled **no bound**, for a reason that matters more than the
cost: the two obvious mitigations are both worse than the thing they fix.
A *candidate cap* would be a false-negative risk on exactly our users â€” a CS assignment body pasting
code with sixty-odd braces ahead of the answer would push the real object past the cap.
*Reversing the scan* is the tempting one, because the answer always sits at the end after the echoed
prompt, so right-to-left finds it in one or two candidates. **It is also wrong**: an
`importance_reason` containing a JSON object with our own field names â€” `"like {\"effort_hours\": 9,
\"importance\": 5}"` â€” would be found first, parse cleanly, pass the required-field check, and be
returned as the answer, silently replacing the real one. Left-to-right reaches the true object's
opening brace first and is immune. A comment now says this at the scan, because the next reader will
try `.rev()`. Cost if wrong: milliseconds on a background pass, per item, against a 120 s timeout.

**R-3a-20 â€” `LowConfidence` gains a structural `cause`, reversing my own ruling on Task 2's
preference.** Task 2's reviewer said they would have split the transport failure into its own
variant "so a dashboard can distinguish 'the model is unsure' from 'the model is broken' without
string-matching", and I ruled *leave it*. They were right. The cost surfaced exactly where they
predicted: `entry_for` correctly drops `why` for privacy (on the `ModelFailed` branch it carries the
runtime's `stderr_tail`), so all three causes record as `low confidence` â€” and on a friend's laptop
with a runtime that installs but does not run, the judgment log would say *low confidence* twice a
day forever, in the file whose own doc says it exists to answer "why is nothing being enriched?".
That is R-3a-13 reappearing one layer up: Task 3 spent a whole fix round making a broken runtime
distinguishable from an uncertain model, and the log threw the distinction away.
So: `LowCause::{BelowFloor, Incomplete, ModelFailed}` on the variant, filled at the three
construction sites that already know structurally which is which, and `Entry.cause:
Option<&'static str>`. **Not** parsed back out of `why` â€” a `&'static str` from a closed set is the
whole safety argument, because there is no value it can take that came from a note, a model or a
process. The split is now explicit: **structured cause in the log, full text on stdout.** Cheapest
possible moment â€” Task 7 does not exist yet, so nothing consumes the variant. Cost if wrong: one
enum and a field, both removable. Task 7 and Task 8 dispatches must carry `LowConfidence { seed,
why, .. }`.

**R-3a-21 â€” the `fields` privacy check belongs to Task 7, and its dispatch carries it.** The review
was right that `no_note_text_ever_reaches_a_log_line` cannot construct the case it names: `Entry`
has no field for a title or a body, so the guarantee is the **type**, not the test â€” a good place to
be, but the test's doc implied more than it proved. What neither the type nor the test can defend is
a caller passing a note's title as the *value* of a `fields` tuple, since `fields` carries free text
verbatim by design. Task 7's dispatch and review get an explicit test for it: plant a token in the
note's title and body, run a full enrichment, assert it reaches no log line.

**R-3a-22 â€” no note this system writes may contain a YAML anchor, and this plan edits `engine/`
once to make that true.** The dual-run harness caught that PyYAML aliases by object identity:
`propose_amendment` passes the *same* `date` object to `proposed_at` and `first_proposed_at`, and
`ignore_aliases` exempts str/int/float/bool/None but **not** `date` â€” so every amend card Python has
ever written carries `proposed_at: &id001 <date>` / `first_proposed_at: *id001`. Task 6 reproduced
that faithfully by growing `yamlemit` an `Anchored`/`Alias` pair. The reviewer then found what it
sits one line away from: **`defer_over_budget` (`engine/approvals.py:577`) rewrites `proposed_at` by
single-line surgery on every pending non-digest approval, amend cards included**, leaving
`first_proposed_at` pointing at nothing. The card is then unparseable â€” `ComposerError: found
undefined alias 'id001'` â€” and every tolerant reader silently drops it, so it vanishes from the deck
*and* from `find_pending_amendment`, and the next run mints another.

Two facts I verified myself before ruling: it is **reachable** (only `events-digest` is excluded
from deferral), and it has **never fired** â€” no `&id` or `*id` appears in any note in the live
vault, so the routine's enrichment has evidently never re-judged a field Quinn had already set.
That is exactly why it must be fixed now rather than recorded: plan 3a's whole purpose is local
enrichment re-judging tasks Quinn has hand-estimated, so amend cards go from never to routine the
week Task 7 ships.

**The ruling is at the emitter, not the readers.** Teaching every reader about the YAML object graph
contradicts the invariant that a note is never parsed and re-dumped. One both-engines commit: one
line in `engine/write.py` giving the two fields distinct `date` objects (PyYAML aliases by identity,
so equal-but-distinct emits nothing), and `Anchored`/`Alias` deleted from `yamlemit` again â€” which
resolves review Minors m2, m3 and m5 by deletion. The guard is deliberately **not** "the card has no
`&id`", which is true by construction afterwards; it is *mint a card, rewrite `proposed_at` the way
`defer_over_budget` does, load it, assert it parses* â€” a test that fails today.
**This is the only edit to `engine/` in the whole plan**, so `pytest` joins the gate for this round
only. Cost if wrong: amend cards change bytes â€” but the live vault contains none, so no existing
note moves, and the dual-run scripts prove both engines still agree.

**Worth flagging to Quinn on merge**, because it touches the live engine during the cutover week:
the change is one line, affects only `kind: amend` cards, and of those the live vault has zero.

**R-3a-23 â€” the `QUINN_OPS_DEVICE` test race is fixed now, out of plan scope, because every
remaining gate depends on it.** The Task 6 scoped re-review reported `cargo test` intermittently red:
`runs::tests::start_step_end_records_full_text_and_fold_steps` failed on a full run, passed in
isolation and on re-run. Cause: `src/history.rs:428`'s `DEVICE_ENV_MUTEX` doc says it serialises
tests "in this module", but `QUINN_OPS_DEVICE` is global to the **process**, and `src/runs.rs`'s
tests read `device_name()` directly â€” their own comment at `:565â€“569` says so. A lock only the
writers hold protects nothing against a reader in another module.
Pre-existing, in modules this plan does not touch, and I fixed it anyway: **every task gate in this
plan is "`cargo test` green, 0 warnings"**, and an intermittently red suite trains whoever sees it to
re-run until green, which is precisely how a real regression gets waved through. The lock moves to
`journal.rs` beside `device_name`, crate-wide, and **readers take it too**. Folded in: the Rust
deferral guard now pins its date once instead of reading `now()` twice (re-review N2 â€” a midnight
rollover between mint and assert would have failed it for no defect). Verification is three
consecutive green runs, because one was never evidence here. Cost if wrong: test-only change, no
production signature touched, and the dual-run scripts cannot be affected.

**R-3a-24 â€” the vault slip has now happened twice; the answer is a checklist item, not a guard.**
Task 7's implementer, like Task 3's, ran an engine write against the worktree's own vault while
debugging, caught it, and reverted. Both times I verified independently and both cost nothing: at
`ea08f72` the tree is clean under `--untracked-files=all`, the commit is three source files, and
`git diff --name-only main...HEAD` names seventeen files with no vault path among them.
I reconsidered R-3a-16's rejection of a structural guard and it still holds â€” a refusal in `--vault`
resolution is a behaviour change to a command the dual-run scripts compare against Python byte for
byte, and Python has no such guard. What changes instead: **every remaining dispatch requires
`git status --porcelain --untracked-files=all` before committing, with the output pasted into the
report**, which turns "catch it if you happen to look" into a step, and I keep verifying. Cost if
wrong: the failure mode is an implementer who neither notices nor reports â€” which is exactly what my
own check catches.

**Environment note for Tasks 8â€“12: the dual-run scripts and this session's PowerShell tool.**
Task 7's implementer found the tool wrapping native stderr as a terminating error under the scripts'
own `$ErrorActionPreference = "Stop"`, and confirmed it reproduces on the **pre-Task-7 commit**, so
it is a harness artifact rather than a regression. Their workaround was a detached `Start-Process`.
Because that is the plan's main parity gate and the person who worked around the harness is not the
right person to confirm it, Task 7's reviewer is running `diff-engines-notes.ps1` independently.

**R-3a-25 â€” "does the failure text survive this boundary?" is now a standing question for every
remaining task.** Task 7's review found a Critical: the `why` suffix is appended only inside the
write's `Ok` arm, after the `literals.is_empty()` early exit, so an item that writes nothing prints
`nothing to write (low confidence)` with the runtime's stderr tail nowhere. That reads like a corner
and is not â€” `tier1` seeds nothing unless a note already names a course or its uid is pinned, and a
wizard-created vault has an **empty** `course_map` and `effort_source: inferred`, so on a friend's
broken-runtime machine **every** line takes that path. The existing test passed only because its
fixture is uid-pinned and takes the other arm. The reviewer reproduced it against a scratch vault.

**The pattern is the finding.** This is the third time the same diagnostic has been lost at a new
boundary: the **process** boundary (R-3a-13 â€” a failed `llama-cli` returned `Ok("")`), the **log**
boundary (R-3a-20 â€” three causes collapsed to one label), and now the **stdout** boundary. Each time
the code was locally reasonable; each time the friend with the broken runtime was the one who paid.
The remaining boundaries are **Task 9's slot step name** and **Task 10's settings row**, and both
dispatches will carry this question explicitly rather than hoping it is noticed.

Also ruled in this round: the field-set assertion is tautological (M1) â€” it reads names back out of
the `judgment:` block, which `write.rs` has already filtered to the judged fields, so it asserts
`write.rs`'s guarantee rather than `enrich.rs`'s, and a non-judged field added to `literals_for`
would be written and journalled while invisible to every assertion in the file. Bounded at the source
in the pure unit test instead. Plus: a test for production's real starting shape (`NOTE_TEMPLATE`
seeds `effort_confidence: low`, so F4 swallows it and the block names **four**, not the five the
module doc implies), a corrected test count (**873**, not 876 â€” the `main.rs` unit suite was
double-counted), and `--limit 0` becoming a usage error rather than a run that cannot progress.

**R-3a-26 â€” a manifest field is never a filesystem path (Task 8 C1).**
`install_from_manifest` built `root.join("downloads").join(&asset.name)` from a string deserialised
off the network with no invariant. `Path::join` **discards the base** for an absolute path and keeps
`..` for the OS to resolve, so a manifest naming a Startup folder or an absolute path sent
`download_to` there â€” `create_dir_all` the parents, rename over whatever was there, then delete it.
Arbitrary file overwrite **and** delete, as the user, and the digest check cannot help because the
bytes and the digest measuring them come from the same manifest. R-P3a-2 was never broken â€” a bad
runtime is still refused and never executed â€” but the module's own premise is that a manifest is
"never the root of trust", and treating one of its fields as a trusted path is precisely trusting it.
Fixed by reducing to a bare file name and **refusing** anything that was not already one, so a
manifest meaning the real asset is unaffected and one meaning anything else is an error the user
sees. Latent today; **Tasks 9 and 10 wire the Download button**, which is why it is fixed in the
module now rather than guarded at a call site later.

**R-3a-27 â€” the unchecked extraction half gets a source-scanning guard, because the next two tasks
are exactly where it would be routed around.** `extract_runtime_zip`'s "only
`install_runtime_from_zip` may call this" contract lived in a doc comment. Tasks 9 and 10 write the
`commands.rs` that calls into this module, and "the check already ran, call the extractor" is a
plausible mistake that would silently remove both the digest gate and the check-then-extract ordering
that bounds a zip bomb. Three lines, using the crate's existing source-scan precedent.

**R-3a-28 â€” the security claim is narrowed rather than the paths closed.** Three unverified execution
paths exist outside this diff: a hand-placed file in `runtime\`, `KNOWLU_RUNTIME`, and a sibling exe
beside the app. **None crosses a privilege boundary** â€” in each case whoever placed the bytes is
whoever runs the app â€” so closing them buys nothing and costs the escape hatches. The module doc's
absolute claim becomes the exactly-true one: *no bytes Knowlu itself fetches or installs reach
execution unverified.* A security claim that overstates by one word is worse than a narrower one that
holds.
Also ruled: TOCTOU between hash and extract is removed by class rather than mitigated â€” read the
runtime zip once, hash that buffer, extract from it â€” but **not** for the model half, where a `.gguf`
is hundreds of megabytes and is copied rather than extracted.

**R-3a-29 â€” a bare `git stash` reverted Task 8's security fixes in the working tree; my dispatches
never carried the rule that forbids it.** Task 9's implementer ran `git stash` / `git stash pop` to
take baseline test counts. CLAUDE.md is explicit that **the stash stack is shared with the main
checkout and every other worktree** and that bare stash/pop must never be used â€” and **none of my
dispatches said so**. The pop restored `f587b11`'s pre-fix `app/src/inference.rs` and
`app/tests/inference.rs` over the working tree *and staged them*, silently undoing the C1 manifest
path fix, `validate_asset`, the source-scan guard and the rest of the round in the working copy.

**Nothing was lost.** The fixes live in `12178ca` and are contained in HEAD (`97bd7f0`); Task 9's
commit used an explicit pathspec, so it never swept the reverted files in. I diagnosed it by
comparing the index against both commits â€” `git diff --cached f587b11` was **empty** and against
`12178ca` showed âˆ’280/+68 â€” restored with `git checkout HEAD --`, and confirmed: tree matches HEAD,
`validate_asset` present, `cd app; cargo test` **117 passed, 1 ignored**, only the pre-existing
`.rsrc` linker warning. The stash list is now empty.

**Two consequences.** First, every remaining dispatch carries the prohibition explicitly: never bare
`git stash`/`git stash pop`; take baselines with `git stash list`-free methods, or from a commit.
Second â€” **this was my error, not the implementer's**: I dispatched Task 9 and the Task 8 re-review
concurrently into one worktree, and while the re-review was read-only, the stash stack reaches
outside this worktree entirely. Concurrency stops here: **one tree-touching agent at a time**, and
read-only reviewers may overlap only when nothing else can write.
Also worth flagging to Quinn: because the stash stack is shared, it is possible â€” not confirmed â€”
that a stash entry belonging to another session or the main checkout was consumed. `git stash list`
is empty now.

**R-3a-30 â€” a stale working copy can corrupt a review, not just a build.** The Task 8 re-review was
mid-flight reading those very files while they held the pre-fix content, and would have reported
that C1 and most of the round did not land. I messaged it to discard any on-disk reading taken in
that window and re-read, noting that the **diff file it was given came from git and was always
correct**. Worth carrying: when a working tree is disturbed, the agents *reading* it are as affected
as the ones writing it, and they will report confidently either way.

**R-3a-31 â€” `validate_asset` becomes an allowlist, and a doc comment stops overstating.** The scoped
re-review confirmed **ALL FIXES LANDED** and the property holding, verified against `git show
12178ca:` rather than the working tree â€” which is why the stash corruption never touched its
verdicts. It found `validate_asset` refusing eight of eleven hostile name forms but accepting a
trailing-dot name, a trailing-space name and a NUL-embedded one. **None escapes `downloads\`**, so
C1's threat stays closed and this is not a re-opened Critical. Fixed as an allowlist rather than
three more refusals â€” the same reasoning as R-3a-18: remove the class, not the instance. I added one
the reviewer did not raise, **Windows reserved device names** (`CON`, `NUL`, `COM1`â€¦): a file named
`NUL` in `downloads\` silently discards every byte written to it, which would present as a corrupt
download nobody can explain.
Second item: the m1 doc comments claim the hash and the extraction are "provably the same
read/bytes". One handle defeats the delete-and-swap race, but Windows' default share mode still
permits in-place mutation between the two reads â€” so the true claim is *the same file, not a swapped
one*, with the residual window accepted because exploiting it needs local write access as the same
user. Precision in a security comment, for the same reason as R-3a-28.

**R-3a-32 â€” a fix found by an implementer still needs a test, especially when its regression is
invisible.** Task 10's implementer found a genuine bug in the plan: the onboarding-offer marker was
wired only into `apply_profile_settings` (adopt-vault), while create-vault and restore-vault share
the same finish-panel checkbox and never wrote it â€” so every friend who creates a fresh vault would
be re-offered the model download on every launch with their "no thanks" silently forgotten. It fixed
it correctly and flagged in its own report that no test covered it. **Flagging a gap is not closing
it**, and `app/tests/onboarding.rs` already imports the three seams needed. Six small tests, one per
path per answer. The reason this is Major rather than Minor: when this regresses nobody hears about
it â€” the product just nags, forever.

Also ruled in the same round: the wizard-never-installs guard test slices only `startWizard`'s
init-only body, so it would not catch an auto-download added in `wizFinish` or `wizRegister`, which
is exactly where a well-meaning one would go (the invariant does hold today â€” confirmed by
whole-file grep). And `installInference` derives which half to install by **parsing the row's own
UI text**, so once both halves are present a stray Download click silently re-fetches gigabytes with
the same "installingâ€¦" wording as a first install. Fixed at the root â€” take the half from the status
object, not from a string the code itself rendered â€” plus a **"Replace modelâ€¦"** label once ready,
because replacing a model is legitimate and should simply stop looking identical to installing one.

---

## Task log

**Task 1: complete.** Spike, throwaway, docs-only. Commits `0b9c120` (the report) and `5d61291`
(the review's corrections). Outcome **B**. Also recorded, for later tasks: the zip is **flat** (no
`bin/`), so Task 8's extraction walk finds `llama-cli.exe` at the top of the extracted tree; a
`127.0.0.1` listener raised **no** firewall prompt; and a killed server's port re-binds on the
first try. Reviewed by the controller (R-3a-5): spec âœ…, quality âœ… after the two corrections.
Outcome B's routing â€” Task 3 loses `Server`/`for_port`/`free_port`/`wait_healthy`/`LOAD_TIMEOUT`
and the four loopback tests and gains `PerCall`; Task 4 step 5 implements `Model` for `PerCall`;
Task 7's `run_lines` constructs `PerCall`; Task 8 unaffected â€” is carried in each dispatch.

**Task 2: implemented, reviewed, fix round 1 dispatched.** Commit `7c5672d` (BASE `5d61291`).
812 lib tests / 0 failed / 2 ignored, 0 warnings, +12 new; both oracles green; both dual-run
scripts exit 0. Review (`task-2-review.md`, opus): **spec âœ…**, **quality approved with findings** â€”
0 Critical, 2 Major, 9 Minor, and it established by extraction-diff that the implementation and
tests are **byte-identical to the brief**, so every finding is a defect in the brief's own code.
Fix round 1 brief: `task-2-fix-1.md`. Rulings R-3a-8â€¦R-3a-11 below.

Process note: `task-2-brief.md` did not exist when Task 2 was dispatched â€” my batched `task-brief`
call had been refused by the worktree guard and I did not re-run it. The implementer fell back to
the plan's own Task 2 section, which is the same text, so nothing was lost. Briefs are now
generated one at a time, immediately before each dispatch.

**Task 2: complete.** Commits `7c5672d` (implementation) + `57503f9` (fix round 1). 816 lib tests,
0 failed, 2 ignored, 0 warnings; both oracles green; both dual-run scripts exit 0. Scoped re-review
(`task-2-rereview.md`): **ALL FIXES LANDED**, no new finding. It confirmed the two that mattered
were done to the strong spec rather than the easy one â€” M1's test asserts the course *survives on
the Outcome*, not merely that `knows_course` is true, and m5's four failure paths (no frontmatter,
unparseable YAML, empty `slug:`, non-string `slug:`) were each checked for panics. The three
`trim()` sites, the `NoRules` pin, the temp-dir convention and the `LowConfidence` shape are all
confirmed untouched, so no ruled "no change" leaked into the fix.

**Task 3: dispatched**, BASE `57503f9`, with `task-3-outcome-b.md` as a controller amendment that
wins over the brief wherever they disagree â€” the brief is written for the loopback server the spike
rejected. It carries R-3a-1 (why B), R-3a-2 (`--single-turn`) and R-3a-6 (the reader thread),
names the four loopback tests to drop and the two to keep verbatim, renames the `Drop` test to
`kill_tree_ends_a_process_and_its_children`, and specifies `wait_and_read` as a private seam so the
pipe-buffer fix gets two tests that need no model: one spawning `powershell` to write past the
buffer (which hangs and fails without the thread â€” the test is load-bearing, not decorative) and
one for the timeout path that also proves `kill_tree` reaps.

**Task 3: complete.** Commits `dac8fc8` (implementation) + `f2753f3` (fix round 1, eleven items) +
`b0940ec` (fix round 2, four items). 821 tests, 0 failed, 2 ignored, 0 warnings; both oracles green;
both dual-run scripts exit 0. Review: **spec âœ… zero drift**, quality approved with 0 Critical / 2
Major / 9 Minor. Scoped re-review of round 1: all ten landed, plus four new findings including one
Major that was **mine** (R-3a-17). Round 2 I verified myself rather than spending a fourth review
seat â€” `wait_and_read` is one screen, and I read every path: pipe-take failure kills before any
thread exists; `try_wait` error kills and joins both; timeout kills and falls through to the joins;
both readers are joined before the status is judged; all five error paths carry the stderr tail; and
`read_capped` takes 1 MiB then `io::copy`s the remainder into a sink, so it never stops reading
before EOF. The implementer agreed with the 120 s ruling on the merits and noted the test never gets
near it in practice.

**Task 4: dispatched**, BASE `b0940ec`, with `task-4-outcome-b.md` as the controller amendment. It
carries R-3a-7a (the corrected `parse_reply`), the `PerCall` substitution for `Server`, the
`KNOWLU_RUNTIME` rename the brief still gets wrong at its lines 316 and 318, and a note that
`prompt_for`'s `if !weights.is_empty()` guard became load-bearing under R-3a-8.

**Task 4: complete.** Commits `b300e3a` (implementation) + `778b2c3` (fix round 1, two Minors).
833 tests, 0 failed, **3 ignored** (traps 4 and 5 plus the new real-runtime smoke test), 0 warnings;
both oracles green; both dual-run scripts exit 0. Review: **spec âœ… fully compliant** â€” `GRAMMAR` and
`prompt_for` mechanically diffed as byte-identical to the brief, `parse_reply` on the amendment's
corrected approach rather than the brief's superseded anchor â€” and **quality solid, 0 Critical, 0
Major, 3 Minor**. The reviewer hand-traced the parser against all four noise cases rather than
trusting the tests, and confirmed each test does real distinguishing work: `{TBD}` and trailing text
defeat a whole-string first-to-last parse, the stray `{"a":1}` defeats a field-checkless scanner,
and braces-in-a-string defeats the superseded last-`{` anchor. No re-review of the two-item fix
round â€” a comment and one test, and the count moved 832 â†’ 833 exactly as expected.

**Task 5: dispatched**, BASE `778b2c3`. No amendment needed: `judgelog` is independent of the spike
outcome. The dispatch leans on the one constraint the task exists for â€” ids and field values, never
note text â€” and asks for the `no_note_text_ever_reaches_a_log_line` test to be built as a real one
(a distinctive token in body and title, asserted absent from the whole line) rather than a
happy-shape check that would pass an implementation which also serialised the body.

**Task 5: complete.** Commits `b9cf73a` (implementation) + `626ca47` (fix round 1). 839 tests,
0 failed, 3 ignored, 0 warnings; both oracles green; both dual-run scripts exit 0. Review: **spec âœ…
zero drift**, quality good, 0 Critical / 0 Major / 3 Minor, and a field-by-field table confirming no
note text can reach a log line by any path. No scoped re-review â€” instead I checked the one thing a
"three causes produce three distinct lines" test would *not* catch, which is the three causes being
assigned to the wrong branches: `src/judge.rs:391` `ModelFailed` on the `model.judge` Err arm, `:396`
`BelowFloor` under the floor, `:409â€“412` `Incomplete` on `!merged.complete()`, and the tests at
`:731/:750/:765` pin each mapping rather than just distinctness.

**Task 6: dispatched**, BASE `626ca47`, on opus â€” the highest-risk task in the plan and the only one
that changes the write path **both engines share**. It ports `engine/write.py`'s `propose_amendment`
and deletes `WriteError::ProposeNotPorted`, which Task 7 hits on its first re-judgement of anything
Quinn has already judged. The dispatch names the three fidelity traps the plan's pre-execution
review caught â€” Python's `if evidence:` truthiness (`{}` is falsy and appends nothing, where a
`!is_null()` filter would write an empty block), `json.dumps` insertion order against
`dumps_value`'s sorted keys (which is why the dual-run evidence mapping is deliberately one key),
and the six random hex characters in the proposal's filename that reach both the journal `create`
record's `path` and stdout and must be masked â€” plus the `w5`/`w6`/`w7` name collision that would
have silently deleted three existing dual-run cases, and the PowerShell 5.1 `"""` quoting that has
already cost this repo a session.

**Task 6: complete.** Commits `a7b8ba6` (the port) + `fe7186c` (fix round 1, the anchor ruling).
860 Rust tests / 3 ignored / 0 warnings; **pytest 688, up from 687, with no existing Python test
changed** â€” the `engine/write.py` edit alters object identity and no observable value. Both dual-run
scripts exit 0 with `w13`â€“`w15` and all twelve pre-existing cases; the trio now proves the
**absence** of the anchor, which is the stronger claim. Review: spec âœ… PASS, quality PASS, 0
Critical / 1 Major / 6 Minor. Scoped re-review: **ALL FIXES LANDED**, with `date.replace()` confirmed
non-identical on the pinned CPython for both `date` and `datetime` (type, value, microseconds and
tzinfo preserved), nine comment lines naming `defer_over_budget` so it cannot be mistaken for dead
code, and **both guard tests confirmed to fail before the commit** by rebuilding the pre-fix card out
of tree. The implementer went past one instruction correctly: the review's own `title` snippet gave
`"true"` where Python's f-string gives `"True"`, so it added a `Bool` arm and documented the one
remaining bounded gap. Its own summary of the round is worth keeping: *"my first round was faithful
to Python and wrong about the system â€” 'match the reference' is the rule because the reference is
correct, and here it wasn't."*

**Out-of-band fix: commit `d52e99b`** (R-3a-23). Three green `cargo test` runs. It found **three**
previously-unguarded readers, not the one the re-review named: `runs::tests::start_step_end_â€¦` plus
two `issues.rs` tests reaching `device_name()` through `open_issue`. It also reported honestly that
it **could not reproduce** the original failure in 18 full runs, 10 of them at `--test-threads=200`
â€” the right thing to say rather than claim a reproduction it did not have.

**Task 7: dispatched**, BASE `d52e99b`, with `task-7-amendment.md`. Five things changed under the
brief while Tasks 1â€“6 ran, and the dispatch names the five that decide correctness: exit 0 always,
the `agent:knowlu.enrich` actor, full failure text to **stdout** with only the structured cause to
the log, R-3a-21's privacy test, and counting `res.proposal` rather than only `res.skipped`.

**Task 7: complete.** Commits `ea08f72` (implementation) + `78c4ec3` (fix round 1, five items).
876 tests / 3 ignored / 0 warnings; both dual-run scripts exit 0, the notes script confirmed
**independently by the reviewer** at 42 comparisons all `same:`. Review: spec âœ… PASS, quality approve
with one required fix â€” 1 Critical, 1 Major, 3 Minor. The C1 fix was verified live against a scratch
vault with an empty `course_map` and a broken fake runtime: the line now carries the stderr tail
where it previously vanished. I checked the three sites myself (`:217` empty-literals, `:246` the
`Ok` arm, `:252` the `Err` arm) rather than take the report's word, since a hoisted suffix landing in
two of three places would look identical in a summary. One deviation, correct: the reviewer's
`clap::value_parser!(usize).range(1..)` does not compile on clap 4.6.6 (`usize` has no
`ValueParserFactory`), so `RangedU64ValueParser::<usize>` was used instead.

**Task 8: dispatched**, BASE `78c4ec3`. Carries R-3a-3's measured seed values, the flat-zip fact (the
`ggml-*.dll` must land beside the exe or it will not start), the ban on hard-coding
`â€¦/releases/latest/download/â€¦`, R-P3a-2's no-unverified-execution property with `install_runtime_from_zip`
as check-then-extract and the only thing production calls â€” and **R-3a-25 as a standing question**,
pointed at this task's own boundary: an unrecognised digest must be refused *with the computed digest
printed*, because that is the only way an unrecognised build ever gets added deliberately.

**Task 8: complete.** Commits `f587b11` (implementation) + `12178ca` (fix round 1: 1 Critical, 3
Major, 8 Minor) + `7043174` (fix round 2: the allowlist and the doc precision). app 97 â†’ 115 â†’ 117
tests, zero new warnings; repo root unaffected at 876/3/0. Review: spec âœ… PASS, quality PASS with
findings, led by a nine-row enumeration of every path bytes can take into `runtime\`. Scoped
re-review: **ALL FIXES LANDED**, property holding, verified from `git show 12178ca:` throughout â€”
which is the only reason the stash corruption (R-3a-29) did not poison its verdicts.

**Task 9: implemented** at `97bd7f0` and under review. app 115 â†’ 117; repo root unaffected. The
three step names it reports: `judge`, `judge (skipped: no runtime)`, `judge (skipped: no model)` â€”
verified by the implementer against a scratch vault via `scratch-vault.ps1` and `--run-slot-once`,
which returned `["judge (skipped: no runtime)", 0]` with `engine_ok` unaffected by the skip.
Its commit used an explicit pathspec, which is why the concurrent stash damage never entered it.

â–¶ RESUME HERE â€” Task 9 review running (range `12178ca..97bd7f0`; note `7043174` sits on top). Then
Task 10 (`task-10-amendment.md` is written and staged â€” the settings row is the **last** boundary
where the runtime diagnostic can be lost, and the refusal text with its computed digest must reach
the user). Then Task 11 (`starvation.ps1`, read-only, and its `.EXAMPLE` must not hard-code a path
under `C:\Users\danie\`), then Task 12 (close: docs, the spec's Â§5.2 actor-name and Â§5.3
amendments, and this plan's status line). **All three briefs are already extracted.**
Every remaining dispatch carries: the bare-`git stash` prohibition (R-3a-29), the clean-tree check
pasted into the report, and one tree-touching agent at a time.
(`app/src/inference.rs`), whose dispatch must carry R-3a-3's measured seed values for
`SUPPORTED_RUNTIMES` (tag `b10840`, asset `llama-b10840-bin-win-cpu-x64.zip`, 18,417,566 bytes,
SHA-256 `7063dfc6b874e7eee0ddf601bdf8e70e6f4a3d708926641ffad046ec51e8e30b`), the fact that the zip is
**flat** so the extraction walk finds `llama-cli.exe` at the top, and that
`â€¦/releases/latest/download/â€¦` must never be hard-coded because that release is a nightly pointer
carrying no binaries.

---

# Appendix A - whole-branch final review

# Knowlu plan 3a â€” whole-branch final review

**Branch** `worktree-knowlu-plan-3a`, HEAD `dfd86f7`, base `main` at `02cc688`. 25 commits, 40 files,
+5,647/âˆ’97. Read-only review; every verdict below was taken from `git show dfd86f7:<path>`, not from
the working tree.

---

## Verdict

**Safe to merge.** The one edit to the live Python engine (`engine/write.py`) is a one-line bug fix
inside `propose_amendment`, it changes bytes only on `kind: amend` cards, the live vault has none
(`grep -rln "kind: amend" approvals/ archive/ tasks/` is empty, and so is `&id00` across every note
directory), and the cutover harness never reaches that path â€” `scripts/dual-run.ps1`'s `$steps` are
`coursework` and `rank` only. Nothing on the branch touches the cloud routine, its prompt,
`config/runners.yaml`, `$mode`, or any vault note or journal line in any of the 25 commits; the only
non-`src`/`app`/`docs`/`scripts` path in the whole diff is `engine/write.py`. I re-ran the full gate
myself: engine `cargo test` **880 passed / 3 ignored / 0 warnings**, app `cargo test` **126 passed /
1 ignored** with only the pre-existing `.rsrc` linker warning, `pytest` **688 passed**, both oracles
green, `diff-engines.ps1` clean on all three fixtures, and `diff-engines-notes.ps1` clean **including
the new `--propose` trio (w13â€“w15)** â€” which is the byte-for-byte proof that the two engines now mint
identical amend cards. No frozen reference was regenerated. The security property and the data-
minimisation property both hold end to end. Nothing below is a merge blocker; findings 1 and 2 should
be fixed before the app's scheduler is ever switched on for a vault with a model installed.

**Findings: 0 Critical, 3 Major, 9 Minor.**

---

## Major

### M1 â€” A wedged runtime turns the judge step into the amber tray it was designed to prevent

`src/enrich.rs:59` (`DEFAULT_LIMIT = 50`), `src/runtime.rs:61` (`CALL_TIMEOUT = 120 s`),
`app/src/scheduler.rs:27` (`CHILD_TIMEOUT = 20 min`), `app/src/scheduler.rs:489â€“495`.

`runtime.rs`'s doc argues the per-call bound is what "keeps a wedged process from eating the whole
slot", and it is right about **one** call. It is not composed with the batch. `50 Ã— 120 s = 100
minutes` against a slot child budget of 20 minutes: eleven consecutive timed-out calls are enough.
`run_child` returns `-2` on its deadline, `run_slot_inner` sets `engine_ok = false`, and
`attach_scheduler` paints the tray amber and puts the slot into retry backoff â€” twice a day, forever.
That is precisely outcome D7 and the "judge always exits 0" invariant failing on the one machine the
whole design was written for: a friend whose runtime installs but does not run. It is also reachable
without a wedge â€” 100 flagged tasks after a semester's first ingest, `limit 50`, a 4 B model on a
low-end laptop at ~25 s a call, is 1,250 s.

Every layer is individually correct; the gap is between two tasks, which is why the per-task reviews
could not see it.

**Smallest fix:** give `enrich::enrich_with` a wall-clock budget alongside `limit` â€” stop the loop
once `started.elapsed()` exceeds, say, 15 minutes, and fold the untaken items into the existing
`"{left} left for the next slot"` line, which already exists and already means exactly this. (Cutting
`DEFAULT_LIMIT` to 9 would also close it arithmetically, but at the cost of the batch size.)

### M2 â€” The judge-skip slot test reads the developer's real `%LOCALAPPDATA%` and will start failing the first time this branch's own feature is used

`app/tests/scheduler.rs:90` (`a_machine_with_no_model_records_the_judge_skip_and_stays_green`).

`run_slot_inner` resolves the judge through `judge_state(cs)` â†’
`commands::inference_root()` â†’ `state::app_data_root()`, which reads the process's real
`LOCALAPPDATA`. The test has no seam for it. The moment Quinn uses the settings row this branch just
shipped to install a runtime **and** a model into `%LOCALAPPDATA%\knowlu\{runtime,models}`,
`judge_state` returns `Ready`, no skip step is pushed, and
`.expect("a judge step, named")` panics. `judge_state_in`'s own doc names this exact failure mode
("a test that breaks when the feature starts working is worse than no test") â€” the Task 10 review fix
introduced the seam and used it for the unit test, then left the integration test on the unseamed
path. Secondary effect: `app_data_root()` calls `profiles::migrate_flat_layout` on the real root, so
every run of the app test suite now performs a migration attempt against the user's live app data â€”
a side effect `run_slot_inner` did not have before this branch.

**Smallest fix:** set `LOCALAPPDATA` to a temp directory for the duration of the call, under the
existing `ENGINE_ENV_LOCK` (both variables are process-global and the test already holds that lock
across `run_slot_inner`). Restore it in the same block.

### M3 â€” CLAUDE.md's test numbers are stale and one of its stated invariants is now false

`CLAUDE.md:326â€“331`.

Task 12's job was to make the docs true of the code, and it recounted the `#[tauri::command]`s
correctly (verified: 26 in `commands.rs`, 11 in `onboarding.rs` = 37; the console handler list has 29
entries, the shell's 11 â€” all four numbers in the new text are right). It did not touch the test
block four sections down:

- line 327: `cargo test` "**814 passing, 2 ignored**, 0 warnings, 2026-09-06" â€” actual **880 passing,
  3 ignored, 0 warnings**.
- line 328: `cd app; cargo test` "**87 passing**, 1 ignored, 2026-09-06" â€” actual **126 passing, 1
  ignored**.
- lines 330â€“331: "**The two ignored tests** are cross-cutting traps 4 and 5, each named in its own
  `#[ignore]` attribute; neither may be un-ignored by changing the assertion." There is now a **third**
  â€” `src/runtime.rs:484`, `real_runtime`, the `KNOWLU_RUNTIME`/`KNOWLU_MODEL` smoke test â€” and this
  sentence is a rule, not a statistic. A future reader who finds three ignored tests and this sentence
  has to guess which one is the intruder.

**Smallest fix:** update the three numbers to 880/3, 126/1 and 688 (Python is also stale at 687), and
rewrite the invariant as "two of the three ignored tests are traps 4 and 5 â€¦ ; the third is
`runtime::tests::real_runtime`, which needs a real runtime and model and is run by hand."

---

## Minor

### m1 â€” The live cutover harness still says `--propose` is unported

`scripts/dual-run.ps1:153â€“154`. The comment reads "*propose_amendment is unported by decision*" and
the guard finishes `"SKIPPED refused --propose"`. Both are now false. The guard is inert â€” `$steps`
is hard-coded to `coursework` and `rank`, neither of which can ever carry the flag â€” so this is a
correctness-of-documentation problem, not a behaviour one; but it is a file read during cutover week,
and `diff-engines-notes.ps1`'s header was updated while this one was not. **Fix:** update the comment
to say the path is ported and this harness deliberately does not exercise it (`diff-engines-notes.ps1`
w13â€“w15 does), or delete the now-unreachable guard.

### m2 â€” `check_runtime_supported` is public, uncalled and untested

`app/src/inference.rs:250`. The plan's fidelity ledger D6a names it as a carrier of ruling R-P3a-2,
and the module doc reads as though it is one of the two gates. It has no caller anywhere in
`app/src/` or `app/tests/` â€” the actual production check is the inline `runtime_release_for(&got)` in
`install_runtime_from_zip` (`app/src/inference.rs:329â€“332`), which is correct and is tested by
`a_runtime_whose_digest_is_not_in_the_table_is_refused_by_the_production_path`. So the security
property holds, but the function the ledger points at is dead and unexercised, and could rot out of
agreement with the path that matters. **Fix:** either give it a test that pins its verdict and message
against `install_runtime_from_zip`'s for the same file, or delete it and correct D6a.

### m3 â€” The M3 source-scanning guard names three files rather than a rule

`app/tests/inference.rs:241â€“246`. It scans `src/commands.rs`, `src/onboarding.rs`,
`src/scheduler.rs`. A future caller in `src/main.rs`, `src/tray.rs`, `src/updates.rs` or a new module
escapes it silently. **Fix:** walk `app/src/*.rs` and skip `inference.rs`, so the guard states the rule
("nothing outside this module calls the digest-free half") rather than an enumeration of today's files.

### m4 â€” Stale `llama-server` naming in shipped text after the spike chose `llama-cli`

`src/judge.rs:928` is a doc comment asserting "llama-server rejects a malformed rule set and every
judgment then fails identically" â€” a claim about a process this branch does not start.
`src/judge.rs:762`, `src/enrich.rs:577` and `src/judgelog.rs:234` use `"llama-server: â€¦"` as scripted
error text, and `app/tests/scheduler.rs`'s `JudgeArgs` fixture is `C:\rt\llama-server.exe`. None is a
behaviour defect; all five will read as evidence of a loopback server to the next person. **Fix:**
`llama-server` â†’ `llama-cli` in all five.

### m5 â€” `literals_for` writes `effort_confidence: low` over a vendor-stated effort

`src/enrich.rs:145`. The literal is pushed whenever `v.effort_hours.is_some()`, and tier 1's vendor
branch (`src/judge.rs:339`) sets `effort_hours` from the note when `effort_source: vendor` â€” so a
vendor-sourced note reaching this pass would have `coursework`'s `effort_confidence: high`
(`src/coursework.rs:1276` calls that "written under a vendor's authority") rewritten to `low` while
the hours stay the vendor's. Unreachable today: only `ingest::NOTE_TEMPLATE` emits
`needs_enrichment: true`, and it emits `effort_source: inferred`. The module doc explains why `low` is
right for a model estimate but does not notice the tier-1 vendor case. **Fix:** push
`effort_confidence` only when the effort came from tier 3 (`v.tier == 3`), not whenever it is present.

### m6 â€” `starvation.ps1` mixes a UTC day with a local day

`scripts/starvation.ps1:71` buckets by `$rec.ts.Substring(0,10)` (journal `ts` is UTC) and
`scripts/starvation.ps1:83` compares against `(Get-Date).Date` (local). An 18:00 CT enrichment is
already tomorrow in UTC, so it lands in a bucket the table prints as "tomorrow" and the seven-day gap
can be one day out â€” in the direction that declares the routine starved a day early. **Fix:** compute
`$today` as `[datetime]::UtcNow.Date`, or convert the record's `ts` to local before slicing.

### m7 â€” `tests/starvation.rs` invokes the script by relative path

`tests/starvation.rs:26` passes `-File scripts/starvation.ps1`, so the test only passes when `cargo
test` is run from the repo root. Every other test in the crate builds its own absolute temp paths.
**Fix:** join `env!("CARGO_MANIFEST_DIR")`.

### m8 â€” "Remove model" is visible for one frame before the status is known

`app/static/index.html:126` ships `<button class="b" id="set-judge-remove">` with no `hidden`
attribute; `renderInference` (`app/static/console.js`) sets `.hidden` on every branch, but only after
`inference_status` resolves. On a fresh install the reader sees "Remove model" offered over a blank
status. **Fix:** add `hidden` to the button in `index.html`.

### m9 â€” The spec's decision-12 row dropped "the app runs with no model"

`docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md:452`. The rewritten row 12 now
reads "llama.cpp as a **separate process**, downloaded and hash-verified into app data, not bundled
and not linked" and no longer carries the clause it replaced, "the app runs with no model". That
clause is D7 â€” the reason `judge` exits 0 and the reason the tray stays green â€” and the decisions
table is the index people read. It survives in Â§5.3's body and in D7, so nothing is lost outright.
**Fix:** append "; the app runs with no model" back onto row 12.

---

## The four questions the brief asked, answered

**Live-system safety.** `engine/write.py` is the only file outside `src/`, `app/`, `docs/`, `scripts/`
and `tests/` in the whole diff, and the only edit to `engine/` in the branch. The change gives
`first_proposed_at` `today.replace()` â€” an equal but distinct `date` â€” so PyYAML's identity-keyed
aliasing stops emitting `&id001`/`*id001`. `today` is typed `date` and defaulted `date.today()` at
`engine/write.py:205`, so `.replace()` is total; the dict value is `==` to before, so no other caller
sees any difference; `propose_amendment` is called from exactly one place (`write_literals`, only when
the journal shows Quinn set the field and `propose=True`), and its `yaml.safe_dump` is the only
frontmatter in the system that had two `date` objects in it. Both halves of the claim verified: bytes
change only for `kind: amend` cards, and the live vault holds none â€” no `amend-*.md` in `approvals/`
or `archive/`, no `kind: amend` and no `&id00` anywhere under `approvals/ archive/ tasks/ info/
issues/ courses/`. Nothing in the branch can write the live vault when run: `judge` writes only the
vault it is given, no runner script was changed, `scripts/local-run.ps1` and `scripts/dual-run.ps1`
are untouched, and the app's scheduler still stands down unless `config/runners.yaml` says
`scheduler: app` for the named device (`mode`/`device_ok` are not in the diff). No vault data or
journal line appears in any of the 25 commits.

**Composition.** I traced `quinn-ops judge` against the shipped code. `main.rs:331` destructures the
clap arm and calls `enrich::run(&vault, &via, run_id, runtime, model, log_dir, limit)` â€” argument
order matches the signature at `src/enrich.rs:290` exactly; `--via` is validated against
`journal::VIAS` at the CLI, and `scheduler::slot_argv` passes `local-runner`, which is in it.
`run_lines` calls `runtime::resolve` once per run, hands `enrich_with` either `Err(Missing)` or a
`PerCall`, and `judge_task` reaches `PerCall::complete` â†’ `prompt_for` + `GRAMMAR` â†’ `parse_reply`,
with the grammar written to a per-call temp file and both pipes drained on threads. `literals_for`
feeds both `judgelog::entry_for` and `write::write_literals` from the same slice, so the log and the
note can never disagree about what was written. The `--runtime/--model/--log-dir` flags
`slot_argv` emits match the clap names one for one. **The one seam where two tasks agree in their
tests and disagree in production is M1** â€” Task 3's per-call bound and Task 7's batch limit multiply
past Task 9's child timeout, and no test spans all three.

**The security property.** It holds in the branch's final state. Both production entry points reach
`install_runtime_from_zip` (`install_from_file` â†’ directly; `install_from_manifest` â†’ after a capped,
manifest-digest-verified download), and that function hashes from **one open handle**, checks
`runtime_release_for`, and only then rewinds and extracts â€” so the check cannot be skipped at a call
site and cannot be raced by a delete-and-rename between two opens. The digest-free
`extract_runtime_zip` is `pub` for tests only and is guarded by a source-scanning test (see m3 for its
one weakness). The manifest is never the root of trust: `validate_asset` refuses a non-`https://` URL
and anything that is not a plain file name (allowlist, including trailing dot/space, NUL and reserved
device names), and for the runtime half the manifest's own digest proves only the transport while the
compiled-in table decides execution. `SUPPORTED_RUNTIMES` carries exactly the spike's measured entry â€”
tag, asset name and SHA-256 all match `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`
byte for byte â€” and a test pins it. The three unverified paths in `runtime::resolve` are the recorded,
reasoned exception and cross no privilege boundary; the module doc's own hedge (fix round 1, m9) states
the narrower claim accurately.

**Data minimisation.** It holds structurally, not by discipline. `judgelog::Entry` has no field that
can hold a title, a body or a prompt; `cause` is a `&'static str` from a closed three-value enum, so
the branch that carries the runtime's stderr (`LowConfidence::why`) has nowhere to land. I traced every
path out of an enrichment run: `why` reaches `lines` and therefore stdout and the app's slot log in app
data â€” never the log, never a literal, never the vault. `inputs` (`{source_uid, title_seen}`) goes into
the note's own `judgment:` block in the vault, exactly as the routine's does, and not into the log.
`skipped` lines name the file and go to stdout. The only thing in `fields` that a model could
paraphrase from a note is `importance_reason` â€” and that is a field value written into the note's own
frontmatter in the open, which is what Â§5.4 licences ("ids, field values and confidences"). So: **no,
a note's title or body cannot reach a log line by any path**, and the one derived value that can is the
one the note itself publishes.

## Global Constraints, one by one

| Constraint | Verdict |
|---|---|
| Scratch copies only; never the live vault; no interactive `knowlu.exe` | **Holds** â€” every test builds its own temp vault; both dual-run scripts refuse the working tree |
| No network, no model download in any test | **Holds** â€” the only network-shaped test is the `#[ignore]`d `real_runtime`; no test binds a socket (outcome B removed the loopback seam entirely) |
| Every note write through `write`; `rank` never calls a model; the read model never writes | **Holds** â€” `src/cli.rs` is not in the diff; `judge`/`runtime` are unreachable from it; `surface_oracle` green |
| `cargo test` at repo root, 0 warnings; both oracles green | **Holds, verified** â€” 880 passed, 3 ignored, **0 warnings**; `oracle.rs` and `surface_oracle.rs` green |
| `cd app; cargo test` at zero *new* warnings | **Holds, verified** â€” 126 passed, 1 ignored, only the pre-existing `.rsrc` linker line |
| Both dual-run scripts exit 0 | **Holds, verified** â€” `diff-engines.ps1` clean on `vault-s1`, `vault-s1-migrated`, `vault-full`; `diff-engines-notes.ps1` clean including w13â€“w15 |
| The eight Python references and the three `surface-today-*.json` never regenerated | **Holds** â€” `tests/fixtures/` has no entry in the diff at all |
| No new `uievents::ACTIONS` action | **Holds** â€” `src/uievents.rs` is not in the diff; `ACTIONS` is still `[&str; 11]` |
| No `http://`/`https://` under `app/static/` | **Holds** â€” grep of `console.js` and `index.html` finds none; `MANIFEST_URL` lives in `app/src/inference.rs` and a test asserts the page never names it |
| No single-user assumption | **Holds** â€” no `danie`, no `C:\Users\`, no machine or person named in `src/`, `app/src/`, the new scripts or the spike report; every directory derives from `--vault`, an argument, an env var or the app-data root |
| No secret in any log, note, fixture or test name | **Holds** â€” the one "secret"-named string is a fabricated stderr line in a `judgelog` test |
| Line endings per file; no whole-file flip | **Holds** â€” `git diff --stat` shows no file whose every line changed |
| Nothing edits the routine, `runners.yaml`'s `scheduler:` key, `$mode`, or `engine/` beyond the anchor fix | **Holds** |
| `journal::VIAS` does not grow | **Holds** â€” `judge` passes `local-runner` |
| Actors start with `agent:` | **Holds** â€” `enrich::ACTOR = "agent:knowlu.enrich"`, and the spec's `knowlu/enrich` defect is amended by Task 12 |
| Judged fields âŠ† `JUDGED_FIELDS_TASK` âˆª `{needs_enrichment}` | **Holds** â€” `literals_for` emits exactly `course, effort_hours, effort_confidence, importance, importance_reason, needs_enrichment` |

## Docs against reality

Verified true: the command counts (37 = 26 + 11; console registers 29, shell 11 â€” recounted from both
`generate_handler!` lists as CLAUDE.md instructs); the slot order `coursework â†’ ingest â†’ judge â†’ rank`
in `slot_argv`; the two skip names, `judge (skipped: no runtime)` and `judge (skipped: no model)`,
spelled identically in `scheduler.rs:477â€“479`, `CLAUDE.md`, `app/README.md` and the tests; the runtime
tag, asset name, byte size and SHA-256 quoted in `CLAUDE.md`, `docs/HANDOFF.md`, `app/README.md`,
`src/runtime.rs` and `app/src/inference.rs`, all matching the spike report; `docs/surface/anatomy.md`'s
"seven rows" (index.html has seven `.set-row`s) and its `rank`-never-calls-a-model row. **Not true:**
the test counts and the two-ignored-tests rule (M3).

## Dead ends and leftovers

`check_runtime_supported` is the only uncalled `pub` item (m2). No `TODO`, `FIXME`, `todo!()` or
`unimplemented!()` anywhere in the new modules or scripts. `WriteError::ProposeNotPorted` is gone from
the code; the two surviving mentions are in historical build reports, where they belong, plus one
correct backward reference in a `src/write.rs` test doc. The only design-that-did-not-ship references
are the five `llama-server` strings in m4. One commit on the branch (`d52e99b`) is out-of-plan work â€”
a crate-wide `QUINN_OPS_DEVICE` test lock, labelled "console plan 1 fix" â€” but it is test-only, it
fixes a real parallel-test race in `issues.rs`/`runs.rs`/`history.rs`, and it changes no production
behaviour.

---

# Appendix B - scoped re-review of the final fix wave

# Knowlu plan 3a â€” scoped re-review of the final fix wave

Target: `worktree-knowlu-plan-3a` @ `eea4432` (one commit on `dfd86f7`).
Scope: the twelve ruled items in `final-fix-wave.md`, the two reported deviations, and three checks
against the rest of the tree. **Not** a re-review of the branch.

Every content verdict below was read with `git show eea4432:<path>`, never from disk.

**Verdict: 11 landed, 1 partially landed (M2). No Critical or Major new finding. Merge is not
blocked by the partial â€” the failure mode M2 was written to prevent is closed; what survives is the
side effect the ruling named as its second reason.**

---

## The twelve verdicts

| # | Verdict | Where |
|---|---------|-------|
| M1 | **landed** | `src/enrich.rs:62-72, 85-89, 211-218, 289-296, 347` |
| M2 | **partially landed** | `app/tests/scheduler.rs:110-131` (done) / `app/tests/scheduler.rs:200-204` (residue) |
| M3 | **landed** | `CLAUDE.md:326-334` |
| m1 | **landed** | `scripts/dual-run.ps1:153-156`, `docs/superpowers/plans/2026-09-02-rust-cutover-plan.md:110-115, 145` |
| m2 | **landed** | `app/tests/inference.rs:154-172` |
| m3 | **landed** | `app/tests/inference.rs:259-276` |
| m4 | **landed** | `src/judge.rs:762, 928`, `src/enrich.rs:615`, `src/judgelog.rs:234`, `app/tests/scheduler.rs:61, 70` |
| m5 | **landed** | `src/enrich.rs:169-171` + two tests |
| m6 | **landed** | `scripts/starvation.ps1:79-84` |
| m7 | **landed** | `tests/starvation.rs:30-33` |
| m8 | **landed** | `app/static/index.html:126` |
| m9 | **landed** | `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md:452` |

---

## M1 â€” the wall-clock budget

### Placement

`src/enrich.rs:211-218`:

```rust
for item in &batch {
    if batch_started.elapsed() >= opts.budget {
        break;
    }
    processed += 1;
    ...
```

The check is the **first statement of the loop body**, before `processed += 1` and before
`judge::judge_task`. Correct: the bound is "budget, plus at most one call already in flight".

### The arithmetic, against the constants as they now stand

| constant | value | file:line |
|---|---|---|
| `enrich::BATCH_BUDGET` | `15 * 60` s = **900 s** | `src/enrich.rs:72` |
| `runtime::CALL_TIMEOUT` | `120` s = **2 min** | `src/runtime.rs:61` |
| `scheduler::CHILD_TIMEOUT` | `20 * 60` s = **1200 s** | `app/src/scheduler.rs:27` |
| `enrich::DEFAULT_LIMIT` | **50** (deliberately untouched) | `src/enrich.rs:59` |

`judge::judge_task` (`src/judge.rs:367-407`) makes **exactly one** `model.judge(...)` call â€” one
`if let` seed path, one model call, no retry â€” so one item costs at most one `CALL_TIMEOUT`.
`runtime::wait_and_read` enforces that deadline as a hard kill-and-reap (verified by
`a_call_that_never_answers_is_bounded`, `src/runtime.rs:461-477`), so `CALL_TIMEOUT` is a real
ceiling, not an aspiration.

Worst case: the last item admitted starts at `t < 900 s`; its call returns or is killed by
`t < 900 + 120 = 1020 s` = **17 min**, against `CHILD_TIMEOUT` = 20 min. **Headroom 180 s.**
`slot_argv` runs `judge` as its own child (`app/src/scheduler.rs:489-497`, `run_child(&e, &args,
&log, CHILD_TIMEOUT)` inside the per-step loop), so the 20 minutes is the judge step's own budget,
not the whole slot's. The pre-fix arithmetic â€” `50 x 120 s = 100 min` â€” is now unreachable.

One nuance, not a defect: `batch_started` is taken **after** `pending()` and `Heuristics::load`
(`src/enrich.rs:203-209`), so the vault scan is outside the 15 minutes. It would have to exceed the
180 s of headroom to matter; it reads task frontmatter only.

### Always exit 0

Two independent guarantees, both verified:

- `enrich_with` has exactly one return in the non-empty path â€” `(0, lines)` at
  `src/enrich.rs:298` â€” and one early `(0, lines)` for an empty queue. There is no non-zero return
  anywhere in the function; a write failure becomes a line (`src/enrich.rs:283`), not a code.
- `src/main.rs:331-338`: `Command::Judge` **discards the code entirely** (`let _ = enrich::run(...)`)
  and returns `ExitCode::SUCCESS`, with the comment saying so out loud.

So a budget trip cannot set `engine_ok = false` and cannot paint the tray amber. The invariant holds.

### Remainder counting

`left` is computed twice and composes exactly:

- `src/enrich.rs:196`: `let left = all.len().saturating_sub(opts.limit);` â€” the `limit` cut.
- `src/enrich.rs:291`: `let left = left + (batch.len() - processed);` â€” the budget cut.

With `batch.len() == min(all.len(), limit)` and `processed <= batch.len()`, the total is exactly
`all.len() - processed`. **No off-by-one in either direction, and no underflow** (`processed`
increments once per iteration, at most `batch.len()` times). The item count in the summary moved
from `batch.len()` to `processed` (`src/enrich.rs:293`), which is the number actually started â€”
when the budget never trips they are equal, so no existing assertion changes meaning.

### Interaction with `limit`

Whichever binds first wins and both report through the same line. Verified by both tests:

- `the_batch_is_bounded_and_says_how_many_are_left` (`src/enrich.rs:664-680`): 5 items, `limit = 2`,
  `BATCH_BUDGET` â€” `"2 item"`, `"3 left"`. Unchanged by the fix, still passing.
- `a_run_stops_at_its_wall_clock_budget_and_reports_the_remainder`: 3 items, `limit = 50`,
  budget 20 ms â€” `"judge: 1 item"`, `"2 left"`.

No consumer parses the summary line (`git grep 'item(s)'` finds only `enrich.rs` itself and two
doc comments), so the count change is internal.

### Is the new test real?

Yes. `src/enrich.rs:690-718`. It drives a `Sleepy` model that **actually consumes 300 ms** against a
20 ms budget with three pending items, and asserts all four required things:

- `assert_eq!(code, 0, "a budget cutoff is never a failed slot")` â€” exit 0;
- `summary.starts_with("judge: 1 item")` â€” the run stopped at the budget after one item;
- `summary.contains("2 left")` â€” the remainder is reported through the existing line;
- the first item is always admitted (`elapsed` at the first check is a few instructions, far under
  20 ms), so this is not the degenerate zero-budget test the ruling warned against.

It exercises the composition: the stop is caused by time the model spent, not by a constant.

---

## M2 â€” the judge-skip slot test

**Partially landed.**

What landed (`app/tests/scheduler.rs:110-131`):

- A fresh temp directory (`std::env::temp_dir().join(format!("qo-console-sched-localappdata-{}",
  std::process::id()))`), created before the lock, removed after.
- `LOCALAPPDATA` set to it, and the previous value captured into `prev_local_appdata` and
  **restored** (`Some(p) => set_var`, `None => remove_var`) before `drop(_guard)`.
- Both `set_var`s are inside the `ENGINE_ENV_LOCK` critical section, and the comment was updated to
  say both variables are process-global.
- `state::app_data_root()` (`app/src/state.rs:198-206`) reads `LOCALAPPDATA` fresh on every call â€”
  no `OnceLock`, no cache â€” so the seam genuinely takes effect.
- The test is now hermetic: a temp root can never contain a runtime, so `judge_state` always returns
  `NoRuntime` and `.expect("a judge step, named")` cannot start failing the day Quinn installs a
  model. **The failure mode M2 was written to prevent is closed.**

What did not land â€” **the ruling's second, separately-stated reason**:

- `a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list`
  (`app/tests/scheduler.rs:190-204`) calls `run_slot_inner` with **no `LOCALAPPDATA` seam**. Its
  vault is written with `scheduler: app` and `device: <device_name()>`, so it passes both early
  returns (`app/src/scheduler.rs:440-445`) and reaches the unconditional `let judge =
  judge_state(cs);` at `app/src/scheduler.rs:477` -> `commands::inference_root()` ->
  `state::app_data_root()` -> `profiles::migrate_flat_layout(<real %LOCALAPPDATA%>, None)`.
- So **the app test suite still performs a `migrate_flat_layout` attempt against the user's live app
  data on every `cargo test`**, plus a `create_dir_all` of `%LOCALAPPDATA%\knowlu`. The implementer's
  report claims otherwise ("no `cargo test` run performs a `migrate_flat_layout` attempt against live
  app data anymore"); the *code comment* it wrote is correctly scoped ("a side effect **this test**
  had"), so this is an overclaim in the report, not a false comment in the tree.
- Live state today: `%LOCALAPPDATA%\quinn-ops\` holds only `dual\ logs\ rehearsal\ scratch\ shots\` â€”
  exactly what R-P4a-28 leaves alone â€” so the fold is a no-op scan and nothing is currently at risk.
  It is still the side effect the ruling said was "reason enough on its own".
- The third `run_slot_inner` test (`run_slot_inner_refuses_on_scheduler_script_without_spawning_anything`,
  line 293) returns at the `mode() != App` guard and never reaches `judge_state`. Not affected.

**Panic-unwind:** not covered. The restore is straight-line code, not a `Drop` guard. A panic inside
`run_slot_inner` would leave `LOCALAPPDATA` (and `KNOWLU_ENGINE_EXE`) pointing at the temp directory
for the rest of the process â€” and because the mutex is taken with
`.unwrap_or_else(|e| e.into_inner())`, later tests would run on the poisoned-but-recovered lock with
the wrong environment. The asserts all sit *after* the restore, so an assertion failure is clean;
only a panic inside the call under test leaks. This mirrors the pre-existing `KNOWLU_ENGINE_EXE`
pattern in the same file, so it is not a regression the wave introduced.

---

## M3 â€” CLAUDE.md's numbers

**Landed, and correct against the suite rather than against the instruction.**

`CLAUDE.md:326-334` now reads 688 Python / 882 `cargo test` (3 ignored, 0 warnings) / 127
`cd app; cargo test` (1 ignored), dated 2026-09-07. The fix-wave file told the implementer to write
880/126; those were the review's **pre-fix** counts, and this wave adds three tests (M1's budget
test, m5's tier gate test, m2's `check_runtime_supported` test). **882/127 is what I measured**
(see Gate below), so writing the measured numbers over the instructed ones was the right call.

The "two ignored tests" sentence is rewritten and now names all three, each with its home, and says
which is run by hand:

> The three ignored tests, each named in its own `#[ignore]` attribute, are cross-cutting traps 4 and
> 5 (`src/events.rs`) plus `src/runtime.rs`'s
> `real_runtime_loads_a_model_and_answers_under_the_grammar` (needs a real `llama-cli.exe` and a
> `.gguf`: set `KNOWLU_RUNTIME` and `KNOWLU_MODEL`, then run it by hand with
> `cargo test -- --ignored real_runtime`); none may be un-ignored by changing the assertion.

Cross-checked against the runner's own `... ignored` lines: traps 4 and 5 in `events::tests`, and
`runtime::tests::real_runtime_loads_a_model_and_answers_under_the_grammar`. All three named
correctly; the rule survives as a rule.

---

## The nine Minors, one line each

- **m1 â€” landed.** `scripts/dual-run.ps1:153-156` now says the path is ported (plan 3a Task 6), that
  the guard is inert defense-in-depth because `$steps` is fixed to `coursework`/`rank`, and that the
  proof is `diff-engines-notes.ps1`'s `w13`-`w15`; the cutover plan's invariant (lines 110-115) and
  C10 row (line 145) are both marked SUPERSEDED, and its claim that `WriteError::ProposeNotPorted`
  no longer exists checks out (`git grep` finds it only in historical plans and HANDOFF prose).
- **m2 â€” landed.** Kept `pub`, with a reason (the plan's D6a ledger row names it as the carrier of
  R-P3a-2, so hiding it would silently rewrite the ledger), and
  `check_runtime_supported_answers_exactly_like_the_production_path` asserts the two paths return
  **byte-identical** error strings for the same file and that checking installs nothing.
- **m3 â€” landed.** `no_command_reaches_the_unchecked_extraction_half` now walks `read_dir("src")`,
  skipping only `inference.rs`, with a `checked >= 3` guard against a silently empty scan; it covers
  all eleven other `app/src/*.rs` including `state.rs`, which the old three-file list missed.
- **m4 â€” landed.** Five occurrences corrected. `git grep llama-server eea4432` leaves only
  `src/runtime.rs:54` (the deliberate `llama-cli.exe`, **not** `llama-server.exe` contrast) and two
  historical documents (the plan and the spike report) â€” correct to leave those as record.
- **m5 â€” landed.** `src/enrich.rs:169-171` gates the push on `v.tier == 3`; the existing
  `a_whole_number_effort_is_written_as_a_float` gained `tier: 3` (which is what it always meant), and
  `effort_confidence_low_is_written_only_when_the_model_answered` proves a tier-1 verdict carries the
  hours forward without rewriting the confidence.
- **m6 â€” landed.** `scripts/starvation.ps1:84` is `$today = [datetime]::UtcNow.Date`, which now
  matches the bucket keys, taken from `([string]$rec.ts).Substring(0, 10)` (line 67) over a `ts` that
  `journal::now_ts` (`src/journal.rs:58-63`) always writes in UTC; the comment states the direction
  of the old error and why that direction mattered, and the file stays ASCII (119 CR / 119 lines).
- **m7 â€” landed.** `tests/starvation.rs:30-33` joins `env!("CARGO_MANIFEST_DIR")` with
  `scripts/starvation.ps1`, so the invocation no longer depends on the process's working directory.
- **m8 â€” landed.** `app/static/index.html:126` now carries `hidden` on `#set-judge-remove`;
  `console.js:1018` and `:1022` already set `.hidden` from `inference_status`, so the button starts
  hidden and is revealed only once a model is known present.
- **m9 â€” landed.** Row 12 of the decisions table (spec line 452) ends `; the app runs with no model`,
  restoring the D7 clause verbatim as specified.

---

## The two reported deviations

### 1. The fix-wave file misattributed m1 â€” **the implementer was right, and my own text was wrong**

Verified independently:

- `final-review.md:102-111` pins the stale text to **`scripts/dual-run.ps1:153-154`** and says in so
  many words that "`diff-engines-notes.ps1`'s header was updated while this one was not".
- `git show dfd86f7:scripts/diff-engines-notes.ps1` lines 32-35 already read: *"Knowlu plan 3a Task 6
  added the `--propose` trio (w13-w15), which **was excluded while** `write::propose_amendment`
  **was unported**"* â€” past tense, accurate, and left correctly untouched (it appears in neither the
  diff nor `git diff --stat`).
- `dual-run.ps1` and both cutover-plan rows are now accurate.

Following the review over the fix-wave summary was the correct call.

### 2. The unlisted `tests/starvation.rs` bug â€” **diagnosis confirmed, fix is genuinely TZ-independent**

- The bug was real and reachable: the fixtures label their timestamps `Z` and the file names are
  UTC day files, but `today` came from `jiff::Zoned::now().date()` â€” the **machine's local** date.
  Once m6 moved the script to `[datetime]::UtcNow.Date`, the two disagreed for every hour where the
  local date differs from the UTC date. In CDT (UTC-5) that is 19:00 onward â€” i.e. it was a live,
  reliably reproducing failure at the time of the fix, not a theoretical one.
- The fix is `jiff::Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC).date()`
  (`tests/starvation.rs:14-20`), used in both tests that construct dates. That is an absolute UTC
  date derived from an absolute instant â€” **it returns the same value in CDT and in UTC+13**, so
  this is a time-zone-independent fix, not a fix that happens to pass tonight. Both sides of the
  comparison (test fixture and script) are now anchored to the same clock.
- Residual, trivial: the test computes its UTC date and then spawns PowerShell, which recomputes
  `UtcNow.Date`. A run straddling UTC midnight in that window would disagree. Pre-existing in shape,
  sub-second window, not worth a change.

---

## Three checks against the rest of the tree

### Regressions

- **No test that can no longer fail.** The M1 test's stop is caused by time a fake model actually
  spends, not by a constant; the m5 test asserts an absence that the pre-fix code would have
  violated; the m2 test compares two independently produced strings; the m3 scan carries a
  `checked >= 3` floor so an empty walk fails rather than passes. The M2 test became *hermetic*
  rather than vacuous â€” it still asserts the step name, its `0` code and `engine_ok`.
- **No newly unused `pub` item.** `check_runtime_supported` gained a caller (its test);
  `BATCH_BUDGET` and `Options::budget` are used by `run()`, `opts()` and two tests. Root
  `cargo test` at **0 compiler warnings** independently confirms no new `dead_code`.
- **No doc claim the fix wave made false.** CLAUDE.md's counts are now measured-accurate; the
  `app/tests/scheduler.rs` M2 comment is correctly scoped to that test; the cutover plan's
  SUPERSEDED rows and `dual-run.ps1`'s comment are all true as written.
  (`docs/HANDOFF.md:212` still says "814 passed, 2 ignored" â€” already stale at `dfd86f7`, untouched
  by this wave, and framed as a historical "Green at close" note. Out of scope, flagged only so it
  is not mistaken for something this wave broke.)

### Gate â€” re-measured

| gate | implementer reported | **I measured** |
|---|---|---|
| `cargo test` (repo root) | 882 / 3 ignored / 0 warnings | **882 passed, 0 failed, 3 ignored, 0 compiler warnings, exit 0** |
| `cd app; cargo test` | 127 / 1 ignored | **127 passed, 0 failed, 1 ignored, exit 0** |
| `.venv\Scripts\python.exe -m pytest` | 688 | **688 passed in 19.22s, exit 0** (run, though not required) |

App warnings: three lines, all the one pre-existing `.rsrc merge failure: multiple non-default
manifests` linker warning and its two "generated 1 warning" roll-ups. No new warnings.
Dual-run scripts not re-run, per the brief.

### Line endings

`git diff --stat dfd86f7..eea4432`: **12 files, 215 insertions, 34 deletions**. Every file's change
count is a small fraction of its length (CLAUDE.md 13 of 435, cutover plan 10 of 611, spec 2 of 500,
`index.html` 2 of 131) â€” **no whole-file flip anywhere**. The index is LF throughout, as always.

Working-tree byte counts (CR / LF), read as bytes:

- CRLF as expected: `CLAUDE.md` 435/435, spec 500/500, cutover plan 611/611, `dual-run.ps1` 193/193,
  `starvation.ps1` 119/119, `index.html` 131/131 â€” every count matches the implementer's own
  before/after figures.
- Bare LF as expected: `src/judge.rs`, `src/judgelog.rs`, `tests/starvation.rs`.
- CRLF where the brief expected LF: `src/enrich.rs` 831/831, `app/tests/scheduler.rs` 444/444,
  `app/tests/inference.rs` 378/378. Harmless and consistent with CLAUDE.md's own rule that
  working-tree endings are **per file, not per directory** (a fresh worktree checkout is CRLF; only
  a file a tool has rewritten whole goes bare LF). The index normalises, the diff is targeted, and
  `git status` is clean.

`git status --porcelain --untracked-files=normal` at review time: **empty**.

---

## New findings

**Major:** none.

**Minor 1 â€” the M2 side effect survives in the sibling test.**
`a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list` (`app/tests/scheduler.rs:190-204`)
reaches `judge_state` -> `state::app_data_root()` -> `profiles::migrate_flat_layout` against the real
`%LOCALAPPDATA%` on every `cargo test`. Harmless today (the flat root holds only the five directories
R-P4a-28 leaves alone, so the fold is a no-op), but it is the second effect M2 called "reason enough
on its own", and it is one `set_var` pair away from being closed. *Smallest fix:* seam it exactly as
the sibling test now is â€” the `ENGINE_ENV_LOCK` guard is already held across the call.

**Minor 2 â€” the environment restore is not unwind-safe.**
Neither `LOCALAPPDATA` nor `KNOWLU_ENGINE_EXE` is restored through a `Drop` guard, so a panic inside
`run_slot_inner` leaks both to the rest of the process, and the poison-recovering
`.unwrap_or_else(|e| e.into_inner())` means later tests would run with them. *Smallest fix:* a
three-line RAII struct in the test file, applied to both variables in all three sites.

**Minor 3 â€” m3's replacement guard reintroduces the fragility m7 removed.**
`no_command_reaches_the_unchecked_extraction_half` scans `std::fs::read_dir("src")` â€” a relative
path, in the same wave that ruled a relative path in `tests/starvation.rs` a defect. It matches the
existing `static_assets.rs` precedent and fails loudly (`unwrap`) rather than silently, so it is
inconsistency rather than risk. *Smallest fix:* `Path::new(env!("CARGO_MANIFEST_DIR")).join("src")`.

**Trivial, no action needed:** the m4 sweep left the test helper `fn server_exe()`
(`app/tests/inference.rs:94`) named for the server even though it returns `RUNTIME_EXE`
(`llama-cli.exe`); and `final-fix-report.md` says it and `COMMIT_MSG_FINAL` were "added by this same
commit" when `.gitignore:7` excludes `.superpowers/` entirely, so neither is in `eea4432`.
