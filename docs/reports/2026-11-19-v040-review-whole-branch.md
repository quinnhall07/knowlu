# v0.4.0 whole-branch review (plan T26)

**Diff reviewed:** `git diff origin/main...feat/v0.4.0-ui`, at `f9f08eca`. I used git and file reads only. I built nothing, ran no cargo, edited nothing, and ran none of the DOM checks.

**Verdict: FAIL.** There are two Important findings:
- **I2** is a bug in the new page itself.
- **I1** already exists on main in a worse form. This branch fixes half of it, but plan §6 says it is fully closed, and the planned proof cannot see the open half.

Both are small fixes. If the main session decides I1 is outside v0.4.0's scope (it records the decision and corrects the §6 row and the note's sentence), the verdict rests on I2 alone.

## Important

**I1. A hidden `mail` or `lms-grades` window opens maximized on screen once its label has been saved maximized.**
- **Where:**
  - `app/src/main.rs:28` (`FLAGS` still includes `MAXIMIZED`), and the builders at `:154` and `:215`, which skip only `"main"`.
  - The hidden windows are built at `app/src/grades.rs:623-629` and `app/src/mail.rs:551-556`.
- **Rule:** plan §6 row "§3 other windows" (plan:538): the hidden grades refresh and the mail step stay hidden.
  - The note's §3 (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md:177`) already says "the live proof confirms" this. No proof has run yet.
- **Mechanism:**
  1. On every new window, window-state 2.4.1 runs `restore_state(FLAGS)`, which calls `self.maximize()` when the saved state is maximized (`tauri-plugin-window-state-2.4.1/src/lib.rs:219-220`).
  2. On Windows, tao applies that with `ShowWindow(SW_MAXIMIZE)` (`tao-0.35.3/src/platform_impl/windows/window_state.rs:377-385`).
  3. `ShowWindow(SW_MAXIMIZE)` shows the window and activates it, even though it was built with `visible(false)` and `focused(false)`.
  - Main avoids this only because of `skip_initial_state("main")`.
- **Scenario:**
  1. During mail setup the student maximizes "Sign in to your email" (label `mail`), then closes it. `CloseRequested` saves `maximized: true`.
  2. From then on, every slot's hidden mail step pops a maximized Outlook window over whatever the student is doing, and takes focus.
  - The same happens with Blackboard's Connect window and the hidden grades refresh, which share the `lms-grades` label.
  - On `origin/main` the default flags also restored `VISIBLE`, so this branch narrows the bug but does not close it.
  - Proof step 10 (plan:722-725) has no step that seeds a saved maximized state, so it would pass anyway.
- **Fix:**
  - Add `.skip_initial_state(grades::WINDOW).skip_initial_state(mail::WINDOW)` (or `with_denylist` with those labels and `mail::POPUP`) to both builders.
  - Pin it in `window_chrome.rs::both_window_state_builders_skip_main_and_never_restore_the_frame`.
  - Correct the note's sentence at :177 and the §6 row so they rest on the pin, not on an unrun proof.
  - Under plan §4 this is T6's scope, reviewed by `contract-reviewer`.

**I2. The top band's clock, Home's "now" and next-class marks, and the Calendar's now line stop moving between revisions.**
- **Where:**
  - `app/static/js/data.js:76`: `arrived()` returns early on an unchanged `revision` and never updates `S`.
  - `engine/src/surface.rs:2466-2476`: `revision` is computed with `now` removed.
  - `app/static/js/shell.js:311-323` and `:374-378`: `drawClock` runs only from `KnowluData.on`, which fires only on a new revision.
  - `app/static/js/pages/home_aside.js:43`, `:157-175`: `nowHM(st)` decides which classes are past, which is next, and draws the "now" mark.
  - `app/static/js/pages/calendar.js:70`, `:225`: `KnowluFmt.now()` reads the same stale `S.now`.
- **Rule:** VISION's "what's next?" has to be honest, and note §4 Shell says the clock is `S.now`. The page shows a time that is no longer true.
- **Scenario:**
  1. The student opens Knowlu at 8:05.
  2. Nothing in the payload changes before the next slot: no write, no due time passes, the plan holds. `runs_panel` and `gauge` carry no minute-level field.
  3. At 11:40 the band still reads "8:05 AM", and Today's schedule still marks the 10:10 to 11:00 lecture as next with the now mark above it.
  4. The Calendar's now line sits at 8:05.
  - The harness cannot catch this: its stub freezes `Date` and each envelope has one fixed revision.
