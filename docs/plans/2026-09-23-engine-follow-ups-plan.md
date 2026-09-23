# Engine follow-ups: the `unsure` card, `judgment_id`, feed paging, two minors

**Status: PLAN, written 2026-09-23, revised the same day to fold in the plan review (see "Plan
review (2026-09-23)" at the end). Not executed.** Base: worktree `.claude/worktrees/j-judgment-quality`,
branch `j-judgment-quality` at `e83f248` (PR #11, not yet merged to `main`). Execute on a new
branch cut from that commit, or on `j-judgment-quality` itself if PR #11 is still open when this
starts. **Written to survive a context compaction:** every task names its files, its decisions and
its tests, so a fresh session can execute from this document alone.

## Authority

- **`docs/notes/2026-09-23-know-whats-next-direction.md` §5** (on `main` at `5aacf65`, not on this
  branch; read it in the main checkout at
  `C:\Users\danie\GitHub\knowlu\docs\notes\2026-09-23-know-whats-next-direction.md`). It lists all three of Quinn's decisions this plan implements: `unsure`
  events surface as a decision card ("Does this apply to you?"; approve → obligation, reject →
  drop), built engine-side now. `judgment_id` is threaded end to end so every class-(b) correction
  becomes a calibration pair. The Localist/Engage feeds return only their first page.
- **CHECKPOINT J-1** (Quinn, 2026-09-22): `unsure` is a fourth verdict word
  (`docs/plans/2026-09-22-judgment-quality-plan.md` T1).
- **The direction note §4**, which says approval summaries are built "from structured fields
  ('Career fair · Thu 1 Oct 10–3 · venue')".
- Background, not authority: the research note, which is not in this worktree; it lives in the main
  checkout at
  `C:\Users\danie\GitHub\knowlu\.superpowers\sdd\2026-09-22-judgment-quality-plan\research-unsure-surfacing.md`
  (ranks the decision card first. It traces the `judgment_id` gap, `cloudmodel.rs` discards the
  reply's id). `docs/notes/2026-09-22-completion-detection-design.md` covers T8.
  `docs/notes/2026-09-22-stream-j-rollout.md` gives the deploy-ordering discipline this plan copies.
- The items to build are Quinn's. **Whether** each one is built is settled. This plan decides **how**.
  Every decision below gives a one-line reason, and the table at the end lists what each costs if
  it turns out wrong.

## Global Constraints (binding on every task)

Copied from `CLAUDE.md` where they apply. The task text never overrides them.

1. **No single-user assumptions.** Nothing names a person's vault, machine, account or credential. The
   one existing human-actor literal (`journal::human_set` matches actor `"quinn"`, `journal.rs:274`) is
   pre-existing and is **not** extended. New code records whatever actor the journal names.
   F3 and F8 **depend** on it: they find the student's decision through `human_set`, which works for
   every user today only because the app's `console_ctx()` writes actor `"quinn"` for every student
   (so every ledger answer reads `by:quinn`). If the console actor is ever renamed, F3 falls to
   `by:unknown` and F8 silently reports nothing; the rename must change `human_set` in the same
   commit. F3's approve test writes under that exact literal, so an engine-side rename fails it.
2. **Never regenerate a frozen reference.** The eight Python-written fixtures in `engine/tests/fixtures/`
   (including `vault-full/state/events.md`) are the contract: if the engine disagrees, the engine is
   wrong. `surface-today-{s1,s1-migrated,full}.json` change only in a commit whose diff shows the change
   and whose message says why. **No task in this plan changes any of them.** `--test oracle` and
   `--test surface_oracle` are in every relevant task's verification.
3. **`rank` never calls a model.** The event-check card (F2) and its settlement (F3) are deterministic.
   They run in `rank`, and `rank` makes no request of any kind for them. The label report (F8) runs in
   `judge`'s cloud arm, never in `rank`.
4. **Never rewrite a vault file wholesale.** Every note write goes through `write` (journal first,
   single-line frontmatter surgery second). New cards use `write::create`, and stamps use
   `write::write_literals`. `src/yamlemit.rs` is the only YAML emitter: no anchors, no aliases, two
   distinct dates.
5. **Ledgers, journal, run records and frontmatter are byte contracts with existing vaults.** Adding a
   key or an optional line field is allowed only when an old reader ignores it harmlessly, and each
   task states what the old reader does. Nothing is renamed. All JSON goes through
   `ledger::dumps_value`. `state/events-seen.md` stays CRLF (`pystr::NEWLINE`).
6. **Judge once, re-propose freely.** An agent never re-sets a field the journal shows the user set.
   **Never re-ask about an event the student already answered.**
7. **Approvals:** at most 15 new proposals a day. Overflow is snoozed, never deleted.
   `proposed_at` is the charge day. `first_proposed_at` is set once and drives every age.
8. **TDD.** Write the named tests first, see them fail, then write the code. Run tests in the
   foreground.
9. **0 warnings is part of green.** The one accepted line is the app's `.rsrc merge failure`. Four
   tests are `#[ignore]` by design and stay that way. Test in the dev profile (`--release` does not link).
10. **LF everywhere** (`*.ps1` CRLF). `engine/tests/fixtures/**` is `-text`: never re-encode it.
11. **No bare `git stash`.** Use a named stash or a commit, never `git stash` with no message.
12. **No secrets.** No token, key or session goes in the repo, a log, a test name or a fixture.
    Loopback-only tests (`127.0.0.1:0`) are fine. Nothing touches Supabase staging or production.
13. **No personal data in the repo.** Test events and emails are invented. The labelled items in
    `.superpowers/.../labelling/` are Quinn's and are never copied into a test or a fixture.
14. **Gmail Limited Use (controller ruling, 2026-09-23, on review I-4).** Gmail-derived signals only
    train that user's own personalised model. The device **never reports an `email`-kind decision**
    to the shared calibration/rules path: F8 sends no row whose `judgment_kind` is `email`, and F7's
    handler refuses one. `email` label evidence stays per-account (the `judgment_id` stays on the
    Gmail note or card in the student's vault, F6b, for a later per-user join) and is excluded from
    cross-account calibration (`calibration_query.sql`) and from `promote_rules`.
15. **Stay out of these paths:** `.claude/worktrees/c3-sync`, `.claude/worktrees/ci-self-hosted`,
    `.github/**`, and anything live. **No new engine module.** Every task adds code to an existing
    `engine/src/*.rs`, so `engine/src/lib.rs` and `engine/Cargo.toml`, which C3 edits, are not touched.
    No migration is added. The columns this plan needs already exist (`20260911000100`).

## Ordering and file ownership

| task | item | files | depends on |
|---|---|---|---|
| F1 | (a) the ledger learns answers, titles and `jid:` | `engine/src/eventledger.rs` | — |
| F2 | (a) the event-check emitter and series inheritance | `engine/src/eventemit.rs`, `engine/src/cli.rs` | F1 |
| F3 | (a) settling the card | `engine/src/approvals.rs`, `docs/surface/anatomy.md` | F1, F2 |
| F4 | (b) the device keeps `judgment_id` | `engine/src/judge.rs`, `engine/src/cloudmodel.rs`, `engine/src/events.rs`, `engine/tests/cloud_contract.rs` | F1 |
| F5 | (b) on task notes and amend cards | `engine/src/enrich.rs`, `engine/src/write.rs` | F4 |
| F6a | (b) the Gmail queue returns it | `cloud/supabase/functions/gmail-read/{index.ts,handler.ts,handler_test.ts}` | — |
| F6b | (b) Gmail notes and cards keep it | `engine/src/cloudmodel.rs`, `engine/src/enrich.rs`, `engine/src/completion.rs` | F4, F5, F11 |
| F7 | (b) the service accepts labels; the query | `cloud/supabase/functions/telemetry/{handler.ts,handler_test.ts,index.ts}`, `cloud/eval/calibration_query.sql`, `cloud/eval/calibration_cli_test.ts` | — |
| F8 | (b) the device reports decisions | `engine/src/enrich.rs`, `engine/src/cloudmodel.rs`, `engine/tests/cloud_contract.rs`, `CLAUDE.md` (one line) | F3, F5, F6b, F7 |
| F9 | (c) paging the two feeds, with a time budget | `engine/src/eventfeed.rs` | — |
| F10 | (c) judge only what `rank` would keep | `engine/src/events.rs` | F2, F4, F9 |
| F11 | (d) completion keyed on the note's `id:` | `engine/src/completion.rs` | — |
| F12 | (d) `MIN_CLASS_N` justified | `cloud/eval/calibration.ts`, `cloud/eval/calibration_test.ts` | — |

**Parallel lanes, with disjoint files inside each wave:**

- **Wave 1:** F1, F6a, F7, F9, F11 and F12. No two of them share a file.
- **Wave 2:** F2 and F4. They share no file.
- **Wave 3:** F3 and F5.
- **Wave 4:** F6b and F10. They share no file (F6b: `cloudmodel.rs`, `enrich.rs`, `completion.rs`;
  F10: `events.rs`).
- **Wave 5:** F8.

**Shared files, each edited in sequence and never in parallel:**

- `cloudmodel.rs`: F4, then F6b, then F8.
- `enrich.rs`: F5, then F6b, then F8.
- `completion.rs`: F11 (wave 1), then F6b (wave 4). F6b's `propose_done` parameter is added on top
  of F11's `target_id` change, never beside it.
- `events.rs`: F4, then F10.
- `cloud_contract.rs`: F4, then F8.
- `cli.rs`: F2 only.
- `eventfeed.rs`: F9 only. The judge-side use of its time budget is F10's change in `events.rs`.

**Conflict risk with other streams:**

- **C3.** This plan edits no C3 file, adds no module and adds no migration.
- **c1b/c1c.** No `app/**` file is edited. The decide path already works unchanged (F3 explains why).
- **The one cloud file with an owner of record:** `telemetry/handler.ts` is C1's (merged). No live
  branch lists it.

---

## (a) The `unsure` decision card

**The shape, in one paragraph.** When `rank` runs, its events pass finds events whose ledger verdict
is `unsure`, that nobody has answered and that start within `propose_horizon_days`. For each one it
files a `kind: event-check` approval: "Career fair · Thu 1 Oct 10am–3pm", with the body "Does this
apply to you?". The card rides the existing approvals machinery: the daily cap, snooze, expiry,
`surface --view decisions`, the console's generic Approve/Reject/Snooze, and the app's `decide`.
Settling happens in `approvals::process_approvals`, which runs inside `rank` and also runs in-process
at the end of the app's `decide_inner` (`app/src/commands.rs:248`). An approved card writes a
human-answer line to the event ledger meaning `obligation`, and a rejected card writes one meaning
`drop`. **So the app needs no change**, and the card leaves the deck on the click; the event joins
Coming up at the next `rank`.

**Why not file the card from `judge`, where `unsure` is minted.** `events::judge_roster` has no
`WriteContext` and no approval budget. `rank` already emits the events digest with both
(`cli.rs:391-400`), sized to the day's remaining budget. Filing from `rank` also catches every
`unsure` already in a ledger, including those written before this plan, with no backfill step.

### F1: the ledger learns human answers and `jid:` (`engine/src/eventledger.rs`)

**Decisions.**

1. **A human answer is a new, additive line shape** that `load_ledger` lets **supersede an `unsure`
   verdict, and nothing else**:
   `- <uid> · <title> · verdict:<obligation|drop> · by:<actor> · jid:<uuid> · answered <YYYY-MM-DD>`.
   Reason: the ledger's rule is "first verdict wins" (`eventledger.rs` `load_ledger`,
   `if current.verdict.is_some() { continue }`). Without an exception the student's answer could
   never take effect. The exception is narrowed to `unsure` so that a confident machine verdict can
   still never be silently flipped.
2. **Machine verdict lines gain an optional ` · jid:<uuid>` field** before `first seen`. `record_verdict`
   keeps its signature. A new `record_judged_verdict(vault, uid, title, when, verdict, why,
   judgment_id: Option<&str>)` shares one line builder with it, so the five existing callers
   (`cli.rs` ×2, `events.rs` ×2, `eventemit.rs` ×1) do not change.
3. **`record_answer(vault, uid, title, when, verdict, by, judgment_id: Option<&str>)`**, where
   `verdict ∈ ANSWER_VERDICTS = ["obligation", "drop"]`. It refuses anything else. `by` must match
   `^[A-Za-z0-9_.:@-]{1,64}$` and `jid` must match `^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`.
   A failure is the new `VerdictError::BadField(&'static str)`. Reason: `by:` and `jid:` sit unquoted
   on the line, so they cannot be allowed to carry ` · `.
   **The title is sanitised by construction (review I-6).** `record_verdict`'s title cleaning
   (`·`→`-`, `\n`/`\r`→space, trim, empty→`(untitled)`, `eventledger.rs:435-439`) moves into one
   private `clean_title(&str) -> String`, and all three writers (`record_verdict`,
   `record_judged_verdict`, `record_answer`) build their line through it. No writer can put a raw
   title on a line, so a card title like `Career fair · Thu 1 Oct 10am–3pm · +3 more` is written as
   `Career fair - Thu 1 Oct 10am–3pm - +3 more`.
4. **`LedgerEntry` gains `answered_by: String` and `judgment_id: String`**, both defaulting to empty.
   It also gains **`title: String`** (default empty): the second `·`-segment of the uid's first
   verdict line, read by a new `TITLE` regex `^- [A-Za-z0-9_.:@+-]+ · (?P<title>[^·]*?) · verdict:`.
   It exists so F3 can give each uid its own title on its answer line (review I-6); it is read-only
   state and changes no line anyone writes.
   On a superseding answer the entry keeps the `unsure` line's `why`, and its `judgment_id` becomes
   the answer's `jid` (the card carries the `unsure` judgment's id, see F2). The first human answer
   wins, and a second one is ignored. That is what makes F3's settlement idempotent.
5. **`HEADER` is not changed.** It is written only when the file is created, so changing it would
   change the bytes of every new vault's ledger. The module doc gains a paragraph on the exception.

**What an old reader does with the new lines.** It still matches `VERDICT` (`· verdict:obligation`),
and the word is valid, but first-verdict-wins keeps the earlier `unsure`. So on an older engine an
answered event stays invisible, exactly as it was before the answer. There is no crash, no warning
and no misread. It ignores `jid:`, `by:` and `answered` because no regex looks for them. A title
cannot forge them: `record_verdict` already maps `·` to `-` in titles, and `why_problem` refuses
` · `.

**Tests first** (in-module, `mod tests`):
- `a_human_answer_replaces_unsure`: `record_judged_verdict(.., "unsure", .., Some(J))`, then
  `record_answer(.., "obligation", "quinn", Some(J))`. `load_ledger` gives `verdict == Some("obligation")`,
  `answered_by == "quinn"` and `judgment_id == J`.
- `a_human_answer_never_replaces_a_confident_verdict`: `drop`, then an answer of `obligation`. The
  verdict stays `drop` and `answered_by` stays empty.
- `the_first_human_answer_wins`: `unsure`, then two answers. The verdict is the first answer's.
- `an_answer_before_any_verdict_is_just_a_verdict`: an answer line alone reads as that verdict, with
  `answered_by` set.
- `the_answer_line_is_byte_exact`: the appended bytes equal
  `- engage:1 · Career Fair · verdict:obligation · by:quinn · jid:<J> · answered 2026-10-01` plus
  `NEWLINE`.
- `a_judged_verdict_line_carries_jid_and_round_trips`: its bytes, and `judgment_id` on read.
- `record_verdict_bytes_are_unchanged`: the existing `verdict_round_trips` shape with no `jid`. Byte
  for byte as before.
- `record_answer_refuses_a_bad_word_actor_or_jid`: `opportunity`, `by:"a b"` and `jid:"x"` each
  give `Err`, and the file is not touched.
- `an_answer_title_is_sanitised_like_a_verdict_title`: `record_answer` with the title
  `Career fair · Thu 1 Oct 10am–3pm · +3 more` and one with a `\n` writes the `·`→`-`,
  newline→space form, byte for byte, and the line still reads back to exactly one verdict,
  `by` and `jid`.
- `the_first_verdict_lines_title_is_kept`: a verdict line, then an answer line with a different
  title. `title` is the verdict line's.

**Verify:** `cargo test -p knowlu-engine --lib -- eventledger::`, then `cargo test -p knowlu-engine --lib -- eventroster:: eventemit::`
(the existing `LedgerEntry { .. }` literals compile with `..Default::default()`; add it where one is
missing).

### F2: the event-check emitter (`engine/src/eventemit.rs`, `engine/src/cli.rs`)

**Decisions.**

1. **`pub fn emit_event_checks(vault, events, ledger, config, today, budget: i64, ctx, journal) -> (Vec<PathBuf>, usize)`**
   lives beside `emit_digest`, because that is the same job for a different verdict.
2. **Who is asked.** An event qualifies only if all of these hold:
   - ledger verdict `unsure`, `answered_by` empty, not `declined`, not `proposed`;
   - start date in `[today, today + propose_horizon_days]`. Asking earlier is premature, and after
     the start is pointless;
   - `uid` not in `approvals::existing_source_uids(vault)`;
   - no `event-check` card in `approvals/` or `archive/` names the same `series_uid`.

   Reason: the source-uid, series and `proposed` checks are the three ways to be "already asked", and
   any one of them closes the question for good. That is judge-once for questions. **Closing the
   question is not the same as dropping the answer:** a settled series card's answer reaches the
   series' later instances through decision 9. An empty `series_uid` cannot occur, because
   `DiscoveredEvent::normalized` sets it to the uid (`events.rs:79-83`), so a one-off event is its
   own series.
3. **One card per series.** The primary instance is the soonest one. The card's `events:` lists every
   qualifying instance of that series, sorted by `(start, uid)` and capped at 20. Reason: a weekly
   meeting asked about eight times trains the student to reject blindly, and its instances are not
   independent answers.
4. **The budget.** `allowance = min(budget, EVENT_CHECKS_PER_DAY - event-checks first proposed today)`,
   with `EVENT_CHECKS_PER_DAY = 3`. "First proposed today" counts `kind: event-check` cards in
   `approvals/` and `archive/` whose `first_proposed_at` is `today`, **not** `proposed_at`:
   `defer_over_budget` rewrites `proposed_at` to tomorrow (`approvals.rs:962-966`), and counting
   by it would charge a deferred card against tomorrow's 3 (review M-4). `first_proposed_at` is set
   once and never moves. The emitter sizes itself exactly as the digest does, so there is
   never overflow for `defer_over_budget` to snooze. Reason: the cards cost real attention and must
   not crowd task and amend cards out of the 15.
5. **Order.** `(primary start, primary uid)`. The whole function is a pure function of its inputs plus
   the vault's `approvals/` and `archive/`.
6. **The title comes only from structured fields:** `what_and_when(event)`, a public function, gives
   `{title≤60} · {Ddd} {d} {Mon} {range}`. It never uses a model.
   - `range` uses a 12-hour clock with minutes only when non-zero. The end always carries am/pm. The
     start carries am/pm only when it is on the other side of noon.
   - Examples: `10am–3pm`, `7–9pm`, `10:30–11:15am`, `11am–12pm`, `12–1pm`.
   - The dash is U+2013. An all-day event (midnight to midnight) gets no range. A multi-day event
     gets `Thu 1 Oct – Sat 3 Oct`.
   - A series card appends ` · +N more`.
   - **Quinn's example read `10–3`; ruled `10am–3pm`** (see Open questions).
7. **The card**, written through `write::create` with `yamlemit::Node::map` (the `completion.rs` shape).
   - **Frontmatter:** `type: approval`, `kind: event-check`, `title`, `status: pending`,
     `source_uid: <primary uid>`, `series_uid`, `events: [uids]`, `judgment_id` (only when the
     primary's ledger entry has one), `judgment_kind: event` (only with `judgment_id`),
     `proposed_at` and `first_proposed_at` (both `Node::Date(today)`, never an anchor),
     `expires: <primary start date>`, `snooze_until: null`, `created_by: events`. The literal
     `events` matches the digest (`eventemit.rs:316`) and `calendar_note` (`approvals.rs:1055`),
     the two cards the events pass already files; the journal record's actor stays
     `agent:events`, exactly as the digest's does (review M-3).
   - **Only the primary instance's `judgment_id` rides on the card.** The other instances' `unsure`
     judgments get no label row. That is accepted: `unsure` is excluded from calibration anyway
     (F7), and the series answer still reaches every instance's ledger line (F3, decision 9)
     (review M-9).
   - **Body:** a first paragraph reading `**Does this apply to you?** Knowlu could not tell from the
     event's own listing whether it is meant for you.` That is the paragraph `surface::first_paragraph`
     shows as the card's `why`. Then a `When · Where · Who` line built from the fields, the URL when
     present, an `Also on:` line for a series, a closing line (`Approve if it applies to you: it joins
     Coming up as something you're expected at. Reject and it's dropped. Either way you won't be
     asked again.`) and `eventemit::BUTTONS`.
   - **Path:** `approvals/event-check-<slug>-<YYYY-MM-DD of start>.md`, where the slug is a
     lowercase `[a-z0-9-]` form of the title of at most 40 characters. A collision takes `-2`, `-3`
     and so on, as `approvals::materialize` does.
   - After each `create`, the emitter calls `record_proposed(uid)` for every member, as the digest
     does. That is the second guard against a re-ask.
8. **`cli.rs`**, two calls and no other change.
   - Directly after `load_ledger` (`cli.rs:373`) and before `write_roster`: call
     `inherit_series_answers` (decision 9) with the prefiltered `candidates` and `&mut ledger`, and
     push its warnings as `ledger: …` like the load warnings beside it. Placing it there means the
     roster and digest of the same run already see the inherited verdict.
   - Directly after `emit_digest` (`cli.rs:391-400`): call `emit_event_checks` with
     `(remaining_budget - emitted as i64).max(0)`, the prefiltered `candidates`, the ledger, and
     `ctx.with_actor("agent:events")`. Add the count to `approvals.pending`.
   - In the `vault-full` fixture no ledger line is `unsure` and no `event-check` card exists, so both
     calls are no-ops there and the golden output cannot move.
9. **A settled series answers its later instances (review I-1).** Without this, approving a weekly
   meeting shows two weeks of it and then nothing: the instances that enter the horizon later are
   judged `unsure`, are never asked (decision 2's series check), and never get the answer.
   `pub fn inherit_series_answers(vault, events, ledger: &mut BTreeMap<String, LedgerEntry>, today)
   -> (usize, Vec<String>)` lives in `eventemit.rs` and is deterministic:
   - `settled_series(vault) -> BTreeMap<String, SeriesAnswer>` (public; F10 reuses it) reads every
     `kind: event-check` card in `archive/` whose `status` is `executed` (answer `obligation`) or
     `rejected` (answer `drop`), keyed by `series_uid`. `SeriesAnswer { verdict, by, source_uid }`,
     where `by` is the `answered_by` of the ledger entry for the card's `source_uid` (F3 wrote it),
     falling back to `"unknown"`. Two settled cards for one series cannot arise (decision 2), and if
     a hand edit makes one, the card with the lowest file name wins, so the result is still a pure
     function of the vault.
   - For each event in `events`, in `(start, uid)` order, whose ledger verdict is `unsure` **or
     absent**, whose `answered_by` is empty, and whose `series_uid` is in `settled_series`:
     `eventledger::record_answer(vault, uid, <event title>, when, answer.verdict, answer.by,
     <the uid's own ledger judgment_id, if any>)`, then update the in-memory entry exactly as
     `load_ledger` would read the new line (verdict, `answered_by`, `judgment_id`).
   - The uid's **own** `jid` is used, not the card's, because F1 decision 4 defines an entry's
     `judgment_id` as the judgment the answer settles, and this line settles the instance's own
     `unsure` judgment. F8 reports from cards, never from these lines, so no extra label row
     results.
   - **What it does not touch:** an instance that already has a confident machine verdict keeps it
     (F1's supersede-`unsure`-only rule). F10 stops `judge` from paying for, and confidently
     judging, a new instance of a settled series, so in practice that only covers instances judged
     before the card was settled. A series whose card **expired** unanswered is not in
     `settled_series`, so its later instances stay `unsure` and unasked: the same "an expired card
     leaves the event `unsure`" decision, extended to the series and stated as such.
   - A write failure is a warning (`series answer not recorded for <uid> (<err>)`) and the next run
     retries; the run never fails on it.

**Tests first** (`eventemit.rs` `mod tests`, invented events only):
- `what_and_when_formats_every_shape`: a table covering the six range examples above, all-day,
  multi-day, and 2026-10-01 read as `Thu 1 Oct`.
- `an_unsure_event_in_the_horizon_files_one_event_check_card`: checks the exact title, `kind`,
  `source_uid`, `expires == start date`, `judgment_id`, `judgment_kind`, the body's first paragraph,
  `proposed_at == first_proposed_at == today`, and a `create` record in the journal.
- `a_second_run_files_nothing`, and `nothing_is_asked_after_a_card_is_archived_rejected_or_expired`.
- `confident_answered_declined_or_proposed_events_are_never_asked`.
- `events_past_or_beyond_the_horizon_are_not_asked`.
- `a_series_gets_one_card_listing_its_unsure_instances`: 4 instances produce one card with 4 `events`
  and a ` · +3 more` suffix.
- `the_emitter_respects_the_budget_and_the_daily_ceiling`: 5 qualify, budget 15, 3 filed. A second
  call the same day files 0. With budget 1, 1 is filed.
- `a_card_without_a_ledger_jid_omits_both_judgment_keys`.
- `the_daily_ceiling_counts_first_proposed_at_not_proposed_at`: a card filed today and then
  deferred (`proposed_at` tomorrow, `first_proposed_at` today) still counts against today's 3, and
  tomorrow's allowance is a full 3.
- `the_card_says_created_by_events_and_the_journal_says_agent_events`.
- `a_later_instance_of_an_answered_series_inherits_the_answer`: a weekly series, an archived
  `executed` card naming its `series_uid`, the first instance answered `by:quinn`. A new instance
  whose ledger verdict is `unsure` gets an `obligation` line `by:quinn` with its own `jid`, and the
  in-memory ledger shows it at once. A `rejected` card gives `drop` the same way.
- `an_unjudged_instance_of_a_settled_series_inherits_too`: no ledger line at all, and the answer
  line is written.
- `a_confident_instance_or_an_expired_series_card_inherits_nothing`.
- `inheriting_twice_writes_one_line`: the second run finds `answered_by` set and writes nothing.
- `cli.rs`: `rank_files_an_event_check_for_an_unsure_event_and_counts_it_pending`, using an injected
  `Fetchers.events` closure that serves an invented ICS feed and a seeded ledger.
- `cli.rs`: `rank_shows_a_later_instance_of_an_approved_series_in_the_same_run`: the inherited
  instance is in `state/events.md`'s roster as an obligation on the run that writes its line.

**Verify:** `cargo test -p knowlu-engine --lib -- eventemit:: cli::`, then `cargo test -p knowlu-engine --test oracle --test surface_oracle`.

### F3: settling the card (`engine/src/approvals.rs`, `docs/surface/anatomy.md`)

**Decisions.**

1. **The settlement goes in `transition_note`** (`approvals.rs:1211`), next to the other kinds.
   - **`rejected`:** before the generic `delete`, and as the `events-digest` arm already does,
     `settle_event_check(vault, meta, "drop", today, journal)?`.
   - **`approved`:** a new `kind == "event-check"` arm settles with `"obligation"`, stamps
     `status: executed` and `executed_at`, `delete`s the card, and pushes the stem to
     `result.executed`. That is exactly the `amend` arm's tail.
   - **`pending` / `snoozed` / `expired`:** unchanged. An expired card writes nothing, so the uid stays
     `unsure` and is never asked again, because the archived card still names it.
2. **`settle_event_check`** takes the uids from the card's `events:`, falling back to `[source_uid]`.
   - `by` is the actor of `journal.human_set(<card id>, "status")`, or `"unknown"` when the journal
     holds no human record, for example a card edited by hand outside the console.
   - `jid` is the card's `judgment_id` for the primary uid (`source_uid`); every other uid gets its
     **own** ledger entry's `judgment_id` when it has one, and none otherwise, for the reason in F2
     decision 9.
   - **Each uid gets its own title (review I-6):** the `title` of its ledger entry (F1 decision 4),
     falling back to the card's `title` only when the entry has none. The series card's
     `… · +N more` title is therefore never copied onto every instance. Either way `record_answer`
     sanitises it (F1 decision 3).
   - It calls `eventledger::record_answer` once per uid.
   - An I/O error is returned as `WriteError::Io`. The pass then logs `transition failed:` and leaves
     the card where it is, so the next run retries. A repeated answer is harmless (F1, decision 4).
3. **Why this needs no app change.** `decide_inner` writes `status` with `console_ctx()` and then calls
   `process_approvals` in the same process with the same `Journal` (`app/src/commands.rs:241-248`).
   The new arm runs there. The console renders any `kind` through `renderDecisionsView`/`renderDeck`
   (research note §2B) with Approve, Reject and Snooze. **What the app gains later, not in this plan:**
   the "not sure this applies to you" marker in Today (direction note §5). That work follows C1b,
   C1c and C3′.
4. **`anatomy.md`:** one paragraph under Decisions naming `event-check` and what approve and reject
   write, and that a settled answer carries to the series' later instances (F2 decision 9).
5. **What an old reader does (review M-5).** An engine from before this plan that meets an
   `event-check` card warns `unknown kind` while it is approved (`approvals.rs:1406-1407`) and
   leaves it; when it is rejected, it archives the card through the generic path **without**
   writing the answer, so that `drop` is lost for good (the archived card still closes the
   question, so the event stays `unsure` and invisible, which is what `drop` would have shown
   anyway). The app and engine ship as a pair, so this is reachable only across C3 multi-device
   version skew. Accepted and named; no guard is added.

**Tests first** (`approvals.rs` `mod tests`):
- `approving_an_event_check_records_obligation_by_the_human_and_archives`: seed an `unsure` line with
  `jid` J, and a card like F2's. Mimic `decide_inner` with `write_literals(status=approved)` under
  `WriteContext::new("quinn","dashboard")`, then `process_approvals` under `default_ctx()`. Assert that
  `load_ledger` gives `verdict == "obligation"`, `answered_by == "quinn"` and `judgment_id == J`, that
  the card is in `archive/` with `status: executed`, and that `eventroster::relevant_events`
  now includes the event.
- `rejecting_an_event_check_records_drop`: likewise, and the event is not relevant.
- `every_instance_on_a_series_card_is_answered`: and each answer line carries that uid's own ledger
  title, never the card's `· +N more` title; a uid with no ledger title falls back to the card title,
  sanitised.
- `an_expired_or_snoozed_event_check_writes_nothing_to_the_ledger`.
- `a_ledger_write_failure_leaves_the_card_for_the_next_run`: `state/events-seen.md` is created as a
  **directory**. Assert the warning `transition failed: …` and that the card is still in `approvals/`
  with its status unchanged.
- `settling_twice_is_idempotent`.
- `a_card_with_no_journaled_human_decision_records_by_unknown`.

**Verify:** `cargo test -p knowlu-engine --lib -- approvals:: eventledger::`, then
`cargo test -p knowlu-engine --test oracle --test surface_oracle`.

---

## (b) `judgment_id`, end to end

**What exists.** Every `/judge-*` reply carries `judgment_id` whenever a `judgments` row was written
(`judge_pipeline.ts:154,233,271,290`), and the device throws it away: `cloudmodel.rs` `judge` and
`judge_event` never read it. `corrections.judgment_id`/`judgment_kind` exist (`20260911000100`).
Today they are filled only by the nightly `backfill_correction_judgments()`, which joins on
`judgments.item_id = corrections.item_id` (`20260911000500`). `calibration_query.sql` already joins
on `corrections.judgment_id`.

**Where that leaves each kind of evidence, and the decision for each.**

| evidence | reaches `corrections` today? | joined to its judgment today? | this plan |
|---|---|---|---|
| a human re-setting a field `enrich` wrote on a task (journal `set` over an `agent:` set) | yes, from `app/src/telemetry.rs` | yes, via the backfill (`item_id` = the note's `id:` = the judgment's `item.id`) | the note also keeps the id (F5), so an app-side join becomes possible later; **no app change** |
| an `amend` card from a judged write, rejected | no | no | the card keeps the id (F5), and the device reports the rejection (F8) |
| a Gmail-derived card, rejected | no | no | the card keeps the id (F6b) **in the vault only**. Under the Limited Use ruling (Global Constraint 14) the device does **not** report it; it waits for a per-user join |
| a field corrected on a Gmail-derived task note | yes (telemetry) | **no**: the judgment's `item_id` is the `message_id`, the correction's is the note id | the note keeps the id (F6b). Any join is per-user only (Global Constraint 14), needs `app/src/telemetry.rs`, and is **deferred** (see the fidelity ledger) |
| an `unsure` event answered on the card | no | no | the ledger and card keep the id (F1, F2, F4), and the device reports the answer (F8) as a **label**, not a calibration point |

**Where the join happens: on the service, keyed by an id the device sends.** The device posts
label rows that carry `judgment_id` to the existing `/telemetry` (class (b) under the terms; the
cloud design §6's (b) row already names "amend-card decisions … declined events", so **no new
consent**). `calibration_query.sql`'s existing `c.judgment_id = j.id` join then finds them with no
heuristic. Reason: the vault never leaves the device, so the service cannot see a decision unless it
is told. `/telemetry` already authenticates, deduplicates and refuses free text, so a new endpoint
would add a second copy of all of that.

### F4: the device keeps `judgment_id` (`judge.rs`, `cloudmodel.rs`, `events.rs`, `tests/cloud_contract.rs`)

**Decisions.**

1. **The structs.** `judge::Verdict` gains `judgment_id: Option<String>`. `judge::EventVerdict` gains
   the same and derives `Default`. `judge::merge` takes `upper`'s id. Existing struct literals
   (about 20 `Verdict`, 5 `EventVerdict`) gain `..Default::default()`.
2. **`cloudmodel::judgment_id_of(reply: &Value) -> Option<String>`** accepts only a lowercase UUID
   (the regex in F1), and anything else is `None`. Reason: the id is written unquoted into a ledger
   line and a flow mapping, so it must never carry a separator.
3. **Where it is set.** `judge()` sets it on the returned verdict. `judge_event()` sets it in the
   `Ok` arm **and** in the rescued-`unsure` arm, because the low-confidence reply carries it too
   (`judge_pipeline.ts:233,271`).
4. **`events::judge_roster`** writes through `record_judged_verdict(.., verdict.judgment_id.as_deref())`.
   Nothing else about the pass changes.

**Old and new pairings.**
- A **new engine with an old server** gets no `judgment_id`, which gives `None`. The ledger line has
  no `jid:` and is byte-identical to today's.
- An **old engine with the new server** is already true today: the id is sent and ignored.

**Tests first.**
- `cloud_contract.rs`: `an_event_reply_hands_its_judgment_id_to_the_verdict`,
  `a_rescued_unsure_keeps_the_judgment_id`, `a_task_reply_hands_its_judgment_id_to_the_verdict`,
  `a_reply_without_a_judgment_id_is_none`, and `a_malformed_judgment_id_is_dropped` (`"judgment-1"` and
  `"../x"` both give `None`).
- `events.rs`: `judge_roster_writes_the_jid_onto_the_ledger_line`, plus a byte check with a
  scripted `EventModel` that returns an id, and `judge_roster_without_an_id_writes_todays_bytes`.

**Verify:** `cargo test -p knowlu-engine --test cloud_contract`, then `cargo test -p knowlu-engine --lib -- events:: judge::`.

### F5: task notes and amend cards keep it (`engine/src/enrich.rs`, `engine/src/write.rs`)

**Decisions.**

1. **`enrich`** adds `judgment_id` and `judgment_kind: task` (both, or neither) to the `inputs`
   mapping it already builds (`enrich.rs:257-259`) whenever the verdict it writes carries an id.
   `write::write_literals` flattens `inputs` into the `judgment:` flow mapping, so there is **no new
   frontmatter key on the task note and no change to `provenance::judgment_value`**.
2. **`write::propose_amendment`** (`write.rs:622`) gains a parameter
   `judgment: Option<(&str, &str)>`, filled at its one call site (`write.rs:354`) from
   `opts.inputs`'s `judgment_id` and `judgment_kind`. When it is present, the card's frontmatter gets
   `judgment_id:` and `judgment_kind:` after `created_by`.

**Byte-contract statement.**
- A judged write with no id is **byte-identical** to today: the same `judgment:` literal and the same
  journal `set judgment` record.
- With an id, the block reads
  `{actor: …, at: …, fields: […], inputs: {judgment_id: <uuid>, judgment_kind: task, source_uid: …, title_seen: …}, run_id: …}`
  (sorted keys, one line).
- **Old readers** of `judgment:` are `passes::heal`, which re-applies the whole journaled literal and
  is key-agnostic, and `issues.rs`, which snapshots it whole. Neither reads `inputs` by key, so the
  extra key is ignored.
- `validate_amendment` and `surface` read named card keys only, so the two extra card keys are
  ignored.

**Tests first.**
- `enrich.rs`: `a_cloud_judgment_with_an_id_stamps_it_into_the_judgment_block`, and
  `a_judgment_without_an_id_writes_the_block_exactly_as_before`, a byte comparison against the
  current test's expected literal.
- `write.rs`: `an_amend_card_from_a_judged_write_carries_judgment_id_and_kind` and
  `an_amend_card_without_one_is_byte_identical`.

**Verify:** `cargo test -p knowlu-engine --lib -- enrich:: write:: passes::`.

### F6a: the Gmail queue returns the id (`cloud/supabase/functions/gmail-read/`)

**Decisions.**
- `index.ts`'s `undelivered` select (`index.ts:141`, `select=uid,tier,payload`) adds `judgment_id`.
  The column is already written by `enqueue` (`index.ts:132-135`).
- `handler.ts`'s item type (`handler.ts:177`) and `forDevice` carry `judgment_id?: string | null`
  through unchanged, including when a declared-only tier is rewritten to `information`.
- No migration.
- An old engine ignores the extra key: `cloudmodel.rs`'s pull parser reads named keys only.

**Tests first** (`handler_test.ts`):
- `an undelivered row's judgment_id reaches the device`.
- `forDevice keeps judgment_id when it rewrites a tier`.

**Verify:** `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/functions/gmail-read/`
and `deno check --config cloud/supabase/deno.json cloud/supabase/functions/gmail-read/*.ts`.

### F6b: Gmail notes and cards keep it (`engine/src/cloudmodel.rs`, `engine/src/enrich.rs`, `engine/src/completion.rs`)

**Runs after F11** (wave 4; F11 is wave 1). Both edit `completion.rs`: F11 changes `already_proposed`
and adds `target_id` to `propose_done`'s card, and F6b then adds one parameter to `propose_done`.
They are never edited in parallel (review I-2).

**Decisions.**
- `cloudmodel::GmailItem` gains `judgment_id: Option<String>`, parsed with F4's `judgment_id_of`
  applied to the row.
- `enrich.rs`'s `write_gmail_note` and `write_gmail_card` put `judgment_id:` and
  `judgment_kind: email` into the frontmatter they already emit, when the id is present.
- **Completion cards are built in `completion.rs`, not `enrich.rs` (review I-2).**
  `propose_gmail_completion` (`enrich.rs:791-813`) emits no frontmatter; it delegates to
  `completion::propose_done` (`enrich.rs:807`), which builds the card (`completion.rs:219-240`). So:
  - `propose_done` gains a last parameter `judgment: Option<(&str, &str)>` (`(judgment_id,
    judgment_kind)`). When `Some`, the card gets `judgment_id:` and `judgment_kind:` directly after
    `created_by`, the same place F5 puts them on an amend card.
  - **Its callers, all of them:** `enrich.rs:807` (`propose_gmail_completion`) passes
    `item.judgment_id.as_deref().map(|id| (id, "email"))`; `completion.rs:292`
    (`propose_vendor_completions`, tier 1) passes `None`; the two test calls at
    `completion.rs:646,648` pass `None`.
  - **`coursework.rs` does not change.** It never calls `propose_done`; it calls
    `propose_vendor_completions` (`coursework.rs:1663`), whose signature is unchanged.
- **These ids stay in the vault (Global Constraint 14).** F8 never reports an `email`-kind card; the
  id is stamped only so a later **per-user** join has it.
- **What an old reader does:** a task note's loader (`models`) and the ranking read named keys, so an
  unknown frontmatter key on a note is ignored exactly as `source_uid` was before it was used.
  `validate_amendment` and `already_proposed` read named card keys, so the two extra keys on a
  completion card are ignored too.

**Tests first.**
- `enrich.rs`: `a_pulled_task_note_carries_the_email_judgment_id`, `a_pulled_card_carries_it`,
  `a_gmail_completion_card_carries_the_email_judgment_id`, and `an_item_without_one_writes_todays_bytes`.
- `completion.rs`: `a_vendor_completion_card_carries_no_judgment_keys` (tier 1 passes `None`; the
  card's bytes equal F11's expected card), and `the_judged_completion_card_still_validates`
  (`validate_amendment` stays `Ok` with both keys present).

**Verify:** `cargo test -p knowlu-engine --lib -- enrich:: completion:: coursework::`, then
`cargo test -p knowlu-engine --test cloud_contract`.

### F7: the service accepts labels, and the calibration query reads them

Files: `telemetry/handler.ts`, `telemetry/handler_test.ts`, `telemetry/index.ts` (the ownership
lookup, decision 6), `cloud/eval/calibration_query.sql`, `cloud/eval/calibration_cli_test.ts`.

**Decisions.**

1. **`CorrectionIn` gains optional `judgment_id` and `judgment_kind`.** A present `judgment_id` must be
   a UUID and `judgment_kind` must be one of `task`/`event`/`email`, or the batch is refused with 400.
   This is the same "not free text" discipline as `isToken`.
2. **`LABEL_FIELDS = ["verdict", "decision"]`** is a third list next to `VALUED_FIELDS` and
   `FLAGGED_FIELDS`, accepted **only on a row that carries a `judgment_id`**.
   - A `verdict` row requires `judgment_kind: "event"` and `ours`/`theirs` in `EVENT_VERDICTS`
     (imported from `judge_validate.ts`).
   - A `decision` row requires `ours: "proposed"`, `theirs` in `["approved","rejected"]`, and
     `judgment_kind` in `["task","event"]`.
   - **A label row with `judgment_kind: "email"` is refused with 400** (Global Constraint 14). F8
     never sends one; this makes a future device bug a refused batch, not a Limited Use breach.
   - **A `judgment_id` is accepted only on a `LABEL_FIELDS` row (review M-10).** A `VALUED_FIELDS`
     or `FLAGGED_FIELDS` row that carries one is refused with 400. Nothing sends that shape today
     (the app's rows never name the column, and task field corrections keep the server backfill),
     so refusing it keeps decision 3's two-call split exact and keeps the deferred email join from
     arriving by accident.
   - Reason: these fields have closed vocabularies and ride only on a named judgment, so a sentence
     has nowhere to hide.
3. **Rows with and without a `judgment_id` are saved in two separate `saveCorrections` calls.**
   Reason: PostgREST's `merge-duplicates` upsert sets every column named in the payload. Today's rows
   never name `judgment_id`, so today an app row that is re-sent after the nightly backfill filled
   its `judgment_id` keeps that id. If one call mixed the two shapes, the plain rows would carry
   `judgment_id: null` and a re-send would **erase a backfilled id**. The test pins this.
4. **`calibration_query.sql`:**
   - Add `and coalesce(j.fields ->> 'verdict', '') <> 'unsure'`. An abstention claims nothing, so it
     is not a calibration point. Its answered label is kept in `corrections` for the event work that
     follows.
   - **The `wrong` subquery gains `and c.account_id = j.account_id`** (review I-5, second layer), so
     a correction can only mark its own account's judgment wrong even if a foreign id were ever
     saved.
   - **The `wrong` subquery gains `and not (c.field in ('verdict','decision') and c.judgment_kind =
     'email')`** (Global Constraint 14): even a stray `email` label row can never reach the
     cross-account aggregate.
   - Narrow `wrong` to exclude `c.field = 'decision' and c.theirs = 'approved'`, a defensive clause
     since F8 never sends approvals. The header comment gains a paragraph listing the two
     device-reported row shapes.
   - **No new column**, so `CalibrationRow` and the harness are untouched.
5. **What the promotion job does with them.** `promote_rules()`'s contradiction guard joins on
   `corrections.judgment_id`. A rejected judged card or an answered `unsure` therefore now blocks
   promoting a rule that would repeat that judgment. That is correct: it is exactly the evidence the
   guard exists for. It is stated in the commit message. `promote_rules()`' `contradicted` CTE
   (`20260911000500_rule_promotion_fix.sql:187-189`) has **no** `c.account_id = j.account_id`
   clause, and this plan adds no migration, so on that side the account boundary rests on
   decision 6's handler check alone. Accepted until a migration can add the clause (listed in the
   fidelity ledger as deferred). `email` label rows cannot reach it because the handler refuses them
   (decision 2).
6. **The handler checks that each `judgment_id` belongs to the caller (review I-5, first layer).**
   Until now only the server backfill wrote `corrections.judgment_id`; from F7 the device supplies
   it, which is a new trust boundary.
   - `Deps` gains `ownedJudgments: (accountId: string, ids: string[]) => Promise<Set<string>>`.
     `index.ts` implements it with one `restSelect` on `judgments`
     (`select=id&account_id=eq.<user.id>&id=in.(<ids>)`, the ids already UUID-checked by decision 1),
     called once per batch and only when the batch holds an id-bearing row.
   - A row whose `judgment_id` is not in the returned set is **dropped with a named count**, not
     400ed: the response gains `"unowned": N` beside `events`/`corrections`, and the row is not
     saved. Reason: a judgment can legitimately vanish (retention, account deletion); a 400 would
     make F8 retry the same batch forever, while a drop lets the good rows land and says how many
     did not.
   - A lookup failure throws, so the batch is a 5xx, F8 stamps nothing, and it retries next slot.

**Old and new pairings.**
- An **old handler with the new engine** 400s a batch holding a `verdict`/`decision` row. F8 treats
  that as "not sent", stamps nothing and retries next slot. **Deploy F7 before shipping F8** (see
  Rollout).
- A **new handler with the old app** changes nothing for that app's rows.

**Tests first.**
- `handler_test.ts`:
  - `a verdict label with a judgment_id is saved with it`;
  - `a label field without a judgment_id is refused`;
  - `a verdict outside the event vocabulary is refused`;
  - `a malformed judgment_id is refused`;
  - `plain rows and id-bearing rows reach saveCorrections in separate calls, and plain rows carry no judgment_id key`;
  - `today's correction rows are saved exactly as before`, a deep-equal against the current shape,
    and `ownedJudgments` is never called for a batch with no id-bearing row;
  - `an email label row is refused`;
  - `a judgment_id on a valued or flagged field is refused`;
  - `a judgment_id the caller does not own is dropped and counted`: a fake `ownedJudgments` returns
    one of two ids; one row is saved, the response says `"unowned": 1`;
  - `an ownership lookup failure saves nothing`.
- `calibration_cli_test.ts`: `the calibration query excludes unsure verdicts, approved decisions and
  email labels, and joins on the account`, a static pin that reads the `.sql` file and asserts each
  of the four clauses (including `c.account_id = j.account_id`), in the same way
  `app/tests/telemetry.rs` pins `ACTIONS`.

**Verify** (the `--allow-write=cloud/eval` is required: `calibration_cli_test.ts` builds temp dirs
under `cloud/eval/`, as `ci.yml`'s `deno test` step already grants; review M-1):
`deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/functions/telemetry/ cloud/eval/calibration_cli_test.ts`
and `deno check --config cloud/supabase/deno.json cloud/supabase/functions/telemetry/*.ts cloud/eval/*.ts`.

### F8: the device reports decisions (`engine/src/enrich.rs`, `engine/src/cloudmodel.rs`, `engine/tests/cloud_contract.rs`, one `CLAUDE.md` line)

**Decisions.**

1. **`enrich::report_labels(vault, client, opts, budget) -> Vec<String>`** is the fifth pass of the
   cloud arm, after `pull_rules`. It follows `pull_rules`' budget discipline: it checks
   `elapsed + CALL_TIMEOUT > budget` before the request. `labels_waiting(vault)` joins the early-return
   predicate (`enrich.rs:374`), so a vault with only a decision to report still reaches it.
   `cloudmodel::post_labels(client, rows) -> Result<usize, CloudError>` POSTs
   `{"events": [], "corrections": rows}` to `/telemetry`.
2. **What is reported.** A card is reported when it is in `approvals/` or `archive/`, is
   `type: approval`, carries a valid `judgment_id` and `judgment_kind`, and has no `reported_at`.
   - An **`event-check`** card whose `status` is `executed` or `rejected` gives a row with
     `field: "verdict"`, `ours: "unsure"`, `theirs: "obligation"` or `"drop"`, and
     `judgment_kind: "event"`.
   - **Any other kind** whose `status` is `rejected` gives a row with `field: "decision"`,
     `ours: "proposed"`, `theirs: "rejected"`, and the card's own `judgment_kind`.
   - **No `email`-kind card is ever reported (Global Constraint 14, controller ruling on review
     I-4).** A card whose `judgment_kind` is `email` (a Gmail-derived task card or a Gmail
     completion card, F6b) is skipped by `labels_waiting` and `report_labels` alike: it is never a
     row, never stamped `reported_at`, and never re-read as "waiting". Gmail-derived signals train
     only that user's own model; that per-user path is later work, and the id stays on the card for
     it.
   - **Approvals of non-event cards are not sent.** An approved judgment stays right by absence,
     which is the query's standing reading.
   - Every row has `item_id: <card id:>` and `kind: "approval"`. Its `ts` is the `ts` of
     `journal.human_set(<card id>, "status")`.
   - **A card with no human journal record is not reported.** There is no truthful `ts` to give it,
     and re-scanning it costs one frontmatter read a slot.
3. **Idempotence.** On a 200, each reported card gets `reported_at: "<UTC ISO>"` through
   `write_literals`, under actor `agent:knowlu.labels` and `opts.via`. Any other outcome is one line
   (`labels: not sent (…)`), stamps nothing, and is retried next slot. A duplicate send lands on the
   same `corrections_once` key. Batches hold at most 100 rows, in `(ts, item_id)` order.
   A 200 whose body carries `"unowned": N > 0` (F7 decision 6) still stamps every card in the batch,
   because the service has answered for them and a resend would be dropped again, and adds one line
   `labels: sent K, N not accepted (judgment not on this account)`.
4. **Exit code.** `judge` still always exits 0.
5. **One doc line.** `CLAUDE.md` says `run_lines_with` "hosts the four cloud pulls". F8 makes it five.
   Update that sentence in the F8 commit (a docs-only line in a file no live branch owns).

**Tests first** (`cloud_contract.rs`, loopback only):
- `a_settled_event_check_is_reported_once_with_its_judgment_id`: checks the exact request body
  JSON and `reported_at` stamped, and a second call makes no request.
- `a_rejected_judged_amend_card_is_reported_as_a_decision`.
- `an_approved_amend_card_is_not_reported`.
- `a_card_without_a_judgment_id_or_without_a_human_decision_is_not_reported`.
- `a_rejected_email_kind_card_is_never_reported_or_stamped`: a rejected Gmail task card and a
  rejected Gmail completion card, both with `judgment_kind: email`. No request is made, neither card
  gets `reported_at`, and `labels_waiting` is false for a vault holding only them.
- `an_unowned_count_is_named_and_the_batch_is_still_stamped`.
- `a_refused_batch_stamps_nothing_and_says_so`: 400, one `labels: not sent` line, no stamp.
- `nothing_waiting_means_no_request`.
- `the_probe_fires_when_only_a_label_is_waiting`: the early-return predicate.

**Verify:** `cargo test -p knowlu-engine --test cloud_contract`, then `cargo test -p knowlu-engine --lib -- enrich::`.

---

## (c) The event feed pagination bug

**Where the fetch is.** `eventfeed::load_discovered_events` (`eventfeed.rs:414`) calls `fetch(&source.url)`
**once per source** and parses one payload. `rank` calls it through `Fetchers.events` (`cli.rs:355`),
and `judge` through `judge_roster` (`events.rs:393`). For a cloud vault, `fetch` is
`cloudmodel::fetch_event_source`, a URL-agnostic `POST /events` proxy (`events/handler.ts`: any
public https URL), with the on-device `eventfeed::fetch_event_source` as the fallback. The URLs come
from the vault's `config/events.yaml` `sources:` (`events::load_events_config`).

**The evidence, with no network call.** The collector that found the bug
(`.superpowers/.../labelling/collector/src/main.rs:21-41`) records that "the configured
Localist/Engage URLs carry no query". It widened them with Localist `days=30&pp=100&page=N`, and
with Engage `endsAfter=<UTC>&orderByField=endsOn&orderByDirection=ascending&status=Approved&take=100&skip=N*100`.
Through the engine's own parsers that returned 462 and 176 events. The parsers already read the
shapes involved. Localist's top level is `events` (`parse_localist`), and its `page` object is
ignored today. Engage's is `value` with `@odata.count` (see the test fixture `engage()`,
`eventfeed.rs:603`).

### F9: paging the two JSON feeds (`engine/src/eventfeed.rs`)

**Decisions.**

1. **Paging is keyed on `source.kind`**, not on the URL's path. `ics` and `html` are fetched once,
   exactly as today. Reason: the kind is the vault's declaration of what the URL is, and the
   collector's path-sniffing would miss a campus that proxies the API.
2. **The URL is built deterministically.**
   - The engine owns these keys:
     - Localist: `days`, `pp`, `page`.
     - Engage: `endsAfter`, `orderByField`, `orderByDirection`, `status`, `take`, `skip`.
   - Any of those keys already in the configured query are removed. Every other key is kept in its
     original order, and the engine's keys are appended in the fixed order listed above.
   - Nothing is percent-decoded or re-encoded.
   - Localist `days = clamp(roster_window_days, 1, 365)`, which is 90 by default, because the roster
     shows 90 days.
   - Engage `endsAfter` = `now` in UTC as `%Y-%m-%dT%H:%M:%SZ`. That is the collector's proven
     format.
   - `PAGE_SIZE = 100` (the `pp`/`take`).
3. **When paging stops.**
   - **Localist** stops at `page >= payload.page.total`. If there is no `page.total`, it stops when a
     page holds fewer than `pp` events.
   - **Engage** stops at `skip + take >= @odata.count`. If there is no count, it stops when `value`
     holds fewer than `take`.
   - **Both** stop on an empty page.
   - **Hard caps:** `MAX_PAGES = 20` per source per call, which is also a cap of 2,000 events.
4. **Failures.**
   - **Page 1 failing** gives exactly today's `"{name}: fetch failed ({err})"` or
     `"{name}: parse failed ({err})"`. Reason: that string reaches the runner log, and the
     `cli.rs` test at `:960` pins its prefix (the assertion is at `cli.rs:974`).
   - **A later page failing** keeps what was already parsed and warns
     `"{name}: page {n} failed ({err}); kept {k} events"`.
   - **Hitting the cap** warns once: `"{name}: stopped after 20 pages; later events wait for the window to move"`.
5. **Merge.** Each page is parsed by the existing `parse_localist`/`parse_engage`. Events are
   deduplicated by uid across pages and sources with the existing first-wins `seen` set, and
   `sort_events` runs once at the end. The output order is `(start, uid)` whatever order the pages
   arrived in.
6. **The clocks are injected.**
   - New: `load_discovered_events_at(vault, fetcher, now: jiff::Timestamp, limits: PagingLimits,
     elapsed: &dyn Fn() -> Duration)`. `now` builds `endsAfter`; `elapsed` is wall-clock time since
     the call began (production: a closure over one `Instant`; tests: a `Cell<Duration>` the fake
     fetcher advances).
   - `load_discovered_events` keeps its signature and delegates with `Timestamp::now()`,
     `PagingLimits::DEFAULT` and a real `Instant`, so `cli.rs` does not change. `judge_roster`'s
     call moves to `load_discovered_events_at` in F10, not here.
7. **A wall-clock budget on paging (review I-3).** Each page can take up to 150 s on a slow but
   live network: 120 s through the proxy (`cloudmodel::CALL_TIMEOUT`, `cloudmodel.rs:42`) plus 30 s
   for the direct fallback (`eventfeed.rs:40`, `cli.rs:236-237`). Twenty pages on two sources is
   then 100 minutes against `scheduler::CHILD_TIMEOUT` of 20 minutes per step
   (`app/src/scheduler.rs:27`), and a killed `rank` writes no `today.md`.
   - `PagingLimits { per_source: Duration, per_run: Duration }`, with
     `PagingLimits::DEFAULT = { per_source: 90 s, per_run: 180 s }`.
   - **Checked before each fetch, never mid-request**, the same "check before starting" rule as
     `enrich::BATCH_BUDGET` and `judge_roster`'s budget. Before page `n > 1` of a source: stop that
     source if its own paging has run `>= per_source`, or if the whole call has run `>= per_run`.
     Before page 1 of a source: skip the source only if the whole call has run `>= per_run`.
   - **The values, justified against the 20-minute step limit.** The worst case is the budget plus
     one page already in flight: `180 s + 150 s = 330 s`, 5.5 minutes, which leaves `rank` about
     14.5 minutes of its 20 for the calendar fetch and everything else, the same order of headroom
     `BATCH_BUDGET` leaves `judge` (17 of 20). At a normal 1-3 s a page, 90 s per source is 30 or
     more pages, well above the 14 Localist pages a 90-day window needs, so the budget only bites on
     a network that is already failing. It is one constant each.
   - **On the deadline, keep what was fetched and name the truncation** as a warning, exactly as a
     failed later page is named. The warnings reach `rank`'s run record through `event_warnings`
     (the `events` step turns `WARN` with the line in its message, `cli.rs:575-577`, and
     `first_with_count("events", …)` puts it in `problems`) and `judge`'s output as `events: …`:
     - `"{name}: stopped at page {n} (time budget); kept {k} events"` (per source);
     - `"{name}: stopped at page {n} (run time budget); kept {k} events"` (per run, mid-source);
     - `"{name}: skipped (run time budget)"` (per run, before page 1). A skipped source behaves as a
       failed one does today: `rank`'s `feeds_failed` fallback to the last roster still applies when
       nothing at all was fetched.
   - Events kept from a truncated source are merged, deduplicated and sorted like any other.
8. **Cost, stated.**
   - A 90-day window on a campus of this size is about 14 Localist pages and 2 to 5 Engage pages.
   - That is fetched in both `rank` and `judge`, in two slots a day: about 80 `/events` proxy calls
     a day, against 8 today. These are transport calls with no model behind them.
   - The judgment cost is bounded by `judge_per_run_cap` (150) and the service's `DAILY_CAP.event = 80`
     (`judge_caps.ts:36`).
   - The first week works through the backlog soonest-first. Steady state is new events only: 462
     a month is about 15 a day.
   - F10 removes the ones `rank` would never show.

**The frozen fixture.** `vault-full/config/events.yaml` uses `unreachable://` URLs, so page 1 fails
in the URL parser before any socket opens. The warning shape is unchanged, and the roster comes from
`state/events.md` as before. `golden-today-full.md` does not carry the warning text, and neither
does `surface-today-full.json` (verified: neither contains `fetch failed`).

**Tests first** (`eventfeed.rs` `mod tests`, with a `RefCell` URL-recording fake fetcher and no network):
- `localist_pages_until_page_total`: 3 pages. Asserts the three exact URLs, merged and sorted
  events, and no warning.
- `engage_pages_by_skip_until_the_count`: `@odata.count` 250 gives `skip=0,100,200`, and `endsAfter`
  is the injected instant.
- `paging_stops_at_the_page_cap_and_warns_once`.
- `an_existing_query_is_kept_and_the_engines_keys_replaced`: `?group=x&pp=10` becomes
  `?group=x&days=90&pp=100&page=1`.
- `a_failed_later_page_keeps_earlier_pages_and_warns`.
- `a_first_page_failure_warns_exactly_as_before`.
- `a_short_page_without_a_total_ends_paging`.
- `a_duplicate_uid_across_pages_collapses_first_wins`.
- `ics_and_html_sources_are_fetched_once_with_the_url_unchanged`.
- `paging_stops_at_the_per_source_time_budget_and_keeps_what_it_has`: the fake fetcher advances the
  injected clock 40 s a page; with `per_source` 90 s, pages 1-3 are fetched (checks at 0, 40, 80 s),
  page 4 is not, the three pages' events are kept, and the warning is
  `"campus: stopped at page 4 (time budget); kept 300 events"`.
- `the_run_time_budget_skips_a_later_source_and_names_it`: two sources, the first spends past
  `per_run`; the second is never fetched and warns `"<name>: skipped (run time budget)"`, and the
  first source's events are kept.
- `the_default_limits_are_90_and_180_seconds`: pins `PagingLimits::DEFAULT`, with a comment citing
  the 20-minute `CHILD_TIMEOUT` arithmetic above.

**Verify:** `cargo test -p knowlu-engine --lib -- eventfeed:: cli::`, then `cargo test -p knowlu-engine --test oracle --test surface_oracle`.

### F10: judge only what `rank` would keep (`engine/src/events.rs`)

**Decision.** `judge_roster` runs `eventfilter::prefilter_events(&discovered, &interests, &config, today)`
(with interests from `events::load_interests`) and chooses `pending` from the survivors only.
Reason: before F9, `judge` paid for every discovered event, including events past the roster window,
standing exhibits, `never` interests and staff-only audiences, all of which `rank` drops unjudged
(`cli.rs:363`). F9 multiplies that waste by about 30. The filtered events still appear in the
roster's audit section as `· filtered`, exactly as today.

**Two further `judge_roster` changes, both in `events.rs`:**

1. **Paging spends the events pass's budget, not extra time (review I-3, the `judge` side).**
   `started` moves above the feed load, and the load becomes
   `load_discovered_events_at(vault, fetch, Timestamp::now(), PagingLimits { per_source: 90 s,
   per_run: min(180 s, budget) }, <elapsed since started>)`. The item loop's existing
   `started.elapsed() >= budget` check then counts the paging time too. So the cloud arm's bound is
   unchanged: `BATCH_BUDGET` (15 min) plus one call in flight, inside `CHILD_TIMEOUT`. Truncation
   warnings reach the output as `events: …` lines, like every feed warning today.
2. **A new instance of a settled series is not judged (review I-1, the `judge` side).** `pending`
   also excludes an event whose `series_uid` is in `eventemit::settled_series(vault)` (F2
   decision 9). `rank` gives it the student's answer on its next run, so paying the model for it
   would be waste, and a confident machine verdict written first would block the student's answer
   (F1's supersede-`unsure`-only rule).

**Tests first.**
- `judge_roster_does_not_pay_for_an_event_rank_would_filter`: four invented events, one per
  prefilter rule. A recording model is never called, and no ledger line is written for any of them.
- `judge_roster_still_judges_a_survivor`.
- `judge_roster_does_not_judge_a_new_instance_of_a_settled_series`: an archived `executed`
  event-check card naming the series; the recording model is never called for the new instance.
- `judge_roster_paging_time_counts_against_its_budget`: a 20 ms `budget` and a fetcher that sleeps
  30 ms, the pattern of the existing
  `the_events_pass_stops_at_its_wall_clock_budget_and_reports_the_remainder`; no item is judged, the remainder line names every pending event, and the
  truncation warning appears as an `events:` line.
- Every existing `events.rs` `judge_roster` test keeps its assertions. If one fails only because its
  invented event falls outside the 90-day window from its `today`, move the event's date inside the
  window. Record each such move in the commit message. Never weaken an assertion.

**Verify:** `cargo test -p knowlu-engine --lib -- events:: eventfilter:: eventemit::`.

---

## (d) Two minors due before term rollover

### F11: completion keyed on the note's `id:` (`engine/src/completion.rs`)

**The defect, verified.** `already_proposed(vault, target_rel)` (`completion.rs`, the function of that
name) matches a card when `target == target_rel`, a vault-relative **path**. At rollover a new term's
`tasks/hw-01.md` can reuse a path whose archived card belongs to last term's note, so the new task is
never proposed. A renamed note would be proposed twice.

**Decisions.**
1. `propose_done` writes `target_id: <note id>` on the card when the note has an `id:`. `target` keeps
   the path, because `validate_amendment`/`resolve_amend_target` need a path to apply the change.
2. `already_proposed(vault, target_rel, target_id: Option<&str>)` matches a card:
   - by `target_id` when both the card and the note have one;
   - by path only when the card has no `target_id`. That covers legacy cards, of which none have
     shipped, because T8 is unreleased;
   - by path when the note has no `id`.

   Reason: `id:` is the vault's opaque identity (CLAUDE.md), and paths are not.
3. `source_uid` is **not** used as the key. A vendor uid is the external key of the *task*, and the
   card's identity is the note it amends.

**Tests first.**
- `a_new_task_at_a_reused_path_is_still_proposed`: an archived card with `target_id` A, and a new
  note at the same path with id B. One card is filed.
- `a_renamed_task_is_not_proposed_twice`: the card has `target_id` A, and the note has moved to a new
  stem with the same id. The result is `Skipped("already proposed")`.
- `a_legacy_card_without_target_id_still_matches_by_path`.
- `the_card_carries_target_id_and_still_validates`: extends the existing
  `the_card_is_a_status_amend…`, and `validate_amendment` stays `Ok`.
- All existing `completion.rs` tests pass unchanged.

**Shared file.** F6b edits `completion.rs` after this task lands (it adds a `judgment` parameter to
`propose_done`). F11 must be merged before F6b starts, and F11 does not touch `propose_done`'s
parameter list.

**Verify:** `cargo test -p knowlu-engine --lib -- completion:: approvals::`.

### F12: `MIN_CLASS_N` justified against the calibration plan (`cloud/eval/calibration.ts`, `calibration_test.ts`)

**What T6 meant.** The judgment-quality plan's T6 gives one small-sample number: "A bin under about
thirty items is decoration: at three-quarters accuracy its standard error is near nine points." It
sets the AUROC bands at ≥0.80 usable and <0.70 cosmetic. `calibration.ts:189-194` says of its own
`MIN_CLASS_N = 10` that it is "the harness's own floor, not a number from the brief", chosen so that
"a synthetic test with a few dozen rows still gets a real answer". That reason is about the tests, not
about the measurement.

**The arithmetic.** These are Hanley–McNeil standard errors at AUROC 0.75, with equal classes:

| per class | SE | 95% half-width |
|---|---|---|
| 10 | 0.112 | ±0.22 |
| 30 | 0.063 | ±0.12 |
| 50 | 0.049 | ±0.10 |

At 10 per class the interval covers both bands entirely, so a grade would be noise with a label on it.

**Decision: `MIN_CLASS_N = 30`.** It borrows T6's small-sample floor. Above it the reported
`ci95` is narrow enough that Quinn, reading B2, can see which band it leans to.
- **What T6's sentence is about (review M-6).** T6's "under about thirty items is decoration"
  (`2026-09-22-judgment-quality-plan.md:263`) is about **calibration bins**, not AUROC class sizes.
  The comment says so: it borrows that floor as a round number, and the justification for applying
  it to AUROC is the Hanley–McNeil table, not T6.
- The comment is replaced with that citation (plan T6, the sentence quoted above, named as a bin
  floor that is borrowed) and the table.
- It states plainly that even 30 per class leaves the 0.70 and 0.80 bands within one interval, which
  is why B2 is a look at the number and its CI, not an automatic verdict.
- **The verdict logic is not changed.** Changing it would be T5/T7's business.

**Tests first.**
- `calibration_test.ts`: `MIN_CLASS_N is thirty, the T6 floor`, which asserts
  `assertEquals(MIN_CLASS_N, 30)` and replaces the `10` pin at line 212.
- `29 in a class is undefined and 30 is graded`, a boundary test.
- Every existing test that grades `usable`/`marginal`/`cosmetic` gets synthetic rows raised to at
  least 30 per class, with its expected verdicts unchanged. `rowsFromRanks` already builds any size.

**Verify:** `deno test --allow-read --allow-write=cloud/eval --config cloud/supabase/deno.json cloud/eval/calibration_test.ts cloud/eval/calibration_cli_test.ts`
(`--allow-write=cloud/eval` because `calibration_cli_test.ts` builds temp dirs there; review M-1).

---

## Rollout (the stream-J discipline)

1. **Deploy `telemetry` (F7) before any engine that carries F8 reaches a device.** The old handler 400s a
   label batch. That is harmless, because F8 retries and stamps nothing, but it is noisy.
2. **`gmail-read` (F6a) can deploy in either order.** An old engine ignores `judgment_id`, and a new
   engine with an old queue gets `None`.
3. **The engine's changes need no server change** for (a), (c) and (d). F4's capture works against
   the server that is already live.
4. **No migration.** Nothing touches Supabase staging or production from a subagent; deploys are the
   controller's.
5. **Before merge:** `cargo test --workspace` at 0 warnings (the accepted `.rsrc` line only), then the
   Deno check, lint and test run CI uses (`.github/workflows/ci.yml:73-89`). CI minutes are stopped
   (memory, 09-23), so verify locally.

## Decisions taken, with their cost if wrong

| decision | cost if wrong |
|---|---|
| The card is filed by `rank`, not `judge` | none structural. A move is one call site |
| A human answer supersedes `unsure` only | a later "correct a confident verdict" feature widens one `if` in `load_ledger` |
| One card per series, 3 cards a day, asked within `propose_horizon_days` | a constant or a config key; nothing stored changes |
| An expired card leaves the event `unsure` forever | an obligation the student ignored stays invisible. The same as today, never worse |
| Labels travel over `/telemetry`, not a new endpoint | one handler file. A new endpoint later would reuse the same row shape |
| Non-event approvals are not reported | calibration keeps "right by absence". Adding `approved` rows later is one filter in F8 |
| Task field corrections keep the server backfill; the email-note join is deferred to the app | email-kind calibration pairs from field edits wait for an `app/src/telemetry.rs` change |
| Localist `days` = the roster window, 20-page cap | a slower `rank` on a very large campus. It is one constant |
| `judge` prefilters like `rank` | an event `rank` filters could never be judged. That is already true for what the student sees |
| `MIN_CLASS_N = 30` | the first real B2 read says `insufficient_data` for longer (about 30 wrong judgments per kind and prompt) |
| A settled series card answers the series' later `unsure`/unjudged instances, in `rank` (F2 d9, F10) | a student who meant "this one, not the series" must reject later instances by hand; one function to narrow |
| Inherited answer lines carry the instance's own `jid`, not the card's | none for calibration: labels come from cards (F8), never from ledger lines |
| Paging budget 90 s per source, 180 s per call, checked before each fetch | a slow network loses late pages for a run; the warning names it and the next run re-fetches. Two constants |
| No `email`-kind label row is reported, and the handler refuses one (controller ruling on I-4) | email-kind calibration waits for a per-user path; the ids are already on the Gmail notes and cards |
| An unowned `judgment_id` is dropped with a named count, not 400ed (I-5) | a forged id is silently discarded instead of failing loudly; the count is in the reply and the log |
| `promote_rules`' account clause waits for a migration; the handler check guards it meanwhile (I-5) | if the handler check were bypassed, a foreign correction could block another account's rule promotion (not grant one) |
| A `judgment_id` is refused on valued/flagged fields (M-10) | the deferred email field-correction join, if ever allowed per-user, needs one handler rule relaxed |
| Event-check cards write `created_by: events`, like the digest (M-3) | none; a literal |
| The daily ceiling of 3 counts `first_proposed_at` (M-4) | none; it is the field that never moves |

**Review Minors rejected:** none. M-1 to M-10 were each judged correct and folded in (see "Plan
review (2026-09-23)").

## Fidelity ledger

| item | task | what proves it |
|---|---|---|
| (a) an `unsure` event files a "Does this apply to you?" card | F2 | `an_unsure_event_in_the_horizon_files_one_event_check_card` |
| (a) the what-and-when title comes from structured fields | F2 | `what_and_when_formats_every_shape` |
| (a) the 15-a-day cap, overflow never created, `first_proposed_at` set | F2 | `the_emitter_respects_the_budget_and_the_daily_ceiling`; the card test's date asserts |
| (a) approve → obligation, reject → drop, the human as actor | F1, F3 | `approving_…_records_obligation_by_the_human_and_archives`, `rejecting_…_records_drop`, `a_human_answer_replaces_unsure` |
| (a) resolved by the existing `decide` path, no app change | F3 | the approve test drives `write_literals` and then `process_approvals`, exactly as `decide_inner` does |
| (a) never re-ask an answered event | F1, F2 | `a_second_run_files_nothing`, `nothing_is_asked_after_a_card_is_archived…`, `the_first_human_answer_wins` |
| (a) an answered series stays answered as new instances arrive (I-1) | F2, F10 | `a_later_instance_of_an_answered_series_inherits_the_answer`, `an_unjudged_instance_of_a_settled_series_inherits_too`, `rank_shows_a_later_instance_of_an_approved_series_in_the_same_run`, `judge_roster_does_not_judge_a_new_instance_of_a_settled_series` |
| (a) every ledger title is sanitised, each uid with its own (I-6) | F1, F3 | `an_answer_title_is_sanitised_like_a_verdict_title`, `every_instance_on_a_series_card_is_answered` |
| (a) the human-actor dependency is visible (M-2) | F3 | the approve test writes under `WriteContext::new("quinn","dashboard")`, the console's literal. A cross-crate pin (app `console_ctx()` ⇔ `human_set`) is **deferred**: it lives in `app/tests/**`, outside this plan's files |
| (a) the ledger stays a compatible contract | F1 | `the_answer_line_is_byte_exact`, `record_verdict_bytes_are_unchanged`, plus the stated old-reader behaviour |
| (b) the device keeps `judgment_id` | F4 | the `cloud_contract.rs` capture tests; `judge_roster_writes_the_jid_onto_the_ledger_line` |
| (b) it reaches notes and cards compatibly | F5, F6a, F6b | the `…exactly_as_before` byte tests, and the carry tests |
| (b) a correction or rejection joins its judgment | F7, F8 | `a_settled_event_check_is_reported_once…`, `a_rejected_judged_amend_card_is_reported_as_a_decision`, the handler save tests |
| (b) `calibration_query.sql` finds the pairs | F7 | the static pin on the query; the join is the existing `c.judgment_id = j.id` |
| (b) no backfilled id is ever erased | F7 | `plain rows and id-bearing rows reach saveCorrections in separate calls…` |
| (b) Gmail-derived labels never reach cross-account calibration or `promote_rules` (I-4, ruled) | F7, F8 | `a_rejected_email_kind_card_is_never_reported_or_stamped`, `an email label row is refused`, the static pin's email clause |
| (b) a device-supplied `judgment_id` cannot cross accounts (I-5) | F7 | `a judgment_id the caller does not own is dropped and counted`, the static pin's `c.account_id = j.account_id` |
| (b) `promote_rules()`' own account clause | **deferred** | needs a migration (none in this plan); guarded meanwhile by the handler's ownership check |
| (b) Gmail completion cards keep the id (I-2) | F6b | `a_gmail_completion_card_carries_the_email_judgment_id`, `a_vendor_completion_card_carries_no_judgment_keys` |
| (b) an email field-correction join | **deferred** | per-user only (Global Constraint 14); needs `app/src/telemetry.rs` to read the note's `judgment_id`. The id is on the note after F6b |
| (c) Localist and Engage page to completion | F9 | `localist_pages_until_page_total`, `engage_pages_by_skip_until_the_count` |
| (c) hard caps and deterministic order | F9 | `paging_stops_at_the_page_cap_and_warns_once`, `a_duplicate_uid_across_pages_collapses_first_wins` |
| (c) paging cannot hold `rank` or `judge` past the 20-minute step limit (I-3) | F9, F10 | `paging_stops_at_the_per_source_time_budget_and_keeps_what_it_has`, `the_run_time_budget_skips_a_later_source_and_names_it`, `the_default_limits_are_90_and_180_seconds`, `judge_roster_paging_time_counts_against_its_budget` |
| (c) the frozen `events.md` is untouched | F9, F2, F3 | `--test oracle` and `--test surface_oracle` green with no fixture diff |
| (c) no extra model spend on events `rank` drops | F10 | `judge_roster_does_not_pay_for_an_event_rank_would_filter` |
| (d) completion keyed on `id:` | F11 | `a_new_task_at_a_reused_path_is_still_proposed`, `a_renamed_task_is_not_proposed_twice` |
| (d) `MIN_CLASS_N` justified | F12 | the citation comment; `MIN_CLASS_N is thirty…`; the boundary test |

## Open questions for Quinn

None. The one raised in drafting was ruled by the controller (2026-09-23): a card title writes
`10am–3pm` and `7–9pm`, never a bare `10–3`, because a bare range cannot tell morning from evening.
Cost if wrong: one formatting function.

## Plan review (2026-09-23)

Review: `C:\Users\danie\GitHub\knowlu\.superpowers\sdd\2026-09-23-engine-follow-ups-plan\plan-review.md`
(verdict "Ready with fixes", no Critical). Every finding and its disposition:

| finding | disposition |
|---|---|
| I-1 approving a series hides its later instances | **Fixed.** F2 decision 9 `inherit_series_answers` (in `rank`, before the roster) and `settled_series`; F10 skips judging new instances of a settled series; five tests |
| I-2 F6b's file list was wrong | **Fixed.** `completion.rs` joins F6b; `propose_done` gains `judgment: Option<(&str, &str)>`; callers listed (`coursework.rs` untouched); F6b depends on F11; `completion.rs` sequenced F11 → F6b |
| I-3 paging had no wall-clock bound | **Fixed.** F9 decision 7: 90 s per source, 180 s per call, checked before each fetch, worst case 5.5 min against the 20-minute step; truncation named in warnings. F10 makes paging spend `judge`'s events budget |
| I-4 Gmail labels fed cross-account calibration | **Fixed per controller ruling.** Global Constraint 14; F8 never reports `email`-kind cards; F7 refuses an `email` label row and the query excludes one |
| I-5 client `judgment_id` trusted across accounts | **Fixed, both layers.** F7 decision 6 drops unowned ids with a named `unowned` count; the query joins `c.account_id = j.account_id`; `promote_rules`' clause deferred to a migration |
| I-6 answer-line titles | **Fixed.** F1 shares one `clean_title` across all writers and records `title` per entry; F3 uses each uid's own title |
| M-1 Deno `--allow-write` | Accepted; F7 and F12 Verify lines corrected |
| M-2 `human_set` is `"quinn"`-only | Accepted; stated in Global Constraint 1; cross-crate pin deferred (app files) |
| M-3 `created_by` | Accepted; `created_by: events`, journal actor `agent:events` |
| M-4 ceiling counted `proposed_at` | Accepted; counts `first_proposed_at` |
| M-5 older engine meets an `event-check` card | Accepted; named in F3 decision 5 |
| M-6 T6's floor is about bins | Accepted; F12's comment says the floor is borrowed |
| M-7 line drift | Accepted; `cli.rs:391-400`, `enrich.rs:374`, `eventfeed.rs:603`, `cli.rs:974` corrected |
| M-8 authority paths | Accepted; absolute main-checkout paths given |
| M-9 only the primary's `judgment_id` | Accepted; stated in F2 decision 7 |
| M-10 `judgment_id` on valued fields | Accepted; refused with 400 (F7 decision 2) |
