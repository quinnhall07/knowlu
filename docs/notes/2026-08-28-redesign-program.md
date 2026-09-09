# Redesign program — brainstorm record and build order

**Date:** 2026-08-28
**Status (2026-08-29, later):** **S1 BUILT AND LIVE ON `main`** — merged `ef180a8` (32 commits,
subagent-driven, every task reviewed + one whole-branch review + one fix wave), vault migrated
`5e512bd` (178 notes, 178 `system:migration` journal records, 686 tests). Migration report:
`docs/superpowers/reports/2026-08-29-s1-migration-report.md`. Remaining S1 tail: **Task 15**
(cloud routine prompt — `engine.write`/`engine.runs`/`first_proposed_at`/`--via`/`--run-id`;
live + canonical together) and **Task 16** docs (this note, CLAUDE.md, HANDOFF). **Standing rule
(Quinn, 2026-08-29): lose no nuance from this or earlier sessions; when rulings conflict, the most
recent wins.** Next spec: **S2 (surface) ∥ S3 (data quality)**. This file is the running record.

Supersedes the *layout* conclusions of `2026-08-26-today-page-redesign-brainstorm.md`
(Quinn: that mockup "wasn't made with the goal-first, build-backwards mentality"). Two of its
decisions survive as givens (§3); the rest of that note stays valid as verified facts and
plugin survey.

---

## 1. Goal, in Quinn's words (paraphrased from the 2026-08-28 brainstorm)

Sit down in the morning, open **one place**, and see everything the agents have already
aggregated from every source. Work from that page all day. Tick things as they get done and
have them gone on the next run; if he forgets, the agents notice from the source (submitted,
graded, sent) and tick it for him — visibly. Nothing duplicated. Approvals that are actually
worth a decision. A clear visual "am I doing well or badly" (the slack meter). Full autonomy
to change the frontend himself. Everything accessible from desktop and laptop, synced.
And a way to flag anything the AI judged wrong so it can be fixed **later, in one sitting**,
rather than autonomously.

Observed degradation to reverse: quality dropped as sources were added — duplicated tasks
across sources (`GN 103 Reflexion 1` from Blackboard **and** `Erste Reflexion` from Gmail),
event digests stuffed with near-duplicates, approvals for things that need no decision.

## 2. The surface rubric — what "best" is measured against

Quinn explicitly does **not** want an option argued for; he wants the option that scores best
on these. R1 scores every candidate against them.

1. Full capability on the desktop **and** the laptop (phone out of scope)
2. Click-CRUD on tasks; every property editable
3. A write shows up **sorted** quickly, with a visible cue it worked
4. Syncs across devices often; **changes made between syncs are tracked**, so conflicts
   reconcile instead of one side silently losing
