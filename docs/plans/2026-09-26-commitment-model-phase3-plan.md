# The commitment model, phase 3 — plan (the UA registrar)

**Status: PLAN, written 2026-09-26 while Quinn was away. Not executed.** **Base:** branch
`p3-registrar` at `eb10c1b` (cut from `p2-commitments`: phase 1, phase 2 and C1c). No task here
depends on the order in which `p2-commitments` reaches `main`.
**Written to survive a context compaction:** every task names its files with line ranges from the
code at `eb10c1b`, its interfaces, its tests and its commit, so a fresh session can execute from
this document alone.

> **For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.**
> R0 is not a subagent task: it is the controller's, with Quinn at the machine.

**Goal.** A UA student presses **Get my class times from myBama**, signs in on the school's own
page in a window Knowlu opens, and presses **I'm signed in**. Knowlu reads that window's session for
the registrar host, fetches the term's schedule once, throws the session away, and the engine turns
the rows into series. Rows that match a vault course become confirmed class and lab notes (R24);
the rest are proposals marked "from myBama". A later fetch that drops a course files an end card.

**Architecture.** The engine does every step that decides anything. `knowlu-engine commitments
--registrar <file> --school ua` parses the fetched JSON (`registrar::parse_banner`, the one place
that knows Banner's shape), merges the rows into `state/calendar-series.json` under D5's rule, and
writes the R24 notes through phase 2's `commitments::confirm`. It makes no network call. The app
adds `app/src/registrar.rs`: three console commands that reuse `lms_link.rs`'s incognito,
capability-less sign-in window, read its cookies for the registrar host only, make the recorded
GETs with `ureq`, hand the bytes to the engine through a temp file, and close and wipe the window.
`console.js` gains the button in two places and the "from myBama" marker.

**Tech stack.** Rust 1.98 (`stable-x86_64-pc-windows-gnu`), `jiff`, `serde_json`, `serde_yaml_ng`,
`ureq` 3, Tauri 2, plain JavaScript (no bundler), Playwright (`scripts/wizard-check.py`).

**Spec.** `docs/specs/2026-09-26-commitment-model-phase3-design.md` (binding; "the spec", §1–§6,
decisions D1–D8). Where it is silent, `docs/specs/2026-09-23-commitment-model-design.md` governs
("the parent": §2, §3.1–§3.5, §5.2–§5.4, §10 Phase 3), then
`docs/specs/2026-09-24-commitment-model-phase2-design.md` ("phase 2"). Where this plan and the spec
disagree, the spec wins and this plan is wrong. Spec problems 1–3 and the resolution of problem 4
are now the spec's own "Amendments (2026-09-26, plan review)" section (controller rulings at the
pre-execution review, `docs/plans/2026-09-26-commitment-model-phase3-plan-review.md`).

**Preconditions before R1 (controller, not a subagent):**
- **The p2 judge-once fix is merged into `p3-registrar`.** On `p2-commitments`, commitment change
  detection counts "the student set this field" only from a later `set` record by `quinn`; the
  `create` record no longer counts (a helper such as `journal::human_edited`, or a filtered
  `commitments::is_human_set`). R3's R24 notes are written as `quinn` via `dashboard` (spec §3) and
  rely on it: R3's `a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank` fails
  without it, and that failure is the pin. If the fix changes `detect_changes`' signature, R2's
  `a_dropped_registrar_course_ends_its_confirmed_note` follows it.
- **R0 is not needed before R1–R3** (see "Order and file ownership"): R1 writes the fixture in the
  documented Banner 9 shape (b) as a provisional contract; R0 confirms or replaces it before R4.

**Task numbering against the spec's §5.** Spec R0 = plan R0. Spec R1 = plan R1 (the parse) + R2
(D5, D7). Spec R2 = plan R3. Spec R3 = plan R4. Spec R4 = plan R5. Spec R5 = plan R6, whose live
proof is a separate closing checklist that does not block the code.

## Global Constraints (binding on every task)

1. **Line endings:** LF in every file this plan touches. `engine/tests/fixtures/**` is `-text`: the
   new fixture is written LF by the Write tool and never re-encoded; the frozen ones are not opened
   for writing.
2. **0 warnings:** `cargo build`/`cargo test` print no `warning:` line but the accepted `.rsrc merge
   failure: multiple non-default manifests`.
3. **TDD:** in every task the failing test comes first, is run and seen failing, then the code.
4. **Frozen references never change:** the eight Python references in `engine/tests/fixtures/` and
   `surface-today-{s1,s1-migrated,full}.json` are byte-identical at the end of every task. None of
   their vaults holds a `registrar:` calendar, so no path this plan adds is reached by them.
   `engine/tests/fixtures/registrar/banner-ua-registration.json` is **new, hand-written and not
   frozen**: R1 writes it in the documented shape (b) (provisional), and afterwards only R0's
   findings may change it.
