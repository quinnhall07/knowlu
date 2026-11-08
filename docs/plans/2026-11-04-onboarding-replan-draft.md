# Wave B replan after the 2026-11-04 live sign-in run: draft

**Status: r10, PASSED review r10 (0 Critical, 0 Important, 5 Minor, as relayed by the coordinator), its
five minors applied as r10's final pass. Nothing dispatched.** r10 closed r9's 1 Important and 3 Minor
(`docs/reports/2026-11-07-onboarding-replan-review-r9.md`). The ledger's "Review r9" and "Review r10" rows
name each finding. **Dispatch now waits only on R0c, the 24-hour run (§12).**
- r6 revised it for the 11-06 and 11-07 spike results and Quinn's rulings RQ3, RQ4, RQ5 and RQ16, and
  replaced outcomes A and B and the shared folder (§2, §4.3).
- r6, r7 and r8 failed review (`docs/reports/2026-11-07-onboarding-replan-review-r6.md`, `-r7.md` and
  `-r8.md`). Each round closes the one before, and r9 closes r8's 4 Important and 6 Minor. The ledger's
  "Review r6", "Review r7" and "Review r8" rows name each finding.
- r5 passed review (`docs/reports/2026-11-04-onboarding-replan-review-r5.md`).
- Quinn has ruled RQ1 to RQ18 (§10), and **confirmed the setup split on 2026-11-07** ("yes to all four";
  `docs/notes/2026-11-07-onboarding-rulings.md`; §7.4, checkpoint 1).
- **The plan stands pending R0c, the 24-hour repeat run** (§2.3).

This draft amends `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md` ("the plan") and does not edit
it. With the setup split confirmed, the rulings fold into the plan as a new §14 entry, and this file is
its record.

**Date:** 2026-11-04. **Lanes:** O3 and O4. **Branch read:** `feat/o3-long-onboarding`.

**Authority.** The cloud design wins where it speaks. Then Quinn's ruling of 2026-11-04 (the plan's §3,
"Results of the 2026-11-04 live run"): the registrar is optional, and onboarding must not depend on a
registrar sign-in. That ruling retires spec §4.2 item 1 ("Registrar, automatically"). Quinn's rulings of
2026-11-07 (RQ3, RQ4, RQ5, RQ16) come next: the school password is saved by default, the LMS and the
mailbox each get their own folder, the LMS signs in hidden at setup from Knowlu's own form, and a lapsed
LMS session signs back in silently every time. With two folders,
Q1's "one sign-in" is not met as written, so C9 drafts a signed note on Q1 (§2.2). The rest of the spec
and the plan stand unless this draft says otherwise.

**How to read it.** §1 corrects three premises against the code. §2 says how the LMS signs in now. §4 is
the task list. §5 is the registrar, §6 the class-search reader (§6.6 its own source, §6.7 the press
rule, §6.12 what is deleted), §7 the LMS link. §10 holds the rulings and the open questions.

---

## 1. Three things the code says that today's record does not

These do not reopen today's facts. They change what the class-search reader costs.

1. **The class-search answer is not in the shape `parse_banner` reads.** The record says "the same row
   shape". The fields inside one meeting match (days, `beginTime`, `endTime`, `meetingType`, dates,
   building, room). The container does not:

   | | `parse_banner` | Class search, as recorded |
   |---|---|---|
   | Rows | `data.registrations[]` | not recorded; R0b records it |
   | A row's meetings | flat `meetingTimes[]` | `meetingsFaculty[].meetingTime` |
   | Registered status | `statusIndicator` | no such thing: a public list has no student |

   A test pins the difference: `p2_the_meetings_faculty_wrapper_is_not_read`. Handing class-search
   bytes to `commitments --registrar` would exit 2 with "not a list of registration rows". So C1 adds a
   second reader beside `parse_banner`, sharing its meeting rules. That test stays as it is.

2. **`run_parsed` reads an answer as the whole term.** A confirmed note whose section is missing from
   the answer gets its `until` pinned to yesterday. The class ends. Class search asks about one
   course at a time, so one failed or skipped query must never read as "dropped". Class search
   therefore never goes through `run_parsed`, and nothing it omits ends anything (§6.6).

3. **`R24` must not run on class search.** The registrar proves the student is enrolled, so its rows
   are confirmed hard with no press. Class search proves only that a section exists and when it meets.
   At the test account 15 LMS shells begin with the current term, more than a normal load, so some are
   not real classes. Auto-confirming them would put hard blocks on a student's week that Knowlu
   invented. Every class-search row is a proposal the student confirms (§6.7).

One smaller fact: the app's `ureq` has no cookie jar (`app/Cargo.toml` says nothing needs one), and
Banner holds the chosen term in an anonymous session. C4 carries that one session cookie by hand, on
the stack, for one run. It is not a sign-in and is never stored.

## 2. How the LMS signs in (rewritten 2026-11-07)