5. Free beyond the Claude subscription
6. Easy setup and maintenance
7. Fully customizable by Quinn
8. Everything on one page
9. Run health at a glance (last runs, what worked, what didn't)
10. Online fine; privacy nice-to-have, not required

Accepted: "last-synced" semantics are fine provided sync is frequent and a last-synced stamp
is visible.

## 3. Decisions settled 2026-08-28 (Quinn) — do not relitigate

| # | Topic | Decision |
|---|---|---|
| 1 | Page purpose | The surface he works in **all day** (carried from 08-26). |
| 2 | "Done" | **Progress slid to 100%** on the dashboard (carried from 08-26). Also: completion detected from a source (Blackboard, zyBooks, VHL, email notification) or from a **sent email** that an open task was waiting on. |
| 3 | Health signal | **The slack meter.** Nothing else. |
| 4 | Surface | **B — local web app in the repo, run per machine, engine in-process** (Quinn, 2026-08-28, from the R1 table). Vault stays markdown + frontmatter as the single source of truth; Obsidian becomes an optional editor, not the dashboard. |
| 5 | Sync | GitHub is the sync bus (already true for cloud routine, local runner, Obsidian Git). The surface owns its own pull/push; must not depend on Obsidian being open. Writes between syncs are journaled (§5) so conflicts reconcile per change, not per file. |
| 6 | Approval vs task | **An approval is a *decision*; a task is an *obligation*.** See §4.1. |
| 7 | Events | **No digests.** Each relevant event is its own approval, alive **until the event starts**. Repeated workshop → one card with all dates, showing which fits the calendar. Never propose something already on the calendar; show conflicts with existing commitments on the card. |
| 8 | Approval notes | Every approval has a free-text note field filled at approve/reject time. |
| 9 | Issue notes | On almost every AI-judged object, an icon opens an **issue note**: stable reference to the object, timestamp, snapshot of its properties, Quinn's text. Appended to an issue list. **Nothing acts on it autonomously**; a later Claude session addresses the list in one sitting. |
| 10 | Agent completions visible | "Completed in the past 7 days" section with a **me / agent** mark per item. |
| 11 | Info space | A place for things to know that are neither task nor approval, with an **open/close lifecycle** (e.g. "package is here" opens, "package received" closes). |
| 12 | Grades & GPA | Autonomous GPA tracking against the **AEMX 3.75 floor**, from Blackboard grades + syllabus weights + UA GPA rules, feeding importance/urgency. Quinn: "a hurdle, but possible, and super valuable data for the rest of the system." |
| 13 | Layout | The 08-26 mockup layout is **reopened**; re-derive from goals in S2. |
| 14 | Build order | **S1 → S2 ∥ S3 → S4 → S5 → S6 → S7** (§6). S5 is fifth because it is riskiest, not least valuable. |
| 15 | Process | Research first (R1–R3), consult Quinn on every decision, then spec S1. |

## 4. Derived rules (from Quinn's rulings on real items)

### 4.1 Taxonomy — what becomes what

Rulings given: BUI Discussion Facilitator sign-up → **task** ("technically an assignment");
EDGE membership agreement → **task** ("something I have to do, not AI"); Feminella meeting →
**task** only because no date was set — **anything that adds to the calendar is an
approval**; security/sign-in alerts (Okta, Notion, Namecheap) → **dropped**.

Rule: if the world already decided Quinn has to do it (graded, required, paperwork only he
can sign) → **task**, no yes/no. If he has a genuine choice (optional event, application,
club, anything that puts time on the calendar) → **approval**. If it needs nothing from his
actual life (account/security notifications) → **drop**. Things to *know* → **info**.

Inferred for the rest of the 08-28 queue (Quinn to veto): Supe Store checkout error → task;
Freshman Forum application → approval; Blount Crafting weekly → approval; AEMX seminar → approval
(calendar add); Cursor GitHub permissions → drop.

### 4.2 Event relevance (from the 08-28 digest, six events)

Yes: Career Fair Game Plan (either date), Co-op Info Session, Internship Know-How.
No: "Level Up Your Handshake Profile" (profile-tooling workshop). Big Al's Mystery Night — no,
"not my vibe, but other fun events might be worth proposing."

Rule: substantive prep/info sessions tied to real goals (internships, co-ops, career fair) → yes;
profile-tooling workshops → no; social events → only when they fit — **not encodable from one
example**; the tuning loop is the issue note (§3 #9), not a guess.

### 4.3 Email → typed signals with lifecycles

| Email | Signal | Fate |
|---|---|---|
| Bama Life newsletter | events | one approval per relevant event |
| "Package is here" | info **opens** | info space |
| "Package received" | info **closes** | removed |
| "Missed first UA 101 class" | course fact | recorded on the course note → grade model |
| AEMX seminar dates | calendar dates | one approval per date |
| Quinn sends the email a task was waiting on | completion | task ticked, marked *agent* |
| A sent email expects a reply | waiting-on | "Follow up with X" task after a sensible delay |

Open/close pairs and waiting-on → follow-up exist nowhere in the system today.

## 5. Cross-cutting mechanisms every spec relies on

- **Stable ids** on every object (task, approval, event, info item, issue).
- **Judgment provenance** on everything AI-judged: run id, agent/rule, inputs seen. Without it
  an issue note references a thing that may have been re-judged since.
- **Change journal**: one append-only record per write — object id, field, old → new, when,
  device, actor (Quinn / agent / which runner). Serves: sync reconciliation (§3 #5),
  "completed in the past 7 days" with me/agent mark (§3 #10), what-changed tracking, audit.
- **Run-health record**: per run — runner, start/end, ok/WARN/FAIL, counts, full WARN text
  (the current runner-log truncates and persists nothing).
- **Determinism invariant holds**: the engine consumes journals and provenance; judgment still
  happens only in Claude (routine enrichment, or Quinn).

## 6. Program map — specs in dependency order

| Spec | Scope | Depends on |
|---|---|---|
| **S1 Foundation** | ids, provenance, change journal, issue notes, run-health record, info items with open/close, approval note field, `first_proposed_at` fix. Engine-only, fully testable, no UI. | — |
| **S2 Surface** | The dashboard (R1 pick), click-CRUD, sync, one page: slack meter, schedule/day, work sections, approvals with notes + **pending changes** view, info space, completed-last-7-days, run health, issue icons. Layout re-derived from goals. | S1 |
| **S3 Data quality** | Cross-source task dedup with a source-authority order for which version wins; event dedup (repeat → one card); calendar-conflict check; taxonomy (§4.1) encoded in the routine; no digests; events live until start. Routine prompt + engine. | S1; **parallel with S2** |
| **S4 Completion evidence** | Slider → 100; zyBooks/VHL submission state; Gmail sent-mail matching; agent completions journaled with actor. Blackboard submission joins with S5. | S1, S3 |
| **S5 Grades & GPA** | Blackboard grade capture (local browser runner via CAS session — riskiest piece), per-course grade model from syllabus weights + course facts, UA GPA rules, projected GPA vs 3.75, hook into importance/urgency. Unlocks Blackboard content posts for S7. | S1, S4 |
| **S6 Email intelligence** | §4.3 in full, incl. waiting-on → follow-up. | S1, S3, S4 |
| **S7 New sources & cadences** | Org commitments (ECDC, CS hub), Blackboard content posts, appointment cadence (advisor, contacts, haircut, doctor) via `profile/people.md`. | S5 (content), S6 |
| **S8 Observability** *(added 2026-08-31, §12.3)* | Interaction events, non-events (seen-and-ignored), derivation traceability, retention, a query surface and an inspectable export. Makes the system's own usage analysable — for Quinn now, and for other installs later. Spans surface, engine, runners and routine. | S2 (which emits the events; they cannot be collected retroactively) |

## 7. Research before S1 — findings appended as they land

### R1 — Surface options scored against §2 (2026-08-28, analysis)

Scoring 0–2 per criterion (2 = fully met, 1 = met with real friction, 0 = not met).
Facts relied on: repo is Python 3.14 with only `PyYAML`/`tzdata`/`pytest` as deps; the local
runner already does `pull --rebase → engine → add specific paths → push`
(`scripts/local-run.ps1:23-40`); `state/*` is `merge=union`.

| # | Criterion | A Obsidian + Meta Bind + CSS + Git | B Local web app (engine in-process, per machine) | C Hosted (CF/Vercel) + Actions re-rank |
|---|---|---|---|---|
| 1 | Desktop + laptop | 2 | 2 (clone + venv + start script on the laptop, scripted once) | 2 |
| 2 | Click-CRUD, every property | 1 — properties panel + Templater; not forms; cross-file Meta Bind round trip still unproven | 2 — real forms | 2 |
| 3 | Write → sorted quickly, visible cue | 0 — engine only runs 4×/day; a file-watcher that re-ranks on change is most of B without the UI | 2 — re-rank in-process, ~1s | 1 — 30–90s via Actions + page poll |
| 4 | Frequent sync + journaled between-sync changes | 1 — Obsidian Git syncs files, but Meta Bind writes carry no actor/time and conflicts are file-level and manual | 2 — app owns pull/push; every write journaled with actor/device; per-change reconcile | 1 — GitHub API commits from a Worker; reconciliation logic lives in the Worker |
| 5 | Free | 2 | 2 | 2 (free tiers; quota risk) |
| 6 | Easy setup + maintenance | 1 — plugins on two machines; CSS vs Obsidian styles; cloud render must match | 1 — one more local process per machine (login-start task); no external services | 0 — Worker + token rotation + Actions + Cloudflare Access = three new moving parts |
| 7 | Fully customizable by Quinn | 1 — CSS + Python render; markdown is single-column | 2 — HTML/CSS/JS in the repo, no build step | 1 — same code, plus a deploy pipeline |
| 8 | Everything on one page | 2 (CSS columns) | 2 | 2 |
| 9 | Run health at a glance | 1 | 2 | 2 |
| 10 | Online ok / privacy | 2 | 2 | 1 — needs an auth layer or tasks are public |
| | **Total /20** | **13** | **19** | **14** |

Where B loses its point (#6) is the one honest cost: a process must be running on the
machine you are looking at. Where A and C lose is structural — A cannot do #3 at all, C makes
#6 worse in exactly the way Quinn said he does not want.
Hybrid noted for later: B now, plus a read-only static publish of the same page if a
phone/anywhere view ever matters (adds nothing to the write path).
**Decision: Quinn's, pending.**

### R2b — Gmail sent-mail matching (2026-08-28, measured)

- The Gmail account holds **incoming** Crimson mail only (`label:Crimson`, id
  `Label_2588503205049924458`, all addressed to `dqhall@crimson.ua.edu`). `in:sent` over 14
  days shows **2 messages, both tutoring, both from `quinn.hall.scho@gmail.com`**; every UA
  reply in `in:sent` predates enrollment (Jan–Apr 2026). **Mail Quinn sends from the Crimson
  (Outlook) account is invisible to the system.** A Microsoft 365 connector exists in this
  environment and could read Crimson Sent Items — **decision needed from Quinn** (§8).
- Matching is deterministic when it works: a Gmail-created task carries
  `source_uid: gmail:<message-id>`; a reply Quinn sends lands in the **same threadId**. Rule:
  "a SENT message from Quinn exists in the task's source thread, dated after the task was
  created" → completion, actor = agent. For send-to-X tasks (not replies) the task must carry
  an expectation (`expects: {kind: sent_email, to: <address>}`) set at enrichment; match on
  recipient + date.
- Target today is small: 4 email-shaped tasks in `tasks/` (2 already done by hand). The value
  is the rule, not the volume.

### R2a — zyBooks / VHL completion evidence (2026-08-28, live payloads via the existing fetchers)

Raw payloads (tokens redacted) and probe scripts are in the session scratchpad, not the repo.

**zyBooks — deterministic, zero extra requests.** The assignments payload the ingest already
fetches carries `section_scores[]` per assignment — per-user earned points per section
(`participation_earned`, `challenge_earned`, `lab_earned`), currently **ignored**
(`engine/zybooks.py:69-133` parses `sections[].total_points` for the body only). Rule:
`earned = Σ section_scores`, `total = Σ sections[].total_points`,
`progress = round(100·earned/total)`, **done ⇔ earned ≥ total** (float-tolerant; total 0 → not
derivable). Live today: HW 01 = 193/193, Lab 01 = 10/10, Lab 02 = 5/15, the other 21 unscored.
No `submitted`/`completed`/timestamp field exists; per-activity state is instructor-only (403).
Ambiguities: a lab with one failing test sits at `earned < total` forever (indistinguishable
from in-progress); late vs on-time is invisible; sections shared between assignments count
toward each.

**VHL — deterministic, one extra GET per open bucket, same host.** The dashboard's
`data-assignment-summaries` lists only buckets with `due_date ≥ today`; past-incomplete
buckets are at `past_assignment_summaries`; **completed buckets appear in neither** — so
`percentage_complete: 100` is only ever visible if a bucket finishes before its due date and a
run lands in between. Safe rule: `GET {detail_url}` with `Accept: application/json` (the
`detail_url` is already parsed) → `groups[].status ∈ {not started, partial, complete}` with
`completed_count/assigned_count`; **done ⇔ every group `complete`**; progress =
Σcompleted/Σassigned. Verified live on four buckets. Granularity is per group (lesson block),
not per activity. `complete` means activities finished, not credit earned (`can_be_started`
stays true after the deadline; GN 103 gives no late credit); no timestamps. The absence-based
alternative (one GET to `past_assignment_summaries`, "in neither list ⇒ complete") is cheaper
but a truncated response would mark work done — rejected.
The VHL opener is built inside `login_and_fetch_dashboard` (`engine/vhl.py:234-250`) and not
returned; S4 needs it exposed for the follow-up GETs. No new egress: both hosts are the ones
the ingest already talks to.

**Stale right now, proving the value:** `tasks/cs-100-hw-01.md` and `tasks/cs-100-lab-01.md`
are `active, progress: 0` while zyBooks reports 100%; `tasks/gn-103-hausaufgaben-2026-08-26.md`
(`progress: 10`) and `-27.md` (`progress: 0`) are `active` while VHL reports complete. The
invariant S4 changes is `engine/coursework.py:193-194` ("progress is never refreshed") and the
spec row at `docs/superpowers/specs/2026-08-25-coursework-ingest-design.md:143`; the rule only
ever moves active → done and never lowers `progress`.

### R3a — UA GPA rules + the AEMX floor (2026-08-28, web research, primary sources)

Catalog: https://catalog.ua.edu/undergraduate/about/academic-regulations/records/grades-grade-points-gpa/

- **Plus/minus counts.** ±0.33 per hour: A+ 4.33, A 4.0, A- 3.67, B+ 3.33, B 3.0, B- 2.67,
  C+ 2.33, C 2.0, C- 1.67, D+ 1.33, D 1.0, D- 0.67, F 0. F and IN take no modifier.
  **Reported cumulative GPA is capped at 4.0** (A+ still earns 4.33 quality points).
- **Formula:** total quality points ÷ total GPA hours, rounded to 3 decimals. Credit hours
  weight each course.
- **Excluded:** IN, P, W, NA, NC. An IN not resolved by the end of the next regular semester
  becomes F.
- **No grade forgiveness.** Every attempt of a repeated course stays in the GPA (only the last
  counts for earned hours).
- **Two GPAs:** overall (incl. transfer work, incl. transferred F's) vs institutional/UA
  (UA coursework only). AP/IB/CLEP credit is not UA coursework and (registrar practice, not
  catalog text) carries no quality points. Dual-enrollment enters the *overall* GPA only.
- **Letter → percentage is per-syllabus** — no university scale. Confirms the weights already
  extracted into `courses/*.md` are the right input.
- **AEMX = Alabama-Esslingen-Mercedes-Xchange** (College of Engineering; Mercedes-Benz USI +
  Hochschule Esslingen). Requirement, verbatim from the admissions page: **"minimum 3.75
  cumulative earned GPA while at UA"** — cumulative, not term.
  https://eng.ua.edu/academics/alabama-esslingen-mercedes-xchange/admissions/
  **ANSWERED BY QUINN 2026-08-29 — this supersedes the published wording, which is misleading:**
  the requirement is **3.75 PER SEMESTER (term GPA), checked each semester**, with **a little
  leeway if he has performed well previously — "but not too much."** No email needed.
- Adjacent: scholarship renewal (Presidential/National Merit) is a 3.0 cumulative UA GPA + 67%
  completion, reviewed each semester after grades finalize, with a one-semester warning.
- **Where grades live:** item-level in-progress grades exist **only in Blackboard**; myBama
  carries instructor-submitted midterm and final letter grades. Grade capture (S5) must read
  Blackboard; myBama is the audit source at term end.

**Design consequences for S5 (revised 2026-08-29 after Quinn's answer):** the model is
quality-points ÷ GPA-hours with ±0.33 modifiers, a 4.0 display cap, and a per-syllabus
percentage→letter map. **The primary projection is the CURRENT TERM's GPA against 3.75, not the
cumulative** — the binding constraint is per-semester, so a strong past cannot rescue a weak term
except by a small, discretionary margin. Practical shape: project the term GPA continuously from
in-progress Blackboard grades + syllabus weights; surface *how far the term is from 3.75* and
*which course is dragging it*; and — because the leeway exists but is thin — treat a projection
below 3.75 as a real signal rather than waiting for it to be certain. Cumulative GPA is still
worth showing (scholarship renewal is a 3.0 cumulative UA GPA, reviewed each semester) but is
secondary. This also raises the value of catching a bad grade EARLY in a term, which is the
argument for wiring grade capture into ranking rather than only reporting it.

### R3b — Blackboard grade-access feasibility (2026-08-28, web research; nothing probed live yet)

**Verdict: feasible, via Blackboard's public REST endpoints called with the browser session
cookie from a dedicated Playwright profile.** Consistent with design doc §8 "Blackboard session
strategy" (dedicated profile, ride the CAS session, Duo remember-device).

- **Endpoints (student-permitted):** `GET /learn/api/public/v1/users/me` → id;
  `/v1/users/{id}/courses` → memberships; `/v2/courses/{courseId}/gradebook/columns`;
  **`/v2/courses/{courseId}/gradebook/users/{userId}`** — every grade for one user in one
  course, the single best call. Students without the gradebook-modify entitlement get a
  restricted set: `userId, columnId, status, text, score, exempt, feedback`. Attempt status
  ∈ `InProgress, NeedsGrading, Completed, InProgressAgain, NeedsGradingAgain`; "not attempted"
  = no grade row. Column carries `score.possible`, `grading.type` (Attempt/Calculated/Manual),
  `grading.due`, `gradebookCategoryId`, `includeInCalculations`.
  Sources: https://docs.anthology.com/docs/blackboard/rest-apis/hands-on/pulling-gradebook-data-and-assessment-grades ;
  https://github.com/mcharris/blackboard-rest-php/blob/master/docs/Api/CourseGradesApi.md
- **Auth:** official route is OAuth (institution-registered app — unavailable to a student).
  **Community evidence that the `BbRouter` session cookie alone works** on these endpoints:
  https://github.com/p-doyle/Python-UARK-Blackboard-API (a UARK student project, no OAuth).
  Undocumented → must be proven on UA's tenant by the 5-minute probe below.
- **Session longevity:** Learn SaaS idle timeout 15–480 min (default 180), absolute 3–24 h;
  no remember-me. CAS TGT typically ≤ 8 h. **Assume every run re-logs in** with the password
  from Credential Manager (same pattern as zyBooks/VHL). **Duo:** UA remembers a device for
  **30 days per browser** (cookie), unavailable while Duo auto-push is on
  (https://oit.ua.edu/software/duo/duo-faqs/). No headless Duo re-auth exists → **one human
  tap roughly monthly**, and a visible WARN when the session dies (spec §8 "session-health
  check" survives).
- **Options:** (a) Playwright `launch_persistent_context` on a dedicated dir, REST GETs via
  `context.request` (shares the jar; no cookie harvesting, no XSRF) — **recommended**;
  (b) reading the real browser's cookies — **rejected** (Chrome 127+ App-Bound Encryption,
  fragile per update, and the session dies within hours anyway); (c) session-free export —
  **does not exist** (grade download is instructor-only; the .ics carries due dates only).
- **Overall Grade:** a `Calculated` column, visible to students only if the instructor
  enables it; when shown, its *value* arrives via Get User Grades, but the formula/weights are
  privileged-only. → S5 computes projections from the syllabus weights in `courses/*.md` and
  uses the instructor's Overall Grade, when present, as a cross-check, never as the model.
- **New dependency:** Playwright + a Chromium download (free; ~150 MB). First in the repo.

**Probe RUN by Quinn 2026-08-29 (browser, session cookie only) — cookie auth WORKS on UA's
tenant.** Output kept outside the repo (`~/Downloads/bb_probe.txt`; contains ids and grades).
- `/v1/users/me` → JSON (`id`, `userName`, `institutionRoleIds: [STUDENT, BbMobile]`,
  `nodes[].title` = the declared major — "Cyber Security, Bachelor of Science").
- `/v1/users/{id}/courses` → 19 memberships; 8 `available: Yes` with `lastAccessed`, the rest
  `Disabled` (orientation/old shells). `courseId` is the key for the gradebook calls.
- `/v2/courses/{GN 103}/gradebook/users/{id}` → one row for the one attempted item:
  `{userId, columnId, status: "NeedsGrading", exempt: false, changeIndex}` — the restricted
  student subset, exactly as R3b predicted. Unattempted columns have **no row**. `score` will
  appear once graded (not yet observable: nothing is graded).
- `/v2/courses/{GN 103}/gradebook/columns` → 3 columns (Erste Reflexion 100 pts due 08-27,
  Kulturpass 1 = 10, Kulturpass 2 = 100), each with `grading.type: Attempts`, `grading.due`,
  `gradebookCategoryId`, `includeInCalculations: true`. **No Overall Grade column** in that
  course yet.
- Bonus for S3 — **corrected 2026-08-29 after checking the notes**: the duplicate pair is
  `tasks/task-erste-reflexion.md` (`created_by: blackboard`, `source_uid` = gradebook column
  `_4686399_1`, the same columnId the probe returned) and `tasks/gn-103-reflexion-1.md`
  (`created_by: claude`, **no `source_uid`** — extracted from the GN 103 syllabus on 08-19,
  along with `-reflexion-2` and `-3`, which **will duplicate again in Oct and Dec**). Gmail was
  *not* the cause: `state/ingest-seen.md:74` shows the Gmail routine recognised the "New
  Assignment (Erste Reflexion)" mail as already tracked. Same disease as HANDOFF docket item 0
  (CS 100 Projects 2–5): **syllabus-derived notes carry no external key, so every source that
  later posts the item creates a twin.** S3's dedup must reconcile keyless notes against
  incoming items by course + due-date proximity + title similarity — and `created_by` +
  `source_uid` on every note is exactly the provenance that made this diagnosis possible.
  `created_by` today: blackboard 30, claude 65, zybooks 21, gmail 13, vhl 10, events 1.
**Consequence:** S5 is the clean design (persistent Playwright profile + these four
endpoints); no HTML scraping. Still to verify when the first grade posts: `score` present
in the student row. Quinn's note: the probe worked *because a logged-in tab existed in the same
browser* — that is the mechanism; the runner reproduces it by logging in itself.

**CORRECTION 2026-08-29 (Quinn asked "isn't UA switching to Okta?" — yes):** the CAS/Duo
assumptions above are stale. Per UA OIT, myBama **and Blackboard moved to Okta SSO on
2026-05-10** (login = UA email + password, new page), and **Duo is replaced by Okta Verify on
2026-09-14**. So: **Duo auto-push-off is withdrawn** (moot in two weeks); S5's login step is an
Okta flow, and its unattended-MFA story depends on Okta's session/"stay signed in" policy as UA
configures it — **re-research after 09-14, before S5's spec**. The REST-with-session-cookie
finding is unaffected (SSO only changes how the session is obtained).
Sources: https://oit.ua.edu/software/okta-verify/okta-transition/ ;
https://cit.ua.edu/upcoming-blackboard-login-change-5-10-26/ ;
https://oit.ua.edu/2026/04/03/upcoming-changes-to-university-login-experience/

**Okta facts gathered 2026-08-29 (UA OIT pages + UA Confluence):**
- Timeline: 05-10 myBama + Blackboard → Okta SSO; 05-17 all Shibboleth/Entra apps; 06-15
  Okta Verify available alongside Duo; **09-14 Duo unavailable**.
- Sign-in portal **`myapps.ua.edu`**; sign-in address = `<mybama>@crimson.ua.edu`.
- Enrollment (recommended "phone only"): install Okta Verify → Get Started → Add Account →
  **Organization** → **Skip** (not "Add Account from Another Device") → **"No, Sign In Instead"**
  → Sign-In URL `myapps.ua.edu` → full address → verify with **Duo (until 09-14)** or password.
  Source: https://bama.atlassian.net/wiki/spaces/OKB/pages/4462575627
- Authenticators UA lists: Okta Verify **push**, Okta Verify in-app **passcodes** (offline),
  **Okta FastPass** (device biometric), self-service **temporary passcodes** (72 h, single use).
  Multiple devices allowed via Okta Verify settings. **Not listed:** Google Authenticator /
  generic TOTP, SMS, security keys. Nothing published about "keep me signed in", remembered
  devices, or prompt frequency. Source: https://oit.ua.edu/software/okta-verify/okta-verify-faqs/
- **Implication for S5's unattended login:** the Okta Verify passcode secret is app-bound (not
  a standard otpauth secret), so a pyotp-style runner needs a *generic TOTP* authenticator UA
  does not list. Realistic design = persistent browser profile riding UA's Okta session /
  remember-device policy + a human push-tap when it lapses (as with Duo, cadence unknown) —
  **unless** Quinn's Security Methods page shows Google Authenticator, or FastPass on this PC
  proves to work headlessly. Both are things only Quinn's account can reveal (§9).
- quinn-ops itself touches neither Duo nor CAS today (Blackboard = token .ics URL; zyBooks/VHL
  have their own logins), so "migrating the system" = nothing; migrating **Quinn** = enrol
  Okta Verify before 09-14.

## 10. S3 Data-quality decisions (brainstormed with Quinn 2026-08-29)

**Root cause, corrected.** Both known duplicates are the same bug and neither is
Blackboard-vs-Gmail: **a note created from a syllabus by Claude carries no external key**, so the
next ingest that posts the same obligation sees no note holding that uid and creates a twin.
`GN 103 Reflexion 1` was syllabus-derived; `cs-100-project-2..5` are in that state right now.
Reflexion 2 and 3 repeat it in Oct/Dec.

| # | Topic | Decision (Quinn, 2026-08-29) |
|---|---|---|
| S3.1 | Binding | Before creating, an ingest checks for an existing note that is plainly the same obligation (same course, due within a day, title clearly the same) and **attaches its uid to that note** instead. Confident matches bind automatically and carry an issue icon; ambiguous ones create separately, marked `possible duplicate of <id>`. Never an approval — a duplicate is the system malfunctioning, not a life decision. |
| S3.2 | Progress on bind | **Sources may set `progress`** — reversing the old "never overwrite" rule. Upward always applies (60% observed on a note at 0 → 60; 100% → ticks complete with the **agent** mark). **Once Quinn touches the slider it becomes a floor**: sources may raise, never lower. A downward disagreement is displayed on the card (`you: 100% · zyBooks: 60%`) with the issue icon, never applied. |
| S3.3 | Field freeze | Generalised per-field: **any field Quinn has edited is frozen against sources; any field he has not keeps syncing.** Rename a task and Blackboard stops clobbering the title while its due date still updates. Uses the S1 journal to tell which is which. |
| S3.4 | Field authority | Identity fields (title, effort) follow a ladder: **zyBooks/VHL > Blackboard > Gmail > hand-written**. |
| S3.5 | Due dates | **Earliest wins, whoever said it**, with the other date shown on the card. Rationale: a stale-later date costs a grade, a stale-early date costs an afternoon. Neither source is reliably ahead of the other. |
| S3.6 | Date-change override | An **explicit change event** ("extended to Sunday") beats earliest-wins even when later, because it is evidence of a change rather than a stale reading. Carries its evidence — the sentence, who said it, when — and the card shows it. **An override pushing a deadline LATER is flagged, not applied quietly.** Producer in S3 = Gmail (the routine already reads both mailboxes daily). **Blackboard announcements cannot land until S5's authenticated session exists → S7.** A date changed in Blackboard itself already flows through the ICS feed as a normal reading. |
| S3.7 | Event relevance | **Profile as the floor, decision history as the correction.** `profile/interests.md` (exists) + new `profile/me.md` (written 2026-08-29) give the cold start; the S1 journal's approve/reject records with Quinn's notes tune it. This is what turns "wasn't my vibe" on Big Al's Mystery Night into a rule without Quinn having to state one. |
| S3.8 | Calendar dedup | **Match on time first, title second.** Already-on-calendar = something occupies that start slot AND the titles are plausibly the same. A different date of a repeating session is a separate proposal. **Title alone never suppresses.** Time matches but title does not → still propose, showing *"you have something at this time: <event>"* on the card. |
| S3.9 | Approval lifecycle | Undated opportunities **lapse after 14 days**, archived not deleted, shown as lapsed in the last-7-days view so a filter eating wanted things is visible. Events already die at their start time. |
| S3.10 | Standing invitations | A recurring invitation (Blount Crafting, weekly Friday noon) is **answered once for the series**: approve → a recurring commitment; reject → the whole series suppressed at the source. Not a card per week. |
| S3.11 | Queue cleanup | S3 applies rulings already given to the live queue: **drop** the 3 security-alert approvals (Notion/Google, Cursor GitHub permissions, Microsoft/Stripe), **convert** Supe Store checkout error to a task, **retire** both `events-digest` approvals. 6 of the 11 pending approvals are noise by Quinn's own taxonomy. |
| S3.12 | Task zero | Stamp `source_uid` on `cs-100-project-2..5` before zyBooks posts P2 on **Mon 2026-09-07**. Controller ruling 2026-08-29: if S3 has not landed by **Fri 2026-09-04**, hand-stamp instead. |

**Profile.** `profile/me.md` written 2026-08-29 (goals, fixed points, capacity, what earns a yes).
Quinn is a **Cybersecurity major** (College of Engineering, CS department) — not CS; security
talks, CTFs and cyber teams are major-relevant and were added to `interests.md` `strong`.
Confirmed by silence: this year's goals are undergraduate research, an internship or co-op, and
the career fair; tutoring and SetNForget stay **capacity context only**, outside the ranking.

## 8. Still open — to raise with Quinn at the right spec

- **Which account does Quinn send academic mail from?** If Crimson/Outlook, sent-mail
  completion (S4/S6) needs the Microsoft 365 connector attached to read Sent Items; if Gmail
  with a send-as alias, the existing connector suffices. (R2b)
- ~~**AEMX 3.75:** which GPA, when checked, any grace~~ — **ANSWERED 2026-08-29:** 3.75 **per
  semester**, checked each semester, small discretionary leeway for a strong prior record. S5
  projects the term GPA as the primary number. (R3a)
- What "fully customizable" means in practice for S2: editing HTML/CSS/JS in the repo, a
  config-driven layout, or both.
- Source-authority order for dedup (S3): proposed zyBooks/VHL > Blackboard > Gmail > hand-written
  for title/due; `progress` never overwritten.
- The "appropriate amount of time" before a follow-up task (S6) — per-recipient class?
- Whether Meta Bind `enableJs` matters at all (only if Obsidian survives R1).
- The 08-26 day-plan section question is moot unless the R1 pick keeps `today.md` as the
  primary surface.

## 9. Decisions pending with Quinn (as of 2026-08-28, end of research)

1. **Surface — DECIDED: B** (Quinn, 2026-08-28).
2. **Sent-mail visibility — DECIDED 2026-08-29: classic Outlook desktop on this PC, signed
   into Crimson and left logged in, read locally by the runner via Outlook's COM interface
   (`Outlook.Application`).** Quinn keeps using New Outlook day-to-day; classic is install-only.
   Background: connecting the crimson.ua.edu mailbox to Claude is **blocked by UA**; the Outlook
   after-send Cc rule **does not work**; Phone Link has no API and carries no mail. Fallback if
   COM fails with the M365 account: the S5 browser session reading Outlook Web's Sent Items.
   Gmail "send as" the Crimson alias is rejected (DMARC → professors' spam). Lands in S6; the
   local runner only (desktop), like coursework ingest.
   **PROBED 2026-08-29 — works.** Quinn signed classic Outlook (16.0) into Crimson; a read-only
   PowerShell COM probe (`Outlook.Application` → MAPI namespace) found one Exchange store
   (`dqhall@crimson.ua.edu`, cached mode), the default Sent Items folder with **62 items**, the
   newest sent 2026-08-29, each with `SentOn`, `Subject`, recipients and a 32-char
   `ConversationID` — the last being the key that matches a sent reply to the incoming
   message a task was created from. One implementation detail for S6: `MailItem.To` returns
   display names; SMTP addresses come from `Recipients[i].PropertyAccessor`
   (`PR_SMTP_ADDRESS`). Constraint: classic Outlook must be running (COM launches it if not)
   in the same Windows session as the runner — the Task Scheduler job must run interactively,
   not as a service.
3. **Blackboard probe — DONE 2026-08-29, cookie auth works** (R3b addendum). The Duo
   auto-push step is **withdrawn**: UA replaces Duo with Okta Verify on 2026-09-14 (Quinn has
   already switched); S5's unattended-login design is re-researched after that date.
   **Okta session probe — Quinn logged in 2026-08-29 15:23Z and it WORKED end-to-end:** the
   dedicated Playwright/Chromium profile authenticated through Okta and
   `/learn/api/public/v1/users/me` returned `userName` from `context.request` — the exact S5
   mechanism, proven on UA's tenant. Cookie lifetimes observed: **`DT` (Duo device trust) expires
   2027-10-03**, `ln` 2027-08-29, `luf_*` 2026-09-28 (30 days), while **Okta's `idx` and
   `JSESSIONID` are session-only** — so persistence rests on the device-trust cookies surviving in
   the persistent profile, not on the session cookie. The scheduled probe
   (`C:\Users\danie\okta-probe`, every 2 h for a week) now measures how long that actually holds:
   rows will read `signed_in`, `password_prompt`, `mfa_prompt` or `refreshed_silently`. **Read
   `results.csv` before designing S5's login step.**

**S1 design review (in chat, 2026-08-28/29):** §1 objects-and-locations **approved** (journal =
JSONL per day in `state/journal/`, behind an `append/read` seam; markdown notes for issues and
info); §2 sync reconciliation **approved** (per-field later-timestamp-wins, losers journaled as
`superseded_by`, ties → human; applies to `progress` too); §3 provenance + issue notes
**approved with two amendments** — issue notes carry category chips **and** free text
(both, always), and the routine **may re-propose** (as an approval, never a silent overwrite)
when its inputs change materially, even for a field Quinn has set; §4 run-health records +
info items (with `close_key`) **approved**; §5 migration + testing **approved** (2026-08-29).
**S1 design fully approved → spec next:** `docs/superpowers/specs/2026-08-29-s1-foundation-design.md`.

---

## 11. S2 Surface decisions (brainstormed with Quinn 2026-08-30/31)

Everything below is settled unless marked OPEN. Read §11.3 before touching any surface file —
it constrains the whole stack. The chosen mockup is
`docs/mockups/2026-08-31-s2-console-CHOSEN.html`; open it in a browser before reading further.

### 11.1 Form — a native desktop window

**DECIDED (Quinn, 2026-08-31): "I want a real native window."** Not a browser tab, not a PWA.

- **pywebview 6.2.1** hosting **Edge WebView2** (built into Windows 11 — nothing to install).
- **One process**: a stdlib `http.server` bound to `127.0.0.1:8765` on a daemon thread, and the
  webview window on the main thread. One icon, one launch, and closing the window stops
  everything — there is no "is the server running?" problem and no Task Scheduler entry.
- Browser fallback: while the app is open, `localhost:8765` also works in Chrome.

**PROBED LIVE 2026-08-31 on Quinn's desktop — it works.** Python **3.14.2**; `pythonnet 3.1.0`
ships a **cp314 wheel** (this was the real risk and it is cleared); nothing compiled from source.
The window rendered the mockup correctly (`document.title` = "Ops Console", 39 spill blocks
present, `--acc` resolving to `#3FB68B`), reported `Chrome/151.0.0.0`, and closed cleanly.
Probe script pattern: `webview.create_window(...)` + `webview.start(fn, win)`, with the callback
using `window.evaluate_js(...)` to assert on the DOM, then `window.destroy()`.

**Dependency boundary — do not breach.** `CLAUDE.md` says the engine is stdlib + PyYAML + tzdata.
That stays true:
- `engine/` imports nothing new. `engine/serve.py` (the JSON API) is stdlib only.
- `app/window.py` is the **only** file that imports pywebview, and nothing in `engine/` imports it.
- `requirements.txt` unchanged; **`requirements-desktop.txt`** holds the GUI tree
  (pywebview, pythonnet, clr_loader, cffi, pycparser, proxy_tools, bottle, typing_extensions —
  eight packages, all wheels). The cloud environment and the local runner never install it.

**Rejected:** Electron (~150 MB, npm, a second runtime to maintain); Tauri (adds Rust and a build
step, so the shipped app stops being the files in the repo — directly against §11.3);
browser-only PWA (Quinn asked for a real window).

### 11.2 Stack

No build step, no bundler, no framework, no minification. Plain HTML + CSS + vanilla JS.
The server exposes a small JSON API (`/api/today`, `/api/task/<id>`, …) and the page renders it.
Rendering client-side (rather than server-rendered templates) keeps the shipped page identical to
the source file, which §11.3 requires.

### 11.3 What "extremely customizable" means — DEFINED by Quinn, 2026-08-31

> "I can open a Claude session and have things understood and altered by AI easily."

This is an architectural constraint, not a preference. Consequences:

- **No JSX, hooks, or Tailwind class soup.** "Make the decisions deck wider" must not become
  archaeology across forty utility classes or a trace through component state.
- **The file you read is the file that ships.** No generated CSS, no compiled output.
- **Tokens live at the top of one stylesheet**, commented with what each step is *for*, so a
  session can restyle the whole page by editing eight numbers.
- **`docs/surface/anatomy.md`** (to write) explains what each region is and why, so a cold
  session gets the intent rather than just the markup.

### 11.4 Not keyboard-driven — Quinn, 2026-08-31

Drop `J`/`K`, `A`, `⌘K` and the keyboard hint block from the sidebar. Consequences: adding a task
needs a **visible "New task" button** in the main column header (opening an inline row, not a
modal), and every property is edited by clicking it directly.

### 11.5 Chosen direction — "Ops Console"

Three full directions were built and compared at real data. Quinn picked **Console**.
The other two stay in `docs/mockups/` as reference, not as dead ends:
- `2026-08-31-s2-runway.html` — dark instrument panel, amber as the data ink, capacity as a
  runway with a threshold. **Its threshold strip is worth stealing** into Console later.
- `2026-08-31-s2-ledger.html` — light, print/Tufte, greyscale plus one red for deficit only,
  double-entry arithmetic. The only direction where the numbers are auditable.

Earlier rejected attempts and *why*, so they are not re-proposed: a card-and-shadow dashboard
("I kind of hate it… even the old proposal was better" — generic, ignored horizontal space); a
mono terminal version ("the second one's colours were ugly, it was just a list of things" —
five hues used decoratively, uniform density so nothing grouped).

### 11.6 The signature element

**Capacity as a segmented meter.** One block = 30 minutes. Blocks that fit sit inside a bordered,
accented container; blocks that do not are drawn **outside** it in outline red. Today that reads
**10 in, 39 out**. It answers "does today fit?" before any number is read, needs no metaphor to
explain, and is specific to Quinn (a 4h/day budget against a freshman load).

Chosen over the day-shape timeline and the course heat strip. Quinn: "I like your candidate."

### 11.7 Layout

| Region | Width | Holds |
|---|---|---|
| Left rail | 264px | brand, nav with counts, **THE DAY** (the calendar) |
| Main | measure 900px, shell 1560px | topline, verdict `h1`, lede, capacity meter, **MUST DO** grouped by horizon |
| Right rail | 356px | **DECISIONS** (deck), **AHEAD**, **GOOD TO KNOW**, **CLOSED THIS WEEK**, **RUNS** |

- **The calendar moved into the left rail** (Quinn, 2026-08-31). Laid out vertically because the
  column is narrow: time above, block below, scheduled work nested under each open slot.
- The page is a **plan, not a feed** (settled 2026-08-30) — stable and ranked, with the delta as
  one thin line at the top rather than a reordering stream.
- Main is capped at 900px. Without that cap the `1fr` column stretched to the viewport and left
  an ~800px void between a task title and its hours at 1920.
- Shell capped at 1560px: past that the page gains **gutters, not width**.
- Breakpoints: **1400** (rails narrow), **1180** (right rail becomes a full-width auto-fit grid —
  each `.rblock` is one grid item so headings never detach from their content), **820** (single
  column, nav becomes a wrapping strip, titles wrap instead of truncating, progress column drops).

### 11.8 Decisions are a DECK, not a list — Quinn, 2026-08-31

> "Make sure the decisions don't just push the rest of the content down forever. At >3, they need
> to create a stack of cards that slide up as decisions are made."

One card at a time, two card edges peeking behind it, and a count ("4 behind"). Deciding the top
card animates it out and slides the next up; page height never changes however long the queue is.
When empty it says so rather than leaving a hole.

The note field is **part of the card and always visible**; a single click commits. An earlier
two-step version changed the button label to "Confirm" on first click — that breaks the rule that
an action keeps its name through the whole flow, and was removed.

### 11.9 Course load CUT, "Ahead" added — Quinn, 2026-08-31

> "I'm not sure what the course load section is for. It confused me."

Correct instinct: it answered "where does the semester's weight sit", which is real but not a
question asked at 8am, and nothing about today changes because of it. **Cut.**

Replaced by **AHEAD** — hours actually due per day for the next fortnight, computed from every
live task note (overdue work lands on today). Real figures as of 2026-08-29:

```
Sun 8/30  0.0 | Mon 8/31  5.4 | Tue 9/1  3.2 | Wed 9/2  0.7 | Thu 9/3  3.3 | Fri 9/4  5.8
Sat 9/5   0.8 | Sun 9/6   0.0 | Mon 9/7  0.0 | Tue 9/8  5.1 | Wed 9/9  3.1 | Thu 9/10 3.5
Fri 9/11  8.7 (7 items — the wall)
```

The takeaway line under it is the point: *Fri 11 Sep is the heaviest day in a fortnight; Sun 6th
and Mon 7th are empty — that is where it goes.*

The PH 106 finding (**35.0h hidden behind only 3 tasks**, fourth-heaviest course by hours but
lightest-looking by count) is genuinely valuable and must not be lost — it belongs as an
**occasional alert when a course hides many hours behind few tasks**, not as a permanent panel.

### 11.10 Design system (as built in the chosen mockup)

- **Surfaces, four lifts:** canvas `#0A0B0D`, `#101215`, `#15181B`, `#1A1D21`;
  hairlines `#22262B` / `#30353B`.
- **Text, four levels carry the hierarchy — not colour:** `#F1F3F4`, `#C3C9CE`, `#878E95`, `#5A6169`.
- **One accent**, interactive state and "this fits" only, never decorative: `#3FB68B` (dim `#15302A`).
  Semantic only where it encodes data: critical `#E0605A` / `#33191A`, warning `#D9A441` / `#302512`.
- **Type:** Instrument Sans (UI) + JetBrains Mono (all figures, times, ids). Deliberately not
  Inter — Linear's own face, and the flagged "safe" default.
- **Spacing scale, 4px base, every gap on the page comes from it:** 4 inside a control · 8 label
  to value · 12 row padding and column gap · 16 related rows · 24 heading to body · 32 blocks in a
  column · 48 between sections · 64 between zones. Radii 4 / 6 / 8.
- **Deliberately single-theme dark** — a console is dark. `color-scheme: dark` unconditionally,
  and every colour painted from a token so the page never borrows the host background.
- Discipline borrowed from Linear (studied 2026-08-31): restraint at scale, one accent used only
  for interactive state, density managed through gradations of near-white rather than colour.

### 11.11 Three features MISSING from the mockup that S2 must build

Flagged 2026-08-31 so they are not discovered late. All three were asked for explicitly:

1. **⚑ Issue-flag icons on every AI-judged object.** Click → capture the object, its properties,
   the judging run, a timestamp, category chips **and** free text (both, always) → append to the
   issue list. **Nothing acts on it autonomously**; a later Claude session clears the list in one
   sitting. Present in the terminal draft, dropped in Console.
2. **The delta line.** "since 08:04 — 2 ticked by agent · 3 new approvals · CS 100 P2 date moved."
   One line, expandable, at the top. Console currently shows only sync status.
3. **Full click-CRUD.** The mockup only moves progress and answers approvals. Needed: create a
   task, edit **every** property inline, delete.

### 11.12 Rendering discipline — look at it, do not reason about it

`scripts/mockup-shots.py` renders a mockup at eight viewports, reports horizontal overflow, and
screenshots each. **Use it before claiming a layout works.** Three bugs in the 2026-08-31 session
were invisible in source and obvious on screen:

- an ~800px void between task titles and their hours at 1920 (unbounded `1fr`);
- the 1180 breakpoint made every rail *child* a grid cell, so "The day" sat alone in one column
  with its timeline in the next and "Closed this week" split mid-list across three;
- `<span class="k ok">` collided with the approvals' `.ok { display: none }`, deleting every
  ok-status run label from the grid and shifting the remaining cells — which read as a text
  wrapping bug, not a missing element.

Playwright is **not** a project dependency and must not become one; the script documents the
throwaway-venv invocation.

### 11.13 Scope — what S2 does NOT include

Dedup/taxonomy (S3, runs in parallel), completion detection from sources (S4), grades and GPA
(S5), sent-mail completion (S6), new sources (S7). The surface will *display* each the moment it
exists; it does not implement them.

### 11.14 OPEN — decide during the S2 spec

- `scripts/setup-laptop.ps1` — one-time clone + venv + desktop shortcut. Quinn works on desktop
  **and** laptop; sync is git, so "synced" means last-pushed and the topline must show the stamp.
- Window size/position persistence (pywebview has no built-in; a small config file).
- Push debounce after a write (proposed ~30s) and pull-on-open behaviour.
- What the inline "New task" row contains at minimum (title, course, due, effort, importance?).
- Whether `profile/` joins the journaled note set (`engine.write` currently covers
  `tasks/ approvals/ archive/ courses/ issues/ info/`).

### 11.15 Controller ruling carried forward

CS 100 Projects 2–5 carry no `source_uid` and **will duplicate when zyBooks posts Project 2 on
Mon 2026-09-07**. Since S2 starts now, hand-stamp the four notes rather than racing S3 to beat the
date (stated 2026-08-31, not vetoed). See §10 S3.12.

---

## 12. Rulings 2026-08-31 (later session) — sections, soft ranking, and the AI/script boundary

Four rulings from Quinn, in his words where quoted. **Most recent wins** (standing rule §, 08-29),
so where these touch §11 or §5 they supersede.

### 12.1 All sections always render

> "Headings should remain. The dashboard has changed now, so I think it's fitting to show all the
> sections to keep things clear."

This **reverses** the 2026-08-26 mockup rule that a section with nothing to say emits no heading,
and **ratifies** S2 §9.6. Every region renders in every state, with a designed empty state naming
what is absent — `Nothing is at risk today.`, `Queue clear`, `No spare capacity today.`,
`No runs recorded in 24h.` The reasoning is the standing one: a console whose sections disappear
makes absence ambiguous, and silence is never ambiguous (`VISION.md`).

### 12.2 Ranking is soft

> "The ranking should be really soft. For the most part, due dates are going to make sure that
> what needs to get done gets done when it needs to. The ranking really only matters when you have
> a task spread across multiple days, recurring tasks, or when there are no more 'must dos'
> (i.e. the system must recommend work)."

The deadline does the work. Ranking is not a global ordering to be trusted and tuned; it is a
tie-breaker plus a scheduler for three specific cases.

**Ranking earns its keep in exactly three places:**

| # | Case | Why ranking is needed |
|---|---|---|
| R1 | **Work spread across multiple days** | The deadline says *when it is due*, not *how much to do today*. `start_by`, `slice_hours` and `designate_today` answer that. |
| R2 | **Recurring / cadence work** | German practice, weekly commitments — no hard due date, so something must decide when it surfaces and how urgent it becomes as the period closes. |
| R3 | **Recommending work when must-do is empty** | Spare capacity is the one moment the system must *choose*, with no deadline forcing the answer. |

**Derived rules — for confirmation before they land in the engine:**

- **Must-do-first rendering is unchanged.** Slack ≤ 0 still renders unconditionally; duration never
  decides visibility (CLAUDE.md invariant, "Quinn's core requirement").
- **Within a group, order by deadline pressure, not by a blended score.** The default sort inside
  Must do and inside each horizon group becomes `start_by` then `due` — the order Quinn already
  reads in `views/today.base` and in `## Everything active`.
- **Importance stops being a general re-orderer.** It becomes (a) a tie-breaker at equal deadline
  pressure and (b) a real input in R1–R3, where no deadline is deciding. This narrows
  `pressure()`'s remit rather than deleting it.
- **The consequence for the learning loop** (`docs/surface/inventory.md` G4/G12): a soft ranking is
  worth much less correction machinery. `rank_override` stays editable, but the `preferences.md`
  rank-correction loop and the daily pairwise calibration question drop from "stated success
  criterion, worth building" to "probably unnecessary". **This directly answers the recommendation
  made earlier the same day and lowers both from the top of the gap list.** VISION.md criterion 2
  ("correctable in under 30 seconds") is met by editing the fields that drive the deadline, not by
  teaching a ranker.
- **Open:** whether `importance` should influence `start_by` (it currently does not), and whether
  R2's cadence urgency needs the never-built progress-against-cadence signal (inventory G14).

### 12.3 The system is optimised for information collection

> "As we build this out, I want this system optimized for information collection. I want actions to
> be back-traceable… so that I can use AI to gain valuable insights on where we can improve the
> product and its workflow. That way, as I improve this and start giving it to other people, I have
> a plethora of data I can use to see HOW people are using this product, what's going right, what's
> going wrong, what people want, and what needs to change."

This promotes traceability from an implementation detail of S1 to a **product goal**, and it changes
what the system is for: quinn-ops is now also an instrument for studying how a personal ops system
is actually used.

**What already exists (S1).** Every vault write is journalled with object, field, old → new,
timestamp, device, actor and run id; every agent-judged field carries a `judgment` block with run,
actor, inputs and fields; every run writes start/step/end records with counts and full warning text;
approvals carry verdicts, notes and `first_proposed_at`; issues carry categories, free text and a
property snapshot. The foundation is unusually good — the gap is not recording, it is *reach*.

**What is missing:**

| Gap | Why it matters |
|---|---|
| **Interaction events** | The journal records writes, not attention. It cannot tell that a card sat unread for four days, that a recommendation was scrolled past every morning, that an edit was started and abandoned, or which view is actually worked in. |
| **Non-events** | A lapsed proposal records a lapse; it does not record that it was seen and ignored eleven times first. **Rejection-by-inaction is the most common outcome and the least recorded.** |
| **Derivation traceability** | `importance_reason` explains a judgment. Nothing explains a *computed* value — why this task ranks here, why `start_by` is that date, which free block absorbed which take. The inputs exist; the explanation is discarded. |
| **An analysis path** | Nothing reads any of it back. No query surface, no export, no aggregation — so the data accumulates unused. |

**Direction (to be specced, not assumed): a new slot, S8 — Observability**, after S2, because it
spans the surface, the engine, the runners and the routine. S2 contributes the origin point: the
console emits interaction events through the same ledger seam the journal already uses, so there is
one append-only record store, not two.

**Design constraints that follow from "giving it to other people":**

- **Local-first, per-install.** The store stays in the user's own vault. Nothing is transmitted by
  default; the product goal is served by Quinn's own data now and by *voluntarily shared* exports
  later.
- **Sharing is explicit and inspectable.** If usage data leaves a machine it is an export the user
  can read first, in the same plain formats as everything else, behind an opt-in. This is a schema
  decision, not a policy footnote — a log designed as if it will be shipped is shaped differently
  from one designed to stay local, and building a clean one now is far cheaper than sanitising a
  dirty one later.
- **No free text in interaction events.** They reference object ids; they never copy titles, note
  bodies or email subjects. That keeps an export shareable without redaction, and it is the single
  decision that most determines whether this data is ever usable beyond one machine.
- **Interaction events are not vault writes.** They never touch frontmatter, never affect ranking,
  and are never read back by the engine — otherwise determinism is gone.

### 12.4 AI has a defined, minimal role

> "If you can write something down as an executable script rather than having AI take on the full
> task, let me know how and we can figure it out. The goal is for AI to have a clearly defined and
> optimized role in this system."

This extends the existing invariant — *the engine is deterministic; judgment happens in Claude* —
with a second, sharper one:

> **Claude does judgment only. Everything the AI does that is mechanism — string formatting,
> arithmetic, lookups, file appends, schema construction — moves into the engine.**

Three reasons this is correctness work and not economy: prose heuristics in a prompt **cannot be
tested**; logic duplicated between prompt and engine **drifts** — the 2026-08-31 security-alert line
is exactly that failure, where the prompt instructed the opposite of the agreed taxonomy for days
and produced four wrong approvals; and **every judgment the AI does not have to make is one that
cannot be made wrongly.**

**Audit done the same day:**
`docs/superpowers/reports/2026-08-31-ai-script-boundary-audit.md`. Headline findings, all verified
against the live vault: dropped Gmail messages are **never ledgered** (prompt line 21 says "for
tiers a-d"), so one run judged **56 messages to produce zero artifacts** and re-judges them every
run — which is also the real cause of the "two runs an hour apart judged disjoint message sets"
symptom in CLAUDE.md; the approval boilerplate has **already drifted in production** (13 notes with
3 buttons, 2 with 2, from three independent definitions of one schema); and `conflicts_with` is in
**zero** notes vault-wide, so the ⚠ conflict line in Must do has never rendered — the model wrote
the finding into `importance_reason`, where the renderer cannot see it. Estimated effect of the
top eight conversions: **~60–80 model judgments per run → ~8–12.**

The rule for all new work from here:

- **If a step can be written as a function with a test, it is a function.**
- **If a heuristic can be written as a config table, it is a config table** — and editing that file
  is how it gets corrected, not a prompt rewrite. (The principle is already established for
  `profile/interests.md` in the events spec: "the fix is editing the file, not a code change.")
- **The prompt states judgment criteria and calls tools. It does not compute, format, or count.**

### 12.5 Rulings 2026-08-31 (question round) — settled

Answers to the eight blocking questions and the gap list. Items still under discussion are marked
**OPEN** and are being brainstormed before they land.

| # | Question | Ruling |
|---|---|---|
| 1 | "Everything on one page" | **Multiple pages are fine, one window.** Quinn: *"it's okay if there's different tabs for other information; we'll likely have to build that capability out as the app expands anyway."* The page set itself is **OPEN** — Quinn asked to be consulted on it. |
| 2 | Delta window | **Since the last run**, not "since I last looked". Simpler, and the same line means the same thing to any observer. |
| 3 | Launch at login | **No.** A taskbar-pinnable app he launches himself. No startup entry, no Task Scheduler. |
| 4 | PH 106 alert | **OPEN** — Quinn asked to dissect the root cause first rather than pick a placement. |
| 5 | Runway threshold strip | **OPEN** — needed a clearer description before a decision. |
| 6 | Where an approved standing invitation is written | **Decided by the controller on product grounds (Quinn: "you decide and plan for it"): a new `commitments/` note type**, not `config/planning.yaml`. See below. |
| 7 | Bind confirmation for the first month | **Not needed.** Bind automatically; one-click unbind is the safety net. |
| 8 | `title` in the freeze set | **Frozen — and keep the previous names.** Quinn: *"maybe keep the last X names in the metadata for binding."* New field `title_history` (see below). |
| G2 | Heads-up items | **Merged into the info space.** Quinn's split: *who is owed an email* → a recurring task; *registration windows* → a task; *grade changes* and *late/missing flags* → **info items**. No separate heads-up region. |
| G3 | Pull-ahead | **Build it.** Quinn: *"We want to incentivize working ahead."* |
| G5 | `actual_hours` | **OPEN** — Quinn asked what it was for and whether the value can be gleaned, inferred, or asked for. |
| G6 | Notifications and the daily brief | **Neither. Dropped.** No toasts, no 8am email. The console is the only surface. |
| G8 | Week view | **Build it.** |
| G9 | `profile/preferences.md` | **Keep it, AI-managed.** It is live: the cloud routine's enrichment step is instructed to honour it, and it carries scheduling policy that exists nowhere else (the ECDC soft-avoid ruling, German near-daily slicing, "weekend capacity is for pulling ahead, not rescue"). **Consequence: `profile/` joins the journalled note set**, reversing S2 §15's earlier call — if an agent writes it, the write must be journalled. No UI. |
| G10 | Grades on the dashboard | **OPEN** — Quinn: *"I DO plan on implementing it"*, so its dashboard shape is designed now rather than deferred to S5. |
| G11 | A reason on each change | **Build it.** |
| G13 | Say plainly when the week went wrong | **Build it.** |
| G14 | Cadence progress | **OPEN** — needed clarification. |
| Q-A | Which figure is "slack" | Controller's call, on reliability grounds. See below. |
| Q-B | Where the script conversions land | **Fold into S3**, which is already rewriting the same routine prompt. Revisit for further optimisation after it lands. |

#### 12.5.1 Standing invitations live in `commitments/`, not in config

Quinn asked for the decision to be made against the product's trajectory — *organisation and clarity
→ friends testing → selling to classmates → selling at scale*. That settles it against
`config/planning.yaml`:

- A recurring commitment created by **approving a card** is *user data*, not configuration.
  `planning.yaml` holds tuning parameters (daily budget, session cap, approval ceiling); mixing
  generated state into it means a user resetting their config silently deletes their commitments.
- Config is outside the journalled note set, so a commitment written there would have **no id, no
  provenance, no per-field sync reconciliation, and no undo** — losing every guarantee S1 built.
- At product scale, "edit this YAML file" is not a supported user action.

So: **`commitments/*.md`**, a note type like any other — `id`, `judgment`, journalled through
`engine.write`, with `name`, `hours`, `days`, `source` (the approval it came from), and a
lifecycle (`active` / `ended`). `planning.yaml`'s `recurring:` list migrates into it and the key is
retired. `engine/planning.py` reads config for parameters and `commitments/` for obligations.

#### 12.5.2 `title_history` — Quinn's amendment to the freeze

`title` stays frozen once Quinn edits it (S3.3), **but the note keeps the names it used to have**:

```yaml
title: "Reflexion 1 (rewrite)"
title_history: ["GN 103 Reflexion 1", "Erste Reflexion"]
```

Appended (capped at 5, most recent first, deduped) whenever a title is overridden by Quinn or a
source proposes a different one. **The binder scores against `title` *and* every entry in
`title_history`, taking the best match.** This directly repairs the trade the freeze introduced —
a renamed note stayed frozen but became harder to bind. Now renaming costs nothing.

#### 12.5.3 The gauge metric (Q-A)

Chosen for reliability, not expressiveness. Literal aggregate slack — summing `slack_days` across
dated active tasks — is **rejected**: it is dominated by far-future work and falls whenever a task
is merely *added*, so it would report losing ground for good reasons.

The gauge plots **`days_to_recover` = deficit hours ÷ daily effort budget**, where deficit hours =
must-do remaining hours minus today's capacity, floored at 0. Zero means even. It moves only when
something real happens: work completed lowers it, a missed day raises it, adding far-future work
does not touch it. `runway_days` and `must_count` are recorded alongside as secondary series.

Direction word: **gaining / holding / losing**. VISION's "slack trending flat or up" maps to
`days_to_recover` trending flat or **down**, so the axis is labelled explicitly rather than left to
be misread.

### 12.6 Rulings 2026-08-31 (final round) — every open question closed

| # | Item | Ruling |
|---|---|---|
| 5 | Runway threshold strip | **Not a second instrument.** The capacity meter's spill blocks become **hoverable/clickable and name their tasks**, which delivers Runway's information (*which* work doesn't fit, and how big it is) inside the meter already chosen. |
| 4 | PH 106 / "hours hiding behind few tasks" | **Route C + Route B. Route A (an alert) is dropped** — it treated a symptom and could not be acted on. |
| G5 | `actual_hours` | **Infer *and* ask.** Both signals, see 12.6.2. |
| G10 | Grades | Design approved as proposed — see 12.6.3. |
| G14 | Cadence progress | **Build it.** Quinn: *"if I didn't check off a day earlier that week, it will make sure I still complete it later on."* |
| 1 | Pages | **All eight approved**, plus: a **search/filter box on Work**; **Grades ships from day one**, not deferred until it has data; **"Health"** confirmed as the name (it covers sync and ingest sources, not just run records). |

#### 12.6.1 The PH 106 root cause, and the two-part fix

The symptom was PH 106 holding **35.0h behind only 3 tasks** — fourth-heaviest course by hours,
lightest-looking by count. The dissection found the display was not the disease:

1. **Sources emit incompatible granularity.** zyBooks produces 24 fine-grained CS 100 items; a
   syllabus produces one "PH 106 Exam 1 prep — 12h". Any count-based figure mixes those units and
   therefore means nothing — "Overdue 11" does not distinguish 3 hours from 30.
2. **A 12-hour task is a project, not a task.** It cannot be meaningfully half-done, `start_by`
   treats it as one blob, and progress 0→100 has no useful middle.

**Route C — every count carries its hours.** `Overdue 11` becomes `Overdue 11 · 8.4h`, in the nav,
every group header and every course figure. Nearly free, and it makes the distortion structurally
impossible: nothing can hide behind a count when counts always carry hours.

**Route B — propose a split.** When a single task exceeds a threshold (~6h), the routine proposes
decomposing it into real sub-tasks ("read ch 4", "problem set", "practice exam"). Quinn: *"I love
the idea of a little recommendation thing that will automatically split the task into multiple
parts."* This is judgment, so it is a **proposal, not an automatic rewrite**, and it rides the
existing approval taxonomy. Route C makes the alert unnecessary; Route B removes the lumpiness that
caused it.

#### 12.6.2 `actual_hours` — infer and ask

Sources cannot supply it: zyBooks reports earned points, VHL reports group status, Blackboard
reports submission state. All report *done*; none report *duration*, and none carry timestamps to
subtract. So two signals, together:

- **Infer** — when a task was allocated to a free block by `designate_today` and its progress
  completed during that block, take the block's hours as the estimate. This is the only signal
  available for tasks an *agent* ticks, and it uses data the engine already computes. Recorded with
  low confidence.
- **Ask** — when progress reaches 100 in the console, the row offers an inline **`took: [2.5h]`**
  pre-filled with the original estimate. Right estimate: do nothing. Wrong: change one number.
  Dismissible. This is the design doc's original "recorded on check-off" at near-zero friction, and
  it is the high-confidence signal.

Recalibration is a per-course × per-kind ratio of actual ÷ estimated, applied as a prior to new
estimates of that shape. **Note the asymmetry with §12.2:** the *ranking* learning loop was closed
as unnecessary under soft ranking, but the *estimation* loop is retained deliberately — bad
estimates are what make capacity lie, and capacity is what the meter, the gauge and `start_by` all
rest on.

#### 12.6.3 Grades on the dashboard

The organising insight: **a grade is only actionable through what it implies about remaining
work.** The central figure is therefore not "you have an 87" but *"you need X on what is left to
hold 3.75."* Four placements, each earning its spot:

- **Grades page** — per course: earned / possible so far, what is still ungraded, current standing,
  the needed-on-remainder figure, and late/missing flags.
- **Today** — only the **projected term GPA against 3.75**, and only when at or below it. Above the
  floor it is noise at 8am.
- **Info** — a grade *posting* opens an info item ("PH 106 Exam 1 posted: 82%"), closed by reading
  it. This is the G2 merge in practice.
- **Ranking** — remaining work in a course below target gets an importance bump. This is the
  "feeding importance/urgency" §3 #12 asked for, and the reason grades are worth capturing at all.

Term GPA, not cumulative, because the AEMX floor is per-semester (§8, answered 2026-08-29).
Cumulative stays secondary, for scholarship renewal.

#### 12.6.4 The page set

| Page | The question it answers |
|---|---|
| **Today** | What do I do right now? (the console as designed) |
| **Work** | Everything — horizon filters (overdue / this week / later / undated / all) plus a plain **text filter box**, not a command palette (§11.4 stands) |
| **Week** | Seven days: capacity per day, hours due per day, commitments — and the home for **pull-ahead** ("Thursday has 3.5h spare; here is what to pull") |
| **Decisions** | The full approval queue: every kind and urgency, snoozed items, budget spent and remaining |
| **Info** | Things to know, open/close — now also grade postings and late/missing flags |
| **Grades** | Projected term GPA vs 3.75, per-course standing, needed-on-remainder |
| **Issues** | Flagged judgments cleared in one sitting; duplicate resolution lives here |
| **Health** | Runs, sync, ingest and source status, with full untruncated warnings |

Today keeps the rails (THE DAY, decisions deck, AHEAD, closed, runs) as summaries; each rail's full
version is its page. The mockup's `Overdue` / `This week` / `Later` nav entries become **filters
inside Work**, not separate destinations.
