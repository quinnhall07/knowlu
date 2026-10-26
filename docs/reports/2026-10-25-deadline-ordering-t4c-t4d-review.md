# T1g, T4c and T4d review: the row line, its reason and the clock-based Overdue

Reviewer: `reviewer` (read-only; the controller filed this report). Only `engine/src/surface.rs`
changed, with no fixture or oracle edits and nothing on the contract list.

## Round 1 (T4c, 6c3c0b3): FAIL
1. **Important:** rounding could print "3.0h left; you have 3.0h" for 180 minutes of work against
   179 available.
2. **Important:** the drawer (`note_detail`) loads with no moment, so it carries no row line.
   **Controller ruling:** T5 shows the line and the reason from the list row's own fields, in the
   row and in its drawer. No engine change.

Also from this round:
- **For Quinn:**
  - an item that can't fit alone now reads "Won't fit" even with a placeholder ahead (ruled yes);
  - a placeholder's "1.0h left" is kept (ruled keep).
- **Minor:** DS14 was loose, and T4c's rulings 1 and 2 had no tests.

T4c's three rulings are sound: "by" for a bare date and "before" for a timed one, the item's own
placeholder wins, and the dates are the page date.

## Round 2 (8478793..27e951d: DS14 tightened, T1h, dd13a6a fixes, 27e951d T4d): PASS
- **Rounding:** work rounds up and time rounds down, so the work figure is always strictly larger
  when the "you have" reason prints.
- **Rule order:** the item's own placeholder, then can't-fit-alone, then a placeholder ahead. This
  matches Quinn's rulings.
- **`horizon` with a moment:**
  - a due at the moment is overdue (`<=`);
  - the moment is truncated;
  - a bare date is 23:59, never midnight;
  - with no moment, nothing changes.
- **The views agree:** the nav count, the Overdue view and This week share one function and one
  moment.

### Minor, follow-ups
1. **The reason's hours can be 0.1h above the row's and the meter's,** because work rounds up. This
   is accepted under the I-1 ruling and reported to Quinn.
2. **A bare date counts as overdue from 23:59,** one minute before the day ends. This is consistent
   with `group_label`, and reported to Quinn.
3. **Untested:** an item that is its own placeholder and can't fit alone. *Test added in the T5
   window.*
4. **Untested in the nav:** a due exactly at the moment, and a bare date due today seen at 21:00.
   *Tests added in the T5 window.*
