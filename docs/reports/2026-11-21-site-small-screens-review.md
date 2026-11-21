PASS

# Review: knowlu.com on small screens (`site/responsive`)

Diff: `git diff origin/main..HEAD` (base ff5f4847, HEAD 7a17343a, two commits). 13 files: 12 under `site/` plus
`docs/specs/2026-11-21-site-small-screens.md`. No contract-list file, no cargo change, no workflow change. The
contract is the design note (decisions 1 to 4, signed).

No blocking findings. There are two minor ones. One is an edge of decision 4 that Quinn should rule on, and the
other is a risk that needs checking on an iPhone.

## Findings

### 1. Minor: tablets narrower than 760px in portrait get the phone layout and the phone cuts

`site/css/base.css:257`, `site/css/hero.css:154`, `site/css/pieces.css:236`, `site/css/price.css:291`,
`site/css/proof.css:62`, all `(max-width: 759.98px) and (orientation: portrait)`. The phone nav below 760 is in
`site/css/base.css:166`.

Rule: decision 4 says "Tablet portrait keeps everything" (note, line 50), and decision 2 says "a tablet should read as
a small desktop". The 760px breakpoint lets the 768px iPad through, but smaller tablets are narrower than 760.

Failing scenario: an iPad mini (6th generation) held upright has a 744×1133 viewport. It gets the Menu button, a
single column and every portrait-phone cut. The hero's demo card, the stat beside each demo, the pilot's three steps
and two demo captions are all hidden. 7-inch Android tablets (600 to 712px wide) behave the same way. The note was
checked at 768 and 1024 only.

For Quinn, as a decision: either accept 744 as within "about 760px" and add one line saying so to the note, or
capture 744×1133 first. If the two-column layouts fit at that width, lower the breakpoint to 740. If they don't, keep
760 for the columns and nav, and give the portrait cuts a narrower bound such as `max-width: 599.98px`. My
recommendation is to capture first. It costs one screenshot, and the iPad mini is the most common tablet under 760.

### 2. Minor (not tested on a device): tapping outside may not close the menu on iOS Safari

`site/js/site.js:709`:

```js
document.addEventListener('click', function (e) { if (isOpen() && !nav.contains(e.target)) setMenu(false); });
```

Rule: decision 1, "closes … on a tap outside".

Failing scenario: an iPhone user opens Menu and then taps the 75.6% figure or a paragraph of body text. Mobile WebKit
has a long-standing habit of not sending a `click` to a listener on `document` when the tapped element is not
"clickable": it has no click or mouse listener and no `cursor: pointer`. If that still holds, the panel stays open.
The `focusout` fallback (`site.js:713`) does not help, because iOS Safari does not focus a `<button>` on tap. The
implementer's checks ran under Chromium emulation, which does deliver the click. The note already says iOS Safari is
untested, so this is a risk to check, not a confirmed failure.

The same pattern already exists at `site/js/pieces.js:392`, which puts a held week block back when you tap outside
the week, so any fix should cover both. Two possible fixes: listen for `pointerdown` on `document` instead of
`click`, or set `cursor: pointer` on `body` while `.nav.is-open`. Either way, check it on a real iPhone before
calling decision 1 done.

## What was checked and found clean

**(1) Desktop at 1180 and wider.** I compared every rule that now applies at 1180 and wider with the old 1000px
blocks:
- `base.css`: `.nav-in` ends at `1fr auto 1fr` with gap 24px, the same as before. `.nav nav` is `flex` with gap 8px.
  The Menu button is `display: none` (`.js .nav-menu` at 760px and up has the same specificity as the base rule and
  comes later). All `.is-open` rules sit inside the under-760 query.
- `--safe-t` and `--safe-b` resolve to 0 on desktop, and `--gut` is `max(old clamp, 0, 0)`, the old value.
- `hero.css`: every property the 760px block sets either matches the old 1000px block or is reset by the 1180px
  block. That covers columns, column gap, lockup size, card width, `.hero-go` top margin and the note's font size,
  which goes back to `var(--t-body)`, the base value. The short-landscape block stops at 1179.98px.
- The `hts` container query moved from 290px to 19.3em. At 1180 and wider the sheet is at least 375px wide (25em at
  15 to 17px), far from either threshold.
- New clamp floors (`--sec` 56px, `.pc + .pc` 72px, `.pc-foot` 40px, the `.pc` and `.pf-in` column gaps) all sit
  below their vw term at 1180px, so the values used are unchanged.
