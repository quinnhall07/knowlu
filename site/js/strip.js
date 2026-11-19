/* Knowlu logo strip: the row of sites under the hero.
   The row moves at about 33 px a second. Pointing at it eases it to a stop (the logo under the pointer comes up in
   color); leaving eases it back up. The pause button holds it still.
   A strip marked data-kick takes a click as a push: the row lurches ahead and coasts back down, and a shock runs up
   the hero's dots from where you clicked (js/dots.js, window.klDots). Three numbers set the feel, and the
   strip can carry its own as attributes:
     data-kick-push   how much speed a click adds, px a second (1000)
     data-kick-coast  how long the extra speed takes to fade, seconds (0.9)
     data-kick-shake  how hard the dots are knocked, 0 for not at all (1)
   Reduced motion: none of this runs; the CSS lays the logos out still. */
(function () {
  'use strict';
  if (window.klStrip) return;

  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
  var SPEED = 33;
  var PUSH = 1000, COAST = 0.9, SHAKE = 1;
  var all = [];

  function dots() { return window.klDots || null; }

  function setting(s, name, fallback) {
    var v = parseFloat(s.zone.getAttribute('data-kick-' + name));
    return isFinite(v) ? v : fallback;
  }

  function goal(s) { return s.held || s.hovering || performance.now() < s.tapUntil ? 0 : SPEED; }

  function step(s, t) {
    s.raf = 0;
    var dt = s.last ? Math.min(0.05, (t - s.last) / 1000) : 0;
    s.last = t;
    var g = goal(s);
    s.v += (g - s.v) * (1 - Math.exp(-dt / (g ? 0.6 : 0.22)));
    if (Math.abs(g - s.v) < 0.8) s.v = g;
    if (s.boost) {
      s.boost *= Math.exp(-dt / s.coast);
      if (Math.abs(s.boost) < 3 || s.held) s.boost = 0;
      var fast = Math.abs(s.boost) > 60;
      if (fast !== s.fast) { s.fast = fast; s.zone.classList.toggle('is-kicked', fast); }
    }
    var w = s.track.offsetWidth;
    if (w > 0) {
      s.off = (((s.off + (s.v + s.boost) * dt) % w) + w) % w;
      s.mq.style.transform = 'translate3d(' + (-s.off).toFixed(2) + 'px,0,0)';
    }
    if (s.onScreen && !document.hidden && (s.v > 0 || g > 0 || s.boost || performance.now() < s.tapUntil)) request(s);
    else s.last = 0;
  }
  function request(s) { s.raf = requestAnimationFrame(function (t) { step(s, t); }); }
  function run(s) { if (s.driven && !s.raf && s.onScreen && !document.hidden) request(s); }

  function kicks(s) { return s.zone.hasAttribute('data-kick'); }
  function kick(s, e) {
    if (!kicks(s) || s.held || !s.driven) return;
    s.coast = Math.max(0.15, setting(s, 'coast', COAST));
    /* a second click while it is still running adds to a third of what is left, so quick clicks build a little */
    s.boost = s.boost * 0.35 + setting(s, 'push', PUSH);
    var d = dots(), shake = setting(s, 'shake', SHAKE);
    if (d && shake > 0) d.shock(e.clientX, shake);
    run(s);
  }

  function drive(s, on) {
    if (on === s.driven) return;
    s.driven = on;
    if (on) {
      /* take over from the CSS animation where it is, so the row does not jump */
      var tx = 0;
      try { tx = new DOMMatrixReadOnly(getComputedStyle(s.track).transform).m41; } catch (e) { tx = 0; }
      if (isFinite(tx) && tx < 0) s.off = -tx;
      s.mq.classList.add('is-driven');
      run(s);
    } else {
      s.mq.classList.remove('is-driven');
      s.mq.style.transform = '';
      s.boost = 0; s.fast = false;
      s.zone.classList.remove('is-kicked');
    }
  }

  function setup(mq) {
    if (mq._klStrip) return;
    var track = mq.querySelector('.mq-track');
    /* a row an older copy of this code is already moving is left to it */
    if (!track || mq.classList.contains('is-driven')) return;
    var zone = mq.closest('.logos') || mq;
    var s = mq._klStrip = { mq: mq, track: track, zone: zone, off: 0, v: SPEED, boost: 0, fast: false, coast: COAST,
      hovering: false, held: false, tapUntil: 0, raf: 0, last: 0, onScreen: true, tapped: null, tapTimer: 0, driven: false };
    all.push(s);
    /* a strip that arrives under a resting pointer starts out pointed at */
    try { s.hovering = zone.matches(':hover'); } catch (e) { s.hovering = false; }

    var btn = zone.querySelector('[data-mq-pause]');
    if (btn) {
      btn.addEventListener('click', function (e) {
        e.stopPropagation();
        s.held = !s.held;
        mq.classList.toggle('paused', s.held);
        btn.setAttribute('aria-pressed', String(s.held));
        btn.querySelector('span').textContent = s.held ? 'Play' : 'Pause';
        btn.querySelector('[data-i="pause"]').hidden = s.held;
        btn.querySelector('[data-i="play"]').hidden = !s.held;
        run(s);
      });
    }
    zone.addEventListener('pointerenter', function (e) { if (e.pointerType !== 'touch') { s.hovering = true; run(s); } });
    zone.addEventListener('pointerleave', function (e) { if (e.pointerType !== 'touch') { s.hovering = false; run(s); } });
    zone.addEventListener('pointerdown', function (e) {
      /* a tap holds the row for a moment and lights the logo tapped; on a strip that takes a push, the tap is the push */
      if (e.pointerType !== 'touch' || !s.driven || kicks(s)) return;
      var li = e.target.closest && e.target.closest('.mq-i');
      if (s.tapped) s.tapped.classList.remove('is-on');
      s.tapped = li;
      if (li) li.classList.add('is-on');
      s.tapUntil = performance.now() + 1800;
      clearTimeout(s.tapTimer);
      s.tapTimer = setTimeout(function () { if (s.tapped) s.tapped.classList.remove('is-on'); s.tapped = null; run(s); }, 1800);
      run(s);
    });
    zone.addEventListener('click', function (e) { kick(s, e); });
    if ('IntersectionObserver' in window) {
      new IntersectionObserver(function (es) { s.onScreen = es[es.length - 1].isIntersecting; run(s); }).observe(zone);
    }
    drive(s, !reduced.matches);
  }

  function scan(rootEl) {
    if (!rootEl || rootEl.nodeType !== 1) return;
    if (rootEl.matches('[data-mq]')) setup(rootEl);
    Array.prototype.forEach.call(rootEl.querySelectorAll('[data-mq]'), setup);
  }

  window.klStrip = { scan: scan };
  scan(document.documentElement);
  /* a strip added to the page later is picked up too */
  if ('MutationObserver' in window) {
    new MutationObserver(function (list) {
      for (var i = 0; i < list.length; i++) Array.prototype.forEach.call(list[i].addedNodes, scan);
    }).observe(document.documentElement, { childList: true, subtree: true });
  }
  document.addEventListener('visibilitychange', function () { all.forEach(run); });
  if (reduced.addEventListener) reduced.addEventListener('change', function () { all.forEach(function (s) { drive(s, !reduced.matches); }); });
})();
