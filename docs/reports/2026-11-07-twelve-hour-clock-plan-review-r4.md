# Twelve-hour clock plan, review r4 (2026-11-07)

**FAIL at 42d0a7e:** 0 Critical, 1 Important, 3 Minor. No finding lets a 24-hour time ship or changes vault bytes. All r3 findings are closed, and the 19 anchors each occur exactly once.

## Important

1. **T5 cannot pass T4 on the head T4 was written against.** That head has no T3, so `surface-today-full.json`'s `coming_up[].when` is still on 24-hour time. **Fix:** run T5 after both T3 and T4, and change §4 Order and §5 to match.

## Minor

1. **The recorder must stay permissive for every other member.** `paint` calls `querySelectorAll`, `closest`, `prepend` and `contains` on recorders through `preserveAcross` (`console.js:176`, `:188-198`).
2. **Use the exact key paths:** `closed_this_week` and `topline.sync.at`.
3. **The supplement's due card duplicates `decisions.cards[1]`.** Keep only the `importance → 4` card, in full shape.
