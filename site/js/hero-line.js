/* Knowlu hero: the line under the tagline that shows what Knowlu would say right now.
   The lines are the <li> items in index.html (edit them there). One shows at a time; about every 4 seconds it rises
   and fades and the next comes in. It waits while the tab is hidden, while the hero is off screen, and while the
   pointer or keyboard focus is on the line. With scripts off only the first line shows; with reduced motion the
   lines swap without moving (css/hero.css). */
(function () {
  'use strict';

  var EVERY = 4000;   /* ms each line stays */
  var OUT = 700;      /* ms the leaving line takes to go (matches --slow plus a little) */

  var box = document.querySelector('[data-hero-now]');
  var items = box ? Array.prototype.slice.call(box.querySelectorAll('.hero-now-i')) : [];
  if (items.length < 2) return;
  var hero = box.closest('[data-hero]') || box;

  var cur = Math.max(0, items.findIndex(function (li) { return li.classList.contains('is-cur'); }));
  items.forEach(function (li, i) { li.classList.toggle('is-cur', i === cur); });

  var timer = 0, hovered = false, focused = false, onScreen = true;
  function paused() { return hovered || focused || !onScreen || document.hidden; }

  function next() {
    timer = 0;
    if (paused()) return;
    var from = items[cur];
    cur = (cur + 1) % items.length;
    var to = items[cur];
    from.classList.remove('is-cur');
    from.classList.add('is-out');
    to.classList.remove('is-out');
    to.classList.add('is-cur');
    setTimeout(function () { if (!from.classList.contains('is-cur')) from.classList.remove('is-out'); }, OUT);
    schedule();
  }
  /* each line gets its full time again after a pause, so a line never changes the moment you look away */
  function schedule() {
    clearTimeout(timer);
    timer = 0;
    if (!paused()) timer = setTimeout(next, EVERY);
  }

  box.addEventListener('pointerenter', function (e) { if (e.pointerType !== 'touch') { hovered = true; schedule(); } });
  box.addEventListener('pointerleave', function (e) { if (e.pointerType !== 'touch') { hovered = false; schedule(); } });
  box.addEventListener('focusin', function () { focused = true; schedule(); });
  box.addEventListener('focusout', function () { focused = false; schedule(); });
  document.addEventListener('visibilitychange', schedule);
  if ('IntersectionObserver' in window) {
    new IntersectionObserver(function (es) {
      onScreen = es[es.length - 1].isIntersecting;
      schedule();
    }, { threshold: 0.05 }).observe(hero);
  }
  schedule();
})();
