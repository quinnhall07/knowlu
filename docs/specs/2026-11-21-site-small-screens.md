# knowlu.com on small screens

Lane. 2026-11-21. Scope: `site/index.html`, `site/joined.html`, `site/signed-in.html`, `site/subscribed.html` and
the CSS and JS they load. `privacy.html` and `terms.html` are out of scope (Quinn's call), and so is `site/site.css`
(only privacy and terms use it — check before changing it).

## What is wrong now

Captured at 375×812, 430×932, 844×390, 768×1024, 1024×768 and 1180×820 (Chromium emulation). No page scrolls
sideways. Phone portrait mostly works. The problems are in between:

- **768 portrait** gets the phone layout stretched. Text columns stop at about 500px while the demos run full
  width, the hero is centered but everything below it is left-aligned, and the 75.6% figure fills the screen.
- **1024 landscape** gets the desktop layout too early. The hero card almost touches the right edge while the
  lockup sits at the gutter, the nav links float in the middle with no wordmark beside them (it is hidden while
  the hero is in view), and the week demo drops back to its stacked list inside a narrow column.
- **Phones.** The demo captions assume a mouse and keyboard ("Choose a block … Arrow keys work too."), there
  are large empty gaps around the week demo, the "made up" note and the letter, and the hero runs about two
  screens tall in landscape.

## Decisions (Quinn, 2026-11-21)

1. **Phone nav: wordmark, Apply button and a menu.** Below the width where the section links fit, the nav shows
   the wordmark at left, the Apply button, and a menu button that opens What it does / Pilot / Questions. The
   menu is a disclosure (`aria-expanded`, `aria-controls`), closes on Escape, on a link tap and on a tap outside,
   returns focus to its button, and works without JS (the links stay reachable in the footer). Every target is at
   least 44×44. Reason: visitors can reach the sections, and the only extra element is one button.
2. **Tablets get two columns earlier.** The side-by-side layouts (hero lockup | card, stat number | words, piece
   text | demo, letter, pilot, FAQ) start at about 740px rather than 1000px (760 at first; lowered so an upright iPad mini, 744 wide, gets it), with the demos scaled to fit their
   column. The breakpoint follows the content: whatever width each section actually fits at, checked at 768 and
   1024. Reason: a tablet should read as a small desktop, not a stretched phone.
3. **Demos are tuned for touch.** They stay interactive. On coarse pointers the captions say "Tap" where they
   say "Choose" or "Press", the keyboard sentence shows only where it applies (not on `pointer: coarse`), and
   every control in a demo is at least 44×44 on touch. Reason: the demos are the product, so they should work
   with a finger.

4. **Portrait phones leave things out, step by step** (added later on 2026-11-21). On a portrait phone each section
   keeps one job: the red 75.6% section carries the proof (kept at every size), the demos carry the product, and
   whatever repeats them steps back. Hidden on portrait phones only: the hero's demo card; the stat beside each
   demo; the pilot section's three steps; the captions "Answer the question in the window." and "Press Start or
   Later." (the week demo keeps its caption). The hero fine print becomes "Free forever if you stay the semester.
   Windows and Mac.", because with the steps gone it is the only place a phone sees the offer. Gaps between
   sections shrink by about a third. Hiding is CSS only and the markup stays. Phone landscape keeps everything
   (Quinn: it already looks fine) and gets fixes only where something doesn't fit. Tablet portrait keeps
   everything. Not approved: folding the pilot form's eight checkboxes.

## Also fixed (no decision needed)

- The hero card keeps the same gutter as the lockup at every width, and the hero fits on one screen in phone
  landscape (shorter vertical padding and a smaller lockup when the screen is short, `max-height` queries).
- The big stat number scales with the narrower side of the screen so it never fills a tablet.
- Vertical rhythm between sections is reduced on phones. The empty space around the week demo and the "made up"
  note is closed up.
- Safe areas: `viewport-fit=cover`, with the fixed nav and footer padded by `env(safe-area-inset-*)`.
- On touch screens, hover effects that do something have an equivalent that does not need hover.

## What must not break

- Desktop at 1180 and wider looks the same as before (compare before/after captures at 1280 and 1440).
- The brand stays as it is: colors, fonts and the wordmark geometry in `css/base.css` (the comment block there
  is a ruling). No new colors. Every color comes from `tokens.css`.
- Reduced motion, dark mode, forced colors, the skip link and keyboard use all keep working.
- The pilot form posts as before. Copy changes only where decisions 3 and 4 say so.

## How it is checked

Before/after Chromium captures at the six sizes above plus 320×568, 1280×800 and 1440×900, light and dark, for
every in-scope page; `scrollWidth == innerWidth` at every size; the menu and the demos operated by emulated touch
and by keyboard; the impeccable detector run once over the changed files. Emulation is not a real device, so the
PR names what stayed untested (iOS Safari).
