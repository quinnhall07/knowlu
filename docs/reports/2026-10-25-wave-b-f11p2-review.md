# Wave B F11p2 review: the week strip and step 8's one-site form

Reviewer: `reviewer` (read-only; the controller filed this report). Round 1 covered
`7e0b40e..c712860` and failed. Round 2 covered `c712860..e78e025` and passed. No contract-list
file is touched, no pin was loosened, and endings are LF.

## Round 1: FAIL
1. **Important, the Found line joined codes with a comma** ("CS 201, BIO 101"). *Fixed:*
   `joinCodes` gives "A", "A and B" and "A, B and C", and a node pin checks each.
2. **Important, keyboard focus was lost on Show and on advance.** *Fixed:* every site heading has
   `tabindex=-1`, and `focusSite8` runs on Show, Skip, Done and advance, never on a poll.
3. **Minor, the strip's name wasn't announced.** *Fixed:* it has `role="region"`.
4. **Minor, the "Knowlu's form" frame took in the skip line and buttons.** *Fixed:* it ends at the
   privacy line (BRIEF R2.7).
5. **Pre-existing, the login labels had no `for`.** *Fixed.*

What held:
- the credential rules: only the two wrappers are called, no `vault`, and the fields are cleared
  in both arms;
- `unmatched` never reaches the page;
- codes keep the status order;
- the label uses no Date arithmetic;
- the strip shows and hides at exactly the specified steps;
- every site stays reachable.

The two things the controller asked about are as specified. Past days show no count (plan line
6294), and the label "Week of October 19" is the spec's wording (6295). The prototype's "4 turned in,
kept off your list" is out of scope, owned by no wave-B task for the strip; it is offered to Quinn.

## Round 2: PASS
**Carry:** a failed Skip, a failed sign-in, the "storing" repaint and the end of reading a site also
repaint the site without focusing it. These are older than F11p2. Add `focusSite8()` after
`paintSite` in those paths the next time step 8 is touched (plan §14.61).
