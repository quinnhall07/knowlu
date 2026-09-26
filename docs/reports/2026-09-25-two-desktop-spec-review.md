# Review: the two-desktop design spec (draft `c34fc1f`)

Reviewed 2026-09-25 against `docs/specs/2026-09-25-two-desktop-design.md` (647 lines) at branch
`two-desktop-spec` (`c34fc1f` on main `510a88c`), its brief and drafting report (`.superpowers/spec/`),
CLAUDE.md, the cloud design and its Amendment 2026-09-17, the C5 spec, the C3′ whole-branch review,
and the code. Read-only; no tests run. The four reference ids of §2.1 were recomputed with Python's
`hashlib` and all four match; the collision figure (10,000 notes, 40 bits: 4.5 × 10⁻⁵) is right.

**Verdict: READY AFTER FIXES.** The shape is sound and faithful to Quinn's decision: D1's
derivation, the single-statement lease, clock authority on the server, fail-open onto the id
backstop, the named skip lines, and *Sync now* staying transport only. One Critical finding breaks
the stream's own promise ("sync merges it", no silent divergence) for notes already doubled or
doubled during the rollout. Six Important findings are design gaps a plan writer would otherwise
have to invent, one of which the C3′ review had already named. None needs the decision reopened.

Counts: **1 Critical, 6 Important, 16 Minor.**

## Critical

**C1. D5 (a) and D6 contradict each other, and either reading leaves a silent divergence.**
(Spec lines 53–54, 148–150, 176–198; D5, D6.)
- D5 (a): step 4 finds the local note by id, "falling back to the record's path only for an id this
  vault does not hold". D6: the loser's pushed records "carry an id it has never held, so they stay
  inert". Under D5 (a) they are not inert. They are reconciled by path against a note with a
  different id, with `mine` built from the foreign id. That is I2's own mechanism
  (`sync.rs:1907-1912` takes the path from the foreign record; `:1969-1981` builds `mine` from
  `journal.records_for(id)` with the foreign id; `reconcile.rs:122-140` falls to the mtime stand-in).
- If the fallback is removed, so that D6's "inert" is true, a field set only on the losing copy
  before convergence never reaches the winner. The link record is under `sync::ACTOR` and never sent,
  and `reconcile::resolve` builds chains only from the foreign side's `set` records
  (`reconcile.rs:96-104`). With no foreign chain for that field, no card is filed either. D6's "at
  most one sync card, never silently" is then false for exactly that case. Test (v) (line 518) would
  not catch it unless it compares the two desktops' final values.
- **Ordering.** Convergence is detected "on every foreign `create` in step 4 and on every pulled
  text in step 6" (line 166). Step 6 runs after step 4 (`sync.rs:1904`, `:2161`), so D6's promise
  that "the step-4 reconcile that follows sees this desktop's own edits" does not hold for a double
  detected by its text.
- **Existing doubles.** Their `create` records were pulled before the upgrade, and the cursor is
  past them. They are therefore never detected unless the note's text changes again. Yet §9 Q9
  recommends converging them, and there is no one-time pass that would.
- **Fix.**
  - Build an **alias map as a pre-pass**, before step 4, beside the N2 seed pre-pass. Build it from
    every foreign `create` record in the journal and in the page: a `create` record's `new` is the
    whole frontmatter (`write.rs:416-418`), so `import_key` is computable from it. The map is
    foreign id → local id wherever the import keys match under different ids.
  - Step 4 groups the foreign records under the alias. `records_for` follows the alias on **both**
    desktops, so the winner also applies the loser's one-sided edits cleanly.
  - Allow the path fallback only for a local note with no `id:`.
  - The lower-id re-id then only tidies the id line.
  - Add to (v): a field set only on the losing copy ends equal on both desktops.

## Important

**I1. Per-desktop config decides what the turn holder fetches. The review named this in I2; the
spec drops it.** (§0, D4, D11, §5.4.)
- `config/` never syncs (`sync.rs:366-368`), and each wizard writes its own `course_map`, zyBooks
  and VHL mappings (`app/src/onboarding.rs:609-629`).
- Under a moving turn, the holder's config decides:
  - which books are fetched at all: an unmapped book is `zybook {code} not in config; skipped`
    (`coursework.rs:664-675`);
  - every title's prefix (`zybooks.rs:296`, `{course_label} {title}`). The update branch rewrites
    `title` whenever it differs (`coursework.rs:424-429`), so two different labels flip-flop the
    title each time the turn moves.
- `apply_map_cards` writes the mapping into the config of whichever desktop runs `coursework`
  (`coursework.rs:1156-1170`). Under D11 that is the portal holder only. The other desktop:
  - never gets the mapping;
  - skips that book on every slot it wins;
  - files a second card for it once the approved card's 30 days lapse (`asked_map_keys`,
    `coursework.rs:1131-1154`).
- The review's I2 said both halves (`docs/reports/2026-09-25-c3-account-vault-whole-branch-review.md:135-138`:
  the course map, and a restored desktop's missing `- name: personal`). §5.4 rules on neither.
- **Fix.**
  - Add a D: every desktop applies approved `coursework-map` cards, in `approvals/` or `archive/`,
    to its own config at each slot, whether or not it holds the job. This is an idempotent text
    insertion that `apply_map_cards` already makes.
  - Rule the personal-calendar bullet in or out.
  - Add a question for Quinn: move the course map into the account now (ruling 3 already names
    `config/` parameters as cloud data), or accept per-desktop config until C5 or later.

**I2. D8's account-level event verdict cannot be built as written.** (Spec lines 56, 226–231.)
- `/judge-event` is to answer "from that row". But:
  - `fieldsOf` drops `why` and `confidence` from every stored row
    (`cloud/supabase/functions/_shared/judge_pipeline.ts:77-78`);
  - the event verdict is invalid without a non-empty `why` (`_shared/judge_validate.ts:121-124`;
    the device reads it at `engine/src/cloudmodel.rs:449`);
  - `_shared/judge_log.ts:4-7` makes "nowhere to put a reply" a structural privacy property (cloud
    design §5.6).
- It also closes the ledger's own escape hatch: "delete a line to force a deliberate re-judge"
  (`eventledger.rs:46-47`). It changes `/judge-event` for every account, one-desktop ones included.
- **Fix.** Pick one, and say what the device does with the answer:
  - answer with verdict and confidence only, and the device accepts an empty `why` (the digest line
    shows no reason); or
  - keep the account's verdicts somewhere outside `judgments`.
  The second is a privacy question for Quinn (§9).

**I3. D5 (d) regresses a hand delete.** (Spec lines 161–164.)
- A tombstone settles the local note only when "another desktop's records place that note's id at
  P", or no record anywhere names the id.
- Take a note created on B that A never edited, then deleted by hand on A (in Explorer, so with no
  `delete` record). The only record naming the id is B's own `create`, so the tombstone is ignored
  and the note lives on B forever, silently.
