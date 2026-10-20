# L7 plan review, round 1 (2026-10-19)

Plan: `docs/plans/2026-10-19-l7-named-issues-plan.md` (draft), checked against the email branch at
`bf3e135`. Reviewer, plan mode. Filed by the controller from the reviewer's returned text (the reviewer
cannot write files).

**Verdict: FAIL.** 0 critical, 4 important, 6 minor. The design holds. The important findings need plan
edits before T0, then a short re-review of those edits.

## Code facts: all five hold

- **Provider 402.** `judge_openrouter.ts:88` throws a plain Error. `judge_pipeline.ts:229-262` turns it
  into `cause: "model failed"`, refunds the charge and replies 200. `judge_handler.ts:44` sends a 402 only
  from `requireActiveEntitlement`.
- **Device 402.** `cloudmodel.rs:223` reads a device-facing 402 as "no entitlement".
- **`quiet` and `last_read_at`.** Nothing writes `google_accounts` `'quiet'` or `last_read_at`;
  `index.ts:121` writes only `revoked`.
- **`quiet: true` means not connected** (`handler.ts:325,333`). The device reads any unknown reason as
  Revoked (`cloudmodel.rs:678-683`).
- **Mail heads-ups** are keyed `mail-source:<id>` (`mail/heads.rs:48`) and written through
  `info::open_info`/`close_info` with a journal (`mail.rs:236-257`).

## Invariants: all hold

- **Exit 0.** `judge` still always exits 0, and `sync` is untouched.
- **`rank` is unchanged.** The probe is `GET /judge-rules`, which makes no model call
  (`cloudmodel.rs:981`).
- **Writes** go through `write`, journal first, with real actors.
- **Determinism** holds.
- **References.** The skip-when-empty claim holds: `IssuesPanel` (`surface.rs:1623`) gains only a field
  skipped when empty, and `nav_counts` takes its counts as arguments (`:331`). No reference vault holds a
  heads-up.
- **Fixtures and contract list.** No frozen fixture, no contract-list file, no `app/src` change and no
  Tauri command.

## Important

1. **§1 "Money" (:144-145) is wrong about the eval gate.**
   - `ci.yml:242` runs the eval gate whenever `judge_pipeline.ts` changes, and T2 edits that file. So the
     PR runs the gate with staging secrets. It fails loudly if staging is down, and spends money if
     `eval_cases` has rows (`ci.yml:205-206`, capped at 1,800 calls).
   - Fix: correct §1. Add a T8 step that checks the `eval_cases` count before pushing, and raise the
     money question if it is non-zero.
2. **§3.3 (:252-260): "quiet" first fires after about 21 days, not 14.**
   - `last_read_at` is stamped by any read whose 7-day list has a message. Mail from day 0 keeps stamping
     it until day 7, so `went_quiet` first fires on day 21. That breaks cloud design §5.3's "stops
     yielding for 14 days".
   - Fix (recommended): when the 7-day list is empty, make one more list call with `newer_than:14d`, the
     excluded labels kept and `maxResults` 1. Report quiet only if that is also empty and `connected_at`
     is at least 14 days old. This drops the `last_read_at` writer, Q5, and the note to L4.
   - Alternative: stamp only when there are unseen ids after dedup.
   - Either way, add a T2 test: one message 10 days old must answer `went_quiet: false`.
3. **T1 acceptance (:576) lists test 4 as red, but it is green at T1.** Nothing clears a heads-up today,
   so `a_tier_two_answer_does_not_clear_it` passes now.
   - Fix: move it to the green list as a regression pin for T3.
4. **§6 and T3 (:468, 641-686): T3 is too big.** It touches five files and about 120 non-test lines,
   over the ~80-line bound.
   - Fix: split T3 into T3a (`source_issues.rs`, `lib.rs`, `judge.rs`, `cloudmodel.rs`; test 6 and its
     in-module tests) and T3b (the two `enrich.rs` sites; tests 1-4 and 7-9).
   - Also split T2 into T2a (the typed 402, `unavailable`, the gmail-read stop) and T2b (`went_quiet`).
     They run in sequence, since both edit `gmail-read/handler.ts`.

## Minor

5. **§5 "Untouched: events.rs" (:457).** `events.rs:500-503` prints one "not judged" line per event on any
   `Err`. L3's T3 ends that loop at the first `Capped` (L3 plan :886), but out of credit won't.
   - Fix: add an `events.rs` arm that ends the loop on `OutOfCredit`, coordinated with L3, which owns
     `events.rs` edits. Add a §5 row. Test 6 asserts no per-event lines.
6. **§3.4 (:322).** "Other errors leave it alone" would leave "Gmail has gone quiet" open next to
   "re-connect from settings".
   - Fix: clear the notice on every `CloudError::Quiet(_)`; leave it alone only on transport and status
     errors.
7. **T1 test 1 (:523-527).** `multi_reply_loopback` (`enrich.rs:1978`) answers in strict order, so the red
   result could come from replies arriving out of order.
   - Fix: check the request count first, or use a loopback that picks the reply by path.
8. **§2 Deno line (:166).** CI's full line needs `--allow-write=cloud/eval` (`ci.yml:167`). Say so.
9. **Q8 (:424-428).** `judge.rs` is in L6's file list (L3 treats it as L6's, L3 plan :508).
   - Make "cut after L6 merges" the default.
10. **The T0 ledger path (:484)** says 2026-10-18; the plan is dated 10-19. Pick one.

## Design choices

- **`unavailable` as a reply field: right.** A provider 402 can never look like the student's own, and
  older engines and servers keep working together.
- **`went_quiet` computed server-side: right**, but its 14-day comparison is wrong (finding 2).
- **Out of credit as a global stop: right.**
- **Clearing on the next answered reply: acceptable.** Tell Quinn with Q2.
- **Server-side Gmail and forwarded-mail judgments never clear the notice: acceptable** for the MVP.
