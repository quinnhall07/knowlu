# Wave B F11p1 review r1 (93f2313 vs 417ed72): FAIL

Reviewer: `reviewer`. Filed by the controller.

## Important
- **I-1: the payment line can be wrong in both directions.**
  - The server sends `"none"` for every unpaid account, and testers arrive as `active` + `plan: "tester"`. No `"comped"` status exists.
  - A student whose cache still says "none" after checkout sees "Nothing to pay.", and testers see "Paid. Renews {date}."
  - Fix:
    - `active` + `tester` → "Nothing to pay.";
    - `trialing`/`active` with an end date → the dated lines;
    - everything else, `"none"` included → no line.
  - The pin at onboarding_static.rs:989 assumes a status that doesn't exist, so it is corrected, not loosened.
- **I-2: the minutes estimate rises after a step.** Each step is rounded up and then summed, so step 3 shows 11 after the shell's 10. Sum seconds instead, then `ceil(sum/60)`, with a floor of 1.

## Minor
- **m-1:** `console.css:681` puts `tabular-nums` on all of `.setup`/`.picker`, which causes the "3 1" gaps. Delete it, and fix the README's line about proportional figures.
- **m-2:** "lu" should be the theme's ink in both themes (§14.58.2), not cream, and the block should use `var(--su-brand)`.
- **m-3:** the fixture fallback is stricter than the spec. Document it in the README.
- **m-4:** account lines refresh only on `OB.show` moves. Noted for F11p2.

## Checked
- `account_status` is local only, has one call site, and is never called on the poll.
- No F11t assertion was edited.
- onboarding.js is net +59 lines.
- No single-user data, no storage, and the faces are local.
- The six shot pairs match the prototype in layout, regions and controls. The only unintended differences are m-1 and m-2.
