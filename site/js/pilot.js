/* The pilot's line: one fine line runs down through the three steps of the pilot band (css/price.css). Here it draws
   itself as the band scrolls in. Its tip follows a point 15% up from the bottom of the screen and never goes back up,
   and each step's tick and number light as the tip reaches them, the third (free forever) last. Without this script,
   or under reduced motion, the line is simply there, whole. */
(function () {
  'use strict';

  var ol = document.querySelector('[data-pl-steps]');
  if (!ol || !window.requestAnimationFrame || !window.getComputedStyle) return;
  var reduced = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)');
  if (reduced && reduced.matches) return;

  var steps = Array.prototype.slice.call(ol.children);
  var READ = 0.85;      /* where the tip sits, as a share of the screen's height from the top */
  var ticks = [];       /* each step's tick, px down from the top of the list */
  var reach = -Infinity; /* the furthest the tip has come, px down the line from the first tick */
  var queued = false, done = false;

  function measure() {
    ticks = steps.map(function (li) {
      var t = parseFloat(window.getComputedStyle(li, '::after').top);
      return li.offsetTop + (isNaN(t) ? 0 : t);
    });
    ol.style.setProperty('--pl-a', ticks[0] + 'px');
    ol.style.setProperty('--pl-len', (ticks[ticks.length - 1] - ticks[0]) + 'px');
    update();
  }

  function update() {
    queued = false;
    if (!ticks.length) return;
    var len = ticks[ticks.length - 1] - ticks[0];
    var tip = window.innerHeight * READ - ol.getBoundingClientRect().top - ticks[0];
    if (tip > reach) reach = tip;
    var p = len > 0 ? Math.max(0, Math.min(1, reach / len)) : 1;
    ol.style.setProperty('--pl-p', p.toFixed(4));
    steps.forEach(function (li, i) {
      if (reach >= ticks[i] - ticks[0] - 0.5) li.classList.add('is-on');
    });
    if (p >= 1 && !done) {
      done = true;
      window.removeEventListener('scroll', queue);
    }
  }

  function queue() {
    if (!queued) { queued = true; window.requestAnimationFrame(update); }
  }

  ol.classList.add('pl-live');
  measure();
  window.addEventListener('scroll', queue, { passive: true });
  window.addEventListener('resize', measure);
  window.addEventListener('load', measure);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(measure);
  if ('ResizeObserver' in window) new ResizeObserver(measure).observe(ol);
})();