- **Fix:**
  - In `arrived()`, when the revision is unchanged, still copy `st.now` into `S` (and into `HELD` if one is waiting).
  - Have the shell call `drawClock` from `onPoll`.
  - Have Home's aside and the Calendar redraw their now-dependent parts from `onPoll` when `now`'s minute has changed.
  - Pins:
    - `ui_shell`: `drawClock` is registered on `onPoll`, and `data.js` updates `now` on an unchanged revision.
    - A DOM check: two `state` answers with the same revision and different `now`, and the clock and Home's now mark move.
  - Owners are T5 (`data.js`, `shell.js`), T8 and T11, as `v040/tNN-fix`.

## Minor

**M1. Two docs still describe the old page.**
- `app/README.md:293` lists `static/index.html, console.js, console.css` as the console page. CLAUDE.md sends agents to this file.
- `.claude/agents/console-ui.md:4` also names `console.js` and `console.css`. That file is tooling, which loads from the main checkout.
- **Scenario:** an agent briefed from `app/README.md` looks for `console.js`.
- **Fix:** `docs-keeper` updates the README row to the `css/`, `js/`, `js/pages/`, `js/carried/` layout. Apply the agent-file edit by hand in the main checkout.

**M2. Wave A's M7 is still open: the fixture server sends no CSP.**
- **Where:** `scripts/ui-fixture/serve.py:119-121` sets only `Content-Type`, `Content-Length` and `Cache-Control`.
- The wave-A report marked it "for T23 or after". No later commit or report accepts or fixes it.
- **Scenario:** a page change that needs a directive the app's CSP denies passes `run.py --checks` and fails only in the real WebView.
  - Today the pins on URLs, inline scripts and `fetch` cover most of this, so the risk is low.
- **Fix:** have `serve.py` send `tauri.conf.json`'s CSP, with `connect-src` widened only for the stub, or record it as accepted.

**M3. The early-access form ships from the repository, and its data is not on the privacy page.**
- **Where:**
  - `site/js/site.js:76` opens a `mailto:hello@knowlu.com` "Knowlu early access" message.
  - `site/privacy.html` (unchanged) has no waitlist or early-access purpose.
- This is T1-T4 review M2. It is tracked as legal review r1 I3, and nothing here fixes it.
- **Scenario:**
  - The `v0.4.0` tag's Pages deploy (`release.yml:298`) publishes the form from `main`.
  - Production already serves it from the hand deploy, so the tag adds no new exposure.
  - It does make the repository the source of an undisclosed data class.
- **Fix:** add a line to plan §8's pre-tag list: the legal lane's waitlist sentence (bump A, with its `PRIVACY_VERSION` move) is live, or Quinn rules to tag without it.

## §6, the fidelity ledger: 90 rows

| Section | Rows | Met | Met as far as pins and DOM checks reach; proof (§8) pending | Partly met | Unmet |
|---|---|---|---|---|---|
| Decisions and shell | 17 | 12 | 4 | 1 | 0 |
| Home | 15 | 14 | 0 | 1 | 0 |
| Tasks, Calendar, Classes, Lock in, shell row | 15 | 14 | 0 | 1 | 0 |
| Settings | 9 | 9 | 0 | 0 | 0 |
| Carried surfaces | 13 | 13 | 0 | 0 | 0 |
| Wizard, courses, tests, release, must-not-break | 21 | 19 | 2 | 0 | 0 |
| **Total** | **90** | **81** | **6** | **3** | **0** |

Row by row:
- **Decisions and shell**
  - Met: §2 one document, D1, D3, D4, D5, D7, §3 config, B1/N2, show path/N9, capabilities/N6, snap spike/A7, §3 border/N8.
    - §2: `index.html` was touched only by T5 (`d1844d04`) and the T23 move.
    - Snap spike: T2 was dropped and there is no decorum (A7 fallback).
    - Border: its source pin is present.
  - Proof pending: D2, D6, drawn chrome, acceptance bar.
  - Partly met: §3 other windows. The `VISIBLE` half holds; the `MAXIMIZED` half does not (I1).