5. **Journal first:** every note write goes through `write::create` / `write::write_literals`
   (here, only through phase 2's `commitments::confirm`); no note is parsed and re-dumped.
6. **The app computes nothing:** `app/src/commands.rs`, `app/src/week.rs` and the new
   `app/src/registrar.rs` marshal, call the engine and return envelopes. The term code, the
   signed-out test and every parse are engine functions.
7. **`rank` never calls a model or a network it did not call before:**
   `rank_cannot_reach_a_judgment_endpoint` (`engine/tests/cloud_contract.rs`) stays green, and the
   registrar is never a slot step (spec D8).
8. **`--registrar` makes no network call:** pinned by `registrar_makes_no_network_call`, the same
   loopback-listener shape as phase 2's `confirm_makes_no_network_call`.
9. **No stored password or cookie:** no file, log, envelope or error string holds a cookie, a
   password or the fetched bytes. The bytes live in one temp file for the length of one engine run.
10. **Who writes:** the console's human edits stay `quinn` via `dashboard`
    (`commands::console_ctx()`; the engine's `--actor quinn --via dashboard`). The R24 notes are
    written as `quinn` via `dashboard` too, as spec §3 says (Plan ruling R3-c, as amended at the
    plan review): the student pressed the button. Their change and end cards still come because of
    the p2 judge-once fix (see Preconditions).
11. **Tauri command counts:** the console window's `generate_handler!` list (the second one in
    `app/src/main.rs`, in `run_console`) goes from **47 → 50** (`registrar::open_registrar_window`,
    `registrar::capture_registrar`, `registrar::close_registrar_window`); the vault-less window's
    list (`run_shell`) stays **29**; **69** distinct. Counted by script (R6 Step 1), never by hand.
12. **Never `git stash`.** Set work aside with a WIP commit.
13. **One cargo at a time, with `-j 2`** (host memory is low). App tests that spawn the engine need
    the real exe: run `cargo build -p knowlu-engine -j 2` first, because `app/build.rs` drops a
    zero-byte placeholder over `target/debug/knowlu-engine.exe`.
14. **Commits:** write the message with the Write tool to the session scratchpad, then
    `git commit -F <file>`. Every message ends with the two lines
    `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>` and
    `Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF`.
15. **No personal data:** every title, room, CRN, course and key in a test or fixture is invented.
    R0 records shapes, never values.
16. **Small edits:** keep every Edit/Write at **60 lines or fewer**. A long block below is applied
    as several edits in order. Agents stalled on large single edits in phase 2.
17. **The wizard keeps nine panels:** the registrar is not in the wizard (spec D6), and
    `the_wizard_has_nine_panels_and_the_privacy_words_and_no_live_fetch` is not edited.

## Fidelity ledger

| Spec requirement | Task |
|---|---|
| Rulings: UA only; the school's own window, never a stored password, session thrown away; login-only onboarding | R4 (window reuse, no credential path), R6 (live proof) |
| D1 — `lms_link.rs`'s pattern: incognito window, throwaway directory, no capability; cookies for the registrar host only; one or two requests; close and wipe | R4 (R4-a, R4-d; `cookie_url`; the capability pin), R6 |
| D2 — the registrar entry in the curated table: start URL, host, prefix, endpoint; no entry, no button | R4 (`scaffold::Registrar`), R5 (the button is hidden when `registrar_label` is null) |
| D3 — the endpoint settled by a spike; shape (b) preferred; five questions; shapes only, invented fixture | R0 (checklist), R1 (built on shape (b), R1-a) |
| D4 — the app writes the raw JSON to a temp file, runs `commitments --registrar <file> --school ua`, deletes the file whatever the outcome; the engine parses, merges, writes and prints JSON | R3 (engine), R4 (`run_file`, R4-e) |
| D5 — `registrar:` calendars count as configured; replaced only by a fresh fetch; `until` ends them; a missing row goes to `ended` at once and files an end card | R2 (R2-a, R2-b, R2-c, R2-g), R3 (the end card, end to end) |
| D6 — the button under *Your classes* on `#week-setup` and at the top of *Schedule*; not in the wizard; "Refresh from myBama" once a term is held | R5 (R5-c), constraint 17 |
| D7 — registrar > google > ics by signature; no ask card for a course with a registrar class; `emit_asks` waits until the day after the term starts | R2 (R2-f, R2-h, the cli test) |
| D8 — only on a button press, never a slot step; the refresh label by term; nothing nags | R3 (`Overview.registrar`), R5; `scheduler.rs` is not touched |
| §2 flow: open, "Sign in to myBama…", I'm signed in, `{ok, error}` with "You're not signed in yet", engine run, `close_and_wipe`, repaint; no partial write; an empty parse is a failure | R4 (R4-d), R5, R1 (empty → `Err`), R3 (exit 2 writes nothing) |
| §3 parse: the kept-row rule; the Series field table; `" Lab"`; `where` ≤ 80; `meets`; `first`/`until`; `event_type: registrar`; weekly rule; no instances | R1 |
| §3 rows dropped: no time (counted), across midnight (warned); kind from the schedule type, no classifier; course through the code table | R1 (R1-d), R2 (R2-e) |
| §3 merge through `refresh_series` under D5 | R3 (one calendar key per term, R2-a) |
| §3 write (R24): matched rows confirmed at `hard`, idempotent; others are proposals carded from day 2; change and end cards by the existing machinery | R3 (R3-b, R3-c), R2 (R2-g) |
| §3 output keys, exit codes 0/2, nothing written on exit 2, no network | R3 |
| §4 the privacy line; the temp file deleted; the series file local; only confirmed notes sync; Google's consent screen unchanged | R6 (R6-a), R4 (R4-e); nothing touches `sync` |
| §5 R0–R5 | R0–R6 (see "Task numbering") |
| §6 items 1–3 for Quinn | R0 (item 1), R6-a (item 2), R3-c as amended: R24 as written, `quinn` via `dashboard` (item 3) |
| CLAUDE.md's command list and counts, `app/README.md`, anatomy | R6 |

## Order and file ownership

| Task | Unit | Files | Depends on |
|---|---|---|---|
| R0 | the spike (controller + Quinn) | `docs/specs/2026-09-26-commitment-model-phase3-design.md` (D3 amendment), `engine/tests/fixtures/registrar/banner-ua-registration.json` (confirmed, or replaced in R0's shape) | Quinn; runs any time before R4 |
| R1 | `registrar::parse_banner`, the school table, `term_for`, `looks_signed_out`; the provisional fixture | `engine/tests/fixtures/registrar/banner-ua-registration.json` (new), `engine/src/registrar.rs` (new), `engine/src/lib.rs` | the p2 fix merged (Preconditions) |
| R2 | D5 and D7: the series file, the classifier, precedence, the change watch, the ask gate | `engine/src/commitments.rs`, `engine/src/cli.rs`, `engine/src/registrar.rs` (tests) | R1 |
| R3 | `registrar::run`, `commitments --registrar`, exit codes, the no-network pin, `Overview.registrar` | `engine/src/registrar.rs`, `engine/src/commitments.rs`, `engine/src/cli.rs`, `engine/src/main.rs`, `engine/tests/commitments_registrar.rs` (new) | R2 |
| R4 | the app: the curated entry and three commands | `app/src/scaffold.rs`, `app/src/lms_link.rs`, `app/src/registrar.rs` (new), `app/src/lib.rs`, `app/src/week.rs`, `app/src/main.rs`, `app/tests/registrar.rs` (new) | R3; R0 (the call list) |
| R5 | the console: two buttons, the marker, the refresh label, the behavioural check | `app/static/{index.html,console.js,console.css}`, `app/tests/static_assets.rs`, `scripts/wizard-check.py` | R4 |
| R6 | the privacy line, docs, the recount, full verification; then the live-proof checklist | `site/privacy.html`, `CLAUDE.md`, `app/README.md`, `docs/surface/anatomy.md` | R5; the proof needs Quinn |

Strictly sequential: the engine's `registrar.rs` is edited by R1–R3, `commitments.rs` by R2 and
R3, `cli.rs` by R2 and R3. No two tasks run in parallel. Each task ends with a green run of its own
filter and one commit.

**R0 and R1–R3 (plan review).** R1–R3 proceed before R0 against Banner 9's documented shape (b):
the engine never touches the network, and every Banner key it reads is in `parse_banner`, the
fixture, and four test bodies that build or edit rows by key (R1's
`where_is_cut_to_80_and_a_row_without_a_building_has_none`,
`an_empty_parse_is_a_failure_never_an_empty_semester`,
`another_term_or_a_repeated_crn_is_skipped_with_a_warning`; R3's
`a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank`). R4 needs R0 for
`UA_ROWS_PATH`, the term-selection call, and R4-h's headers. What R0 can still change upstream:
- **The same facts in another shape** (renamed keys, another envelope, times as `"09:30"`): R0
  replaces the fixture and edits `parse_banner` and those four test bodies in one follow-up
  commit; every expected value stays.
- **The spring suffix** (R0 answer 3): `SCHOOLS`' `(1, "10")` and the `term_for` test change.
- **Re-plan trigger — stop for Quinn:** the rows call carries no per-meeting `startDate`/`endDate`
  (for example shape (a), one week of occurrences). Then `first`/`until` are unknown, and R2-b
  (the term in play), R2-h (the ask gate) and R3-d (the refresh flag) have nothing to read. That
  is not a `parse_banner`-only change.

## Plan rulings (collected; each is repeated in its task)

- **R0-a** R0 is controller work with Quinn; it records shapes only, through CDP on a dev build.
- **R1-a** all Banner shape knowledge lives in `registrar::parse_banner`, its fixture and four
  test bodies (listed under "Order and file ownership"). R0's findings change those, the curated
  row's call list (data) and the spring suffix, unless the re-plan trigger fires.
- **R1-b** the file is a top-level array of rows or Banner's `{"data": [rows]}` envelope.
- **R1-c** `where` is `buildingDescription` (else `building`) then `room`, one space, cut to 80.
- **R1-d** a row's patterns become one `Meet` each, identical ones merged; `first` is the earliest
  `startDate`, `until` the latest `endDate`; one pattern across midnight drops the whole row.
- **R1-e** the term is the first kept row's; a row of another term, or a repeated CRN, is skipped
  with a warning.
- **R1-f** `term_for("ua", d)`: January–May is `<year>10`, June–December is `<year>40`.
- **R1-g** `looks_signed_out(body)` is "the body is not a JSON array or object".
- **R2-a** a registrar calendar key is `registrar:<school>:<term>`, one per term.
- **R2-b** a registrar calendar counts as configured while it holds a series whose `until` is open
  or less than 28 days past; after that it ages like a removed feed.
- **R2-c** a registrar row missing from a fresh fetch of its term goes to `ended` at once, with
  `last_instance` = the day before the fetch.
- **R2-d** `instances_map` leaves registrar series out, so their notes block time by `meets`.
- **R2-e** `classify` has a registrar arm: kind `lab` when the title ends `" Lab"`, else `class`;
  the course by the code table; no eligibility test.
- **R2-f** `precedence` tiers: `registrar:` 0, `google:` 1, anything else 2; then the calendar key.
- **R2-g** `detect_changes` watches `registrar:` keys, and `rank` passes every registrar calendar in
  the file as fresh to it.
- **R2-h** the ask gate is `commitments::asks_wait_for_registrar`, called in
  `cli::commitment_passes`, not inside `emit_asks`.
- **R2-i** a registrar note's body reads "Found in your school's class schedule."
- **R3-a** `--registrar` conflicts with `--confirm` and requires `--school`.
- **R3-b** `mine` is the matched rows that are current proposals; `confirmed` counts the notes this
  run created; `proposed` counts kept rows with no course.
- **R3-c** the R24 notes are written under the context the command is given: `quinn` via
  `dashboard` (spec §3; amended at the plan review, see the Preconditions).
- **R3-d** `Overview` gains `registrar` and `registrar_proposals`.
- **R4-a** the registrar reuses `lms_link`'s sign-in window: its label, its session directory,
  `LmsSession`, the wipe and the sweep.
- **R4-b** the curated row's `Registrar` holds `school`, `label`, `host`, `prefix`, `start` and
  `calls`; the call list is R0's, verbatim.
- **R4-c** the school comes from `config/campus.yaml`'s `unitid`, read as text or as a number.
- **R4-d** the window stays open on "not signed in yet" and on a failed request; once bytes reach
  the engine, `close_and_wipe` runs whatever the engine says. The envelope carries `closed`.
- **R4-e** the temp file is `<profile>\tmp\registrar-<pid>-<stamp>.json`, written and run under
  `vault_io`, and deleted after the child exits.
- **R4-f** `run_console` manages `LmsSession`, wipes on the sign-in window's `Destroyed`, hides
  only `main` on close, and wipes at exit.
- **R4-g** `your_week`'s envelope gains `registrar_label`.
- **R4-h** each request sends `cookie`, `accept: application/json` and
  `x-requested-with: XMLHttpRequest`; R0 amends this if Banner needs more.
- **R5-a** "from myBama" is shown on a proposal whose `source_uid` starts `registrar:`.
- **R5-b** after a fetch, the confirm screen re-reads `commitment_proposals` and keeps the answers
  already given; the confirmed count is said on the button's line.
- **R5-c** *Schedule*'s label: "Get my class times from myBama" with no term held, "Refresh from
  myBama" once one is; the primary style only when `refresh` is true.
- **R6-a** the privacy version and Effective date do not move in this plan: Quinn's, with P1.

---

## R0 — the spike, with Quinn at the machine (spec D3)

**Who:** the controller, with Quinn signing in. **Not a subagent task** (memory: autonomous live
proof; the controller runs every live step, and no subagent holds a session).
**Files.** `docs/specs/2026-09-26-commitment-model-phase3-design.md` (a "D3 amendment (R0,
<date>)" subsection under D3); `engine/tests/fixtures/registrar/banner-ua-registration.json`
(R1's provisional file: confirmed as is, or replaced in R0's shape).
**Output for later tasks:** the call list R4 copies into `scaffold.rs` (method, path under the
prefix, form keys, extra headers), and the row shape `parse_banner` reads. **When R0 runs after
R1–R3** and the shape differs, the controller dispatches one follow-up task under "Order and file
ownership"'s rules before R4; if the re-plan trigger fires, it stops for Quinn.

**Plan ruling R0-a:** R0 records **shapes only**: key names, JSON types and nesting, the request
methods and paths, the query and form key names, the status codes and which headers were needed.
Never a value: no CRN, course, room, name, cookie or token is written to disk, printed to the
transcript or put in a commit. The term code is the one exception, because it is Banner's public
term identifier, not the student's data (C1c §0 already names `202640`). *Why:* spec D3 and
constraint 15; a shape is all the parser needs.

- [ ] **1. Build a dev copy.** One cargo at a time:
  `cargo build -p knowlu-engine -j 2`, then `cargo build -p knowlu -j 2`. Copy
  `target/debug/knowlu.exe` and `target/debug/knowlu-engine.exe` into
  `<scratchpad>/proof/r0/` (memory: dev build copy), so a later build cannot replace the exe under
  a running window.
- [ ] **2. Start it with remote debugging, on a scratch profile.** In PowerShell:
  `$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9223"`, then start the copy
  with `--pick`. Create a scratch profile through the wizard's first panels only if the picker is
  empty (memory: live-proof profile cleanup). The CDP driver lives in `<scratchpad>/proof/`
  (memory: CDP driver). DOM and CDP only: no synthetic OS keyboard or mouse input, and screenshots
  by window handle only (CLAUDE.md desktop safety).
- [ ] **3. Open the sign-in window at the registrar.** The console has no registrar command yet, so
  the vault-less window's existing one is used: through CDP on the `main` target,
  `window.__TAURI__.core.invoke('open_lms_window', { unitid: '100751' })`. That opens `lms_link`'s
  incognito, capability-less window. Then, through CDP on that window's target, `Page.navigate` to
  `https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/registration`. Nothing is typed by the
  controller.
- [ ] **4. Quinn signs in.** Quinn types their own myBama login in the school's page and approves
  Okta Verify on their phone. The controller does not watch the keystrokes: `Network` recording
  starts only after Quinn says "signed in".
- [ ] **5. Record the call sequence.** Enable CDP `Network` on the sign-in target. Quinn opens the
  registration page's schedule for Fall 2026 (the page Banner calls *View Registration
  Information*, or *Schedule and Options* in *Register for Classes*, whichever shows meeting times).
  For each XHR to `bannerssb.ua.edu`, the driver keeps the method, the path, the query key names,
  the form key names, the request header names and the status. It reduces each JSON response body
  to its **shape** in memory (every string becomes `"str"`, number `0`, boolean `true`, `null`
  stays `null`, an array keeps one element's shape and its length bucket `0`, `1`, `2+`) and
  prints only that. The raw body is never written anywhere.
- [ ] **6. Answer the five questions of D3,** each in one sentence, in the amendment:
  1. Which calls a signed-in student's session may make (the list from step 5, the one whose body
     carries meeting patterns marked **rows call**).
  2. Whether a term must be selected first (`term/search?mode=registration` POST or another), and
     its form keys.
  3. The term code for Fall 2026, and whether spring is `<year>10` (Plan ruling R1-f is checked
     against it: if Banner's codes differ, R1-f's two suffixes change in the same amendment).
  4. What an online or TBA section looks like in the rows call (null `beginTime`? all day booleans
     false? no `meetingsFaculty`?).
  5. Whether Banner answers the rows call made **outside** the browser with only the cookies. Test
     it once: the driver reads the cookies for `https://bannerssb.ua.edu/` through CDP
     `Network.getCookies` into memory and makes the calls from step 5 with Python's
     `urllib.request`, sending only `cookie`, `accept: application/json` and
     `x-requested-with: XMLHttpRequest` (Plan ruling R4-h). Record the status and the body's shape,
     then drop the cookies. If Banner refuses (say, it wants `X-Synchronizer-Token`), record which
     header and where the page carries it. That is a change to R4's call list, and the controller
     stops for Quinn before R4. Record too whether any call answers with a **redirect**: `ureq`
     drops the `cookie` header on every redirect, same host included (`ureq-proto`'s
     `redirect.rs`), so a redirecting call reads as "not signed in" and R4 must call its target.
- [ ] **7. Write the D3 amendment.** Under D3: the call list as R4's `scaffold::Call` rows
  (`method`, `path` relative to `/StudentRegistrationSsb/ssb/` with `{term}` where the term goes,
  `form` with `{term}`), the rows call last; the rows call's shape as an indented key tree; the five
  answers. If the rows call is shape (a) (`getRegistrationEvents`), not (b), say so: then R1's
  `parse_banner` field list follows the recorded shape instead of the one in R1, and nothing else
  in this plan changes (Plan ruling R1-a).
- [ ] **8. Check the fixture against the recorded shape.** R1 already wrote
  `engine/tests/fixtures/registrar/banner-ua-registration.json` as the block below (shape (b),
  invented). If R0 confirms shape (b), it stays byte for byte. If the shape differs, rewrite it by
  hand with the Write tool (LF, **invented** courses only): the same six rows carry the same
  invented facts in R0's shape, and every expected value in R1–R3 is unchanged.
- [ ] **9. Clean up.** Close the sign-in window through CDP
  (`window.__TAURI__.core.invoke('close_lms_window', {})` on `main`), quit the copy, confirm that no
  `knowlu-lms-session-*` folder is left in `%TEMP%`, and remove the scratch profile, its vault, its
  credentials and its autostart entry (memory: live-proof profile cleanup). Unset
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`.
- [ ] **10. Commit** (docs, and the fixture only if step 8 rewrote it): `git add docs/specs/2026-09-26-commitment-model-phase3-design.md engine/tests/fixtures/registrar/banner-ua-registration.json`;
  message:

```
docs: phase 3 spike — the registrar's call sequence and row shape (D3 amendment)

Recorded with Quinn signed in once: the calls a student session may make, the
term selection, the term code, the online/TBA row, and whether Banner answers
outside the browser. Shapes only; the fixture is hand-written with invented
courses.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

**The fixture (shape (b), invented; R1 Step 1 writes it).** Six rows: CS 100 lecture (MWF 12:00–12:50, the P16 test
vault's course, and the same times as `cli.rs`'s `cs100_item`), CS 100 lab (Thu 14:00–15:50),
ENGL 101 (Tue/Thu 09:30–10:45, its pattern repeated for a second instructor as Banner does), HIST
105 online (no time), MUS 250 across midnight, ART 110 (no vault course anywhere in the tests).

```json
{
  "success": true,
  "totalCount": 6,
  "data": [
    {"term": "202640", "termDesc": "Fall 2026", "courseReferenceNumber": "40001", "subject": "CS", "courseNumber": "100", "sequenceNumber": "001", "courseTitle": "Invented Computing", "scheduleTypeDescription": "Lecture",
     "meetingsFaculty": [{"meetingTime": {"beginTime": "1200", "endTime": "1250", "monday": true, "tuesday": false, "wednesday": true, "thursday": false, "friday": true, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": "INV", "buildingDescription": "Invented Hall", "room": "101"}}]},
    {"term": "202640", "termDesc": "Fall 2026", "courseReferenceNumber": "40002", "subject": "CS", "courseNumber": "100", "sequenceNumber": "L01", "courseTitle": "Invented Computing Lab", "scheduleTypeDescription": "Laboratory",
     "meetingsFaculty": [{"meetingTime": {"beginTime": "1400", "endTime": "1550", "monday": false, "tuesday": false, "wednesday": false, "thursday": true, "friday": false, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": "INV", "buildingDescription": "Invented Hall", "room": "B12"}}]},
    {"term": "202640", "termDesc": "Fall 2026", "courseReferenceNumber": "40003", "subject": "ENGL", "courseNumber": "101", "sequenceNumber": "002", "courseTitle": "Invented Writing", "scheduleTypeDescription": "Lecture",
     "meetingsFaculty": [
       {"meetingTime": {"beginTime": "0930", "endTime": "1045", "monday": false, "tuesday": true, "wednesday": false, "thursday": true, "friday": false, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": "PRT", "buildingDescription": "Pretend Library", "room": "210"}},
       {"meetingTime": {"beginTime": "0930", "endTime": "1045", "monday": false, "tuesday": true, "wednesday": false, "thursday": true, "friday": false, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": "PRT", "buildingDescription": "Pretend Library", "room": "210"}}]},
    {"term": "202640", "termDesc": "Fall 2026", "courseReferenceNumber": "40004", "subject": "HIST", "courseNumber": "105", "sequenceNumber": "OL1", "courseTitle": "Invented History Online", "scheduleTypeDescription": "Online",
     "meetingsFaculty": [{"meetingTime": {"beginTime": null, "endTime": null, "monday": false, "tuesday": false, "wednesday": false, "thursday": false, "friday": false, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": null, "buildingDescription": null, "room": null}}]},
    {"term": "202640", "termDesc": "Fall 2026", "courseReferenceNumber": "40005", "subject": "MUS", "courseNumber": "250", "sequenceNumber": "001", "courseTitle": "Invented Night Ensemble", "scheduleTypeDescription": "Lecture",
     "meetingsFaculty": [{"meetingTime": {"beginTime": "2300", "endTime": "0030", "monday": false, "tuesday": false, "wednesday": false, "thursday": false, "friday": true, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": "INV", "buildingDescription": "Invented Hall", "room": "300"}}]},
    {"term": "202640", "termDesc": "Fall 2026", "courseReferenceNumber": "40006", "subject": "ART", "courseNumber": "110", "sequenceNumber": "001", "courseTitle": "Invented Drawing", "scheduleTypeDescription": "Studio",
     "meetingsFaculty": [{"meetingTime": {"beginTime": "1800", "endTime": "2050", "monday": true, "tuesday": false, "wednesday": false, "thursday": false, "friday": false, "saturday": false, "sunday": false, "startDate": "08/19/2026", "endDate": "12/04/2026", "building": "STU", "buildingDescription": "Make-Believe Studio", "room": "4"}}]}
  ]
}
```

Expected by R1 and R3: four kept rows (`40001`, `40002`, `40003`, `40006`), one dropped for no
time (`40004`), one dropped across midnight (`40005`); term `202640`; `first` 2026-08-19, `until`
2026-12-04 on every kept row.

---

## R1 — `registrar::parse_banner`, the school table, `term_for`, `looks_signed_out` (spec §3 parse)

**Files.**
- `engine/tests/fixtures/registrar/banner-ua-registration.json` (new): R0's fixture block, verbatim
  (shape (b), invented), written with the Write tool, LF. Provisional until R0 (constraint 4).
- `engine/src/registrar.rs` (new): the module and its `#[cfg(test)] mod tests`.
- `engine/src/lib.rs`: `pub mod registrar;` after `pub mod commitments;` (find the line with
  `grep -n "pub mod commitments" engine/src/lib.rs`).

**Precondition:** the p2 judge-once fix is merged (see Preconditions at the top).

**Interfaces.**
- Consumes `commitments::{Series, Rule, Meet}` (`commitments.rs:75–79`, `934–940`, `946–969`; every
  field `pub`) and `weekcal::DayKey` (`weekcal.rs:23`).
- Produces, all `pub`:
  - `struct School { key: &'static str, terms: &'static [(i8, &'static str)] }`, `SCHOOLS: [School; 1]`,
    `fn school(key: &str) -> Option<&'static School>`.
  - `const CALENDAR_PREFIX: &str = "registrar:"`, `const EVENT_TYPE: &str = "registrar"`,
    `fn calendar_key(school: &str, term: &str) -> String` (`registrar:<school>:<term>`).
  - `fn term_for(school: &str, today: Date) -> Option<String>`.
  - `fn looks_signed_out(body: &str) -> bool`.
  - `struct Parsed { term: String, series: Vec<Series>, no_time: usize, midnight: usize, warnings: Vec<String> }`
    and `fn parse_banner(json: &serde_json::Value, school: &str) -> Result<Parsed, String>`.

**Plan ruling R1-a: all knowledge of Banner's response shape lives in `parse_banner` and its
fixture.** No other function in the engine or the app names a Banner key; four test bodies do
(listed under "Order and file ownership"). R0's findings change those, the curated row's call list
(R4-b, which is data) and the spring suffix, unless the re-plan trigger fires. *Why:* the spec
builds on shape (b) before the spike has run; this keeps a different answer from R0 to one function.
**Plan ruling R1-b:** the file is either a top-level array of rows or Banner's search-results
envelope `{"data": [rows], ...}`; anything else is `Err`. *Why:* both are how Banner 9 returns row
lists; R0 says which one UA's call uses.
**Plan ruling R1-c:** `where` is `buildingDescription` (else `building`), a space, then `room`,
trimmed and cut to 80 characters; taken from the row's first pattern that has either. *Why:* the
spec's "`building room`" read with Banner's readable building name; §2.2's bound.
**Plan ruling R1-d:** each pattern with at least one day and both times is one `Meet`; identical
`(days, start, end)` patterns are merged (Banner repeats `meetingTime` per instructor); `first` is
the earliest `startDate`, `until` the latest `endDate` over the kept patterns. One pattern whose
`endTime` is not after its `beginTime` drops the whole row with a warning. *Why:* spec §3's "one
entry per meeting pattern" and "dropped with a warning", without a half-kept course.
**Plan ruling R1-e:** the term is the first kept row's `term`; a later row naming another term, or
a CRN already kept, is skipped with a warning. *Why:* one fetch is one term (D5's "the fetch is the
whole term's truth"), and R1's key `registrar:<school>:<term>-<crn>` must be unique.
**Plan ruling R1-f:** `term_for("ua", d)` is `<year>10` from January through May and `<year>40`
from June through December. Summer terms are never fetched before the pilot (the June–August pause,
cloud design R3), so a June fetch reads fall. *Why:* the capture needs a term before its first call
and the spec does not say where it comes from. C1c §0 names `202640` as Fall 2026; R0 question 3
checks spring. The rule is the engine's, so the app computes nothing (constraint 6). (Spec problem 6.)
**Plan ruling R1-g:** `looks_signed_out(body)` is true when the trimmed body is not a JSON array or
object. *Why:* a Banner call from a session that is not signed in is redirected to the school's
HTML sign-in page; the app must say "You're not signed in yet" and keep the window open (R4-d).

- [ ] **Step 1 — failing tests.** Write the fixture: R0's "The fixture (shape (b), invented)" JSON
  block, verbatim, to `engine/tests/fixtures/registrar/banner-ua-registration.json` with the Write
  tool (LF). Then create `engine/src/registrar.rs` holding only the module doc line
  `//! Phase 3 of the commitment model: the school registrar's schedule, parsed on the device.` and
  this test module; add `pub mod registrar;` to `lib.rs`.

```rust
#[cfg(test)]
mod tests {
    //! Phase 3 (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md` §3). The fixture is
    //! hand-written and invented (R0); every other value here is invented too.
    use super::*;
    use jiff::civil::{date, time};
    use serde_json::json;

    pub(crate) const FIXTURE: &str = include_str!("../tests/fixtures/registrar/banner-ua-registration.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).unwrap()
    }

    fn by_crn<'a>(p: &'a Parsed, crn: &str) -> &'a Series {
        let key = format!("registrar:ua:202640-{crn}");
        p.series.iter().find(|s| s.source_uid == key).unwrap_or_else(|| panic!("{key} not kept: {:?}", p.series))
    }

    #[test]
    fn the_fixture_parses_to_four_series_of_one_term() {
        let p = parse_banner(&fixture(), "ua").unwrap();
        assert_eq!(p.term, "202640");
        let keys: Vec<&str> = p.series.iter().map(|s| s.source_uid.as_str()).collect();
        assert_eq!(keys, ["registrar:ua:202640-40001", "registrar:ua:202640-40002", "registrar:ua:202640-40003", "registrar:ua:202640-40006"]);
        assert_eq!((p.no_time, p.midnight), (1, 1));
        assert!(p.warnings.iter().any(|w| w.contains("MUS 250") && w.contains("midnight")), "{:?}", p.warnings);
    }

    #[test]
    fn a_row_becomes_the_series_the_spec_table_names() {
        let p = parse_banner(&fixture(), "ua").unwrap();
        let s = by_crn(&p, "40001");
        assert_eq!(s.calendar, "registrar:ua:202640");
        assert_eq!(s.title, "CS 100");
        assert_eq!(s.where_.as_deref(), Some("Invented Hall 101"));
        assert_eq!(s.event_type.as_deref(), Some("registrar"));
        assert_eq!(s.rule, Rule { freq: "WEEKLY".into(), interval: 1, until: Some(date(2026, 12, 4)), count: None });
        assert!(s.has_master && !s.rdate && !s.unsupported);
        assert!(s.instances.is_empty(), "a registrar series carries no instances");
        assert_eq!(s.meets, vec![Meet { days: vec!["mon", "wed", "fri"], start: time(12, 0, 0, 0), end: time(12, 50, 0, 0) }]);
        assert_eq!((s.first, s.until, s.last_seen), (Some(date(2026, 8, 19)), Some(date(2026, 12, 4)), None));
    }

    #[test]
    fn a_laboratory_is_titled_lab_and_a_repeated_pattern_is_one_meet() {
        let p = parse_banner(&fixture(), "ua").unwrap();
        assert_eq!(by_crn(&p, "40002").title, "CS 100 Lab");
        let engl = by_crn(&p, "40003");
        assert_eq!(engl.meets.len(), 1, "{:?}", engl.meets);
        assert_eq!(engl.meets[0].days, vec!["tue", "thu"]);
        assert_eq!(engl.where_.as_deref(), Some("Pretend Library 210"));
        let mut v = fixture();
        v["data"][0]["scheduleTypeDescription"] = json!("Collaborative Seminar");
        assert_eq!(by_crn(&parse_banner(&v, "ua").unwrap(), "40001").title, "CS 100", "a word inside a word is not a lab");
    }

    #[test]
    fn where_is_cut_to_80_and_a_row_without_a_building_has_none() {
        let long = "B".repeat(90);
        let row = |building: Value, room: Value| json!([{ "term": "202640", "courseReferenceNumber": "1", "subject": "ZZT",
            "courseNumber": "101", "scheduleTypeDescription": "Lecture", "meetingsFaculty": [{ "meetingTime": {
            "beginTime": "0800", "endTime": "0850", "monday": true, "startDate": "08/19/2026", "endDate": "12/04/2026",
            "buildingDescription": building, "room": room } }] }]);
        let p = parse_banner(&row(json!(long), json!("1")), "ua").unwrap();
        assert_eq!(p.series[0].where_.as_deref().map(|w| w.chars().count()), Some(80));
        let p = parse_banner(&row(Value::Null, Value::Null), "ua").unwrap();
        assert_eq!(p.series[0].where_, None);
    }

    #[test]
    fn an_empty_parse_is_a_failure_never_an_empty_semester() {
        assert!(parse_banner(&json!([]), "ua").is_err());
        assert!(parse_banner(&json!({ "data": [] }), "ua").is_err());
        assert!(parse_banner(&json!({ "nothing": 1 }), "ua").is_err());
        let online_only = json!([{ "term": "202640", "courseReferenceNumber": "9", "subject": "ZZT", "courseNumber": "150",
            "meetingsFaculty": [{ "meetingTime": { "beginTime": null, "endTime": null } }] }]);
        let err = parse_banner(&online_only, "ua").unwrap_err();
        assert!(err.contains("1 without times"), "{err}");
    }

    #[test]
    fn another_term_or_a_repeated_crn_is_skipped_with_a_warning() {
        let mut v = fixture();
        let rows = v["data"].as_array_mut().unwrap();
        let mut other = rows[0].clone();
        other["term"] = json!("202710");
        other["courseReferenceNumber"] = json!("50001");
        rows.push(other);
        rows.push(rows[1].clone());
        let p = parse_banner(&v, "ua").unwrap();
        assert_eq!(p.series.len(), 4);
        assert!(p.warnings.iter().any(|w| w.contains("202710")), "{:?}", p.warnings);
        assert!(p.warnings.iter().any(|w| w.contains("40002") && w.contains("twice")), "{:?}", p.warnings);
    }

    #[test]
    fn the_ua_term_is_spring_to_may_and_fall_from_june() {
        assert_eq!(term_for("ua", date(2026, 9, 26)).as_deref(), Some("202640"));
        assert_eq!(term_for("ua", date(2026, 6, 1)).as_deref(), Some("202640"));
        assert_eq!(term_for("ua", date(2027, 1, 4)).as_deref(), Some("202710"));
        assert_eq!(term_for("ua", date(2027, 5, 31)).as_deref(), Some("202710"));
        assert_eq!(term_for("zz", date(2026, 9, 26)), None);
        assert_eq!(calendar_key("ua", "202640"), "registrar:ua:202640");
    }

    #[test]
    fn a_sign_in_page_is_not_a_schedule() {
        assert!(looks_signed_out("<!DOCTYPE html><html><body>Sign in</body></html>"));
        assert!(looks_signed_out(""));
        assert!(looks_signed_out("\"just a string\""));
        assert!(!looks_signed_out(FIXTURE));
        assert!(!looks_signed_out(" [] "));
    }
}
```

- [ ] **Step 2 — run and see them fail.** `cargo test -p knowlu-engine --lib -j 2 -- registrar::tests`
  Expected: compile errors (no `parse_banner`, `term_for`, `looks_signed_out`, `Parsed`).

- [ ] **Step 3 — implement.** Above the test module in `engine/src/registrar.rs`, in three edits
  (constraint 16): the header and school table, then `looks_signed_out` and the helpers, then
  `parse_banner`.

```rust
//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`):
//! the school registrar's schedule, parsed on the device. **Every Banner key this crate reads is in
//! [`parse_banner`]** (Plan ruling R1-a); its fixture is
//! `engine/tests/fixtures/registrar/banner-ua-registration.json`. Nothing here fetches: the app
//! carries the bytes (spec D4).
use std::collections::BTreeSet;