- `.pf-n` returns to `clamp(160px, 15vw, 248px)` at 1180px.
- `price.css`, `letter.css` and `questions.css` only moved their media line. The `.pl-full` and `.pc-ahead` tablet
  rules stop at 1179.98px.
- Wrapping hover rules in `(hover: hover)` keeps their source order. The reduced-motion `transform: none` overrides
  still come later at equal specificity. The `.pc-drop:hover` outline still follows the dashed rule it overrides
  (both 0,6,0).
- `min-width: 44px` on `.pc-ev` does not affect any pill: the narrowest is "Shift" plus a time.
- The nav's Apply link is now `<span>Apply<span class="nav-x"> for the pilot</span></span>`. At 360px and up it shows
  the same words, and the outer span keeps the space inside the inline-flex button.
- `joined.html` keeps its single-column nav (`.jn-nav .nav-in`, 0,2,0). `signed-in.html` and `subscribed.html` do not
  load `base.css`.

**(2) Menu accessibility.**
- `aria-expanded` and `aria-controls="nav-links"` are present and toggled together with `.is-open`.
- Escape closes the menu and returns focus to Menu (`setMenu(false, true)`). A link tap, `focusout` to outside the
  header, and growing past 760px also close it.
- Clicking Menu does not close the menu immediately: the document listener sees that the header contains the button.
  The `relatedTarget` guard keeps Safari's no-focus-on-click from closing it.
- With scripts off, `.js` is never set, so Menu stays hidden. The bar's `<nav>` is hidden under 760px, and the footer
  nav (`index.html:460`) has all three links.
- The skip link is untouched apart from `top: calc(12px + var(--safe-t))`. Its z-index (30) is above the nav (10).
- The open panel's links are 56px tall and Menu is at least 44×44.
- Hero fine print: `.hero-go-short` is `display: none` by default. Inside `(max-width: 759.98px) and
  (orientation: portrait)` the long version is hidden and the short one shown. `orientation` is always exactly one of
  portrait or landscape, so exactly one paragraph is rendered, and so in the accessibility tree, at every size.
  `dots.js` skips the hidden one (it checks `offsetParent` and client rects).

**(3) JavaScript.**
- Every listener is added once at load on a static page, so nothing leaks.
- The `matchMedia('(min-width: 760px)')` change closes the menu, with an `addListener` fallback.
- `pieces.js` checks for `(pointer: coarse)` before using it. It rebuilds the captions on `change` (attaching or
  removing a mouse), skips the week's caption while a block is held (`_klBusy`), and restores the right idle text on
  release. No `.pc-hint` contains child elements, so writing `textContent` loses no markup.
- Hiding `.pl-steps` on portrait phones leaves `pilot.js` safe: zero-size ticks give `p = 1`, with no error.
- `dots.js` ignores zero-width quiet boxes.

**(4) Copy.**
- The touch captions ("Tap Start or Later.", "Tap a block, then tap a day to move it to.", "Now tap a day. …") only
  swap Choose or Press for Tap and drop the arrow-key sentence.
- The short hero line matches the note word for word.
- "Apply" alone appears only under 360px.
- The only other new visible string is the button label "Menu", which decision 1 requires. Everything else new is an
  HTML comment.

**(5) Pilot form.** No hunk touches the `<form>` or any `name=`, `action` or hidden field. `.pl-go`, the way to the full
application, is a sibling of `.pl-steps` (`index.html` around line 413), so it stays visible on portrait phones.

**(6) Line endings and scope.** `git ls-files --eol` shows `i/lf w/lf` for all 13 files, there are no CR bytes in the
diff, and `git diff --check` is clean. Every file is under `site/` except the one design note.
`privacy.html`, `terms.html` and `site.css` are untouched, as the note's scope requires.

## Follow-up (controller, 2026-11-21)

- Finding 1: the two-column breakpoint is lowered from 760px to 740px everywhere (CSS and the menu's matchMedia).
  Checked at 739, 740, 744×1133 and 768×1024 in Chromium emulation: no sideways scroll, the nav's wordmark, links and
  Apply don't overlap at 740, and every two-column section fits at 744. An upright iPad mini now gets the tablet layout.
- Finding 2: the menu's and the week demo's outside-tap listeners use `pointerdown` instead of `click`. An emulated
  touch tap outside the open menu closes it. A real iPhone is still untested.
