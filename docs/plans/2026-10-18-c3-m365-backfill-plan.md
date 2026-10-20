# C3: Microsoft 365 backfill, read in the page and upload from the device (implementation plan)

**Date:** 2026-10-18. **Status: DRAFT r12; the stamp rule's fourth re-review PASSED (2026-10-19) and
its minors are closed (r12)**, then Quinn at the checkpoint. Nothing here is built. r2 resolved the plan review's round 1: the hold's removal moved into
the routing work, every existing test the routing breaks is named with its fate (§6, "Tests the routing
retires or rewrites"), the migration is never renamed after C1 (§5), four oversized tasks were split
(R2a/R2b, R3b/R3d, R7b/R7c, R9a/R9b), and the minors were fixed. r3 resolves round 2:
- R9c is split per path into R9c1 to R9c5: the routing, the fake hosts, setup and Retry, launch resume,
  and the slot with the function's removal. The hold function stays until its last call site goes.
- The fake hosts' upload route, the `wait` helper's D13 branch and an empty-mailbox default are now
  named tasks. Two more setup tests that reach the pass are added to the table.
- The synthetic id prefix and the Exchange-id pattern are pinned (R1, R3a).
- The charge RPC's single-statement form is pinned (R6), an upload with no declared length is a 411
  (P7, R7b), and C1.8 checks the compare-and-set on Postgres.
- `docs/reference/app.md` is named as shared with the Docs lane (§5).
r4 resolves the convergence review's round 1. Where the batch's `received` compare-and-set runs is now
pinned (P7): the binding guard runs inside the upload hand-off's own transaction, after every refusal,
and step 2 keeps only a pre-check that reserves nothing. R6, R7a, R7b, R7c, C1.6 and C1.8 follow from
that. New tests check that a refusal after the charge leaves `received` unchanged, and C1.8 also counts
`mail_work` rows.
r5 resolves the convergence review's round 2:
- `migrations_test.ts` is now treated as a shared file (handoff §5). R6 moves the execute-revoke guard's
  pinned function count by three and adds a clause to its comment. §5 gives the merge-order rule for
  that one line.
- R6's three RPCs are `security invoker` and service-role only, like `mail_hand_off`, with `execute`
  revoked from `public`, `anon` and `authenticated`. A new test pins this.
- R7c is split into R7c1 (steps 3 to 6, the refusals) and R7c2 (steps 7 and 8, store and count). The
  reconcile hashes move to their own task, R7d.
r6 resolves the convergence review's round 3. R6's upload hand-off RPC now opens with `mail_hand_off`'s
share-locked mailbox check (Checkpoint B), word for word, before the batch's guard, and answers `gone`
for a mailbox that is gone or revoked. Disconnect's purge depends on that check. On `gone`, R7a's
shared hand-off takes the put back out, and R7c2 answers with step 1's revoked-mailbox refusal. R6
and R7c2 each gain a test, R7a's `full` test gains a `gone` case, and P7 and the ledger (§10.2)
follow.
r7 (2026-10-19) aligns the migration stamp with the shared rule M1-M8 (L3's plan §2.1, L8's spec
§8.4), after Q8's ruling and the L3/L8 convergence review's important finding. §5: R6 and R13 pick and
re-check against `main`, the stamps L3, L8 and L5 recorded, and staging's applied versions, using the
rule's next free stamp instead of "one minute after the newest on `main`"; no staging or prod push uses
`--include-all`, and a forced rename follows M6. P11, R13, risk 4 and the ledger row follow. Q8's
ruling is unchanged.
r8 (2026-10-19) follows the stamp-rules review's important finding (recorded in L3's plan §9.00 item
7): M7 now clears a lane ahead only once its migration is on staging or it is recorded as slipped,
never on its merge alone. §5 and C1: C1 waits for Q8's merges **and** for each merged lane's migration
to be on staging (stricter than Q8's words; Quinn hears it at the next checkpoint). §5's remedy for a
version a staging push finds out of order now has one answer per case: not merged, its lane re-stamps
it by M5; merged but not pushed, the controller's re-stamp commit on `main` by M8 before its first
push; pushed, it stays. R13, risk 4 and the ledger row follow. Q8's ruling text is unchanged.
r9 (2026-10-19) follows the re-review of r8 (recorded in L3's plan §9.00 item 8): a merged, unpushed
lane recorded as slipped still rode into staging inside the next lane's rebased `db push`. By the
rule's revised M7, a lane's migration merges only with its staging push (the controller pushes right
after the merge, so "merged but not pushed" never waits), a recorded slip clears only an unmerged
lane, and every staging push, C1's included, first runs `supabase db push --dry-run` and stops if the
list holds any version but the pushing lane's own. M8's re-stamp commit stays only as the remedy if
that is ever broken. §5, R13, C1, risk 4 and the ledger row follow. Q8's ruling text is unchanged.
r10 (2026-10-19) follows the rule's second re-review (recorded in L3's plan §9.00 item 9): a lane that
pushed from its lane branch but had not merged counted as cleared and could slip, though every dry
run behind it then refuses (staging holds a version `main` lacks), and the CLI's
`migration repair --status reverted` hint would let C1 or another lane push past it, after which that
lane's push from `main` re-runs non-idempotent SQL. By the rule's revised M7, a lane ahead is cleared
only once its migration is on staging **and** on `main`; a lane that pushed from its lane branch
holds every lane behind it, C1 included, until it merges; a recorded slip clears only a lane that has
neither merged nor pushed; a dry-run refusal naming a remote version with no local file waits for
that merge and a rebase, never `migration repair` outside M6; and no lane merges until its dry run on
the rebased lane branch lists its own version alone. §5, R13, C1, risk 4 and the ledger row follow.
Q8's ruling text is unchanged; C1's wait is now stricter than its words in a second way (§5).
r11 (2026-10-19) follows the rule's third re-review, its r7 (recorded in L3's plan §9.00 item 10). A
lane that pushed from its lane branch and then stopped could never be cleared, so it held C1 for good.
By the rule's revised M7, only the controller **withdraws** such a lane: it drops the lane's objects
on staging, then removes its version row (`migration repair --status reverted`, safe once the objects
are gone), redeploys from `main` any function the lane deployed from its branch, and confirms by a dry
run on `main`. The lane then counts as neither merged nor pushed. `migration repair` runs only for
M6's rename and in a withdrawal, a dry-run refusal waits for the lane ahead's merge or withdrawal, and
a remote version no lane recorded goes to Quinn. M8 adds a revert of a merge that broke M7 behind a
pushed, unmerged lane, on Quinn's word only. L1 itself pushes at C1 before it merges, so C1 gains a
withdrawal step. §5, R13, C1, risk 4 and the ledger row follow. Q8's ruling text is unchanged.
r12 (2026-10-19) follows the rule's fourth re-review, of its r7 (**PASS**, five minors and one M5
note; recorded in L3's plan §9.00 item 11), closed in the rule's final pass, r8. M7's withdrawal now
carries **Quinn's ruling of 2026-10-19**: the controller withdraws a lane on its own when its
staging proof shows a defect in its SQL, records it in the session ledger and tells Quinn; a merge
Quinn declined is Quinn's word; any other case asks Quinn first; a defect in the functions alone is
redeployed from the lane branch. The withdrawal redeploys from `main` first and drops second (so
`main`'s `mail` and `mail-inbound` are live before `mail_batches.via` and the upload RPCs go, and no
forwarded message meets a half-withdrawn schema), runs the drop as one `begin … commit` and checks
the catalogue before the repair. M8 offers Quinn two remedies for a broken merge behind a pushed,
unmerged lane: the revert, or a withdrawal of that lane, which rewrites nothing on `main`. §5's
remedy list, C1's withdrawal and the ledger row follow. Q8's ruling text is unchanged.
**Governs:** `docs/specs/2026-10-10-m365-backfill-redesign.md` ("the note"), v2 of 2026-10-13. Its §5
decisions are all ruled, its §6 pre-build live gate is closed, and its §7 lists tasks R0 to R12. The
note amends the signed email spec, `docs/specs/2026-09-30-email-forwarding-design.md` ("the email
spec"), for one routine: Microsoft 365's "Bring in recent mail" (email spec §4.2 step 5), plus the saved
session's recover (§4.11 steps 3 and 4). The rulings of record are the progress ledger
`.superpowers/sdd/2026-09-30-email-forwarding/progress.md`: the C3 rows of 2026-10-10 to 10-18, and
"QUINN RULING D18", "QUINN RULING W8(b)", the W8 narrowing ruling in the REVIEW PASS row, and the MVP
planning rows of 2026-10-18.
**Above it:** the cloud design and its amendments, then `VISION.md` and `CLAUDE.md`'s two overriding
rules. The session-5 handoff (`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md`) has two
sections that bind this lane: §1 (rulings) and §5 (lane L1 and the conflict map).
**Lane:** L1. **Branch** `c3-m365-backfill`, with worktree `.claude/worktrees/c3-m365-backfill`. It is
cut from `main` **after the email-forwarding merge** and its sha is recorded in the lane ledger
`.superpowers/sdd/2026-10-18-c3-m365-backfill/progress.md` (dated rows; created by the controller in
R0). The plan is written in the email worktree only so it can be reviewed now. If the email branch merges
before this file is committed, the controller commits it as the first commit on L1's branch.
**Size:** L. **Slip rule (handoff §7):** C3 does not block friends. If it slips, Microsoft 365's
backfill stays guided (D13, `29a0ddb`) and L9 and L10 go ahead without it.

## 1. Goal, and what must not move

A Microsoft 365 student's "Bring in recent mail" reads the mailbox in the signed-in, hidden mail window,
using only OWA's own same-origin read services. It counts the received mail of the fixed 120-day window
exactly, then uploads each original the service does not already hold, from the device to Knowlu's
`mail` function. The route judges it like any backfilled original and hands it to `mail-process`. The
mailbox is never sent from and never written to. The saved session's daily recover runs the same pass.
The progress view shows "N of M messages".

VISION check:
- Success 2 (nothing missed) and success 5 (the first session already knows what's next) are the reason
  for the lane.
- "Knowlu reads" (A8) gets the reduction that D18 signed: Knowlu writes less in the mailbox.
- Commitment 3 holds trivially, since nothing is sent.
- "Fetch on device, think in the cloud" holds: the device fetches and uploads, and the cloud judges.

These must not change:

- **The contract list.** No task edits `engine/src/{write,journal,yamlemit,yaml,pystr,ledger,ids,
  provenance,approvals,sync,entitle,wincred,reconcile}.rs`, `app/src/{credentials,account,updates}.rs`,
  the oracle, sync or entitlement tests, or `engine/tests/fixtures/**` (note §7). Every gate checks
  `git diff --stat origin/main...HEAD` over that list and expects it empty. `account.rs` is L4's and
  `approvals.rs` is L5's (handoff §5).
- **Frozen references.** None is regenerated; `engine/tests/fixtures` is untouched. The two new
  fixtures (`owa_shapes.json`, `message_key_vectors.json`) are lane fixtures, not references (note §4.11).
- **The engine.** It is not touched. The chain `sync → coursework → ingest → grades → judge → rank` and
  `slot_argv` stay as they are. The `mail` step is still the app's and runs before the chain.
- **Gmail.** Gmail's scripts, batch sends, Sent cleanup and send pace are unchanged (note decision 22).
  So are intake's batch-send path, live forwarding, the canary and the setup order (decision 21).
- **D5's floor.** The product uses no CDP, no debugging port and no OS input. Decision 19's browser
  arguments are **not** added, because W1 passed with none (§2). `crate::debug_port` stays a
  harness-only seam that no release build reaches.
- **Rule 1.** No school, tenant, mailbox, address or person is named in code, tests, fixtures or copy.
  Every test value is synthetic. W9's numbers are evidence for the caps, never constants keyed to a
  tenant.
- **Privacy.** No new class of data leaves the device (note §4.9). There is no new telemetry row shape.
  An original's bytes are never written to disk, a log, a `Stop`, an error string or telemetry.
  `PRIVACY_VERSION` does not move in this lane (D18); `site/privacy.html` is L4's (§4, Q3).
- **Tests never leave the machine.** Every server a test needs listens on `127.0.0.1:0`. No Credential
  Manager target is written or read by a new test.
- **Line endings.** LF everywhere. No fixture under `engine/tests/fixtures` is touched.

### 1.1 Hard rules of this lane (each has a named test)

1. **An original is always a mailbox item's own MIME, never an item attachment** (reviewer minor #2,
   2026-10-18, ruled with the REVIEW PASS row).
   - `mime.js` reads only the `MimeContent` of the listed item id.
   - No script and no `init.js` path names `GetAttachment`, `GetAttachmentPreviews`, `AttachmentId` or
     `CreateAttachment`, and the read allow-list holds none of them.
   - The device never parses a `message/rfc822` part out of an item to upload it.
   - The upload route never unwraps one. A body whose top-level `Content-Type` is `message/rfc822` (a
     wrapper) or `message/partial` is a 422. A message that carries an attached message is judged and
     stored as the outer message only.
   - Tests: `m365_scripts_never_read_an_attachment`, `a_chunk_mismatch_drops_the_original`,
     `upload_refuses_a_wrapped_message_and_never_unwraps_an_attached_one`.
2. **The guided Backfill step never tells a student to forward as an attachment** (minor #2's copy half,
   handoff §6). This covers the D13 guided row, which C3 removes for the normal path, and the C2 fallback
   shown on a named `not found`. Neither may say "forward as attachment", "attach" or "forward these
   messages" for Microsoft 365. The copy is Quinn's (§4, Q1). Test (console-ui, R11):
   `the_m365_backfill_guide_never_asks_for_a_forward_as_attachment`.
3. **Reads only, enforced in the closure** (decision 3). The network allow-list is exactly
   `GetFolder`, `FindFolder`, `FindItem`, `GetItem` and `GetOwaUserConfiguration` (W6's address read).
   Anything else is refused before any request is made.
4. **OWA's bearer never leaves the page, and Knowlu's session never enters it** (I9).
5. **One original in memory at a time**, on the device and in the page's slot.

## 2. What the live gate settled, and what it changes in the note

Read from the ledger rows of 2026-10-13 and 2026-10-18. Every check stayed read-only in the mailbox
under D11's scope.

| Check | Result | What this plan does with it |
|---|---|---|
| W1 | **Pass.** The app's own hidden mail window (`mail::open_hidden`, `visible(false)`, `skip_taskbar`) answered `GetFolder(inbox)` with 200 in 114 to 120 ms at 0, 6, 12 and 20 minutes. Freeze count 0, resume count 0, `wasDiscarded` false. `visibilityState` read `visible` throughout: WebView2 keeps the controller visible when Tauri hides the HWND. | **Decision 19 is not needed.** W2 and W3 never run, and R10's "window" half is void: no browser arguments, no `put_IsVisible`. The guard of §4.3 items 1 to 3 is still built, as new code: `alive.js` (R4) and the closure's freeze and resume counters (R2b). It covers a runtime or power mode W1 did not see. |
| W4 | **Pass.** `FindItem` honours `Body.Paging` = `{"__type":"IndexedPageView:#Exchange", ...}`: 5 rows, and offset 5 gives 5 new ones. `RootFolder` carries `IndexedPagingOffset`, `TotalItemsInView` and `IncludesLastItemInRange`. OWA's own `FindItem` bodies were not visible to the page (a worker). | `items.js` sends paging in `Body.Paging` **and** keeps the received-time slices (note §4.2). Slicing is still the guarantee and paging is the bound inside a slice. `owa_shapes.json` records `RootFolder`'s three keys. |
| W5 | **Pass.** `GetItem` with MIME left `IsRead` false and the Inbox unread count unchanged. | I4's read-state half is proven. The re-proof samples it again (§8). |
| W6 | **Pass.** `GetOwaUserConfiguration` gives `SessionSettings.UserEmailAddress`, which was compared in the page with the hint and matched. | The fifth allowed action is `GetOwaUserConfiguration`. `whoami.js` compares the case-folded address with the passed-in hint and returns only `{address_matches}`. |
| W7 | **Pass.** A 4,314,607-byte original moved in six 1 MiB base64 slices through the script-result channel. Length, SHA-256 and Message-ID matched, and the slot was empty afterwards. | 1 MiB chunks, as the note says. |
| W8 | **Pass** after `ac08d6d` and `cdeff72`. Exchange's stored MIME carries **no ARC set**. External mail aligns through the delivery stamp (authserv-id `mx.microsoft.com`, with or without ` 1`). Intra-tenant mail aligns through exactly one `X-MS-Exchange-Organization-AuthAs: Internal` on a sole From at the hint's domain, under it, or a parent of it no higher than `registrable(hint)`, never a public suffix. `X-MS-Exchange-CrossTenant-Id` is **present on external mail**. | The route calls `originalAuthenticity(original, mailbox, ...)` unchanged. Decision 17b applies, because W8 shows the header (§3 P6). |
| W9 | 120-day window over the 4 allowed folders, 72 week slices, 0 failed: **1,139 items, 332,981,327 bytes (about 318 MiB), largest 18,066,786 bytes (about 17.2 MiB), 0 over 25 MB.** | Decision 16's caps are confirmed as recommended (§3 P4): every original fits under 25 MiB, a whole window fits one backfill day's byte cap more than four times over, and 1,139 is under the day's 1,500 and the window's 3,000. |
| W10 | **Pass by proxy.** A Gmail-sent synthetic message, forwarded live, was bound in about 30 s. The exact `msg_hash` cannot be computed off the server. | Decision 8 stands. The re-proof adds an exact check (§8): a message forwarded live during the pass is answered `held` or `duplicate`, never queued twice. |

**What else changed since the note was written.**
- The D13 hold is built (`29a0ddb`): `mail_steps::backfill_held_for_c3` guards four separate paths.
  They are setup Backfill (`Setup::new`'s `held` closure and the Backfill block after it, which make the
  step `Mode::Guided`), Retry (`Setup::rearm_backfill` refuses M365), launch resume
  (`backfill::resume_one`) and the slot's recover pass (`slot.rs`, before its `run_pass`). Each path is
  tested on its own, so C3 lifts the hold one path at a time (R9c3 to R9c5), after the routing (R9c1)
  and the fake hosts (R9c2). The function stays until its last call site goes, in R9c5.
- `recover::run_pass` holds both providers today. The C3 pass goes into a new module, so Gmail's
  half stays byte-for-byte as it is (§3 P2).
- The email spec's stamp rule (email spec §4.6 step 6) is fixed by L0 before the merge (handoff §6).
  R12 checks that it reads as W8 ruled, and fixes it if L0 did not.
- `POST /reconcile` already pages `mail_seen` past PostgREST's 1,000-row cap
  (`functions/mail/index.ts`, `held`). Message-ID hashes ride the same paged read.

## 3. Decisions this plan makes (reviewer checks each; none changes a ruling)

- **P1. The closure's one function, with local verbs.** The note says the closure exposes "exactly one
  function, `read(action, body)`", but it also puts the MIME slot and the lifecycle counters in the
  closure. Both hold if there is one non-enumerable, non-writable global function (its name is fixed in
  R2a and pinned by test) that takes two kinds of verb:
  - **Network actions:** exactly the five of §1.1 rule 3. Each makes one same-origin `service.svc` POST,
    with the body in `x-owa-urlpostdata`.
  - **Local verbs**, which never make a request: `#state` (`{visibility, frozen_count, discarded,
    closure_ready}`), `#mime` (runs `GetItem` with `IncludeMimeContent` for one id and parks the
    answer) and `#chunk`.

  The Deno allow-list test pins both lists exactly. `contract-reviewer` reads this choice first.
- **P2. A new module for the pass.** The C3 pass lives in `app/src/mail/upload.rs`, behind the
  existing `Mailbox` and `Clock` traits and a new `Uploader` trait that `MailClient` implements.
  `recover::run_pass` sends a Microsoft 365 job there and keeps Gmail's code unchanged. This keeps R9a
  and R9b testable without the window, and keeps `recover.rs`'s diff small.
- **P3. `open_batch` keeps its signature.** R8 adds `open_batch_via(.., Via)` and `upload_original`,
  so the Gmail callers in `recover.rs` do not move in R8. Tests: `app/tests/mail_client.rs` (new).
- **P4. Caps, set from W9 (decision 16, ruled at its recommendation).** Bytes are binary units, stated
  once in each language:

  | Cap | Value | Device constant (`upload.rs`) | Server constant (`mail/upload.ts`) |
  |---|---|---|---|
  | One original | 25 MiB = 26,214,400 bytes (W9's largest: 18,066,786) | `MAX_ORIGINAL_BYTES` (listed `Size` and the assembled length) | the same; a larger declared or streamed body is a 413 |
  | One backfill day, per mailbox | 1,500 originals and 1.5 GiB (W9's window: 1,139 and 318 MiB) | none | `DAY_CAP.backfill` |
  | One recover day, per mailbox | 150 originals and 300 MiB | none | `DAY_CAP.recover` |
  | One slot's recover pass | 30 originals (`SLOT_CAP`) and 10 minutes | `SLOT_CAP`, `SLOT_BUDGET` | none |
  | One first window | 3,000 messages (W9: 1,139) | `MAX_WINDOW_MESSAGES` | none |

  Every cap sits below intake's `MAX_OBJECT_BYTES` (40 MiB). C1 proves the platform passes a body just
  under the cap (§8). If it does not, the cap comes down to fit and Quinn hears it (decision 15).
- **P5. Throttling survives a restart.** "The next run is at least an hour later" (I5) is kept as
  `throttled_until` on the mailbox's `mail.json` record (`mail_session.rs`, read with serde's default,
  so older files still parse). `run_m365_pass` takes no path (R3c), so it never reads or writes the
  field; it only reports a throttle in its `Outcome`. Three paths run that pass, and two call sites
  cover them:
  - **`backfill::drive_backfill`** serves both the setup's Backfill (`runner.rs`, through the `Drive`)
    and launch resume (`backfill::resume_one`). At the head of each loop turn it reads the field, and
    while `now < throttled_until` it holds to that instant, as it holds for a spent day (`resumes_at`
    set, `paused: Outlook asked Knowlu to slow down`), with no pass and no read of the page. After each
    pass it writes the `Outcome`'s `throttled_until` before anything else and holds to it, never to the
    next UTC day and never as a failure. A pass that ends without a throttle clears the field.
  - **`slot.rs`**, before its `run_pass`: a mailbox inside its hour gets no pass and a named line, and
    the slot writes a throttle it hits the same way.

  `runner.rs` gets no throttle line of its own, because its Backfill reaches the pass only through
  `drive_backfill`. A second write there would be a second writer of one field. Q4 covers L4's overlap.
- **P6. Decision 17b is adopted**, as ruled "if W8 shows the header". The route answers `foreign`, and
  stores nothing, when the topmost `X-MS-Exchange-CrossTenant-Id` differs (case-folded) from the
  mailbox's `forward_hint`. An absent header, or a null `forward_hint`, never refuses (W8 recorded the
  header on external mail only). The re-proof records as booleans whether intra-tenant MIME carries it.
- **P7. Charging the caps.** The day counters are charged by one compare-and-set RPC at route step 2,
  by the declared body length, before anything is parsed or stored. Nothing is refunded:
  conservative, and a duplicate costs one slot of the cap. The batch's `received` stays at or under
  `expected`, also by compare-and-set, but **that compare-and-set is not at step 2**:
  - **Where the batch's count runs.** The binding guard on `received < expected` is the note's §4.5
    step 8 ("count"). It runs inside the same transaction as step 7's hand-off, the one that writes the
    `mail_work` and `mail_seen` rows, and never earlier. A refusal at steps 3 to 6 (not one message,
    outside the window, `own`, `foreign`) and a 500 at step 7 therefore never touch `received`.
    `received` counts exactly the `queued` and `duplicate` answers, as the note says.
    - A `queued` upload is counted by a new upload hand-off RPC (R6). In one transaction it first
      runs `mail_hand_off`'s share-locked mailbox check, then the guarded `update` of the batch, then
      the two inserts. For a mailbox that is gone or `revoked`, the RPC answers `gone` and writes and
      counts nothing. The route then takes the put back out and gives step 1's revoked-mailbox
      refusal. That check locks the mailbox and counts nothing, so the single-statement rule below
      does not apply to it. If the batch's guard matches no row the RPC answers `full` and writes
      nothing, so the route takes the put back out (as intake does today when a concurrent delivery
      wins) and answers the batch's 429.
    - A duplicate counts too. If step 7's `mail_seen` lookup finds the hash before the put, the route
      runs only the guarded `received` update (R6). If the hash lands in a race, the RPC's `mail_seen`
      insert conflicts after the guard has matched, and the transaction keeps the count and skips
      `mail_work`. If either of the batch's guards refuses, the answer is the same 429.
    - Step 2 still reads the batch row that step 1 loaded and refuses with that 429 when it already
      reads `received >= expected`. That is a cheap pre-check before the day caps are charged. It
      reserves nothing and is not the guard.
    - **Why not reserve at step 2 and release on a refusal.** A release is a second write. A function
      that dies between the two leaves a phantom count, and then `received` exceeds what the device saw
      queued or duplicate. Guarding inside the transaction needs no release. The cost: when uploads race
      past `expected`, each one past it is parsed, judged, put and then taken back out, and still pays
      its day-cap charge (nothing is refunded). That cost is bounded by how many uploads the device has
      in flight. C1.8 checks it on Postgres.
  - **The compare-and-set is one guarded statement**, never a read followed by a write. That is a
    single `insert ... on conflict (...) do update set ... where <both sums at or under the caps>
    returning`, or an `update ... where ... returning` on a row created first. A plain `select` and
    then an `update`, or `select ... for update` in a function, is not the form (R6 pins it). The same
    holds for the batch's guard, both alone and inside the upload hand-off RPC.
  - **An upload with no declared length is refused** with 411 and a fixed sentence, before anything
    is read. A body longer than it declared is a 413 once it passes the declared length. The device
    always sends `Content-Length` (R8).
- **P8. The window cap moves `since` by bisection.** If the 120-day count is over 3,000, the device
  bisects by whole days over `[since, until)` with `count.js` (at most 7 rounds). It takes the earliest
  day whose count fits, before `POST /batches` fixes `backfill_since`. Each round is a few calls at the
  250 ms floor.
- **P9. A resume for a mailbox the service no longer has.** The proof profile holds two stale
  `open_backfills` ids. A 404 from `own()` on resume drops that marker with the named line `this
  mailbox is no longer set up` and opens nothing. Any other failure keeps the marker.
- **P10. No new Tauri command.** Progress travels through the two existing commands,
  `mail::mail_setup_progress` and `mail::mail_status` (both in `app/src/mail.rs`, already registered in
  `main.rs`), whose answers are built by `progress_of` and `status_for` in `app/src/mail/commands.rs`.
  It travels as new fields:
  `unit: "messages"`, `bytes_uploaded`, `left_too_large`, `skipped` and the pause word. If a task finds
  it needs a command after all, it hands the `generate_handler!` line to L6's owner (handoff §5) and
  does not edit `app/src/main.rs` or `app/src/commands.rs`.
- **P11. The migration is idempotent**, so that any second apply is harmless: chiefly the one forced
  rename §5 allows (M6), if the controller finishes it with a plain `db push` instead of marking the
  new version applied. No planned push uses `--include-all` (§5, Q8). That means `add column if not
  exists`, `create table if not exists`, `create or replace function`, and grants and policies dropped
  and re-created. Idempotence is a safety net, not a licence to rename: §5 forbids a rename after C1
  except by M6's procedure.

## 4. Open questions for Quinn (each with a recommendation; none blocks R1 to R8)

**All eight ruled 2026-10-19 (Quinn)**, each as recommended; each ruling is the dated line under its
question, quoted from the session-5 ledger (`.superpowers/sdd/2026-10-18-session-5/progress.md`).

- **Q1. The Microsoft 365 guided Backfill copy** (minor #2 and handoff §6's copy list). It is shown
  when the pass ends in a named `not found`, and it must not ask for a forward as an attachment.
  - **Recommend** no hand-forwarding at all: "Knowlu couldn't bring in older Outlook mail this time.
    New mail is already arriving. Knowlu will try again on its next run," with **Try again**.
  - Quinn words it, along with minor #4's three lines: Settings' "Resume carries on", the per-launch
    "set up by hand", and "setup stopped" though forwarding worked.
  - Needed before R11.
  - **Ruled 2026-10-19 (Quinn): as recommended.** The copy after a named `not found` is "Knowlu
    couldn't bring in older Outlook mail this time. New mail is already arriving. Knowlu will try again
    on its next run." with a **Try again** button. No hand-forwarding.
  - **Minor #4's three lines, ruled 2026-10-19 (Quinn), "until C3 lands":** (1) Settings, for an M365
    backfill, instead of "Still bringing in recent mail; Resume carries on": "New mail is arriving.
    Knowlu will bring in older Outlook mail on a later run."; (2) an M365 setup that forwarded but
    ended "setup stopped": "Forwarding is set. New mail is arriving."; (3) the per-launch resume of an
    old M365 marker that ends failed "set up by hand" shows nothing, and Knowlu stops trying to resume
    an M365 backfill it cannot run yet. They are built ahead of C3 on the email branch (`console-ui`
    (1), `implementer` (2) and (3)).
- **Q2. The VISION diff and merge authority.** D18's wording is signed and lands in R12a. The handoff's
  §1.7 still asks Quinn before merging "any rule/VISION change".
  - **Recommend:** treat D18 as the text's approval, and ask Quinn once, at the PR, for the merge.
  - **Ruled 2026-10-19 (Quinn): as recommended.** D18's signed wording is the approval of the VISION
    text; the controller asks Quinn once, at the C3 PR, before merging it. No separate pre-PR VISION
    diff review.
- **Q3. `site/privacy.html` after L4.** L4 rewrites the page now to say what the app does today, and
  today Microsoft 365's backfill is guided. Once C3 merges, the page must name D18's upload row and the
  mailbox-actions sentence too, or it stops being honest.
  - **Recommend:** R12b adds those two to the page after L4 merges, using D18's signed words, and
    `PRIVACY_VERSION` stays put (D18; nothing new leaves the device). A reviewer reads the text and
    Quinn reads the page.
  - If Quinn wants testers to re-accept, the bump is L4's file (`account.rs`) and is handed to L4's
    owner.
  - **Ruled 2026-10-19 (Quinn): as recommended.** After L4 merges, R12b adds D18's upload row and the
    mailbox-actions sentence to `site/privacy.html` in D18's signed words; `PRIVACY_VERSION` stays put
    (no tester re-accept); a reviewer reads the text and Quinn reads the page.
- **Q4. School forwarding until the re-proof.** The re-proof needs the kept M365 session and an
  `active` mailbox on staging. T18c's cleanup may turn school forwarding off.
  - **Recommend:** keep it on until L1's re-proof, or until the slip rule fires. Otherwise the re-proof
    starts with a fresh setup.
  - **Ruled 2026-10-19 (Quinn): as recommended.** School (M365) forwarding stays on until C3's
    re-proof, or until the slip rule fires; T18c's cleanup must not turn it off or remove the kept M365
    session. (This overrides the session handoff's "restore school forwarding to OFF in T18c cleanup".)
- **Q5. Staging model spend in the re-proof.** A full 120-day backfill (about 1,100 originals) runs
  through `mail-process` on staging, which spends OpenRouter credit up to the backfill model allowance.
  - **Recommend:** yes, once, in the re-proof block.
  - Alternative: cap the proof at a recover slot plus a narrowed backfill (W9 makes about 1,100
    originals the upper bound).
  - **Ruled 2026-10-19 (Quinn): yes, once.** One full 120-day M365 backfill (about 1,100 originals)
    through staging `mail-process` in the re-proof block, spending staging OpenRouter credit up to the
    backfill model allowance.
- **Q6. The note's header.** It still reads "DRAFT v2 for Quinn" though every decision is ruled.
  - **Recommend:** R12 records the rulings by date in the header ("decisions ruled 2026-10-11/13/18;
    live gate closed 2026-10-18"), with no change to any decision.
  - **Ruled 2026-10-19 (Quinn): as recommended.** R12 replaces the note's "DRAFT v2 for Quinn"
    header with "decisions ruled 2026-10-11/13/18; live gate closed 2026-10-18"; no decision changes.
- **Q7. The lawyer-packet splice.** D18 signs "reading through Outlook's own services with the saved
  sign-in; no mail is sent" as the automated-access line, but Gmail still sends.
  - **Recommend:** keep the line as one item and scope the signed words to Microsoft 365: "the mailbox
    actions done with a saved session (for Microsoft 365, reading through Outlook's own services with
    the saved sign-in; no mail is sent; Gmail's forward-and-delete as before) ...".
  - Quinn confirms the splice. The packet is a before-Pilot item and does not block the merge.
  - **Ruled 2026-10-19 (Quinn): as recommended.** The lawyer packet keeps one automated-access item,
    scoped by provider, in the words above. Before-Pilot; blocks no merge.
- **Q8. Does C1 wait for L3's, L8's and L5's migrations?** §5 freezes L1's migration name at C1's
  staging push. If one of those lanes merges after C1 but before L1's PR, `main` holds a migration that
  sorts after L1's yet was applied first. Staging then needs `db push --include-all` (staging is the
  controller's), and so does the prod deploy (Quinn's).
  - **Recommend:** C1 waits until those three lanes have merged, or until the controller records that
    one of them has slipped past L1. C1 sits at the close in any case, so the wait is usually free.
  - Alternative: run C1 when L1 is ready and accept `--include-all` at the prod deploy. Quinn hears
    which lane's migration it was.
  - **Ruled 2026-10-19 (Quinn): as recommended.** C1 waits until L3, L8 and L5 have merged, or until
    the controller records that one slipped past C3; the prod deploy stays a plain push.

## 5. Lane mechanics and the conflict map (handoff §5)

- **Branch and base.** The controller cuts `c3-m365-backfill` from `main` once the email merge has
  landed and CI on `main` is green (the ingest time bomb is fixed by that merge). It records the sha in
  the lane ledger. The controller commits; agents never commit, push or branch.
- **Files this lane owns while it runs:**
  - `app/src/mail/upload.rs` (new), `app/src/mail/{recover,backfill,slot,runner,setup}.rs`,
    `app/src/mail.rs` (script list and `pub mod upload;` only), `app/src/mail_client.rs`,
    `app/src/mail_steps.rs`;
  - `app/assets/mail/**`;
  - `app/tests/{mail,mail_steps,mail_window,mail_client,mail_upload,mail_progress}.rs` (the last three
    new) and `app/tests/support/{slot_rig,owa_fake}.rs` (`owa_fake.rs` new);
  - `cloud/supabase/functions/{mail,mail-inbound}/**` and `_shared/mail_handoff.ts` (new);
  - `_shared/testdata/message_key_vectors.json` (new);
  - one migration, `<stamp>_mail_upload.sql`.

  `cloud/supabase/migrations/migrations_test.ts` is **not** this lane's alone. Handoff §5 serialises it
  with the migrations (email → L3 → L8 → L5 → L1), so it is listed under the shared files below. The
  root `cloud/supabase/migrations_test.ts` is C1's suite (it reads only `20260910` files) and is not
  touched.

  `app/src/mail/commands.rs` is **not** `app/src/commands.rs`: the first is this lane's (P10's fields),
  the second is L6's.
- **Shared files, and who goes second:**
  - **`cloud/supabase/migrations/`.** L1's stamp follows **the shared migration-stamp rule, M1-M8**
    (written 2026-10-19 after Q8's ruling; the same text is in L3's plan,
    `docs/plans/2026-10-18-first-day-allowance-plan.md` §2.1, and L8's spec,
    `docs/specs/2026-10-18-friends-testers-design.md` §8.4). Where this section and the rule differ,
    the rule wins and the controller aligns this section. In brief:
    - **Order and format (M1, M2).** Stamps sort in merge order: email → L3 → L8 → L5 → L1. A stamp is
      the date it is fixed plus a step counter (`000100`, `000200`, …), never a wall-clock time. **The
      next free stamp after X** is today's date with `000100` if that sorts after X, otherwise X plus
      `000100`.
    - **R6 picks a provisional stamp (M3).** Before R6 creates `<stamp>_mail_upload.sql`, the lane
      ledger records the next free stamp after the greatest of three sets: every migration on `main`;
      every stamp L3, L8 and L5 have recorded (the controller passes them at dispatch); and every
      version staging has applied (the controller reads `supabase migration list` on staging; no agent
      touches staging). It stays provisional until L3, L8 and L5 have all recorded theirs. Its tests
      find the file by `^\d{14}_mail_upload\.sql$`, never by its stamp (M4), so a re-stamp changes no
      test line.
    - **The one re-stamp, before C1 only (M5, M7).** C1 waits until L3, L8 and L5 have merged, or the
      controller has recorded that one slipped past C3 (Q8), **and**, by M7, until each merged one's
      migration is on staging. By M7 a lane ahead is cleared only once its migration is on staging
      **and** on `main`, and a recorded slip applies only to a lane that has **neither merged nor
      pushed**. A merged, unpushed lane holds C1 because its file is on `main` and would ride into C1's
      push; by M7 a lane's migration merges only once its own push is allowed, and the controller
      pushes it right after the merge (L3's merge waits for Q-E as well as Q-G), so that wait lasts
      only between a merge and its push. A lane that pushed from its lane branch and has not merged
      (L8's S7a always does, and L3's T5 does if Quinn asks for the proof first) holds C1 until it
      merges or the controller withdraws it (M7), because C1's dry run refuses while staging holds a
      version `main` lacks; while its version is on staging it cannot be recorded as slipped. A
      withdrawn lane counts as neither merged nor pushed, so it may then be recorded as slipped. Both
      conditions are stricter than Q8's words, which name merges only and leave the slip record to
      the controller; Quinn hears them at the next checkpoint, including that Quinn's go on L8's merge
      also releases C1. Then,
      immediately before C1's staging push, the integrator (R13) rebases on `main` and checks L1's
      stamp again against the same three sets: `main`, the stamps L3, L8 and L5 recorded (including a
      slipped lane's, which has neither merged nor pushed), and staging's applied versions, which the
      controller reads and hands to R13.
      If anything sorts at or after L1's, R13 renames L1's file with `git mv` to the next free stamp
      after the greatest of the three (same suffix), recounts the pins by the `migrations_test.ts` rule
      below and re-runs `deno test`. A dated ledger row records both stamps. That is the last free
      rename.
    - **C1's dry run (M7).** Before C1's push, the controller runs `supabase db push --dry-run` from
      the rebased lane branch. If it refuses, or its list holds any version other than L1's own, C1
      stops and does not push; the controller records the version. So no other lane's migration
      reaches staging inside C1's push. A refusal naming a remote version with no local file means a
      lane ahead pushed before merging, and the session ledger names it: C1 waits for that lane's
      merge or its withdrawal (M7), R13 rebases, and the dry run runs again. A remote version no lane
      recorded is not waited on: the controller stops and asks Quinn. Nobody runs
      `supabase migration repair` to get past a refusal (it runs only for M6's rename of a frozen file
      and in M7's withdrawal, after the lane's objects are dropped); the CLI's `--status reverted`
      hint, with the objects still there, would let C1 push, and that lane's later push from `main`
      would re-run its SQL on staging. L1 does not merge until a dry run on its rebased lane branch
      lists nothing but L1's own version (nothing at all once C1 has pushed).
    - **After C1's push the name is frozen (M6).** Staging's `supabase_migrations` history now holds
      that version, and `supabase db push` refuses to run while a remote version has no local file. So no
      rename happens before the PR, and none after, unless the controller withdraws L1 (M7; §8's C1
      withdrawal step): then the name is unfrozen, and R13's re-check runs again before the next C1.
      From C1 until L1's merge, L1 is itself a lane that pushed before merging, so a slipped lane
      that re-lands meanwhile waits for L1's merge or withdrawal. A lane that lands later takes a later stamp itself
      (M8): a lane that has neither merged nor pushed (only such a lane can be recorded as slipped,
      M7) re-checks by M5 on its lane branch before its merge, so it sorts after L1's frozen stamp. A
      lane that pushed before merging cannot be overtaken while its version is on staging (M8): C1
      waits for its merge, or for its withdrawal, after which it is a lane that has neither merged
      nor pushed.
    - **No push uses `--include-all`** (M7, M8; Q8: the prod deploy stays a plain push). If a staging
      push asks for it anyway, the rule was broken somewhere: the controller stops and records which
      version sorts out of order. One remedy per case:
      - **Not merged and not pushed:** not frozen; its own lane rebases and re-stamps it by M5.
      - **Merged but not pushed** (only if M7's merge-with-push rule was broken): not frozen (staging
        never applied it), and it has no lane branch to rebase. The controller re-stamps it on `main`
        by M8 before its first push: one commit with the `git mv` to the next free stamp after M3's
        three sets, the pins recounted and both stamps in the session and lane ledgers, landed as a
        one-commit PR on green CI; then a plain `db push` from `main`. No `migration repair` runs (M6
        is only for a version staging holds). The re-stamp fixes the order only: the file stays on
        `main`, so M7's dry run holds every push from a branch carrying it, C1's included, until its
        own push is allowed and done. If a lane that pushed before merging sorts ahead of it, the
        re-stamp cannot help (that lane's merge dry run lists the stray file, and the stray file's
        push refuses on that lane's version). The controller pushes nothing from `main` and offers
        Quinn M8's two remedies, the choice being Quinn's: a one-commit revert of the broken merge on
        `main`; or a withdrawal of the pushed lane ahead (M7), which rewrites nothing on `main`: the
        controller records that lane's slip, pushes the stray migration from `main` once its own push
        is allowed, and the withdrawn lane re-stamps by M5 and pushes again, re-running its proof.
      - **Already pushed:** frozen; it stays. Prod is protected by M8's check before each prod push: a
        migration on `main` that sorts before prod's newest applied version is re-stamped by M6 before
        the push, or Quinn is asked.

      The PR notes no flag for the prod deploy.
    - **A rename after C1 that cannot be avoided** (for example, CI's name-format test refuses the stamp,
      or M8's prod check) is the controller's alone, done by M6 as one step with both stamps and the
      repair in the ledger: `supabase migration repair --status reverted <old stamp>` on staging, then
      the `git mv`, then `supabase migration repair --status applied <new stamp>` (M6's form; the SQL is
      not re-run). A plain `db push` in place of the last step re-runs the file, which P11 makes safe for
      this migration only. No agent renames the file after C1.
  - **`cloud/supabase/migrations/migrations_test.ts`.** It is shared in the same order as the
    migrations. L1 changes three things in it (its own block, the count and the comment), and two
    rules govern the shared line:
    - **Its own block.** R6's tests and the `mailUploadMigration()` and `mailUploadFunction()` helpers
      are one block. The block is appended after the mail stream's last test under its own comment
      heading (`// C3 (Microsoft 365 upload) …`). If another lane appended a block at the same place,
      the rebase keeps both, with L1's last.
    - **The pinned function count.** The execute-revoke guard ("every SECURITY DEFINER or writing
      function in every migration has execute revoked …") ends with `assertEquals(parsed, N, …)`. That
      count is the number of function creations parsed across every migration file, so each lane that
      adds a function moves it. R6 sets it to **the N it finds on its branch plus three**, for the
      charge RPC, the `received` compare-and-set and the upload hand-off. On today's corpus that is 40
      to 43.
    - **The comment above that count.** R6 adds one clause at the end, naming the three by kind:
      writing, `security invoker`, and revoked from `public, anon, authenticated` in
      `_mail_upload.sql`. The clause names the file by its suffix, never by its stamp, so a re-stamp
      changes no test line.
    - **The view count does not move.** The view guard's `assertEquals(parsed, 5, …)` stays at 5,
      because R6 adds no view.
    - **The rule on a conflict (R13).** L3, L8 and L5 each add functions, so if any of them merges first
      the rebase conflicts on that line. The integrator takes `main`'s number and `main`'s comment, adds
      three, and puts L1's clause back at the end of the comment. It never keeps either side's number as
      it stands. `deno test` is the check: a wrong sum fails the guard by name. The ledger records
      `main`'s number and L1's result. A lane that merges after L1 does the same sum over L1's number.
  - **`app/static/console.js`.** One `console-ui` queue: L4 re-consent → L7 → **L1 R11** → L5. R11
    starts only once L4 and L7 have merged, or after the controller's word if either slips.
  - **`app/src/main.rs` and `app/src/commands.rs`.** L6 owns them, and P10 means none of their lines
    change here. Any line that does become needed goes to L6's owner.
  - **`app/src/mail_session.rs`.** P5 adds one field. L4's sign-out work (`pending_setups`) may touch
    the same file, so R9d rebases on L4 if L4 merged first and keeps its edit to the one field and its
    reader.
  - **`app/tests/scheduler.rs`.** R9c3 changes one M365 test (the setup in
    `no_file_or_log_of_knowlus_own_holds_the_address_or_token`), R9c5 changes two, and R10 none (§6,
    "Tests the routing retires or rewrites"). L6 edits the judge/runtime tests in the same file, so
    whichever lands second rebases. No test is weakened and none is ignored. Every changed test is
    named in that table with its fate.
  - **`app/tests/support/slot_rig.rs`.** It is this lane's while it runs, but `app/tests/account.rs`
    (L4's test file) and `scheduler.rs` include it by `#[path]`. R9c2 and R10 change it add-only or
    M365-only, keep `--test account` green, and never edit `app/tests/account.rs`. If L4 changes the
    rig first, whichever lands second rebases.
  - **`account.rs`** (L4, contract list) and **`approvals.rs`** (L5): not touched. `account.rs` names
    `m365/forwarding_off.js`, which this lane keeps.
  - **`HANDOFF.md`**: the Docs lane's. R12 hands its lines (lane state, risks 13/15 closed for M365) to
    `docs-keeper` in the Docs lane rather than editing it here.
  - **`docs/reference/app.md`**: also the Docs lane's (handoff §5 lists `docs/reference`). R12b edits
    only the mail sections it names, each found by heading. Whichever of R12b and the Docs lane's
    reference refresh lands second rebases onto the first and keeps both edits. If the Docs lane is
    still open when R12b runs, the controller tells its owner which headings R12b touches.
- **Builds.** At most two cargo runs machine-wide (handoff §5). Rust tasks run tests in the foreground,
  `cargo test -p knowlu --test <file>` while working and the workspace at the gates. Deno tasks run
  `deno check`, `deno lint` and `deno test` with each tree's own config, the same command lines as
  `ci.yml`. Those are `cloud/supabase/deno.json` for `cloud/supabase/` and `app/assets/mail/deno.json`
  for `app/assets/mail/`.
- **Long documents** (R0, R12) are written in pieces of at most 150 lines (ledger lesson of 09-17).
- **Live work** (R0's report, R1's capture if needed, C1, the re-proof) is the controller's alone.
  Every live check uses the kept scratch proof profile, stays structure-only and is never OS input.
  Subagents never hold a token, and the probe kit is copied from session 4's scratchpad into this
  session's.

## 6. Tasks

Each task names its agent (CLAUDE.md's table) and why, its files, what must land first, its tests (red
first), the behaviour, and when it is done. Test names are the note's §4.8 and §4.11 names where they
exist. Briefs cite headings and function names, never line numbers.

**Sizing.** No task asks one dispatch for more than about 80 changed lines of product code, or for more
than about a dozen tests written or rewritten. That is the stall pattern the ledger recorded on 09-17.
Where the note's task was bigger, it is split along a seam a reviewer can read on its own, and every
part names the same review. A rewrite whose only change is a comment does not count; those go to R10.

### R0. The live-check report (main session)

**Why the main session:** the evidence is live and in the controller's ledger. **Files:**
`docs/reports/2026-10-18-c3-live-checks.md` (new); the lane ledger (created).
- It writes up W1 and W4 to W10 from the ledger rows of 2026-10-13 and 10-18. It records counts,
  booleans, statuses, key paths and the three W9 numbers only.
- It states the build gate as closed: W1, W4, W5, W6 and W8 pass.
- It records that decision 19 is void (W1) and that 17b is adopted (W8, §3 P6).
- No value from a real message, and no tenant or address name (rule 1).
- **Done when** the report is committed docs-only and the lane ledger's first row cites it.

### R1. The recorded structure (main session)

**Why the main session:** it is written straight from live output. **Files:**
`app/assets/mail/m365/testdata/owa_shapes.json` (new; synthetic values only).
- For each of the five actions, it records the request body's key paths and the answer's key paths,
  types and enum values. That includes `ResponseCode`, `ResponseClass`, `RootFolder.{TotalItemsInView,
  IndexedPagingOffset, IncludesLastItemInRange}`, `Items[].{ItemId.Id, ParentFolderId.Id,
  InternetMessageId, DateTimeReceived, Size, ItemClass, From.Mailbox.{EmailAddress, RoutingType}}`,
  `MimeContent.Value`, `SessionSettings.UserEmailAddress`, the folder fields `FindFolder` answers,
  and the throttling shapes (`ErrorServerBusy`, `BackOffMilliseconds`).
- **Source.** First, the probe kit's saved outputs (`w469.mjs`, `w578.mjs`, `wfind.mjs`, `w8shape.mjs`).
  If they lack a key path, the controller runs one short structure-only capture under D11's scope: the
  hidden app window, the harness copy of the closure, keys and types only.
- Every value in the file is invented: item ids like `synthetic-item-1`, folder ids like
  `synthetic-folder-inbox`, addresses at `example.edu`. No synthetic id begins `AAMk` or `AQMk` (real
  Exchange ids do), so R3a's pattern never has to tell a synthetic id from a real one.
- **Done when** the file parses, names no real value (a reviewer checks by eye, and R3a's
  `owa_shapes_holds_no_real_looking_value` test checks the patterns), and R3b's tests load it.

### R3a. The `init.js` tests, red (`test-writer`)

**Why `test-writer`:** §4.11's failing tests, ahead of R2. **Files:** `app/assets/mail/init_test.ts`
(new). **Before:** none. **Tests**, each red against today's `init.js`:
- `init_captures_only_on_the_two_outlook_hosts_top_frame_and_only_from_same_origin_service_svc`
- `init_wrapper_is_inert_off_the_mail_hosts_and_in_subframes` (and on `/owa/` paths that are not
  `service.svc`)
- `init_never_reads_cookies` (a `document.cookie` getter trap)
- `init_read_refuses_every_action_off_the_allow_list` (a fixed word, and the fake `fetch` records no
  request). The list it pins is exactly the five network actions and the three local verbs (P1).
- `init_read_sends_the_body_in_x_owa_urlpostdata_with_a_fresh_correlation_id`
- `no_answer_error_or_property_of_the_closure_carries_a_captured_header` (it enumerates every reachable
  property and every answer for the captured bearer's sentinel value)
- `the_read_function_is_not_enumerable_not_writable_and_not_configurable`
- `get_item_with_an_update_shaped_body_is_refused`
- `the_mime_slot_holds_one_original_clears_on_the_last_chunk_and_after_five_minutes`
- `freeze_and_resume_are_counted_and_was_discarded_is_reported`
- `owa_shapes_holds_no_real_looking_value`. It reads every string in R1's file and fails on any of:
  - an address whose domain is not `example.*`;
  - a string beginning `AAMk` or `AQMk` (Exchange's item and folder id prefixes);
  - a run of 40 or more characters from the base64 or base64url alphabet (`[A-Za-z0-9+/=_-]{40,}`),
    which catches an Exchange id, a change key or a token under any prefix.
  - The one exemption is the value at `MimeContent.Value`, which is base64 by nature. The test decodes
    it instead and applies the address rule to the decoded message.
  - It also self-tests: a sample `AAMkAGI2...` id of 60 characters is flagged, and `synthetic-item-1`
    and `synthetic-folder-inbox` are not.

The fake `fetch` and `XMLHttpRequest` serve answers built from `owa_shapes.json`. Until R1 lands, a stub
with the same key paths stands in. **Which half turns each green:** the slot test and the freeze test
are R2b's; every other test is R2a's. **Done when** every test is red for the right reason (`deno test`
shows the failures) and `deno check` and `deno lint` are clean.

### R2a and R2b. The read closure (`contract-engineer`)

**Why `contract-engineer` (Opus, xhigh):** the file is not on the contract list, but CLAUDE.md's "no
agent fits?" applies. A silent error here leaks OWA's bearer or student mail (note §7, risk 2).
**Review:** `contract-reviewer`, once, after R2b (§6 Review R2). **Files:** `app/assets/mail/init.js`
only, in two dispatches:
- **R2a: capture, the allow-list and the verb dispatch.** It covers "What `init.js` keeps", "Capture",
  "Off those hosts", the network actions and `#state`'s `closure_ready`, and "Answers" below. `#mime`,
  `#chunk` and the counters are in the pinned verb list but answer a fixed `not built` word until R2b.
  **Before:** R3a red. **Done when** R3a's R2a tests are green.
- **R2b: the MIME slot and the lifecycle counters.** It covers `#mime`, `#chunk`, the slot's clearing
  and `#state`'s `frozen_count` and `discarded`. **Before:** R2a. **Done when** below.

**Behaviour** (note §4.2, §4.3 items 2 and 3, §4.4):
- **What `init.js` keeps.** Today's `registerProtocolHandler` no-op stays exactly as it is. It is the
  wrapper's first block.
- **Capture.** On the top frame of `outlook.office.com` or `outlook.cloud.microsoft` only, `fetch` and
  `XMLHttpRequest.prototype.setRequestHeader` are wrapped before any page script runs. The capture
  reads only OWA's own same-origin `service.svc` requests, and only the P3 header set (`authorization`,
  `x-anchormailbox`, `x-owa-sessionid`, `x-tenantid`, `x-client-version`, `x-req-source`). It keeps the
  newest values in closure variables. It never reads cookies.
- **Off those hosts.** In a subframe, and on any other path, the wrapper is not installed: the
  originals stay untouched.
- **The verbs.** One global function, as in P1:
  - Network actions go to the read allow-list.
  - `#state` returns `{visibility, frozen_count, discarded, closure_ready}`. `closure_ready` means the
    full header set has been captured at least once.
  - `#mime {id}` runs `GetItem` with `IncludeMimeContent` and checks the answer's folder id and
    `InternetMessageId`. It parks the base64 under a random handle and returns `{handle, b64_len,
    sha256, mid, folder_id}`, with `sha256` computed over the decoded bytes with `crypto.subtle`.
  - `#chunk {handle, index}` returns base64 slice `index` (at most 1 MiB), and the last one clears the
    slot.
  - A new `#mime` clears the old slot. A slot unread for 5 minutes is cleared on the next call.
- **Answers.** They are parsed JSON or a fixed word. They never include a header, and nothing is logged.
- **Hard rule 1 holds here:** no attachment action or attachment id is ever accepted.
- **Done when** R3a is green, the existing `app/assets/mail` tests stay green, `deno check` and
  `deno lint` are clean, and Review R2 passes.

### R3b. The listing-script tests and the fake OWA service, red (`test-writer`)

**Why `test-writer`:** §4.11. **Files:** `app/assets/mail/m365_read_fake.ts` (new; a fake OWA service
built from `owa_shapes.json`, routing by `action`) and `app/assets/mail/m365_read_test.ts` (new).
**Before:** R1, or a stub of it. **Tests** run each script through the real `init.js` (after R2a;
before it, through a stub of P1's function with the same contract):
- `m365_alive_reports_state_and_closure_ready`
- `m365_whoami_answers_only_a_boolean_and_compares_case_folded`
- `m365_folders_keeps_ipf_note_and_drops_the_seven_excluded_subtrees_by_distinguished_id` (and never
  returns a display name)
- `m365_count_sums_allowed_folders_only`
- `m365_items_asks_by_received_time_slices_and_splits_a_full_slice` (7-day start, split above 200 rows,
  or when `TotalItemsInView` exceeds the rows returned). Slices are half-open `[since, until)` and
  disjoint, so no item is listed twice. This replaces the retired span test's "judged once" guarantee.
- `m365_items_sends_paging_in_body_paging_as_indexed_page_view` (W4)
- `m365_items_never_lists_an_excluded_folder`
- `m365_items_drops_drafts_reports_other_own_mail_and_the_canary_sender_counting_them_skipped` (the
  own address and the canary sender are passed in as arguments, never literals)
- `m365_a_short_list_is_not_found_never_a_partial_pass` (the per-folder total check)

**Done when** they are red for the right reason, and check and lint are clean.

### R3d. The MIME-script and cross-cutting tests, red (`test-writer`)

**Why `test-writer`:** §4.11, the MIME half. **Files:** `app/assets/mail/m365_mime_test.ts` (new; it
imports R3b's fake). **Before:** R3b. **Tests:**
- `m365_mime_leaves_an_item_moved_to_deleted_items_since_the_list`
- `m365_a_message_id_that_changed_is_left_never_uploaded`
- `m365_mime_chunk_returns_one_mib_slices_and_the_last_clears_the_slot`
- `m365_a_shape_the_recording_lacks_is_not_found_with_a_control_name` (validation, every script)
- `m365_a_throttling_answer_is_reported_as_throttled` (429, 503, `ErrorServerBusy`,
  `BackOffMilliseconds`; every script)
- `m365_scripts_never_read_an_attachment` (hard rule 1)
- `no_m365_script_result_carries_a_bearer_or_jwt_shape`
- `no_m365_script_chains_page_timers` (a source check: no `setTimeout` or `setInterval` loop in `m365/*`)

**Done when** they are red for the right reason, and check and lint are clean.

### R3c. The tripwires (`test-writer`)

**Why `test-writer`:** source-level failing tests. **Files:** `app/assets/mail/m365_tripwire_test.ts`
(new) and `app/tests/mail_window.rs` (test 34 widened; the T10.2b script test is left to R10).
**Tests:**
- `m365_scripts_and_init_name_no_write_action`. It scopes to the read scripts and `init.js`;
  `m365/forwarding_*.js` (setup's own, unchanged DOM scripts) is an explicit, commented exception.
  - **What it reads:** string literals only (the contents of single-, double- and backtick-quoted
    strings, comments stripped first). Identifiers and property names are not scanned, so `init.js`'s
    own wrappers of `XMLHttpRequest.prototype.setRequestHeader` and `send` never match.
  - **What it matches, case-sensitively:** a literal equal to one of `CreateItem`, `SendItem`,
    `UpdateItem`, `DeleteItem`, `MoveItem`, `CopyItem`, `MarkAllItemsAsRead`, `EmptyFolder`,
    `UpdateFolder`, `CreateFolder`, `UpdateInboxRules` or `SetMailboxForwarding`, or any literal that
    matches `^(Set|Create|Update|Delete|Move|Send)[A-Z]`.
  - **It proves its own reach:** a self-test runs the scanner over an inline sample and expects it to
    flag `"UpdateItem"` and `'SendFoo'`, and not flag `xhr.send(`, `setRequestHeader`, `"send"` or
    `"settings"`.
- `no_m365_script_names_a_knowlu_host` (the note's name). Its scope is widened from `m365/*` to all
  of `app/assets/mail/**`, and it looks for `knowlu.com`, `supabase.co` and `/functions/v1`.
- Test 34 widened in `mail_window.rs`: no `Input.`, `CallDevToolsProtocolMethod`, `SendInput`,
  `setWebLifecycleState` or `remote-debugging` in `app/src/mail*` or `app/assets/mail/**`. `debug_port.rs`
  stays outside `app/src/mail*`. If the scan would reach it, the test names it as the one harness file
  and asserts it is `#[cfg(debug_assertions)]`-gated.
- `no_m365_code_path_writes_an_original_to_disk` (a source test: no `std::fs` and no `File` in
  `app/src/mail/upload.rs`, and `run_m365_pass` takes no path argument).

**Done when** the new tests are red only where R4 or R9a code is still missing, and green where they
already hold today. The write-action self-test is green at once.

### R4. The step scripts (`implementer`)

**Why `implementer` (Sonnet, high):** specified, and checked by R3b and R3d. **Review:** `contract-reviewer`
for `mime.js` and `mime_chunk.js` (the MIME path, note §4.9), `reviewer` for the rest. **Files:**
`app/assets/mail/m365/{alive,whoami,folders,count,items,mime,mime_chunk}.js` (new) and `app/src/mail.rs`
(the seven added to the script list; nothing removed yet). **Before:** R1, R2b, R3b, R3d. **Behaviour:** note
§4.1 steps 1 to 5 and 7, and §4.2's paging and waits.
- Each script makes a few calls and returns. Every wait between calls is Rust's.
- `items.js` returns rows `{id, folder, received_at, sender, subject, mid, size, kind}` and the
  per-folder `skipped` count.
- `mime.js` and `mime_chunk.js` are thin callers of `#mime` and `#chunk`. They add the folder-allowed
  re-check and the Message-ID equality, and their answers carry no bytes beyond one slice.
- **Done when** R3b, R3d and R3c's script tripwires are green, every older `app/assets/mail` test is green,
  `cargo build -p knowlu` is clean (the `include_str!` list), and both reviews pass.

### R5. The shared Message-ID vectors (`test-writer`)

**Why `test-writer`:** one fixture that both sides read. **Files:**
`cloud/supabase/functions/_shared/testdata/message_key_vectors.json` (new) and
`cloud/supabase/functions/_shared/message_key_vectors_test.ts` (new). **Before:** none.
- Each vector has a header value, the expected `messageKey` and the expected
  `HMAC_k(JSON.stringify(["msg", key]))` hex under a fixed test key.
- Vectors: angled, unangled, folded, with comments, a `[`-led id (refused, so the Date, From and
  Subject form applies), absent, two ids (the first one wins), non-ASCII, and an embedded control
  character.
- For the `[`-led and absent vectors, the expected **device** answer is `null`. The device forms only
  the Message-ID form; it falls back to the fingerprint rule and leaves the row to the server's dedup
  (note §4.6). Each vector carries both `server_key` and `device_key`.
- The Deno test reads the file and checks `messageKey` in `cloud/supabase/functions/_shared/mime.ts`
  and the hash in `cloud/supabase/functions/mail-inbound/keys.ts` (`seenHashes`, `hmacHex`). It is
  green at once (it pins today's behaviour). R9a's Rust test reads the same file.
- **One hash form, imported, never copied.** `keys.ts` stays where it is. The `mail` function already
  imports it across directories (`mail/index.ts` imports `hmacKeyFromBase64`, `nonceHmac` and
  `tokenHmac` from `../mail-inbound/keys.ts`). R7c2's route imports `seenHashes` the same way. No task
  copies the HMAC form into `mail/`, and none moves `keys.ts` into `_shared/` (that would be a rewrite of
  intake's imports for no gain).
- **Done when** it is green, and the file holds only synthetic ids.

### R6. The migration (`cloud-engineer`)

**Why `cloud-engineer` (Opus, high):** `cloud/`. **Files:**
`cloud/supabase/migrations/<stamp>_mail_upload.sql` (new; provisional stamp, §5) and
`cloud/supabase/migrations/migrations_test.ts`, which is shared (§5): L1's own block, plus the
execute-revoke guard's pinned count and its comment. The root `cloud/supabase/migrations_test.ts` is
C1's and is not touched. **Before:** none.
**Tests first.** They use two new helpers. `mailUploadMigration()` finds the one file matching
`^\d{14}_mail_upload\.sql$` (the way `mailMigration()` finds `_mail.sql`) and never names a stamp.
`mailUploadFunction(fn)` is `mailFunction`'s shape over that file:
- `the_three_upload_rpcs_are_security_invoker_and_service_role_only`. For each of the charge RPC, the
  `received` compare-and-set and the upload hand-off, it checks:
  - the header has no `security definer`;
  - `set search_path = public, extensions` is pinned;
  - `assertServiceRoleOnly`'s two lines are present: `revoke execute … from public, anon,
    authenticated;` and `grant execute … to service_role;`.
- `mail_batches_via_is_send_or_upload_default_send`
- `the_upload_day_counter_table_has_rls_on_and_no_client_policy` (service role only, like every
  `mail_*` table)
- `the_upload_charge_rpc_is_compare_and_set_and_refuses_over_either_cap`. It reads the RPC's body and
  checks P7's form. The body holds exactly one statement that writes the counter table. That statement
  is an `insert ... on conflict ... do update ... where ... returning` whose `where` names both caps,
  or an `update ... where ... returning` of that shape. It holds no `select` from the counter table
  and no `for update`. A text test cannot prove atomicity on Postgres; C1.8 does that (§8).
- `the_received_compare_and_set_is_one_guarded_update` (the same check for `received < expected`)
- `the_upload_hand_off_guards_received_before_it_writes_a_row` (P7). It reads the upload hand-off RPC's
  body. The first statement that writes is the guarded `update` of `mail_batches` (`received <
  expected`, open, `returning`). It holds no `select` from `mail_batches` and no `for update`. The
  `mail_seen` and `mail_work` inserts come after it, the `mail_work` insert runs only when the
  `mail_seen` insert took, and a guard that matched no row answers `full` before any insert. Being one
  function, it is one transaction. C1.8 checks the behaviour on Postgres. Only the mailbox check of the
  next test comes before the guard.
- `the_upload_hand_off_checks_its_mailbox_for_share_first` (Checkpoint B). It reads the same body. Its
  first statement is `mail_hand_off`'s mailbox check, `perform 1 from public.mail_addresses where id =
  p_address and account_id = p_account and status <> 'revoked' for share`. It is compared,
  whitespace-normalised, with that statement read out of `mailFunction("mail_hand_off")`'s body, never
  with a copy. The next statement is `if not found`, answering `gone`, and both come before the batch's
  guard and either insert.
  - **Why.** `purgeAddress` (`functions/mail/handler.ts`) revokes the mailbox, reads its `work/` keys,
    deletes those objects, then runs `delete_mail_address`. The share lock makes the revoke wait for a
    hand-off already past the check, so the key read sees that hand-off's row. A hand-off that runs
    after the revoke writes nothing. The foreign keys from `mail_seen` and `mail_work` cannot do this,
    because their `key share` lock does not conflict with the revoke's `update` of `status`.
  - **No C1 row.** On Postgres the check behaves as `mail_hand_off`'s own statement does. A staging
    Disconnect would also purge the proof profile's mailbox before the re-proof.
- `the_mail_upload_migration_leaves_mail_hand_off_alone`. L1's file never redefines intake's
  `mail_hand_off`, so intake's hand-off stays exactly as it is. It also redefines no other function
  that `_mail.sql` defines.
- `the_upload_counters_go_with_their_mailbox` (cascade on `mail_addresses` delete, so Disconnect and
  account deletion purge them)
- `the_mail_upload_migration_is_idempotent` (P11's forms only)
- `the_mail_upload_migration_is_forward_only`. It applies the same three words as "no migration in this
  stream drops or truncates a table" (`drop table`, `drop column`, `truncate`, word-bounded) to L1's file.
  That existing test reads only `20260911` files through `ours()`, so it never sees L1's migration.
  P11's `drop policy if exists` and `drop function if exists` are not on the list and stay allowed.
- **The tests that read every file** through `everyMigrationFile()` see L1's file. Exactly one of them
  is edited:
  - **Edited: the execute-revoke guard** ("every SECURITY DEFINER or writing function in every
    migration has execute revoked …"). Its pinned count becomes the N R6 finds plus three (40 to 43 on
    today's corpus), and its comment gains L1's clause (§5). Without that edit it fails on "today's
    corpus should parse exactly 40 function creations".
  - **The guard itself still has to pass.** All three RPCs write, so the widened guard wants their
    revoke whether they are definer or invoker. `revoke … from public, anon, authenticated` meets it.
  - **Unedited and green:** the view guard (still 5; R6 adds no view). `every_mail_table_has_rls_on…`
    is also unedited: it scans every file, and L1's file grants none of the seven mail tables and opens
    no policy on them.
  - **Unedited and green:** each `mailFunction` test ("the mail migration's definition governs").
    These pass because L1 redefines no function that `_mail.sql` defines. L1's own new test
    `the_mail_upload_migration_leaves_mail_hand_off_alone` checks that across all of them.
  - The `ours()`-scoped tests say nothing about L1, and the plan does not count them.

**Behaviour:**
- `mail_batches.via text not null default 'send' check (via in ('send','upload'))`.
- A per-mailbox, per-UTC-day, per-phase counter (`originals`, `bytes`).
- **Privileges, for all three RPCs below.** They follow `mail_hand_off` exactly:
  - `security invoker`, with `set search_path = public, extensions`;
  - `revoke execute on function public.<fn>(<args>) from public, anon, authenticated;`
  - `grant execute on function public.<fn>(<args>) to service_role;`
  - **Why invoker, not definer.** The mail stream's write RPCs (`mail_hand_off`, `count_mail_arrival`,
    `expire_mail_work`) are invoker. `delete_mail_address` is the one definer, for a reason of its own.
    Only the service role calls these three, and it needs no elevation. Under invoker, if an `EXECUTE`
    grant were ever left over (PUBLIC's default, or a later broad grant), a caller with the anon key
    would still meet the `mail_*` tables' RLS with no policy, and their revoked grants, and write
    nothing. Under definer, the same caller could write `mail_work` and `mail_seen` rows into any
    account, or raise any batch's `received`.
  - `public` is named in the revoke, as `mail_hand_off`'s revoke names it. Naming only `anon` and
    `authenticated` would leave PUBLIC's default grant.
- A charge RPC that adds one original and its bytes only if both stay at or under the caps it is
  passed, in P7's single guarded statement. The first charge of a day inserts the row only if its own
  bytes fit. It answers a boolean.
- A `received` compare-and-set for upload batches. `mail_batches` has none today: intake counts
  `received` by an optimistic PATCH with no `expected` bound, in `mail-inbound/index.ts`. The new one is
  one `update ... set received = received + 1 where <the batch> and received < expected and <open>
  returning received`. The route calls it alone only for a duplicate found before the put (P7).
- An upload hand-off RPC. It takes `mail_hand_off`'s arguments. In one transaction, in this order, it
  runs `mail_hand_off`'s share-locked mailbox check word for word (Checkpoint B), then that guarded
  `update`, then the same two writes. It locks the mailbox before the batch, the order in which
  `delete_mail_address` takes them too, so the two cannot deadlock. It answers one of:
  - `gone`: the mailbox is missing or `revoked`, and nothing was written or counted;
  - `queued`;
  - `duplicate`: the `mail_seen` insert conflicted, the count stands and no `mail_work` row is written;
  - `full`: the guard matched no row and nothing was written.

  Each answer comes with the batch's `received` (null for `gone` and `full`). Intake's `mail_hand_off`
  is not changed.
- The `received` compare-and-set needs no mailbox check when it runs alone (a duplicate found before
  the put). It writes no row that points at a `work/` object, and the batch is deleted with its
  mailbox.

**Done when** `deno test` is green on `cloud/supabase/`, with the execute-revoke guard at the N R6 found
plus three. Check and lint are clean. R6's report states both numbers.

### R7a. The shared hand-off, and intake never binds an upload batch (`cloud-engineer`)

**Why `cloud-engineer`:** it touches intake. **Files:** `cloud/supabase/functions/_shared/mail_handoff.ts`
(new; intake's steps 7 and 8 moved out of `mail-inbound/handler.ts`), `mail-inbound/handler.ts`,
`mail-inbound/index.ts` (`bindRows` filters `via = 'send'`), `mail-inbound/handler_test.ts` and
`_shared/mail_handoff_test.ts` (new). **Before:** R6.
- **Tests first:** `bind_rows_never_offers_an_upload_batch`; `the_moved_hand_off_writes_work_and_seen_in_one_call_after_the_put`;
  `a_failed_put_writes_no_row`; `a_hand_off_answered_full_or_gone_takes_the_put_back_out_and_kicks_nothing`
  (one case for each of the two answers). Every existing intake test passes unchanged: it is a move,
  not a rewrite.
- **Behaviour:** the shared hand-off takes its row writer from the caller, and the writer answers
  `queued`, `duplicate`, `full` or `gone` (P7, R6). On any answer but `queued` the put is taken back
  out, as intake does today when a concurrent delivery wins or the mailbox was revoked. Intake's writer
  wraps `mail_hand_off` unchanged: its `true` is `queued` and its `false` is `duplicate`, and it never
  answers `full` or `gone`. That `false` covers both a held Message-ID and a mailbox gone or revoked,
  which `mail_hand_off` does not tell apart and intake already treats alike ("duplicate or revoked,
  deleted"). The upload route's writer, which is R7c2's, calls R6's upload hand-off RPC.
- **Done when** intake's suite is green with no test edited except the import lines.

### R7b. The upload route: `via`, binding, refusals and caps (`cloud-engineer`)

**Why `cloud-engineer`:** `cloud/`. **Review:** `contract-reviewer`, once, after R7c2 (Review R7: the
MIME path, bytes of student mail). **Files:** `cloud/supabase/functions/mail/upload.ts` (new),
`mail/routes.ts` (`openBatch` takes `via`), `mail/handler.ts` (routing `POST /batches/<id>/originals`),
`mail/index.ts` (the batch lookup and the charge RPC call), `mail/upload_test.ts` (new) and
`mail/handler_test.ts`. **Before:** R6, R7a and R7d. R7d edits `routes.ts`, `handler.ts`, `index.ts`
and `handler_test.ts` first, so the two never run at once.

**Tests first**, in the note's §4.5 order (steps 1 and 2):
- `upload_refuses_another_accounts_batch`
- `upload_refuses_a_send_batch`
- `upload_refuses_a_closed_or_counted_batch`
- `upload_refuses_a_revoked_or_off_mailbox`
- `upload_over_the_original_cap_is_413_and_reads_nothing_more` (declared and streamed, and a body
  longer than it declared)
- `upload_without_a_declared_length_is_411_and_charges_nothing` (P7)
- `upload_over_the_batch_expected_or_the_day_cap_is_429` (both phases). The batch arm is P7's
  pre-check: a batch whose row already reads `received >= expected` is refused before the day caps are
  charged, and the charge RPC is never called.
- `upload_counters_hold_under_concurrency` (parallel uploads with the day cap at N+1). It runs against a
  fake RPC, so it proves only that the route obeys the RPC's answer and never charges twice. The
  atomicity itself is R6's statement form and C1.8's check on Postgres. The batch's guard is R7c2's to
  test, not this task's.
- `step_two_never_writes_received`. An upload that passes step 2 reaches `judgeAndHandOff`, a spy
  stub in this test so the test survives R7c, and by then no `received` RPC has been called and the
  batch's `received` is as it was.
- `open_batch_upload_is_m365_only` and `open_batch_without_via_is_send` (Gmail unchanged)
- `an_upload_batch_nonce_is_minted_and_stored_but_never_answered`

**Behaviour:** §4.5 steps 1 and 2: bind by session to an owned, `active` mailbox's open `upload`
batch, refuse a batch that already reads full (P7's pre-check, which reserves nothing), then charge
the day caps by the declared length before reading the body. A body that passes both
goes to one function, `judgeAndHandOff`, which R7c1 and R7c2 write. Until R7c1 lands, it answers a
fixed 501 sentence, and R7b's tests never reach it. Every refusal is a fixed sentence that echoes nothing.

**Done when** the whole `cloud/supabase/` Deno suite is green, and check and lint are clean.

### R7c1 and R7c2. The upload route after step 2 (`cloud-engineer`)

The note's §4.5 steps 3 to 8 are one piece, done as two dispatches in order. **The seam:** steps 3 to 6
refuse before anything is stored or counted, and steps 7 and 8 store and count. A reviewer can read
each diff alone. A reference to "R7c" elsewhere in this plan means both parts. **Review:** Review R7
(`contract-reviewer`) covers R7b, R7c1, R7c2 and R7d together, after R7c2.

#### R7c1. Steps 3 to 6: one message, the window, `own`, `foreign` and authenticity (`cloud-engineer`)

**Why `cloud-engineer`:** `cloud/`. **Files:** `mail/upload.ts` (`judgeAndHandOff`, steps 3 to 6) and
`mail/upload_test.ts`. **Before:** R7b (and so R6, R7a and R7d).

**Tests first** (§4.5 steps 3 to 6):
- `upload_refuses_a_body_that_is_not_one_message` (422)
- `upload_refuses_a_wrapped_message_and_never_unwraps_an_attached_one` (hard rule 1)
- `upload_outside_the_batch_window_is_422` (`deliveredAt` before `since` less one day, or after now)
- `upload_refuses_the_mailboxs_own_mail` (`own`)
- `upload_refuses_another_tenant_when_the_header_and_hint_disagree` (`foreign`; P6)
- `an_absent_tenant_header_or_hint_never_refuses`
- `upload_judges_authenticity_from_the_original_alone` (stamp arm, AuthAs-Internal arm, unaligned; no
  device field is read)

**Behaviour:** §4.5 steps 3 to 6 exactly. Authenticity is `originalAuthenticity(...)` unchanged.
`own` and `foreign` answer `{result, received}`, where `received` is the batch row's current count,
unchanged. A body that passes step 6 goes to one function, `storeAndCount`, which R7c2 writes. Until
then it answers a fixed 501 sentence.
- The tests that pass step 6 reach `storeAndCount` through a spy injected by the test, as R7b's
  `step_two_never_writes_received` does with `judgeAndHandOff`. That way they survive R7c2.
- The authenticity test reads the `authenticity` the spy received.

**Done when** the whole `cloud/supabase/` Deno suite is green, and check and lint are clean. This is
about 7 tests and about 50 lines of product code.

#### R7c2. Steps 7 and 8: dedupe, the hand-off and the count (`cloud-engineer`)

**Why `cloud-engineer`:** `cloud/`. **Files:** `mail/upload.ts` (`storeAndCount`), `mail/index.ts`
(the upload writer, which calls R6's upload hand-off RPC and its guarded `received` update) and
`mail/upload_test.ts`. **Before:** R5 and R7c1.

**Tests first** (§4.5 steps 7 and 8):
- `upload_twice_is_one_work_item` (`duplicate` the second time)
- `queued_and_duplicate_both_count_received` (both duplicate paths: found before the put, and the
  `mail_seen` insert that conflicts inside the RPC)
- `a_refusal_after_the_charge_leaves_received_unchanged`. One case for each answer that follows step 2:
  the 422 for not one message, the 422 for outside the window, `own`, `foreign`, a failed put (500) and
  a failed row write (500). In each, the batch's `received` reads what it read before, no `received`
  RPC is called, and the day counter keeps its charge (nothing is refunded; P7).
- `an_upload_the_hand_off_answers_full_is_429_and_keeps_nothing` (the fake RPC answers `full`: the 429
  batch sentence, no `mail_work` or `mail_seen` row, the put taken back out, no kick, `received`
  unchanged)
- `a_duplicate_into_a_full_batch_is_429` (found before the put; the guarded `received` update matches
  no row)
- `an_upload_whose_mailbox_was_revoked_after_step_one_keeps_nothing` (Checkpoint B: step 1 passed,
  then the fake RPC answers `gone`). The answer is step 1's revoked-mailbox refusal, with the same
  status and sentence `upload_refuses_a_revoked_or_off_mailbox` reads. No `mail_work` or `mail_seen`
  row is written, the put is taken back out, nothing is kicked, `received` is unchanged, and the day
  counter keeps its charge.
- `a_queued_upload_kicks_mail_process_once`
- `upload_logs_only_fixed_phrases` (a log spy holds no subject, sender, id or byte; it covers R7b's
  and R7c1's refusals too)

**Behaviour:** §4.5 steps 7 and 8 exactly, with step 8's count where P7 puts it.
- The hashes are `seenHashes`, imported from `../mail-inbound/keys.ts` (R5).
- The hand-off is R7a's `_shared/mail_handoff.ts` with this route's writer: R6's upload hand-off RPC,
  which counts `received` in the hand-off's own transaction.
- A duplicate found before the put is counted by R6's guarded `received` update alone.
- A `full` from either is the 429 batch sentence.
- A `gone` from the upload hand-off RPC is step 1's revoked-mailbox refusal, with the put taken back
  out and no kick. The device therefore meets no refusal it does not already know (R8).
- The answer is `{result, received}`, with `result` one of `queued`, `duplicate`, `own` or `foreign`
  (the last two are R7c1's). `received` is the count the RPC returned.
- `receivedAt` is the delivery time clamped to now.
- The tests run against fakes, so they prove the order and the route's reading of each answer. That
  the guard and the inserts share one transaction is R6's text test and C1.8's check. That the mailbox
  check runs first, under its share lock, is R6's `the_upload_hand_off_checks_its_mailbox_for_share_first`.

**Done when** the whole `cloud/supabase/` Deno suite is green, and check and lint are clean. This is 8
tests (one of them with six cases) and about 60 lines of product code.

### R7d. The reconcile answers held Message-ID hashes (`cloud-engineer`)

**Why `cloud-engineer`:** `cloud/`, and the answer is the dedup set the device trusts (note §4.6,
decision 8). **Review:** Review R7. **Files:** `mail/routes.ts` (`reconcile` answers `held_msg`),
`mail/handler.ts` (the store's held entry carries the Message-ID hash), `mail/index.ts` (the paged
`mail_seen` read also selects `msg_hash`) and `mail/handler_test.ts`. **Before:** R5. It touches
none of R7a's files, so it runs beside R7a. It runs before R7b because the two share these four files.

**Tests first:**
- `reconcile_answers_held_message_id_hashes_in_its_window`. It checks the account's hashes in the same
  widened window as `held`, never another account's, and paging past 1,000 rows.
- **One existing line changes:** `reconcile_answers_only_held_fingerprints_in_the_window_and_the_key`
  pins the answer's keys exactly. Its key list gains `held_msg`, and nothing else in that test moves.
  `the_reconcile_request_carries_only_the_window` stays unedited: the request is unchanged.

**Behaviour:** `held_msg` is the list of the account's `mail_seen.msg_hash` values whose received
minute lies in the window `held` already uses. It carries the hash only, never the minute, and it is
read by the same paged read as `held`. Gmail's device reads the answer as a loose `Value`, so the new
key changes nothing for it.

**Done when** the whole `cloud/supabase/` Deno suite is green, and check and lint are clean.

### R8. The device client (`implementer`)

**Why `implementer`:** specified HTTP against a local listener. **Review:** `contract-reviewer` for
`upload_original` (the MIME path). **Files:** `app/src/mail_client.rs` and `app/tests/mail_client.rs`
(new, so `app/tests/mail.rs` stays R9c's). **Before:** none (it codes to R7b's and R7c's contract, as
written here).

**Tests first** (a server on `127.0.0.1:0`):
- `open_batch_via_upload_sends_via_and_open_batch_still_sends_none`
- `upload_original_posts_message_rfc822_with_the_session_bearer` (and a `Content-Length` equal to the
  body's length, never a chunked body; P7)
- `upload_answers_map_to_queued_duplicate_own_foreign_and_named_refusals` (413, 411, 422, 429, 409, 402)
- `an_upload_body_never_reaches_an_error_string_or_a_log`
- `a_timeout_on_upload_is_a_named_outcome`

**Behaviour:**
- `open_batch_via(address_id, phase, since, expected, Via)`.
- `upload_original(batch_id, &[u8]) -> Result<Upload, MailError>`.
- A new `MailError` variant for each fixed refusal, with names in the existing `name()` and
  `fixed_sentence()` style.
- The retry policy is not here: it is R9b's.

**Done when** the tests are green and the workspace builds with 0 warnings.

### R9a. The pass, up to the upload list: list, diff, caps and bisection (`implementer`)

**Why `implementer` (Sonnet, high):** orchestration through traits, fully tested against fakes.
**Review:** `reviewer` (Review the rest). **Files:** `app/src/mail/upload.rs` (new), `app/src/mail.rs`
(`pub mod upload;` only), `app/tests/mail_upload.rs` (new) and `app/tests/support/owa_fake.rs` (new).
`owa_fake.rs` is a fake `Mailbox` that answers the seven new scripts' result shapes (R1's key paths,
synthetic values). It has an empty-mailbox constructor (`alive` ready, `whoami` matching, every allowed
folder's total 0), which R9c2 uses as the two fake hosts' default. **Before:** R4's script interfaces (the fake
answers their shapes meanwhile), R5 and R8. It codes to R7d's `held_msg` answer as written here.

**Tests first** (`owa_fake`, a fake `Uploader`, a fake `Clock`):
- `the_rust_message_key_and_hash_match_the_shared_vectors` (reads R5's file through
  `CARGO_MANIFEST_DIR`)
- `a_row_is_held_by_its_message_id_hash_first_and_by_fingerprint_without_one`. It also asserts the
  `/reconcile` body's keys are exactly `address_id`, `since` and `until`, which carries over the
  M365 half of the retired `nothing_about_an_unheld_message_is_sent_to_the_service`.
- `a_resumed_m365_pass_uploads_nothing_held`
- `m365_the_students_own_message_is_never_uploaded` (the note's I1 name; the device half). A row whose
  `kind` is not `received` never reaches the `Uploader`, even if a script let it through.
- `an_item_over_the_original_cap_is_left_too_large_and_never_read`
- `the_per_folder_total_is_checked_and_a_shortfall_is_not_found_list`
- `a_closure_never_ready_within_60_seconds_is_not_found_reader`
- `a_mismatched_mailbox_stops_the_pass_with_its_named_line`
- `the_floor_between_service_calls_is_250_ms_and_lives_in_the_clock`
- `a_window_over_3000_moves_since_later_by_bisection_before_the_batch_opens` (P8)

**Behaviour:** the note's §4.1 steps 1 to 5 and §4.6's device half: `alive`, `whoami`, the count and
P8's bisection, `POST /batches` with `via: upload`, `/reconcile`, the listing and the diff.
- `run_m365_pass(mb, uploader, clock, job) -> Outcome` reuses `Window`, `Held`, `Job`, `Outcome` and
  `Stop` from `recover.rs`, adding `Stop` variants where §4.1 names new outcomes. It takes no `Routine`:
  Microsoft 365 sends nothing, so it has no send pace (decisions 4 and 5 void).
- The diff hands the ordered rows to be uploaded to one function, `upload_rows`, which R9b writes.
  Until then it is a stub that uploads nothing, and R9a's tests assert only what comes before it.
- The key from `/reconcile` lives in memory for the pass only. It is never passed to a script.

**Done when** the file's tests are green, R3c's `no_m365_code_path_writes_an_original_to_disk` is
green, and the workspace builds with 0 warnings.

### R9b. The pass: chunk, upload and stop loop (`implementer`)

**Why `implementer` (Sonnet, high):** as R9a. **Review:** `contract-reviewer` (Review R9: chunk
assembly and the upload loop, the MIME path). **Files:** `app/src/mail/upload.rs` (`upload_rows`
and the stops) and `app/tests/mail_upload.rs`. **Before:** R9a, and R4's `mime.js` and
`mime_chunk.js` interfaces.

**Tests first:**
- `the_pass_uploads_oldest_first_one_at_a_time_and_drops_bytes_before_the_next_row`
- `a_chunk_mismatch_drops_the_original` (length, SHA-256 or Message-ID; counted `left`)
- `a_lost_upload_answer_is_retried_at_2_4_8_seconds_then_left`
- `m365_a_throttling_answer_stops_the_pass` (the `Outcome` carries `throttled_until` one hour on; R9d
  writes it down)
- `a_429_cap_stops_the_pass_for_the_utc_day`
- `a_frozen_page_is_a_named_pause_never_a_hang` (a non-zero `frozen_count`, or `discarded`, at the
  start or every 50th row, and two consecutive step timeouts while hidden)
- `a_lapsed_account_stops_the_pass_by_name`
- `the_slot_pass_stops_at_30_originals_or_10_minutes`. A backfill job is bounded by the day cap and
  never by the slot cap; this carries over the slot-cap half of the retired
  `an_open_backfill_ignores_the_slot_cap_and_keeps_microsofts_pace`.
- `the_batch_closes_with_queued_plus_duplicate`
- `progress_counts_messages_held_uploaded_skipped_left_and_bytes`

**Behaviour:** the note's §4.1 steps 6 to 8, §4.4's device half and §4.7. One original's bytes are
assembled in one buffer and dropped before the next row starts. The words are the note's: `paused:
Outlook asked Knowlu to slow down`, `paused: the mail page was put to sleep`, `the signed-in mailbox is
not the one set up`, `left (too large)`, and `not found` with its control.

**Done when** the file's tests are green, R3c's source tripwire is green, the workspace builds with 0
warnings, and Review R9 passes.

### R9c1 to R9c5. Routing, then the hold lifted one path at a time

The routing change and the hold's removal are one piece of the note's R9, done as five dispatches in
order: R9c1, then R9c2, R9c3, R9c4 and R9c5. Each one ends green, so a reviewer can read each diff on
its own. **The seam:** the four hold paths are tested separately (setup and Retry in `mail.rs` and
`mail_steps.rs`, launch resume in `mail.rs`, the slot in `scheduler.rs`), and `backfill_held_for_c3`
keeps answering for the paths not yet lifted. **Review:** `reviewer`, as part of Review the rest, with
the table "Tests the routing retires or rewrites" read row by row against each part's diff.

Rules for all five:
- **Kept means unedited.** The tests the table marks *kept* pass with no edit. That is decision 22's
  proof (`gmail_runs_exactly_as_before` in the ledger names this set, not a new test).
- **A red test the table does not name** stops the part. The implementer reports it to the controller
  with its reason before touching it. A test that starts to reach the upload pass and stays green with
  no edit needs no row.
- **Dead code.** 0 warnings is part of green. A part deletes whatever the compiler reports unused
  because of that part's own change, including test fakes, helpers and `use` items. It never adds
  `#[allow(dead_code)]` to product code. The support modules' existing module-level allow stays.
- **Rebase.** A part that touches `scheduler.rs` rebases on L6 if L6 merged first (§5).
- **Done when**, for every part: the test files it names are green under `cargo test -p knowlu --test
  <file>`; every other `app/tests` file is unedited and green under `cargo test -p knowlu`; nothing is
  newly ignored; the workspace builds with 0 warnings (the `.rsrc` line only). The part's report lists
  each table row it handled and what it did.

#### R9c1. The routing in `recover::run_pass` (`implementer`)

**Why `implementer`:** one specified branch, and the recover-pass tests it moves are named in the
table. **Files:** `app/src/mail/recover.rs`, and in `app/tests/mail.rs` the recover-pass section (the
`Fake` mailbox, `holding`, `pass` and the tests that call them) and the three pace tests that use
`quit_after_first_forward`. **Before:** R9b.
- **Tests first:** `run_pass_sends_an_m365_job_to_the_upload_pass_and_a_gmail_job_to_forwarding`. For
  M365 the mailbox is `owa_fake`, which panics on any DOM script. For Gmail it is `Fake`, which panics on
  any read script.
- Then the table's R9c1 rows: eight rewritten on Gmail (with the `quit_after_first_forward` helper) and
  three deleted.
- **Behaviour.** `run_pass`'s Microsoft 365 arm calls `upload::run_m365_pass` with the client as the
  `Uploader`. For M365 the `Routine` is passed through untouched, and nothing changes for Gmail.
- Every hold still stands, so no setup, resume or slot reaches the new arm yet. Only direct callers do.
- The M365 DOM helpers in `recover.rs` that the compiler now reports unused are deleted here. R10
  deletes what the compiler cannot see.

#### R9c2. The fake hosts answer the read pass (`test-writer`)

**Why `test-writer`:** it is test-support code only, fully specified, and green at once. **Files:**
- `app/tests/mail.rs`: `SHost`'s script arms and `setup_service`'s routes only.
- `app/tests/support/slot_rig.rs`: `FakeHost::script`'s arms and `Svc`'s routes.
- `app/tests/support/owa_fake.rs`: only if R9a left out the empty-mailbox constructor.

**Before:** R9c1.
- **The scripts.** Both hosts answer the seven read scripts through `owa_fake`, with an empty mailbox by
  default and a seam to load rows (as `msgs` does for Gmail today). Both hosts still panic on any script
  they do not know.
- **The routes.** Both fake services gain `POST /batches/<id>/originals`. It answers `{"result":
  "queued", "received": n}` and notes `upload` in the rig's event log. They also gain `held_msg: []` on
  `/reconcile`, and the batch's `via` is recorded beside its `since`. The existing event strings
  (`svc batch`, `svc canary` and the rest) and `batches` stay exactly as they are, because order
  assertions read them. If R8 closes an upload batch by a route other than `/batches/<id>/sent`, that
  route is added too.
- **`slot_rig.rs` is shared.** `app/tests/account.rs` (L4's test file) also includes it by `#[path]`.
  The change must compile and keep `account.rs`'s tests green with no edit to `account.rs`. If
  `slot_rig.rs` uses `owa_fake`, it includes that file itself.
- **Self-test:** `the_fake_setup_host_answers_every_read_script_and_panics_on_any_other` (`mail.rs`).
- **No other edit.** The holds still stand, so nothing reaches the new arms, and every existing test
  stays green unedited. **Also green:** `--test scheduler` and `--test account`.

#### R9c3. Setup and Retry (`implementer`)

**Why `implementer`:** two hold sites and the rows that pin them. **Files:**
- `app/src/mail_steps.rs`: the `held` closure in `Setup::new`, the Backfill block after it, and the M365
  refusal in `Setup::rearm_backfill`. `backfill_held_for_c3` itself stays, because `backfill.rs` and
  `slot.rs` still call it. `MAX_PER_MINUTE`, `MAX_PER_DAY` and `Routine` stay for Gmail.
- `app/src/mail/setup.rs` and `app/src/mail/runner.rs`, only if the scripted M365 Backfill needs a line
  there. The part's report names any such line.
- `app/tests/mail.rs`: the setup tests in the table and the `wait` helper. The helper's D13 branch, the
  one that answers an M365 Backfill's `needs_you` with `Done`, is removed. A test that drives the C2
  fallback answers it itself. An unexpected guided fallback then hangs to the helper's deadline and
  fails visibly, rather than being closed silently.
- `app/tests/mail_steps.rs`: the two rows in the table, and `backfill_held_for_c3` dropped from its
  `use` line once the deleted test was its only caller.
- `app/tests/scheduler.rs`: `no_file_or_log_of_knowlus_own_holds_the_address_or_token`'s setup half.

**Before:** R9c2.
- **Tests first:**
  - `an_m365_setup_backfill_runs_the_upload_pass_after_the_canary` (decision 21's order kept; it keeps
    the `ran_retired` check)
  - `an_m365_retry_runs_the_upload_pass`
  - `the_microsoft_backfill_is_scripted_and_re_armed_once_the_canary_arrives` (`mail_steps.rs`)
- Then the table's R9c3 rows: thirteen rewritten and two deleted.
- **Size.** Counted plainly, this is the one part over the sizing bound: 3 new tests and 13 rewrites.
  Six of the rewrites change only the one D13 `ended` assertion, so the substantive load is 3 new tests
  and 7 real rewrites.
  - It does not split further. The setup site and the Retry site are pinned together by two tests that
    cover both: `the_microsoft_backfill_is_the_guided_step_while_c3_is_built_and_is_never_re_armed`
    and `a_canary_not_back_in_three_minutes_…`. A split would need an interim rewrite of each that the
    next part rewrites again.
  - If the dispatch stalls anyway, the controller re-dispatches it with the six one-line rows done first
    and reported, then the rest. That is the same part and the same diff.
- **Behaviour.** An M365 setup's Backfill step is scripted and waits behind the canary, as Gmail's
  does. Retry re-arms a failed one. The `Mode::Guided` Backfill row is reached only through a named
  `not found`, as the C2 fallback.

#### R9c4. Launch resume, and the stale marker (`implementer`)

**Why `implementer`:** one hold site and P9. **Files:** `app/src/mail/backfill.rs` (`resume_one`: the
hold dropped from its guard, which still refuses `Provider::Guided`; P9's 404 branch; the module comment
naming the hold) and the resume section of `app/tests/mail.rs`. **Before:** R9c3.
- **Tests first:**
  - `an_m365_launch_resume_runs_the_upload_pass` (hidden, on the kept profile, with the window the
    first backfill fixed)
  - `a_resume_for_a_mailbox_the_service_no_longer_has_drops_its_marker_with_a_named_line` (P9: any
    other failure keeps the marker)
- Then the table's R9c4 row: one deleted.

#### R9c5. The slot, and the hold function removed (`implementer`)

**Why `implementer`:** the last hold site and the function's removal. **Files:** `app/src/mail/slot.rs`
(the hold and its `use`), `app/src/mail_steps.rs` (`backfill_held_for_c3` and its doc comment) and
`app/tests/scheduler.rs` (the table's R9c5 rows). **Before:** R9c4.
- **Tests first:** `the_slot_recovers_an_m365_mailbox_with_the_upload_pass_under_the_slot_cap`.
- Then the table's R9c5 rows: one rewritten and one deleted. The deleted test's Gmail half becomes its
  own test, unedited.
- **Behaviour.** The slot passes an M365 recover job to `run_pass` with `SLOT_CAP`, like Gmail's.
- `no_file_or_log_of_knowlus_own_holds_the_address_or_token`'s slot half now runs the upload pass on
  `slot_rig`'s empty mailbox. It must stay green unedited after R9c3's rewrite. If it does not, that is
  a stop and report.
- **Also done when** a search finds `backfill_held_for_c3` nowhere under `app/`.

### R9d. The throttle record and the progress fields (`implementer`)

**Why `implementer`:** two small, specified fields, at P5's two call sites. **Files:**
- `app/src/mail_session.rs`: P5's `throttled_until`, read with serde's default and skipped when empty.
- `app/src/mail/backfill.rs`: `drive_backfill` reads the field at the head of each loop turn, writes
  the `Outcome`'s throttle, and holds or clears as P5 says. This one site covers the setup's Backfill
  (`runner.rs` calls `drive_backfill`) and launch resume (`resume_one` calls `drive_backfill`).
  `resume_one`'s guard is R9c4's and does not move here.
- `app/src/mail/slot.rs`: the read before its `run_pass`, and the write after it.
- `app/src/mail/commands.rs`: P10's fields in `progress_of` and `status_for`.
- Tests: `app/tests/mail_progress.rs` (new), the backfill and resume sections of `app/tests/mail.rs`
  (R9c4's rig and kept profile), and the slot section of `app/tests/scheduler.rs` (`slot_rig`).
  `app/tests/support/slot_rig.rs` is touched only for the throttle seam below, under R9c2's rule that
  `app/tests/account.rs` stays green with no edit.

`runner.rs` is not in the list (P5). If the implementer finds a Backfill path in `runner.rs` that
reaches a pass without `drive_backfill`, that is a stop and report, not a second writer. If `SHost` or
`FakeHost` cannot yet answer a throttle, R9d gives each one a seam that answers `ErrorServerBusy`
through `owa_fake`'s throttling shape (the one R9b's `m365_a_throttling_answer_stops_the_pass` uses).
That is a test-support edit only, and every existing test stays green unedited.
**Before:** R9c5. It rebases on L4 if L4 merged first (§5).

**Tests first:**
- `throttled_until_survives_a_restart_and_older_mail_json_still_reads` (P5, `mail_progress.rs`). With
  nothing kept, the field is not written, as
  `the_pace_is_kept_beside_the_open_marker_and_costs_nothing_when_empty` checks for its own field.
- `a_throttled_mailbox_is_not_passed_again_within_the_hour` (I5, `mail.rs`, M365 on R9c2's `SHost`
  and a fake clock). It has three cases, and each counts the host's read-script calls:
  - **Setup.** A setup Backfill answered `ErrorServerBusy` at 10:00 writes `throttled_until` 11:00,
    shows `paused: Outlook asked Knowlu to slow down` with `resumes_at` 11:00, makes no read before
    11:00, and passes again after it.
  - **Launch resume, kept throttle.** `mail.json` holds the open marker and `throttled_until` 11:00
    from that run. A launch resume at 10:10 (`run_resume`, then `resume_one`) makes no read before
    11:00, then runs the upload pass. Without this case a resume read the page at 10:10 and every
    other test stayed green, because launch resume reaches the pass without `runner.rs` or `slot.rs`.
  - **Launch resume, new throttle.** A resume whose own pass is answered `ErrorServerBusy` writes
    `throttled_until` one hour on to `mail.json` before it holds. A second relaunch inside that hour
    reads nothing.
- `the_slot_skips_a_throttled_mailbox_and_keeps_a_throttle_it_hits` (I5, `scheduler.rs`). A slot at
  10:30 against the `mail.json` that the launch-resume case leaves makes no read, with a named line. A
  slot whose own pass is throttled writes the field, and the next slot inside the hour skips.
- `a_pass_after_the_hour_that_is_not_throttled_clears_the_field` (P5, `mail.rs`, through
  `drive_backfill` on the same rig).
- `the_progress_answer_carries_unit_messages_bytes_and_the_pause_word` (P10; `mail_status` too)

**Done when** the tests are green, the existing resume, backfill and slot tests are green unedited, and
the workspace builds with 0 warnings.

### R10. Retiring the DOM routine (`implementer`)

**Why `implementer`:** removal that needs judgment about what is M365-only. The window half of the
note's R10 is void (W1, §2), and the hold went in R9c. **Files:**
- `app/src/mail.rs`: `m365/search.js`, `list.js`, `forward.js` and `sent_cleanup.js` dropped from the
  script list.
- `app/src/mail/recover.rs`: any M365 residue the compiler could not flag, such as a `match` arm that
  still names `Provider::M365` on the DOM path, or a span or conversation helper Gmail does not call.
  `CLEANUP_*` stay if Gmail uses them.
- Deleted: `app/assets/mail/m365/{search,list,forward,sent_cleanup}.js`,
  `m365_mailbox_read_test.ts`, `m365_mailbox_send_test.ts` and `m365_mailbox_pages.ts`.
- `m365_scripts_test.ts`: its one log-and-credential test is kept and widened to the new scripts.
- `app/tests/mail.rs`, `app/tests/mail_window.rs` and `app/tests/support/slot_rig.rs`: the R10 rows of
  the table below, including the D13 comments R9c left (comments only; no assertion moves).

**Before:** R9d. **Tests:** no new behaviour, so the suites are the proof. Add
`no_retired_m365_script_name_remains_in_app_src_or_app_assets`, a source test. It scans `app/src/**`
and `app/assets/**` and never `app/tests/**`, so its own list of names does not trip it.

**Done when** `cargo build --workspace` and `cargo test --workspace` are green with the accepted
`.rsrc` line only, and `app/assets/mail` Deno is green. A search for the four retired names finds none
under `app/src` or `app/assets`. Under `app/tests` they survive only in the negative checks the table
keeps: `mail.rs`'s `RETIRED` const with `ran_retired`, `scheduler.rs`'s never-ran loop, and the new
tripwire's own list. Each of those says it is a never-ran check.

### Tests the routing retires or rewrites (R9c1 to R9c5, and R10)

Every existing test the routing change breaks is listed here with its fate. **Deleted** means its subject
retires with the DOM routine or the hold, and its C3 counterpart is named. **Rewritten on Gmail** means the
property in its name is Gmail's too (the send pace, Sent cleanup and the pass's privacy are kept for Gmail,
decision 22). In that case the provider and the script names change to Gmail's, and every other assertion
stays. If a count is in M365's one-forward-per-message unit, it is restated in Gmail's unit, and the
report says which line changed and why. **Rewritten for C3** means the test's other assertions stay and
only its D13 lines change to the upload pass. The reviewer checks each row against the diff.

| Test (file) | Fate | Task |
|---|---|---|
| `nothing_about_an_unheld_message_is_sent_to_the_service` (`mail.rs`) | rewritten on Gmail. The M365 reconcile-body check moves to R9a. | R9c1 |
| `recover_forwards_oldest_first_and_at_most_30_a_slot_when_no_backfill_is_open` (`mail.rs`) | rewritten on Gmail. The M365 counterpart is R9b's slot-cap test. | R9c1 |
| `an_open_backfill_ignores_the_slot_cap_and_keeps_microsofts_pace` (`mail.rs`) | rewritten on Gmail as `…_and_keeps_the_send_pace`. The slot-cap half for M365 is in R9b. | R9c1 |
| `the_batch_closes_with_the_count_sent_and_the_sent_copies_are_cleaned` (`mail.rs`) | rewritten on Gmail (`gmail/forward.js`, `gmail/sent_cleanup.js`) | R9c1 |
| `each_sent_cleanup_call_fits_inside_one_step` (`mail.rs`) | rewritten on Gmail (its own comment is about Gmail's hash navigation) | R9c1 |
| `a_microsoft_window_too_large_to_draw_is_narrowed_and_one_day_too_large_is_left` (`mail.rs`) | deleted (span narrowing retires). Counterparts: R3b's slice test and R9a's bisection. | R9c1 |
| `a_microsoft_conversation_row_is_opened_and_every_message_in_it_is_judged` (`mail.rs`) | deleted (C3 lists items, never conversations). Counterparts: R3b's own-mail drop and R7c's `own`. | R9c1 |
| `a_microsoft_conversation_found_in_two_spans_is_judged_once` (`mail.rs`) | deleted. Counterparts: R3b's disjoint slices, R9a's resume and R7c's `upload_twice…`. | R9c1 |
| `quit_after_first_forward` helper, `an_open_backfill_resumes_again_as_its_pace_allows`, `a_relaunch_on_the_same_utc_day_never_sends_past_the_days_600`, `the_pace_is_kept_beside_the_open_marker_and_costs_nothing_when_empty` (`mail.rs`) | rewritten on Gmail. The rig is already `google`; only the `Drive`'s provider and `m365/forward.js` move. They drive `run_pass` below `resume_one`'s hold, so the routing breaks them. | R9c1 |
| `a_canary_not_back_in_three_minutes_leaves_blocked_with_retry_and_offers_what_remains` (`mail.rs`) | rewritten for C3: after Retry, the canary re-arms the backfill and the upload pass runs. The blocked half stays. | R9c3 |
| `a_forwarding_just_set_is_given_time_to_start_and_a_second_canary_before_the_step_fails`, `an_uncurated_school_asks_once_for_the_address_and_the_service_decides_the_provider`, `with_the_saved_sign_in_off_setup_uses_mail_setup_and_wipes_it_on_close`, `a_microsoft_folder_pane_drawn_after_the_title_still_signs_in` (`mail.rs`) | rewritten for C3: the D13 `ended: stopped` line becomes the finished setup | R9c3 |
| `forwarding_elsewhere_is_asked_and_only_a_yes_replaces_it` (`mail.rs`) | rewritten for C3: in the first case (a yes) forwarding is set, the canary runs and the pass follows, so `ended` reads `finished`. The second case (no yes) never reaches the canary and is unchanged. | R9c3 |
| `the_progress_never_holds_an_address_or_a_token` (`mail.rs`) | rewritten for C3: the uncurated M365 setup now finishes through the upload pass, so `ended` reads `finished`. Every no-secret assertion stays, over the longer progress. | R9c3 |
| `no_date_is_asked_and_every_school_reads_the_same_120_days` (`mail.rs`) | rewritten for C3: the M365 window start is read from the upload batch's `since` | R9c3 |
| `a_microsoft_setup_ends_at_a_guided_backfill_and_runs_none_of_the_retired_scripts` (`mail.rs`) | deleted. It is replaced by `an_m365_setup_backfill_runs_the_upload_pass_after_the_canary`, which keeps the `ran_retired` check. | R9c3 |
| `an_open_microsoft_backfill_is_left_open_and_not_driven_at_launch` (`mail.rs`) | deleted. It is replaced by `an_m365_launch_resume_runs_the_upload_pass`. | R9c4 |
| `a_school_microsoft_mailbox_sets_up_in_order` (`mail.rs`) | rewritten for C3: every order assertion stays, `svc batch` follows `svc canary`, and `ran_retired` stays empty | R9c3 |
| `a_forwarding_set_by_hand_holds_the_backfill_until_the_canary_arrives`, `a_setup_in_flight_keeps_its_address_and_whether_it_was_placed_on_this_device` (`mail.rs`) | rewritten for C3: the backfill waits as a scripted step, and a begun upload backfill reads `pending(A, true, true)` | R9c3 |
| `a_setup_whose_backfill_began_never_removes_its_address`, `retry_runs_a_failed_scripted_backfill_again_…`, `a_closed_backfill_batch_starts_a_slot_now` (`mail.rs`) | kept (they run on Gmail) and unedited through R9c; only their D13 comments are updated, in R10 | R10 |
| `a_failed_step_falls_back_to_guided_for_that_step_only_and_later_steps_continue` (`mail_steps.rs`) | rewritten for C3: M365's Backfill is scripted and waits behind the canary | R9c3 |
| `the_microsoft_backfill_is_the_guided_step_while_c3_is_built_and_is_never_re_armed` (`mail_steps.rs`) | deleted with the setup and Retry holds. It is replaced by `the_microsoft_backfill_is_scripted_and_re_armed_once_the_canary_arrives`. | R9c3 |
| `no_file_or_log_of_knowlus_own_holds_the_address_or_token` (`scheduler.rs`) | rewritten for C3: the setup half runs to the finish through the upload pass instead of waiting for the guided step, and every no-address and no-token assertion stays. Its slot half is unedited and stays green after R9c5. | R9c3 |
| `a_microsoft_forwarding_found_off_is_turned_back_on_and_reported_reenabled_with_no_recover_pass` (`scheduler.rs`) | rewritten for C3 as `…_and_the_upload_pass_follows`: every forwarding assertion and the never-ran loop stay | R9c5 |
| `a_microsoft_mailbox_is_never_recovered_while_c3_is_built_and_a_gmail_one_still_is` (`scheduler.rs`) | deleted. It is replaced by `the_slot_recovers_an_m365_mailbox_…`; its Gmail half becomes `a_gmail_mailbox_still_recovers_by_forwarding`, unedited. | R9c5 |
| the D13 comments left elsewhere in `mail.rs` (the `RETIRED` const's, and the Gmail tests' "held while C3 is built" lines) and `backfill.rs`'s module comment | comments only, no assertion | R10 (`backfill.rs`'s in R9c4) |
| the `m365/{search,list,forward,sent_cleanup}.js` arms of `FakeHost::script` (`support/slot_rig.rs`) and the M365 DOM arms of `Fake` (`mail.rs`) | deleted; the shared `gmail/` arms keep only their Gmail name | R10 |
| `the_m365_mailbox_scripts_run_for_m365_only` (T10.2b, `mail_window.rs`) | rewritten as `the_m365_read_scripts_run_for_m365_only` over the seven new scripts, with the same assertions | R10 |
| `the_m365_scripts_run_for_m365_only` and test 34 (`mail_window.rs`) | kept (test 34 as R3c widened it) | — |

### R11. Progress and Settings copy (`console-ui`)

**Why `console-ui`:** `app/static`. **Files:** `app/static/console.js` (and `console.css` only if a
row needs it). **Before:** L4 and L7 merged (the queue, §5), Q1 answered and R9d's progress fields.

**Tests first.** They are source and string checks in `app/tests/static_assets.rs`, the file that
already reads `console.js`. If L4 or L7 added a page-level harness first, they go there instead:
- `the_m365_backfill_row_reads_n_of_m_messages_bytes_and_time_left`
- `the_named_pauses_read_as_their_words`
- `left_too_large_is_named`
- `the_m365_backfill_guide_never_asks_for_a_forward_as_attachment` (hard rule 2)
- `settings_never_says_resume_carries_on_for_a_mailbox_resume_cannot_carry`

**Behaviour:**
- The D13 guided placeholder for M365 Backfill goes; the C2 guide, with Q1's words, replaces it.
- `MP_PATH.m365`, the forwarding path, is no longer shown for the Backfill step.
- Minor #4's three lines take Quinn's wording.
- Gmail's rows are unchanged.
- *Q1 and minor #4 ruled 2026-10-19 (§4):* the guide's words and **Try again** are Q1's. Minor #4's
  lines are worded "until C3 lands" and land first on the email branch; which of them R11 keeps once
  the pass runs is not ruled, so R11 asks at the checkpoint rather than rewording.

**Done when** the console tests are green and a fixture screenshot (memory: console fixture
screenshots) shows the row, checked by the controller.

### R12a. D18's text: VISION A8 and email spec §7 (`docs-keeper`, in a build commit)

**Why `docs-keeper`:** signed wording, placed by heading. **Why it waits:** D18 says the edits "land
in the build's commits, so they never describe behavior the branch does not have". **Before:** R10
(the behaviour exists on the branch). **Files:** `VISION.md` and
`docs/specs/2026-09-30-email-forwarding-design.md`. Each edit is found by its phrase, never by line.
The words are the ledger row "QUINN RULING D18" and the note's §4.9, letter for letter:
1. **VISION, the "Knowlu reads" bullet.** After "older or missed mail forwarded to Knowlu's own address
   (whose Sent copies it removes)" insert " or, for school Microsoft 365, read on this computer and
   uploaded to Knowlu without changing the mailbox". Then add, after the bullet's existing marker,
   "*(amended 2026-10-13, M365 backfill redesign decision 18, signed by Quinn)*". The marker's form
   copies the bullet's own 2026-09-30 marker.
2. **Email spec §7, the mailbox-actions paragraph** ("What Knowlu does inside the student's mailbox").
   The forward-and-delete clause is marked "(Gmail)". Add "for Microsoft 365, reads older or missed
   mail through Outlook's own services and uploads it from this computer to Knowlu; it sends and
   deletes nothing".
3. **Email spec §7, the data table.** A new row after "The whole message": "| An older message read on
   your computer (Microsoft 365) | uploaded from your computer to our function's memory | for the
   length of that call |". The S3 row is left as it is.
4. **Email spec §7, the lawyer-packet paragraph.** The automated-access item takes "reading through
   Outlook's own services with the saved sign-in; no mail is sent", in the form Q7 settles.
5. `PRIVACY_VERSION` is not touched (D18).

*Q2 and Q7 ruled 2026-10-19 (§4):* edit 4 takes Q7's provider-scoped item; the VISION text needs no
pre-PR review, and Quinn's one ask is at the C3 PR, before the merge.

**Done when** `git diff` shows exactly these four edits, the reviewer checks them word for word against
the ledger row, and Q2's merge ask is noted on the PR.

### R12b. The rest of the docs (`docs-keeper`)

**Files:**
- Email spec §4.12's list (§4.2 step 5, §4.6, §4.11 steps 3 and 4, §5.1, §6.3, risks 13 and 15 closed
  for M365, test 34 widened). Each edit carries a dated "amended by the M365 backfill redesign" marker.
- §4.6 step 6 checked against W8, and fixed if L0 left it.
- The note's header status (Q6), plus a line that decision 19 was void after W1.
- The email plan's T10.2b and T11.2 ("the M365 half replaced by C3's plan") and §9.5's three rows.
- `docs/reference/app.md` (the pass, the caps, `throttled_until`, no new command).
- `cloud/supabase/README.md` (the route, `via`, the counters).
- After L4 merges and Q3 is answered, `site/privacy.html`'s two M365 sentences.
- *Q3 and Q6 ruled 2026-10-19 (§4):* the page takes D18's signed words with `PRIVACY_VERSION`
  unmoved (a reviewer reads the text, Quinn the page); the note's header takes Q6's line.

**HANDOFF lines** go to the Docs lane, not into the file here. **Before:** R12a. **Done when** every
cited heading still resolves and no doc names a school, tenant or person.

### R13. Integration, re-stamp and the merge train (`integrator`)

**Why `integrator` (Opus, high):** the rebase over lanes that landed first, the migration re-stamp
(§5) and the whole-branch gate. **Before:** R12b, and Reviews R2, R7 and R9 and the whole-branch
review passed.
- Rebase on `main`. Re-stamp the migration if needed, **immediately before C1's staging push and never
  after it** (§5): once C1's wait is met (Q8's merges, and by M7 each lane ahead cleared, its
  migration on staging and on `main`; only a lane that has neither merged nor pushed may instead be
  recorded as slipped), by M5's re-check against `main`, the stamps L3, L8 and L5 recorded, and
  staging's applied versions (the controller hands R13 that list), to M2's next free stamp. Any later
  rebase leaves the name as it is. The controller's `db push --dry-run` before C1 must then list L1's
  version alone (M7, §5); if it refuses on a remote version with no local file, R13 rebases again
  after that lane's merge or withdrawal, and no one runs `migration repair` to get past it. If the
  controller withdraws L1 after C1 (§8), R13's re-check runs again before the next C1.
- On every rebase, a conflict in `migrations_test.ts`'s execute-revoke guard is resolved by §5's rule:
  `main`'s number plus three, `main`'s comment with L1's clause last. Another lane's appended test
  block is kept beside L1's. The ledger records `main`'s number and the result.
- Run `cargo build --workspace` and `cargo test --workspace` (dev profile), then the three Deno trees,
  `scripts/ci/eol-check.ps1` and the contract-list and fixtures diffs (§1). It reads as green with the
  `.rsrc` line only.
- Record the counts in the ledger.
- **Done when** CI is green on the PR's head.

### Reviews

| Review | Agent | Covers | When |
|---|---|---|---|
| Plan review | `reviewer` (plan mode) | this plan against the note, the ledger rulings and VISION | before any task |
| Review R2 | `contract-reviewer` | `init.js` (capture scope, allow-list, slot, no header in any answer), R2a and R2b together | after R2b |
| Review R7 | `contract-reviewer` | the upload route: binding, caps, authenticity, dedupe, logging (R7b, R7c1, R7c2), the reconcile's `held_msg` (R7d), and P7's order: R6's upload hand-off RPC (its share-locked mailbox check first) and guarded `received` update (each `security invoker`, service-role only) read beside the route that calls them | after R7c2 |
| Review R9 | `contract-reviewer` | `mime.js`, `mime_chunk.js`, `upload_original`, and `upload.rs`'s chunk and upload loop (R9b) | after R9b |
| Review the rest | `reviewer` | R4 (other scripts), R6, R7a, R8, R9a, R9c1 to R9c5 (each part's diff, with the test table row by row), R9d, R10, R11, R12 | before the PR |
| Whole branch | `reviewer` | the PR diff over `main`, §1's must-not-moves | before R13's final run |

Two failed attempts on any task means one model level up (CLAUDE.md), never `max` or Fable without
Quinn.

## 7. Order, parallelism and checkpoints

```
plan review ── Quinn checkpoint (Q1–Q8) ── email merge ── branch cut (controller)
  wave A (parallel, disjoint files):  R0 · R1 · R3a · R5 · R6 · R8 · R3c
  wave B:  R2a ── R2b (after R3a) · R3b ── R3d (after R1) · R7a (after R6) · R7d (after R5)
  wave C:  R4 (after R1, R2b, R3b, R3d) · R7b (after R6, R7a, R7d) ── R7c1 ── R7c2
  wave D:  R9a (after R4's interfaces, R5, R8) ── R9b ── R9c1 ── R9c2 ── R9c3 ── R9c4 ── R9c5
           ── R9d ── R10
  wave E:  R11 (console queue: after L4, L7, Q1) · R12a (after R10) ── R12b
  close:   reviews ── R13 (the one re-stamp) ── C1 (staging; name frozen) ── re-proof ── PR
           ── Quinn's word (Q2) ── merge
```

- **Disjoint files in each wave.** In wave A, R3c's `mail_window.rs` edit is the only Rust test file
  touched, and R8 owns `mail_client.rs` and its new test file.
- **The `mail.rs` script list.** Only R4 (add), R9a (`pub mod upload;`) and R10 (drop) edit
  `app/src/mail.rs`, in that order.
- **Within a wave**, the parts of a split task (R2a and R2b, R3b and R3d, R7b, R7c1 and R7c2) share
  files and run one after the other, never in parallel. R7d shares `mail/` files with R7b, so it
  finishes in wave B before R7b starts. In wave B it is disjoint from R7a, which touches only
  `mail-inbound/` and `_shared/`.
- **Cargo runs.** At most two at a time machine-wide. Wave A runs one (R8); wave D runs one at a time.
- **Checkpoint 1** (after wave C): cloud review. `contract-reviewer` on R2 and R7.
- **Checkpoint 2** (after R10): app review. `contract-reviewer` on R9 and the MIME path; `reviewer` on
  the rest, with the test table read row by row against each of R9c1 to R9c5.
- **Checkpoint 3:** Quinn at C1 and the re-proof (§8), and the merge (Q2).

## 8. The closing gate: C1 and the live re-proof (controller only, structure only)

Both checks are the main session's. They run on staging and the kept scratch proof profile, with a
staging session minted by OTP (memory: staging session by OTP), reach no prod resource, and use no OS
input. Their results are counts, booleans and statuses only, and go to the lane ledger and a report in
`docs/reports/`. **The merge waits for both.**

### C1. The staging upload check (after R13's re-stamp and the staging deploy)

The controller pushes the migration and the `mail` and `mail-inbound` functions to **staging** (never
prod: prod deploys are Quinn's, handoff §1.7). From this push on, the migration's name is frozen (§5).
Q8 decides whether the push waits for L3, L8 and L5 (*ruled 2026-10-19: it waits until all three
have merged, or until the controller records that one slipped past C3*). By M7 (§5) it also waits
until each merged one's migration is on staging, and until each one that pushed from its lane branch
has merged or been withdrawn; neither a merge alone nor a pre-merge push alone is enough, and a
recorded slip clears only a lane that has neither merged nor pushed (a withdrawn lane counts as one).
Before the push, `supabase db push --dry-run` must list L1's version alone; otherwise C1 stops and
does not push (M7). A refusal naming a remote version with no local file waits for that lane's merge
or withdrawal and a rebase, never `migration repair` to get past it; a version no lane recorded goes
to Quinn (§5). The controller records C1's push in the session ledger. A loopback harness copies `upload_original`'s request
form, the token comes only from `SESSION_JWT_FILE`, and the uploads go to a recover batch of the
scratch profile's mailbox.

| # | Upload | Pass condition |
|---|---|---|
| C1.1 | W7's synthetic original (from the probe kit, held in memory) | `queued`; one `mail_work` row, `aligned` (count query) |
| C1.2 | the same again | `duplicate`; still one row; `received` 2 |
| C1.3 | a synthetic body of 27,262,976 bytes (26 MiB, over P4's 25 MiB cap) | 413 |
| C1.4 | a synthetic valid MIME just under the cap (about 24.9 MiB, a generated attachment, From a non-hint domain) | `queued`. If the platform refuses it first, decision 15's revisit: the cap comes down to what passes, Quinn hears it, and P4 and R7b change. |
| C1.5 | an upload into a `via = 'send'` batch | refused with the fixed sentence |
| C1.6 | a wrapped `message/rfc822` body (hard rule 1) | 422, and the batch's `received` reads what it read just before (a refusal after the charge counts nothing; P7) |
| C1.7 | the function logs over the window | fixed phrases only (a count of lines that do not match the phrase list: 0) |
| C1.8 | the compare-and-set on Postgres: a fresh recover batch opened with `expected` 2, then 4 distinct synthetic originals (From a non-hint domain) sent at once by the harness | exactly 2 answer `queued` and 2 are refused with the 429 sentence. The batch's `received` reads 2, and the `mail_work` rows carrying the batch's id number exactly 2, so no upload past `expected` was stored. The day counter's `originals` rose by no more than 4, the number sent; nothing is refunded (P7). These are count queries. Whichever of the two P7 guards refused (the step-2 pre-check or the hand-off's), the counts must hold. The day counter's RPC has the same single-statement form, so this run stands for both compare-and-sets. |
| C1.9 | an upload with no `Content-Length` (a chunked body) | 411, and the day counter is unchanged |

**If L1 will not merge as pushed** (a C1 row or the re-proof shows a defect in the migration's SQL,
or Quinn declines the merge at Q2), L1 has pushed from its lane branch and not merged, so it holds
any lane that re-lands behind it. The controller withdraws it by the rule's M7. Who decides follows
Quinn's ruling of 2026-10-19 (M7): on a defect in the SQL the controller withdraws on its own,
records it in the session ledger and tells Quinn; Quinn's no at Q2 is Quinn's word; any other case
(a failure it cannot place in the SQL or the functions, or L1 stopping for another reason) goes to
Quinn first. Each step goes in the lane and session ledgers:
- first redeploy `mail` and `mail-inbound` from `main`. `main`'s code never reads what L1 adds:
  `mail_batches.via` is `not null default 'send'` (R6), so `main`'s batch inserts still succeed, and
  L1 only adds functions, redefining none of `_mail.sql`'s (R6's
  `the_mail_upload_migration_leaves_mail_hand_off_alone`). So the withdrawal opens no window, and a
  forwarded message arriving meanwhile is handled by one build or the other against a full schema;
- then, on staging, in one `begin … commit`, drop what `<stamp>_mail_upload.sql` creates, as the
  file names them (the three upload RPCs and the upload day-counter table), and drop the columns it
  adds to `mail_batches` (`via` among them), so `main`'s definitions stand again;
- then check the catalogue (the three functions, the counter table, the added `mail_batches`
  columns) and record that none is left; anything left stops the withdrawal, the version row in place;
- then `supabase migration repair --status reverted <stamp>`;
- then confirm that `supabase db push --dry-run` on `main` no longer names L1's version.

L1 then counts as neither merged nor pushed: its name is unfrozen, and R13's re-check runs again
before the next C1. A defect in the functions alone needs no withdrawal: it is fixed on the lane
branch and the two functions are redeployed, and the migration stays as pushed.

### The re-proof (T18c's M365 backfill and a recover slot)

A dev build on the kept scratch profile runs the pass in its own hidden mail window. The harness reads
only the app's progress answers, the ledger counts and these structure checks. There is no
`Page.setWebLifecycleState`, no `bringToFront`, no focus emulation and no input.
*Q4 and Q5 ruled 2026-10-19 (§4):* school forwarding and the kept M365 session stay until this block
(T18c's cleanup leaves both), and the backfill below runs once, in full, within the model allowance.
- **Preflight.** School forwarding is on to staging (Q4). The two stale `open_backfills` markers resolve
  under P9 (the named line, then the marker gone). `throttled_until` is empty.
- **Before and after counts:**
  - Sent Items, Drafts and Deleted Items `TotalCount` must be unchanged.
  - One sampled unread message keeps `IsRead` false (W5 again).
  - The Inbox unread count may change only by mail arriving.
- **Backfill.** It counts `expected` (W9 makes about 1,100 to 1,200 likely, never a constant), runs to
  done or a named stop, and closes the batch with queued plus duplicate.
  - The progress view shows N of M messages.
  - Staging's `mail_work` rows for the batch number `queued` (a count query).
  - `frozen_count` is 0 throughout.
  - The model spend stays under the backfill allowance (Q5).
- **Live overlap.** The controller sends one synthetic message (D11 scope) while the pass runs. It is
  answered `held` or `duplicate`, and `mail_seen` holds one row for it (count 1; W10 made exact).
- **Recover slot.** One slot pass on the same profile uploads at most 30 and closes its batch.
- **Booleans recorded for P6:** whether intra-tenant MIME carries `X-MS-Exchange-CrossTenant-Id`.
- **Cleanup** (memory: live-proof profile cleanup) after Quinn's T18c decision on forwarding.

## 9. Risks

1. **OWA reshapes its services.** Validation makes the change a named `not found`, then the guided
   step; I3 means a wrong message is never uploaded. The re-proof and each release's smoke catch it.
   R1's recording is re-recorded, never edited by hand to pass.
2. **The closure is the most sensitive code here.** It gets `contract-engineer`, `contract-reviewer`, P1's
   pinned verb list and the tripwires.
3. **Platform request size (C1.4).** It could force the cap below 25 MiB. W9's largest was 17.2 MiB, so
   a cap at 20 MiB would still have held that window.
4. **Lane collisions.** `scheduler.rs` (L6), `mail_session.rs` (L4), `static_assets.rs` and
   `console.js` (the queue) and the migration stamp (L3, L8, L5). §5 says who rebases. The integrator
   re-runs the whole workspace after each. The stamp is the one collision that cannot be fixed by a
   rebase after C1. §5 freezes it there, Q8 (ruled 2026-10-19) makes C1 wait for the others (and
   M7 for each lane ahead's migration to be on staging and on `main`, a slip clearing only a lane
   that has neither merged nor pushed, and a `db push --dry-run` that lists L1's version alone, whose
   refusal on a pre-merge push waits for that merge or that lane's withdrawal, never a
   `migration repair` to get past it), and the shared rule's
   M5 re-check covers
   `main`, the lanes' recorded stamps and staging. The same lanes also move `migrations_test.ts`'s pinned function count. §5 fixes that sum (`main`'s number plus
   three), and the guard fails by name if the sum is wrong.
5. **A mailbox heavier than W9's.** The window cap (3,000) and the day cap (1,500) bound it, and a larger
   window simply takes more than one day (I7).
6. **Staging spend in the re-proof** (Q5). It is bounded by the allowance and asked first.
7. **A frozen page in a runtime W1 did not see.** The pass names it and resumes later. Decision 19's
   ladder stays documented in the note, unbuilt, and would be a new task if it ever fires.
8. **The test rewrites hide a weakening.** About thirty existing tests change in R9c1 to R9c5 and R10.
   The table in §6 ("Tests the routing retires or rewrites") fixes each one's fate in advance, a red
   test not in the table stops the part, and the reviewer reads the table against each part's diff row
   by row. No part's diff carries more than R9c3's 13 rewrites, and six of those are one line.
9. **Uploads that race past `expected` do work that is then discarded.** P7 guards `received` in the
   hand-off's transaction, so an upload over `expected` is parsed, judged and put, then refused and
   its put taken back out, and its day-cap charge is not refunded. This is bounded by how many uploads
   the device has in flight. A function that dies between the put and the transaction leaves one
   unreferenced `work/` object, as intake can today, and never a count. C1.8 checks that nothing past
   `expected` is stored.

## 10. Fidelity ledger

Each row is a requirement of the note (or of a ruling that binds this lane), the task that meets it,
and what proves it.

### 10.1 The note's decisions (§5)

| Decision | Ruling | Task | What proves it |
|---|---|---|---|
| 1 Mechanism: C3; C2 the fallback | 2026-10-13 | R4, R9a to R9c5, R10, R11 | the re-proof; `no_retired_m365_script_name_remains...` |
| 2 D5 in-page only; floor unchanged | 2026-10-11 | R2a, R3c | test 34 widened; `init_wrapper_is_inert_off_the_mail_hosts_and_in_subframes` |
| 3 The bearer closure, reads only, enforced in the closure | 2026-10-11/13 | R2a, R3a | `init_read_refuses_every_action_off_the_allow_list`; Review R2 |
| 4 Sent copies; 5 grouping and pace | void | R9c1, R10 | M365 never reaches the send pace (R9c1's routing; `run_pass_sends_an_m365_job_to_the_upload_pass…`); its forward and cleanup scripts removed (R10); the pace and cleanup tests rewritten on Gmail (§6 table) |
| 6 Which folders, by distinguished id | at recommendation | R4 | `m365_folders_keeps_ipf_note_and_drops_the_seven_excluded_subtrees...`, `m365_items_never_lists_an_excluded_folder` |
| 7 Which items; own and canary `skipped` | at recommendation | R4, R7c, R9a | `m365_items_drops_drafts_reports_other_own_mail_and_the_canary_sender...`, `m365_the_students_own_message_is_never_uploaded`, `upload_refuses_the_mailboxs_own_mail` |
| 8 Exact reconcile by Message-ID hash | at recommendation | R5, R7d, R9a | the shared vectors (both sides); `reconcile_answers_held_message_id_hashes...`; the re-proof's live overlap |
| 9 Progress in messages | at recommendation | R9b, R9d, R11 | `progress_counts_messages...`, `the_progress_answer_carries_unit_messages...`, `the_m365_backfill_row_reads_n_of_m_messages...` |
| 10 Recover uses the same pass | at recommendation | R9c1, R9c5 | `the_slot_recovers_an_m365_mailbox_with_the_upload_pass...`; the re-proof's slot |
| 11 D11 scope: W1 to W10 and C1 | 2026-10-13 | R0, §8 | R0's report; C1 within scope |
| 12 C3 adopted | 2026-10-13 | all | — |
| 13 Guided while C3 is built | 2026-10-11 (built `29a0ddb`) | R9c3 to R9c5, R11 | the hold lifted path by path after the routing (setup and Retry R9c3, resume R9c4, slot and the function R9c5; no `backfill_held_for_c3` left under `app/`); the slip rule keeps it if L1 slips |
| 14 Gmail G1 | L0's (handoff §4 item 2) | — | not this lane |
| 15 The route on `mail`, not a presigned PUT | at recommendation | R7b, R7c, §8 C1.4 | the route's tests; C1.4's platform-size result |
| 16 Caps, set from W9 | at recommendation; W9 1,139 / 318 MiB / 17.2 MiB / 0 over 25 MB | §3 P4 and P7, R6, R7b, R9a, R9b, §8 C1.8 and C1.9 | `upload_over_the_original_cap...`, `..._day_cap_is_429`, `upload_without_a_declared_length_is_411...`, `the_upload_charge_rpc_is_compare_and_set...` (the single-statement form), `the_slot_pass_stops_at_30...`, `a_window_over_3000...`; C1.8's compare-and-set on Postgres |
| 17 Bind by session, owned mailbox, open upload batch; judge on the original's own headers | at recommendation | R7b, R7c | `upload_refuses_another_accounts_batch`, `upload_judges_authenticity_from_the_original_alone` |
| 17b Tenant-id refusal if W8 shows it | at recommendation; W8 shows it on external mail | §3 P6, R7c | `upload_refuses_another_tenant...`, `an_absent_tenant_header_or_hint_never_refuses` |
| 18 VISION A8, spec §7, lawyer line; upload as its own row; `PRIVACY_VERSION` unchanged; edits land with the build | 2026-10-13 | R12a (and R12b for the page, Q3) | R12a's word-for-word check against the ledger row; Q2 |
| 19 Keep the page running | at recommendation; **void after W1** | — (the guard only: R2b's counters, R4's `alive.js`, R9b) | W1; `a_frozen_page_is_a_named_pause_never_a_hang` |
| 20 Over the cap: `left (too large)`, never cut | at recommendation | R9a, R11 | `an_item_over_the_original_cap_is_left_too_large_and_never_read`, `left_too_large_is_named` |
| 21 Setup order kept (backfill after the canary) | at recommendation | R9c3 | `an_m365_setup_backfill_runs_the_upload_pass_after_the_canary`; `a_school_microsoft_mailbox_sets_up_in_order` rewritten |
| 22 Gmail stays on sends | at recommendation | R7b, R9c1 to R9c5 | `open_batch_upload_is_m365_only`; the Gmail tests the §6 table marks kept, unedited (`gmail_runs_exactly_as_before` names that set) |
| W8(b) AuthAs-Internal, and the registrable-domain floor | 2026-10-18 | R7c (uses `ac08d6d` and `cdeff72` unchanged) | `upload_judges_authenticity_from_the_original_alone` (Internal arm) |

### 10.2 The bar (§2), the design (§4) and the reviewer's minors

| Requirement | Task | What proves it |
|---|---|---|
| I1 Received mail only | R4, R7c, R9a | the note's §4.8 I1 tests: `m365_items_drops_drafts_reports_other_own_mail...` (R3b), `m365_the_students_own_message_is_never_uploaded` (R9a, the device half), `upload_refuses_the_mailboxs_own_mail` and `upload_refuses_a_wrapped_message...` (R7c) |
| I2 Only to Knowlu, only this mailbox | R7b, R3c | `upload_refuses_another_accounts_batch`, `..._a_send_batch`, `..._a_closed_or_counted_batch`, `no_m365_script_names_a_knowlu_host` (the note's name, scope widened to `app/assets/mail/**`) |
| I3 The bytes are the message chosen | R4, R9b | `m365_a_message_id_that_changed_is_left...`, `a_chunk_mismatch_drops_the_original` |
| I4 Writes nothing in the mailbox | R2a, R3c, §8 | the allow-list test, the write-action tripwire and its self-test, the re-proof's before and after counts |
| I5 Microsoft's pace | R9a, R9b, R9d (P5's two call sites: `drive_backfill` for setup and launch resume, `slot.rs` for the slot) | `m365_a_throttling_answer_stops_the_pass`, `the_floor_between_service_calls...`, `throttled_until_survives_a_restart...`, `a_throttled_mailbox_is_not_passed_again_within_the_hour` (its setup, kept-throttle launch-resume and new-throttle launch-resume cases), `the_slot_skips_a_throttled_mailbox_and_keeps_a_throttle_it_hits`, `a_pass_after_the_hour_that_is_not_throttled_clears_the_field` |
| I6 A total first | R4, R9a | `m365_count_sums_allowed_folders_only`, `m365_a_short_list_is_not_found...`, `the_per_folder_total_is_checked...` |
| I7 Resumable | R7c, R9a, R9b, R9c3, R9c4 | `a_resumed_m365_pass_uploads_nothing_held`, `upload_twice_is_one_work_item`, `a_lost_upload_answer_is_retried...`, `an_m365_retry_runs_the_upload_pass`, `an_m365_launch_resume_runs_the_upload_pass` |
| I8 Runs hidden; never hangs | R2b, R9b, §8 | `a_frozen_page_is_a_named_pause_never_a_hang`; W1; the re-proof's `frozen_count` |
| I9 D5 floor; token and JWT separate; no bytes on disk or in logs | R2a, R3c, R7c, R8 | `no_m365_script_result_carries_a_bearer_or_jwt_shape`, `no_answer_error_or_property...`, `upload_logs_only_fixed_phrases`, `an_upload_body_never_reaches_an_error_string...`, `no_m365_code_path_writes_an_original_to_disk` |
| §4.1 steps 1 to 8 | R4, R9a, R9b | R3b's, R3d's, R9a's and R9b's tests in step order |
| §4.2 capture, use, pinned list, paging, waits | R2a, R4 | R3a; `m365_items_sends_paging_in_body_paging...`; `no_m365_script_chains_page_timers` |
| §4.3 items 1 to 3 (items 4 and 5 void after W1) | R2b, R9b | `freeze_and_resume_are_counted...`, `a_frozen_page...` |
| §4.4 slot, chunks, size, on device, never the other way | R2b, R4, R9a, R9b | slot and chunk tests; `an_item_over_the_original_cap...`; R3c |
| §4.5 the route, steps 1 to 8; `via`; `bindRows` | R6, R7a, R7b, R7c | R7b's and R7c's ordered refusals; `bind_rows_never_offers_an_upload_batch` |
| §4.5 step 8's count, placed by P7 (in the hand-off's transaction, after every refusal) | §3 P7, R6, R7a, R7b, R7c, §8 C1.6 and C1.8 | `the_upload_hand_off_guards_received_before_it_writes_a_row`, `the_mail_upload_migration_leaves_mail_hand_off_alone`, `a_hand_off_answered_full_or_gone_takes_the_put_back_out_and_kicks_nothing`, `step_two_never_writes_received`, `a_refusal_after_the_charge_leaves_received_unchanged`, `an_upload_the_hand_off_answers_full_is_429_and_keeps_nothing`, `a_duplicate_into_a_full_batch_is_429`, `queued_and_duplicate_both_count_received`; C1.8's `received` and `mail_work` counts |
| §4.6 dedupe with forwarded copies | R5, R7c2, R7d, R9a, §8 | the vectors; `upload_twice...`; the re-proof's live overlap |
| §4.7 pace, caps, progress, resume | §3 P4 and P5, R6, R7b, R9a, R9b, R9d | as decision 16, I5 and I7 |
| §4.9 privacy and D5 | R12a, R12b, Q3 | R12a's diff; the page after L4 |
| §4.10 what retires | R9c1 to R9c5, R10 | the hold (R9c3 to R9c5) and the DOM routine (R9c1's compiler-flagged half, then R10) removed; every changed test in §6's table with its fate; Gmail's kept tests unedited |
| §4.11 tests, the shapes fixture, vectors, tripwires | R1, R3a to R3d, R5, R7b, R7c, R7d, R8, R9a, R9b | each named above |
| §4.12 the spec, plan and VISION changes | R12a, R12b | the dated markers |
| Reviewer minor #2 (an original is the item's own MIME; no forward-as-attachment copy) | §1.1 rules 1 and 2, R2a, R2b, R4, R7c, R11 | `m365_scripts_never_read_an_attachment`, `upload_refuses_a_wrapped_message...`, `the_m365_backfill_guide_never_asks_for_a_forward_as_attachment`, C1.6 |
| Reviewer minor #4 (three copy lines) | R11, Q1 | `settings_never_says_resume_carries_on...` |
| Minor #5 (the note committed) | L0 | handoff §6 |
| The mail stream's RPC privileges (as `mail_hand_off`: `security invoker`, pinned `search_path`, revoked from `public, anon, authenticated`, granted to `service_role`) | R6 | `the_three_upload_rpcs_are_security_invoker_and_service_role_only`; the execute-revoke guard over every file |
| Checkpoint B kept for uploads: Disconnect's purge sees every upload hand-off (`mail_hand_off`'s share-locked mailbox check runs first in the upload hand-off; a mailbox gone or `revoked` answers `gone`, nothing is written and the put is taken back out) | §3 P7, R6, R7a, R7c2 | `the_upload_hand_off_checks_its_mailbox_for_share_first`, `a_hand_off_answered_full_or_gone_takes_the_put_back_out_and_kicks_nothing`, `an_upload_whose_mailbox_was_revoked_after_step_one_keeps_nothing` |

### 10.3 The note's §7 tasks, and the gates

| Note task | Plan task(s) | Agent |
|---|---|---|
| R0 live checks | R0 (report; checks already done) | main session |
| R1 recorded structure | R1 | main session |
| R2 read closure | R2a, R2b | contract-engineer; Review R2 by contract-reviewer |
| R3 failing tests | R3a, R3b, R3c, R3d | test-writer |
| R4 scripts | R4 (adds); R10 (deletes) | implementer |
| R5 shared vectors | R5; R9a's Rust half | test-writer; implementer |
| R6 migration | R6; R13's one re-stamp, before C1 only | cloud-engineer; integrator |
| R7 route, hand-off, reconcile hashes | R7a, R7b, R7c1, R7c2 (the route), R7d (the reconcile hashes) | cloud-engineer; Review R7 by contract-reviewer |
| R8 device client | R8 | implementer |
| R9 the routine | R9a, R9b (the pass), R9c1 (routing), R9c2 (the fake hosts), R9c3 to R9c5 (the hold, path by path), R9d (throttle record and progress) | implementer (R9c2: test-writer); Review R9 by contract-reviewer |
| R10 the window | R10 (the window half void, W1; the retire half moved here, the hold to R9c3 to R9c5) | implementer |
| R11 copy | R11 (last in the console queue) | console-ui |
| R12 docs | R12a (D18, in the build), R12b | docs-keeper |
| Review R2; review the rest | the Reviews table | contract-reviewer; reviewer |
| Re-proof | §8 C1 and the re-proof | main session, with Quinn |
| Handoff L1: Tauri lines to L6 | §3 P10, §5 | none expected |
| Handoff L1: migration after email, L3, L8, L5 | §5 under the shared stamp rule M1-M8 (re-stamp before C1 only, against `main`, the lanes' recorded stamps and staging; frozen after; C1 waits for Q8's merges and, by M7, each lane ahead's migration on staging and on `main` (a lane that pushed before merging holds C1 until it merges or the controller withdraws it), a recorded slip clearing only a lane that has neither merged nor pushed; by M7 a migration merges only with its staging push; C1's `db push --dry-run` lists L1's version alone or C1 stops, and a refusal on a remote version with no local file waits for that merge or withdrawal, never `migration repair` to get past it, while a version no lane recorded goes to Quinn; a merged, unpushed migration (M7 broken) out of order gets M8's re-stamp commit on `main`, or, behind a pushed, unmerged lane, Quinn's choice of a revert of its merge or a withdrawal of that lane; L1's own withdrawal after C1 if it will not merge as pushed (§8: the controller's own call on a defect in the SQL, Quinn told; Quinn's no is Quinn's word; anything else asked of Quinn first; the functions redeployed from `main` before the drop, the drop in one transaction, a catalogue check before the repair); no `--include-all`), R13, C1, Q8 | integrator; the controller for any post-C1 repair |
| Handoff §5: `migrations_test.ts` serialised with the migrations | §5 (L1's own block; the pinned function count is `main`'s number plus three), R6, R13 | cloud-engineer (R6); integrator (each rebase) |
| Handoff L1: console.js last in the queue | §5, R11 | console-ui |