- Today it is archived (`sync.rs:2236-2241`).
- **Fix.** Invert the default: settle unless another desktop's records place a **different** id at
  P and none place this id there. Test (iv) should cover the hand delete.

**I4. The `devices` retention promise is false for an idle or lapsed account.** (Spec lines 262,
286.)
- The prune runs only inside `fetch_turn`, "for the calling account (no cron job)". `/turn` is gated
  by `requireActiveEntitlement`.
- So an account whose computers stop calling it never prunes. Examples: the subscription lapsed, or
  the student stopped using Knowlu.
- The privacy sentence "A computer leaves the list 90 days after it was last seen" is then untrue.
- **Fix.** Prune from `pg_cron`, which this project already uses (`sync_prune`,
  `migrations/20260912000100_sync.sql:188-203`), or reword the sentence.

**I5. D14 does not hold in the common wake-up, and proof step 6 expects the wrong lines.**
(Spec lines 62, 405–406, 413–421, 583–585.)
- §4.4 says "a renewal the service cannot answer counts as granted". On resume, the network is
  usually still coming up, so the sleeper's first renewal fails in transport. The sleeper then goes
  on to `ingest` and `judge` while another computer holds the jobs. D1–D5 absorb the double run,
  but D14's stated behaviour never happens.
- The in-flight child is timed on the app's clock (`CHILD_TIMEOUT`, `app/src/scheduler.rs:28`,
  `:561-569`). After a long sleep, or a `pssuspend` of the tree, it is killed on resume as `-2`.
  That makes `engine_ok` false (`:821-823`): an amber tray and retry backoff, not the quiet skips
  proof step 6 expects. Whether `Instant` counts a real sleep on Windows is unverified.
- **Fix.**
  - An unanswerable renewal counts as granted only while the device's own elapsed time since its
    last grant is under the `seconds_left` that grant carried. This is a device-local duration,
    not a comparison with the server's clock. Past that point, skip with
    `(skipped: the turn passed to another computer)`.
  - State the in-flight step's expected line (possibly `-2`), and whether its retry then claims
    again.

**I6. The privacy words (§3.3) promise more than D11 and D13 deliver, and they miss the paragraph
they touch.**
- "only one of your computers at a time fetches your coursework, school calendar and email". But:
  - the lease is per job, so zyBooks on one computer and VHL on another fetch at the same time;
  - D13 lets every computer fetch while `/turn` is down;
  - every computer's `rank` still fetches event feeds and calendars (`engine/src/cli.rs:360-400`,
    the transport CLAUDE.md names).
- The *Your coursework logins* paragraph (`site/privacy.html:40`: "The password stays on the
  machine, and there is no column in our database for one") is the one a reader will check. Yet
  the account now records which computer holds which login.
- **Fix.**
  - Say the computers "take turns fetching", not "only one at a time".
  - Add one sentence to the logins paragraph.
  - Pin both sentences in `engine/tests/site.rs`.

## Minor

- **M1. Lock order in the claim** (line 318). `insert … select … from unnest(p_claim)` locks rows in
  array order. Two claimants that list the same jobs in different orders can deadlock, and Postgres
  aborts one. D13 then turns that 5xx into a double run. Today's client order is fixed, so this is
  a belt: add `order by j`.
- **M2. Restore keeps the older text** (line 160). "The first row by `rev` wins" is the opposite of
  E2's "the latest row for a path wins" (`sync.rs:1062-1067`), and it drops the other desktop's
  later edits from the restored file. Use the highest `rev`.
- **M3. A re-id orphans references.** Issues carry `target_id` (`engine/src/issues.rs:217`), and
  issues sync. D6 should list what points at an old id and say whether it follows.
- **M4. D3 changes the digest's behaviour.** Today "one per day" is only a path check
  (`eventemit.rs:263-267`). A digest settled and archived the same day frees the path, and a second
  digest can be emitted. Under D1 that second digest is `IdHeld`. That is fine, but line 111's "one
  per day by construction" is not true today: say D3 makes it so.
- **M5. One id, two contents** under D13 or a turn race. Both desktops emit today's digest with the
  same deterministic id and different payloads, and D5 (c) keeps each desktop's own. A body edit
  never travels to a desktop that already holds the file (`sync.rs:2273-2275`). Name it as a cost of
  D13. Unverified: whether the console ever ticks a digest's boxes in the body (it appears only to
  count them, `app/static/console.js:256-288`).
- **M6. A shared `COMPUTERNAME` is a C3′ limit too, not only the turn's** (§3.2). Two Windows users
  on one account share the journal's `device`, which `apply`'s `never_took_effect`
  (`sync.rs:1961-1968`) and `reconcile`'s tie-break (`reconcile.rs:55-57`) read as "this device".
  Say so.
- **M7. The proof's `KNOWLU_DEVICE` must be set before each user's wizard runs.** Otherwise
  `runners.yaml`'s `device:` names `COMPUTERNAME`, and `device_ok` refuses every slot ("not the
  designated device", `scheduler.rs:456-461`, `:618-620`). Proof step 5's "previous release" talks
  to production: it needs a dev build of that release's commit, pointed at staging.
- **M8. The widened RLS guard needs whitespace tolerance.** `20260912000100_sync.sql:205-208`
  aligns `alter table public.sync_records    enable …`, which an `includes()` match misses. Every
  table the corpus creates does enable RLS today (checked, all 28 creations).
- **M9. §4.3's table lacks two lines:**
  - a partial `coursework`, when this computer holds zyBooks and another holds VHL;
  - a vault with no coursework site set up at all. As written it would read "no coursework login
    on this computer" every slot.
- **M10. D13 does not classify a 400 from `/turn`,** for example a name left empty after its control
  characters are stripped. Say open or closed.
- **M11. `devices.sources` reuses the name of `public.sources`** (`20260910000100_accounts.sql:51`,
  the LMS and calendar feeds). Call the column `logins` or `portals`.
