# T1d and T3c review: the merge and the 8 pm rule removed (diff 17e91d2..1a76f9a)

Reviewer: `reviewer` (read-only; the controller filed this report). **Verdict: PASS**, with no
Critical or Important findings, three Minor findings and one interim note. Only
`engine/src/{cli,ranking,surface}.rs` change. The workspace result is the implementer's: 3179 passed,
0 failed, 3 ignored.

## Checked
- **Deletions match T0's retire list exactly:** R3, R4, R7, R14 and R15, plus C3 and S1. No
  keep-listed test was edited, and nothing under `engine/tests/` changed.
- **Re-points match the plan word for word:** C5's last assertion and S2's 21:00 half. The commit
  names Q5 and the allowance.
- **The merge matches §2.7.** `ahead` is checked against `before(d_i) - 30` for every remaining kept
  item, the refill length equals the positions, and W goes last. DP21 was re-derived by hand.
- **`rank` is byte-for-byte the old pressure order:** the same stable sorts, partition order and
  override clamp. `plan_at(None)` equals `rank`.
- **Determinism holds:** the keys are integers or `DateTime`, and the sorts are stable.
- **The clamp** touches only `work_minutes`.
- **`today.md`** carries no "fit" wording (`render.rs` is untouched; DC1).

## Interim note: the branch must not merge or ship between T3c and T4a
On DS1's vault at 21:00, `today.md` reads Erste Reflexion, then the probe, then CS 100 HW 01. The
page still groups by first appearance and sorts rows, so it puts CS 100 HW 01 ahead of the probe.
T4a's DS1 and `must_do` bullets already fix this, so the brief needs no new text.

## Minor
1. **`surface.rs:224`'s doc comment** still names "the evening step". T4a fixes it while it edits
   `load_at`.
2. **`ranking.rs:564`'s section header** named R1-R15. *Fixed by the controller:* it now names the
   retired five.
3. **`docs/reference/engine-commands.md:7-16,129`** still describes the 20:00 rule and
   `EVENING_HOUR`. T6's brief must name these lines.