r5's outcomes A and B are **withdrawn**. They asked whether one sign-in covers both the LMS and the
mailbox, and r5 leaned on the 11-05 claim that a mailbox sign-in carries the LMS session. That claim is
withdrawn (the plan's §3, "Correction and results, 2026-11-06 and 11-07"). So are `A-lms`, `A-mail`, the
`school_sign_in` enum and the shared folder.

### 2.1 What the 11-06 and 11-07 runs settle

- **The mailbox does not carry the LMS.** A mailbox-only sign-in did not carry the link. Nothing here
  relies on one sign-in covering both.
- **The LMS session lapses about 12 hours after sign-in, used or not.** Keeping it warm does not extend
  it. No task keeps a window open or polls to keep a session alive.
- **Saved cookies are dropped.** LMS cookies held in memory died near 3 hours. No task stores or replays
  an LMS cookie.
- **There is one route.** The LMS's public login page holds only the school's own sign-in link (§7.1).
- **The saved password works.** A hidden window on an empty folder, filling the school's own sign-in
  page from Credential Manager, signed in to the LMS in about 8 s, with no MFA and no window shown. The
  earlier failures were the fill's own mistakes. §7.5 holds the rules that fixed them.

### 2.2 The design that follows

- **Two folders (RQ4).** The LMS gets its own kept folder, used by grades and the course-list capture. The
  school mailbox keeps its own `mail-session-<id>` folder, as the mail lane built it. "Disconnect" stays
  per source.
- **The school password is saved by default (RQ16).** It is stored like every other portal login: target
  `knowlu/<profile_id>/school` in Credential Manager, written by `credentials::write`, read by
  `wincred::read_credential`. It stays on the device. It is never sent to the cloud, returned to the page,
  logged, or put in a run record.
- **Step 3 (RQ5).** The student types the school username and password once, into Knowlu's own form, as
  for every other portal login. A hidden window opens at the link and fills the school's page (§7.5). No
  school page is shown. If the school shows its recognised "wrong password" message, the student retypes
  the password in Knowlu's form, with one line, "Your school didn't accept that password". Anything else
  after the password (an MFA prompt or an unknown page), or a page the fill cannot read, opens the
  school's own page in one visible window, and the student finishes there (§7.4). Quinn confirmed this
  split on 2026-11-07. Knowlu never captures a password typed on the school's page, and never re-sends
  one on its own.
- **A lapse (RQ3).** Every hidden LMS read that meets the school's sign-in page fills it again, silently,
  every time.
- **After setup, only two things reach the student.** First, the saved password was rejected: after the
  submit, any ending but a signed-in LMS (§7.4, §7.5). It is asked for once, in Knowlu's own form, and
  never retried until a new save, because a retry risks a lockout. Second, the fill cannot read the page
  before the password is sent. Then the visible sign-in at the link is offered, not the form. Only these
  show the named signed-out row ("Sign in to update grades").
- **Step 5 (RQ5).** One visible school-mailbox sign-in, on the mailbox's own folder: MFA once and "stay
  signed in". After it, mail stays silent, as the mail lane built it. The saved password is not filled
  into the mailbox.
- **Q1.** The student now types one password and does one mailbox sign-in. Spec §1.3's "one sign-in" is
  not met as written, so C9 drafts a signed note on Q1. In r5 that note was outcome B's alone.
- **Rule 1.** Which host the fill may fill is curated row data (`sign_in_hosts`, S2). A row with
  `lms_sign_in` and no `sign_in_hosts` gets no fill: the student signs in in a visible window at the link.

### 2.3 Pending: R0c, the 24-hour repeat run

R0c is running: a fresh folder every 3 hours, stopping at the first failure. **The plan stands pending
it.** P1, P2 and P3 do not depend on its answer. P4's words and P5's proof do, so they wait for it.

**If R0c shows an MFA prompt on a fresh sign-in:**
- **At setup** (review r7, Important 3). The fill never answers MFA. An MFA page after the password is
  `Rejected(Unconfirmed)`, so the school's page opens in a visible window and the student answers MFA
  there. Setup finishes, and no false "didn't accept" line is shown. The saved password was deleted, so
  step 3 offers the form once more (§7.4), without "to keep signing in for you", since under MFA a new
  save would not do that (review r8, Minor 4). How many MFA prompts or pushes the student gets is not
  known. The fill sends the password once, but the school decides what it pushes, and R0c records it.
- **After setup.** Each lapse, about one every 12 hours, meets MFA after the password. That is
  `Rejected(Unconfirmed)`: the password is deleted, and the form's words say Knowlu couldn't finish
  signing in, never that the school refused it. A re-saved password would meet MFA again at the next
  lapse, and each try pushes to the student's phone unasked. So RQ3's "silently every time" could not
  hold. RQ3 goes back to Quinn with the old choices: the signed-out row and a sign-in from Settings, or
  grades refreshed only when the student signs in. The same question asks whether an MFA page should
  become a recognised page that stops the fill before any further push. That would change the
  conservative rule, and it needs its own review.
- If MFA shows only sometimes, the same rules apply to each attempt. R0c records how often, for Quinn.
- P5 counts MFA prompts as well as windows.

**A sample of one.** R0c is one account. P5 repeats the check on a friend's account at the founder's
school (RQ13) before the default is relied on.

## 3. Checked against VISION and the two rules

- **"What's next?"** Class times become known without typing or a registrar sign-in, so the week and
  free time are honest sooner. Passes.
- **Commitment 1, deterministic engine.** The parse and the name reading are pure engine code with no
  model. Passes.
- **Commitment 4, confirmed commitments.** No row lands without a press on that row, on any screen.
  §6.7 walks every path. Passes.
- **Commitment 5, who started it.** Knowlu inferred the section from an LMS name, so this is "Knowlu
  noticed": it proposes and waits. The public list is not treated as an authoritative change, and no
  row is labelled with the registrar's name, because class search has its own source (§6.6).
- **Fetch on device, think in the cloud.** The read runs on the student's computer, anonymously. The
  cloud never fetches it, and nothing reaches a model. `VISION.md` names this source (RQ10, done).
- **Data minimization.** One query per course, public data only. The bundle and every unconfirmed row
  are deleted at setup end and at sign-out, no decline marker carries a section, and no backup holds
  class-search data (§6.12, RQ11). One small list outlives setup, by ruling, and the privacy text says so
  (RQ12, RQ14).
- **Failures are visible.** Each course ends in a named state (§6.5). A school that closes guest
  search reads as closed, never as "no classes".
- **Portal credentials never leave the device.** The school password is in Credential Manager, keyed by
  profile, read only in the app's own process, and filled only into the school's own sign-in page on a
  curated host over https (§7.5). No cloud call, page reply, log or run record carries it. Passes.
- **Failures are visible.** A rejected password or a page the fill cannot read is a named row, never a
  silent stop. A rejection is never retried, so Knowlu cannot lock a student out. Passes.
- **Rule 1, no single-user assumptions.** Class search, the link prefix and the fill's sign-in hosts are
  all curated row data, per school. The credential target is `knowlu/<profile_id>/school`, derived and
  never hand-edited. Nothing is per student or hand-edited.
- **Rule 2, frozen references.** No task touches `engine/tests/fixtures/**`. New tests use invented
  rows inline, as `engine/src/registrar.rs`'s tests already do. No frozen reference changes.

## 4. Tasks

The plan's §2 constraints all carry over: TDD with `todo!` stubs, 0 warnings, about 80 lines per file
per task, tests never leave the machine, no new capability, every live step the controller's.
**Contract list:** the only contract-list file the replan edits is `app/src/account.rs`, by the plan's
existing S5, S7 and F2c. S5 now also deletes the saved school password (§4.5, RQ17), so S5 stays
`contract-engineer`'s. **The saved password goes through the app's existing credential code:**
`credentials::target_for`, `write`, `exists` and `delete`, and `wincred::read_credential`, all already
public. No task plans an edit to `app/src/credentials.rs` or `engine/src/wincred.rs`. Both are on the
contract list, so if any task finds it needs one, that edit is `contract-engineer`'s, `contract-reviewer`
reads the diff, and the controller is told before it lands. P1 and P2 handle the password itself, so they
are `contract-engineer`'s too, though their files are off the list. No other new task touches the list: the purge at sign-out sits inside
`on_sign_out`, which F2c's `sign_out_clean` already calls, and that function keeps its signature.
The launch sweep only reads `account::cloud_config`, `account::session_target` and
`wincred::read_credential`, all already public. `commitments.rs`, `registrar.rs`, `backup.rs`, `scaffold.rs`, `lms_link.rs` and `grades.rs` are
off the list, but C2a to C2e and C8 can silently damage vault meaning, student data or a session, so
they take stronger agents or reviewers than the list alone would require.

Task ids are new and do not collide: `R` decision runs, `C` common, `P` the saved school password. r5's
`A1` to `A4` and `B1` to `B4` are withdrawn with outcomes A and B (§2), and no id is reused.
**Counts:** 4 decision runs (R0a done), 18 common tasks plus T3b, and 9 for the saved password (P1 and P2
are each split in two, review r8, Minor 6). Where the plan says P1 or P2, it means both halves.

### 4.1 Decision runs (common; first)

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| R0a | The sign-in runs, 11-05 to 11-07. **Done**; results in the plan's §3 and §2.1 here (brief in §8) | main session | live; Quinn at the machine; a throwaway build | no | none |
| R0b | The class-search spike: public data, no sign-in. **Done 2026-11-08**; results under its brief in §8 | main session | live reads are the controller's | no | none |
| R0c | The 24-hour repeat of the saved-password fill, **running** (brief in §8) | main session | live; the throwaway build | no | R0a |
| T3a | S-G alone, split from T3: hidden reads while the student is elsewhere, now through the link | main session | live; S4a and S4b depend on it | no | R0a's build |

### 4.2 Common tasks

**What needs no sign-in result:** C0, C1, C2a to C2e, C3, C4, C5a, C5b, C7, C8 and C10. C1's and C4's
exact keys wait on R0b, not on R0a.

**What waits on the sign-in work:** C5c, C5d and C6. C5c writes through F8b2's fenced writer. F8b2
follows F8b, and F8b now follows P3 (§4.3). P3 does not wait on R0c.

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| C0 | `scaffold.rs`: `ClassSearch`, `Curated.class_search`, `Curated.lms_sign_in`, `None` on every row | `mechanical` | one struct and two fields, checked by the compiler; ends the `Curated` collisions | no | none |
| C1 | Engine: `section_in_name`, `parse_class_search`, the `School` row's name shape | `test-writer`, then `implementer` | specified and test-checked | no | R0b for `parse_class_search`'s keys; `section_in_name` needs nothing |
| C2a | Engine: the `classsearch:` source row, tier, keep-alive, `classify` arm, note body, never a card, never a successor. About 30 lines in `commitments.rs` | `test-writer`, then `integrator`; `contract-reviewer` reads the diff | a wrong tier silently reverses which source wins; the syllabus plan gave the same edit to `integrator` | no, but vault meaning | C1 |
| C2b | Engine: `purge_class_search`, the no-time block, `uncovered_courses`' new input. About 110 lines in `commitments.rs`, 2 in `cli.rs` | `test-writer`, then `integrator`; `contract-reviewer` reads the diff | a miss makes the approved privacy text false, or undoes RQ9 | no, but student data | C2a |
| C2c | Engine: a registrar read adopts a class-search note (§6.6). About 30 lines in `registrar.rs`, 8 in `commitments.rs` | `test-writer`, then `integrator`; `contract-reviewer` reads the diff | it re-keys a confirmed note | no, but vault meaning | C1, C2a, **C2b** |
| C2d | Engine: Not mine with no marker, the guard in `create_marker_as`, fixed words in `confirm`'s warnings. About 35 lines in `commitments.rs` | `test-writer`, then `integrator`; `contract-reviewer` reads the diff | a miss sends a section off the device | no, but student data | C2b |
| C2e | Engine: backups never hold class-search data. About 20 lines in `commitments.rs`, 35 in `backup.rs` | `test-writer`, then `integrator`; `contract-reviewer` reads the diff | a miss outlives the purge for 30 days | no, but student data | C2b |
| C3 | Engine: `commitments --class-search`; `onboarding --preview --class-search` | `test-writer`, then `implementer` | specified; both read-only of notes | no | C1, C2a, C2b |
| C4 | App: `app/src/class_search.rs` (the fetch), and the founder's row once curated | `test-writer`, then `implementer` | specified; proven on a local fake server | no | C0, C1, R0b; RQ7's check for the row |
| C5a | Queue: `registrar.confirm` out of `onboarding_jobs.rs` and its tests | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | a removal, but 30 test references need judging one by one | no | none; lands at the head of that file's chain |
| C5b | State: the registrar pieces out of `onboarding_state.rs` and its tests | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | the file holds capture state | no | C5a |
| C5c | Queue: `school.class_search`, `class_times.apply`, and `proposals_ready` waits for both | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | orchestration; a stale capture must not land | no | C3, C4, C5b, **F8b, F8b2**; takes F8h-runner's place in the `onboarding_jobs.rs` chain |
| C5d | Purge callers: finish, sign-out, `change_school`, and the fail-safe launch sweep over every profile. About 70 lines in `onboarding_state.rs`, 6 in `onboarding_jobs.rs`, 2 in `main.rs` | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | the privacy text rests on it, and a wrong answer deletes a signed-in student's data | no, but student data | C2b, C5c, F2e, F4c |
| C6 | Page: step 10's class-search group, the proof line, the job labels | `test-writer` (pins), then `console-ui` | `app/static` | no | C3, **C5c**, F11p3; last in the panel merge order, after F11e2 |
| C7 | Page: the week strip's "N turned in, kept off your list" note (amends §14.58.5) | `test-writer` (pins), then `console-ui` | `app/static`; Quinn's addition | no | none |
| C8 | App: the LMS sign-in link; hidden and visible windows open at it; the `read_hidden` seam | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | a wrong host leaks a session (the plan's Q-B6) | no, but sessions | C0; lands before S4a and S4b |
| C9 | Docs: amendment drafts, the curation check, reference pages | `docs-keeper` | docs only | no | C0 to C8, C10; F13 follows it |
| C10 | Page: the first-day screen and Schedule treat a `classsearch:` row as its own source | `test-writer` (pins), then `console-ui` | `app/static`; Critical 2's second screen | no | C2a |

**Files with more than one task, and their order:**

| File | Order |
|---|---|
| `scaffold.rs` | C0, C8's row, C4's row if its curation check passes, then S2 (with the founder's `sign_in_hosts`) |
| `lms_link.rs` | C8, then S4b, then P3 |
| `grades.rs` | C8, then S4a, then P3 |
| `onboarding.rs` | P1b only (`retarget_credentials`' source list) |
| `school_password.rs` | P1a, then P1b |
| `school_fill.rs` | P2a, then P2b |
| `school_session.rs` | S2, then S6a (after P1), then P3's step-3 sign-in |
| `commitments.rs` | C2a, C2b, C2c, C2d, C2e, then C3's wiring |
| `cli.rs` | C2b, then C3 |
| engine `registrar.rs` | C1, then C2c |
| `onboarding_jobs.rs` | C5a; the plan's chain with F8h-runner's slot now C5c; then C5d |
| `onboarding_state.rs` | C5b at once, ahead of the plan's remaining work there, which rebases on it; then that work; then C5c and C5d |
| `main.rs` | the plan's chain, F4, F5, F4c, F5b, S6r (which also carries P1's command); then C5d's call in `main`. P1 never edits it |
| `onboarding.js` | C7 at any point; C6 last, after F11e2 |

**Released by RQ4.** S1, S2, S3, S4a, S4b, S5, S6t, S6a, S6r, S6b, S7, S8 and S9, as §4.5 changes them for
two folders, are now common. S4c is dropped (§4.5). The finding has reached Quinn and the planner, so the
plan's rule that holds O4 tasks is met. S4a and S4b still wait on T3a for S-G.

**After the rewrite, common.** **T3b** (main session): S-E and S-F's live half, moved so they time the
chosen flow (§4.6). It depends on F4c: `main.rs` holds neither `Queue::start` nor `load_state_for_launch`
yet, so until F4c lands no queue runs and there is nothing to time.

### 4.3 The saved school password (RQ3, RQ16; replaces outcomes A and B)

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| P1t | Failing tests for P1 and for P2's pure decisions (briefs in §8) | `test-writer` | specified; its one real-credential test needs the file-scoped `CREDMAN_LOCK` and the controller's sign-off, as `test-writer`'s rule says | no | S2 for the row's `sign_in_hosts` |
| P1a | App: `app/src/school_password.rs`, the credential: save under the lock, read inside the process, forget; the save command, whose `generate_handler!` line is S6r's | `contract-engineer`, then `contract-reviewer` | it holds a student's school password; a silent error leaks it. It routes through `credentials.rs` and `wincred.rs` and edits neither | no; routes through `credentials.rs` | P1t |
| P1b | App: `school_password.rs`, the rejected and in-flight marks, flushed and ordered; `retarget_credentials` moves `school`, never a marked one | `contract-engineer`, then `contract-reviewer` | a mark that is lost or cleared out of order re-sends a password and can lock the account | no | P1a |
| P2a | App: `app/src/school_fill.rs`, the pure decision (§7.5) | `contract-engineer`, then `contract-reviewer` | a wrong host, method or submit kind sends the password where it must not go | no | P1t, C8 |
| P2b | App: `school_fill.rs`, the hidden window's script: the mark before the first credential-carrying submit, the one `serde_json` argument, the capture-phase listener | `contract-engineer`, then `contract-reviewer` | it puts the password into a page | no | P2a, P1b |
| P3 | Wiring: the hidden LMS reads fill on a lapse (§7.4); step 3's sign-in from the saved password replaces S4d; the rejected and unreadable outcomes show the named row | `implementer`, reviewed by `contract-reviewer` | as S4d; P2 holds the risky part | no | P2, S4a, S4b (the setup split it builds was confirmed by Quinn on 2026-11-07). **F8b follows it** |
| P4t | Failing pins for P4 (brief in §8) | `test-writer` | pins first, as C6, C7 and C10 have | no | P3 |
| P4 | Page: step 3's school password form and its line; step 5's own mailbox sign-in; Settings' "Update school password"; spec §1.3's "You won't sign in to these again" goes | `console-ui` | inside F11e1, F11b and S6b, in the panel merge order; Quinn reads the words | no | P4t, P3, S6r, **R0c** |
| P5 | S10 and F15's checks: one typed password, no visible LMS window, one mailbox sign-in; a lapse past 12 hours signs back in silently; one deliberately wrong password is asked once and never retried; a friend's account repeats it (RQ13) | main session | live; Quinn's go | no | P3, P4, T3b, S9, F14, **R0c** |

The signed note that Q1's "one sign-in" is not met as written (§2.2) is now always needed. C9 drafts it.

### 4.4 What happens if R0c shows MFA

No task changes. P4's words and P5's checks follow §2.3's branch, and RQ3 goes back to Quinn before P4
goes out.

### 4.5 Existing tasks: dropped, changed, kept

**Status caveat.** The planner has no shell. Merge state below comes from reading this branch's code:
`app/src/onboarding_live.rs` does not exist, `set_school` and `write_capture_file_for_gen` do not
exist, `lms_link::validate_only` and `capture_link_only` are still `todo!` stubs, and no
`school_session.rs` exists. The controller re-checks before the first dispatch.

**Dropped (3).**

| Task | Why |
|---|---|
| F8h-runner | `registrar.confirm` leaves onboarding, so its runner and `registrar_file_missing` have no job |
| F8b's registrar half | Onboarding no longer captures registrar rows. `REGISTRAR_FILE` and `onboarding-registrar.json`, which F2b2b's comment reserves, are never built; C5b removes the comment |
| S4c | Two folders (RQ4): the school mailbox stays on its own `mail-session-<id>` folder, so the mail window, `mail/commands.rs` and `mail_session.rs` change nothing for O4 |

**Changed (23).**

| Task | Change |
|---|---|
| T3 | Split: T3a (S-G) stays early; T3b (S-E, S-F's live half) moves after the rewrite (§4.6) |
| S1 | `add_use(Registrar)` and `add_use(SchoolMail)` go: the folder is the LMS's alone (RQ4), and its uses are grades and the course list. `profile_for` gives it to grades and the LMS capture only. The folder holds no password: that is in Credential Manager (P1). Test 5, the lock, becomes per profile (review r9, Minor 2; review r10, Minor 3): a second `try_take` for the same profile id is refused while the first guard lives, a `try_take` for another profile id succeeds beside it, dropping the guard frees it, and neither call blocks. A window-opening caller that holds the session lock but finds `Capturing` busy answers `Busy` and drops the session guard |
| S2 | **S2 owns the lock (review r9, Minor 2).** `school_session.rs` exposes a public, per-profile, non-blocking lock: `pub fn try_take(profile_id: &str) -> Option<SessionGuard>`, where `SessionGuard` frees the lock on drop and `None` means busy. It is keyed by profile id (review r10, Minor 3), so two profiles never block each other on their folder, marks or secret. **The window guard stays process-wide:** window labels are not per profile, so today's `grades.rs` `Capturing::try_take` stays the guard on the hidden window's label. A window-opening caller takes `school_session::try_take` first, then `Capturing`. Both are non-blocking, and either one busy answers `Busy`, with the first guard dropped. P1a's save opens no window, so it takes only the session lock. The fill, the hidden reads, F8b's window and P1a's save all use it. P1a does not edit `school_session.rs`. `shared_sign_in` is withdrawn, and so is r5's `school_sign_in` enum. The row keeps `sign_in_hosts`: the hosts the fill may fill on (§7.5), set on the founder's row from R0a's record, empty elsewhere. A `CAMPUSES` test refuses a host with a scheme, a path or a port. The row also gains `wrong_password_text`: the school's own "wrong password" message, as fixed text the fill may match (§7.5). It ships empty on every row, the founder's included, unless the text comes from the school's or the identity provider's public documentation, cited by date in the row (review r8, Important 3). Empty means every ending after the submit is `Rejected(Unconfirmed)`, which is safe. No wrong password is sent to learn it. A `CAMPUSES` test refuses any `wrong_password_text` holding an `@` or a token of digits only, so no username, address or id can reach the public row |
| S3 | The registrar leaves tests 1, 3, 5 and 8; the fake-agent host pairs drop it. Test 1 drops the school mailbox. Test 4's shared-folder navigation list and its fallback go, and so does test 7: the mail lane's own tests stand unedited. Test 4 gains one named exception: P2's fill, in the LMS folder's hidden window, on a `sign_in_hosts` host only |
| S4b | `lms_link.rs` only. `registrar.rs` stays on its incognito throwaway window, so "the registrar never shares this profile" stays true |
| S4d | Replaced by P3. Every plan row that names S4d now names P3 |
| S5 | Also deletes the saved school password, `knowlu/<profile_id>/school`: at sign-out (RQ17, ruled yes) and in `delete_local_data`'s source list beside `zybooks` and `vhl`. It uses `credentials::delete` directly, as `delete_local_data` already does, so it needs nothing from P1. The marks need no delete: with no password no fill runs, and the next save clears them (P1t's order). Tests in `app/tests/account.rs` under its `CREDMAN_LOCK`, with generated ids and `Drop` cleanup. `contract-engineer`, as before |
| S6t | Gains (review r6, Important 4): `school_forget` calls P1's `forget` before it deletes the folder, and a failure of either is reported. After Forget, the next grades slot finds no saved password and fills nothing. It now follows P1t, which defines `forget`'s seam (review r7, Minor 3) |
| S6a | `school_forget` calls P1's `forget` (the password and the rejected mark) before it forgets the folder, so a folder-delete failure never leaves a password that signs back in. It now follows P1a, which holds `forget` |
| S6r | Also puts P1's save command in the console's `generate_handler!` list, with a pin that it is there; recount (review r6, Minor 2). It now follows P1 |
| S6b | The row reads RQ18's ruled words, "School sign-in: kept on this computer, used for grades and your course list"; the mailbox keeps its own row. It gains "Update school password", which P4 builds. Forget deletes the folder and the saved password |
| S7 | The page carries RQ6's approved text, with RQ14's clause, and one plain line that the school password is kept on this computer (RQ16). That line's words do not hold the feature. The text is true only with C2e, C5d and F2c merged, so S7's page waits for all three |
| S8 | The plan's §9 row A6 is withdrawn. Rows A2 and A4 are withdrawn too: with two folders the mailbox keeps one profile per mailbox (RQ4). Row A1 is rewritten for two folders (review r6, Minor 5): "one kept LMS folder per profile, used by grades and the course list; the mailbox keeps its own; the registrar keeps its throwaway window". A3 names the saved school password. A5's Forget also deletes the saved school password (review r7, Minor 5: A5 names no registrar, so it has nothing to lose) |
| S10 | The registrar read and the shared mailbox leave it. P5 holds the prompt count |
| F8b | Capture half only: one hidden window through the link does the feed, the course list and the grades before it closes, because the session dies with the last window on the folder (§7.3). No `registrar_rows`. **Its S4d dependency becomes P3**, and F8b2, F8b2-advance, F8g, F3z, F8lb and C5c wait with it |
| F8b2 | The capture files it fences are `onboarding-class-search.json`, not the registrar file |
| F8w1 | No `registrar.confirm` arm, so no interim `registrar_runner_pending`. It gains the two class-search arms, so it lands after C5c |
| F11e1 | Step 3's sign-in follows P4 |
| F11e2 | The proof's course list marks a course that was not found in class search with a "doesn't look like a class" hint. The student still decides (§6.4). The hint's data comes from C5c, so it lands after C5c |
| F11b | Step 5 follows P4: the mailbox's own sign-in on its own folder |
| F12 | The walk-through drops its registrar fixture and adds class-search rows, and one run with no class search at all |
| F13 | The plan's §9 rows change as C9 lists them |
| F15 | `finish_ready: true` on the build to merge, once with class-search rows and once with class search failing or absent, and never by a registrar read. Its F8h-runner dependency becomes C5c, and it gains T3b |

The plan's §8 O3-to-`main` checklist changes with F15: its `registrar_runner_pending` search and its "registrar
gave class times" item go. In their place: `app/src` holds no `registrar.confirm`, and F15's two runs
passed.

**Kept, not altered.** S4a, S9, F2c, F3z, F4c, F8la, F8lb, F8b2-advance, F8g,
F8w2, F11d, F11f and F14. Every merged task stands. F7a's `--registrar` input stays in the engine: it
is small, tested and documented, and it does no harm unused. S4a and S4b keep their scope and land
after C8 (§4.2's file table).

### 4.6 The timing test moves (Quinn, 2026-11-04)

S-E and S-F's live half run **after the rewrite**, as T3b, on the chosen flow and before F15. Timing
the old flow would measure a sign-in path that no longer exists. Moving them changes four things:

- **T3 cannot move whole.** S4a and S4b depend on T3 for S-G's result, so S-G stays early as T3a.
- **S-F rides with S-E.** The plan runs them in one session and S-F's check needs a first full slot.
  This is the planner's call, not Quinn's words. The cost is that a thin step-3 proof shows up late.
- **S-E's gates become checks on built code.** Its "before F8f" gate is past, since F8f is merged. A
  stage over 60 seconds, or a wait past Q7's 3 minutes, now goes to Quinn as a change to built work.
- **Every number here is provisional until T3b:** the class-search budget (§6.8), and the copy's minutes.

T3b also records class search's seconds, and the step-3 wait it adds.

### 4.7 Order

1. **At once, in parallel.** R0b, and R0c already running. C0, C5a and C7, then C8 and C5b. The
   `section_in_name` tests. T3a on R0a's build. `parse_class_search`'s tests wait for R0b's keys.
2. **The engine chain.** C1, C2a, C2b, then C2c, C2d and C2e in that file order, then C3. C4 runs beside
   C2a once C1 and C0 are in. C10 follows C2a.
3. **The LMS folder and the password.** S1, then S2 once C8's row and C4's row are in, then S3, then S4a
   and S4b. P1t once S2 is in, then P1, then P2 once C8 is in, then P3.
4. **The queue.** After P3, the plan's `onboarding_jobs.rs` chain: F8b, F8b2, F8b2-advance, F8g,
   F3z, F8lb, then C5c in F8h-runner's old place, then F8w1 and F8w2. Then `main.rs`'s chain: F4c, F5b, S6r.
   Then C5d.
5. **The screens.** P4t's pins first. Then, in the panel merge order: F11b and F11e1 with P4 inside them
   once R0c is in, F11e2, then C6. P4's Settings button follows S6b.
6. **The O4 tail.** S7 once C2e, C5d and F2c are in, then S8 and S9.
7. **Reviews and live.** C9, F12, F13, T3b, then F14, then P5, then the plan's §8 checklist. P5 holds S10
   and F15's checks, and F15 follows F14, so the review comes first. **No wave-B release before S7's page
   is deployed**, as the plan already rules.

**The order has no cycle.** Each row below depends only on rows above it, on earlier tasks in its own row,
or on merged work. The edges are §4.1 to §4.5's, plus every row of the plan's §7 table: its dependency
column and the file chains its rows name. S4d reads as P3, and F8h-runner as C5c.

**The table is complete as far as this branch can show.** Each task in the plan's §7 table that is not
below was read and is either merged or waits on no task below. Merge state comes from this branch's code
(§4.5's caveat), and the controller re-checks it before the first dispatch. The rows marked "if open" are
the ones the code leaves unclear: F5c's names are on `main.rs`'s list, F8lb's two commands are not, and
`Queue::start` is not there at all.

| Layer | Tasks | Waiting on, from the layers above |
|---|---|---|
| 1 | R0a (done), R0b, C0, C5a, C7 | nothing |
| 2 | T3a, R0c, C1, C8, C5b | R0a; R0b; C0; C5a |
| 3 | C2a, C4 | C1; C0, R0b |
| 4 | C2b, C10 | C2a |
| 5 | C2c, C2d, C2e, C3 | C1, C2a, C2b |
| 6 | S1, then S2, S3, S5 | RQ4 and RQ17 (ruled); S2 also C8's row and C4's row |
| 7 | S4a, S4b, F2c, P1t | S3, T3a, C8; F2e, S5; S2 |
| 8 | F8la, S6t, P1a, then P1b, S6a, S6b | S4b; S4a and S4b; S6t also P1t (review r7, Minor 3); P1t; P1b also P1a; S6a also P1a; S6b also S5 and RQ18 (ruled) |
| 9 | P2a, then P2b | P1t, C8; P2b also P2a and P1b |
| 10 | P3 | P2, S4a, S4b |
| 11 | F8b, F8b2, F8b2-advance, F8g, F3z, F8lb, then F5 if open for F8lb's two commands | P3, F8la, F2b2b, F2b4 |
| 12 | C5c, F8w1, F8w2, F4c, then F5b if open, S6r, C5d, and P4t | C3, C4, C5b, F8b, F8b2, F8lb; F4, F4b, F5, F2b2b; F10, S6a, P1; C2b, F2e; P3 |
| 13 | F11d and F11c2 if open, then P4 inside F11b and F11e1 and after S6b, then F11e2, then C6 | P4t, P3, S6r, R0c, S6b, F8lb, F8g, F3z, C5c, C3 |
| 14 | S7, F7b if Q-B10 is yes, S8, S9 | C2e, C5d, F2c, P1 to P3, S4a, S4b, S5, S6b, S6r |
| 15 | C9, F12, F13, T3b | every C task; F11e2; C9; F4c, F8w2, C6 |
| 16 | F14, then P5, then the §8 checklist | F13, T3b, S9, P4, R0c |

Five edges were wrong in earlier rounds and are fixed above: S2 sat before C8's row while S1 to S4c
followed C8; C2c ran beside C2b while needing it; the live proof ran before F14, while it holds F15's
check and F15 follows F14; F4c was missing, so T3b read as ready with no queue started; and S6r sat before
F4c, against `main.rs`'s chain. r6 replaces A1 to B4 with P1t to P5 and drops S4c: P3 takes S4d's place
before F8b, and P4 sits inside F11b and F11e1, as A3 and B3 did. r7 adds P4t before P4, and the edges
P1 to S6a and P1 to S6r (review r6, Important 4 and Minor 2).

## 5. The registrar is optional

**What stays on `main` as a bonus:**
- `app/src/registrar.rs`, `commitments --registrar`, R24, and applied-and-listed, unchanged.
- `engine/src/registrar.rs` gains C1's new functions, and one new step at the start of `run_parsed`:
  adopting a class-search note (§6.6). Nothing else in a registrar read changes, and a vault with no
  `classsearch:` note reads as it does today. Existing registrar tests stay unedited.
- The Schedule view's and first-day screen's "Get my class times" button, where a curated row has a
  registrar. It keeps its own incognito window and its own sign-in.
- The registrar's top tier. A later press outranks class search and corrects it (§6.6).

**What onboarding stops requiring:**
- Step 3 never opens or reads the registrar. `school.capture` carries no registrar call.
- `registrar.confirm` leaves `JOB_ORDER`, `LANE`, `NOTE_JOBS`, `LANE_REQUIRED` and `inputs_ready`.
  `finish_refusal` no longer waits on it, so no registrar state can hold Open Knowlu.
- `class_times` in step 3's proof may be `null`, and the proof omits it. Step 10 still opens.
- S-A's pass no longer names the registrar. S-A phase 2's table row for it is history.
- F15 passes without any registrar read.

**A known gap, not fixed here.** The curated `start` page is public and never redirects, so the bonus
button needs the student to click through to the schedule before pressing "I'm signed in". Pointing
`start` past the menu is a one-line data change, and it is a follow-up (§9).

## 6. The public class-search reader

**What it is for.** A student at a curated school gets class times proposed at step 10 with no
registrar sign-in. **What it is not:** a general answer. Banner is about 24% of the North American
higher-ed student-system market (ListedTech, January 2025; an analyst blog, one secondary source, not
checked here). Whether guest class search is open also looks set per school. So most students will
never have this reader, and the calendar, syllabus and typing paths must stand on their own.

### 6.1 Where each part lives

| Part | Where | Why |
|---|---|---|
| Reading a course name; parsing rows | `engine/src/registrar.rs`, beside `parse_banner` | ruling R1-a keeps every Banner key in one place; pure, no network |
| The fetch | new `app/src/class_search.rs` | "the app carries the bytes", as with the registrar |
| The school's data | `Curated.class_search` in `scaffold.rs`; the term table in `registrar::SCHOOLS` | rule 1: a row, never a per-student setting |
| Writing proposals | `commitments --class-search` | one engine command, notes untouched |
| Confirming | today's `commitments --confirm` from step 10 | the student's press, journal first, no new write path |
| Deleting | `commitments::purge_class_search`, called in-process by the app | §6.12; works even when the sibling engine cannot start |

The fetch uses plain `ureq`, with no window and no capability. It sends no student cookie, because
nothing here needs a sign-in. A test pins that the reader never touches a window or a kept profile.

### 6.2 The fetch

Per run, in order:
1. `POST term/search?mode=search` with the term, once. Keep the one anonymous session cookie from the
   answer on the stack. No jar, and no change to `ureq`'s features.
2. For each distinct `(subject, number)`: `POST classSearch/resetDataForm`, then `GET
   searchResults/searchResults` with `txt_term`, `txt_subject`, `txt_courseNumber`. R0b records
   whether the reset is truly needed; it stays either way, since the sample only ran with it.
3. Write one bundle: `{school, term, queries: [{subject, number, sections, status, body}]}`, with
   `sections` the sections wanted and `body` the raw answer. No course name, no student id.

**Paging is unrecorded.** R0b checks a course with many sections. If answers are paged, C4 reads the
answer's count and follows pages for that one course only, at most 3. A course whose section could sit
on an unread page ends `not_found` only after the pages are read; otherwise it is `fetch_failed`.

### 6.3 How a course name becomes a query

`section_in_name(name, shape) -> Option<SectionRef{term, subject, number, section}>`, a pure function.
The shape is one named variant on the `School` row (`TermSubjectNumberSection` is the only one today).
A new school's naming is one new variant. Rules, in the style of `course_code_in_name`:
- Six-digit term, then subject (two to four capitals), three-digit number with an optional trailing
  letter, then a section of one to four letters or digits, dash-separated: `209940-XYZ-100-001`.
- The run must start the name and end at a boundary, so trailing words are allowed. A further dash
  group means a merged shell (`209940-XYZ-100-001-002`) and **does not fit**. It is never cut down.
- Never a guess: anything else is `None`. `suggest_course` and `course_code_in_name` are not used here.

The school's LMS names decide this, and LMS **codes** never carried the shape at the test account, so
only the name is read. Today's sample: 14 of 19 names fit.

### 6.4 What happens to each kind of course

Each kept course ends in exactly one state. The vocabulary is closed, and the preview counts by state.

| State | Meaning | What the student sees |
|---|---|---|
| `found` | one section matched and was read whole | a proposal row at step 10 |
| `no_time` | one section matched, listed with no day or time | "No set meeting time listed" at step 10, no proposal, no later ask (RQ9) |
| `part_read` | matched, but a meeting could not be read (`Reading::Partial`) | no proposal; the calendar and typing paths as today |
| `not_found` | the query answered, no such section | no proposal; a hint at step 3's list |
| `ambiguous` | two or more rows match subject, number and section | no proposal: never picked |
| `other_term` | the name's term is not the term being read | no query, no proposal |
| `no_fit` | the name does not fit the shape | no query; the calendar, syllabus and typing paths as today |
| `fetch_failed`, `paused`, `closed` | a network failure, a stop for politeness, guest search closed | no proposal; one named line at step 10; never a failed job; the calendar, syllabus and typing paths as today, with no Try again in wave B (§6.8) |

- **Sections with no time.** Many have none, and that is an answer, not a fault. `parse_banner`'s P4
  rule holds: nothing is filled in. A read of only no-time rows is not an "empty parse" failure,
  unlike the registrar's rule. An answer with no rows at all stays `not_found`. The record that stops
  the later ask is in §6.5.
- **Non-class shells.** A shell that fits the shape but has no section is `not_found`, and step 3's
  removable list shows the hint. Knowlu never removes a course on this evidence, since hiding a real
  class is worse than offering an old one (`lms_link.rs`'s rule). A shell that **does** match a real
  section, such as a dropped class still in the LMS, cannot be told apart. That is why §6.7 asks.
- **Wrong term.** The term read is `registrar::fetch_term`'s. A name with another term is
  `other_term`. A returned row whose `term` differs from the one asked, or whose `until` is before
  today, is dropped and counted. The summer gap in the `terms` table stays, as the phase-3 spec names
  it: a summer student gets `other_term`, not a wrong fall schedule.
- **Cross-listed sections.** Only the subject and number the name carries are asked about, so a
  cross-listed twin never appears. Two LMS courses naming one CRN are one series, and `by_key` keeps
  one record. A twin under a calendar series with the same meeting signature collapses, as today.
- **Linked sections (a lecture with a lab or recitation).** The name gives one section, and the linked
  one cannot be known from public data. It is never guessed. If R0b finds a link field, a `found` row
  whose section is linked carries `linked: true`, and step 10 shows "This class may also have a lab
  or recitation. Add it if so." A separate LMS shell for the lab is read as its own course.
- **Lab or class.** The title rule is `parse_banner`'s (P5). R0b records which of the kind fields this
  answer carries, since the registration call's `scheduleDescription` may not be there.
- **Reader-side checks.** A row is matched only when the returned subject, number and term equal the
  query's and `sequenceNumber` equals the section. The answer is never trusted to be filtered.

### 6.5 What a run reports

`commitments --class-search` prints the counts per state, `term`, and `warnings`. **It always exits 0
and writes only `state/calendar-series.json`**, so it fits `CLAUDE.md`'s `commitments` line and needs no
rule change. An unreadable bundle is a named line with nothing written. It holds the series lock, is
never gated, fetches nothing, makes no model call and writes no journal record.

`onboarding --preview --class-search <file>` fills the existing `class_times` key and adds
`class_search: {found, no_time, ...}`. It never writes, reaches no network and exits 0.

**The no-time record (RQ9's data path).** A `no_time` section makes no series, and the bundle is
deleted, so the fact needs a home of its own. It is a new top-level block in the series file, beside
`registrar`, absent until written:

```json
"class_search": {"classsearch:<key>:<term>": {"no_time": ["<course_key>", "..."]}}
```

- It holds course keys only, sorted: no section, CRN, time or room. The vault already knows these courses.
- `uncovered_courses` gains one input, the set of keys `no_time_courses(file, today)` returns, and treats
  them as covered. Its three callers (`emit_asks`, `overview`, `cli.rs`) pass it. So no new ask is filed.
- **Accepted limit, no `approvals.rs` edit in wave B.** The block stops new asks only. `ask_withdrawal_reason`
  (`:3623`) has no no-time input, so an ask filed before the block was written stands until the student
  answers or rejects it. That needs a setup still open on day 3, and rejecting it closes the course.
- `Overview` gains `no_time_courses`, which step 10 reads for "No set meeting time listed". Step 10
  never says "Knowlu will ask this week" about these courses.
- `no_time_courses` ignores a block whose term is not `registrar::term_for(school, today)`, so a new term
  asks again.
- `refresh_series_held` rebuilds `SeriesFile` by hand (`commitments.rs:2446`), so it must carry the
  block. The block is not tied to `calendars`, because purge removes that entry while the block stays.
- **A block's term has passed** only when it sorts strictly before `term_for(school, today)`, by year and
  then by position in `School::terms`. If `term_for` gives `None`, or the block's term cannot be placed,
  it has not passed. So a future block, which a re-setup can write, is ignored but kept. Every drop in this
  plan uses this one test.
- **The first `rank` of a later term drops a passed block**, and stores the file for that alone. Today the
  file is stored only with something `fresh` (`:2447`), so a student with no calendar feed would keep the
  block for good. The lock is taken only then too (`:2299-2302`), and an unlocked write could rename a stale
  copy over a locked registrar write and lose `applied` and `ended` history. So `refresh_series_with` reads
  first with no lock. If a drop is due, it takes the series lock, and `refresh_series_held` re-reads under
  it and writes. With nothing due it takes no lock and writes what it writes today. `refresh_series_held`
  stores for a drop only when it holds the lock. After a failed lock (`:2313-2317`), or when the first read
  missed the drop, it stores nothing and the block waits for the next `rank`. `rank` calls it on every run
  (`cli.rs:837`). This is the record's ruled lifetime, decided by the date, and reads no credential.
- **Accepted limit.** A clock set ahead past the term drops the block for real, and setting it back does
  not restore it. Those courses get the later ask. Nothing the student typed is lost.
- A registrar read leaves it alone. The registrar may list no time either, and the block's only job is
  to stop an ask, so it stays until the term changes.
- A confirmed note or an answered ask needs nothing from it. Purge's two scopes treat it differently:
  it survives setup end and goes at sign-out (§6.12, RQ12). No backup holds it (§6.12).

### 6.6 Class search's own source

**Ruled order (RQ1): registrar, class search, syllabus, Google, the rest.** `SOURCES` gains a row at
tier 1, `google:` becomes 2 and the rest 3. L5 has not landed, so its `syllabus:` row becomes tier 2 and
those below it move down one: a spec E4 amendment (C9).

**r1's shared `registrar:` key is withdrawn.** It did not survive review findings 4 and 5:
- A confirmed note built a hold from its `registrar:` uid, while `proposals` tests the hold by calendar.
  So Mine on lecture 001 would make recitation 101 vanish with no card or ask, and would hold every
  Google `class` of that course.
- `registrar:` would no longer mean the registrar said so. Four places in `console.js` would label a
  guess with the registrar's name, which breaks VISION commitment 5.
- A registrar read that omits an unconfirmed row would leave it standing, because that clean-up reads
  `registrar:` calendars only.

**A row carries `source_uid` `classsearch:<key>:<term>-<crn>`, under calendar
`classsearch:<key>:<term>`.** The `SOURCES` row sits before the catch-all rows:

| Column | Value | Why |
|---|---|---|
| `calendar` | `Some("classsearch:")` | its own tier and keep-alive |
| `uid` | `Some("classsearch:")` | `None` is the catch-all in `by_uid`, so the row must name its prefix |
| `tier` | 1 | RQ1 |
| `keep_alive` | a new arm, `KeepAlive::PerCourse` | a read covers only the courses asked about, so an omitted row stays until purged or until its `until` passes. It needs two changes in `refresh_series_held`, not one (below) |
| `instances` | false | a weekly rule only |
| `holds_per_kind` | false | a class-search row hides nothing, because it may not be the student's class. It can still be held, so a registrar series or note holds it |
| `lands` | `Lands::NotWatched` | a row leaving the file never makes an ended entry or a change card, so class search can never change a confirmed note |

**What the prefix settles.** `fromRegistrar`, `paintRegistrarAdded`, `registrar_proposals`, `RegistrarHold`
and `card_reason` test `registrar:`. `withdrawal_reason` and the settle pass read the row's `lands`, and
`detect_changes` watches only notes whose `lands` is `Carded`, so a class-search **note** is never
watched. None of them acts on a class-search key, and C2a pins each one. A confirmed class-search note
holds nothing: an equal-signature row still closes, as for any confirmed note, and a recitation or a
differently timed Google row stays proposed.

**Changes the prefix does need.**
- `classify` takes the registrar's no-eligibility branch for this calendar, because a series with no
  instances would otherwise fail `eligible`.
- `create`'s body gains an arm: "Found in your school's public class search."
- `asks_wait_for_registrar` does not count this calendar.
- `refresh_series_held`, for `PerCourse`: `configured` is true while the calendar holds an open row, which
  covers the `None` branch. A fresh read that omits a row takes the `Some(_)` branch instead, where an old
  `last_seen` would reach `aged_out` at `:2383`. So a new arm before `recent` keeps the row while its
  `until` is open and drops it silently afterwards. On the `None` branch, a row past its `until` stays
  until `UNSEEN_DAYS` and is then dropped silently, which is harmless because no read is coming. A
  class-search row therefore never reaches `aged_out`, and `ended` never gains a `classsearch:` key.
- `detect_changes` leaves `classsearch:` rows out of `live` (`:4052`). Otherwise an open row could be
  picked as a Google note's successor, and a change card would carry a section (`:4095-4120`).
- `emit_checks_locked`'s `asked` filter (`:3994`) drops class-search rows (§6.7).

`Proposal` needs no new field: the key names the source, and the pages test it as `fromRegistrar` does. An
engine helper, `is_class_search(key)`, is the one test every guard uses.

**When a later registrar read meets a class-search note.** The registrar's answer is the whole term and
outranks class search, so it **adopts** the note. A new first step in `run_parsed`, after the parse
succeeded and under the series lock it already holds, before `confirmed` is built, for the fetched term:
1. Each confirmed note keyed `classsearch:<school>:<term>-<crn>` is re-keyed to
   `registrar:<school>:<term>-<crn>`: one `write` of `source_uid`, as `agent:knowlu.registrar`, journal
   first. `write_literals` takes any named field, and this is single-line surgery.
2. The note is then the registrar's, and today's rules run unchanged. CRN present and read whole:
   changed `meets`, `until` and `where` are applied, and hand-set fields are kept. CRN absent: ended and
   listed, which is right, because the student is not registered in it. R24 makes the real section's note.
3. That term's unconfirmed `classsearch:` rows go, in the merge's own series-file write: when `news` is
   `Some`, `refresh_series_held` discards them and their `calendars` entry without touching `aged_out`.
   The registrar has answered for the term, so a row it omitted must not linger. The no-time block stays
   (§6.5).
4. A re-key that fails is a named warning, its note keeps its old key, and R24 skips that CRN, so one
   section never gets two notes. The next read tries again.
5. An empty answer is still `Err` and writes nothing. A note from another term is left alone.

A `registrar:` key on a note therefore always means the registrar read that section. The note's body
keeps its "public class search" line, since no note body is rewritten. Without adoption, differing
times would leave two hard notes for one class, because R24's guard closes equal signatures only.

### 6.7 How a row becomes a confirmed commitment

**The rule (RQ2, kept strict): no row reaches a confirmed commitment without a press on that row, on any
screen.**

1. `class_times.apply` runs `commitments --class-search` after `school.courses`, so the code table
   holds the student's courses. Only `found` rows that name a kept course become series.
2. Step 10 shows them in their own group, with words that say where they came from, each with Mine,
   Not mine and the firmness control F11p3 built.
3. The student's press writes through `commitments --confirm`: the student's token, journal first.
   A firmness press counts as a press on that row, as it does for every row at step 10.
4. **No marker ever carries a `classsearch:` key.** A decline marker syncs, and it would carry a section
   off the device. The guard is in `create_marker_as` (`:4359`), the one place every marker is written, so
   it covers Not mine and a rejected card's `settle_rejected` alike. It returns `Ok(None)` for such a key.
   - **Not mine on a class-search row** removes that row from the series file at once, under the lock,
     and `twin_keys` marks its Google or other twins as today. A same-signature Google twin is hidden
     behind the row, so this is how it gets declined.
   - **Not mine on a Google row** is possible only for a by-title twin, because a same-signature key is
     collapsed away and reports stale at `:5170`. It marks the Google key and removes the row.
   - **A rejected card** whose twin is a class-search row writes no marker for it. The row stays until
     the purge.

   No re-read brings a row back, because each job runs once per school answer (§6.8).
5. **A row left blank is deleted at setup end** (§6.12). If its signature had collapsed a Google row,
   that row comes back as a calendar proposal. On the first-day screen it then wears `setupRow`'s Mine
   preset, as every Google class row does today. That is a Google row with its own source and not a
   class-search row, so this plan leaves it alone. Otherwise the course falls to today's `commitment-ask`
   from day 3. The student must type the time the row already had, and step 10 says so (RQ11).

**Every path, checked against the code:**

| Path | Why no row passes without a press |
|---|---|
| Step 10 | `propHtml` presets nothing, and `confirmArg10` sends only rows pressed Mine or given a firmness |
| First-day screen | `setupRow` presets class rows Mine, and Finish writes them. Purge empties these rows first, and **C10 removes the preset for `classsearch:` rows anyway**, so the rule holds if a purge failed |
| Schedule's registrar list | `registrar_proposals` and `fromRegistrar` match `registrar:` only |
| Proposal cards | `commitment_passes` builds one list (`cli.rs:869`) for `withdraw_asks`, `emit_checks_locked` and `emit_asks`. The list keeps class-search rows, and `emit_checks_locked`'s `asked` filter drops them, so no `commitment-check` is ever filed for one |
| Change cards | `detect_changes` leaves `classsearch:` rows out of `live`, so none can be a successor |
| Rejected cards | `settle_rejected` reaches `create_marker`, and `create_marker_as`'s guard refuses the key |
| R24 | `run_parsed`'s `mine` is built from the fetch's own `registrar:` rows |
| Adoption | re-keys notes that are already confirmed, and creates none |
| Bulk | no approve-all exists in the app. RQ2's "these are all mine" press is withdrawn: one press per row |

**Why the filter sits in `emit_checks_locked`.** `uncovered_courses` counts coverage only from the list it
is given (`:892`), so removing rows from `cli.rs`'s one list would make an ask fire beside a row. Every
production proposal card passes that one filter, so `cli.rs` needs no edit. `proposals`' `for_cards`
argument stays as it is.

**The cap.** Class-search rows never become cards, at step 10 or after, so they never charge the
15-a-day cap. While a `class` row exists it covers its course, so `emit_asks` files no ask beside it and
`withdraw_asks` withdraws one that stood. A course whose row was left blank and purged gets a
`commitment-ask`, and that does charge the cap, exactly as it does today.

**Accepted limit, no `approvals.rs` edit in wave B.** A `lab` row covers nothing, because `:892` and `:3631`
count `class` only, as they do for every source. So a course whose only row is a lab can get an ask beside
it. The ask is the same one any lab-only course gets today.

Confirmed notes sync. The series
file, the bundle and the no-time block never leave the device. The sync tripwires read only `sync.rs`,
`sync_rows.ts` and migration text, so they prove nothing about this. C2d's tests do.

### 6.8 Rate limits and politeness

- **One query per distinct `(subject, number)`**, with the reset before it. Two sections of one course
  share a query.
- **No bulk download, by construction.** The query type cannot be built without a subject and a number,
  and a test pins that. The term's whole list is never asked for.
- **Serial, with a gap** of at least 1 second between requests. No parallel requests, from one device.
- **Caps:** 20 distinct queries and 60 seconds a run. Past either, the rest are `paused`. The cap
  exists to stop a bug or an odd enrolment; the test account's 15 current-term shells fit under it.
  T3b sets the final numbers.
- **Stop at the first refusal.** A 429, a 403 or a 5xx ends the run: the rest are `paused`, with no
  retry and no backoff loop. An answer that is not JSON, or a redirect to a sign-in, ends it as `closed`.
- **Honest identity:** a `user-agent` naming Knowlu, its version and `knowlu.com`.
- **Once per school answer, with no Try again in wave B.** Per-course state lives only in the bundle,
  which `class_times.apply` deletes, so a re-ask would have no record of what was answered and a rejected
  row would come back. Both jobs go through `enqueue_once`, and neither ever ends failed. The queue's
  `job_crashed` retry re-runs only a job that never finished. `change_school` starts a new read, after
  purging the old school's rows: it clears `school.class_search`'s and `class_times.apply`'s records
  beside the three it clears today, or `enqueue_once` would run neither. A course left `fetch_failed` or
  `paused` takes the other paths (§9).
- **Each student's own computer.** The cloud never relays or caches it, so no shared list of a school's
  sections is built.

**A risk this leaves.** Removing a school means a new release, because the row ships in the app. A
remote off-switch is a later question if a school ever objects.

### 6.9 After onboarding

Wave B reads class search during setup and nowhere else: no slot step, no scheduler entry (RQ8). A
confirmed time can go stale if a section moves mid-term, and a registrar press corrects it where the
school has one (§6.6). A new term needs the student's calendar, a syllabus or the registrar button.

### 6.10 How a school opts in (rule 1)

`Curated.class_search: Option<ClassSearch>`, `None` on every row by default.

```rust
pub struct ClassSearch {
    pub school: &'static str,  // the key in registrar::SCHOOLS, which holds the term table and name shape
    pub host: &'static str,
    pub prefix: &'static str,
    pub checked: &'static str, // YYYY-MM-DD: the curation check below passed on this date
}
```

It stands apart from `Registrar`, so a school can have class search with no registrar row. A class-search
school needs a `SCHOOLS` entry with `registered: &[]`.

**Guest search being open is a per-school fact, so it is curated, never detected.** Knowing a school
runs Banner proves nothing. There is no runtime probing of uncurated schools, and no inference from
`lms_kind` or the mail provider. A row reaches `main` only after the **curation check**, run by the
controller, with no sign-in, and recorded by date in the row:
1. The term POST and one search answer JSON with no sign-in.
2. A sample of sections, found by subject, number and section, each match one row.
3. A sample of that school's real LMS names, or its published naming, fit a shape on the `School` row.
4. The school's `robots.txt` and any posted terms for the page were read, dated in the row, and the
   question added to the lawyer's list (RQ7).
5. Where the row also sets `lms_sign_in`: a plain anonymous GET under §7.1's rule finds exactly one
   link. This check applies to any school with that field, with or without class search.

A test over `CAMPUSES` pins the structure offline: `school` is a `SCHOOLS` key, `checked` is a date,
the host carries no scheme or path, `prefix` starts and ends with `/`, and `lms_sign_in` starts with `/`.
No test reaches the school.
If a school closes guest search later, the run reads `closed`, and setup goes on without class times.
Adding a school is one row: no vault, no student and no credential is involved.

### 6.11 The queue

Two jobs replace `registrar.confirm`:
- **`school.class_search`**, serial, in `UNGATED` (public data, before payment), after `school.capture`
  and before `school.preview`, so the proof can count class times. It runs only when the row has
  `class_search`, and it writes the bundle to `onboarding-class-search.json` through F8b2's fenced
  writer. It always ends `Done`: any failure is a state in the bundle, never a failed job.
- **`class_times.apply`**, local lane, after `school.apply` and `school.courses`. It runs the command,
  deletes the bundle, and always ends `Done`, with a named note when the engine could not read it.
  It is not in `LANE_REQUIRED`, so it cannot refuse Open Knowlu.

The student's `onboarding.json` state version stays 1: no released build has written
`registrar.confirm`.

**Step 10 must see the rows.** `askProposals` latches once, on `proposals_ready`, which today means
`config.ingest` is done, and nothing orders `class_times.apply` against it. So:
- `proposals_ready` also requires `class_times.apply` to be done, or to have no record because the row
  has no `class_search`. A job stuck behind a paused or failed `school.courses` does not hold it, so
  step 10 still opens with its calendar rows.
- The latch then normally fires once, after both, so nothing repaints.
- **A late finish.** If `class_times.apply` ends after the latch, because the queue's own Try again
  freed a stuck `school.courses`, `onStatus10` adds only the rows not already on the page and leaves every
  existing row alone, so no pressed answer is lost. `console.js`'s `reloadSetupRows` is the pattern:
  answers go back by key.
- `onStatus10` moves its `loadSet10` reload from `registrar.confirm` to `class_times.apply`.
- `splitProposals10` puts `classsearch:` rows in their own group, because "Your calendar repeats
  these" is wrong for them.

### 6.12 What is deleted, and when (RQ11, ruled)

The approved text (RQ6, with RQ14's clause) says the results "are deleted when setup finishes or you sign
out", except a list of which classes show no meeting time. Deleting the bundle alone would not make that
true, because `found` rows sit in the series file. So unconfirmed rows are purged.

| Data | Where | When it goes |
|---|---|---|
| The raw bundle | `onboarding-class-search.json`, app data | `class_times.apply`; also `change_school`, finish, sign-out, and the launch sweep on its two deleting rows |
| Unconfirmed rows: every `classsearch:` series and its `calendars` entry | series file | setup end, sign-out, `change_school`, the launch sweep on its two deleting rows, a registrar read of that term; one row at once on Not mine |
| The no-time block | series file | sign-out, `change_school`, the launch sweep on a confirmed signed out, and the first `rank` after the term changes (§6.5). It outlives setup end (RQ12) |
| Confirmed notes | `commitments/` | never. They are the student's plan, and they sync, as the text says |

**The operation.** `commitments::purge_class_search(vault, scope, today)`, with scope `Rows` or `All`, `All`
adding the no-time block. All four callers pass `today`, so the term test reads no clock in the engine. It
reads the file first, with no lock. A clean file means no lock, no `state/` folder
created and no write. A dirty file takes the series lock, **re-reads the file under it**, and deletes those
entries from that fresh copy, directly, never through `aged_out`. The first read only decides whether to
lock, so nothing another writer stored in between is lost. It also drops a block whose term has passed
(§6.5's test), and any `ended` key under `classsearch:` as a belt, though §6.6 means none should exist. It
writes only the series file, and only when something changed. It touches no note, writes no journal record, and needs no sign-in, entitlement or network. The
app calls it in-process, like `commitments::overview`.

**The callers (C5d).**
- **Setup end:** `Rows` and the bundle delete, through `ExitSeams`, after the drain and before `write_finish`.
- **Sign-out:** `All` and the bundle delete, inside `on_sign_out_with`, whether or not the file is
  finished, as one more step whose failure is collected. `Settings` has no vault field, so `on_sign_out`
  finds the vault in `profiles::load`, by `profile_id`, with the root taken from `data_dir`. It keeps its
  signature, so `account.rs` and F2c stay as they are. No such row, or an unreadable registry, is a
  collected failure. The text is true only once F2c has wired it.
- **`change_school`:** `All` and the bundle delete, since both belong to the old school. It takes no vault
  today, so it gains a purge input, the way `on_sign_out_with` has one, and F8b2's `Queue::set_school`
  passes the real purge. Its failure is a named line, as below.
- **Launch sweep:** below.

**The launch sweep.** Sign-out relaunches `--pick`, which opens no profile, and `main.rs` has two `.setup`
blocks, so a retry inside either would miss cases. So `onboarding_state::launch_sweep(root, seams)` runs
once in `main`, after `app_data_root` at `:58` and before the dispatch at `:70-72`. Neither `.setup` block
changes. Its job is to finish a sign-out or a Finish that could not finish itself.

> **The fail-safe rule.** The sweep deletes only on a fact it has read, never on a failure to read one.
> Anything short of a confirmed "signed out" runs nothing destructive: no purge of either scope, and no
> bundle delete. Unknown and every error mean the same thing, which is do nothing. **One exception, by
> Quinn's ruling (RQ15):** a confirmed signed in whose setup reads as finished gets what Finish already
> owed, the bundle delete and `Rows`, and never `All`.

For each profile in `profiles::load(root)`, a pure `session_answer`, over an injected credential read,
gives one of three answers:

| Answer | What the sweep read | What runs |
|---|---|---|
| Confirmed signed out | `account::cloud_config(&p.vault)` reads, its `session_credential_target` is not empty, `read_credential` of that target returns `NotFound { code: 1168, .. }`, Windows' `ERROR_NOT_FOUND`, **and** `read_credential(&account::session_target(&p.id))` returns the same code | the bundle delete, then `All`, whatever the setup state |
| Confirmed signed in, setup finished | the same target, `read_credential` returns the credential or `CredError::Blob`, which means it exists, and `onboarding.json` reads `Loaded::Ok` with `finished` true | the bundle delete, then `Rows`. Never `All`, so a current no-time block stays |
| Confirmed signed in, anything else | the same credential read, with setup unfinished, `Loaded::Absent` or `Loaded::Unreadable` | nothing |
| Unknown | any other code, such as 1312; a 1168 on `cloud.yaml`'s target with `session_target(id)` holding a session, or reading as anything but 1168; any `cloud_config` error, whether unreadable, malformed or a foreign base; an empty target; no `cloud.yaml` | nothing |

- **Why 1168 alone.** `wincred.rs:158-163` returns `NotFound { code }` for every `CredReadW` failure, so
  the variant by itself says nothing. Code 1312, for a signed-in student at step 10, would otherwise delete
  rows that `enqueue_once` never re-reads.
- **Why the vault's own target.** The app reads the target in `config/cloud.yaml` (`account.rs:863`, `:997`),
  written once (`scaffold.rs:701`). A registry id derives from the path (`profiles.rs:24`, `:69`) and
  `retarget_credentials` does not move the session, so after a moved folder `session_target(id)` names a
  credential that never existed. The sweep reads the target the app's own reads use. Sign-out and attach
  do not, as the limits below say.
- **No `cloud.yaml` is Unknown, with no fallback to `session_target(id)`.** The derived id is the thing
  shown to be unreliable, and such a vault has no rows to lose: the account comes before step 3, and
  `cloud.yaml` is written with it.
- **It reads `onboarding.json` only on a confirmed signed in.** A confirmed signed out and an Unknown never
  open it. Only `finished` true moves the sweep, so `Loaded::Unreadable` and `Loaded::Absent` run nothing.
- **Why the exception is safe.** It needs two facts read, and removes only what Finish already owed. A
  student at step 10 is not `finished`, so it cannot reach them. `Rows` leaves a current no-time block.
- **Unknown only delays.** The next launch asks again. An unreadable registry is skipped with a named line.
- **It runs before the single-instance plugin**, so a second launch can reach it beside a running app. A
  clean profile costs one read and creates nothing. A dirty one takes the series lock with
  `File::try_lock`, and a busy lock skips that profile until the next launch. Finish, sign-out and
  `change_school` keep the blocking lock.
- **Accepted limit.** A Credential Manager cleared by hand reads as 1168, to the app and the sweep alike,
  so the two agree. The rows go without the sign-out's rewind of the jobs, and those courses fall to the
  later ask.
- **Accepted limit: sign-out on a moved vault.** Sign-out deletes `session_target(id)` (`account.rs:759-775`),
  not `cloud.yaml`'s target, and on a moved vault the two differ. The old credential survives, so the sweep
  reads confirmed signed in. A sign-out purge that failed there gets no retry of `All`, which fails safe.
- **Accepted limit: attach on a moved vault.** `attach_config_and_session` keeps an existing `cloud.yaml`
  (`:1088-1089`) and puts the new session at `session_target(profile_id)`. So the file can name a target
  that no longer exists while the student is signed in under the new one. The table's second read catches
  this: the answer is Unknown and nothing runs. That read can only stop a delete, never allow one, so it is
  no fallback to the derived id. The cost is that such a vault reads as Unknown at every launch while signed
  in, so a failed Finish purge there gets no retry. The real fix is in §9.

**Failure.** A purge or a bundle delete that cannot run is a named line and never blocks Finish, sign-out
or `change_school`. The first-day screen still cannot write a row with no press (C10). After a failed
sign-out purge the session is gone, so the next launch reads a confirmed signed out and retries, except on
a moved vault, as above. After a failed Finish purge, the next launch retries on the exception row. An
abandoned setup keeps its rows until the student finishes or signs out, which is what the text says.

**Backups.** `BACKUP_FOLDERS` includes `state`, and each snapshot zip is kept for 30 days. A normal setup has
no backup folder (`onboarding.rs`, C1), but a profile that has one must not break the text. Skipping the
whole series file would not do: the no-time block outlives setup, and the file's `registrar` block is
history that cannot be regenerated. So backups take a **filtered copy**, and one helper in `backup.rs`
decides per path, for `mirror` and `snapshot` alike:

| Path | What happens |
|---|---|
| `state/calendar-series.json` | the bytes from `commitments::series_for_backup(vault)` |
| Any other file in `state/` whose name starts `calendar-series.json` | never copied, in either. `write_state_file` leaves `calendar-series.json.tmp<pid>` (`:2233`), and `TMP_MARK` is `.tmp-`, so that name does not catch it |
| Everything else | copied as today |

`series_for_backup` returns the file's **raw bytes** if they hold neither `classsearch` nor `class_search`,
malformed or not, so other students' backups are unchanged to the byte. `parse_file` (`:2121`) drops
unknown keys, so going through it for those files would not be. Otherwise it parses the file, removes every
`classsearch:` series, `calendars` entry and `ended` key and the `class_search` block, and returns
`file_bytes`. A file that holds that text and cannot be parsed, or cannot be read at all, gives `None`: it
is skipped for that run and still counted as wanted, so `mirror` never archives the older copy.

`same_bytes` (`backup.rs:43`) compares source to destination, which would recopy a filtered file on every
tick. For this path `mirror` compares the filtered bytes to the destination, so a second tick copies nothing.

**Logs and warnings.** `--class-search` reports counts and fixed words only. `confirm`'s five warnings print
the key (`:5131`, `:5151`, `:5156`, `:5161`, `:5182`), so for a class-search key they print "a class-search
row" in its place. No slot log or warning carries a section, a CRN or a time.

## 7. The LMS sign-in link

### 7.1 How the link is obtained

**The row names only a prefix. The link itself is read fresh, from the public login page.**
- `Curated.lms_sign_in: Option<&'static str>` holds the path prefix (`/auth-saml/saml/login`), `None`
  by default. It is per school because a school may use another sign-in, and no tenant is assumed.
- Each time a window needs it: one anonymous GET of `https://<lms_host>/`, with no cookie. Redirects are
  followed at most 3 times and only while they stay on exactly `lms_host`; any other hop gives `None`.
  Then a pure `lms_link::sign_in_link(html, lms_host, prefix) -> Option<String>`.
- The link is the same for every student at a school. It holds no user data and is never stored, so
  nothing per-student exists to hand-edit. A tenant changing its identity provider id breaks nothing.

**The rules of `sign_in_link`:** HTML entities in `href` decoded; absolute on exactly `lms_host`, or
root-relative; `https` only; the path starts with the prefix. Refused: another host, a look-alike
suffix, `//host`, userinfo (`@`), a backslash. **Exactly one distinct link passes, or the answer is
`None`.** Two is a page Knowlu does not understand, and it never guesses.

**Settled at the founder's school.** The link is in a plain cookie-less fetch, so there is no whole-link
fallback, and C8 waits on nothing. The redirect rule above stays for other schools, each of which proves
the link at curation (§6.10, item 5).

### 7.2 Where it is used

- **Hidden:** `grades::open_hidden` starts at the link, and at `https://<host>/ultra/` as today when the
  row has no prefix or the link was `None`. `read_hidden`'s ending changes (§7.4).
- **Visible:** a visible LMS sign-in opens at the link too, so the student skips the login page. With
  the saved password it is only the fallback: `CannotRead`, at setup or after it; at setup only,
  `Rejected(Unconfirmed)`; and a school row with no `sign_in_hosts` (§7.4). A recognised "wrong password"
  goes back to Knowlu's own form, never to a visible school page.
- **The fill:** when the hidden window at the link lands on the school's sign-in page, P2 fills it from
  the saved password (§7.5).
- **Everything else stands:** no capability, cookies read per host, and `users/me` read only on the LMS
  host. A test pins that the anonymous GET sends no `cookie` header.

### 7.3 One window's life

The LMS session dies when the last window on the folder closes, so F8b does the feed link, the course
list and the grades capture in **one** hidden window before it closes. After a close, the next open
waits the measured 250 ms (S-B's step 4), and `school_session`'s lock holds it for that long.

The sign-in behind the link lapses about 12 hours after it was made, used or not (§2.1). No window is
kept open between slots and nothing keeps it warm, because neither extends it. The next hidden read
after a lapse fills the page again (§7.4, §7.5).

### 7.4 When a hidden refresh fails

C8 builds the table below with no new state: today's signed-out path, reached from a new start page.
P3 then puts the fill in front of it (after the table).

**`reached` no longer tells anything.** The link is on the LMS host, so `read_hidden`'s `reached` goes true on
its first poll. A session that fails at the identity provider would then rest on a 401, and one transport
failure would read as `Unreachable` with no mark. So C8 replaces the deadline's test with a pure
`hidden_outcome(on_host_now, last)`, reading where the window is at the deadline, not where it has been:

| What happens | Named outcome | What follows |
|---|---|---|
| `users/me` answers 200 within `SETTLE` (45 s) | capture | the mark clears |
| At the deadline the window is **not** on the LMS host, whatever `last` holds | `SignedOut` | `grades-signed-out` set; row `grades (skipped: signed out)` |
| On the LMS host, and `last` is 401 or 403 | `SignedOut` | as above |
| On the LMS host, and any other `last`, or no answer | `Unreachable` | row `grades (skipped: Blackboard unreachable)`; no mark |
| The login page answers, but `sign_in_link` is `None` | falls back to `/ultra/`, then as above | a changed page reads as signed out, visibly |
| The login page cannot be fetched (a transport failure or a 5xx) | `Unreachable`, with no window opened | the LMS is down, so a window would cost 45 s and prove nothing |

An identity provider that is down reads as signed out. That is the accepted cost: the student sees the
sign-in row, and a press there shows what is wrong. A window that starts at `/ultra/` ends the same as
today under `hidden_outcome`, which C8's tests pin.

**With the saved password (P3, RQ3).** A hidden read whose window lands on a `sign_in_hosts` page fills
it (§7.5), silently, every time the session has lapsed. The fill's answer replaces the "not on the LMS
host" row:

**When a password counts as sent (review r7, Important 1; review r8, Important 4; review r10, Minor 2).**
Before the **first submit that carries a credential** runs, the fill writes an **in-flight mark** and
flushes it with `sync_all`, as `scaffold.rs` and `ledger.rs` already do. On a two-step page that is the
username step's submit, not the password's. **In this plan, "the submit", "the password's submit" and
"after the password" all mean that first credential-carrying submit.** An unknown page after the username
step is therefore `Rejected(Unconfirmed)`, and the saved password is deleted though it was never sent. That
is one more cost of the conservative rule. If the write or the flush fails, nothing is submitted, and the
attempt is `CannotRead`. Only `users/me` 200 in that same attempt, or a new save, clears the mark. Every
other ending counts as `Rejected`: a navigation that loses the script (`Lost`), a timeout, an LMS 5xx, a
network drop, the app closing, or the machine sleeping.

**When the marks are checked (review r8, Minors 2 and 3).** Both marks are read only after
`school_session`'s lock is taken. The in-flight mark is checked **only before a fill**, never at the
start of a read. A read whose session is still live after a crash therefore still reads grades. The
mark stays, because a live session does not prove the earlier submit worked. The next time a fill would
run, the fill finds the mark, treats it as `Rejected(Unconfirmed)`, and sends nothing.

`Rejected` has two kinds, which differ only in the words and in where setup goes next (below):
- **`Rejected(Wrong)`:** the school's own recognised "wrong password" message after the submit.
- **`Rejected(Unconfirmed)`:** any other ending after the submit, such as an MFA page, a "password
  expired" page, an unknown page, or any of the endings above.

| Fill outcome | Named outcome | What follows, after setup |
|---|---|---|
| Signed in: `users/me` answers 200 within `SUBMIT_SETTLE` | capture | `grades-signed-out` and the in-flight mark clear |
| `Rejected`, either kind, **or the rejected mark set with no password saved** | `SignedOut`, reason "password" | the rejected mark is set, then the saved password is deleted (§8, P1t's order); the row reads "Sign in to update grades" and opens Knowlu's own password form; **only a new save ends the pause**, and no visible sign-in does |
| `CannotRead`: an unrecognised page **before** the first credential-carrying submit, so nothing was sent | `SignedOut` | the password stays; the row's press and Settings open **the visible sign-in at the link**, not the form, because saving the same password again could not fix a page the fill cannot read (review r7, Important 2); the next slot tries the fill again |
| No saved password **and no rejected mark**, or the row has no `sign_in_hosts` | `SignedOut` | today's path: the named row and Settings' sign-in |

**The form's words follow the kind.** "Your school didn't accept that password" shows only for
`Rejected(Wrong)`. For `Rejected(Unconfirmed)` the form reads "Knowlu couldn't finish signing in with
your saved password. Type it again to keep signing in for you." Neither ever claims more than Knowlu saw.
The second half, "to keep signing in for you", is true only if R0c is clean. Under §2.3's MFA branch a
new save would meet MFA again, so P4 drops that half (review r8, Minor 4).

**`SUBMIT_SETTLE`** is the wait after each credential-carrying submit, separate from the 45 s `SETTLE`, so a slow
identity provider gets its own window. T3b sets it, and until then it equals `SETTLE`.

Always: the hidden window is destroyed and never shown. Knowlu sends no keyboard or mouse input; the
fill sets fields inside the page (§7.5). The identity provider is never hammered: at most one fill per
hidden read, and one fill at a time per profile. F8b's queue window and a slot's grades read can both
want the LMS, so the fill runs only while `school_session`'s lock is held, and it reads both marks and
the secret only after taking it. A second reader gets `Busy` (S3 test 5) and fills nothing, and a
`Rejected` from the first stops the next. **A save takes the same lock (review r8, Minor 1).** While a
fill holds it, the save answers `Busy` and writes nothing, and the page asks the student to try again
in a moment. So a save can never clear a mark that a running fill is about to set. The row stays exit 0, so the tray stays green. The student sees
the sign-in row only in the cases above.

**Inside onboarding: the setup split (review r7, Important 3; the coordinator's design, ruled: Quinn
confirmed it on 2026-11-07, "yes to all four", `docs/notes/2026-11-07-onboarding-rulings.md`).** RQ5 as ruled says any failure at setup shows the school's page. This split
narrows it in one case. Step 3 saves the password, then one hidden window at the link fills and reaches
`users/me` 200 (P3). If it does not:
- **`Rejected(Wrong)`.** The student goes back to Knowlu's own form at step 3, with one line: "Your school
  didn't accept that password". They type it again there, and that is the one ask. No school page is
  shown.
- **`Rejected(Unconfirmed)` and `CannotRead`.** The student is present at setup, so the school's own page
  opens in one visible window at the link, and the student finishes there. That covers an MFA prompt and
  an unknown page. Knowlu reads nothing the student types on that page. If the window is closed or fails,
  step 3's feed-link paste stands, with its named cost.
- **Knowlu never re-sends a password on its own.** After either kind of `Rejected`, the saved password is
  already deleted and the rejected mark set, as after setup. After `CannotRead`, the password was never
  sent, so it stays.
- **After a visible finish that followed `Rejected(Unconfirmed)`** there is no saved password, so step 3
  shows the form once more, with the `Unconfirmed` words and a "Not now" choice. This is the planner's
  recommendation inside the pending design. If the student picks "Not now", the first lapse shows "Sign
  in to update grades".

**Accepted costs of the conservative rule (review r7, Minor 1).** A correct password is deleted, and the
student types it again, whenever the submit ends any way but `users/me` 200. That includes a page the fill
does not know (such as a new terms screen), an LMS 5xx, a network drop, a slow identity provider past
`SUBMIT_SETTLE`, the app closing and the machine sleeping. That is the price of never sending a rejected
password twice. P5 counts how often `Rejected(Unconfirmed)` happens.

### 7.5 The fill (P2)

The 11-07 run found the rules below. Each is a test in P1t.
- **Where.** Only in the LMS folder's hidden window, only on a host in the row's `sign_in_hosts`, only
  over `https`. The host and scheme are checked again before each field and again before each submit.
  Anything else ends the attempt with nothing more filled: `CannotRead` before the first
  credential-carrying submit, `Rejected(Unconfirmed)` after it, because the mark is set (§7.4).
- **Where the form sends it (review r7, Important 4; review r8, Important 1).** The target is read from
  the submit control the fill will press, not from the form alone. It is read again on **every** page of
  the attempt, because the password step loads a new form. It is checked before each field and again
  just before each submit. Any failure leaves that field unfilled and submits nothing. It is `CannotRead`
  before the first credential-carrying submit and `Rejected(Unconfirmed)` after it. There are two kinds
  of submit:
  - **A navigation submit:** the control has a `formaction` or `formmethod`, or the form has a non-empty
    `action` or a `method`. The address is the control's `formaction`, else the form's `action`. An empty
    or absent `action` with no `formaction` resolves to the page's own URL and goes through the same checks
    (review r10, Minor 4). The method is the control's `formmethod`, else the form's `method`. The target
    must be `https`, on a `sign_in_hosts` host, and `POST`, so a password never goes in a URL or the
    folder's history.
  - **A script submit:** the form has no `method`, an empty or absent `action`, and no control carries a
    `formaction` or `formmethod`. The page's own script sends the fields. This is allowed only on a
    curated `sign_in_hosts` host over `https`. Where the script sends them is the accepted limit below.
  - **Evidence (spike, 2026-11-07, the controller's reading, values only, nothing filled):** the founder's
    sign-in page has one form with no `method`, an empty `action`, and no `formaction` or `formmethod` on
    any control. It is a script submit.

  **Accepted limits:** a curated page's own script decides where a script submit's fields go, and could
  send one elsewhere. A listed host may also answer a navigation `POST` with a 307 or 308 redirect, which
  re-sends the body to the redirect's target (review r8, Minor 5). In both cases the curated host is the
  trust boundary, as it is for a browser's password manager.
- **The page is two-step.** It asks for the username, then the password. While a school session is live
  it may ask for the password only. The fill handles both and nothing else.
- **Each field at most once per attempt.** A field seen a second time in one attempt ends it.
- **How a field is set.** The order the evidence block records (focus, set, the page's input events,
  leave the box, then a wait of about 1.5 s), so the page has registered the value before the submit.
- **Which button.** The form's own submit control, never the first button in the form: that can be a
  show-password control.
- **How it is pressed (review r9, Important 1).** By the chosen control's `click()`, or by
  `requestSubmit(control)`, so the page's own submit handler runs. Never `form.submit()`, which skips that
  handler. Where the page's handler does not take over, the founder's form (no `method`, empty `action`)
  would natively `GET` its own URL with the fields in the query. The password would then sit in the
  identity provider's logs and in the LMS folder's history on disk.
- **A script submit's backstop (review r9, Important 1; review r10, Minor 1).** Just before the press,
  the fill adds a **capture-phase** submit listener on `window`. It always calls `preventDefault`, so the
  form's native navigation never happens and no field reaches a URL. The page's own handlers still run,
  on the form or on `document`, and send the fields by script as they would anyway. The listener ends
  nothing by itself. The outcome is read from `users/me` as usual: 200 within `SUBMIT_SETTLE` is signed
  in, and anything else is `Rejected`, with the in-flight mark already set.
- **`Rejected`, the conservative rule (review r6, Important 1; review r7, Important 1).** Once the in-flight
  mark is written (§7.4), any ending but `users/me` 200 is `Rejected`. The school's recognised "wrong
  password" message is `Rejected(Wrong)`. Everything else is `Rejected(Unconfirmed)`, including the
  password asked again with no message, a "password expired" page, an MFA page, an unknown page, `Lost`
  or a timeout. Only a new save ends the pause. The mark is written before the first credential-carrying
  submit, the username step's included (§7.4). Before the mark is written, an unknown page is
  `CannotRead`. Which message counts as "wrong password" is the row's curated `wrong_password_text` (S2).
  Any other message is `Unconfirmed`.
  - **What it is matched against (review r8, Important 2):** only text visible on the page, read after
    `SUBMIT_SETTLE`. Hidden elements, script text, templates and attributes are never matched, so an error
    string hidden on an MFA page reads as `Unconfirmed`, never `Wrong`.
  - **Where the text comes from (review r8, Important 3):** only the school's or the identity provider's
    public documentation, cited in the row by date. Without that, it ships empty, which is the
    founder's row today. Empty means every ending after the password is `Unconfirmed`, so at setup the
    school's own page opens, which is safe. No wrong password is ever sent to learn the text.
- **How the value reaches the page (review r6, Important 3; review r7, Minor 4).** The script is built the
  way `mail.rs`'s `start_js` builds one: a fixed body, then `})(`, then one argument, then `)`. That
  argument is the only place the value enters, and its bytes are exactly `serde_json::to_string` of a
  JSON value holding it. Nothing else appends the value with `push_str` or `format!`. P1t's static pin on
  that splice is the guard. The round-trip test of `'`, `"`, `\`, a newline, U+2028, `</script>` and a
  non-BMP character is only a sanity check. The script's own error text is never forwarded: a failed
  script is `CannotRead` before a submit, in fixed words.
- **The secret.** It is read from Credential Manager inside the app's process, after the lock is taken,
  for that one attempt. It never reaches Knowlu's page, a log, a run record, a test name or an error
  message, as `credentials.rs`'s own rules say. Tests use invented values.
- **Never MFA.** The fill never reads or answers an MFA page. After a password submit, one counts as
  `Rejected(Unconfirmed)`. At setup, that opens the school's page for the student (§7.4). If R0c finds
  one, §2.3 applies.

## 8. Task briefs

Behaviour and tests only. Every test uses invented names, sections and hosts. Each `test-writer` half
leaves its tests compiling and failing at run time.

### R0a. The sign-in run (main session, Quinn at the machine): done

**Done, 11-05 to 11-07.** The answers are in the plan's §3 and in §2.1 here. (a) The session lapses about
12 hours from sign-in, used or not. (b) The password asks were the fill's own faults, now fixed. (c) and
(d) The mailbox does not carry the LMS, and the LMS signs in alone from the saved password. (e) is moot:
the fill does not rest on "stay signed in". The brief as run is kept below for the record.

The plan's §3 rules apply: a throwaway build, no debug port, counts and host kinds only, folders deleted.
Each item runs on a **fresh** folder unless it says otherwise.
- **(a) Lifetime.** Sign in, then a hidden link read at 12 hours or more in a new process. Add 24 hours
  if the clock allows. Record silent or not, and seconds.
- **(b) The password ask.** Repeat a click-through and an open at the link, each after a close. Record
  which surface asked, and what had closed before it.
- **(c) One sign-in.** Sign in at the LMS link only, then the mailbox hidden. On a second folder, sign
  in at the mailbox only, then the LMS link hidden. This decides §2's table.
- **(d) What carries it.** LMS sign-in alone, with no mailbox ever opened, then a new process and the
  link hidden. This decides RQ4, and it is what a student who skips school email gets.
- **(e) "Stay signed in" declined,** if time allows: the same as (a) at a shorter gap. This also feeds
  §2's fallbacks.

### R0c. The 24-hour repeat run (main session): running

The throwaway build, under the plan's §3 rules. A fresh, empty folder every 3 hours, a hidden window at
the link, the fill from the test account's saved password, stopping at the first failure. Record per
run: signed in or not, seconds, whether an MFA page appeared, and which step the page asked for. Never a
password, a cookie, a path or a name. Afterwards the folder goes, the saved login is deleted, and the
result is written in the plan's §3. A clean run leaves this plan as written. Any MFA page sends §2.3's
branch to Quinn before P4 goes out.

### R0b. The class-search spike (main session, no sign-in)

Public pages only. No student's courses: use a few sections from the public list itself. Record keys,
counts, seconds and status codes. Record: the rows' container; whether `term`, `subject`,
`courseNumber` and `sequenceNumber` are on each row; which kind fields exist; whether the session
cookie is required; whether a search needs `resetDataForm`; paging on a many-section course; any link
or cross-list field; seconds per query; whether a plain `ureq`-style request is answered. Stay under
§6.8's limits. Then run §6.10's curation check for the founder's school.

**Results, 2026-11-08 (controller; keys, counts, seconds and statuses only).** These came from 18 requests in three short runs. Each run was serial, with a gap of at least 1 s, under a `user-agent` naming Knowlu and `knowlu.com`. No refusal came back: every answer was a 200.
- **Container.** The answer is `{success, totalCount, data: [...]}`, and the rows are under `data`. Each meeting is at `data[].meetingsFaculty[].meetingTime`. Its keys are `beginTime`, `endTime`, `startDate`, `endDate`, the seven day booleans `monday`…`sunday`, `building`, `room`, `meetingType`, `meetingScheduleType` and `hoursWeek`, the same names `read_meetings` reads.
- **Row keys.** `term`, `subject`, `courseNumber` and `sequenceNumber` are on every row: 29 of 29 in a 29-section course. `courseReferenceNumber` is present. The 29 subject-number-section keys are unique.
- **Kind fields.** Present: `scheduleTypeDescription`, `instructionalMethod`, `instructionalMethodDescription`. Absent: `scheduleDescription`. The P5 title rule reads `scheduleTypeDescription` here.
- **Link and cross-list.** Present: `isSectionLinked`, `linkIdentifier`, `crossList`, `crossListCount`. In the sample, no section was linked and none set a `linkIdentifier`. `crossList` is a string or null. It was set on all 29 sections of the one course and on 48 of 50 in a term-wide page. So `linked: true` (§6.3) is readable, but this sample never exercised it.
- **Session cookie.** It is required. The same course query with no cookie answers 200 with `success: true` and 0 rows. An empty answer after a term POST that worked is therefore `closed`, never `not_found`, and C4's tests pin this.
- **The reset.** It is required. The site keeps the last search's filters on the session. With no reset after a course query, a query with blank course fields came back with the previous course's 29 rows. After a reset, the same blank query returned the whole term (`totalCount` 10077). §6.2's reset before every query stays. The reader-side check in §6.3 is what catches a stale answer.
- **A query with no subject or number lists the whole term.** That confirms §6.8's rule that the query type cannot be built without both.
- **Paging.** `pageOffset` and `pageMaxSize` work: offset 10 with size 10 returned 10 rows, and `totalCount` gives the full count. The many-section course, 29 sections, fit on one page of 50. C4 asks for 50 and follows `totalCount` as §6.2 says.
- **Time.** 0.95 s for the 29-section course query.
- **Plain request.** A request with no browser headers beyond the `user-agent` and the cookie is answered, so a plain `ureq` fetch matches.

**Curation check (§6.10), founder's school, 2026-11-08:**
1. Pass. The term POST and a course search answer JSON with no sign-in.
2. Pass. 29 of 29 sections each matched exactly one row by subject, number and section.
3. Pass, from the record in §6.3: 14 of 19 LMS names fit `TermSubjectNumberSection`.
4. Partly done. The host has no `robots.txt` (a 404). Posted terms were not looked for: the search page's entry URL answered with a redirect and no body. The school's terms of use are still to be read, and the RQ7 question is still on the lawyer's list. The row's `checked` date waits on this.
5. Not run here. The `lms_sign_in` link check belongs to the row that sets it (C4 and C9).

### C0. `scaffold.rs`: the row's new fields

Adds `ClassSearch` (§6.10), `Curated.class_search` and `Curated.lms_sign_in`, `None` on every row, so no
later task edits the struct. Tests that build rows use `..CAMPUSES[0]`, so they compile unchanged. S2's
`sign_in_hosts` stays S2's. r5's `school_sign_in` is withdrawn (§2).

### C1. Engine: the name and the parse

Tests: `section_in_name` over the fitting form, trailing words, a merged shell, four-digit numbers, a
trailing letter, lower case, no term, and a name with the code mid-string, each with its expected
`Some` or `None`. `parse_class_search` over inline rows: one section matched among many, two rows for
one section is `ambiguous`, a wrong `term` or subject is dropped, `EXAM` skipped, a no-time row is
`no_time`, a part-read row is `part_read`, an all-no-time course is no failure, a meeting across
midnight is dropped, and a school with a non-empty `registered` set still reads rows that carry no
status. `p2_the_meetings_faculty_wrapper_is_not_read` stays, unedited and green.
Build: `School` gains the name shape; `parse_class_search` shares `read_meetings`'s rules, by lifting
each row's meetings into the flat form. Output is sorted, so the same bundle gives the same bytes.

### C2a. Engine: the source row

Tests pin orders, never numbers, as the syllabus plan's T4t does: registrar before class search before
Google before the rest, in all four readers of `precedence`. Then:
- **Collapse.** A class-search row and a Google row with one signature collapse to class search. Mine
  on it closes both.
- **Holds nothing.** A class-search series holds no Google proposal. A confirmed class-search note holds
  none either: Mine on lecture 001 leaves recitation 101 proposed, and leaves a differently timed Google
  `class` of that course proposed. The test names both.
- **Can be held.** A registrar series or note of that course and kind holds a class-search row.
- **Keep-alive.** A read omitting a course leaves its series and ends nothing. An unfetched `classsearch:`
  calendar stays while its `until` is open. A **fresh read that omits a row whose `last_seen` is older
  than `UNSEEN_DAYS`** keeps it too, with no `ended` entry. On a fresh read a row past its `until` goes.
  Unfetched, a row past its `until` stays until `UNSEEN_DAYS` and is then dropped. Neither makes an
  `ended` entry or a change card.
- **Never a card, always covered, tested through `rank`.** On vault days 2 and 3 a vault with a class-search
  `class` row files no `commitment-check` for it, files no `commitment-ask` for its course, and withdraws an
  ask that stood. With the row gone, the ask files. `proposals`' `for_cards` output is unchanged.
- **The lab limit, pinned.** A course whose only class-search row is a `lab` files its ask, as a lab-only
  course does today (§6.7).
- **Never a successor.** A Google note whose series ended, beside an open class-search row of that course
  and kind, gets no change card naming the `classsearch:` key.
- **Body.** A note from a class-search key reads "Found in your school's public class search."
- **Nothing else moves.** `by_uid` of a `gcal-series:` and an `ics-series:` key is unchanged, and a vault
  with no class-search row gives the same bytes as today.

Each use of the `registrar:` prefix, in the engine and on the page, gets one line in the report:
unchanged, or changed and why. The list to start from: `commitments.rs` `:1289`, `:2280`, `:2556`, `:2590`, `:3537`, `:3951`,
`:3994`, `:4520`, `:4609`, `:4870`, `:5321`; `console.js` `:981`, `:1041-1043`, `:1122-1125`, `:1171`.
Line numbers are as of this draft, so find each by its text.

### C2b. Engine: the purge and the no-time block

Tests:
- **Purge.** `Rows` removes every `classsearch:` series, its `calendars` entry and any `ended` key under
  that prefix, and keeps a current no-time block. `All` removes the block too. Either drops a block whose
  term has passed. A confirmed `classsearch:` note is byte-identical afterwards, and no journal record
  appears. The series file then holds no `classsearch:` text but the block's key, and after `All` none.
  A second run writes nothing. It runs with no entitlement.
- **Clean and busy.** On a clean vault, purge takes no lock, creates no `state/` folder and writes nothing.
  With the lock held elsewhere, the try-lock form returns "busy" at once and changes nothing.
- **Changed under it.** A file that changes between purge's first read and the lock keeps the change: a
  registrar `applied` entry stored in that gap survives, and the class-search rows still go.
- **No-time.** A listed no-time course is not `uncovered` for `emit_asks`, `overview` or `cli.rs`. The block
  survives `refresh_series_held`, survives `Rows` with its `calendars` entry gone, and survives a registrar
  read of that term. `overview` lists `no_time_courses`. The bytes are sorted and go through
  `ledger::dumps_value`.
- **The term changes.** A block of a passed term is ignored at once. The next `rank` drops it and writes the
  file, on a vault with no `fresh` series too. A vault with no such block and nothing `fresh` takes no lock
  and writes nothing, as today.
- **The drop holds the lock.** With nothing `fresh`, the drop takes the series lock before it writes, and a
  registrar `applied` or `ended` entry stored between the unlocked read and the lock survives it.
- **A failed lock.** With the lock not taken, `rank` stores nothing and its warning names the lock. The same
  holds for a drop `refresh_series_held` sees with no lock held, and the next `rank` that gets the lock
  drops the block.
- **What is not passed.** A block of a future term is ignored and kept through `rank`. A block whose school
  gives `term_for` `None`, or whose term cannot be placed in `School::terms`, is kept.
- **The ask limit, pinned.** An ask filed before the block was written still stands after it (§6.5).
  `approvals.rs` is absent from the diff.

### C2c. Engine: a registrar read adopts a class-search note

Tests, through `run_parsed` with invented rows:
- A confirmed `classsearch:` note of the fetched term becomes `registrar:` with one journal record, actor
  `agent:knowlu.registrar`, written before the surgery, through `write::to_literal`.
- CRN present with a changed time: the change applies, a hand-set field is kept, and there is exactly
  one note. CRN absent: the note ends and is listed, and R24 makes the real section's note.
- A class-search note of another term is untouched, and an empty answer writes nothing.
- That term's unconfirmed `classsearch:` rows and `calendars` entry are gone after the read, with no
  `ended` entry, and another term's stay. The no-time block stays.
- A failed re-key is a named warning, R24 skips that CRN, and there is never a second note.
- A vault with no `classsearch:` note: every existing registrar test passes unedited.

### C2d. Engine: no marker, no key in a warning

Tests, each through the real route. After every one, a search of every note finds no `classsearch:` key:
- **Not mine on a class-search row** writes no marker, removes the row and exits 0. With a same-signature
  Google twin hidden behind it, the Google key is marked.
- **Not mine on a by-title Google twin** marks the Google key and removes the class-search row. Not mine on
  a same-signature Google key stays stale, as today.
- **A rejected `commitment-check` card** whose key has a class-search twin, through `settle_rejected`'s
  `Shape::Proposal` arm, writes the Google marker only. The `Shape::Change` arm's `twin_keys` is tested the
  same way.
- **`create_marker_as`**, called directly with a `classsearch:` key, returns `Ok(None)` and writes nothing,
  with no journal record.
- **`--confirm`** still writes nothing at all on bad input, row removal included.
- **Warnings.** Each of `confirm`'s five warnings, on a class-search key, reads "a class-search row" and
  holds no key text. On any other key the words are today's, byte for byte.

### C2e. Engine: backups

`series_for_backup` and one per-path helper used by `mirror` and `snapshot` in `backup.rs` (§6.12). Tests:
- With a class-search series, a no-time block and an `ended` key under the prefix all present, no file in
  the mirror and no entry in the snapshot zip contains `classsearch`, and no `class_search` key appears.
- The same file's `registrar` block is present and equal in both.
- **Exact bytes.** A series file with no class-search data, written by hand to be non-canonical with an
  extra key and odd whitespace, is copied byte for byte, and a second tick copies nothing.
- **Second tick.** With class-search data present, a second tick with nothing changed copies 0 files and
  reports the path `unchanged`.
- **The temp file.** A leftover `calendar-series.json.tmp<pid>` holding class-search text reaches neither the
  mirror nor the zip.
- **Malformed.** A malformed file holding class-search text is skipped for that run, and `mirror` leaves the
  older copy in place and archives nothing. A malformed file with none is copied as today.
- Every other file is unaffected, and `backup.rs`'s existing tests pass unedited.

### C3. Engine: the two commands

Tests: `--class-search` writes the series file only, with no note, card or journal record; `found` rows
carry `classsearch:` keys; `no_time` courses land in the block; the report holds counts and fixed words
only; a bad bundle exits 0 with a named line and nothing written; it conflicts with `--confirm` and
`--registrar`; an unknown `--school` is a named line. `--preview --class-search` fills `class_times` and
the state counts, with `null` for an unreadable file, and writes nothing. The `surface_oracle`
references stay.

### C4. App: the fetch

`app/tests/class_search.rs`, on a fake Banner bound to `127.0.0.1:0`. Tests: the call order, with reset
before each search; the anonymous cookie carried, and no other; one query for two sections of a course;
subject and number in every search; the 1-second gap on an injected clock; the query and time caps;
paging stopped at 3; 429 and 403 stop the run; HTML gives `closed`; no store of a cookie in any file;
the bundle holds no course name; and a `CAMPUSES` test (§6.10). Errors use fixed words, as
`registrar::fetch_error` does. The founder's row gets `class_search` only after R0b's curation check.

### C5a. Queue: the job out

`registrar.confirm` leaves `LANE`, `NOTE_JOBS`, `JOB_ORDER` and `inputs_ready`, and `registrar_confirm_rows`
goes. It has 30 references in `app/tests/onboarding_jobs.rs` and 11 in `app/src/onboarding_jobs.rs` by a
search of the job name; review r1 counted 82 across 7 files with the related names. Tests: the job is absent
from every list, and a resumed queue holding an old record of it starts cleanly. Each changed test is
listed in the report with why. No frozen or contract test is among them. The page's labels are C6's.

### C5b. State: the registrar pieces out

In `onboarding_state.rs`: `Answers.registrar_rows` and `RegistrarRow` go (a serde default keeps any dev
file readable); the step-3 capture arm; step 3's proceed list; `LANE_REQUIRED`; the line-16 comment.
Tests: `finish_refusal` never waits on the job, and step 3's proceed no longer asks for it.

### C5c. Queue: the two new jobs

Tests first, through the existing fakes: `school.class_search` runs between capture and preview, and is
skipped with no row; it ends `Done` on every failure; `class_times.apply` waits for `school.courses`; a
failed read leaves Open Knowlu offered; a stale generation never writes the bundle; `proposals_ready` is
false until `class_times.apply` is done, is true with no such job, and is true when the job is stuck
behind a paused `school.courses`; a second ask of either job in one school generation runs nothing; and
no Try again exists for class search, on the page or in the queue's status.

### C5d. Purge callers

Tests over fakes:
- **Finish.** `ExitSeams`' purge and bundle delete run after the drain and before `write_finish`, and a
  failed one never stops Finish.
- **Sign-out.** `on_sign_out_with` runs `All` and deletes the bundle on a finished and an unfinished file,
  and collects a failure with the rest (its test callers take the new input). `on_sign_out` keeps its
  signature and finds the vault through `profiles::load` by `profile_id`. A missing vault folder is no
  failure. No registry row, or an unreadable registry, is a collected failure.
- **`change_school`** runs `All` and deletes the bundle through its new input, and a failed one is a named
  line that does not stop the change. Its three test calls, in `app/tests/onboarding_jobs.rs` and
  `app/tests/onboarding_local_lane.rs`, take the input. Its job-removal list gains `school.class_search` and
  `class_times.apply`, and a test proves a new school reads again through `enqueue_once`.
- **`session_answer`**, as a table test over an injected credential read and invented `cloud.yaml` files:
  1168 on both reads is confirmed signed out; 1312 and any other code are Unknown; the credential and
  `CredError::Blob` are confirmed signed in; an empty target, a missing, unreadable or malformed `cloud.yaml`
  and a foreign base are Unknown. With 1168 on `cloud.yaml`'s target, a `session_target(id)` read that
  returns the credential, `CredError::Blob` or 1312 is Unknown.
- **`launch_sweep`**, over seams for the registry, the credential read, the purge and the delete, with
  invented profiles:
  - Confirmed signed out runs the delete, then `All`, for a mid-setup, a finished, an `Absent` and an
    `Unreadable` onboarding file alike.
  - **The exception (RQ15).** Confirmed signed in with `finished` true runs the delete, then `Rows`, and the
    `All` seam records no call. A bundle and rows left by a failed Finish go, and a current no-time block
    stays. An Unknown with `finished` true runs nothing.
  - **Nothing destructive runs** for each of these, and the purge and delete seams record no call:
    code 1312 at step 10; signed in and mid-setup; signed in with an `Absent` file; signed in with an
    `Unreadable` file; no `cloud.yaml`; an empty target; a malformed `cloud.yaml`.
  - **A moved vault.** The path and registry id have changed, `cloud.yaml` names the old id's target and the
    credential is there: confirmed signed in, no `All`. The old registry row, whose vault is gone, is
    Unknown and runs nothing.
  - **A re-attached vault.** `cloud.yaml` names a target that is gone, and the live session sits at
    `session_target(id)`: Unknown, and the purge and delete seams record no call, with `finished` true or
    false. `session_target(id)` is read only after a 1168 on `cloud.yaml`'s target, and never in its place.
  - A dirty signed-out profile with the lock busy is skipped and purged on the next launch. A clean one
    creates no `state/` folder. An unreadable registry is a named line. A sign-out whose purge failed is
    finished by the next launch.
  - The `onboarding.json` seam records a call only on a confirmed signed in, never on a confirmed signed
    out or an Unknown.
- **`main`.** A source pin that `launch_sweep` is called once, before the dispatch, and in neither `.setup`
  block.
- **A real miss.** No test here writes or deletes a real credential, and all but one touch none. That one
  reads a generated, missing target through `wincred::read_credential` and asserts `NotFound { code: 1168, .. }`,
  under a file-scoped `CREDMAN_LOCK`, because the whole rule rests on that number. It writes nothing, so it
  has nothing to clean up.

### C6. Page: step 10

Pins, then the page. Every line below is a pin in `app/tests/onboarding_static.rs`.
- `summaryLine10` stops saying "set from your school's registrar". The words become Quinn's, and until
  then "N class times found at your school. Check them below."
- `classsearch:` rows sit in their own group under their own heading, with no answer pre-pressed. Until
  Quinn's words: "Found in your school's public class search. Mark the ones that are yours." Under the
  group, one line: "Anything you leave blank is removed when setup ends, and Knowlu will ask about that
  class later."
- `confirmBarLine10`'s blank-row line stays as it is, since it holds for every row: a blank class-search
  row comes back as an ask.
- `no_time_courses` show "No set meeting time listed". They never show "Knowlu will ask this week"
  (`onboarding.js:633`). When they are the only courses to show, the empty-state line at `:635`, "It will
  ask in the coming days", does not show either.
- A `closed`, `paused` or `fetch_failed` state shows one line, with no error styling and no Try again.
- The labels at `onboarding.js`'s job names and `w10.reg`'s reads move from `registrar.confirm` to
  `class_times.apply`.
- A late `class_times.apply` adds new rows and leaves pressed rows as they were. The pin presses a row,
  fires the status, and reads the answer back.

Acceptance: side-by-side shots.

### C7. Page: the week strip's turned-in note (amends §14.58.5)

Quinn's addition, common to every school. The plan's §14.58.5 gains one bullet:
- **The note.** Under the days, "N turned in, kept off your list", with N exactly `step3.turned_in`.
- **When it shows:** only while the strip shows, and only when the field is a number of at least 1.
  `0`, `null` or absent shows nothing, with no placeholder. `null` means no grades bundle, so the count
  is unknown, not zero.
- **Rules kept:** the page sums nothing, the figure is the field, and it is a count with no titles.
  `turned_in` is already totalled over kept courses, so removing a course at step 3 lowers it.

Pins in `app/tests/onboarding_static.rs`, over `weekStripModel`: 3, 1, 0, `null`, absent, hidden steps,
and a fixture before and after a removal. About 8 lines in `onboarding.js` and 3 in `onboarding.css`.
The prototype has no such note, so the reviewer treats it as a ruled departure in the shots.

**The promise has to be true.** O1 archives Blackboard's submitted work at first sight only if the first
slot's grades capture ran. If that hidden read is signed out, those items arrive live and are marked
done a slot later, so the note would be briefly false. This is one more reason for §7.3's single window.
T3b's S-F check covers the number: N equals what the vault then holds as turned-in LMS work, or differs
by a stated reason. It is not the reveal's whole `imported_done`, which also counts homework sites.

### C8. App: the link

Tests over `sign_in_link`: one link; none; two; an entity-encoded query; root-relative; another host; a
suffix look-alike; `http`; `//host`; `@`; a backslash.

On a local listener: the GET asks for `/`; it sends no cookie; a same-host redirect is followed; a fourth
hop, or any other host, gives `None`; `None` falls back to `/ultra/`; a transport failure or a 5xx
gives `Unreachable` and opens no window.

`read_hidden` has only a real `AppHandle` today, so C8 splits it: a `read_hidden_with` over injected
closures for the window's URL, the `users/me` read and the clock, with `read_hidden` a thin wrapper.
`hidden_outcome` is pure. Tests take §7.4's table row by row, including the two the link makes new: on
the identity provider at the deadline with `last` empty is `SignedOut`, and with `last` a transport
failure is `SignedOut` too. A window at `/ultra/` ends as it does today. No test opens a visible window.

The founder's row sets `lms_sign_in` to its prefix, in `scaffold.rs`'s order (§4.2).

### C9. Docs

`docs-keeper` drafts, for Quinn's signature: spec §1.3 step 3 and §2.4's uses; a dated line retiring spec
§4.2 item 1; the plan's §9 rows A2, A4 and A6 withdrawn, row A1 rewritten for two folders, row A3
naming the saved school password, and row A5's Forget gaining its deletion, as S8's row in §4.5 says; the syllabus spec's E4 tiers; the plan's
§14.58.5 bullet; the note on Q1 (§2.2: one typed password and one mailbox sign-in, not "one sign-in");
and the spec §2.4 and §2.5 lines for two folders and the saved school password. Also, in
`docs/reference/app.md`, the `knowlu/<profile_id>/school` credential, when it is deleted, and the fill's
rules (§7.5). Also the curation check in `docs/reference/app.md`,
`commitments --class-search`, Not mine's row removal, `purge_class_search` and backups' filtered copy in
`docs/reference/engine-commands.md`, the launch sweep in `docs/reference/app.md`, the `class_search` block in `docs/surface/anatomy.md` if the overview
is documented there, app data's new file, and `CLAUDE.md`'s `commitments` line if Quinn wants the
new prefix named there (the planner may not edit it). `VISION.md` needs nothing: RQ10's line is in.

### C10. Page: the first-day screen and Schedule

Pins in `app/tests/`, over `console.js`. A `fromClassSearch(p)` mirrors `fromRegistrar(p)`. `setupRow` gives a
`classsearch:` row no preset, so Finish never writes it unpressed; a class, lab or work row from any
other source keeps today's preset. `renderRegistrar` and `paintRegistrarAdded` never list one. No
`classsearch:` row carries the registrar's label. The test builds the review's case: step 10 confirmed
with two rows blank, no planning-day note, `setup` true, Finish pressed, and neither row written.

### P1t. Failing tests: the saved password and the fill's decisions

`app/tests/school_password.rs` and `app/tests/school_fill.rs`, new. Invented usernames, passwords and
hosts only. Every test but one runs over an injected credential store and a fake page.
- **The target** is `credentials::target_for(profile_id, "school")`, built by that function and never by
  hand. Two profiles never share one.
- **Save** writes through the injected store. The save command's reply holds no secret, and neither do
  its error text or `Debug` output.
- **Read** happens inside the process only. No Tauri command returns it.
- **The order of each step (review r7, Minor 2), checked through a store that records calls:**
  - **A rejection** sets the rejected mark first, then deletes the saved password. If the delete fails,
    the mark still blocks every fill. If the mark cannot be written, the delete still runs.
  - **A save** writes the credential first, then clears the rejected and in-flight marks. If the clear
    fails, the old mark blocks the new password, which fails safe, and the student is asked again.
  - **Forget** deletes the credential first. It clears the marks only when the delete succeeded or the
    credential was already gone. If the delete fails, the marks stay and the failure is reported.
- **Only a new save clears the rejected mark.** A visible sign-in that reaches `users/me` 200 does not
  (review r6, Important 1).
- **The in-flight mark (review r7, Important 1; review r8, Important 4 and Minor 3):**
  - it is written and flushed with `sync_all` before the first credential-carrying submit, through a
    file seam that records both calls in order. On a two-step fake page, the seam records the write and
    flush before the **username** step's submit (review r10, Minor 2);
  - an unknown page after the username step's submit is `Rejected(Unconfirmed)`, and no password is
    filled;
  - if the write or the flush fails, nothing is submitted and the answer is `CannotRead`;
  - only `users/me` 200 in that attempt, or a new save, clears it;
  - `Lost`, a timeout, an LMS 5xx and a dropped connection after the submit each give `Rejected(Unconfirmed)`;
  - it is checked only before a fill. With the mark set from an earlier attempt, as after an app close or
    sleep, a read whose session is live still reads grades, and the mark stays. A read that would fill
    gives `Rejected(Unconfirmed)` instead and submits nothing.
- **The lock (review r8, Minors 1 and 2):** both marks and the secret are read only after
  `school_session`'s lock is held. A save while a fill holds the lock answers `Busy` and writes nothing,
  so no mark is cleared under a running fill.
- **The rejected mark with no password saved** gives the form, never today's path (review r8, Minor 3).
- **`retarget_credentials`** moves `school` with `zybooks` and `vhl`, writing the new target before it
  deletes the old. **A rejected password is never moved** (review r6, Minor 3): with the old id's mark set,
  the old credential is deleted, the new target gets nothing, and the move is not a failure.
- **The fill's decisions**, through a fake page, one test per rule in §7.5:
  - a page host outside `sign_in_hosts`, `http`, or a redirect mid-attempt fills nothing;
  - **the submit target (review r7, Important 4; review r8, Important 1):**
    - a navigation submit: the control's `formaction` wins over the form's `action`, and its `formmethod`
      over the form's `method`; an off-list host, `http` or a `GET`, from either source, fills nothing;
      a form with a non-empty `action` and no `method` resolves to `GET` and fills nothing;
    - a script submit, shaped like the founder's page (one form, no `method`, an empty `action`, no
      `formaction` or `formmethod`), fills on a listed https host and fills nothing on an off-list host;
    - a control that adds a `formmethod="get"` to that script-submit form makes it a navigation submit,
      and it fills nothing;
    - **a script-submit form whose page script does not intercept the submit (review r9, Important 1):**
      the capture-phase listener prevents the default, no navigation happens, and no field appears in
      any URL the fake page records. With no `users/me` 200 within `SUBMIT_SETTLE`, the attempt is
      `Rejected(Unconfirmed)` with the in-flight mark set;
    - **a page that intercepts from a handler on `document` (review r10, Minor 1):** the page's handler
      still runs and sends by script, no field appears in any URL, and `users/me` 200 gives signed in;
    - **a navigation submit with an empty `action` (review r9, Minor 1)**, made one by a
      `method="post"`, resolves to the page's own URL and passes or fails the same https, listed-host and
      `POST` checks: on a listed https page it fills, and on an off-list page it fills nothing;
  - the password step's new form, whose target is off-list after an on-list username step, fills no
    password, and is `Rejected(Unconfirmed)` because the mark is already set;
  - a target or host that changes between the last field and the submit sends nothing;
  - the username step, then the password step; the password-only page;
  - a field seen twice ends the attempt;
  - the submit pressed is the form's own, never a show-password control placed first;
  - the wait comes before the submit;
  - after the password's submit, the row's `wrong_password_text` is `Rejected(Wrong)`. Each of these is
    `Rejected(Unconfirmed)`: the password asked again with no message, another error text, a "password
    expired" page, an MFA page and an unknown page. With the row's text empty, a wrong-password page is
    `Rejected(Unconfirmed)`. Only a signed-in LMS is neither;
  - **visible text only (review r8, Important 2):** an MFA page that carries the row's text in a hidden
    element, a script, a template or an attribute is `Rejected(Unconfirmed)`, not `Wrong`. The match is
    made after `SUBMIT_SETTLE`, never before;
  - an unknown page before the first credential-carrying submit is `CannotRead`, with nothing sent.
- **The value's splice (review r7, Minor 4), the real guard.** A static pin on `app/src/school_fill.rs`'s
  source: the script is a fixed body, then `})(`, then one argument, then `)`, as `mail.rs`'s `start_js`
  splices its `args`. That argument is built only by `serde_json::to_string`, and no `push_str` or
  `format!` appends the value anywhere else. The round-trip test of `'`, `"`, `\`, a newline, U+2028,
  `</script>` and a non-BMP character stays, as a sanity check only. A script that fails yields fixed
  words that hold no part of the value or the script.
- **Static pins on the window script (review r6, Minor 4)**, read from the same source: it picks the
  submit control from the form's own `type=submit` controls and never the first button; it reads
  `formaction` and `formmethod` from that control and the form's `action` and `method`; it fires input,
  keyup and change, then leaves the box, in that order; the wait comes before the submit; it reads
  page text through a visible-text property, never `textContent` or `innerHTML`; it presses by the
  control's `click()` or `requestSubmit(control)`. `submit` never appears as a property or member, so
  the pin rejects `.submit(`, `.submit` alone, `prototype.submit.call` and `["submit"]` alike. The event
  name in `addEventListener("submit", …)` and `requestSubmit(control)` stay allowed (review r10, Minor 1).
  It adds the capture-phase listener on `window` just before the press, and that listener calls
  `preventDefault` with no condition (review r9, Important 1).
- **One real credential test**, under a file-scoped `CREDMAN_LOCK`, with a generated profile id and a
  `Drop` guard: save, read, forget through the real `credentials.rs` and `wincred.rs`. The controller
  signs it off before it is written, as `test-writer`'s rule requires.

"P1" below means P1a and P1b, and "P2" means P2a and P2b (review r8, Minor 6). Each half keeps to the
plan's about-80-lines-a-file rule. All four are `contract-engineer`'s, and `contract-reviewer` reads each
diff.

### P1a. App: `school_password.rs`, the credential (contract-engineer)

Makes P1t's target, save, read, forget, lock and real-credential tests pass. Routes through
`credentials::target_for`, `write`, `exists` and `delete`, and `wincred::read_credential`, and edits
neither file. If an edit to either seems needed, stop and tell the controller. It exposes `forget`, which
S6a's `school_forget` calls; S5's sign-out deletes the credential itself (RQ17). The save takes S2's
`school_session::try_take(profile_id)` and answers `Busy` when it gets `None`. It opens no window, so it
does not take `Capturing`. P1a does not edit
`school_session.rs`. One Tauri command saves, used by step 3
and Settings. P1a writes the command beside the module and does **not** touch `main.rs`; S6r puts its
name in the `generate_handler!` list (review r6, Minor 2). About 60 lines.

### P1b. App: `school_password.rs`, the marks; `retarget_credentials` (contract-engineer)

Makes P1t's mark, order and retarget tests pass. Both marks live in the profile's app data, never in the
vault, and hold no secret. The in-flight mark is written and then flushed with `sync_all` before the
submit (§7.4). Each step runs in P1t's order. `onboarding.rs`'s `retarget_credentials` list gains
`school`, and it never moves a password whose rejected or in-flight mark is set. About 70 lines in
`school_password.rs` and 10 in `onboarding.rs`. It follows P1a.

### P2a. App: `school_fill.rs`, the pure decision (contract-engineer)

Makes P1t's fill-decision tests pass, over a fake page. It is one function over the page's shape, the
page host, the submit kind with the chosen control's resolved target and method, the scheme, the row's
`wrong_password_text` matched against visible text, and what this attempt has filled. It runs again on
every page of the attempt. It touches no window and no credential. About 80 lines.

### P2b. App: `school_fill.rs`, the window script (contract-engineer)

Makes P1t's static pins pass. The window side runs P2a's decision in the LMS folder's hidden window and
nowhere else (S3 test 4's one exception). It writes and flushes the in-flight mark through P1b before the
first credential-carrying submit, the username step's included, and submits nothing if either step fails. It passes the value only as the one
`serde_json` argument, never forwards the script's error text (§7.5), and waits `SUBMIT_SETTLE` after the
submit. It presses by `click()` or `requestSubmit(control)`, never `form.submit()`. For a script submit it
first adds the capture-phase listener on `window`, which always prevents the default (§7.5). It never shows the window, and never sends keyboard or mouse input to the desktop. About 80
lines. It follows P2a and P1b.

### P3. Wiring: the lapse and step 3 (implementer)

Tests first, over `read_hidden_with`'s fakes (C8):
- a hidden read meeting the sign-in page runs the fill, then `hidden_outcome`;
- each row of §7.4's fill table, by name;
- one fill per read;
- after `Rejected`, either kind, no slot fills again, and a visible sign-in that reaches `users/me` 200
  does not change that; only a new save does (review r6, Important 1);
- after setup, `CannotRead`'s row press and Settings open the visible sign-in at the link, and
  `Rejected`'s open Knowlu's form (review r7, Important 2);
- the form's state carries the kind, so "didn't accept" appears only for `Rejected(Wrong)`;
- **concurrency (review r6, Minor 1; review r8, Minor 1):** a queue read and a slot read at once give one
  fill, and the second answers `Busy` with no fill; both marks and the secret are read only after the lock
  is held, so a `Rejected` from the first stops the second; a save during a fill answers `Busy`;
- a live session with a stale in-flight mark reads grades, and the next would-be fill sends nothing;
- the rejected mark with no saved password routes the row to the form.

On step 3, through `school_session`'s seam (S4d's replacement), per the setup split (§7.4, ruled:
confirmed by Quinn on 2026-11-07):
- the saved password leads to one hidden window and a signed-in LMS, with no window shown;
- `Rejected(Wrong)` returns step 3 to Knowlu's form with the "didn't accept" state, opens no window, and
  runs no fill until a new save;
- `Rejected(Unconfirmed)`, an MFA page among its cases, opens one visible window at the link, and sends
  nothing. After the student's visible finish, step 3 offers the form once, with "Not now";
- `CannotRead` opens one visible window at the link, and the saved password stays;
- no visible window's page is ever read for typed values;
- a window the student closes is `signed-out`, never an error.

`contract-reviewer` reads the diff.

### P4t. Failing pins: the password form (test-writer)

Pins in `app/tests/onboarding_static.rs` and `app/tests/static_assets.rs`, red until P4, as C6's are
(review r6, Minor 4):
- step 3's form sends the username and password once to P1's command;
- the page keeps neither after the reply, and the reply never holds them;
- the "kept on this computer" line is under the form;
- no school page shows while the hidden sign-in runs;
- each outcome's path, one pin each (review r7, Important 2):
  - at setup, `Rejected(Wrong)` shows the form again with "Your school didn't accept that password", and
    no window;
  - at setup, `Rejected(Unconfirmed)` and `CannotRead` show the school-page line;
  - at setup, after a visible finish that followed `Unconfirmed`, the form shows once more with the
    `Unconfirmed` words and "Not now";
  - under §2.3's MFA branch, a page flag P4 sets once R0c's result is in, the `Unconfirmed` words carry
    no "to keep signing in for you"; with the flag off, they do (review r8, Minor 4);
  - after setup, the signed-out row's press after `Rejected` opens the form, with words matching the
    kind;
  - after setup, the row's press and Settings after `CannotRead` open the visible sign-in at the link,
    not the form;
- step 5 is one visible mailbox sign-in with the "stay signed in" line;
- Settings' "Update school password" opens the form;
- the Settings row reads RQ18's words.

### P4. Page: the password form and the words (console-ui)

Makes P4t's pins pass. Step 3's form asks for the school username and password. Under it, one line says
the password is kept on this computer. If the school shows its recognised "wrong password" message, the
form shows again with one line, "Your school didn't accept that password". The student types it again
there, and that is the one ask. For anything else, step 3 says the school's page is opening so the
student can finish there. That covers an MFA prompt, an unknown page, and a page the fill cannot read.
After an `Unconfirmed` finish, the form shows once more with its own words and "Not now". After setup,
`CannotRead` sends the student to the visible sign-in, not the form. The setup split is ruled (confirmed
by Quinn on 2026-11-07). Step 5 is one visible mailbox sign-in, with a line asking the student to accept "stay
signed in". Settings' "Update school password" opens the same form. Words are Quinn's. Waits on R0c.

### P5. The live proof (main session, Quinn's go)

A dev build against staging on a scratch profile at the founder's school, with no debug port. Record
counts, seconds and host kinds only. Steps: one typed password into Knowlu's form, no visible school
page; one visible mailbox sign-in with one MFA, then mail silent; a hidden read past 12 hours signs back
in silently. Then one deliberately wrong password, on the test account only, and only one in all. With
the row's `wrong_password_text` empty, as it ships, it is `Rejected(Unconfirmed)`: the school's page
opens, and the wrong password is never sent again. Nothing on that page is read into the row (review r8,
Important 3). `Rejected(Wrong)`'s path is proven over P3's fakes only. Count each
`Rejected(Unconfirmed)` and its cause, for review r7's Minor 1. A friend's account repeats the first
three (RQ13), never the wrong-password step. Afterwards the scratch profile, its credentials and folders
are removed.

## 9. Follow-ups, not in wave B

- **A 12-hour clock for class times (Quinn, 2026-11-04).** Its own task after onboarding, across every
  surface that shows a time. C6 keeps today's time format, so wave B neither fixes nor worsens this.
- The bonus registrar button's `start` page, pointed past the public menu (§5).
- Class search at a new term: a press, once students have used this (RQ8 keeps it out of wave B); a
  name with no section, when a course has only one section; a remote off-switch for a school's row (§6.8).
- A Try again for class search, cut from wave B (§6.8). It needs a stored per-course record of what was
  asked and answered, and that record would join §6.12's table and the privacy text.
- Other student-system class searches. Banner is a minority of schools, so wider coverage needs other
  readers, each its own spec.
- One session target for a moved vault. Reads use `cloud.yaml`'s target, while sign-out and attach use
  `session_target(id)` (§6.12's two limits). Making all three agree edits `account.rs`, on the contract
  list, so it needs its own plan and `contract-engineer`.

## 10. Rulings and open questions

### Ruled, Quinn, 2026-11-04 and 2026-11-07

**The record of the 2026-11-07 rulings** (RQ3, RQ4, RQ5, RQ16, RQ17 and RQ18) is
`docs/notes/2026-11-07-onboarding-rulings.md`, written by the controller (review r9, Minor 3).

- **RQ1. Precedence:** class search sits below the registrar and above the syllabus (§6.6). It amends the
  syllabus spec's E4 tiers (C9).
- **RQ2. The press:** a class-search row is never written without the student's press (§6.7). Read
  strictly, that is one press per row, on every screen. r1's "these are all mine" option is withdrawn.
- **RQ6. Privacy text:** approved, with RQ14's clause, and the Settings row reads "used for grades and
  school email". It holds only with the purge (RQ11). The Settings words were reopened, and RQ18 has since ruled them, because
  RQ4's two folders give the mailbox its own row. The class-search text below is unchanged.
  > "At schools with a public class search, Knowlu looks up your classes there from this computer,
  > without signing in. The results stay on this computer and are deleted when setup finishes or you
  > sign out, except a list of which classes show no meeting time, which stays on this computer until
  > shortly after the term ends or you sign out. The class times you confirm become part of your plan,
  > which syncs to your Knowlu account."
- **RQ7. Before a school's row ships:** `robots.txt` and any posted terms are read and dated in the row,
  and the question goes on the lawyer's list (§6.10). The founder's school's check is due before C4's
  row lands.
- **RQ8. When it runs:** at setup only, in wave B (§6.9).
- **RQ9. No-time sections:** a section listed with no meeting time skips the later "when does this
  meet?" ask (§6.5). The accepted cost: a section whose time is simply not posted yet gets no ask.
- **RQ10. `VISION.md`:** gains the one line. Done: it is in the file, so no task is owed.
- **RQ11. The purge:** approved (§6.12). Every unconfirmed class-search row is deleted at setup end and at
  sign-out. The accepted costs: a row left blank at step 10 is gone, so the student later gets an ask and
  types a time Knowlu once had, and step 10 warns them. Not mine leaves no marker. Setup's answers
  cannot be revisited after Finish.
- **RQ12. The no-time record:** may outlive setup (§6.5). It holds only course keys the vault already has.
- **RQ13. The second account:** a friend at the founder's school (P5; r5 named A4 or B4).
- **RQ14. The added clause:** approved as written, and folded into RQ6's text above. RQ12 keeps the list of
  no-time classes past setup, so the old words were not literally true. "Shortly after" means the first
  `rank` after the term changes (§6.5). "Stays on this computer" holds, because no sync or backup carries it.
- **RQ15. A failed Finish purge (ruled after review r4):** yes, the next launch retries it, as the fail-safe
  rule's one exception (§6.12). A confirmed signed in with `finished` true runs the bundle delete, then
  `Rows`, and never `All`. Without it, a student whose write failed at Finish would keep rows and the bundle
  past setup, and "deleted when setup finishes" would be untrue for them.
- **RQ16. The school password (ruled 2026-11-07):** Knowlu keeps the student's school sign-in in Credential
  Manager exactly as it keeps every other portal login (`knowlu/<profile_id>/<source>`, on the device,
  never sent to the cloud), and signs the course site back in silently when its session lapses. It is the
  default, not an opt-in. Basis: the spike's hidden fill on an empty folder signed in in 8 s with no Okta
  prompt, once the fill waited for the page to register each box (§3, 11-07). A rejected password is asked
  for once and never retried. The privacy page states it plainly; that wording does not hold the feature.
  This supersedes the "about one password a day" fallback as the plan, pending the 24-hour repeat run
  (R0c, §2.3). Built by P1 to P4 (§4.3); the fill's rules are §7.5.
- **RQ4. One folder or two (ruled 2026-11-07):** two. The LMS signs in on its own empty folder with the
  saved password, so it does not need the mailbox's sign-in beside it, and "Disconnect" stays per source.
  It withdraws r5's recommendation of one shared folder, and S4c with it (§4.5).
- **RQ3. When the LMS session lapses (ruled by Quinn in session on 2026-11-07, recorded by the controller):** "yes, sign back in
  silently every time". Knowlu signs back in with the saved password whenever the session lapses (§7.4).
  The named signed-out row ("Sign in to update grades") and the Settings sign-in appear only when the
  school rejects the saved password, which is asked for once and never retried, or when the fill cannot
  read the page. It holds pending R0c: if a fresh sign-in shows MFA, §2.3 sends it back to Quinn.
- **RQ5. Where each sign-in happens (ruled by Quinn in session on 2026-11-07, recorded by the controller).** Setup: (1) the
  student types their school username and password into Knowlu's own form, like every other portal
  login and stored per RQ16, and Knowlu signs the LMS in hidden from it, with no school page shown. If
  that hidden sign-in fails at setup, Knowlu shows the school's own page in a visible window so the
  student can finish. (2) Then one visible school-mailbox sign-in, MFA once and "stay signed in", after
  which mail stays silent. **Rejected:** capturing the password as the student types it on the school's
  own page. Built by P3 and P4 (§7.4, §8).
  **How a setup failure splits (r8, the coordinator's design after review r7; ruled: confirmed by Quinn
  in session on 2026-11-07, "yes to all four", recorded in `docs/notes/2026-11-07-onboarding-rulings.md`).** RQ5 as ruled shows the school's page for any setup failure. The split
  narrows that in one case. A recognised "wrong password" message from the school, after the password,
  sends the student back to Knowlu's own form, as one ask. Anything else after the password, including
  an MFA prompt or an unknown page, opens the school's own page in a visible window, where the student
  finishes, because the student is present. So does a page the fill cannot read. Knowlu never re-sends a
  password on its own. After setup the conservative rule stands, and `CannotRead` offers the visible
  sign-in, not the form (§7.4).
  - r7's split sent every `Rejected` to the form. Review r7 showed it false under MFA, because a correct
    password would read "didn't accept" and setup could never finish.
  - r6's "Update school password" line is also withdrawn. It rested on a false premise: with nothing
    saved, a visible sign-in lasts only until the first lapse.

- **RQ17. Sign-out deletes the saved school password (ruled by Quinn in session on 2026-11-07, recorded by the controller):
  yes.** It is also deleted at Forget and at local-data deletion. Today `zybooks` and `vhl` survive a
  sign-out and go only with `delete_local_data`. The school password is the exception, because the school
  folder already goes at sign-out (spec §2.5, Q9). A password that outlived it would let the next account
  on this profile sign in to the last student's school silently. The cost is that a student who signs
  out and back in types the password again. S5 carries it, on `account.rs`, so it is
  `contract-engineer`'s.
- **RQ18. The Settings row's words (ruled by Quinn in session on 2026-11-07, recorded by the controller):** "School sign-in: kept
  on this computer, used for grades and your course list". The mailbox keeps its own row, as the mail
  lane built it. This replaces RQ6's Settings words. S6b carries it.

### Open

No question is open. The setup split is ruled (RQ5's note above; checkpoint 1).

## 11. Fidelity ledger

| Source | Requirement | Task | Proof |
|---|---|---|---|
| Quinn, 2026-11-04 | The registrar is optional; onboarding does not depend on its sign-in | C5a, C5b, F8b, F8w1, F15 | C5a's and C5b's tests; F15's run with no registrar read; the plan's §8 checklist item |
| Quinn, 2026-11-04 | What stays as a bonus keeps working as it does | C2c only | engine `registrar.rs` changes by C1's new functions and adoption; existing registrar tests unedited; `app/src/registrar.rs` absent from every diff |
| Quinn, RQ1 | Registrar, class search, syllabus, Google, the rest | C2a, C9 | C2a's order tests |
| Quinn, RQ2 | No row reaches a confirmed commitment without a press on that row, on any screen | C2a, C2d, C6, C10 | §6.7's path table; C10's review case; C2a's never-a-card and never-a-successor tests through `rank` |
| Quinn, RQ6, RQ14 | The approved privacy text, with the added clause, is true, literally, with one named exception: after a failed Finish purge, a re-attached vault, or a moved vault with a new registry id, keeps its rows and bundle until sign-out | C2b, C2e, C5d, F2c, S7 | C2b's purge and term-change tests; C2e's backup tests; C5d's caller and sweep tests; S7's page |
| Quinn, RQ15 | A failed Finish purge is retried at the next launch: confirmed signed in with `finished` true runs the bundle delete, then `Rows`, never `All` | C5d | C5d's exception case; its `onboarding.json` seam case |
| Quinn, RQ11 | Unconfirmed rows are purged at setup end and sign-out | C2b, C5d | C2b's purge tests; C5d's caller and sweep tests |
| Quinn, RQ12 | The no-time record may outlive setup | C2b | C2b's survival tests |
| Review r3, 1 and 2 | The sweep runs nothing destructive short of a confirmed signed out, RQ15's row apart, read from the credential the app's own reads use | C5d | C5d's `session_answer` table; its nothing-destructive cases, 1312 and the moved vault among them; `contract-reviewer` on the diff |
| Review r3, Minor 10 | Two accepted limits: a `lab` row covers nothing, and the no-time block stops new asks only. No `approvals.rs` edit in wave B | C2a, C2b | C2a's and C2b's limit pins; `approvals.rs` absent from every diff |
| Review r4, 1 and Minor 2 | Every series-file write from a drop or a purge holds the series lock and re-reads under it. With no lock, a drop stores nothing and waits for the next `rank` | C2b | C2b's "The drop holds the lock", "A failed lock" and "Changed under it" tests |
| Quinn, RQ13 | A friend at the founder's school is the second account | P5 | the check's record in P5 |
| Quinn, RQ16 | The school password is saved by default, at `knowlu/<profile_id>/school`, through `credentials.rs` and `wincred.rs`, on the device only | P1, P4 | P1t's target, save and read tests; P1's diff edits neither credential file; P4's pin that the form saves with no opt-in |
| Quinn, RQ16 | The password never reaches the cloud, the page, a log, a run record or an error | P1, P2 | P1t's reply, error and `Debug` tests; `contract-reviewer` on P1 and P2 |
| Quinn, RQ16 | A rejected password is asked for once, in Knowlu's own form, and never retried; only a new save ends the pause | P1, P3, P4 | P1t's rejected-mark tests, the failed delete among them; P3's "no fill after `Rejected`, even after a visible sign-in" test; P4t's "didn't accept" pin; P5's wrong-password step |
| Quinn, RQ16 | The privacy page states it in one plain line; the wording does not hold the feature | S7 | S7's page and `engine/tests/site.rs` pin; P1 to P4 do not depend on S7 |
| Quinn, RQ3 | A lapsed session signs back in silently every time; the signed-out row and Settings' sign-in only on a rejection or an unreadable page | P3 | P3's tests over §7.4's fill table, row by row; P5's past-12-hours read |
| Quinn, RQ5 | Setup: the password typed into Knowlu's own form, then a hidden LMS sign-in with no school page shown | P3, P4 | P3's step-3 test (one hidden window, none shown); P4's form pins; P5's count |
| Quinn, RQ5, with the setup split (confirmed 2026-11-07, `docs/notes/2026-11-07-onboarding-rulings.md`) | At setup, a recognised "wrong password" returns to Knowlu's form with "Your school didn't accept that password"; anything else after the password (MFA, an unknown page), and a page the fill cannot read, opens the school's own page, where the student finishes | P3, P4 | P3's three step-3 outcome tests; P4t's setup pins; Quinn's confirmation in the rulings note |
| Quinn, RQ5 | Then one visible mailbox sign-in, MFA once, "stay signed in", mail silent after | P4, F11b | P4's step-5 pins; P5's mailbox count |
| Quinn, RQ5 (rejected alternative) | No password is captured from the school's page | P2, P3 | P3's test that the visible window's page is never read for typed values; `contract-reviewer` on P2 and P3 |
| Quinn, RQ4 | Two folders: the LMS's own, and the mailbox's `mail-session-<id>` | S1, S3, §4.5 | S1's `profile_for` tests (grades and the LMS capture only); S3 test 1 without the mailbox; S4c dropped; the mail lane's tests unedited |
| Spike, 11-07 | The fill fills only a `sign_in_hosts` host over https, each field at most once, the form's own submit after the wait | S2, P2 | S2's `CAMPUSES` test; P1t's fill-decision tests, one per §7.5 rule |
| Spike, 11-07 | The fill never answers MFA | P2 | P1t's case: an MFA page after the password's submit is `Rejected(Unconfirmed)` |
| Review r6, Important 1 | After the password's submit, any page but a signed-in LMS is `Rejected`; only a new save ends the pause | P1, P2, P3 | P1t's post-submit cases (error, "password expired", MFA, unknown); P1t's "a visible sign-in does not clear the mark"; P3's matching test |
| Review r6, Important 2 | The form's resolved `action` is https on a `sign_in_hosts` host before any field; the host is re-checked before the submit | P2 | P1t's off-list `action`, `http` `action`, empty `action` and host-change cases |
| Review r6, Important 3 | The value enters the script only `serde_json`-encoded; the script's error text is never forwarded | P2 | P1t's encoding cases (`'`, `"`, `\`, newline, U+2028, `</script>`, non-BMP); its fixed-words case |
| Review r6, Important 4 | Forget deletes the saved password and the mark, not only the folder | S6t, S6a, P1 | S6t's `school_forget` test; the edge P1 to S6a |
| Review r6, Minor 1 | One fill at a time per profile; the mark and the secret read under `school_session`'s lock | P3 | P3's concurrency test |
| Review r6, Minor 2 | P1's command is registered by S6r, in `main.rs`'s chain | S6r | S6r's pin that the name is in the list |
| Review r6, Minor 3 | `retarget_credentials` never moves a rejected password | P1 | P1t's retarget case with the mark set |
| Review r6, Minor 4 | P4 has a pins-first half; P2's window script has static pins | P4t, P1t | P4t's pins; P1t's static script pins |
| Review r6, Minor 5 | The wave-B §9 amendment rows match two folders: A1 rewritten, A2 and A4 withdrawn | S8, C9 | Quinn's signature on C9's drafts |
| Review r6, the setup call | A setup rejection asks once, in Knowlu's form; superseded in r8 by the setup split (review r7, Important 3) | P3, P4 | as review r7, Important 3's row |
| Review r7, Important 1 | An in-flight mark goes to disk before the first credential-carrying submit (review r10, Minor 2); only `users/me` 200 or a new save clears it; any other ending, app close and sleep included, is `Rejected`; no mark, no submit | P1, P2 | P1t's in-flight cases; P2's diff |
| Review r7, Important 2 | After setup, `CannotRead` offers the visible sign-in at the link, not the form | P3, P4 | P3's after-setup routing test; P4t's per-outcome pins |
| Review r7, Important 3 | The setup split: `Rejected(Wrong)` to the form, `Rejected(Unconfirmed)` (MFA included) and `CannotRead` to the school's page; no false "didn't accept"; never a re-send; confirmed by Quinn on 2026-11-07 | P3, P4 | P3's step-3 tests; P4t's setup pins; §2.3's MFA branch; checkpoint 1 |
| Review r7, Important 4 | The submit target comes from the chosen control (`formaction`, `formmethod`), must be https, listed and `POST` for a navigation submit (refined by review r8, Important 1), and is resolved on every page, before each field and before the submit | P2a | P1t's target cases, the password step's new form among them |
| Review r7, Minor 1 | The conservative rule's costs are stated; `SUBMIT_SETTLE` is its own wait; P5 counts `Rejected(Unconfirmed)` | P2, P5, T3b | §7.4's cost paragraph; P5's count; T3b's value |
| Review r7, Minor 2 | Rejection sets the mark then deletes; save writes then clears; Forget clears the marks only after a delete that worked | P1 | P1t's order cases |
| Review r7, Minor 3 | S6t follows P1t | S6t | the layer table, row 8 |
| Review r7, Minor 4 | The static splice pin is the encoding guard; the round trip is a sanity check | P1t, P2 | P1t's splice pin |
| Review r7, Minor 5 | Wave-B row A5's Forget gains the saved password's deletion | S8, C9 | Quinn's signature on C9's drafts |
| Spike, curation | The school's "wrong password" text is curated row data, empty by default; empty reads as `Unconfirmed` | S2 | S2's `CAMPUSES` test; P1t's empty-text case |
| Spike, 2026-11-07 (the controller's reading) | The founder's sign-in page is one script-submitted form: no `method`, an empty `action`, no `formaction` or `formmethod` | P2a | P1t's script-submit case shaped like it |
| Review r8, Important 1 | Two submit kinds: a navigation submit must be https, listed and `POST`; a script submit is allowed only on a listed https host, its destinations an accepted limit | P2a | P1t's navigation, script-submit and `formmethod="get"` cases |
| Review r8, Important 2 | `wrong_password_text` is matched against visible text only, after `SUBMIT_SETTLE` | P2a, P2b | P1t's hidden-text MFA case; P1t's static visible-text pin |
| Review r8, Important 3 | No wrong password is sent to learn the text; it ships empty unless from public documentation; no `@` or digits-only token | S2, P5 | S2's `CAMPUSES` test; P5's one wrong attempt with nothing read |
| Review r8, Important 4 | The in-flight mark is flushed with `sync_all` before the first credential-carrying submit; a failed flush is `CannotRead` | P1b, P2b | P1t's write-then-flush seam case and its failed-flush case |
| Review r8, Minor 1 | A save takes `school_session`'s lock and answers `Busy` during a fill | P1a | P1t's lock case; P3's concurrency test |
| Review r8, Minor 2 | Both marks are read only after the lock is taken | P1b, P2b | P1t's lock case |
| Review r8, Minor 3 | The in-flight mark is checked only before a fill; a live session still reads grades; the rejected mark with no password opens the form | P1b, P3 | P1t's in-flight cases; P3's two routing tests |
| Review r8, Minor 4 | No push count stated as fact; "keep signing in for you" holds only if R0c is clean | §2.3, P4t, P4 | §2.3's text; P4t's pin that under the MFA branch the form's `Unconfirmed` words carry no "to keep signing in for you" |
| Review r8, Minor 5 | A 307 or 308 redirect from a listed host is an accepted limit | §7.5 | §7.5's accepted-limits paragraph |
| Review r8, Minor 6 | P1 and P2 split into halves of about 80 lines a file | P1a, P1b, P2a, P2b | §4.3's table; the layer table, rows 8 and 9 |
| Review r8, authority | Each relayed ruling cites "ruled by Quinn in session on 2026-11-07, recorded by the controller" | §10, §12 | checkpoint 1's record line |
| Review r9, Important 1 | The submit is the control's `click()` or `requestSubmit(control)`, never `form.submit()`; a script submit's listener always prevents the native default (as refined by review r10, Minor 1) | P2b | P1t's non-intercepting script-submit case (nothing in any URL); P1t's static pins |
| Review r10, Minor 1 | The backstop is a capture-phase submit listener on `window`, added just before the press, that always calls `preventDefault`; the page's handlers still send; the outcome is read from `users/me`; the pin bans `submit` as a property | P2b | P1t's non-intercepting and `document`-handler cases; P1t's widened static pin |
| Review r10, Minor 2 | The in-flight mark is written before the first credential-carrying submit, the username step's included | P1b, P2b | P1t's two-step seam case; P1t's unknown-page-after-username case |
| Review r10, Minor 3 | The session lock is keyed by profile id; `Capturing` stays the process-wide window guard, taken second | S1, S2 | S1's per-profile and `Capturing`-busy lock tests |
| Review r10, Minor 4 | A navigation submit's empty `action` resolves to the page's own URL and takes the same checks, stated in §7.5's rule | §7.5, P2a | P1t's empty-`action` navigation case |
| Review r10, Minor 5 | The status line and checkpoint 2 cite the r9 report | §12 | the citations |
| Review r9, Minor 1 | A navigation submit with an empty `action` resolves to the page's URL and takes the same checks | P2a | P1t's empty-`action` navigation case, listed and off-list |
| Review r9, Minor 2 | S2 exposes `school_session::try_take(profile_id) -> Option<SessionGuard>`, per profile and non-blocking; P1a uses it and does not edit the file | S1, S2, P1a | S1's per-profile lock test; P1a's diff has no `school_session.rs` |
| Review r9, Minor 3 | The 2026-11-07 rulings' record is `docs/notes/2026-11-07-onboarding-rulings.md` | §10, §12 | the citation in §10 and checkpoint 1 |
| Spike, 11-06 | The mailbox does not carry the LMS; r5's outcomes A and B and the shared folder are withdrawn | §2, §4.3 | no task depends on a mailbox sign-in for the LMS; S3 test 4's shared-folder list removed |
| Spike, 11-06 | The LMS lapses about 12 hours from sign-in; keeping it warm extends nothing | P3 | no task keeps a window open between slots; P5's past-12-hours read |
| Spike, 11-06 | Saved LMS cookies are dropped (they died near 3 hours) | every task | no diff stores or replays an LMS cookie; S9 checks |
| Pending, R0c | The plan stands pending the 24-hour run; MFA on a fresh sign-in sends RQ3 back to Quinn | R0c, P4, P5 | R0c's record in the plan's §3; P4 and P5 wait on it |
| Quinn, RQ17 | Sign-out deletes the saved school password, as do Forget and local-data deletion | S5, S6a | S5's sign-out and `delete_local_data` tests in `app/tests/account.rs`, under `CREDMAN_LOCK`; S6t's Forget test |
| Quinn, RQ18 | The Settings row reads "School sign-in: kept on this computer, used for grades and your course list"; the mailbox keeps its own row | S6b | S6t's static pin; P4t's Settings-row pin |
| Spec §1.3, Q1 | "One sign-in" is not met as written; a signed note says so | C9 | Quinn's signature on the note |
| Quinn, RQ7 | `robots.txt` and terms read and dated before a row ships | R0b, C4, C9 | §6.10's check; `checked` date; the lawyer's list |
| Quinn, RQ8 | Class search runs at setup only | C5c | no slot step or scheduler entry in any diff |
| Quinn, RQ9 | A listed no-time section skips the later ask, until the term changes | C2b, C3, C6 | C2b's no-time tests through all three callers, and past a registrar read; C6's pins |
| Quinn, RQ10 | `VISION.md` names the source | done | `VISION.md`'s source list |
| Review r1, 4 and 5 | `registrar:` means the registrar said so; a class-search note hides nothing | C2a, C2c, C10 | C2a's holds-nothing tests; C2c's adoption tests; C10's label pin |
| Quinn's additions | The week strip's "N turned in, kept off your list", from `step3.turned_in` | C7 | C7's pins; shots; T3b's count |
| Quinn's additions | A 12-hour clock is a follow-up, not wave B | §9 | no wave-B task changes a time format |
| Quinn's additions | The timing test runs after the rewrite, on the chosen flow | T3b | §4.6; F15 depends on T3b |
| Quinn's additions | Class search is a per-school curated fact, confirmed by a check at curation | C4, C9, R0b | `CAMPUSES` test; `checked` date; §6.10's check in `docs/reference/app.md` |
| Live run | Session cookies die with the last window; the link signs in hidden | C8, F8b, S4a | C8's tests; R0a; P5 |
| Live run | The registrar's start page never redirects | §5 | follow-up, §9 |
| Live run | Mailbox silent after one MFA sign-in, on its own folder | the mail lane, as built; P4 | P5's mailbox count |
| Live run | Many sections have no begin time | C1, C2b, C6 | C1's and C2b's no-time tests |
| Live run | 15 current-term shells: some not classes | C6, C10, F11e2 | every row needs a press; hint at step 3 |
| Live run | One account's sign-in run is a sample | P3, P5, R0c | §7.4's fallbacks, tested in P3; R0c; a friend's account in P5 |
| Spec §1.3 | Step 3 asks one sign-in | P3, P4, and C9's signed note | P5's count: one typed password, no LMS window |
| Spec §1.5, Q7 | Reveal waits at most 3 minutes; failure is named | C5c | `class_times.apply` never holds the gate; a stuck job does not hold `proposals_ready`; T3b |
| Spec §1.7, Q12 | Skips and costs unchanged | F11e1, F11b | F11t's pins |
| Spec §2.4 | Cookies per host; no capability; scripts per window | C8, S3, S4a | S3's tests; C8's no-cookie test |
| Spec §2.5, Q9 | One Forget; last use deletes; sign-out always | S2, S5, S6a | the plan's S1, S5 and S6t tests, minus the registrar use |
| Spec §4.2 item 1 | Retired by the ruling | C9 | Quinn's signature on the dated line |
| Spec §4.2 item 5 | Typing stays the last resort | C6 | no typing added; existing asks stand |
| Syllabus spec §4.4, E4 | Precedence through the per-source table | C2a | C2a's order tests; RQ1 |
| `VISION.md` commitment 4, 5 | Knowlu noticed: it proposes and waits; the student confirms; no row wears the registrar's name | C2a, C3, C6, C10 | `--class-search` writes no note (C3); C10's label pin |
| `VISION.md` data minimization | One query a course, no bulk read, deleted after setup; no marker carries a section; no backup holds class-search data; no warning prints a key | C4, C2b, C2d, C2e, C5d | C4's cap tests; C2d's and C2e's tests; C5d's delete tests |
| `VISION.md` failures visible | Each course ends in a named state | C1, C3, C6 | C3's report; C6's lines |
| `CLAUDE.md`, commitment proposals never leave the device | Only a confirmed note or a marker syncs, and no marker or card carries a class-search key | C2a, C2d | C2d's search of every note after each route; C2a's successor test. The sync tripwires do not read this, so they are not the proof |
| `CLAUDE.md`, every note write goes through `write` | Adoption's re-key is journal first; nothing re-dumps a note | C2c | C2c's journal-order test |
| `CLAUDE.md` rule 1 | Per-school data, nothing per student | C4, C8, S2, P1 | `CAMPUSES` tests; P1t's derived-target test |
| `CLAUDE.md` credentials | The app writes `knowlu/<profile_id>/<source>`; tests that touch the real Credential Manager hold `CREDMAN_LOCK` and clean up | P1, P1t, S5 | P1t's one real test; S5's tests in `app/tests/account.rs` |
| `CLAUDE.md` desktop safety | No synthetic keyboard or mouse input; the hidden window is never shown | P2 | P1t's fill tests run over a fake page; `contract-reviewer` on P2 |
| `CLAUDE.md` rule 2 | No frozen reference, no fixture touched | every task | F14 checks `engine/tests/fixtures/**` is absent from the diff |
| `CLAUDE.md` `commitments` line | Without `--confirm`, exit 0 and only the series file | C3 | C3's tests |
| `CLAUDE.md` tests | Tests never leave the machine | C4, C8 | `127.0.0.1:0` fakes |
| `CLAUDE.md` contract list | Only `account.rs` is edited, by `contract-engineer`; `credentials.rs` and `wincred.rs` are used, never edited, and any edit there is `contract-engineer`'s | S5, S7, F2c, P1, P2 | S9; F14 checks both files are absent from every diff |

## 12. Checkpoints

1. **Rulings.** Every question is ruled, RQ17 and RQ18 included, so S5 and S6b are free. **Where the
   rulings are recorded:** RQ3, RQ4, RQ5, RQ16, RQ17 and RQ18 were ruled by Quinn in session on
   2026-11-07, recorded by the controller in `docs/notes/2026-11-07-onboarding-rulings.md`. RQ4 and RQ16
   also appear in the plan's §3. **The setup split is ruled:** Quinn confirmed it on 2026-11-07 ("yes to
   all four"), recorded in the same note (§7.4, RQ5's note). It narrows RQ5's "any setup failure shows the
   school's page" by sending a recognised "wrong password" to Knowlu's form. Since `wrong_password_text`
   ships empty (§7.5), at the founder's school every setup failure still opens the school's page. P3 and
   P4 are released from this hold, and dispatch waits only on R0c.
2. **`reviewer`, plan mode,** passed r5 (`docs/reports/2026-11-04-onboarding-replan-review-r5.md`) and
   failed r6 to r9 (`docs/reports/2026-11-07-onboarding-replan-review-r6.md` to
   `docs/reports/2026-11-07-onboarding-replan-review-r9.md`), and **passed r10**, with five minors applied
   in r10's final pass. The review gate is met, and checkpoint 1 is met. Dispatch now waits only on R0c.
3. **R0c's result** goes to Quinn. **Result (2026-11-08): clean.** Three hidden fresh-folder sign-ins with the
   saved password, at 16:17, 19:17 and 22:17 on 2026-11-07, all signed in with no MFA. The run was stopped by
   the harness for low memory before run 4, and Quinn ruled it a success ("start the build, consider it a
   success"). Clean: P4 and P5 go as written. Any MFA page: §2.3's branch, and RQ3
   is asked again before P4.
4. The rulings fold into the plan's §14, and the plan's §7 table gains these rows, by `docs-keeper` on the
   controller's word.
5. After the build: T3b, then F14, then Quinn's go for P5, then the plan's §8 checklist.