use jiff::civil::{Date, Time};
use serde_json::Value;

use crate::commitments::{Meet, Rule, Series};
use crate::weekcal::DayKey;

/// A school whose registrar Knowlu reads. Adding one is a code change (parent §10).
pub struct School {
    /// The `--school` value and the middle of every key: `registrar:<key>:<term>-<crn>`.
    pub key: &'static str,
    /// `(first month, term-code suffix)`, in month order (Plan ruling R1-f).
    pub terms: &'static [(i8, &'static str)],
}

/// UA only before the pilot (Quinn, 2026-09-23).
pub const SCHOOLS: [School; 1] = [School { key: "ua", terms: &[(1, "10"), (6, "40")] }];

/// Every registrar calendar key, and every registrar `source_uid`, starts with this.
pub const CALENDAR_PREFIX: &str = "registrar:";

/// A registrar series' `event_type` (spec §3's table).
pub const EVENT_TYPE: &str = "registrar";

/// §2.2's bound on `where`.
const WHERE_MAX: usize = 80;

pub fn school(key: &str) -> Option<&'static School> {
    SCHOOLS.iter().find(|s| s.key == key)
}

/// One calendar per term (Plan ruling R2-a): `registrar:ua:202640`.
pub fn calendar_key(school: &str, term: &str) -> String {
    format!("{CALENDAR_PREFIX}{school}:{term}")
}

/// The term to fetch on `today` (Plan ruling R1-f): the last `terms` entry whose month has come.
pub fn term_for(school_key: &str, today: Date) -> Option<String> {
    let s = school(school_key)?;
    let (_, suffix) = s.terms.iter().rev().find(|(month, _)| *month <= today.month())?;
    Some(format!("{}{suffix}", today.year()))
}
```

```rust
/// Plan ruling R1-g: a signed-out session gets the school's HTML sign-in page, never JSON.
pub fn looks_signed_out(body: &str) -> bool {
    !matches!(serde_json::from_str::<Value>(body.trim()), Ok(Value::Array(_)) | Ok(Value::Object(_)))
}

/// What [`parse_banner`] kept and dropped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parsed {
    pub term: String,
    /// Sorted by `source_uid`.
    pub series: Vec<Series>,
    /// Rows with no meeting time (online, TBA): dropped and counted.
    pub no_time: usize,
    /// Rows with a meeting across midnight: dropped with a warning (§2.2).
    pub midnight: usize,
    pub warnings: Vec<String>,
}

const DAY_FIELDS: [(&str, DayKey); 7] = [
    ("monday", "mon"), ("tuesday", "tue"), ("wednesday", "wed"), ("thursday", "thu"),
    ("friday", "fri"), ("saturday", "sat"), ("sunday", "sun"),
];

/// A trimmed, non-empty string field.
fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Banner's `"0930"`.
fn hhmm(v: &Value, key: &str) -> Option<Time> {
    let s = text(v, key)?;
    if s.len() != 4 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Time::new(s[..2].parse().ok()?, s[2..].parse().ok()?, 0, 0).ok()
}

/// Banner's `"MM/DD/YYYY"`.
fn mdy(v: &Value, key: &str) -> Option<Date> {
    Date::strptime("%m/%d/%Y", text(v, key)?).ok()
}

/// Plan ruling R1-c.
fn place(mt: &Value) -> Option<String> {
    let building = text(mt, "buildingDescription").or_else(|| text(mt, "building"));
    let joined = [building, text(mt, "room")].into_iter().flatten().collect::<Vec<_>>().join(" ");
    let cut: String = joined.chars().take(WHERE_MAX).collect::<String>().trim_end().to_string();
    (!cut.is_empty()).then_some(cut)
}
```

```rust
/// Banner 9's registration rows → [`Series`] (spec §3; shape (b) as R0 recorded it). A row is kept
/// when it has a term, a CRN, a subject, a course number and at least one pattern with days and
/// both times (R1-d). `Err` when no row is kept: an empty parse is a failure, never an empty
/// semester. `school` goes into the key only.
pub fn parse_banner(json: &Value, school: &str) -> Result<Parsed, String> {
    let rows = match json {
        Value::Array(rows) => rows,
        _ => json.get("data").and_then(Value::as_array).ok_or("not a list of registration rows")?,
    };
    let mut out = Parsed::default();
    let mut kept: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        let (Some(term), Some(crn), Some(subject), Some(number)) =
            (text(row, "term"), text(row, "courseReferenceNumber"), text(row, "subject"), text(row, "courseNumber"))
        else {
            out.warnings.push("registrar: a row without a term, CRN, subject or course number; skipped".into());
            continue;
        };
        let name = format!("{subject} {number}");
        // A whole word, so "Collaborative Seminar" is not a lab (plan review M2).
        let lab = text(row, "scheduleTypeDescription").is_some_and(|t| {
            t.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w.eq_ignore_ascii_case("lab") || w.eq_ignore_ascii_case("laboratory"))
        }) || text(row, "scheduleType").is_some_and(|t| t.eq_ignore_ascii_case("LAB"));
        let mut meets: Vec<Meet> = Vec::new();
        let (mut first, mut until, mut place_of, mut crosses) = (None::<Date>, None::<Date>, None, false);
        for mf in row.get("meetingsFaculty").and_then(Value::as_array).into_iter().flatten() {
            let Some(mt) = mf.get("meetingTime") else { continue };
            let days: Vec<DayKey> = DAY_FIELDS.iter().filter(|(f, _)| mt.get(*f).and_then(Value::as_bool) == Some(true)).map(|(_, d)| *d).collect();
            let (Some(start), Some(end)) = (hhmm(mt, "beginTime"), hhmm(mt, "endTime")) else { continue };
            if days.is_empty() {
                continue;
            }
            if end <= start {
                crosses = true;
                continue;
            }
            let meet = Meet { days, start, end };
            if !meets.contains(&meet) {
                meets.push(meet);
            }
            first = [first, mdy(mt, "startDate")].into_iter().flatten().min();
            until = [until, mdy(mt, "endDate")].into_iter().flatten().max();
            place_of = place_of.or_else(|| place(mt));
        }
        if crosses {
            out.midnight += 1;
            out.warnings.push(format!("registrar: {name} meets across midnight; dropped"));
            continue;
        }
        if meets.is_empty() {
            out.no_time += 1;
            continue;
        }
        if out.term.is_empty() {
            out.term = term.to_string();
        } else if out.term != term {
            out.warnings.push(format!("registrar: {name} is in term {term}, not {}; skipped", out.term));
            continue;
        }
        let key = format!("{CALENDAR_PREFIX}{school}:{term}-{crn}");
        if !kept.insert(key.clone()) {
            out.warnings.push(format!("registrar: CRN {crn} listed twice; the first kept"));
            continue;
        }
        out.series.push(Series {
            source_uid: key,
            calendar: calendar_key(school, term),
            title: if lab { format!("{name} Lab") } else { name },
            where_: place_of,
            event_type: Some(EVENT_TYPE.to_string()),
            rule: Rule { freq: "WEEKLY".into(), interval: 1, until, count: None },
            has_master: true,
            rdate: false,
            unsupported: false,
            instances: Vec::new(),
            meets,
            first,
            until,
            last_seen: None,
        });
    }
    if out.series.is_empty() {
        return Err(format!(
            "no class with meeting times in the registrar's answer ({} without times, {} across midnight)",
            out.no_time, out.midnight
        ));
    }
    out.series.sort_by(|a, b| a.source_uid.cmp(&b.source_uid));
    Ok(out)
}
```

  Watch: the test `another_term_or_a_repeated_crn_is_skipped_with_a_warning` looks for "40002" and
  "twice" in one warning: the message above names the CRN.

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- registrar::tests` (8 pass), then
  `cargo test -p knowlu-engine --lib -j 2` (all pass) and `git diff --exit-code eb10c1b -- engine/tests/fixtures ':!engine/tests/fixtures/registrar'`.

- [ ] **Step 5 — commit.** `git add engine/tests/fixtures/registrar/banner-ua-registration.json engine/src/registrar.rs engine/src/lib.rs`; message:

```
feat(engine): parse the registrar's Banner rows into series (phase 3, §3)

registrar::parse_banner is the one place that knows Banner's shape: a row with
days and times becomes a weekly series keyed registrar:<school>:<term>-<crn>,
" Lab" for a laboratory, where from building and room; online rows are counted,
midnight rows warned, and an empty parse is an error. term_for and
looks_signed_out give the app its term and its "not signed in yet".

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

---

## R2 — D5 and D7: the series file, the classifier, precedence, the change watch and the ask gate

**Files.**
- `engine/src/commitments.rs`:
  - `classify` (lines 1274–1352): a registrar arm first; new `registrar_course` and
    `registrar_class` just above it (after `section_kind`, which ends before line 1274).
  - `precedence` (1414–1418).
  - `SeriesFile::instances_map` (1445–1462).
  - `refresh_series` (2061–2170: the doc comment, `configured` at 2100, the `old.series` loop at
    2122–2135, the `Ended` construction at 2149–2154).
  - `proposals`' inner `fn rank` (2351–2353).
  - `SERIES_KEY_PREFIXES` (3363–3364).
  - `create_confirmed_as`' body sentence (3910–3916).
  - A new `asks_wait_for_registrar` after `emit_asks` (it ends just before `pub enum AskSettled`, line 3171).
- `engine/src/cli.rs`: `commitment_passes` (803–870): `fresh_keys` (line 834) and the `emit_asks`
  guard (line 860).
- `engine/src/registrar.rs`: a second test module, `d5_d7_tests`.
- `engine/src/cli.rs`'s `mod tests`: one test after `rank_files_an_ask_for_a_course_with_no_class_from_day_three`
  (ends at line 2780).

**Interfaces.**
- Produces `pub fn registrar_course(series: &Series, codes: &Codes) -> Option<String>` (the course
  slug by `class_course` over the title), `pub fn registrar_class(series: &Series, codes: &Codes) -> Class`,
  and `pub fn asks_wait_for_registrar(file: &SeriesFile, today: Date) -> bool`.
- Changes `precedence` to `fn precedence(series: &Series) -> (u8, &str)`; every caller compares
  it and keeps working (`by_key`, `proposals`' `rank`, `detect_changes`' `min_by`, `refresh_series`'
  `aged_out` sort).
- Consumes `registrar::{CALENDAR_PREFIX, parse_banner}` and R1's fixture (tests only).

**Plan ruling R2-a: a registrar calendar key is `registrar:<school>:<term>`,** one calendar per
term (`registrar::calendar_key`), where the spec says `registrar:ua`. *Why:* UA students register
for spring in November, mid-fall. A fetch of the spring term under one `registrar:ua` key would
replace the fall rows, and D5's rule would then end every fall course that day. One key per term
means a fetch replaces only its own term. The `source_uid` is R1's `registrar:ua:<term>-<crn>`,
unchanged. (Spec problem 1.)
**Plan ruling R2-b: a registrar calendar counts as configured while the file holds a series under
it whose `until` is open or less than 28 days (`ENDED_DAYS`) before `today`;** after that it ages
like a removed feed: its series and its date go 14 days after its last read, never into `ended`.
*Why:* D5 without an end would keep every past term in the series file forever.
**Plan ruling R2-c: a registrar row that a fresh fetch of its own term no longer returns goes to
`ended` that day, with `last_instance` = the day before the fetch** (its `until` stays the term's
end). *Why:* D5 says "at once". A registrar series carries no instances, so `last_instance` would be
`None`, `detect_changes`' ended path would take the term's end, and the `note.until <= end` test
would then file no end card. The day before the fetch is the last day the registrar still named
the course. (Spec problem 2.)
**Plan ruling R2-d: `instances_map` leaves registrar series out.** *Why:* the map says "inside the
horizon only the actual instances are busy". A registrar series has none, so its confirmed notes
would block nothing for 28 days after every fetch. Left out, `commitment_busy_on` and `weekcal`
use the note's weekly `meets`, which is what the registrar gives. (Spec problem 3.)
**Plan ruling R2-e: `classify` answers a registrar series before `eligible`:** kind `lab` when the
title ends `" Lab"` (R1 wrote it from the schedule type), else `class`; the course is the code
table's match for the title's leading code (`class_course`), or none. *Why:* spec §3, "the
classifier is not consulted, because the registrar is the authority", and `eligible` refuses a
series with no instances and an `event_type` other than `default`.
**Plan ruling R2-f: `precedence` is `(tier, calendar)`, tier 0 for `registrar:`, 1 for
`google:`, 2 for anything else.** *Why:* D7. For two non-registrar calendars the order is the one
the old `(bool, &str)` gave, so no existing test moves.
**Plan ruling R2-g: `detect_changes` watches `registrar:` keys (`SERIES_KEY_PREFIXES`), and `rank`
adds every registrar calendar in the file to the `fresh` set it passes.** *Why:* the spec relies on
§5.4's existing change and end cards, but the change path runs only for a calendar read fresh in
the same `rank`, and a registrar is read only on a button press. The file always holds the latest
fetch, so treating it as fresh on every run is exact. A card already filed or answered is not
re-filed: `emit_checks` dedupes on `(target, change)` (`asked`, `commitments.rs:2882`). (Spec problem 5.)
**Plan ruling R2-h: the ask gate is `asks_wait_for_registrar(file, today)`, called in
`cli::commitment_passes` beside `read_failed`.** It is true while the earliest `first` among the
registrar series whose `until` is open or not yet past is on or after `today`. *Why:* D7's "until
the day after the registrar's term starts"; outside `emit_asks`, so its direct tests stay valid
(phase 2's Q1-b pattern).
**Plan ruling R2-i: a note confirmed from a `registrar:` key carries the body "Found in your
school's class schedule."** *Why:* `create_confirmed_as` names the source in the body, and
"Found as a weekly series on your calendar" would be false.

- [ ] **Step 1 — failing tests.** At the end of `engine/src/registrar.rs`, a second module, added in
  two edits:

```rust
#[cfg(test)]
mod d5_d7_tests {
    //! Phase 3, D5 and D7 in `commitments.rs`, driven with R1's parse of the invented fixture.
    use super::*;
    use crate::commitments::{self as cm, Class, Codes, Commitment, Commitments, Instance, Level, SeriesFile};
    use jiff::civil::date;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    const CAL: &str = "registrar:ua:202640";

    fn fall() -> Vec<Series> {
        parse_banner(&serde_json::from_str(super::tests::FIXTURE).unwrap(), "ua").unwrap().series
    }

    fn without(crn: &str) -> Vec<Series> {
        fall().into_iter().filter(|s| !s.source_uid.ends_with(crn)).collect()
    }

    fn codes() -> Codes {
        Codes { table: [("CS100".to_string(), "cs-100".to_string())].into_iter().collect(), names: BTreeMap::new() }
    }

    /// A scratch vault with only a timezone in `config/ingest.yaml`: no feed is configured.
    fn vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowlu-p3-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\n").unwrap();
        dir
    }

    fn keys(file: &SeriesFile) -> Vec<&str> {
        file.series.iter().map(|s| s.source_uid.as_str()).collect()
    }

    #[test]
    fn the_registrar_arm_takes_kind_from_the_title_and_course_from_the_code_table() {
        let f = fall();
        let class = |uid: &str| cm::classify(f.iter().find(|s| s.source_uid.ends_with(uid)).unwrap(), &codes(), &[]);
        assert_eq!(class("40001"), Some(Class::Kind { kind: "class".into(), course: Some("cs-100".into()) }));
        assert_eq!(class("40002"), Some(Class::Kind { kind: "lab".into(), course: Some("cs-100".into()) }));
        assert_eq!(class("40006"), Some(Class::Kind { kind: "class".into(), course: None }));
    }

    #[test]
    fn a_registrar_calendar_stays_configured_while_its_term_is_in_play() {
        let v = vault("inplay");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 8, 20));
        // Two months on, another calendar's fresh read rewrites the file: the term stays whole.
        let (file, _) = cm::refresh_series(&v, &[("personal".to_string(), Vec::new())], date(2026, 10, 20));
        assert_eq!(keys(&file).len(), 4, "{:?}", keys(&file));
        assert_eq!(file.calendars.get(CAL), Some(&date(2026, 8, 20)));
        // 28 days past the term's end (2026-12-04), it ages out like a removed feed: never `ended`.
        let (file, _) = cm::refresh_series(&v, &[("personal".to_string(), Vec::new())], date(2027, 1, 1));
        assert!(keys(&file).is_empty(), "{:?}", keys(&file));
        assert!(file.ended.is_empty() && !file.calendars.contains_key(CAL));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_row_missing_from_a_fresh_fetch_of_its_term_ends_that_day() {
        let v = vault("dropped");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 8, 20));
        let (file, _) = cm::refresh_series(&v, &[(CAL.to_string(), without("40002"))], date(2026, 9, 10));
        assert_eq!(keys(&file).len(), 3);
        let gone = file.ended.get("registrar:ua:202640-40002").expect("the dropped lab is in ended");
        assert_eq!((gone.calendar.as_str(), gone.dropped), (CAL, date(2026, 9, 10)));
        assert_eq!((gone.last_instance, gone.until), (Some(date(2026, 9, 9)), Some(date(2026, 12, 4))));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_next_terms_fetch_leaves_this_term_alone() {
        let v = vault("twoterms");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 11, 10));
        let spring: Vec<Series> = fall().into_iter().map(|mut s| {
            s.source_uid = s.source_uid.replace("202640", "202710");
            s.calendar = "registrar:ua:202710".into();
            s
        }).collect();
        let (file, _) = cm::refresh_series(&v, &[("registrar:ua:202710".to_string(), spring)], date(2026, 11, 15));
        assert_eq!(keys(&file).len(), 8, "{:?}", keys(&file));
        assert!(file.ended.is_empty(), "{:?}", file.ended);
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_registrar_series_blocks_by_its_meets_not_by_instances() {
        let mut file = SeriesFile::default();
        file.calendars.insert(CAL.into(), date(2026, 9, 1));
        file.series = fall();
        assert!(file.instances_map().keys().all(|k| !k.starts_with(CALENDAR_PREFIX)), "{:?}", file.instances_map().keys());
    }
