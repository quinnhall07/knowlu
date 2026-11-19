/* Figures that count up: any [data-count] element whose text is a plain figure ("74.8%") counts up from zero, with the
   same decimals throughout, when its section builds (the kl:in event from js/site.js). Text that is not a plain figure
   ("3 in 4") is left alone; its section's CSS raises it in. This covers the proof strip. Under reduced motion nothing counts: the figures are already in the HTML. */
(function () {
  'use strict';

  var reduced = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var FIGURE = /^(\D*?)(\d+(?:\.\d+)?)(\D*)$/;

  function parse(text) {
    var m = FIGURE.exec(text.trim());
    if (!m) return null;
    var dot = m[2].indexOf('.');
    return { pre: m[1], value: parseFloat(m[2]), places: dot < 0 ? 0 : m[2].length - dot - 1, post: m[3] };
  }
  function shown(el) { return el.offsetParent !== null; }

  /* runs el from one value to its own; the last frame writes the exact original text */
  function run(el, from, ms) {
    var end = el.getAttribute('data-count-text') || el.textContent;
    var f = parse(end);
    if (!f || reduced || !shown(el)) return;
    el.setAttribute('data-count-text', end);
    var id = (el.kCount || 0) + 1; /* a newer run replaces an older one */
    el.kCount = id;
    /* hold the final width so the words beside the figure never shift while it counts */
    var w = el.getBoundingClientRect().width;
    el.style.minWidth = w + 'px';
    var start = null;
    function frame(t) {
      if (el.kCount !== id) return;
      if (start === null) start = t;
      var k = Math.min(1, (t - start) / ms);
      var e = 1 - Math.pow(1 - k, 4); /* ease out */
      if (k < 1) {
        el.textContent = f.pre + (from + (f.value - from) * e).toFixed(f.places) + f.post;
        requestAnimationFrame(frame);
      } else {
        el.textContent = end;
        el.style.minWidth = '';
      }
    }
    el.textContent = f.pre + from.toFixed(f.places) + f.post;
    requestAnimationFrame(frame);
  }

  document.addEventListener('kl:in', function (e) {
    var root = e.target;
    if (!root || !root.querySelectorAll) return;
    var list = root.matches('[data-count]') ? [root] : root.querySelectorAll('[data-count]');
    Array.prototype.forEach.call(list, function (el) { run(el, 0, 1100); });
  });
})();
