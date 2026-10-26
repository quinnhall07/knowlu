# Deadline-ordering spec review r1 (19e868f): FAIL

Reviewer: `reviewer`. Filed by the controller.

## Important
- **I1.** `weekcal::with_day_window` is keyed by weekday, not date, and it drops a window that ends at midnight (weekcal.rs:232-240). Widening tonight widens every same weekday.
- **I2.** "Won't fit" can move the wrong way. A later deadline can get less counted time, and an unrelated small task due tonight can flip another item's verdict. Uncounted evenings make the verdict too early.
- **I3.** "Exactly as today when nothing is tight" is false. Must do is pressure-ordered today, so the formula changes the daytime order and the 12:00 and 18:00 `today.md`.
- **I4.** A placeholder estimate can produce a flat "Won't fit" on a real item; that item should read "may not fit".

## Minor
- M1: Q9's tie-break contradicts §5:5.
- M2: "the rest fits" is false when a pin exists.
- M3: there is no rounding rule for m, and no rule for the day the clocks go back.
- M4: the order of the cut at m and the 45-minute filter isn't stated.
- M5: the designation's input list is left open.
- M6: wording: "tonight" for an overrun on another day, a bare-date due shown as "11:59 pm", and `group_label`.
- M7: T2 and T4 are coupled, and T2 is oversized.
- M8: the routing of E1 and E2, which assert against frozen goldens, isn't settled.
- M9: the `surface --window` preview isn't covered.

## Checked, holds
- The frozen goldens (rank --today, no moment) and the three surface references (--now 09:00: an Overdue-only Must do group, pressure order equals sort order, empty spill) stay byte-identical.
- The scheduling theory is exact on one timeline, and the worked examples are arithmetically right.
- rank stays pure, with no single-user constant and no new synced data.