- **Home**
  - Met: every row except Today's schedule.
  - Partly met: Today's schedule. The past, next and now marks go stale (I2).
- **Tasks, Calendar, Classes, Lock in, shell row**
  - Met: every row except the shell row.
  - Partly met: shell row. The clock goes stale (I2). The sidebar, the Status dot and `google_status` are met.
- **Settings and carried surfaces:** every named pin exists in its named file.
  - I spot-checked that wave B2's I1 to I4 fixes are in the code: `STATUS_WORDS` has `none`, "Through", `update.js` renders error replies, and the two missing names are added.
- **Wizard to release**
  - Met: every row except the two below.
    - "No credential-path change": `app/src` changes only in `main.rs`.
    - Rule 2: nothing under `engine/tests/fixtures/**` and none of the three surface-oracle references changed. `engine/` changes only `src/surface.rs` and `tests/site.rs`.
  - Proof pending: §9 pre-tag gate, §10 live proof.

A read-only script confirmed that every test name §6 cites exists, with the file it names. The only ones not under `app/tests` are the T3 tests in `engine/src/surface.rs` and `site.rs::every_site_font_has_its_licence_and_a_full_file`, and both are present.

## §7, the 165 `static_assets.rs` tests

`origin/main:app/tests/static_assets.rs` has 165 tests. 158 names exist on the branch and 7 are retired as §7 says.