```

```rust
    /// D7: the registrar's CS 100 and a Google series with the same signature are one proposal,
    /// and it is the registrar's.
    #[test]
    fn a_registrar_series_outranks_its_google_twin() {
        let today = date(2026, 9, 1);
        let reg = fall().into_iter().find(|s| s.source_uid.ends_with("40001")).unwrap();
        let mut google = reg.clone();
        google.source_uid = "gcal-series:invented".into();
        google.calendar = "google:invented".into();
        google.event_type = Some("default".into());
        google.last_seen = Some(today);
        google.instances = [31, 2, 4, 7, 9, 11].iter().map(|d| {
            let day = if *d == 31 { date(2026, 8, 31) } else { date(2026, 9, *d) };
            Instance { date: day, start: Some(reg.meets[0].start), end: Some(reg.meets[0].end) }
        }).collect();
        let mut file = SeriesFile::default();
        for (cal, s) in [(CAL, reg), ("google:invented", google)] {
            file.calendars.insert(cal.into(), today);
            file.series.push(s);
        }
        file.series.sort_by(|a, b| (&a.source_uid, &a.calendar).cmp(&(&b.source_uid, &b.calendar)));
        let template = crate::weekcal::WeekCalendar::new(&serde_yaml_ng::Mapping::new(), Vec::new());
        let got = cm::proposals(&file, &Commitments::default(), &codes(), &[], &template, &BTreeSet::new(), today, false);
        let keys: Vec<&str> = got.iter().filter(|p| !p.is_window()).map(|p| p.source_uid.as_str()).collect();
        assert_eq!(keys, ["registrar:ua:202640-40001"]);
    }

    /// D5 + §5.4: a confirmed registrar note whose row left the term files an end card at the day
    /// before the fetch (R2-c's `last_instance`), and `registrar:` keys are watched (R2-g).
    #[test]
    fn a_dropped_registrar_course_ends_its_confirmed_note() {
        let v = vault("endcard");
        cm::refresh_series(&v, &[(CAL.to_string(), fall())], date(2026, 8, 20));
        let (file, _) = cm::refresh_series(&v, &[(CAL.to_string(), without("40002"))], date(2026, 9, 10));
        let lab = fall().into_iter().find(|s| s.source_uid.ends_with("40002")).unwrap();
        let note = Commitment {
            id: "cmt_invented01".into(), path: PathBuf::from("commitments/cs-100-lab.md"), kind: "lab".into(),
            level: Level::Hard, title: lab.title.clone(), course: Some("cs-100".into()), meets: lab.meets.clone(),
            where_: lab.where_.clone(), from: lab.first, until: lab.until, source_uid: Some(lab.source_uid.clone()),
        };
        let set = Commitments { confirmed: vec![note], ..Commitments::default() };
        let mut journal = crate::journal::Journal::new(&v);
        let fresh: BTreeSet<String> = [CAL.to_string()].into_iter().collect();
        let (changes, warnings) = cm::detect_changes(&file, &set, &codes(), &[], &fresh, date(2026, 9, 10), &mut journal);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(changes[0].change.get("until").and_then(|u| u.as_str()), Some("2026-09-09"));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn asks_wait_until_the_day_after_the_registrars_term_starts() {
        let mut file = SeriesFile::default();
        assert!(!cm::asks_wait_for_registrar(&file, date(2026, 8, 1)), "no registrar, no wait");
        file.series = fall();
        assert!(cm::asks_wait_for_registrar(&file, date(2026, 8, 1)));
        assert!(cm::asks_wait_for_registrar(&file, date(2026, 8, 19)), "the term's first day still waits");
        assert!(!cm::asks_wait_for_registrar(&file, date(2026, 8, 20)), "the day after it does not");
        assert!(!cm::asks_wait_for_registrar(&file, date(2026, 12, 20)), "a term already over holds nothing");
    }
}
```

  In `engine/src/cli.rs`'s `mod tests`, after `rank_files_an_ask_for_a_course_with_no_class_from_day_three`
  (it ends at line 2780):

```rust
    /// Phase-3 D7 (Plan ruling R2-h): a vault holding a registrar term that has not started yet
    /// files no ask card; the day after the term starts, the ask comes as before. ART 110 is a
    /// registrar row with no vault course, so CS 100 stays uncovered and only the gate holds it.
    #[test]
    fn asks_wait_for_the_registrars_term_to_start() {
        let vault = p16_vault("p3gate");
        let text = include_str!("../tests/fixtures/registrar/banner-ua-registration.json");
        let parsed = crate::registrar::parse_banner(&serde_json::from_str(text).unwrap(), "ua").unwrap();
        let mut art = parsed.series.into_iter().find(|s| s.source_uid.ends_with("40006")).unwrap();
        art.first = Some(Date::constant(2026, 9, 14));
        crate::commitments::refresh_series(&vault, &[(art.calendar.clone(), vec![art])], P16_MONDAY);
        rank_p16(&vault, p16_day(1), Vec::new());
        assert!(md_names(&vault.join("approvals"), "commitment-ask-").is_empty(), "the term starts on 09-14");
        rank_p16(&vault, Date::constant(2026, 9, 15), Vec::new());
        assert_eq!(md_names(&vault.join("approvals"), "commitment-ask-"), ["commitment-ask-when-does-cs-100-intro-to-computing-meet.md"]);
        let _ = std::fs::remove_dir_all(&vault);
    }
```

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu-engine --lib -j 2 -- registrar::d5_d7_tests cli::tests::asks_wait_for_the_registrars`
  Expected: compile errors (no `asks_wait_for_registrar`); once stubbed, the classify, instances,
  ended, twin and end-card tests fail on the old behaviour.

- [ ] **Step 3 — implement,** one edit per item, each ≤ 60 lines.

  **(a)** `commitments.rs`, just above `pub fn classify` (line 1274):

```rust
/// The course a registrar series names (Plan ruling R2-e): the code table's slug for its title's
/// leading code, or `None` when the vault has no such course.
pub fn registrar_course(series: &Series, codes: &Codes) -> Option<String> {
    class_course(&series.title, &codes.table).map(|(slug, _)| slug)
}

/// A registrar series' class, without the classifier (spec §3: the registrar is the authority):
/// `lab` when R1 titled it `"… Lab"`, else `class`.
pub fn registrar_class(series: &Series, codes: &Codes) -> Class {
    let kind = if series.title.ends_with(" Lab") { "lab" } else { "class" };
    Class::Kind { kind: kind.to_string(), course: registrar_course(series, codes) }
}
```

  and the first lines of `classify`'s body, before `if !eligible(series)`:

```rust
    // Phase 3 (Plan ruling R2-e): a registrar row carries no instances and needs no eligibility.
    if series.calendar.starts_with(crate::registrar::CALENDAR_PREFIX) {
        return Some(registrar_class(series, codes));
    }
```

  **(b)** `precedence` (lines 1414–1418) becomes:

```rust
/// The one precedence between two records of one key or one signature (I2, phase-3 D7): a
/// `registrar:` calendar first, then a `google:` one, then any other; then the lower calendar key.
/// Smaller wins (Plan ruling R2-f).
fn precedence(series: &Series) -> (u8, &str) {
    let tier = if series.calendar.starts_with(crate::registrar::CALENDAR_PREFIX) {
        0
    } else if series.calendar.starts_with("google:") {
        1
    } else {
        2
    };
    (tier, series.calendar.as_str())
}
```

  and `proposals`' inner `fn rank(s: &Series) -> ((bool, &str), &str)` (line 2351) becomes
  `fn rank(s: &Series) -> ((u8, &str), &str)`. `proposals`' doc comment, which describes the old
  order ("a `google:` calendar, then the lower calendar key"), gains "a `registrar:` calendar, then"
  in front.

  **(c)** `instances_map` (line 1445): the chain `self.by_key().into_iter().filter_map(…)` gains
  `.filter(|(_, series)| !series.calendar.starts_with(crate::registrar::CALENDAR_PREFIX))` between
  `.into_iter()` and `.filter_map(…)`, and one sentence at the end of its doc comment: "A registrar series
  carries no instances and is left out, so its notes are busy by their weekly `meets` (phase 3,
  Plan ruling R2-d)."

  **(d)** `refresh_series`. After the `feeds`/`names`/`google` lines (2097–2099) and before
  `configured` (2100):

```rust
    // Phase 3 (Plan rulings R2-a, R2-b): a registrar calendar is one term, read only when the
    // student fetches it. It counts as configured while it holds a series whose `until` is open or
    // less than 28 days past; then it ages like a removed feed.
    let registrar = |calendar: &str| calendar.starts_with(crate::registrar::CALENDAR_PREFIX);
    let in_play: BTreeSet<String> = old
        .series
        .iter()
        .filter(|s| registrar(&s.calendar) && s.until.is_none_or(|u| days_since(u, today) < ENDED_DAYS))
        .map(|s| s.calendar.clone())
        .collect();
```

  `configured` becomes:

```rust
    let configured = |calendar: &str| {
        if registrar(calendar) {
            in_play.contains(calendar)
        } else if calendar.starts_with("google:") {
            google
        } else {
            names.contains(calendar)
        }
    };
```

  In the `for held in old.series` match (2122–2135), a new arm right after
  `Some(keys) if keys.contains(held.source_uid.as_str()) => {}`:

```rust
            // D5 (Plan ruling R2-c): a registrar fetch is the whole term's truth; a row it no
            // longer returns is a dropped course, at once.
            Some(_) if registrar(&held.calendar) => aged_out.push(held),
```

  In the `Ended { … }` construction (2149–2154), `last_instance` becomes:

```rust
            // A registrar series has no instances: the day before the fetch that dropped it is the
            // last day the registrar named it (Plan ruling R2-c).
            last_instance: match registrar(&gone.calendar) {
                true => Some(add_days(today, -1)),
                false => gone.instances.iter().map(|i| i.date).max(),
            },
```

  and the doc comment (2061–2078) gains one bullet: "- A `registrar:` calendar (phase 3, D5) is
  configured while its term is in play (a series with an open `until` or one under 28 days past);
  a row its own fresh fetch no longer returns moves to `ended` that day, with the day before as its
  last instance."
  Borrow note: `in_play` is built from `&old.series` before `old.calendars` and `old.series` are
  moved out, so it goes right after `load_series_file`'s match, not later.

  **(e)** `SERIES_KEY_PREFIXES` (line 3364) becomes
  `const SERIES_KEY_PREFIXES: [&str; 3] = ["gcal-series:", "ics-series:", "registrar:"];` and its
  doc says "calendar or registrar series". `detect_changes`' doc line "confirmed notes keyed
  `gcal-series:`/`ics-series:`" becomes "keyed `gcal-series:`, `ics-series:` or `registrar:`".

  **(f)** `create_confirmed_as`' body chain (3910–3916) gains, before the final `else`:

```rust
        } else if key.starts_with(crate::registrar::CALENDAR_PREFIX) {
            "Found in your school's class schedule.\n"
```

  **(g)** After `emit_asks` (before `pub enum AskSettled`, line 3171):

```rust
/// Phase-3 D7 (Plan ruling R2-h): while the vault holds a registrar term that has not begun, the
/// per-course asks wait for it. True while the earliest `first` among registrar series whose
/// `until` is open or not yet past is `today` or later: asks resume the day after the term starts.
pub fn asks_wait_for_registrar(file: &SeriesFile, today: Date) -> bool {
    file.series
        .iter()
        .filter(|s| s.calendar.starts_with(crate::registrar::CALENDAR_PREFIX))
        .filter(|s| s.until.is_none_or(|u| u >= today))
        .filter_map(|s| s.first)
        .min()
        .is_some_and(|start| today <= start)
}
```

  **(h)** `cli.rs` `commitment_passes`: line 834 becomes

```rust
    let mut fresh_keys: std::collections::BTreeSet<String> = fresh.iter().map(|(c, _)| c.clone()).collect();
    // Phase 3 (Plan ruling R2-g): a registrar term is read only on a button press, and the file
    // always holds that read, so its notes are watched on every run.
    fresh_keys.extend(file.calendars.keys().filter(|c| c.starts_with(crate::registrar::CALENDAR_PREFIX)).cloned());
```

  and line 860, `if !read_failed {` (the ask block), becomes
  `if !read_failed && !cm::asks_wait_for_registrar(&file, today) {`, with the comment above it
  gaining "…, and while a registrar term has not begun (phase-3 D7, Plan ruling R2-h)".

- [ ] **Step 4 — run.**
  `cargo test -p knowlu-engine --lib -j 2 -- registrar:: cli::tests::asks_wait_for_the_registrars`
  (16 + 1 pass); then `cargo test -p knowlu-engine --lib -j 2` (all pass: the precedence tiers keep
  every non-registrar order, so no P8–P16 test moves; if one does, stop and report it, do not edit
  its assertion); then `cargo test -p knowlu-engine --test oracle --test surface_oracle -j 2` and
  `git diff --exit-code eb10c1b -- engine/tests/fixtures ':!engine/tests/fixtures/registrar'`.

- [ ] **Step 5 — commit.** `git add engine/src/commitments.rs engine/src/cli.rs engine/src/registrar.rs`; message:

```
feat(engine): registrar series persist by term and outrank Google (phase 3, D5, D7)

A registrar calendar is one term, configured while the term is in play; a row a
fresh fetch drops ends that day, dated the day before, so its note gets an end
card. Registrar series classify without the classifier, block by meets, win a
signature twin over Google and ICS, are watched for changes on every rank, and
hold the per-course asks until the day after their term starts.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

---

## R3 — `registrar::run`, `commitments --registrar`, the exit codes and the no-network pin (spec §3, D4, D8)

**Files.**
- `engine/src/registrar.rs`: `Report`, `run`, `status`; a third test module, `run_tests`.
- `engine/src/commitments.rs`: `Overview` (lines 4566–4592) and `overview` (4594–4632) gain
  `registrar` and `registrar_proposals`.
- `engine/src/cli.rs`: `commitments_registrar` after `commitments_confirm` (lines 1033–1047); one
  test after R2's `asks_wait_for_the_registrars_term_to_start`.
- `engine/src/main.rs`: `Command::Commitments` (lines 64–85) and its arm (369–412).
- `engine/tests/commitments_registrar.rs` (new).

**Interfaces.**
- Produces in `registrar`: `pub struct Report { term, rows, confirmed, proposed, no_time, midnight,
  warnings }` with `to_json()` →
  `{"confirmed", "dropped": {"midnight", "no_time"}, "proposed", "rows", "term", "warnings"}`
  (spec §3's keys; `dumps_value` sorts them); `pub fn run(vault: &Path, text: &str, school: &str,
  today: Date, ctx: &WriteContext, journal: &mut Journal) -> Result<Report, String>`; `pub fn
  status(file: &SeriesFile, today: Date) -> serde_json::Value` →
  `{"school", "held": [terms], "current", "refresh"}` or `null`.
- Produces `pub fn cli::commitments_registrar(vault: &Path, today_iso: Option<&str>, text: &str,
  school: &str, ctx: &WriteContext) -> Result<registrar::Report, String>`.
- The command: `commitments --vault <v> [--today YYYY-MM-DD] --registrar <file> --school <key>
  [--actor quinn] [--via dashboard]`; `--actor` and `--via` keep their defaults, `quinn` and
  `dashboard`, and the R24 notes are written under them (R3-c).
- Consumes `commitments::{refresh_series, stored_proposals, confirm, ConfirmInput,
  registrar_course, proposal_value}` (`stored_proposals` 4362, `ConfirmInput` 4379, `confirm` 4457,
  `proposal_value` 2740, all `pub`); `WriteContext::new` (`write.rs:139`).

**Plan ruling R3-a:** `--registrar` is declared `conflicts_with = "confirm"` and
`requires = "school"`, so clap refuses either misuse with its own exit 2 before anything runs; an
unknown `--school` value is `run`'s first check. *Why:* spec §3's exit codes, with nothing written.
**Plan ruling R3-b:** `run`'s `mine` holds each kept row whose `registrar_course` is `Some` **and**
whose key is a current proposal (`stored_proposals(vault, today).proposals`). A row already
confirmed, or closed by an existing note's signature, is left out silently. `confirmed` is
`confirm`'s `created` (0 on a repeat fetch); `proposed` counts the kept rows with no course.
*Why:* R24 is idempotent, and `confirm` warns "not a current proposal" for every left-out key,
which a repeat fetch would otherwise print for every class.
**Plan ruling R3-c (amended at the plan review): the R24 notes are written under the `ctx` the
command is given, `quinn` via `dashboard`,** as spec §3 says: the student pressed the button, and
phase 2's confirm screen writes the same way. *Why the cards still come:* `journal::human_set`
(`journal.rs:271–282`) reads a `create` by `quinn` as a human set of every field it wrote, which
would switch off the §5.4 change and end cards D5 relies on. The p2 judge-once fix (Preconditions)
makes change detection count only a later `set` by `quinn`, so a create no longer locks a field.
The plan's earlier answer, writing as `agent:commitments`, is withdrawn. (Spec problem 4.)
**Plan ruling R3-d:** `Overview` gains `registrar` (`registrar::status`) and
`registrar_proposals` (the current proposals whose key starts `registrar:`, as `proposal_value`).
`status.refresh` is true when `term_for(school, today)` is a term the file does not hold. *Why:*
D6/D8's label rule and §2's "the others appear as proposal rows" need data the *Schedule* view
does not have today. `term_for` switches on 1 January and 1 June, which stands in for Banner's term
list until R0 shows one (D8 allows either).

- [ ] **Step 1 — failing tests.** At the end of `engine/src/registrar.rs`:

```rust
#[cfg(test)]
mod run_tests {
    //! Phase 3, `run` and `status`, on a scratch vault whose only course is CS 100. Invented data.
    use super::*;
    use crate::journal::Journal;
    use crate::write::WriteContext;
    use jiff::civil::date;
    use std::path::PathBuf;

