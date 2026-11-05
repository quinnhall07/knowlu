# Onboarding replan draft, review r1 (2026-11-04)

**FAIL at 2d8ad52:** 3 Critical, 7 Important, 6 Minor. Reviewed against plan §3's live results, `CLAUDE.md`, `VISION.md` and the cited code.

## Critical

1. **C5 needs the sign-in result, and §4.7 has a cycle.** Plan lines 1575-1576 make F8b depend on S4d and F8b2 on F8b. The draft replaces S4d with A2/B2, which wait on R0a, and never re-points F8b. C5 writes through F8b2's fenced writer (§6.11), so "C1 to C8 depend on no sign-in result" is false for C5 and in practice C6. §4.7 puts A/B after the F11 screens, yet F11e2 needs F8g, which follows F8b, which needs A2/B2. Fix: move A2/B2 ahead of F8b, re-point F8b, add F8b2 and F8b to C5; C6 also needs C5.
2. **A class-search row can be written with no press on that row** (§6.7, RQ2, C6). `console.js:1168` presets class rows to Mine on the first-day screen and `:1228-1233` writes them on Finish. §5 keeps that screen and no task touches `console.js`. A student who confirms step 10 with two shells blank and no planning-day note opens Knowlu the same day: `setup` is true (`commitments.rs:5234`), both shells show Mine, and one Finish writes them as hard commitments. `Proposal` carries only `source_uid`, so a source field is needed.
3. **The approved privacy text is false** (RQ6, §3). Only the bundle is deleted. Every `found` row goes into `state/calendar-series.json` (§6.5) and ages by `until` (§6.6). `on_sign_out_with` (`onboarding_state.rs:933`) deletes the answers file and pending links; neither it nor `sign_out_with` deletes the vault. A section marked Not mine stays on disk after sign-out. Needs Quinn: reword, or add a purge of unconfirmed rows. Blocks S7.

## Important

4. **The shared key lets a confirmed note hide real classes** (§6.6). The hold is built from `by_uid` (`commitments.rs:2556`) but tested through `by_calendar` in `proposals` (`:2718`), so class-search rows are held, unlike registrar rows. Mine on lecture 001 makes recitation 101 vanish with no card or ask. `card_reason` uses `by_uid` (`:2590`), so a filed card stands while `proposals` hides the row. The note also holds every Google `class` of that course. C2 tests none of this.
5. **`registrar:` no longer means the registrar said so** (§6.6, C2). C2's walk misses `console.js:981`, `:1041-1043`, `:1122-1125`, `:1171`, which label rows with the registrar's name and break VISION commitment 5, and `commitments.rs:2556`, `:2590`, `:3994`, `:4520`, `:4609`, `:4870`. An unconfirmed row survives a registrar read that omits it (`:2381` applies only to `registrar:` calendars).
6. **RQ9 has no data path** (§6.5, C2). `no_time` rows make no series and the bundle is deleted. `uncovered_courses` (`commitments.rs:890`) has no series-file input, so the ask fires anyway; step 10 also says "Knowlu will ask this week" (`onboarding.js:633`). Name the stored record and format.
7. **Step 10 can miss the rows** (§6.11, C6). `askProposals` latches once on `config.ingest` (`onboarding.js:813`, `onboarding_state.rs:895`); the reload at `:905` re-reads only `loadSet10`. `class_times.apply` is not ordered against `config.ingest`. A naive re-paint would wipe pressed answers. "Your calendar repeats these" is wrong for these rows.
8. **A2 omits S4c** (§4.3). A2 needs it more than B2 does, since the mailbox must share the folder.
9. **The S tasks are not common yet** (§4.2, RQ4). RQ4 is unruled, holds S1 to S4c, and is conditional on R0a item (d). Rule it after (d), or state that S1 to S4c wait.
10. **Task sizing and file collisions** (C5, §4.7). 82 references across 7 files, 42 in `app/tests/onboarding_jobs.rs`; C5 also reaches `onboarding_state.rs:534-546`, `:592`, `:1067` and `onboarding.js:49`, `:207`, `:881`, `:904`. Split it. C4 and C8 both edit `Curated`, as do S2 and A1/B1; C8 and S4b share `lms_link.rs`.

## Minor

11. The link is in a plain cookie-less fetch at the founder's school: close §7.1 there, delete the whole-link fallback, narrow R0a (e) to "does `/` redirect first?", drop C8's wait in §4.2 and §8, update §12 item 3. C8 must name the URL and redirect rule; §6.10's curation check needs a link check for other schools.
12. §7.4 row 2 and C8: the link is on the LMS host, so `reached` goes true on first poll (`grades.rs:645`). `SignedOut` then rests on a 401; one transport failure gives `Unreachable` with no mark. `read_hidden` takes a real `AppHandle`, so C8 needs a seam.
13. §6.7 "the cap is untouched" holds only at step 10: blank rows become `commitment-check` cards from day 2 (`commitments.rs:3994`) and charge the cap.
14. §4.7 step 1 conflicts with the C1 row, which depends on R0b for the keys.
15. Stale text after the rulings: the "not ruled" header, §3's open point, RQ10's "needs your signature", §12 item 1.
16. `school_sign_in` comes from one account's R0a. A student who declines "stay signed in" at an `AtLms` school has no step-5 fallback.

## Held up

`journal::VIAS`, actors and `LOCAL_CARD_KINDS` are unchanged. No path takes section times to the cloud. No rule-1 breach. No contract-list breach. §1 items 1 and 2 hold.