| Fate (file) | §7 count | Found |
|---|---|---|
| Kept → `bundle_config.rs` | 6 | 6. Each body is byte-identical to the old one (diffed); the file holds exactly six tests |
| Moved → `window_chrome.rs` | 1 | 1, with the new exact grant list |
| Retired | 4 | 4 absent |
| Retired, parts re-homed | 4 | 3 names absent; their parts are present (see below) |
| `ui_shell.rs`: 8 whole + 6 split negative halves | 14 | 14 |
| Split positive halves (T13, T15, T16, T17, T19, T21) | 9 named | 9 |
| `ui_home.rs` (Tasks half in `ui_tasks`) | 2 | 2 |
| `ui_home_aside.rs` | 5 | 5 |
| `ui_tasks.rs` | 1 | 1 |
| `ui_calendar.rs` | 3 | 3 |
| `ui_classes.rs` | 2 | 2, plus the split `the_grades_breakdown_never_takes_a_note_panel` |
| `ui_details.rs` | 7 | 7 |
| `ui_deck.rs` | 8 | 8 (the confirm half is in `ui_week`) |
| `ui_settings.rs` | 11 | 11 |
| `ui_sources_mail.rs` | 38 | 38 |
| `ui_week.rs` | 9 | 9 |
| `ui_status_update.rs` | 11 | 11 |
| `ui_account_privacy.rs` | 15 | 15 (wave B2's I4 names are added) |
| `ui_wizard.rs` | 23 | 23 |
| `ui_picker.rs` | 2 | 2 |

Where the parts of the four "retired, parts re-homed" tests went:
- `no_mock_data_or_review_aid_ships` (ui_shell).
- `home_renders_state_unreadable` and `a_changed_order_waits_for_the_pointer`.
- `progressFields` and one-sided bounds in `ui_details.rs:129-139`.
- The refused-slot words in `ui_status_update.rs:76-89`.
- `every_control_this_task_added…` keeps its name in `ui_wizard.rs:562`, with its `set-*` half at `ui_settings.rs:324` and its `report*`/`up-*` half at `ui_account_privacy.rs:325`.

**The swap commit (`05caf612`), standing in for T23's own review:**
- No old file ships: no `console.js`, `console.css`, Instrument Sans, JetBrains Mono or `static/next/` under `app/static`.
- `ROOT = "static"` (`ui_common.rs:14`). The fonts path (`ui_fonts.rs:14`) and the fixture server root (`serve.py:27`) follow it.
- `no_git.rs` scans every shipped `.js` and fails if it finds 10 or fewer.
- `tauri.conf.json` differs from main only in T6's window block and T24's version.
- `capabilities/default.json` holds T6's exact list.
- The deleted scripts have no remaining references in CI or other scripts.
- T23 also edited shell and page code: kit toast duration, quiet check, the narrowed mail scan, plain `cm.from`, Settings' toast. Plan §9 says the integrator never edits page code. I read every one of those edits and they are sound. The narrowed mail scan is still backed by `inner_html_only_from_constants_or_h`.

## The other checks

- **Rule 1 (no single-user assumption):**
  - Course codes are ABC/DEF/GHI/JKL/MNO/PQR/ZZZ. Addresses are `example.edu`, `example.net` and `school.example`. Buildings are "Hall N".
  - The mockup names appear only in `no_mock_data_or_review_aid_ships`'s ban list.
  - No real name, school, vault or path appears in a fixture, envelope, test or shipped string.
  - "Quinn" appears in test comments only, as the author of rulings.
  - `founder-quinn.jpg` and the founder letter on the site are intended.
  - Entity: `HANDOFF.md` and the legal spec name only "KNOWLU LLC, a Kentucky LLC", which Quinn ruled public (spec U7). There is no street address and no co-founder detail.
- **Contract list:** no file on it changed. `engine/src/surface.rs` is not on it, and its change passed `contract-reviewer`.
- **No `http(s)://` under `app/static`:**
  - None in any `.html`, `.js` or `.css`.
  - SVGs carry only the SVG namespace.
  - The OFL texts and `fonts/SOURCES.md` carry licence and provenance URLs, which are never loaded.
  - `index.html` is the only document. No script creates a `<script>`, `<link>`, `<iframe>` or `<object>`. Every `<script>` has a `src`.
- **Red commits:** six checked: `v040-red/t07`, `t10`, `t13`, `t16`, `t21`, `t23`.
  - Each red commit's stat and message match its squash body: pin counts (12; 8; 9+3; 16; 29+2; 2), placeholder failures, DOM timeouts.
  - `ui_home.rs` is byte-identical from red to green.
  - The pin edits in later commits are the review fixes each squash body names: wizard `h2 tabindex`, Settings ban words, slot assertion.
- **Privacy and credentials:**
  - No `console.log`. `console.error` is called only with caught exceptions.
  - The only storage is localStorage, with D4's six keys.
  - `KnowluData.last` keeps answers, never arguments.
  - `ev()` sends a note id, a template word and the fixed route name.
  - The two password fields are cleared after use.
  - Nothing in the page reaches the network, so commitment proposals and `calendar-series.json` cannot leave through it.
  - `PRIVACY_VERSION` and `account.rs` are unchanged.
- **Merge readiness:**
  - Caption buttons call `minimize`, `toggleMaximize` and `close`. Each is granted, and `isMaximized` comes with `core:window:default`.
  - `data-tauri-drag-region="deep"` is supported by tauri 2.11.5's `drag.js`. Double-click maximize needs `allow-internal-toggle-maximize`, which is in `core:window:default`.
  - The wizard and picker keep the band (`mode-boot` hides only the sidebar and `#center`).
  - The tray's `eval` globals live in `router.js`, and the update line in `#top-note`.
  - Ignored tests: 3, the same as main.
  - New files are LF, and binaries are stored as binary.
  - Workflows are untouched.
- **Earlier reviews:**
  - Wave A: I1, I2 and M1 to M6 are fixed (M6 by `carve()` in `t11.py`), and M4 is accepted. M7 is still open (M2 above).
  - Wave B1: everything is fixed.
  - Wave B2: everything is fixed except M12, which is recorded in the squash body.
  - T5: I1 to I3 are fixed.
  - T1-T4: I1 and I2 are fixed. M2 is still open (M3 above).

Findings:
- I1: `app/src/main.rs:28`, `:154`, `:215`: `FLAGS` keeps `MAXIMIZED`, so a hidden `mail` or `lms-grades` window saved maximized opens on screen.
- I2: `app/static/js/data.js:76` with `shell.js:374-378`, `home_aside.js:157-175`, `calendar.js:70`: `S.now` only changes with `revision`, which leaves `now` out, so the clock and the now marks go stale.
- M1: `app/README.md:293` and `.claude/agents/console-ui.md:4` still describe `console.js`.
- M2: `scripts/ui-fixture/serve.py:119-121` sends no CSP (wave A M7).
- M3: `site/js/site.js:76`: the early-access mailto ships before the privacy page names it (T1-T4 M2).

No report file was written; this message is the report, for `C:/Users/danie/GitHub/knowlu/.claude/worktrees/v040-ui/docs/reports/2026-11-19-v040-review-whole-branch.md`.

---
Reviewer: `reviewer` (Opus, high), read-only; report text saved by the main session.

## Re-check after the fixes (2026-11-19): PASS

Fixes: I1 in f832549d, after a contract review PASS (three minors fixed); I2 and M2 in dcd2b1c2. M1 (docs) is in a0a2d545 and e9956033. M3 is a pre-tag line in plan §8 for Quinn's ruling.

**I1 is closed (f832549d).** Both window-state builders now skip the initial state of `"main"`, `grades::WINDOW`, `mail::WINDOW` and `mail::POPUP` (`app/src/main.rs:161`, `:222`). That means a saved maximized state can no longer reach `ShowWindow(SW_MAXIMIZE)` for a window built hidden. The new `window_chrome::every_window_built_hidden_is_in_the_skip_list` scans every `WebviewWindowBuilder` chain under `app/src`, and its `.visible(visible)` match covers the mail step. Saving on close is unchanged. The commit matches what the report described.

**I2 is closed (dcd2b1c2).** Each point you asked about:
- **No spurious listeners.** `data.js:80-82` copies `now` into the state on screen and then returns on an unchanged revision without calling `emit`. Only `onPoll` subscribers run.
- **Held state.** A waiting held state also gets the new `now` (`data.js:81`). Note that the only hold in `data.js` is the hover hold on a list, which is the one this covers.
- **Focus.** Home's aside redraws only `#ha-sched`, and puts focus back on the same `[data-lane-remove]` button. The "N earlier" toggle sits outside `#ha-sched`, so it is never replaced. The Calendar moves its now line, gutter label and `is-past` classes in place, so the grid node and focus stay.
- **Repaint.** Both pages compare the minute and stop on an unchanged one, so no page repaints every poll.
- **Home's marks at a block's end.** The schedule redraws when the past count changes or when the now mark's presence flips, so the next block and the now mark move at a block's end and at the first block's start.
- **Calendar day rollover.** It reloads `your_week` and redraws, and `draw()` resets the drawn day and minute.
- **Text.** The new text is set only with `textContent`, plus `innerHTML` built through `h()` in `drawSched`.
- **The red (53236506) failed as stated.** It sent the same revision twice with `now` at 09:01 and then 11:30. The fix turns it green.

**M2 is closed.** `serve.py:35-53` reads `app.security.csp` from `app/tauri.conf.json` and appends `'self'` to `connect-src` only. Every other directive is the shipped one, and the server exits if there is no `connect-src` to widen. The stub is loaded as the external `__fixture__/stub.js`, which the shipped CSP allows as `'self'`, so the CSP needed no exception for it.

**Shell rules hold.** There is no new `<script>` or `<link>`, and `index.html` is unchanged.

**Minor findings (none blocks the PASS):**
- **m1** `app/static/js/shell.js:375`: `drawClock` now also runs from `onPoll`, which fires three times per poll (state, `grades_status`, `mail_status`). Each run clears `#clock` and rebuilds its two spans even when the minute has not changed. It is `aria-hidden`, so a screen reader announces nothing. But it is the one node that is rewritten on every poll. Fix: return early when the date and clock text are unchanged.
- **m2** `app/static/js/pages/calendar.js:298`: the day-rollover branch has no test, because `t05.py` stays on 2026-08-28. The page also keeps its current window (`FROM` is not re-anchored). So a week view left open from Sunday into Monday shows last week, with no today column and no now line. This matches the old `update()` behaviour, so it is not a regression. Fix: add a check that crosses midnight, and decide whether a window still anchored on yesterday should follow today.

Nothing was written to disk: `docs/reports/2026-11-19-v040-review-whole-branch.md` already exists and the hook refuses an existing file. This reply is the text for you to append to `C:\Users\danie\GitHub\knowlu\.claude\worktrees\v040-ui\docs\reports\2026-11-19-v040-review-whole-branch.md`.

The re-check's m1 (the clock redrawn on every poll) and m2 (the Calendar's day rollover is untested, and a week left open across midnight keeps last week) are recorded as v0.4.1 candidates.