    fn vault(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowlu-p3run-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::create_dir_all(dir.join("courses")).unwrap();
        std::fs::write(dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\n").unwrap();
        std::fs::write(dir.join("courses").join("cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
        dir
    }

    fn fetch(v: &Path, text: &str, day: Date) -> Result<Report, String> {
        run(v, text, "ua", day, &WriteContext::new("quinn", "dashboard"), &mut Journal::new(v))
    }

    #[test]
    fn matched_rows_are_confirmed_by_the_student_and_the_rest_proposed() {
        let v = vault("r24");
        let r = fetch(&v, super::tests::FIXTURE, date(2026, 9, 1)).unwrap();
        assert_eq!((r.term.as_str(), r.rows, r.confirmed, r.proposed, r.no_time, r.midnight), ("202640", 4, 2, 2, 1, 1));
        let lab = std::fs::read_to_string(v.join("commitments").join("cs-100-lab.md")).unwrap();
        for line in ["kind: lab", "level: hard", "course: cs-100", "source_uid: registrar:ua:202640-40002", "until: 2026-12-04"] {
            assert!(lab.contains(line), "{line} missing: {lab}");
        }
        assert!(lab.contains("Found in your school's class schedule."), "{lab}");
        let journal: String = std::fs::read_dir(v.join("state").join("journal")).unwrap().flatten()
            .map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
        assert!(journal.contains("\"actor\": \"quinn\"") && journal.contains("\"via\": \"dashboard\""), "{journal}");
        assert!(!journal.contains("\"actor\": \"agent:commitments\""), "R3-c: the R24 writes are the student's (spec §3)");
        let again = fetch(&v, super::tests::FIXTURE, date(2026, 9, 2)).unwrap();
        assert_eq!((again.confirmed, again.warnings.iter().filter(|w| w.contains("not a current proposal")).count()), (0, 0));
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn a_bad_school_or_file_writes_nothing() {
        let v = vault("bad");
        assert!(run(&v, super::tests::FIXTURE, "zz", date(2026, 9, 1), &WriteContext::new("quinn", "dashboard"), &mut Journal::new(&v)).is_err());
        assert!(fetch(&v, "<html>sign in</html>", date(2026, 9, 1)).is_err());
        assert!(fetch(&v, "[]", date(2026, 9, 1)).is_err());
        assert!(!v.join("state").exists() && !v.join("commitments").exists());
        let _ = std::fs::remove_dir_all(&v);
    }

    #[test]
    fn the_overview_names_the_term_and_the_registrar_proposals() {
        let v = vault("overview");
        fetch(&v, super::tests::FIXTURE, date(2026, 9, 1)).unwrap();
        let o = crate::commitments::overview(&v, date(2026, 9, 1));
        assert_eq!(o.registrar, serde_json::json!({ "school": "ua", "held": ["202640"], "current": "202640", "refresh": false }));
        let keys: Vec<&str> = o.registrar_proposals.iter().map(|p| p["source_uid"].as_str().unwrap()).collect();
        assert_eq!(keys, ["registrar:ua:202640-40003", "registrar:ua:202640-40006"]);
        // What `your_week` hands the page (R5 reads `week.registrar` and `week.registrar_proposals`).
        let json = o.to_json();
        assert_eq!((&json["registrar"], json["registrar_proposals"].as_array().map(Vec::len)), (&o.registrar, Some(2)));
        assert_eq!(crate::commitments::overview(&v, date(2027, 1, 5)).registrar["refresh"], true);
        let fresh = vault("none");
        assert_eq!(crate::commitments::overview(&fresh, date(2026, 9, 1)).registrar, serde_json::Value::Null);
        let _ = std::fs::remove_dir_all(&v);
        let _ = std::fs::remove_dir_all(&fresh);
    }
}
```

  In `engine/src/cli.rs`'s `mod tests`, after R2's `asks_wait_for_the_registrars_term_to_start`:

```rust
    /// Phase-3 D5 end to end: a fetch confirms CS 100 and its lab (R24); the next fetch drops the
    /// lab and moves the lecture's room; the next `rank` files an end card for the lab (the day
    /// before the fetch, R2-c) and a change card for the room (R2-g). The notes were created by
    /// `quinn` (R3-c); the cards come because the p2 judge-once fix counts only a later `set`.
    #[test]
    fn a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank() {
        let vault = p16_vault("p3refetch");
        let ctx = WriteContext::new("quinn", "dashboard");
        let text = include_str!("../tests/fixtures/registrar/banner-ua-registration.json");
        let first = crate::registrar::run(&vault, text, "ua", Date::constant(2026, 9, 1), &ctx, &mut Journal::new(&vault)).unwrap();
        assert_eq!((first.confirmed, first.proposed), (2, 2), "CS 100 and its lab; ENGL 101 and ART 110 have no course here");
        let mut later: serde_json::Value = serde_json::from_str(text).unwrap();
        later["data"].as_array_mut().unwrap().retain(|r| r["courseReferenceNumber"] != "40002");
        later["data"][0]["meetingsFaculty"][0]["meetingTime"]["room"] = serde_json::json!("102");
        crate::registrar::run(&vault, &later.to_string(), "ua", P16_MONDAY, &ctx, &mut Journal::new(&vault)).unwrap();
        rank_p16(&vault, P16_MONDAY, Vec::new());
        let cards: Vec<String> = checks(&vault, "approvals")
            .iter()
            .map(|n| std::fs::read_to_string(vault.join("approvals").join(n)).unwrap())
            .collect();
        assert!(cards.iter().any(|t| t.contains("commitments/cs-100-lab.md") && t.contains("2026-09-06")), "{cards:#?}");
        assert!(cards.iter().any(|t| t.contains("commitments/cs-100.md") && t.contains("Invented Hall 102")), "{cards:#?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

  `engine/tests/commitments_registrar.rs` (new):

```rust
//! Phase 3 (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md` §3):
//! `knowlu-engine commitments --registrar`, run as the binary. Every course is invented.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FIXTURE: &str = include_str!("fixtures/registrar/banner-ua-registration.json");

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-registrar-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::create_dir_all(dir.join("courses")).unwrap();
    std::fs::write(dir.join("courses").join("cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
    dir
}

/// The input sits beside the vault, never inside it, so the tree below is the vault's.
fn run(vault: &Path, text: &str, extra: &[&str]) -> Output {
    let file = vault.with_extension("registrar.json");
    std::fs::write(&file, text).unwrap();
    let out = Command::new(binary())
        .args(["commitments", "--vault"])
        .arg(vault)
        .args(["--today", "2026-09-01", "--registrar"])
        .arg(&file)
        .args(extra)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&file);
    out
}

/// Every file under the vault, relative, sorted (`commitments_confirm.rs`'s helper).
fn tree(vault: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(vault, vault, &mut out);
    out.sort();
    out
}

#[test]
fn a_fetch_prints_the_report_and_writes_the_series_and_the_notes() {
    let v = scratch("ok");
    let out = run(&v, FIXTURE, &["--school", "ua"]);
    assert!(out.status.success(), "{out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["term"], "202640");
    assert_eq!((report["rows"].as_u64(), report["confirmed"].as_u64(), report["proposed"].as_u64()), (Some(4), Some(2), Some(2)));
    assert_eq!(report["dropped"], serde_json::json!({ "midnight": 1, "no_time": 1 }));
    let files = tree(&v);
    for f in ["state/calendar-series.json", "commitments/cs-100.md", "commitments/cs-100-lab.md"] {
        assert!(files.iter().any(|x| x == f), "{f} not written: {files:?}");
    }
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn every_exit_2_writes_nothing() {
    for (name, text, extra) in [
        ("school", FIXTURE, vec!["--school", "zz"]),
        ("html", "<!DOCTYPE html><html>Sign in</html>", vec!["--school", "ua"]),
        ("empty", "{\"data\": []}", vec!["--school", "ua"]),
        ("noschool", FIXTURE, vec![]),
        ("both", FIXTURE, vec!["--school", "ua", "--confirm", "x.json"]),
    ] {
        let v = scratch(name);
        let before = tree(&v);
        let out = run(&v, text, &extra);
        assert_eq!(out.status.code(), Some(2), "{name}: {out:?}");
        assert!(out.stdout.is_empty(), "{name}");
        assert_eq!(tree(&v), before, "{name} wrote something");
        let _ = std::fs::remove_dir_all(&v);
    }
}

/// Spec §3: `--registrar` makes no network call. The vault names a feed on a loopback listener
/// that accepts nothing; after the command, no connection is waiting.
#[test]
fn registrar_makes_no_network_call() {
    let v = scratch("nonet");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        v.join("config").join("ingest.yaml"),
        format!("timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: http://127.0.0.1:{port}/a.ics\n"),
    )
    .unwrap();
    std::fs::write(v.join("config").join("cloud.yaml"), format!("api_base: http://127.0.0.1:{port}/functions/v1\n")).unwrap();
    assert!(run(&v, FIXTURE, &["--school", "ua"]).status.success());
    match listener.accept() {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        other => panic!("--registrar opened a connection: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu-engine --lib -j 2 -- registrar::run_tests cli::tests::a_registrar_refetch`
  and `cargo test -p knowlu-engine --test commitments_registrar -j 2`. Expected: compile errors (no
  `run`, `Report`, `Overview.registrar`); the binary tests fail on the unknown `--registrar` flag
  (clap exits 2, which `every_exit_2_writes_nothing` passes, so it is not the red one; the other two
  are).

- [ ] **Step 3 — implement,** in these edits.

  **(a)** `engine/src/registrar.rs`, after `parse_banner` (the `use` block gains
  `use std::path::Path;`, `use crate::commitments::SeriesFile;` and `use serde_json::json;`):

```rust
/// What `commitments --registrar` did (spec §3's output).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub term: String,
    pub rows: usize,
    pub confirmed: usize,
    pub proposed: usize,
    pub no_time: usize,
    pub midnight: usize,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn to_json(&self) -> Value {
        json!({
            "term": self.term, "rows": self.rows, "confirmed": self.confirmed, "proposed": self.proposed,
            "dropped": { "no_time": self.no_time, "midnight": self.midnight },
            "warnings": self.warnings,
        })
    }
}

/// `commitments --registrar <file> --school <key>` (spec §3). Parses first: an unknown school, a
/// file that is not JSON and a parse with no usable row are `Err` (exit 2) before anything is
/// written. Then the term's calendar is merged under D5 ([`crate::commitments::refresh_series`]),
/// and every kept row that names a vault course and is a current proposal is confirmed at `hard`
/// through phase 2's `confirm`, under `ctx` (the student: `quinn` via `dashboard`; Plan rulings
/// R3-b, R3-c). No network.
pub fn run(
    vault: &Path,
    text: &str,
    school_key: &str,
    today: Date,
    ctx: &crate::write::WriteContext,
    journal: &mut crate::journal::Journal,
) -> Result<Report, String> {
    use crate::commitments as cm;
    if school(school_key).is_none() {
        return Err(format!("--school {school_key:?} is not a school Knowlu reads"));
    }
    let json: Value = serde_json::from_str(text).map_err(|e| format!("--registrar is not JSON ({e})"))?;
    let parsed = parse_banner(&json, school_key)?;
    let calendar = calendar_key(school_key, &parsed.term);
    let (_, mut warnings) = cm::refresh_series(vault, &[(calendar, parsed.series.clone())], today);
    warnings.extend(parsed.warnings.iter().cloned());
    let stored = cm::stored_proposals(vault, today);
    let current: BTreeSet<&str> = stored.proposals.iter().map(|p| p.source_uid.as_str()).collect();
    let (mut mine, mut proposed) = (Vec::new(), 0);
    for s in &parsed.series {
        match cm::registrar_course(s, &stored.codes) {
            Some(_) if current.contains(s.source_uid.as_str()) => mine.push((s.source_uid.clone(), "hard".to_string())),
            Some(_) => {}
            None => proposed += 1,
        }
    }
    let input = cm::ConfirmInput { mine, not_mine: Vec::new(), window: None };
    let done = cm::confirm(vault, &input, today, ctx, journal)?;
    warnings.extend(done.warnings);
    Ok(Report {
        term: parsed.term,
        rows: parsed.series.len(),
        confirmed: done.created,
        proposed,
        no_time: parsed.no_time,
        midnight: parsed.midnight,
        warnings,
    })
}

/// The *Schedule* view's registrar line (Plan ruling R3-d): the school and the terms the series
/// file holds, the term [`term_for`] names today, and whether that one is missing. `null` when the
/// file holds no registrar calendar.
pub fn status(file: &SeriesFile, today: Date) -> Value {
    let mut school_key: Option<String> = None;
    let mut held: BTreeSet<String> = BTreeSet::new();
    for calendar in file.calendars.keys() {
        let Some((s, term)) = calendar.strip_prefix(CALENDAR_PREFIX).and_then(|r| r.split_once(':')) else { continue };
        school_key.get_or_insert_with(|| s.to_string());
        held.insert(term.to_string());
    }
    let Some(school_key) = school_key else { return Value::Null };
    let current = term_for(&school_key, today);
    let refresh = current.as_ref().is_some_and(|t| !held.contains(t));
    json!({ "school": school_key, "held": held, "current": current, "refresh": refresh })
}
```

  **(b)** `commitments.rs`: `Overview` (4566) gains, before `setup`,

```rust
    /// Phase 3 (Plan ruling R3-d): [`crate::registrar::status`], or `null`.
    pub registrar: serde_json::Value,
    /// The current registrar proposals (key `registrar:`), as [`proposal_value`].
    pub registrar_proposals: Vec<serde_json::Value>,
```

  `to_json` gains `"registrar": self.registrar, "registrar_proposals": self.registrar_proposals,`;
  `overview` builds them before `Overview { … }`:

```rust
    let registrar = crate::registrar::status(&stored.file, today);
    let registrar_proposals = stored
        .proposals
        .iter()
        .filter(|p| p.source_uid.starts_with(crate::registrar::CALENDAR_PREFIX))
        .map(proposal_value)
        .collect();
```

  and the struct literal names both.

  **(c)** `cli.rs`, after `commitments_confirm`:

```rust
/// Phase-3 spec §3: `commitments --registrar`. Pins `today` (the vault's zone when none is given)
/// and runs `registrar::run`. It fetches nothing. `Err` is the exit-2 message.
pub fn commitments_registrar(
    vault: &Path,
    today_iso: Option<&str>,
    text: &str,
    school: &str,
    ctx: &WriteContext,
) -> Result<crate::registrar::Report, String> {
    let today = match today_iso {
        Some(iso) => Date::strptime("%Y-%m-%d", iso).map_err(|_| format!("bad --today {iso:?}"))?,
        None => Timestamp::now().to_zoned(vault_zone(vault)).date(),
    };
    let mut journal = Journal::new(vault);
    crate::registrar::run(vault, text, school, today, ctx, &mut journal)
}
```

  **(d)** `main.rs`: in `Command::Commitments`, after `confirm`:

```rust
        /// Phase 3: the registrar's fetched JSON (the app's temp file). Fetches nothing; writes
        /// the series and the matched rows' notes; exit 2 on an unknown school or no usable row.
        #[arg(long, conflicts_with = "confirm", requires = "school")]
        registrar: Option<PathBuf>,
        /// The registrar's school key (`registrar::SCHOOLS`), with `--registrar`.
        #[arg(long)]
        school: Option<String>,
```

  The doc comment above `Commitments` gains: "With `--registrar <file> --school <key>` (phase 3) it
  fetches nothing either: it merges the registrar's rows and confirms the ones that match a
  course, prints `{confirmed, dropped, proposed, rows, term, warnings}`, and exits 2 on an unknown
  school or no usable row, having written nothing." The arm's pattern gains `registrar, school`,
  and before `if let Some(path) = confirm {`:

```rust
            if let Some(path) = registrar {
                let text = match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("knowlu-engine: --registrar {}: {err}", path.display());
                        return ExitCode::from(2);
                    }
                };
                let ctx = write::WriteContext::new(&actor, &via);
                let school = school.as_deref().unwrap_or_default();
                return match cli::commitments_registrar(&vault, today.as_deref(), &text, school, &ctx) {
                    Ok(report) => {
                        println!("{}", knowlu_engine::ledger::dumps_value(&report.to_json()));
                        ExitCode::SUCCESS
                    }
                    Err(err) => {
                        eprintln!("knowlu-engine: {err}");
                        ExitCode::from(2)
                    }
                };
            }
```

- [ ] **Step 4 — run.** `cargo test -p knowlu-engine --lib -j 2 -- registrar:: cli::tests::a_registrar_refetch`
  (all pass; if `a_registrar_refetch…` files neither card, the p2 judge-once fix is missing from
  the branch: stop and report it, do not change the actor or the assertion), `cargo test -p knowlu-engine --test commitments_registrar --test commitments_confirm -j 2`,
  `cargo test -p knowlu-engine -j 2` (all pass, `cloud_contract.rs`'s
  `rank_cannot_reach_a_judgment_endpoint` among them), then the fixture byte check of R1 Step 4.

- [ ] **Step 5 — commit.** `git add engine/src/registrar.rs engine/src/commitments.rs engine/src/cli.rs engine/src/main.rs engine/tests/commitments_registrar.rs`; message:

```
feat(engine): commitments --registrar confirms the matched classes (phase 3, §3, R24)

The command parses the app's temp file, merges the term's series under D5, and
confirms each row whose course is in the vault through phase 2's confirm, as the
student (quinn via dashboard, spec §3). It prints
{confirmed, dropped, proposed, rows, term, warnings}, exits 2 having written
nothing on an unknown school or no usable row, and makes no network call.
The overview names the registrar term and its proposals.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

---

## R4 — the app: the curated entry and three commands (spec D1, D2, D4, §2)

**Files.**
- `app/src/scaffold.rs`: new `Call` and `Registrar` above `Curated` (line 26); `Curated` gains
  `registrar: Option<Registrar>`; `CAMPUSES` (lines 40–55) gives UA its entry and UK `None`.
- `app/src/lms_link.rs`: `sweep_stale_sessions` (line 150) and `session_agent` (line 384) become
  `pub`. Nothing else in the file changes.
- `app/src/registrar.rs` (new).
- `app/src/lib.rs`: `pub mod registrar;` after `pub mod profiles;` (line 11).
- `app/src/week.rs`: `your_week_inner` (lines 29–33) gains `registrar_label`.
- `app/src/main.rs`: the `use knowlu::{…}` line (line 3) gains `registrar`; `run_console`
  (lines 125–190): `LmsSession` in `setup`, the window-event handler (line 185), the exit wipe (line 187), and the console
  `generate_handler!` list (line 186) gains the three commands (47 → 50).
- `app/tests/registrar.rs` (new); `app/tests/week.rs` (one test).

**Interfaces.**
- `scaffold::Call { method: &'static str, path: &'static str, form: Option<&'static str> }` and
  `scaffold::Registrar { school, label, host, prefix, start: &'static str, calls: &'static [Call] }`
  (all `Copy`); `Curated.registrar: Option<Registrar>`.
- In `registrar` (app), all `pub`:
  - `fn school_of(vault: &Path) -> Option<&'static Registrar>`
  - `fn start_url(reg: &Registrar) -> String`
  - `fn call_urls(reg: &Registrar, term: &str) -> Vec<(String, String, Option<String>)>` (method,
    URL, form body)
  - `fn registrar_argv(vault: &Path, file: &Path, school: &str, today: jiff::civil::Date) -> Vec<String>`
    → `["commitments", "--vault", <v>, "--today", <d>, "--registrar", <file>, "--school", <s>, "--via", "dashboard"]`
  - `fn run_file(cs: &ConsoleState, view: &str, school: &str, bytes: &[u8], today: Date) -> Value`
    → `{ok, error, closed: true, result, state}`
  - Tauri: `open_registrar_window(app, cs)` → `{ok, error, opened}`;
    `capture_registrar(app, cs, sch, view)` → `{ok, error, closed, result, state}`;
    `close_registrar_window(app)` → `{ok, error}`. Every argument is one word.
- `week::your_week_inner` → `{ok, error, week, registrar_label: "myBama" | null}`.
- Consumes `lms_link::{WINDOW, LmsSession, open_window_at, session_dir, cookie_url, session_agent,
  close_and_wipe, wipe_session, wipe_session_on_exit, sweep_stale_sessions}` (`lms_link.rs:14`,
  `73`, `163`, `40`, `365`, `384`, `207`, `90`, `105`, `150`), `knowlu_engine::registrar::{term_for,
  looks_signed_out}`, `commands::{now_in, state_inner, attach_scheduler}`, `scheduler::engine_exe`,
  `ConsoleState::{vault, data_dir, vault_io, note_write}`.

**Plan ruling R4-a: the registrar uses `lms_link`'s sign-in window as it is** — the same label
(`lms-signin`), `session_dir()`, `open_window_at`, `LmsSession`, `close_and_wipe` and the sweep. One
sign-in window at a time for either purpose. *Why:* D1 says "copies `lms_link.rs`"; reusing it keeps
one incognito, capability-less window whose every guarantee is already pinned by
`app/tests/lms_link.rs` (`no_capability_names_the_sign_in_window`,
`the_sign_in_window_keeps_nothing_and_lives_nowhere_near_the_app_data`), rather than a second copy
of each.
**Plan ruling R4-b: `Registrar` is data** — `school` (the engine's `--school`), `label` (what the
button says: "myBama"), `host` (`bannerssb.ua.edu`), `prefix` (`/StudentRegistrationSsb/ssb/`),
`start` (the sign-in start path under the prefix, `registration`), and `calls` in order, the rows
call last, with `{term}` substituted. **UA's `calls` are copied verbatim from R0's D3 amendment.**
The block below shows the shape with the one call the spec already names (the term selection) and
the rows call as R0 records it; the test pins only structure: every URL on the school's own host
under the prefix, the rows call last and a `GET`. *Why:* spec D2 and D3; the paths are R0's facts,
not the plan's guesses.
**Plan ruling R4-c:** `school_of` reads `config/campus.yaml`'s `unitid` as a string (what
`campus_config_yaml` writes: `unitid: '100751'`) or a number (a hand edit), then
`scaffold::curated(unitid)?.registrar`. *Why:* the vault, not the profile, knows the school.
**Plan ruling R4-d:** the capture keeps the window open when Banner answers with its sign-in page
("You're not signed in yet — finish signing in to myBama in the window, then press I'm signed in")
or a request fails, so the student can finish and press again. Once the bytes reach the engine,
`close_and_wipe` runs whatever the engine said. The envelope's `closed` tells the page which.
*Why:* spec §2 step 2 ("You're not signed in yet") and step 3 (close whatever the outcome) read
together; closing on a half-finished Okta sign-in would throw away the session the student is
still building.
**Plan ruling R4-e:** the temp file is `<profile>\tmp\registrar-<pid>-<stamp>.json`, written and
run under `cs.vault_io`, and removed after the child exits, whatever it printed. *Why:* spec §4;
the same shape as phase 2's `commitments_confirm_inner` (Plan ruling Q9-a), because the child
writes notes.
**Plan ruling R4-f:** `run_console` manages `lms_link::LmsSession`, wipes the session directory on
the sign-in window's `Destroyed` event, hides **only `main`** on `CloseRequested`, and calls
`wipe_session_on_exit` at `RunEvent::Exit`. *Why:* today the console's builder manages no
`LmsSession` (so the close button would leave the directory for the next launch's sweep) and its
`on_window_event` hides every window that asks to close, so the student could never close the
sign-in window. (Spec problem 7.)
**Plan ruling R4-g:** `your_week`'s envelope gains `registrar_label`: the curated `label` when
`school_of` finds one, else `null`. *Why:* spec D2's "no entry, no button" needs the page to know,
and `your_week` is what both the confirm screen and *Schedule* already call.
**Plan ruling R4-h:** each request carries `cookie`, `accept: application/json` and
`x-requested-with: XMLHttpRequest`; a `POST` also carries
`content-type: application/x-www-form-urlencoded`. R0's question 5 amends this if Banner needs
more. *Why:* spec §2 ("the same headers the LMS capture sends") plus the header Banner's own page
sends on every XHR.

- [ ] **Step 1 — failing tests.** `app/tests/registrar.rs` (new), in three edits:

```rust
//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`
//! §2, D1, D4): `registrar.rs`'s pure half, its engine run, and the window promises it inherits
//! from `lms_link.rs`, read from the source where a live WebView2 would be needed. The engine run
//! spawns the real sibling engine: build it first with `cargo build -p knowlu-engine -j 2`, because
//! `app/build.rs` leaves a zero-byte placeholder at `target/debug/knowlu-engine.exe`.
use std::path::{Path, PathBuf};
use knowlu::registrar::{call_urls, registrar_argv, run_file, school_of, start_url};
use knowlu::state::ConsoleState;

const FIXTURE: &str = include_str!("../../engine/tests/fixtures/registrar/banner-ua-registration.json");

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qo-registrar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "courses"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    std::fs::write(dir.join("config/ingest.yaml"), "timezone: America/Chicago\n").unwrap();
    std::fs::write(dir.join("config/campus.yaml"), "unitid: '100751'\nname: 'Invented'\nstate: 'AL'\nlms: 'blackboard'\ncurated: true\n").unwrap();
    std::fs::write(dir.join("courses/cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
    dir
}

fn open(v: &Path, name: &str) -> ConsoleState {
    ConsoleState::open(v.to_path_buf(), std::env::temp_dir().join(format!("qo-registrar-data-{name}-{}", std::process::id())))
}

/// `KNOWLU_ENGINE_EXE` is process-wide; this file's own lock, `week.rs`'s shape.
static ENGINE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn real_engine() -> std::sync::MutexGuard<'static, ()> {
    let exe = Path::new("../target/debug/knowlu-engine.exe");
    let len = std::fs::metadata(exe).map(|m| m.len()).unwrap_or(0);
    assert!(len > 0, "{}: run `cargo build -p knowlu-engine -j 2` first (build.rs leaves a zero-byte placeholder)", exe.display());
    let guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", exe.canonicalize().unwrap()) };
    guard
}

#[test]
fn ua_has_a_registrar_and_every_call_is_on_its_own_registrar_host() {
    let v = scratch("host");
    let reg = school_of(&v).expect("UA's row has a registrar entry");
    assert_eq!((reg.school, reg.label), ("ua", "myBama"));
    assert_eq!(start_url(reg), "https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/registration");
    let calls = call_urls(reg, "202640");
    assert!(!calls.is_empty() && calls.len() <= 3, "{calls:?}");
    for (method, url, form) in &calls {
        assert!(url.starts_with("https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/"), "{url}");
        assert!(!url.contains("{term}") && !form.as_deref().unwrap_or("").contains("{term}"), "{url} {form:?}");
        assert!(method == "GET" || method == "POST", "{method}");
    }
    assert_eq!(calls.last().unwrap().0, "GET", "the rows call is last and a GET");
    for c in knowlu::scaffold::CAMPUSES {
        if c.unitid != "100751" {
            assert!(c.registrar.is_none(), "{} has no registrar before the pilot", c.label);
        }
    }
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_school_is_read_from_campus_yaml_as_text_or_number_and_absent_is_none() {
    let v = scratch("school");
    std::fs::write(v.join("config/campus.yaml"), "unitid: 100751\n").unwrap();
    assert!(school_of(&v).is_some(), "a number is read too");
    std::fs::write(v.join("config/campus.yaml"), "unitid: '157085'\n").unwrap();
    assert!(school_of(&v).is_none(), "a curated school without a registrar");
    std::fs::remove_file(v.join("config/campus.yaml")).unwrap();
    assert!(school_of(&v).is_none(), "no campus.yaml, no button");
    let _ = std::fs::remove_dir_all(&v);
}

/// D1: the cookies go only to the registrar host. On Okta, on Okta Verify's page or anywhere else
/// the window's own URL is not used; the call's URL is.
#[test]
fn the_cookies_are_read_for_the_registrar_host_and_never_for_the_sign_in_provider() {
    use knowlu::lms_link::cookie_url;
    let rows = "https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/classRegistration/x";
    let here = "https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/registration/registration";
    assert_eq!(cookie_url(Some(here), rows), here);
    for elsewhere in ["https://login.example.invalid/oauth2/v1/authorize", "https://bannerssb.ua.edu.example.invalid/x", "about:blank"] {
        assert_eq!(cookie_url(Some(elsewhere), rows), rows, "{elsewhere}");
    }
}

#[test]
fn the_argv_carries_the_vault_the_file_the_school_today_and_the_consoles_via() {
    assert_eq!(
        registrar_argv(Path::new(r"C:\v"), Path::new(r"C:\d\tmp\r.json"), "ua", "2026-09-26".parse().unwrap()),
        ["commitments", "--vault", r"C:\v", "--today", "2026-09-26", "--registrar", r"C:\d\tmp\r.json", "--school", "ua", "--via", "dashboard"]
    );
}
```

```rust
/// D4 + R4-e: the bytes go to a temp file under the profile's `tmp\`, the engine writes the
/// notes as `quinn` via `dashboard`, and the file is gone afterwards.
#[test]
fn a_fetched_schedule_runs_through_the_engine_and_the_temp_file_is_gone() {
    let _engine = real_engine();
    let v = scratch("run");
    let cs = open(&v, "run");
    cs.set_test_today(Some("2026-09-01".parse().unwrap()));
    let env = run_file(&cs, "today", "ua", FIXTURE.as_bytes(), "2026-09-01".parse().unwrap());
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["closed"], true);
    assert_eq!((env["result"]["confirmed"].as_u64(), env["result"]["term"].as_str()), (Some(2), Some("202640")));
    assert_eq!(env["state"]["schema"], 1);
    assert!(v.join("commitments/cs-100-lab.md").is_file());
    let tmp = cs.data_dir.join("tmp");
    assert_eq!(std::fs::read_dir(&tmp).map(|d| d.count()).unwrap_or(0), 0, "the temp file is deleted");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_parse_failure_is_named_writes_nothing_and_the_temp_file_is_gone_too() {
    let _engine = real_engine();
    let v = scratch("fail");
    let cs = open(&v, "fail");
    let env = run_file(&cs, "today", "ua", b"{\"data\": []}", "2026-09-01".parse().unwrap());
    assert_eq!(env["ok"], false, "{env}");
    assert!(env["error"].as_str().unwrap().contains("no class with meeting times"), "{env}");
    assert!(!v.join("commitments").exists() && !v.join("state/calendar-series.json").exists());
    assert_eq!(std::fs::read_dir(cs.data_dir.join("tmp")).map(|d| d.count()).unwrap_or(0), 0);
    let _ = std::fs::remove_dir_all(&v);
}

/// D1 and constraint 9, read from the source: the registrar opens `lms_link`'s window and no
/// other, keeps no credential, and closes and wipes after the engine run.
#[test]
fn the_registrar_uses_the_sign_in_window_and_keeps_nothing() {
    let src = std::fs::read_to_string("src/registrar.rs").expect("src/registrar.rs");
    for needle in ["lms_link::open_window_at", "lms_link::session_dir()", "lms_link::close_and_wipe", "lms_link::cookie_url", "looks_signed_out", "remove_file(&file)"] {
        assert!(src.contains(needle), "registrar.rs must use {needle}");
    }
    for banned in ["WebviewWindowBuilder", "password", "credentials::", "app_data_root", "set_password", "eprintln!", "log::"] {
        assert!(!src.contains(banned), "registrar.rs must not contain {banned}");
    }
    assert!(!src.contains("\"bytes\""), "the fetched bytes never go back to the page");
}

/// R4-f: the console's builder wipes a sign-in session as the wizard's does, and hides only its
/// own window on close, so the sign-in window can be closed.
#[test]
fn the_console_builder_wipes_the_sign_in_session_and_hides_only_main() {
    let src = std::fs::read_to_string("src/main.rs").expect("src/main.rs");
    let console = src.split("fn run_console").nth(1).expect("run_console");
    assert!(console.contains("app.manage(lms_link::LmsSession::default())"), "the console manages LmsSession");
    assert!(console.contains("lms_link::wipe_session(w.app_handle())"), "…wipes on Destroyed");
    assert!(console.contains("lms_link::wipe_session_on_exit(app)"), "…and at exit");
    assert!(console.contains("w.label() == \"main\""), "only main hides on close");
    for cmd in ["registrar::open_registrar_window", "registrar::capture_registrar", "registrar::close_registrar_window"] {
        assert!(console.contains(cmd), "{cmd} is registered on the console window");
    }
    let shell = src.split("fn run_shell").nth(1).and_then(|s| s.split("fn run_console").next()).expect("run_shell");
    assert!(!shell.contains("registrar::"), "the wizard has no registrar command (spec D6)");
}
```

  In `app/tests/week.rs`, after `your_week_reports_setup_on_the_vaults_first_day_only_and_writes_nothing`:

```rust
/// Phase 3, R4-g: the page learns whether this school has a registrar button from `your_week`.
#[test]
fn your_week_names_the_registrar_only_at_a_school_that_has_one() {
    let v = scratch("reglabel");
    let cs = open(&v, "reglabel");
    assert_eq!(your_week_inner(&cs)["registrar_label"], serde_json::Value::Null, "vault-full has no campus.yaml");
    std::fs::write(v.join("config/campus.yaml"), "unitid: '100751'\n").unwrap();
    assert_eq!(your_week_inner(&cs)["registrar_label"], "myBama");
    std::fs::write(v.join("config/campus.yaml"), "unitid: '157085'\n").unwrap();
    assert_eq!(your_week_inner(&cs)["registrar_label"], serde_json::Value::Null);
    let _ = std::fs::remove_dir_all(&v);
}
```

  The existing `no_capability_names_the_sign_in_window` (`app/tests/lms_link.rs:298`) is the
  capability pin for the registrar too (R4-a): it is run in Step 4, not copied.

- [ ] **Step 2 — run and see them fail.** `cargo build -p knowlu-engine -j 2`, then
  `cargo test -p knowlu --test registrar --test week -j 2`. Expected: compile errors (no
  `knowlu::registrar`, no `Curated.registrar`).

- [ ] **Step 3 — implement,** in these edits.

  **(a)** `app/src/scaffold.rs`, above `pub struct Curated`:

```rust
/// One request the registrar capture makes (phase 3, Plan ruling R4-b): `path` is under the
/// registrar's `prefix`; `{term}` in `path` or `form` becomes the term code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Call {
    pub method: &'static str,
    pub path: &'static str,
    pub form: Option<&'static str>,
}

/// A curated school's registrar (phase-3 spec D2). **Data, recorded by the R0 spike** — adding a
/// school is adding one of these to its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registrar {
    /// The engine's `--school` key (`knowlu_engine::registrar::SCHOOLS`).
    pub school: &'static str,
    /// What the student calls it; the button reads "Get my class times from <label>".
    pub label: &'static str,
    pub host: &'static str,
    pub prefix: &'static str,
    /// Where the sign-in window opens, under `prefix`.
    pub start: &'static str,
    /// In order; the last one's body is the schedule.
    pub calls: &'static [Call],
}
```

  `Curated` gains, after `lms_kind`:
  `/// Phase 3: the registrar Knowlu can read, or `None` (every school but UA before the pilot).`
  `pub registrar: Option<Registrar>,`. UK's row gains `registrar: None,`. UA's row gains the entry
  below, whose two `calls` are **R0's D3 amendment, copied verbatim** (Plan ruling R4-b): the first
  is the term selection the spec names; `UA_ROWS_PATH` is the one line of the amendment headed
  "rows call". If R0 recorded one call, the list has one; if three, three.

```rust
        registrar: Some(Registrar {
            school: "ua",
            label: "myBama",
            host: "bannerssb.ua.edu",
            prefix: "/StudentRegistrationSsb/ssb/",
            start: "registration",
            calls: &[
                Call { method: "POST", path: "term/search?mode=registration", form: Some("term={term}") },
                Call { method: "GET", path: UA_ROWS_PATH, form: None },
            ],
        }),
```

  with, above `CAMPUSES`: `/// R0's rows call for UA (spec D3 amendment), under the prefix.` and
  `const UA_ROWS_PATH: &str = "…";` holding that line's path exactly.

  **(b)** `app/src/lms_link.rs`: `fn sweep_stale_sessions()` (line 150) → `pub fn`, its doc gaining
  "Also called by `registrar::open_registrar_window`."; `fn session_agent()` (line 384) → `pub fn`,
  its doc's "both captures" → "every capture (the calendar link, the course list, and phase 3's
  registrar)".

  **(c)** `app/src/registrar.rs` (new), header and pure half:

```rust
//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`
//! D1, D2, D4, §2): the school's registrar, read through the school's own sign-in window.
//!
//! **Knowlu never asks for a campus credential here either.** The window is `lms_link`'s own:
//! incognito, a throwaway data directory, no capability grant (Plan ruling R4-a). Its cookies are
//! read for the registrar host only and live on `capture`'s stack; the fetched bytes go to one
//! temp file that the engine reads and this module deletes. Nothing here parses them.
use std::path::Path;
use jiff::civil::Date;
use serde_json::{json, Value};
use tauri::{Manager, State};
use knowlu_engine::childproc::NoConsole;
use crate::lms_link;
use crate::scaffold::Registrar;
use crate::scheduler::Scheduler;
use crate::state::ConsoleState;

const NO_REGISTRAR: &str = "Knowlu can't read your school's class schedule yet";

/// The vault's school, from `config/campus.yaml`'s `unitid` (Plan ruling R4-c), and its registrar.
pub fn school_of(vault: &Path) -> Option<&'static Registrar> {
    let text = std::fs::read_to_string(vault.join("config").join("campus.yaml")).ok()?;
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).ok()?;
    let unitid = match doc.get("unitid")? {
        serde_yaml_ng::Value::String(s) => s.trim().to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        _ => return None,
    };
    crate::scaffold::curated(&unitid)?.registrar.as_ref()
}

pub fn start_url(reg: &Registrar) -> String {
    format!("https://{}{}{}", reg.host, reg.prefix, reg.start)
}

/// `(method, url, form)` per call, `{term}` filled in.
pub fn call_urls(reg: &Registrar, term: &str) -> Vec<(String, String, Option<String>)> {
    reg.calls
        .iter()
        .map(|c| {
            let url = format!("https://{}{}{}", reg.host, reg.prefix, c.path.replace("{term}", term));
            (c.method.to_string(), url, c.form.map(|f| f.replace("{term}", term)))
        })
        .collect()
}

/// `--via dashboard`, and `--actor` keeps the engine's default, `quinn`: the R24 notes are the
/// student's (spec §3, Plan ruling R3-c), as `week::confirm_argv`'s are.
pub fn registrar_argv(vault: &Path, file: &Path, school: &str, today: Date) -> Vec<String> {
    vec![
        "commitments".into(), "--vault".into(), vault.to_string_lossy().into_owned(),
        "--today".into(), today.to_string(),
        "--registrar".into(), file.to_string_lossy().into_owned(),
        "--school".into(), school.into(),
        "--via".into(), "dashboard".into(),
    ]
}
```

  **(d)** `app/src/registrar.rs`, the engine run (Plan ruling R4-e):

```rust
/// Writes the fetched bytes to `<profile>\tmp\`, runs `commitments --registrar` under `vault_io`,
/// deletes the file whatever happened, and rebuilds the state for `view`. `closed` is always true
/// here: the capture closes the window after this returns (Plan ruling R4-d).
pub fn run_file(cs: &ConsoleState, view: &str, school: &str, bytes: &[u8], today: Date) -> Value {
    let dir = cs.data_dir.join("tmp");
    let stamp = knowlu_engine::journal::now_ts(None).replace([':', '.', '-'], "");
    let file = dir.join(format!("registrar-{}-{stamp}.json", std::process::id()));
    let ran = spawn(cs, &dir, &file, school, bytes, today);
    let _ = std::fs::remove_file(&file);
    let (ok, error, result) = match ran {
        Err(e) => (false, json!(e), Value::Null),
        Ok(out) if out.status.success() => {
            cs.note_write();
            match serde_json::from_slice::<Value>(&out.stdout) {
                Ok(v) => (true, Value::Null, v),
                Err(e) => (false, json!(format!("the engine printed no report ({e})")), Value::Null),
            }
        }
        Ok(out) => {
            let why = String::from_utf8_lossy(&out.stderr).trim().trim_start_matches("knowlu-engine: ").to_string();
            (false, json!(why), Value::Null)
        }
    };
    let state = crate::commands::state_inner(cs, view).map(|env| env["state"].clone()).unwrap_or(Value::Null);
    json!({ "ok": ok, "error": error, "closed": true, "result": result, "state": state })
}

fn spawn(cs: &ConsoleState, dir: &Path, file: &Path, school: &str, bytes: &[u8], today: Date) -> Result<std::process::Output, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(file, bytes).map_err(|e| e.to_string())?;
    let exe = crate::scheduler::engine_exe()?;
    let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
    std::process::Command::new(exe).no_console().args(registrar_argv(&cs.vault, file, school, today)).output().map_err(|e| e.to_string())
}
```

  **(e)** `app/src/registrar.rs`, the capture and the three commands:

```rust
fn failed(error: String, closed: bool) -> Value {
    json!({ "ok": false, "error": error, "closed": closed, "result": Value::Null, "state": Value::Null })
}

/// Spec §2 step 2: the recorded calls, with the window's cookies for each call's host only
/// (`cookie_url`). The cookies live on this stack: never written, logged or put in an error.
fn capture(app: &tauri::AppHandle, cs: &ConsoleState, view: &str) -> Value {
    // The envelope says `closed`, so the window really is closed (plan review M1).
    let Some(reg) = school_of(&cs.vault) else { lms_link::close_and_wipe(app); return failed(NO_REGISTRAR.into(), true) };
    let Some(w) = app.get_webview_window(lms_link::WINDOW) else {
        return failed(format!("the sign-in window is not open; press Get my class times from {} again", reg.label), true);
    };
    let today = crate::commands::now_in(cs).date();
    let Some(term) = knowlu_engine::registrar::term_for(reg.school, today) else { return failed(NO_REGISTRAR.into(), false) };
    let here = w.url().ok();
    let agent = lms_link::session_agent();
    let mut body = String::new();
    for (method, url, form) in call_urls(reg, &term) {
        let Ok(jar_url) = lms_link::cookie_url(here.as_ref().map(|u| u.as_str()), &url).parse::<tauri::Url>() else {
            return failed(format!("{} could not be read (a bad address)", reg.label), false);
        };
        let jar: String = match w.cookies_for_url(jar_url) {
            Ok(cookies) => cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; "),
            Err(e) => return failed(format!("the sign-in could not be read ({e})"), false),
        };
        let sent = match method.as_str() {
            "POST" => agent.post(&url).header("cookie", &jar).header("accept", "application/json")
                .header("x-requested-with", "XMLHttpRequest").header("content-type", "application/x-www-form-urlencoded")
                .send(form.unwrap_or_default()),
            _ => agent.get(&url).header("cookie", &jar).header("accept", "application/json")
                .header("x-requested-with", "XMLHttpRequest").call(),
        };
        match sent.and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string()) {
            Ok(text) => body = text,
            Err(e) => return failed(format!("{} could not be read ({e})", reg.label), false),
        }
    }
    if knowlu_engine::registrar::looks_signed_out(&body) {
        return failed(format!("You're not signed in yet. Finish signing in to {} in the window, then press I'm signed in.", reg.label), false);
    }
    let env = run_file(cs, view, reg.school, body.as_bytes(), today);
    lms_link::close_and_wipe(app);
    env
}

#[tauri::command(async)]
pub fn open_registrar_window(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    lms_link::sweep_stale_sessions();
    let Some(reg) = school_of(&cs.vault) else { return json!({ "ok": false, "error": NO_REGISTRAR, "opened": false }) };
    match lms_link::open_window_at(&app, &start_url(reg), &lms_link::session_dir()) {
        Ok(()) => json!({ "ok": true, "error": Value::Null, "opened": true }),
        Err(e) => json!({ "ok": false, "error": e, "opened": false }),
    }
}

#[tauri::command(async)]
pub fn capture_registrar(app: tauri::AppHandle, cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String) -> Value {
    let mut env = capture(&app, &cs, &view);
    let _ = crate::commands::attach_scheduler(&mut env, &sch);
    env
}

#[tauri::command(async)]
pub fn close_registrar_window(app: tauri::AppHandle) -> Value {
    lms_link::close_and_wipe(&app);
    json!({ "ok": true, "error": Value::Null })
}
```

  Watch: no error string above formats `jar` or `cookies`; a later edit must keep it so
  (constraint 9).

  **(f)** `app/src/lib.rs`: `pub mod registrar;` after `pub mod profiles;`.

  **(g)** `app/src/week.rs` `your_week_inner`: the `json!` becomes
  `json!({ "ok": true, "error": Value::Null, "week": …, "registrar_label": crate::registrar::school_of(&cs.vault).map(|r| r.label) })`
  (Plan ruling R4-g); the poisoned-lock arm gains `"registrar_label": Value::Null`.

  **(h)** `app/src/main.rs`, `run_console` (Plan ruling R4-f): line 3's `use knowlu::{…}` gains
  `registrar`; in `.setup`, after `app.manage(knowlu::updates::Updates::default());`, add
  `app.manage(lms_link::LmsSession::default());` with the comment "Phase 3: the registrar's sign-in
  window is `lms_link`'s, so the console holds its session directory as the wizard does."; line
  185 becomes

```rust
        // Closing the console hides it to the tray (the scheduler keeps running). The sign-in
        // window is a campus session: it closes for real, and its directory is wiped on
        // `Destroyed` (phase 3, R4-f).
        .on_window_event(|w, e| {
            if w.label() == lms_link::WINDOW {
                if matches!(e, tauri::WindowEvent::Destroyed) { lms_link::wipe_session(w.app_handle()); }
            } else if w.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = e { api.prevent_close(); let _ = w.hide(); }
            }
        })
```

  the console `generate_handler!` list (line 186) gains
  `registrar::open_registrar_window, registrar::capture_registrar, registrar::close_registrar_window`
  after `week::preview_window`; and line 187's `.run(tauri::generate_context!())` plus its
  `.expect(…)` become `.build(tauri::generate_context!()).expect("Knowlu: failed to start the Tauri runtime")`
  and `.run(|app, event| { if matches!(event, tauri::RunEvent::Exit) { lms_link::wipe_session_on_exit(app); } });`,
  as `run_shell` does at lines 108–112.

- [ ] **Step 4 — run.** `cargo build -p knowlu-engine -j 2`; then
  `cargo test -p knowlu --test registrar --test week --test lms_link -j 2` (all pass, among them
  `no_capability_names_the_sign_in_window` and
  `every_curated_endpoint_is_built_from_that_schools_own_host`); then `cargo test -p knowlu -j 2`.
  The recount script of R6 Step 1 prints `50 29 69`.

- [ ] **Step 5 — commit.** `git add app/src/scaffold.rs app/src/lms_link.rs app/src/registrar.rs app/src/lib.rs app/src/week.rs app/src/main.rs app/tests/registrar.rs app/tests/week.rs`; message:

```
feat(app): read the registrar through the school's sign-in window (phase 3, D1, D2, D4)

UA's curated row gains its registrar (host, prefix, start and R0's calls).
registrar.rs opens lms_link's incognito, capability-less window, reads its
cookies for the registrar host only, makes the recorded calls, and hands the
bytes to commitments --registrar through a temp file it deletes; the window is
closed and wiped once the engine has run. The console builder now holds the
sign-in session and hides only its own window. Console commands 47 -> 50.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

---

## R5 — the console: two buttons, the "from myBama" marker, the refresh label (spec D6, D8, §2)

**Files.**
- `app/static/index.html`: `#ws-classes` (line 94) gains the registrar controls; `#main-schedule`
  (lines 71–79) gains a first section, `#sched-reg`.
- `app/static/console.js`: a new block after `bindScheduleView` (it ends at line 749);
  `renderScheduleView` (666–691), `checkWeekSetup` (756–761), `setupRow` (765–772) and
  `paintSetupRows` (774–786) each gain a line or two; the bind calls at line 1494 gain two.
- `app/static/console.css`: two rules after `.wsrow` (line 584).
- `app/tests/static_assets.rs`: one test at the end.
- `scripts/wizard-check.py`: `WEEK_FAKE` (lines 154–181) answers the three commands, a new
  `check_registrar`, and `main` runs it once.

**Interfaces.**
- Consumes R4's commands and envelopes: `open_registrar_window` → `{ok, error, opened}`;
  `capture_registrar({view})` → `{ok, error, closed, result, state}`; `close_registrar_window`;
  `your_week` → `{…, registrar_label}`; `week.registrar` (`{school, held, current, refresh}` or
  `null`) and `week.registrar_proposals` (R3-d).
- Produces the JS functions `fromRegistrar`, `registrarControls`, `registrarIdle`,
  `bindRegistrar`, `renderRegistrar`, `reloadSetupRows`, and the page state `registrarLabel`.

**Plan ruling R5-a:** a proposal row shows "from myBama" when its `source_uid` starts
`registrar:` (`fromRegistrar`). *Why:* spec §2 step 4; the key prefix is R1's contract, so the page
reads it rather than asking the engine for another field.
**Plan ruling R5-b:** after a fetch, the confirm screen calls `commitment_proposals` again and
re-applies the answers and levels already given, by key (`reloadSetupRows`). The confirmed rows
leave the list (they are notes now), and the button's line says how many were confirmed. *Why:*
spec §2 step 4, "the confirm screen reloads its proposals", without losing what the student
already marked.
**Plan ruling R5-c:** *Schedule*'s button reads "Get my class times from myBama" while
`week.registrar` holds no term and "Refresh from myBama" once it holds one; it takes the primary
style (`pri y`) only when `week.registrar.refresh` is true (a term the file does not hold has
begun). The registrar proposals are listed under it as rows marked "from myBama" with **Add**,
which confirms at `hard` through `commitments_confirm`. *Why:* D6 and D8 ("nothing nags. The button
is the whole reminder"), and §2's "the others appear as proposal rows" on the *Schedule* view,
which lists no proposals but office hours today.

- [ ] **Step 1 — failing tests.** At the end of `app/tests/static_assets.rs`:

```rust
/// Phase 3 of the commitment model (spec D6, D8, §2): the registrar button on the confirm screen and
/// at the top of Schedule, the sign-in steps, the marker, and nothing typed.
#[test]
fn the_registrar_button_is_on_the_confirm_screen_and_the_schedule_view() {
    let html = read("index.html");
    let classes = html.split("id=\"ws-classes\"").nth(1).unwrap().split("</div></div>").next().unwrap();
    assert!(classes.contains("id=\"ws-reg\""), "D6: under Your classes");
    let sched = html.split("<section id=\"main-schedule\" hidden>").nth(1).unwrap().split("</section>").next().unwrap();
    assert!(sched.find("id=\"sched-reg\"").unwrap() < sched.find("id=\"sched-list\"").unwrap(), "D6: at the top of Schedule");
    for host in ["ws-reg", "sched-reg"] {
        let block = html.split(&format!("id=\"{host}\"")).nth(1).unwrap().split("</div>").next().unwrap();
        for part in ["data-reg-open", "data-reg-done", "data-reg-cancel", "data-reg-say", "I'm signed in"] {
            assert!(block.contains(part), "#{host} lacks {part}");
        }
        assert!(!block.contains("type=\"text\"") && !block.contains("type=\"password\""), "#{host}: nothing is typed in Knowlu");
    }
    let js = read("console.js");
    for f in ["fromRegistrar", "registrarControls", "registrarIdle", "bindRegistrar", "renderRegistrar", "reloadSetupRows"] {
        assert!(js.contains(&format!("function {f}(")), "missing function {f}");
    }
    assert!(js.contains("invoke(\"open_registrar_window\", {})"));
    assert!(js.contains("invoke(\"capture_registrar\", { view: stateView() })"), "one-word argument, the read model's view");
    assert!(js.contains("invoke(\"close_registrar_window\", {})"));
    assert!(js.contains("p.source_uid.indexOf(\"registrar:\") === 0"), "R5-a: the marker by key prefix");
    assert!(js.contains("\"Refresh from \"") && js.contains("\"Get my class times from \""), "R5-c");
    assert!(js.contains("r.registrar_label"), "R4-g: the button only where the school has a registrar");
    assert!(js.contains("bindRegistrar(EL(\"ws-reg\"), reloadSetupRows)") && js.contains("bindRegistrar(EL(\"sched-reg\"),"));
    let bind = js.split("function bindRegistrar(").nth(1).unwrap().split("\n  function ").next().unwrap();
    assert!(bind.contains("e.stopPropagation();"), "no click reaches the drawer or the deck underneath");
    assert!(!js.contains("capture_registrar\", { view: current.view"), "Q10-a: never the page's own view");
}
```

  In `scripts/wizard-check.py`, `WEEK_FAKE`'s `your_week` answer gains `registrar_label: 'myBama'`
  beside `week` (so the button shows on the phase-2 checks too, and must not break them), and
  three answers go before its last `return`:

```js
  if (cmd === 'open_registrar_window') { return Promise.resolve({ ok: true, error: null, opened: true }); }
  if (cmd === 'capture_registrar') { window.__REGISTRAR_DONE = true; return Promise.resolve({ ok: true, error: null, closed: true, state: null,
      result: { term: '202640', rows: 2, confirmed: 1, proposed: 1, dropped: { no_time: 0, midnight: 0 }, warnings: [] } }); }
  if (cmd === 'close_registrar_window') { return Promise.resolve({ ok: true, error: null }); }
```

  and the `commitment_proposals` answer, once `window.__REGISTRAR_DONE` is set, adds one row to its
  list: `{ kind: 'class', level: 'hard', title: 'ART 110', course: null, when: 'Mon 6–8:50pm',
  where: 'Make-Believe Studio 4', source_uid: 'registrar:ua:202640-40006', window: false,
  meets: [{ days: ['mon'], start: '18:00', end: '20:50' }] }` (write it as
  `proposals: [...].concat(window.__REGISTRAR_DONE ? [ART] : [])`, with `ART` a `var` above).

  Then a check, after `check_week_wait`:

```python
def check_registrar(page, errors) -> list:
    """Phase 3 (spec D6, §2): on the confirm screen the button reads 'Get my class times from
    myBama'; pressing it asks for the window and shows I'm signed in; pressing that captures with
    the read model's view, re-reads the proposals, keeps the answers given, and marks the new
    registrar row 'from myBama'. Nothing is typed."""
    bad = []
    if not page.is_visible("#week-setup"):
        return [f"the confirm screen did not open (calls: {names(page)!r})"] + [f"page error: {e}" for e in errors]
    opener = "#ws-reg [data-reg-open]"
    if not page.is_visible(opener) or "Get my class times from myBama" not in page.inner_text(opener):
        bad.append("the registrar button is not on the confirm screen")
    page.click("#week-setup .wsrow[data-key='gcal-series:chess'] [data-answer-set=not]")
    page.click(opener); page.wait_for_timeout(200)
    if "open_registrar_window" not in names(page): bad.append("the button did not open the window")
    if not page.is_visible("#ws-reg [data-reg-done]"): bad.append("I'm signed in did not appear")
    if "Sign in to myBama" not in page.inner_text("#ws-reg"): bad.append("the sign-in instruction is missing")
    page.click("#ws-reg [data-reg-done]"); page.wait_for_timeout(400)
    sent = page.evaluate("window.__CALLS.filter(c => c[0] === 'capture_registrar').map(c => c[1])")
    if sent != [{"view": "today"}]: bad.append(f"capture_registrar was sent {sent!r}")
    if page.evaluate("window.__CALLS.filter(c => c[0] === 'commitment_proposals').length") < 2:
        bad.append("the proposals were not read again after the fetch")
    art = "#week-setup .wsrow[data-key='registrar:ua:202640-40006']"
    if not page.query_selector(art) or "from myBama" not in page.inner_text(art): bad.append("the registrar row is not marked from myBama")
    if page.get_attribute("#week-setup .wsrow[data-key='gcal-series:chess']", "data-answer") != "not":
        bad.append("an answer given before the fetch was lost")
    if "1 confirmed" not in page.inner_text("#ws-reg"): bad.append("the fetch's result is not said")
    if page.evaluate("document.querySelectorAll('#week-setup input[type=text], #week-setup input[type=password]').length"):
        bad.append("the confirm screen accepts typed text")
    for e in errors: bad.append(f"registrar page error: {e}")
    return bad
```

  and in `main`, after the `for succeed in (True, False):` block:

```python
                reg = browser.new_context(viewport={"width": 1280, "height": 860}).new_page()
                rerrors = []
                reg.on("pageerror", lambda e, sink=rerrors: sink.append(str(e)))
                reg.add_init_script("window.__STATE = " + STATE_FIXTURE.read_text(encoding="utf-8") + ";\n" + WEEK_FAKE)
                reg.goto(url); reg.wait_for_timeout(600)
                bad += check_registrar(reg, rerrors)
```

  The module docstring's list of scenarios gains one sentence naming it.

- [ ] **Step 2 — run and see them fail.**
  `cargo test -p knowlu --test static_assets -j 2 -- the_registrar_button` (fails: no `#ws-reg`);
  `.wv\Scripts\python scripts/wizard-check.py` (fails: "the registrar button is not on the confirm
  screen").

- [ ] **Step 3 — implement.**

  **(a)** `index.html`, inside `#ws-classes`, between its `<h2>Your classes</h2>` and
  `<div id="ws-class-rows">`:

```html
<div class="reg" id="ws-reg" hidden><button class="b" type="button" data-reg-open>Get my class times</button><button class="b pri y" type="button" data-reg-done hidden>I'm signed in</button><button class="b" type="button" data-reg-cancel hidden>Cancel</button><span class="hint" data-reg-say></span></div>
```

  and as the first child of `<section id="main-schedule" hidden>`:

```html
      <div class="sec" id="sched-reg-sec" hidden><div class="sec-hd"><h2>Your classes</h2></div>
        <div class="reg" id="sched-reg"><button class="b" type="button" data-reg-open>Get my class times</button><button class="b pri y" type="button" data-reg-done hidden>I'm signed in</button><button class="b" type="button" data-reg-cancel hidden>Cancel</button><span class="hint" data-reg-say></span></div>
        <div id="sched-reg-rows"></div></div>
```

  **(b)** `console.js`, after `bindScheduleView` (line 749), in two edits:

```js
  // ---- Phase 3 (spec D1, D6, §2): Get my class times from myBama. The page opens the school's
  // sign-in window and, once the student says they are signed in, asks the app to read the
  // schedule. The page never sees a cookie or the fetched bytes: `capture_registrar` keeps both.
  var registrarLabel = null;   // your_week's registrar_label; null where the school has none

  // R5-a: a row the registrar proposed.
  function fromRegistrar(p) { return p.source_uid.indexOf("registrar:") === 0; }

  function registrarControls(host) {
    return { open: host.querySelector("[data-reg-open]"), done: host.querySelector("[data-reg-done]"),
      cancel: host.querySelector("[data-reg-cancel]"), say: host.querySelector("[data-reg-say]") };
  }

  function registrarIdle(host, message) {
    var c = registrarControls(host);
    c.open.hidden = false; c.done.hidden = true; c.cancel.hidden = true; c.done.disabled = false;
    c.say.textContent = message || "";
  }

  // `after(env)` runs once a fetch has reached the engine: the confirm screen re-reads its rows;
  // Schedule repaints (paint already ran its renderer when a state came back).
  function bindRegistrar(host, after) {
    host.addEventListener("click", function (e) {
      e.stopPropagation();
      var c = registrarControls(host);
      if (e.target.closest("[data-reg-open]")) {
        invoke("open_registrar_window", {}).then(function (r) {
          if (!r || !r.ok) { registrarIdle(host, r && r.error); return; }
          c.open.hidden = true; c.done.hidden = false; c.cancel.hidden = false;
          c.say.textContent = "Sign in to " + registrarLabel + " in the window Knowlu opened, then press I'm signed in.";
        }).catch(function () { registrarIdle(host, ""); });
      } else if (e.target.closest("[data-reg-done]")) {
        c.done.disabled = true;
        c.say.textContent = "Reading your class schedule…";
        invoke("capture_registrar", { view: stateView() }).then(function (env) {
          c.done.disabled = false;
          if (!env.ok) { if (env.closed) { registrarIdle(host, env.error); } else { c.say.textContent = env.error; } return; }
          if (env.state) { current.pendingOrder = null; paint(env.state, true); }
          var r = env.result || {};
          registrarIdle(host, "Found " + r.rows + (r.rows === 1 ? " class" : " classes") + " in " + registrarLabel + ": " + r.confirmed + " confirmed, " + r.proposed + " to check.");
          after(env);
        }).catch(function () { registrarIdle(host, ""); });
      } else if (e.target.closest("[data-reg-cancel]")) {
        invoke("close_registrar_window", {}).catch(function () {});
        registrarIdle(host, "");
      }
    });
  }
```

```js
  // R5-c: Schedule's registrar section — the label by term, the primary style only when a term the
  // file does not hold has begun, and the registrar's proposals with Add.
  function renderRegistrar(w) {
    var sec = EL("sched-reg-sec");
    sec.hidden = !registrarLabel;
    if (!registrarLabel) { return; }
    var reg = w.registrar, open = EL("sched-reg").querySelector("[data-reg-open]");
    open.textContent = (reg && reg.held && reg.held.length ? "Refresh from " : "Get my class times from ") + registrarLabel;
    open.classList.toggle("pri", !!(reg && reg.refresh));
    open.classList.toggle("y", !!(reg && reg.refresh));
    EL("sched-reg-rows").innerHTML = (w.registrar_proposals || []).map(function (p) {
      return '<div class="row sched reg"><div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") +
        ' <span class="src">from ' + h(registrarLabel) + '</span></span></div><div class="acts"><button class="b" type="button" data-reg-add="' +
        h(p.source_uid) + '">Add</button></div></div>';
    }).join("");
  }

  // R5-b: after a fetch the confirm screen reads its rows again, and the answers and levels already
  // given are put back by key.
  function reloadSetupRows() {
    var kept = {};
    document.querySelectorAll("#week-setup .wsrow[data-key]").forEach(function (r) {
      kept[r.getAttribute("data-key")] = { answer: r.getAttribute("data-answer"), level: r.getAttribute("data-level") };
    });
    return invoke("commitment_proposals", {}).then(function (r) {
      paintSetupRows((r && r.proposals) || [], (r && r.uncovered_courses) || []);
      document.querySelectorAll("#week-setup .wsrow[data-key]").forEach(function (row) {
        var k = kept[row.getAttribute("data-key")]; if (!k) { return; }
        row.setAttribute("data-answer", k.answer); row.setAttribute("data-level", k.level);
        row.querySelectorAll("[data-answer-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-answer-set") === k.answer)); });
        row.querySelectorAll("[data-level-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-level-set") === k.level)); });
      });
    }).catch(function () {});
  }
```

  **(c)** `console.js`, the existing functions:
  - `renderScheduleView`: right after `var w = r.week;`, add
    `registrarLabel = r.registrar_label || null; renderRegistrar(w);`.
  - `checkWeekSetup`: inside the `.then`, before the `if (r && r.ok && r.week && r.week.setup …)`
    test, add `if (r) { registrarLabel = r.registrar_label || null; }`.
  - `setupRow`: the `ttl` div's `<span class="meta">…</span>` is followed by
    `(fromRegistrar(p) && registrarLabel ? ' <span class="src">from ' + h(registrarLabel) + "</span>" : "")`.
  - `paintSetupRows`: `EL("ws-classes").hidden = !classes.length && !uncovered.length;` becomes
    `… && !registrarLabel;`, followed by
    `EL("ws-reg").hidden = !registrarLabel;` and
    `if (registrarLabel) { EL("ws-reg").querySelector("[data-reg-open]").textContent = "Get my class times from " + registrarLabel; }`.
  - After `bindScheduleView();` (line 1494):

```js
  bindRegistrar(EL("ws-reg"), reloadSetupRows);
  bindRegistrar(EL("sched-reg"), function (env) { if (!env.state) { renderScheduleView(); } });
  // R5-c: Add confirms a registrar row at hard, as the office-hours Add confirms at optional.
  EL("sched-reg-rows").addEventListener("click", function (e) {
    e.stopPropagation();
    var b = e.target.closest("[data-reg-add]"); if (!b) { return; }
    b.disabled = true;
    confirmWeek({ mine: [{ source_uid: b.getAttribute("data-reg-add"), level: "hard" }] }).then(function (env) {
      if (!env.ok) { b.disabled = false; showRefusal(null, "refused: " + env.error); }
      if (!env.state) { return renderScheduleView(); }
    }).catch(function () { b.disabled = false; });
  });
```

  **(d)** `console.css`, after the `.wsrow .b[aria-pressed="true"]` rule (line 584):

```css
.reg { display: flex; flex-wrap: wrap; gap: var(--s2); align-items: center; padding: var(--s2) 0; }
.src { color: var(--t3); font-size: 0.9em; }
```

- [ ] **Step 4 — run.** `cargo test -p knowlu --test static_assets -j 2` (all, the phase-2 tests
  among them: `the_confirm_screen_and_the_ask_form_are_there` still finds every id it names, and
  `setupRow`'s preset line is untouched); `.wv\Scripts\python scripts/wizard-check.py` prints `ok`.

- [ ] **Step 5 — commit.** `git add app/static/index.html app/static/console.js app/static/console.css app/tests/static_assets.rs scripts/wizard-check.py`; message:

```
feat(app): Get my class times from myBama, on the confirm screen and Schedule (phase 3, D6, D8)

The button shows only where the school has a registrar. It opens the school's
window, then I'm signed in reads the schedule through capture_registrar; the
confirm screen re-reads its rows and keeps the answers given, and registrar
rows are marked "from myBama". Schedule says Refresh once a term is held, in
the primary style only when a term the file lacks has begun, and lists the
registrar's proposals with Add.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

---

## R6 — the privacy line, the docs, the recount and full verification (spec §4, §5 R5)

**Files.**
- `site/privacy.html`: the "No campus password" item (line 53).
- `engine/tests/site.rs`: one assertion in `the_site_is_plain_html_and_carries_the_privacy_sentence_on_both_pages`.
- `CLAUDE.md`: the `commitments` entry under "The engine's commands"; the Tauri count paragraph
  under "Knowlu (the app)".
- `app/README.md`: the count paragraph and lists (lines 18–60), the file table (the `src/week.rs`
  row, line 284).
- `docs/surface/anatomy.md`: §2's **Confirm screen** row (line 72) and §3.15 SCHEDULE (371–386).

**Interfaces.** None; the page, a test assertion and documentation.

**Plan ruling R6-a: the privacy page's version constant and its Effective date do not move in this
plan.** The line changes in the same release as the button (spec §4), and whether the change needs
a new `PRIVACY_VERSION` (`app/src/account.rs:25`, pinned to the page by
`the_privacy_version_constant_is_the_published_pages_date`) and so a fresh consent is Quinn's, with
the lawyer's read of P1. *Why:* a version bump re-asks every student's consent; that is a product
decision, not a plan's. (Spec problem 8.)

- [ ] **Step 1 — recount by script, before writing a number.** From the worktree root:
  `python -c "t=open('app/src/main.rs',encoding='utf-8').read(); L=[set(x.strip() for x in s.split(']')[0].split(',') if x.strip()) for s in t.split('generate_handler![')[1:]]; print(len(L[1]), len(L[0]), len(L[0]|L[1]))"`
  Expected `50 29 69` (the first list in the file is `run_shell`'s, the second `run_console`'s).
  Count the `#[tauri::command` attributes per module with the Grep tool (count mode) over
  `app/src/commands.rs`, `week.rs`, `registrar.rs`, `onboarding.rs`, `account.rs`, `lms_link.rs`,
  `report.rs`. Expected 27, 4, 3, 14, 14, 5, 2 = 69. Write the numbers the script prints, not
  these.

- [ ] **Step 2 — failing test, then the privacy line.** In `engine/tests/site.rs`, after
  `assert!(index.contains(PRIVACY), …)`:

```rust
    // Phase 3 (spec §4): the school window fetches three things now, the class schedule among them.
    assert!(privacy.contains("your calendar link, your course list and, at schools Knowlu supports, your class schedule"),
        "the privacy page names the registrar fetch");
```

  Run `cargo test -p knowlu-engine --test site -j 2` and see it fail. Then in `site/privacy.html`
  line 53, "to fetch two things — your calendar link and your course list — and then throws the
  session away" becomes "to fetch three things — your calendar link, your course list and, at
  schools Knowlu supports, your class schedule — and then throws the session away". Nothing else
  on the page changes (R6-a). Run the test again: it passes, and so does
  `cargo test -p knowlu --test static_assets -j 2 -- privacy`.

- [ ] **Step 3 — `CLAUDE.md`.**
  - The `commitments` entry's usage line becomes
    `commitments --vault <v> [--today YYYY-MM-DD] [--json] [--confirm <file> [--actor quinn] [--via dashboard]] [--registrar <file> --school <key>]`,
    and the entry gains: "With `--registrar <file> --school ua` (phase 3) it fetches nothing either:
    it parses the registrar's JSON (`registrar::parse_banner`, the one place that knows Banner's
    shape), merges the term under `registrar:<school>:<term>` into `state/calendar-series.json`,
    and confirms each row that matches a vault course at `hard` as the student (`quinn` via
    `dashboard`). It prints `{confirmed, dropped, proposed, rows, term, warnings}` and exits 2 on an
    unknown school or no usable row, having written nothing. The app's `registrar.rs` runs it from
    the school's own sign-in window; it is never a slot step."
  - The Tauri paragraph: "recounted <date> (commitment model phase 3)", the console **50**
    ("phase 3 added `open_registrar_window`, `capture_registrar`, `close_registrar_window`"), the
    vault-less window **29**, **69** distinct (the numbers from Step 1); `registrar.rs` joins the
    list of modules commands live beside. "Nine mutate notes" becomes "Ten mutate notes", adding
    `capture_registrar` (through the engine's `--registrar`).

- [ ] **Step 4 — `app/README.md`.** The count paragraph: **Sixty-nine** (recounted by script,
  phase 3), with "three in `src/registrar.rs`" among the per-module counts; the console window
  registers 50, "all three of `registrar.rs`" added to its list. A new paragraph after the
  `week.rs` one: "In `registrar.rs` (phase 3): `open_registrar_window` opens `lms_link`'s
  incognito, capability-less sign-in window on the school's registrar (`scaffold::Registrar`, UA
  only); `capture_registrar` reads that window's cookies for the registrar host only
  (`lms_link::cookie_url`), makes the calls R0 recorded, and hands the body to
  `commitments --registrar` through a temp file under the profile's `tmp\` (under `vault_io`, deleted
  after), then closes and wipes the window; it keeps the window open when Banner answers with its
  sign-in page. `close_registrar_window` closes and wipes. The console builder manages
  `LmsSession` and hides only `main` on close." The file table gains
  `| src/registrar.rs | the registrar capture: school_of, call_urls, registrar_argv, run_file and the three commands |`.

- [ ] **Step 5 — `docs/surface/anatomy.md`.** §2's **Confirm screen** row: *Your classes* "holds,
  at a school with a registrar, **Get my class times from myBama** (`open_registrar_window`, then
  *I'm signed in* → `capture_registrar`); registrar rows are marked *from myBama*". §3.15 gains a
  bullet before **Left out**: "**Your classes** (only where `your_week`'s `registrar_label` is set):
  *Get my class times from myBama*, or *Refresh from myBama* once `overview.registrar` holds a
  term, in the primary style only when `registrar.refresh`; the registrar's proposals
  (`overview.registrar_proposals`), marked *from myBama*, each with **Add** (a one-row
  `commitments_confirm`, level hard)."

- [ ] **Step 6 — full verification.** One cargo at a time:
  1. `cargo build -p knowlu-engine -j 2` (the real sibling exe over the placeholder).
  2. `cargo test --workspace --no-fail-fast -j 2`. Every test passes but the four `#[ignore]`d
     ones, and there is no `warning:` line but the accepted
     `.rsrc merge failure: multiple non-default manifests`. Check with
     `cargo test --workspace --no-run -j 2 2>&1 | grep -E "^warning"`.
  3. `.wv\Scripts\python scripts/wizard-check.py` prints `ok`.
  4. The fixture byte check: `git diff --exit-code eb10c1b -- engine/tests/fixtures ':!engine/tests/fixtures/registrar'`
     exits 0, and `git status --porcelain -- engine/tests/fixtures` prints nothing. That covers the
     eight Python references and `surface-today-{s1,s1-migrated,full}.json`; the one new fixture
     is `registrar/banner-ua-registration.json`, and `git log --format=%h -- engine/tests/fixtures/registrar`
     names only R1's commit and, if R0 rewrote the fixture, R0's.
  5. `powershell -File scripts/ci/eol-check.ps1` passes: every new file is LF.
  6. `git diff --name-only eb10c1b -- cloud app/src/scheduler.rs app/src/inference.rs engine/src/runtime.rs`
     prints nothing: no cloud change, no slot step, and the local runtime is not extended.

- [ ] **Step 7 — commit.** `git add site/privacy.html engine/tests/site.rs CLAUDE.md app/README.md docs/surface/anatomy.md`; message:

```
docs: the registrar in the privacy page, CLAUDE.md, the app README and anatomy (phase 3)

The school window now fetches three things, the class schedule at schools
Knowlu supports (the version is Quinn's, with P1). commitments --registrar in
the command list; Tauri commands recounted by script: console 50, wizard 29,
69 distinct; registrar.rs in the module tables.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01TC1HP5oAv9SjrvrQzb6juF
```

---

## Closing checklist — the live proof, with Quinn signing in (spec §5 R5; does not block the code)

The code is done when R6 is green. This proof is the controller's, on a dev build copy and a
scratch profile, with Quinn at the machine for the sign-in; it is the same harness as R0 (memory:
autonomous live proof, CDP driver, DOM only). It reads Quinn's real schedule into a **scratch**
vault, which is then removed.

- [ ] Build and copy the dev exes (R0 step 1); start with `--remote-debugging-port` set (R0 step 2).
- [ ] Onboard a scratch profile at UA through the wizard, driven through the DOM (logins from
      `knowlu/dev/{zybooks,vhl}` per memory; the OTP from Gmail). Confirm `config/campus.yaml`
      names `100751`.
- [ ] On the confirm screen, the **Get my class times from myBama** button is visible. Press it
      through the DOM; the sign-in window opens on `bannerssb.ua.edu`.
- [ ] Quinn signs in (Okta, Okta Verify). The controller presses **I'm signed in** through the DOM.
- [ ] The engine's report says `rows > 0`; the class rows that match the wizard's courses are notes
      in `commitments/` with `source_uid: registrar:ua:<term>-<crn>`, `level: hard`, written by
      `quinn` via `dashboard`; the others are rows marked "from myBama". Record counts
      only, never titles or CRNs, in the proof note.
- [ ] The sign-in window is gone and no `knowlu-lms-session-*` folder is left in `%TEMP%`;
      `<profile>\tmp\` is empty.
- [ ] Press the button again before signing in anywhere: the page says "You're not signed in yet"
      and the window stays open; **Cancel** closes and wipes it.
- [ ] `rank` on the scratch vault: the confirmed classes are busy time (the capacity line drops),
      and no `commitment-ask` card names a course the registrar covered.
- [ ] Clean up (memory: live-proof profile cleanup): the scratch profile, its vault, its
      credentials and its autostart entry; unset the debugging variable.
- [ ] Write the proof note in `docs/reports/` (counts and pass/fail lines only).

---

## Self-review (2026-09-26, done while writing)

- **Spec coverage.** Every row of the fidelity ledger names a task, and each spec clause has a
  named test:
  - §3's parse table: `a_row_becomes_the_series_the_spec_table_names`,
    `a_laboratory_is_titled_lab_and_a_repeated_pattern_is_one_meet`,
    `where_is_cut_to_80_and_a_row_without_a_building_has_none`.
  - Rows dropped and the empty parse: `the_fixture_parses_to_four_series_of_one_term`,
    `an_empty_parse_is_a_failure_never_an_empty_semester`.
  - Kind and course without the classifier:
    `the_registrar_arm_takes_kind_from_the_title_and_course_from_the_code_table`.
  - D5: `a_registrar_calendar_stays_configured_while_its_term_is_in_play`,
    `a_row_missing_from_a_fresh_fetch_of_its_term_ends_that_day`,
    `a_next_terms_fetch_leaves_this_term_alone`, `a_dropped_registrar_course_ends_its_confirmed_note`,
    and end to end `a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank`.
  - D7: `a_registrar_series_outranks_its_google_twin`,
    `asks_wait_until_the_day_after_the_registrars_term_starts`,
    `asks_wait_for_the_registrars_term_to_start`.
  - R24, the output and idempotence:
    `matched_rows_are_confirmed_by_the_student_and_the_rest_proposed`,
    `a_fetch_prints_the_report_and_writes_the_series_and_the_notes`.
  - Exit 2 writes nothing: `every_exit_2_writes_nothing`, `a_bad_school_or_file_writes_nothing`.
    No network: `registrar_makes_no_network_call`.
  - D1/D4 in the app: `the_cookies_are_read_for_the_registrar_host_and_never_for_the_sign_in_provider`,
    `the_argv_carries_the_vault_the_file_the_school_today_and_the_consoles_via`,
    `a_fetched_schedule_runs_through_the_engine_and_the_temp_file_is_gone`,
    `a_parse_failure_is_named_writes_nothing_and_the_temp_file_is_gone_too`,
    `the_registrar_uses_the_sign_in_window_and_keeps_nothing`,
    `the_console_builder_wipes_the_sign_in_session_and_hides_only_main`; the capability pin is the
    existing `no_capability_names_the_sign_in_window`.
  - D2: `ua_has_a_registrar_and_every_call_is_on_its_own_registrar_host`,
    `the_school_is_read_from_campus_yaml_as_text_or_number_and_absent_is_none`,
    `your_week_names_the_registrar_only_at_a_school_that_has_one`.
  - D6/D8/§2 in the page: `the_registrar_button_is_on_the_confirm_screen_and_the_schedule_view`
    and `wizard-check.py`'s `check_registrar`; the refresh flag:
    `the_overview_names_the_term_and_the_registrar_proposals`.
  - §4: the `site.rs` assertion.
- **Placeholder scan.** No "TBD", "similar to" or "handle edge cases". One value is deliberately
  not in this plan: `UA_ROWS_PATH` and any extra call in UA's list, which R4 copies from R0's D3
  amendment. That is a scheduled data dependency (R4 depends on R0), and R4's test pins its
  structure, not its string. If R0 finds shape (a), R1-a confines the change to `parse_banner` and
  the fixture.
- **Names checked against the code at `eb10c1b`:** `commitments::{Series, Rule, Meet, Instance,
  Class, Codes, Commitment, Commitments, SeriesFile, Ended, Level, classify, class_course,
  precedence, refresh_series, instances_map, proposals, detect_changes, SERIES_KEY_PREFIXES,
  create_confirmed_as, emit_asks, stored_proposals, ConfirmInput, confirm, CARD_ACTOR, overview,
  Overview, proposal_value, days_since, add_days, ENDED_DAYS}`, `weekcal::{DayKey, WeekCalendar::new}`,
  `journal::{Journal, human_set, now_ts}`, `write::WriteContext::{new, with_actor}`,
  `cli::{commitment_passes, commitments_confirm, vault_zone}` and the tests' `p16_vault`,
  `p16_day`, `rank_p16`, `checks`, `md_names`, `P16_MONDAY`; `lms_link::{WINDOW, LmsSession,
  session_dir, open_window_at, cookie_url, session_agent, close_and_wipe, wipe_session,
  wipe_session_on_exit, sweep_stale_sessions}`, `scaffold::{Curated, CAMPUSES, curated}`,
  `week::your_week_inner`, `commands::{now_in, state_inner, attach_scheduler}`,
  `scheduler::engine_exe`, `ConsoleState::{open, vault, data_dir, vault_io, note_write,
  set_test_today}`; in `console.js`: `stateView`, `paint`, `current.pendingOrder`,
  `renderScheduleView`, `checkWeekSetup`, `setupRow`, `paintSetupRows`, `confirmWeek`,
  `showRefusal`, `EL`, `h`, `invoke`.
- **Consistency.** Names this plan defines, used the same way everywhere: `registrar::{SCHOOLS,
  School, school, CALENDAR_PREFIX, EVENT_TYPE, calendar_key, term_for, looks_signed_out, Parsed,
  parse_banner, Report, run, status}`; `commitments::{registrar_course, registrar_class,
  asks_wait_for_registrar}`, `Overview.{registrar, registrar_proposals}`;
  `cli::commitments_registrar`; `scaffold::{Call, Registrar}`, `Curated.registrar`;
  `app registrar::{school_of, start_url, call_urls, registrar_argv, run_file, open_registrar_window,
  capture_registrar, close_registrar_window}`; `your_week`'s `registrar_label`; the envelope key
  `closed`. Every Tauri argument is one word (`view`).

## Spec problems found while planning, and how they were settled

Each is settled by a Plan ruling so the plan has no gap. **Plan review (2026-09-26), controller
rulings:** items 1–3 are accepted as written; item 4 is resolved differently (below). All four are
now the spec's "Amendments (2026-09-26, plan review)" section. Items 5–8 fill silences.

1. **One `registrar:ua` calendar would end the current term at the next term's fetch.** UA
   students register for spring in November; D5's "a row missing from a later fetch is a dropped
   course" would end every fall class the day spring is fetched. Settled by R2-a: one calendar per
   term, `registrar:<school>:<term>`; R1's `source_uid` is unchanged, and R2-b retires a past term.
2. **D5's end card would never be filed.** A registrar series has no instances, so its `ended`
   entry had no `last_instance`; `detect_changes` then took the term's `until`, equal to the note's,
   and filed nothing. Settled by R2-c: `last_instance` is the day before the fetch.
3. **A confirmed registrar class would block no time for 28 days after each fetch.**
   `instances_map` makes a series' actual instances the only busy time inside its horizon, and a
   registrar series has none. Settled by R2-d: registrar series are left out of the map, so the
   notes block by `meets`.
4. **§3's "actor `quinn` via `dashboard`" would switch off the change and end cards the spec relies
   on.** `journal::human_set` treats a `create` by `quinn` as a human set of every field written,
   and `detect_changes` never proposes a human-set field back. The plan first settled it by writing
   the R24 notes as `agent:commitments`. **Controller ruling at the plan review:** that is
   withdrawn. A fix on `p2-commitments` makes commitment change detection count "the student set
   this field" only from a later `set` by `quinn`, never the `create`; it merges into
   `p3-registrar` before R1. R24's notes and phase 2's confirm screen keep writing as `quinn` via
   `dashboard`, as the spec says, and both get their change and end cards (R3-c as amended; the
   pin is `a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank`).
5. **§5.4's change path runs only for calendars read in the same `rank`,** and a registrar is read
   only on a button press, so "a later fetch that differs files a change card" would not happen.
   Settled by R2-g: every registrar calendar in the file counts as fresh for change detection.
6. **The spec does not say where the term code comes from** before the capture's first call.
   Settled by R1-f: `registrar::term_for` in the engine (Jan–May `<year>10`, Jun–Dec `<year>40`),
   checked against R0's answer 3; D8's refresh flag uses the same rule (R3-d).
7. **The console window cannot close or wipe a sign-in window today.** Its builder manages no
   `LmsSession` and hides every window that asks to close. Settled by R4-f.
8. **The privacy line and the consent version.** Changing `site/privacy.html` may need a new
   `PRIVACY_VERSION` and so a fresh consent from every student. Left to Quinn with P1 (R6-a);
   the line itself ships with the button, as §4 requires.

Two smaller silences, settled without a spec change: *Schedule* had nowhere to show registrar
proposals (R3-d, R5-c: a *Your classes* section with **Add**), and the confirm screen's reload
would have dropped answers already given (R5-b).