- **M12. §5.6 understates the overlap with the commitment model.**
  - Commitment notes carry no `created_by:` (commitment design §2's field table), so "add their
    `created_by` words to `IMPORT_VENDORS`" does not fit. Key them by the `source_uid` prefix
    instead.
  - Both programs edit `sync.rs`'s `apply` and `ids.rs` (`KINDS`, `ID_RE`, `NOTE_FOLDERS`). That is
    a shared-file merge, not "neither blocks".
- **M13. Concurrent *Sync now* and a producer (the M6 race class)** can now put one deterministic id
  in two files. `ensure_ids` (`cli.rs:314`, `ids.rs:171-188`) then re-ids the later file with a
  random id. §5.4's "not widened" should say so.
- **M14. Key namespace.** Calendar-event cards key the raw feed uid and the digest keys
  `events-digest:<date>`, both under (`appr`, `events`). Prefix every key with its producer.
- **M15. Wording.**
  - `sync-push/index.ts` is a roughly 20-line try/catch wrapper, not a four-line shape;
    `judge-event/index.ts` is the four-line one.
  - `claimed_at` is overwritten by every renewal, so it records the last renewal.
  - §2.6's "no note on disk is re-id'd" sits beside D6, which re-ids.
- **M16. Fail-open (D13) and the 90-day retention figure are Quinn-facing.** D13 costs double
  charges, and 90 days is a number printed on the privacy page. Either can stay a decision, but list
  both in §9 so the signature covers them.

## The spec's claims about today's code

Checked against the code, all correct:
- `write::create` mints only when the text carries no id (`write.rs:379-411`). `new_id` is at
  `ids.rs:68` and `derived_id` at `ids.rs:59`.
- None of the nine producers writes an `id:` line: `coursework.rs:184-200`, `ingest.rs:506-527`,
  `enrich.rs:523-526`, `:797-802`, `:1024-1028`, `eventemit.rs:316`, `approvals.rs:1048-1062`.
  §2.2's line numbers, `created_by:` words and keys are right.
- `device_token` is at `sync.rs:159-165`.
- `slot_argv` is at `scheduler.rs:398`, and `run_slot_inner` is at `:614`. The pre-flight has its
  45-minute floor at `:691`. The runner-log filter is at `:792-808`, and the `ingest` and `judge`
  skip lines already pass it.
- *Sync now* is `commands.rs:303` → `state::run_sync` (`state.rs:212-220`), with no machine step.
  The quit flush pushes only (`state.rs:294-311`).
- `PRIVACY_VERSION` is `2026-09-24` (`account.rs:25`). The `SESSION_REFRESH_LOCK` and
  `valid_access_token_at` 120-second margin claims hold (`account.rs:491-499`).
- `migrations_test.ts` pins 28 functions (`:322`) and 5 views (`:338`).
- The account purge list is at `functions/account/index.ts:58-78`.
- The judgment budget is `enrich::BATCH_BUDGET` = 15 minutes (`enrich.rs:72`), and `CHILD_TIMEOUT`
  is 20 minutes (`scheduler.rs:28`).

## The drafting report's eight findings

1. **LMS paths can differ between desktops. Confirmed.** Ingest names a note
   `<course>-<slug(title)>`, with the course from `match_course(event, course_map)`, before its two
   creates (`ingest.rs:709-781`). The course map is per-desktop config that never syncs
   (`sync.rs:366-368`). Coursework titles also carry the config label (`zybooks.rs:296`), which
   feeds I1.
2. **A delete does not reach a desktop holding the note at a different path. Confirmed.**
   - `apply`'s record pass acts only on `move` (`sync.rs:1780-1797`).
   - Step 4's `reconcile::resolve` reads `set` chains only (`reconcile.rs:96-104`).
   - The tombstone is by exact path (`sync.rs:2236-2241`).
   - The sender's archived copy then lands as a second file under `archive/` on the receiver
     (`sync.rs:2301-2317`).
3. **A pulled note whose path is taken by a different local note is never written, and nothing
   reports it. Confirmed.** `sync.rs:2273-2275` is a bare `continue`, with no warning and no count.
4. **`rank` emits the digest on every desktop from a ledger that never syncs. Confirmed, with a
   nuance.**
   - The digest is emitted at `cli.rs:391`.
   - `pending_digest_uids` reads only unsettled digests in `approvals/` (`eventemit.rs:75-115`), so
     a desktop without the proposer's ledger re-proposes an event once its digest has settled.
   - The nuance: "two digests on one day" arise only when both desktops emit before either pulls.
     The path check (`eventemit.rs:263-267`) stops a later one.
   - The hazard already exists under C3′; the turn makes it routine.
5. **The event cap is 80 a day per account. Confirmed.** `_shared/judge_caps.ts:39` has
   `event: 80`; the cloud design says 300 (`2026-09-09-knowlu-cloud-design.md:154`).
6. **`migrations_test.ts`'s RLS test covers only C2's tables. Confirmed.**
   - `ours()` reads `20260911*` only (`migrations_test.ts:8-18`), and the test matches only
     `create table if not exists (\w+)` (`:272-282`), while C1 and C3′ write `create table public.x`.
   - Every table the corpus creates does enable RLS today (checked by script). The gap is the guard,
     not the tables.
7. **Two Windows users on one PC share a device token. Confirmed.**
   `device_token = sha256(account_id + "\n" + device_name())[..16]` (`sync.rs:159-165`), and
   `device_name()` reads `KNOWLU_DEVICE`, then `COMPUTERNAME` (`journal.rs:71-80`). See M6 and M7 for
   what else follows.
8. **The commitment model keeps the lowest id among cross-desktop duplicates. Confirmed.**
   `docs/specs/2026-09-23-commitment-model-design.md:208-213` (§2.5) and its M13 row (`:1118`). But
   see M12: its notes carry no `created_by`.

## The open questions (§9)

Each one is Quinn's, and each recommendation is sound, with these qualifications:
- **Q1 (per job), Q2 (catch-up), Q3 (no head start), Q5 (advisory now, enforced at `/relay`),
  Q8 (a Settings list after the pilot).** Agree.
- **Q4 (archive quietly).** Agree, but it rests on D5 (b). I3's fix does not change it.
- **Q6 (no re-ask screen).** Agree. Add that the 90-day figure is a retention promise on the page,
  and the page must be true (I4).
- **Q7 (two Windows users as the gate).** Agree, and add to the procedure:
  - creating a second Windows user on Quinn's machine is Quinn's to approve;
  - `KNOWLU_DEVICE` must be set before each wizard runs (M7);
  - `pssuspend` trips `CHILD_TIMEOUT` on resume (I5);
  - "the previous release" means a dev build of that commit pointed at staging (M7).
- **Q9 (converge existing doubles).** Agree, but no mechanism exists until C1's alias pre-pass or a
  one-time journal scan is specified.

**Missing questions:**
- **Q10:** per-desktop config under a moving turn. Sync the course map through the account, or
  apply approved map cards on every desktop and accept the rest (I1).
- **Q11:** D8's account-level event verdict. Answer with no `why`, or store verdicts outside
  `judgments`, which touches §5.6's structural privacy (I2).
- **D13 (fail open) and the 90-day retention** are fine as decisions, but they belong in §9 so that
  the signature covers the double-charge cost and the printed number (M16).

Nothing in the spec relitigates Quinn's decision. The fidelity table (§8) is accurate. Its two
"refined" rows are the per-job lease and the added `kind`, and each is flagged honestly. D6's re-id
of on-disk notes is the one place that bends the brief's "ids are a contract", and Q9 surfaces it.

## Length (647 lines against a 300–450 aim)

These cuts lose nothing load-bearing, about 75 lines in all:
- **§1's Reason and Cost columns are restated in the body.** Cut the restatements (about 35 lines):
  - §2.4's second half (lines 137–142, D4);
  - §4.4's "Why open" paragraph (408–411, D13);
  - §3.2 (270–273, D9's cost);
  - §4.7's first three bullets (437–443, D16);
  - most of §5.3 (465–468).
- **§2.5's "The two cases the brief asks about"** (168–172) restates rules (a) and (c) (5 lines).
- **§0's "What the code does today"** (21–43): keep the evidence, but drop the apply bullet, which
  §2.5 repeats (about 8 lines).
- **§6.4's argument for two Windows users** (555–565) duplicates Q7. Keep one of the two (about 8
  lines).
- **§5.5's third bullet and §5.6** can shrink to three lines each (about 10 lines).
- **§6.1 and §6.2** can drop the tests the decisions already name one to one (about 8 lines).

That lands near 570 lines before the fixes, which add about 40. The rest is earned: 16 decisions,
two SQL sketches, the line table, the proof and the fidelity table are what a plan writer needs. The
450 aim is not reachable without dropping one of them.

## Re-review of fix round 1 (9caf4b1)

Scope: `git diff c34fc1f..9caf4b1` of the spec, now 698 lines, against the rulings in
`.superpowers/spec/fix-1.md`. I read the whole new spec where the diff needed context. The four new
reference ids of §2.1 were recomputed with Python's `hashlib`, and all four match.

**Verdict: findings open: 3 new Important, 0 Critical.** Every original Critical and Important
finding is addressed in substance. Two of the fixes carry a new defect, and I5's fix rests on an
unverified clock.

### The original findings

| Finding | Verdict | Where |
|---|---|---|
| C1: D5 (a) against D6 | **ADDRESSED** | D5, D6 (lines 50–51); the path fallback removed (137–141); the alias pre-pass (169–190); the full reconcile of a newly found group, which gives Q9 its mechanism (183–185); test (v) asserts equal values (536–538). See N1 for a new hole. |
| I1: per-desktop config | **ADDRESSED** | D17 (line 62), §4.8 (442–471), Q10 (686–691), proof step 9 (623–624). See N2. |
| I2: D8's verdict | **ADDRESSED** | D8 (53), §2.8's verdict pull (211–225), Q11 (692–695). No free text is stored on the server, and the delete-a-line re-judge is kept (222–224). |
| I3: hand delete | **ADDRESSED** | D5 (d) (153–158), test (iv) (534–535). See m1. |
| I4: 90-day retention | **ADDRESSED** | A daily plain-SQL `pg_cron` job, with no function and no Vault token (241–243, 256–257). The `knowlu-sync-prune` precedent exists (`20260912000100_sync.sql:227`). |
| I5: wake | **ADDRESSED in shape** | D14 (59), §4.5 (412–424), proof step 6 (614–618). See N3. |
| I6: privacy words | **ADDRESSED** | §3.3 (274–289). Every clause now holds under per-job turns and fail-open, and beside `rank`'s own feed reads. The logins sentence is added. |

### New findings

**N1 (Important). An alias group with two local files turns the re-id into a duplicate id.**
(Lines 186–188.)
- Pre-upgrade doubles at different paths are real: they happen whenever the two course maps differ,
  which was drafting finding 1. Today `apply` writes the other desktop's text wherever its path is
  free (`sync.rs:2301-2317`). So each desktop holds **both** files, `a` at `P_a` and `b` at `P_b`.
- The pre-pass then re-ids the loser file to the winner. That leaves two files carrying one id.
  `build_index` sees only the first (`ids.rs:123-135`), and the next `rank`'s `ensure_ids`
  (`cli.rs:314` → `ids.rs:171-188`) re-ids the later path **at random**. The item splits again,
  now under an id no desktop can join.
- **Fix.** When a group has a local note under the winner, reconcile the loser file's fields into it
  (the full reconcile already does this) and settle the loser file with `write::delete` under
  `sync::ACTOR`. Re-id only when this desktop holds no winner file. Add a test in (v)'s family for
  two files on each side.

**N2 (Important). D17's `apply_approved_mappings` reads "approved" cards, but an applied card is
archived as `executed`.** (Lines 457–461.)
- `apply_map_cards` stamps `status: executed` before `write::delete` (`coursework.rs:1215-1222`,
  the `executed`/`executed_at` pair and then the delete).
- So the card the fix exists for, applied on the holder and then archived, is never `approved` in
  `archive/`. A plan that tests `status == approved` would never map the book on the other desktop.
- **Fix.** "`approved` in `approvals/`, or `approved` or `executed` in `archive/`". Excluding
  `rejected`, `expired` and `refused`, as now, is right. Proof step 9 would catch this, but only at
  the gate.

**N3 (Important). I5's renewal rule depends on whether `Instant` counts sleep, which §4.5 itself
calls unverified.** (Lines 59, 412–416.)
- On Windows, `std::time::Instant` is `QueryPerformanceCounter`. If it does not advance through a
  real sleep, a two-hour lid close reads as seconds of elapsed time. The unanswered renewal then
  "counts as granted", which is exactly the I5 case the rule exists for.
- `pssuspend` would not show this, because the machine keeps running.
- **Fix.** Measure the elapsed time on a local clock that includes sleep: `GetTickCount64`, whose
  documentation says it includes sleep and hibernation, or the larger of the `Instant` and
  `SystemTime` deltas. It is still the device's own duration against the server's `seconds_left`,
  with no clocks compared, which keeps the ruling. The §6.2 test should inject the clock.

**New Minor findings**
- **m1. D5 (d) should compare alias groups, not raw ids** (lines 153–158).
  - Take an I2 double at one path, `a` on A and `b` on B, with `b < a`, deleted by hand on A. A's
    records place `a` at `P` and none place `b` there, so B keeps its copy although `a ≡ b`.
  - Evaluate "this note's id" as its group.
- **m2. "Found for the first time" rests on a regenerable file** (lines 174–175, 183–185).
  - `state/id-aliases.json` is "rebuilt by every apply", yet it is what tells a new group from a
    known one. If `state/` is lost, every group is new again.
  - Say that the full reconcile is idempotent: converged fields file no card, and any supersede
    records it appends are harmless. Or record the reconciled groups separately.
- **m3. Keys should come from `create` records when one exists** (line 169). A note whose
  `source_uid` or `created_by` was later edited by hand would sit under two keys and could chain two
  groups. Take the key from the note's `create` record, and from the note only when no record exists.
  Both desktops then read the same immutable input.
- **m4. The verdict pull's details** (lines 211–217).
  - An event judgment has no `strength`: the verdict is `{verdict, why, confidence}`
    (`judge_validate.ts:121-128`), and the device writes `""` for it (`events.rs:457-458`). Drop
    "and strength".
  - `title_prefix` is title text coming down from the server. The device can take the title from its
    own feed or roster, or write `(untitled)`, so no text needs to come down.
  - Page by `(judged_at, id)`, not `judged_at` alone, or ties at a 500-row boundary are skipped.
  - A lost cursor, or a fresh desktop, restores lines the student had deleted. Name that.
- **m5. The catch-up must hold `sch.running` too,** or M13's "a slot is running" guard does not cover
  its `sync` (lines 431, 554). Note that the update-install hold also sets `running`
  (`scheduler.rs:86-88`), so *Sync now* will say "a slot is running" during an install. Word it
  generally.
- **m6. `apply_approved_mappings` re-inserts, every slot, a mapping the student removed by hand**
  from one desktop's config. Say that the card is the source of truth, and how to unmap: reject a
  fresh card, or wait for the companion stream.

### The sixteen Minors: all landed

| Minor | Where it landed |
|---|---|
| M1: `order by j` | line 319 |
| M2: highest `rev` | line 152 |
| M3: `target_id` through `resolve_target` | lines 181–182 |
| M4: one digest a day made true by D3 | lines 108–110 |
| M5: the cost named | lines 403–405 |
| M6 | §3.2 (263–266) |
| M7: the proof procedure | lines 592–597 |
| M8: the whitespace-tolerant RLS guard | lines 570–572 |
| M9: two new table lines | lines 380–382 |
| M10: 400 fails open | lines 58, 397–398 |
| M11: `logins` | lines 236, 250 |
| M12 | §5.6 (517–521) |
| M13: *Sync now* refuses during a slot | lines 61, 499, 554 |
| M14: prefixed keys | lines 47, 75–76, 96–104 |
| M15: the wording nits | lines 165, 327, 343 |
| M16: 90 days in Q6, and a new Q12 | lines 673–677, 696–698 |

§8's fidelity rows for "sync merges it" and "The holder runs…" are updated correctly.

### Length

698 lines against the ~610 the rulings accepted. The review's cuts were taken: §0's apply bullet,
§2.4, §2.5's two-cases paragraph, §4.7 and §5.3 all shrank. The growth is the fixes: §2.6, §2.8,
§4.8 with D17, the §6.4 procedure, and Q10–Q12. It is earned. N1–N3 each add a sentence or two, not
a section.

## Re-review of fix round 2 (c599e92)

Scope: `git diff 9caf4b1..c599e92` of the spec, now 779 lines, against `.superpowers/spec/fix-2.md`.
I read in full D15–D19, §2.6, §3.3, §4.5, §4.6, §4.8, §6 and §9, and checked them against the code
at `510a88c` and against CLAUDE.md.

**Verdict: findings open: 1 Critical and 2 Important, all new and all in D17–D19.** N1, N2 and N3
are addressed. Everything else the round changed holds: the 60-second peek, the two-PC proof, the
answers table and the fidelity rows.

### N1–N3

| Finding | Verdict | Where |
|---|---|---|
| N1: one id on two files | **ADDRESSED** | Lines 193–200: with both files here, the loser is merged and archived, never re-id'd; with only the loser here, it is re-id'd. Test (v) covers both files on each side (603–604). |
| N2: the wrong status | **ADDRESSED** | Lines 524–528: `approved` in `approvals/`, or `approved`/`executed` in `archive/`, citing `coursework.rs:1215-1222`. The function is narrowed to run after a conflict (but see R2-I1). |
| N3: sleep and `Instant` | **ADDRESSED** | Lines 430–437: the larger of the `Instant` and `SystemTime` deltas, never compared with the server. The forward and backward jumps are argued, and the §6.2 test injects the clock (619–621). |

### New findings

**R2-C1 (Critical). A secret calendar address can ride the shared `config/ingest.yaml` into
`sync_notes`.**
- D17 classes `calendars:` as shared, on the premise that its entries are markers (`cloud:personal`,
  `cloud:google`; line 480). The code does not guarantee that.
  - `scaffold::restore_capability_url` (`app/src/scaffold.rs:557-583`) writes the raw secret iCal
    address into `calendars:` (`- name: personal / ics_url: <url>`) whenever the account save
    failed twice.
  - A vault from before C2, or a hand-added entry, can carry one too.
- The wedge guard (lines 493–495) blocks only the listed device keys. So that file is pushed, in
  plain text the service can read, into `sync_notes`, and then into every computer and the export.
- That breaks the published *Your calendar links* promise ("stored encrypted … neither is ever
  written to a log", `site/privacy.html:38`). It also breaks C3′ Task 11's ruling that a capability
  URL belongs only in the encrypted `sources` (`scaffold.rs:442-446`).
- **Fix.**
  - A `calendars:` entry whose `ics_url` is not a `cloud:` marker is **device-owned**: the migration
    moves it to `config/device.yaml`.
  - The wedge guard refuses any shared file carrying such an entry, or any `https://` value outside
    the six files' known public fields (`events.yaml`'s campus feeds).
  - `restore_capability_url`'s fallback writes into `device.yaml`.
  - A test pins that no shared file is ever pushed with a capability URL.

**R2-I1 (Important). Two pushes in one interval lose a settings change silently.**
- `sync-push` upserts `sync_notes` on `(account_id, path)` with no condition
  (`_shared/sync_db.ts:23-26`), so the later push wins.
- Take the earlier pusher. After its push, its base is its own text, and its local file still equals
  that base. At its next pull the account's text differs, and D18's first branch ("the local file
  equals the base: write the account's text") overwrites its change. **There is no conflict file and
  no line.**
- With per-job turns this is ordinary. The zyBooks holder and the VHL holder both run `coursework` in
  the same slot, each `apply_map_cards` inserts a mapping into `config/ingest.yaml`, and both make
  the trailing push.
- The lost mapping's card is already `executed` in `archive/`. `apply_approved_mappings` runs "only
  after a conflict", so nothing restores the mapping, and `asked_map_keys` suppresses a re-ask for 30
  days (`coursework.rs:1131-1154`). The book goes unfetched on every computer: the exact loss D17
  exists to prevent.
- **Fix.** Do both:
  - Make a shared-config push conditional. The row carries its base hash, and `sync-push` refuses
    it, with a named reply, when the stored row's hash differs. The pusher keeps its text, and its
    next pull takes the conflict branch, with the file and the line.
  - Run `apply_approved_mappings` after **every** pull that replaces a shared file, not only after a
    conflict. It is idempotent and cheap.
  - Update the §6.1 test, which now pins "never runs otherwise".

**R2-I2 (Important). D19 on a new computer can make wizard defaults the account's settings.**
- `account_vault_exists` answers yes when the account holds **any note** (line 531). The panels are
  then skipped, and Finish writes default shared files for the restore to replace.
- If the account holds no shared config rows yet, the defaults are never replaced. That happens when
  the first computer is still on the old build during the rollout, or has not yet run a `sync` since
  updating.
- After the pull reaches the end, E1 releases the untouched seeds, and the new computer pushes its
  defaults: an empty `course_map`, no mappings, default slots. The first computer's first config sync
  then has no base, so under line 514 "the account's text wins". Its real mappings go to
  `state/config-conflicts/`, and neither computer fetches a mapped book.
- **Fix.**
  - `account_vault_exists` answers yes only when the account holds `config/ingest.yaml`; otherwise
    the wizard asks as today.
  - Mark a wizard default as never providing settings: it is not pushed until a pull has brought the
    account's copy, or until the student changes it.
  - Add the rollout case to Q13.

### D17–D19 against the invariants, C3′'s sync, entitlement and restore

- **Parse and re-dump: holds.** The migration makes line-level moves (line 491), the pull makes a
  verbatim whole-file copy, and the conflict copy is raw bytes. No YAML is emitted from a parsed
  value.
- **"Never rewrite a vault file wholesale": needs a recorded exception.** D18's pull replaces a
  `config/` file whole. C3′'s seed pre-pass is the precedent (`sync.rs:1890`), but CLAUDE.md states
  the rule without an exception. The plan must name this as the second recorded exception and edit
  CLAUDE.md. That includes its scheduler sentence ("`config/runners.yaml` `local` entry says
  `scheduler: app` for this `device:`"), which D17 moves into `device.yaml`.
- **C3′'s path checks: they fit once widened, as the spec says.**
  - Today `sync_notes_path_check` accepts only `^(tasks|…|info)/…\.md$`
    (`migrations/20260912000400_sync_note_path_check.sql`), and so do `NOTE_PATH_RE`
    (`_shared/sync_rows.ts:28`) and `is_note_path` (`sync.rs:359-379`), which requires `.md` and a
    note folder.
  - The spec widens all three to exactly six paths, and §6.3 pins them.
  - Size is no issue: the files are kilobytes against `MAX_NOTE_BYTES` and the 200 MiB ceiling.
    RLS, purge and export come free with `sync_notes`.
- **Entitlement: fine.** Past the grace `sync` does not run, so config neither migrates nor
  travels, and every reader accepts both layouts (line 486).
- **Restore: fine, apart from R2-I2.** `fold_confirmed` records a restored path's hash
  (`sync.rs:1317-1340`), so the base exists from birth. The seed hold-back works as the spec says.

### Other rows

- **D15 and §4.6, the 60-second peek: correct.**
  - A peek is an empty claim and release. The catch-up alone holds `sch.running`, and the 60-minute
    ceiling stays.
  - Each peek also upserts `devices.last_seen` (§4.1): about 60 small writes per refused slot.
    Acceptable. Say it, or let a peek skip the upsert.
- **§6.4, two physical PCs: sound.**
  - `KNOWLU_API_BASE` and `KNOWLU_ANON_KEY` are honoured by every build (`app/src/account.rs:33-38`).
  - `cloud_config`'s host check compares against `api_base()`, so staging passes (`account.rs:704`).
  - The laptop-driven and Quinn-operated split follows the standing proof rules.

### New Minor findings

- **r1. The cursor's "last received" hash applies to the six shared paths only** (line 504).
  - For notes it would be harmful. A pulled text that §2.5 (c) skips would enter `Cursor.notes` while
    no file is on disk.
  - `build_push`'s tombstone pass (`sync.rs:720-749`) would then tombstone the other desktop's live
    path, which D5 (d) settles.
  - Say "for `SHARED_CONFIG` only".
- **r2. Give `config/device.yaml`'s literal shape.**
  - A bare `    enabled: true` line "appended" (line 490) loses its parents.
  - The shape needs `coursework: <source>: {enabled, credential_target}`, the runner's `device` and
    `scheduler`, the `ics_url` fallback and (R2-C1) any raw calendar entry, written from literals the
    way `scaffold::ingest_yaml` writes.
  - Say what a source block left empty becomes (`{}`, not a bare key that reads as null).
  - Say whether `base_url`, which `coursework.rs:739` calls "the device's own operational settings",
    moves.
- **r3. The server should refuse `deleted: true` for a shared path,** so that "never tombstoned"
  holds against any client.
- **r4. Name the rollout warnings.** An old build refuses each config row it pulls with "a pulled
  note named a path outside the vault's notes" (`sync.rs:2197-2200`), once per changed file, until it
  updates.
- **r5. `apply_approved_mappings` after a conflict** re-inserts a card mapping that the student
  removed by hand on the winning computer. "Stays removed" (line 528) holds only without a conflict.
  Say so.
- **r6. D19 lists only "the sources the account's config sets up"** (line 534). A portal first set up
  on the second computer then has no route in the wizard, since the mapping rows are skipped. Say it
  arrives through *Settings → Logins* (C5) and map cards, or show the mapping rows for a source the
  account's config lacks.
- **r7. §6.4's checklist should end by removing the desktop's two user variables.** They persist,
  and a later real install on that PC would talk to staging.

The length (779 lines) is earned by D17–D19 and the two-PC proof. The fixes above add a few lines
each.

## Re-review of fix round 3 (6e6a5fa)

Scope: only `git diff c599e92..6e6a5fa` of the spec, now 871 lines, against `.superpowers/spec/fix-3.md`.

**Verdict: one finding open: 1 new Important, 0 Critical.** R2-C1, R2-I1 and R2-I2 are addressed.

| Finding | Verdict | Where |
|---|---|---|
| R2-C1: a capability URL in shared config | **ADDRESSED** | D17 (line 62); the invariant and `restore_capability_url` now writing `device.yaml` (487–492); the literal shape of `device.yaml` (494–509); a raw calendar entry is device-owned; the guard refuses any non-marker calendar entry and any `https://` or `webcal://` value outside `events.yaml` (522–526); the pinning test (689–692). I checked every path: the wizard, the failed-save fallback, an adopted old vault, the migration's crash window, the pulled text, and the conflict folder (in `state/`, never synced). None can put a URL-shaped calendar value into a synced file. |
| R2-I1: two pushes in one interval | **ADDRESSED** | D18 (63); `save_config_row` (535–549); the refused push becomes the conflict branch (557–559); `apply_approved_mappings` runs after every replacing pull (587–594); tests (694–697, 725–727); the function pin moves to 30 with both new functions revoked (731–734). The compare-and-set is sound as one statement. A concurrent `update … where sha256(body) = p_base` waits on the row lock and then re-checks against the committed row, so the second writer gets 0 rows and is refused. An empty base's `insert … on conflict do nothing` refuses the second first-writer. `sync_notes_stamp_rev` fires `before insert or update` (`20260912000300_sync_plaintext.sql:110-111`), so an accepted update takes a fresh `rev` and reaches the other computer. It writes, so the revoke rule applies, and the spec applies it. |
| R2-I2: wizard defaults published | **ADDRESSED in intent** | D19 (64); provisional files (568–580); `account_settings_exist` answers yes only on a `config/ingest.yaml` row (600–610); the notes-but-no-settings branch (611–617). But see R3-I1. |

**R3-I1 (Important). "Provisional" ends on a byte change, so settings can be published by accident
or never published at all.** (Lines 64, 572–576.)
- A provisional file is held back only "whose bytes still equal that hash", and any change "makes it
  real". Automatic writes change the bytes too.
- **Published by accident.** A new computer that holds the zyBooks job runs `apply_map_cards` when a
  map card is approved (`coursework.rs:1170` onward, a text insertion into `config/ingest.yaml`).
  - That write ends the mark. The fresh wizard answers go up with an empty base and are stored,
    because the account has no row.
  - The working computer, on updating, then has no base. Under "first to sync wins", its months of
    wizard-typed mappings go to `state/config-conflicts/`. That is the outcome R2-I2 was closed to
    prevent.
- **Never published.** A new computer usually appears because the old one is being replaced. If
  the old computer never syncs settings again (retired before it updated), nothing ends the mark on
  the new one.
  - The account never holds settings, and every later computer asks every question again, also
    provisionally.
  - Two such computers keep separate settings, so the turn holder's config again decides what is
    fetched: I1's problem, back.
  - A restore after the new computer dies loses the settings entirely.
- **Fix.**
  - Keep the mark in `Cursor.provisional_config` and end it **only** by an event:
    - a pull that brings the account's text;
    - the student's explicit edit through the app's settings;
    - a named timeout. Recommended: 7 days in which the account has held notes but no settings,
      after which this computer publishes and prints a line. The old computer is then presumed gone,
      and "first to sync" (Q13) still governs if it returns.
  - An automatic write (`apply_map_cards`, `apply_approved_mappings`) re-records the hash and keeps
    the mark.
  - Test both: a map card applied on a provisional computer publishes nothing, and the timeout
    publishes with its line.

**Minors:** 4. They cover:
- the hash convention: the base and the compare-and-set hash the synced text, as `Cursor.notes`
  does, while the seed hashes are raw bytes (`sync.rs:1085-1092`);
- the guard's scope: `events.yaml` carries `enabled: true` per feed (`app/assets/campus/university-of-alabama.yaml:16-36`),
  so the guard must be scoped by position;
- the handler's unconditional upsert must exclude settings rows;
- `save_config_row` should be stated as not SECURITY DEFINER, like `fetch_turn`.

## Round 4 (6018123): R3-I1, checked by the controller

The drafter applied the reviewer's fix for R3-I1 and the four Minors. The controller read the diff (`6e6a5fa..6018123`): the provisional mark is cursor state (`provisional_config`, `provisional_since`); it ends only on the account's settings arriving by pull, a Settings edit in the app, or seven days with no settings in the account; automatic writes never end it; §6.1 pins it. No further review round: the change is the reviewer's own fix, narrowly applied.

## The controller's rulings, round by round

Preserved from the git-ignored workspace (`.superpowers/spec/`). Round 2 carries Quinn's answers of 2026-09-25 to Q1–Q12; Round 3 carries Q13.

---

<!-- fix-1.md -->

### Spec fix round 1: rulings on the review

The review is `docs/reports/2026-09-25-two-desktop-spec-review.md`: READY AFTER FIXES, 1 Critical, 6 Important, 16 Minor.
Apply every finding as ruled below. Where the review proposes a fix and nothing below overrides it, apply its fix.

- **Critical (D5 (a) against D6).** ACCEPT the review's fix: an **alias pre-pass**.
  - Before step 4, build `import_key → id` from the local vault plus every foreign `create` in the page, and settle
    each pair by D6's lower-id rule.
  - Step 4 and `Journal::records_for` then follow the alias, on BOTH desktops. The loser's records are neither
    inert nor lost: a field set only on the losing copy reaches the winner, or becomes one card.
  - Doubles made before this ships are detected by the pre-pass, which gives Q9 its mechanism.
  - Remove the path fallback that reconciles against a different id.
  - State the rule once, in D5 and D6 consistently.
- **I1 (each computer's config decides what the holder fetches and how it is labelled).** Decide it in the spec
  with a recommendation, and add it as **Q10** for Quinn.
  - Split `config/` by who owns each value: account-level values (course maps, labels, the mapping decisions
    approved from map cards) against device-level ones (`credential_target`, `runners.yaml`'s `device:`, anything
    naming a profile id).
  - Recommend the smallest mechanism that makes the holder fetch and label what the student approved on any
    computer, and say what it costs.
  - If the right answer is "account-level config syncs through the account" and that is too large for this stream,
    say so, scope it as a prerequisite or companion, and keep the backstop honest meanwhile.
- **I2 (D8's account-level event verdict cannot be built; `judgments` stores no `why`).** Redesign D8's third
  bullet without free text on the server and without breaking the ledger's delete-a-line re-judge. Options:
  - carry the verdict decision (proposed / not, and a reason code, never text) where it syncs;
  - let the digest notes carry what a moved turn needs.
  Pick one, give the cost, and add it as **Q11**. The 80-a-day cap starvation must stay addressed.
- **I3 (the tombstone rule regresses a hand delete).** ACCEPT. A note created on B and deleted in Explorer on A
  must still be archived on B. Refine D5 (d) so it settles a note whose id the sender's history ever held at that
  path, including through the sender's own `create`, whether or not a record named the delete.
- **I4 (the 90-day promise is false for idle or lapsed accounts).** ACCEPT the `pg_cron` fix: a daily job, like
  the project's existing cron jobs. It needs a Vault token, or it is plain SQL if no function is involved; say
  which. Keep the privacy sentence true.
- **I5 (D14 after a normal wake).** ACCEPT.
  - A renewal the service cannot answer counts as granted only while the holder's own monotonic time since its
    last grant (a local `Instant`, never compared with the server's clock) is under the lease length. Past that,
    it counts as refused.
  - Say what the in-flight step's timeout does to the tray, and fix proof step 6's expectation to match what the
    code will do.
- **I6 (the privacy wording overpromises).** ACCEPT. Rewrite §3.3 so every clause is true under per-job leases,
  fail-open, and `rank`'s own event and calendar reads, and add the sentence saying the account knows which
  computer holds which login.
- **Minors.** Apply each as the review proposes, unless one conflicts with a ruling above; if it does, note why
  in the commit message.
- **The review's missing questions.** Q10 and Q11 are added above; take the review's wording where it is better.
- **Length.** Take the review's cuts. About 610 lines is accepted as earned; do not cut a table the plan writer
  needs.
- **Fidelity table.** Update any row that changes.

---

<!-- fix-2.md -->

### Spec fix round 2: Quinn's answers (2026-09-25) and the re-review

## Quinn's answers to §9. Fold each in and record it in §9 as answered.

- **Q1: one lease per job.** Accepted as written (D11).
- **Q2: the catch-up, yes, and faster.** Quinn asked for a response sooner than ~20 minutes. The rule:
  - A refused computer **peeks every 60 s** from its slot's end.
  - It runs the catch-up (pre-flight, `sync` both ways, `rank --no-digest`) as soon as every job it was refused
    is released or expired.
  - The ceiling stays 60 minutes after the slot began.
  - The holder already releases right after its trailing push, so the others get the results about a minute
    after the holder finishes, not after the lease length.
  - Rewrite D15 and §4.6 to this. The cost is up to ~60 peeks per refused slot, each a cheap `/turn` call with
    empty claim and release. Say that.
- **Q3: no head start now.** As written.
- **Q4: archive quietly.** As written.
- **Q5 and Q12: fail open, advisory now; enforce for coursework at C5's `/relay`.** As written.
- **Q6: no re-ask screen; 90 days.** As written.
- **Q7: the proof runs on TWO PHYSICAL COMPUTERS.** They are Quinn's laptop, where the controller runs, and
  Quinn's desktop PC. Rewrite §6.4:
  - The laptop is driven by the controller through the autonomous-proof harness: a dev build copy, WebView2
    remote debugging, DOM only, never synthetic input.
  - The desktop runs a candidate build Quinn installs. Say how it is made: a `scripts\release.ps1 -DryRun` bundle,
    or a dev build copy, pointed at staging. Say how it is pointed at staging without a secret.
  - The desktop is operated by Quinn from a short checklist the controller gives at proof time (sign in, *Run
    now*, sleep, edit or delete a note), unless a Claude Code session on the desktop drives it. Recommend the
    checklist.
  - The controller verifies from staging reads and the laptop.
  - Real sleep replaces `pssuspend`: Quinn sleeps whichever computer holds the turn.
  - The two-Windows-users route becomes a fallback, not the gate. Keep §3.2's shared-`COMPUTERNAME` limit, but
    drop `KNOWLU_DEVICE` from the main path.
  - Keep every "what it must show" item that still applies, and say which computer does what.
- **Q8: the computers list after the pilot.** As written.
- **Q9: join doubles made before this ships.** As written, with N1's fix below.
- **Q10: THE ACCOUNT HOLDS THE SHARED SETTINGS, in this stream's scope.** Quinn asked why the profile would ever
  differ between computers; it should not. So the split of `config/` in §4.8 is no longer a recommendation for a
  companion; it is designed here, and two computers are not called working until it ships. Design it: new D-rows,
  and rewrite §4.8 as the design.
  - **What is shared:** the account-owned values in §4.8's table.
  - **What stays on the device:** `credential_target`, `enabled`, `runners.yaml`'s `device:`/`scheduler:`,
    `cloud.yaml`, and a vault-held `ics_url` fallback.
  - **The mechanism:** recommend the smallest correct one. For example, move the device-owned keys out of the
    shared files into a device-local file, so the shared `config/` files can travel through C3′'s existing sync
    as text. Or, if that is wrong, a field-level account store. Weigh it against: text insertion only (never
    parse and re-dump), last-writer-wins per path versus conflicts, and existing vaults' `config/` layout (a
    migration that moves keys, done through the one allowed edit path).
  - **The second computer's wizard:** after sign-in and restore, it skips every question the account already
    answers (course maps, labels, timezone, calendars). This matches the login-only onboarding direction:
    students bring only logins. It asks only for this computer's own logins.
  - Q10 is now answered; remove it as an open question. Add any genuinely new question this raises, with a
    recommendation.
- **Q11: pull event verdicts.** As written.

## The re-review's new findings (appended to docs/reports/2026-09-25-two-desktop-spec-review.md)

- **N1 (the alias re-id leaves one id on two files).** ACCEPT the reviewer's fix. When this vault holds both files
  of an alias group, the losing file is merged into the winner and archived through `write::delete`; it is never
  re-id'd into a second file with the same id. The merge is field by field through the alias's reconcile, so a
  field set only on the loser reaches the winner or becomes one card. Re-id only when the loser is the one file
  this vault holds for the item.
- **N2 (`apply_approved_mappings` looks for the wrong status).** ACCEPT. Read cards with status `approved` in
  `approvals/`, or `executed` in `archive/`, as `apply_map_cards` leaves them (`coursework.rs:1215-1222`). If Q10's
  account-held config makes this function redundant, say so, and keep it only if it still has a job, such as
  existing vaults before the move.
- **N3 (I5's elapsed time may not count sleep).** ACCEPT. The holder's local elapsed time since its last grant is
  the LARGER of `Instant` elapsed and wall-clock elapsed. It is still never compared with the server's clock. A
  forward jump of the wall clock can only make the holder yield early, which is safe because the backstop covers
  it. Say so.
- **The six new Minors:** apply each as the reviewer proposes, unless one conflicts with the above.

## Also

- **The status line:** "Quinn's answers of 2026-09-25 folded in (§9); for signature."
- **The fidelity table:** add rows for Quinn's 2026-09-25 answers where they change a D.
- **Length:** whatever it takes, without padding. Write in pieces of at most ~100 lines.

---

<!-- fix-3.md -->

### Spec fix round 3: the round-2 re-review (appended to the review file) and Q13

- **R2-C1 (Critical): a capability URL could reach the shared config.** ACCEPT the reviewer's fix.
  - A `calendars:` entry that is not a `cloud:` marker is DEVICE-owned. It moves to `config/device.yaml` with the
    other device keys.
  - The push guard refuses any shared file that carries a URL-shaped calendar entry, or any key D17 names as
    device-owned, and names the refusal as a line.
  - Restate the invariant in D17: a capability URL never enters a synced file (C3′ Task 11's ruling; the privacy
    page's promise that calendar links are stored encrypted in the account).
  - Add a test to §6 that pins it.
- **R2-I1 (Important): two pushes close together lose a settings change.** ACCEPT both halves.
  - `sync-push` for a config row is conditional on the base hash the device last synced: compare-and-set. A
    mismatch is a conflict the device resolves on its next pull, never a silent overwrite.
  - `apply_approved_mappings` runs after EVERY pull that replaces a settings file, not only after a conflict.
  - Say how the conditional push fits `sync-push`'s existing handler and row shape.
- **R2-I2 (Important): the second wizard can publish its defaults.** ACCEPT.
  - `account_vault_exists` (or whatever D19 names it) answers yes only when the account holds `config/ingest.yaml`.
  - An untouched wizard default is never pushed as shared config. Say how "untouched" is known: C3′'s seed-hash
    approach is the precedent.
  - Say what the second computer's wizard does when the account has notes but no settings yet: it asks the
    questions as a first computer would, and marks its answers as provisional? Decide with a recommendation. Add
    a question only if it is genuinely Quinn's.
- **The two plan-level points:** put them in the spec now.
  - D18's whole-file replacement of a config text is a second recorded exception to CLAUDE.md's "never rewrite a
    vault file wholesale". Name it in D18, and name the CLAUDE.md edit the plan must make, as a hand-off.
  - Spell out `config/device.yaml`'s shape: its keys, one example, and its creation for an existing vault.
- **The seven new Minors:** apply each as proposed, unless one conflicts with the above.
- **Q13 (answered by Quinn, 2026-09-25): the first to sync after the upgrade wins.** Mark it answered in §9 and
  make sure the design says exactly that.
- **Commit:** the spec alone, the fourth commit, message `docs: spec — two desktops: the round-2 review (no capability URL in shared config, conditional config push, wizard defaults never published) and Q13`.
  Trailers: `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_0116SUa7sU6BMt66hiEXrjTU`. Edits of at most ~100 lines each.
