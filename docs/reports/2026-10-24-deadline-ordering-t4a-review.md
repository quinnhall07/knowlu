# T1e and T4a review: the read model's order (diff d7f499b..69eb99c)

Reviewer: `reviewer` (read-only; the controller filed this report). **Verdict: PASS.** There are no
Critical or Important findings and three Minor ones. Only `engine/src/surface.rs` changes.

## Checked
- **The T3c interim disagreement is closed.** With a moment, `must_do` splits `plan.order`'s Must
  do into runs without re-sorting, so the page reads exactly as `today.md` does (DS1 at 21:00: Erste
  Reflexion, the probe, CS 100 HW 01). The designation reads `rank` on both sides.
- **The no-moment path is byte-identical by the code.** `plan_at(None)` equals `rank`, the new
  label arm is guarded by the moment, and `sort_rows` still applies. Note: `surface_oracle` pins the
  09:00 *with-moment* path, not this one. Every live page has a moment, so the live Must do no
  longer applies `sort_rows`, and labels can repeat. That is what spec §2.1 and §4 (Q6) say, and it
  is a visible change.
- **Preview:** the plan and `scored` are computed on the previewed calendar, and `moved` is
  unaffected.
- **Ruling 1, the moment filtered to today: correct and needed.** **Ruling 2, the clone:**
  harmless.
- **Determinism and no writes:** both hold. The tests match T1e's text.

## Minor
1. **`Loaded.moment` isn't truncated to the minute** (spec §2.1). A task due `10:00:30` seen at
   `10:00:45` is ordered as open but labelled Overdue. *Folded into T4b's brief:* store the
   truncated moment.
2. **Two meanings of "Overdue."** `group_label` uses `due <= m`, while `horizon()` (the nav's
   Overdue count and list) goes by date. A probe due 10:00 at 15:00 is "Overdue" in Must do but
   counted under "This week" in the nav. *Taken to Quinn for a ruling.*
3. **DS4's `moved` half is `None` on both sides** (vault-s1 has no planning-day note), so it proves
   nothing. A later guard should add a planning-day note to the scratch copy. *Carried to T1g.*
