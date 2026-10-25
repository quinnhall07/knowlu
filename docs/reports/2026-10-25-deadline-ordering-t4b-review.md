# T1f and T4b review: the meter and its line

Reviewer: `reviewer` (read-only; the controller filed this report). Only `engine/src/surface.rs`
changes, and nothing on the contract list.

## Round 1 (76e355f): FAIL, two Important findings
1. **The pin was left unnamed in two forms of the line** (Q8, spec §4): the placeholder form and the
   several-items form. Quinn approved wording for both on 2026-10-25 (plan §9). *Fixed in cf95ec0*,
   with two new DS12 cases.
2. **A bare-date deadline serialised as `T23:59`**, so the hover would print an 11:59 pm the source
   never gave (§7). *Fixed:* `spill_deadline` emits the date alone, and a new DS5 case pins it.
- **Minor, carried to T5:** `spill_items` can list items while `spill_blocks == 0`, so the console
  must not hide the list when `spill_blocks == 0`.
- **The four rulings are agreed:** the placeholder sentence replaces the assurance, the count is
  distinct placeholders, the assurance appears only with a single item, and a pin means
  `rank_override` among Must do.
- **Checked:** the oracles' bytes are unchanged with None fields, the totals are untouched, nothing
  is double-counted, the rounding is right, title joining is right, the output is deterministic and
  the M6 wording rule holds. The truncation is correct.

## Round 2 (cf95ec0): PASS
- **New Minor:** "isn't counted" stays singular with two or more pins. *Folded into T4c* as
  `aren't`, with a two-pin test.
