/* Knowlu site: the three product pieces in #what. Each window builds once when it scrolls into view, then stays
   usable: answer the syllabus question, press Start or Later, move a study block. The HTML is the finished state,
   so with scripts off each piece reads as plain content. Motion is transform and opacity, plus the height of the
   first two windows, which glides when a list gets shorter or longer so nothing under it jumps. Under reduced motion
   every change happens at once. */
(function () {
  'use strict';
  var root = document.getElementById('what');
  if (!root) return;

  var EASE = 'cubic-bezier(.16, 1, .3, 1)';
  var mq = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
  function still() { return !!(mq && mq.matches) || !('animate' in Element.prototype); }
  function one(r, s) { return r.querySelector(s); }
  function all(r, s) { return Array.prototype.slice.call(r.querySelectorAll(s)); }
  function text(el) { return el ? el.textContent.replace(/\s+/g, ' ').trim() : ''; }
  function px(n) { return Math.round(n * 10) / 10 + 'px'; }

  // The captions under the windows are written for a mouse and keyboard. On a touch screen (pointer: coarse) each
  // caption with a data-touch line says that instead: Tap for Choose and Press, and no sentence about arrow keys.
  // data-hold and data-touch-hold are the week's caption while a block is picked up. It follows the screen if a
  // keyboard and mouse come or go.
  var touchQ = window.matchMedia ? window.matchMedia('(pointer: coarse)') : null;
  function touch() { return !!(touchQ && touchQ.matches); }
  function hintText(el, hold) {
    if (hold) return (touch() && el.getAttribute('data-touch-hold')) || el.getAttribute('data-hold') || hintText(el);
    return (touch() && el.getAttribute('data-touch')) || el._klIdle;
  }
  var hints = all(root, '.pc-hint');
  hints.forEach(function (el) { el._klIdle = text(el); });
  function captions() { hints.forEach(function (el) { if (!el._klBusy) el.textContent = hintText(el); }); }
  captions();
  if (touchQ && touchQ.addEventListener) touchQ.addEventListener('change', captions);

  // a polite line for screen readers; cleared first so the same words are read again
  function say(piece, words) {
    var el = one(piece, '[data-say]');
    if (!el) return;
    el.textContent = '';
    setTimeout(function () { el.textContent = words; }, 50);
  }
  function anim(el, frames, opts) {
    if (still() || !el) return null;
    var o = { duration: 420, easing: EASE, fill: 'backwards' };
    for (var k in opts) o[k] = opts[k];
    return el.animate(frames, o);
  }
  function stop(el) { el.getAnimations({ subtree: true }).forEach(function (a) { a.cancel(); }); }
  // move things in the page, then slide each one from where it was to where it is now
  function flip(els, change) {
    if (still()) { change(); return; }
    var was = els.map(function (el) { return el.getBoundingClientRect(); });
    els.forEach(stop);
    change();
    els.forEach(function (el, i) {
      if (!el.isConnected) return;
      var r = el.getBoundingClientRect(), dx = was[i].left - r.left, dy = was[i].top - r.top;
      if (Math.abs(dx) < .5 && Math.abs(dy) < .5) return;
      el.animate([{ transform: 'translate(' + px(dx) + ',' + px(dy) + ')' }, { transform: 'none' }], { duration: 460, easing: EASE });
    });
  }

  // A window's height glides from the old layout to the new one, in step with what slides inside it, so its bottom
  // edge and everything under it move once instead of jumping. A change made mid-glide starts from where the edge is.
  function glideOn(body, ms) {
    var end = function () { if (!body.getAnimations().length) body.style.overflow = ''; };
    var glide = function (change) {
      var h0 = body.getBoundingClientRect().height;
      body.getAnimations().forEach(function (a) { a.cancel(); });
      change();
      if (still()) return;
      var h1 = body.getBoundingClientRect().height;
      if (Math.abs(h1 - h0) < .5) { end(); return; }
      body.style.overflow = 'hidden';
      body.animate([{ height: px(h0) }, { height: px(h1) }], { duration: ms, easing: EASE }).finished.then(end, end);
    };
    // For a change that comes in two steps: keep the edge where it is until the glide that follows.
    glide.hold = function () {
      if (still()) return;
      var h = px(body.getBoundingClientRect().height);
      body.getAnimations().forEach(function (a) { a.cancel(); });
      body.style.overflow = 'hidden';
      body.animate([{ height: h }, { height: h }], { duration: 2000 }).finished.then(end, end);
    };
    return glide;
  }

  /* ---------- 1. never miss: the list fills in, then the syllabus question; Yes adds the quiz, No leaves it ---------- */
  function miss(piece) {
    var win = one(piece, '.pc-win'), list = one(piece, '[data-due]'), found = one(piece, '[data-found]');
    var slot = one(piece, '[data-slot]'), yes = one(piece, '[data-yes]'), no = one(piece, '[data-no]'), again = one(piece, '[data-again]');
    var body = one(win, '.pc-body');
    if (!win || !body || !list || !found || !slot || !yes || !no || !again) return null;
    var glide = glideOn(body, 460); // the same 460 ms as the rows sliding inside it
    found.remove();
    win.setAttribute('data-state', 'wait');
    function state(s) { if (s) win.setAttribute('data-state', s); return win.getAttribute('data-state'); }
    function moving() { return all(list, '.pc-row').concat(slot); }

    var turn = 0; // a quick Yes while the quiz is still fading out wins over the fade
    function add() {
      turn++;
      stop(found);
      var d = +found.getAttribute('data-date');
      var before = all(list, '.pc-row').filter(function (r) { return +r.getAttribute('data-date') > d; })[0] || null;
      flip(moving(), function () { list.insertBefore(found, before); });
      anim(found, [{ opacity: 0, transform: 'translateY(-.5em)' }, { opacity: 1, transform: 'none' }], { duration: 480, delay: 140 });
      anim(one(found, '.pc-mk'), [{ transform: 'scale(0)' }, { transform: 'none' }], { duration: 420, delay: 340 });
    }
    function take() {
      var mine = ++turn;
      var gone = function () {
        if (mine !== turn) return;
        glide(function () { flip(moving(), function () { found.remove(); }); });
        stop(found);
      };
      var a = anim(found, [{ opacity: 1 }, { opacity: 0 }], { duration: 200, fill: 'forwards', easing: 'ease-out' });
      if (a) a.finished.then(gone, gone); else gone();
    }

    yes.addEventListener('click', function () {
      if (state() !== 'ask') return;
      glide(function () { add(); state('yes'); });
      again.focus();
      say(piece, 'Added. Quiz 4 for CHEM 110 is on your list for Wed 11, from page 3 of the syllabus.');
    });
    no.addEventListener('click', function () {
      if (state() !== 'ask') return;
      glide(function () { state('no'); });
      again.focus();
      say(piece, 'Not added. Your list is unchanged.');
    });
    again.addEventListener('click', function () {
      var s = state();
      if (s !== 'yes' && s !== 'no') return;
      // With the quiz on the list, the row fades before it leaves: the edge waits for it and then moves once.
      if (found.isConnected) { glide.hold(); take(); state('ask'); }
      else glide(function () { state('ask'); });
      yes.focus();
      say(piece, 'Asking again. Found in your CHEM 110 syllabus, page 3: Quiz 4, Wed Nov 11, in class?');
    });

    return {
      build: function () {
        all(list, '[data-arrive]').forEach(function (li) {
          anim(li, [{ opacity: 0, transform: 'translateY(-.6em)' }, { opacity: 1, transform: 'none' }],
            { duration: 460, delay: 260 + (+li.getAttribute('data-arrive')) * 240 });
        });
        if (still()) { state('ask'); return; }
        setTimeout(function () { if (state() === 'wait') state('ask'); }, 1550);
      }
    };
  }

  /* ---------- 2. what to do now: the list sorts itself; Start finishes the top item, Later sends it down ---------- */
  var NOW = 14 * 60 + 40, LAB = 15 * 60 + 30, LAB_END = 18 * 60 + 20;
  function hm(m) {
    var h = Math.floor(m / 60), mm = m % 60;
    return (h % 12 || 12) + ':' + (mm < 10 ? '0' : '') + mm + (h >= 12 ? ' PM' : ' AM');
  }
  function now(piece) {
    var win = one(piece, '.pc-win'), top = one(piece, '[data-top]'), then = one(piece, '[data-then]');
    var start = one(piece, '[data-start]'), later = one(piece, '[data-later]'), over = one(piece, '[data-over]');
    var empty = one(piece, '[data-empty]'), clock = one(piece, '[data-clock]');
    if (!win || !top || !then || !start || !later || !over || !empty || !clock) return null;
    var items = all(piece, '.pc-it'), body = one(win, '.pc-body');
    if (items.length < 2 || !body) return null;
    var order = items.slice(), t = NOW, busy = false;

    var glide = glideOn(body, 520); // the same 520 ms as the items sliding inside it

    function mins(li) { return +li.getAttribute('data-min') || 0; }
    function fits(li) { return t < LAB && t + mins(li) <= LAB; }
    // a thing that does not fit before the lab is done after it
    function finish(li) { return fits(li) ? t + mins(li) : Math.max(t, LAB_END) + mins(li); }
    // What does not fit before the lab, while the lab is still ahead, says that it comes after it. The words are
    // data-wait on the list in index.html.
    var wait = top.getAttribute('data-wait') || '';
    function place() {
      order.forEach(function (li, i) {
        var late = i === 0 && !fits(li);
        var eve = one(li, '.pc-fit[data-fit="eve"]'), was = eve && one(eve, '.pc-wait');
        (i ? then : top).appendChild(li);
        li.classList.toggle('is-late', late);
        if (was) was.remove();
        if (eve && wait && late && t < LAB_END) {
          var s = document.createElement('span');
          s.className = 'pc-wait';
          s.textContent = wait + ' ';
          eve.insertBefore(s, eve.firstChild);
        }
      });
      win.classList.toggle('is-one', order.length < 2);
      win.classList.toggle('is-empty', !order.length);
      empty.hidden = !!order.length;
      clock.textContent = 'Tue ' + hm(t);
    }
    function reason(li) {
      return one(li, '.pc-fit[data-fit="' + (li.classList.contains('is-late') ? 'eve' : 'now') + '"]') || one(li, '.pc-fit');
    }
    function describe(li) {
      return text(one(li, '.pc-t2')) + ', ' + text(one(li, '.pc-m')).replace(/ · /g, ', ') + '. ' + text(reason(li));
    }
    // the same slide as flip(), and the title grows or shrinks as it moves between the top and the list
    function reorder(change) {
      if (still()) { change(); return; }
      var els = all(win, '.pc-it');
      var was = els.map(function (li) {
        var tt = one(li, '.pc-t2');
        return { li: li.getBoundingClientRect(), t: tt.getBoundingClientRect(), fs: parseFloat(getComputedStyle(tt).fontSize) };
      });
      els.forEach(stop);
      glide(change);
      els.forEach(function (li, i) {
        if (!li.isConnected) return;
        var tt = one(li, '.pc-t2'), a = was[i], r = li.getBoundingClientRect(), rt = tt.getBoundingClientRect();
        var fs = parseFloat(getComputedStyle(tt).fontSize), dx = a.li.left - r.left, dy = a.li.top - r.top;
        if (Math.abs(dx) > .5 || Math.abs(dy) > .5) {
          li.animate([{ transform: 'translate(' + px(dx) + ',' + px(dy) + ')' }, { transform: 'none' }], { duration: 520, easing: EASE });
        }
        if (Math.abs(fs - a.fs) > .1) {
          var tx = a.t.left - rt.left - dx, ty = a.t.top - rt.top - dy;
          tt.animate([{ transform: 'translate(' + px(tx) + ',' + px(ty) + ') scale(' + (a.fs / fs) + ')' }, { transform: 'none' }], { duration: 520, easing: EASE });
        }
      });
      if (order[0]) anim(reason(order[0]), [{ opacity: 0 }, { opacity: 1 }], { duration: 380, delay: 300 });
    }
    function arrange(ix) { order = ix.map(function (i) { return items[i]; }); place(); }

    start.addEventListener('click', function () {
      if (busy || !order.length || win.classList.contains('is-sorting')) return;
      busy = true;
      var li = order[0], name = text(one(li, '.pc-t2'));
      var a = anim(li, [{ opacity: 1, transform: 'none' }, { opacity: 0, transform: 'translateX(1.5em)' }], { duration: 240, fill: 'forwards', easing: 'ease-in' });
      var next = function () {
        t = finish(li);
        order.shift();
        reorder(function () { li.remove(); place(); });
        stop(li);
        busy = false;
        if (!order.length) {
          over.focus();
          say(piece, 'Done: ' + name + '. Nothing else due today. It is ' + hm(t) + '.');
          return;
        }
        say(piece, 'Done: ' + name + '. It is now ' + hm(t) + '. Next: ' + describe(order[0]));
      };
      if (a) a.finished.then(next, next); else next();
    });
    later.addEventListener('click', function () {
      if (busy || !order.length || win.classList.contains('is-sorting')) return;
      if (order.length < 2) { say(piece, 'That is the last thing on your list.'); return; }
      var name = text(one(order[0], '.pc-t2'));
      order.push(order.shift());
      reorder(place);
      say(piece, 'Moved ' + name + ' to the end. Next: ' + describe(order[0]));
    });
    over.addEventListener('click', function () {
      order = items.slice();
      t = NOW;
      glide(place);
      order.forEach(function (li, i) {
        anim(li, [{ opacity: 0, transform: 'translateY(-.5em)' }, { opacity: 1, transform: 'none' }], { duration: 420, delay: i * 70 });
      });
      start.focus();
      say(piece, 'Starting over at ' + hm(t) + '. Next: ' + describe(order[0]));
    });

    // before it arrives the list sits out of order; the HTML order is the sorted one
    if (!still()) { win.classList.add('is-sorting'); arrange([1, 4, 0, 3, 2]); } else place();

    return {
      build: function () {
        if (still()) { win.classList.remove('is-sorting'); place(); return; }
        setTimeout(function () { reorder(function () { arrange([2, 0, 3, 1, 4]); }); }, 650);
        setTimeout(function () { reorder(function () { arrange([0, 1, 2, 3, 4]); }); }, 1250);
        setTimeout(function () { win.classList.remove('is-sorting'); }, 1700);
      }
    };
  }

  /* ---------- 3. get ahead: pick a block up, put it on a day; never on or after its deadline, never on top of something ---------- */
  function ahead(piece) {
    var week = one(piece, '[data-week]'), hint = one(piece, '[data-hint]');
    if (!week || !hint) return null;
    var days = all(week, '.pc-day');
    var held = null, drag = null, skip = false;

    function label(day) { return text(one(day, '.pc-dh')); }
    function span(el) { var p = (el.getAttribute('data-at') || '0-0').split('-'); return [+p[0], +p[1]]; }
    function tm(el) { return text(one(el, '.pc-tm')); }
    function dayOf(el) { return el.closest('.pc-day'); }
    function what(b) { var n = one(b, '[data-n]'); return text(one(b, 'b')) + (n ? ', ' + text(n) : '') + ', ' + tm(b); }
    function why(b, day) {
      if (+day.getAttribute('data-date') >= +b.getAttribute('data-due')) {
        return 'Not ' + label(day) + '. It has to come before ' + b.getAttribute('data-before') + '.';
      }
      var s = span(b);
      var hit = all(day, '[data-at]').filter(function (o) { var x = span(o); return o !== b && s[0] < x[1] && x[0] < s[1]; })[0];
      return hit ? 'Not ' + label(day) + '. It would overlap ' + hit.getAttribute('data-name') + ', ' + tm(hit) + '.' : '';
    }
    function blocks() { return all(week, '[data-move]'); }
    function names() {
      var ps = blocks().filter(function (b) { return one(b, '[data-n]'); });
      ps.forEach(function (b, i) { one(b, '[data-n]').textContent = (i + 1) + ' of ' + ps.length; });
      blocks().forEach(function (b) {
        b.setAttribute('aria-label', what(b) + ', ' + label(dayOf(b)));
        if (!b.hasAttribute('aria-pressed')) b.setAttribute('aria-pressed', 'false');
      });
    }
    function insert(b, day) {
      var evs = one(day, '.pc-evs'), s = span(b)[0];
      var next = all(evs, '[data-at]').filter(function (o) { return o !== b && span(o)[0] > s; })[0] || null;
      evs.insertBefore(b, next);
    }
    function hold(b) {
      if (held && held !== b) release();
      held = b;
      b.setAttribute('aria-pressed', 'true');
      week.classList.add('is-holding');
      days.forEach(function (d) {
        var here = d === dayOf(b), w = here ? '' : why(b, d), drop = one(d, '.pc-drop');
        d.classList.toggle('is-no', !!w);
        d.classList.toggle('is-here', here);
        drop.setAttribute('aria-disabled', w ? 'true' : 'false');
        drop.setAttribute('aria-label', w || (here ? 'Keep it on ' + label(d) : 'Move it to ' + label(d)));
      });
      hint._klBusy = true;
      hint.textContent = hintText(hint, true);
      say(piece, 'Picked up ' + what(b) + ', ' + label(dayOf(b)) + '. Choose a day, or press Escape to put it back.');
    }
    function release() {
      if (!held) return;
      held.setAttribute('aria-pressed', 'false');
      held = null;
      week.classList.remove('is-holding');
      days.forEach(function (d) { d.classList.remove('is-no', 'is-here', 'is-over'); });
      hint._klBusy = false;
      hint.textContent = hintText(hint);
    }
    function move(b, day) {
      if (day === dayOf(b)) {
        flip([b], function () { b.style.transform = ''; release(); });
        b.focus({ preventScroll: true });
        say(piece, 'Left on ' + label(day) + '.');
        return;
      }
      flip(all(week, '.pc-ev'), function () { b.style.transform = ''; insert(b, day); names(); release(); });
      b.focus({ preventScroll: true });
      say(piece, 'Moved ' + what(b) + ' to ' + label(day) + '.');
    }
    function refuse(day, w) {
      hint.textContent = w;
      say(piece, w);
      anim(day, [{ transform: 'none' }, { transform: 'translateX(-4px)' }, { transform: 'translateX(4px)' }, { transform: 'none' }],
        { duration: 260, easing: 'ease-out' });
    }

    week.addEventListener('click', function (e) {
      if (skip) { skip = false; return; }
      var drop = e.target.closest('.pc-drop'), b = e.target.closest('[data-move]');
      if (drop && held) {
        var d = dayOf(drop);
        if (drop.getAttribute('aria-disabled') === 'true') refuse(d, why(held, d)); else move(held, d);
        return;
      }
      if (!b) return;
      if (held === b) { release(); say(piece, 'Put back. Nothing moved.'); } else hold(b);
    });
    week.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && held) {
        var h = held;
        release();
        h.focus();
        say(piece, 'Put back. Nothing moved.');
        e.preventDefault();
        return;
      }
      var b = e.target.closest('[data-move]'), dir = { ArrowLeft: -1, ArrowUp: -1, ArrowRight: 1, ArrowDown: 1 }[e.key];
      if (!b || !dir) return;
      e.preventDefault();
      var i = days.indexOf(dayOf(b)), first = '';
      for (var j = i + dir; j >= 0 && j < days.length; j += dir) {
        var w = why(b, days[j]);
        if (!w) { move(b, days[j]); return; }
        if (!first) first = w;
      }
      first = first || (dir < 0 ? 'Nothing earlier: this week starts Mon 9.' : 'Nothing later: this week ends Mon 16.');
      hint.textContent = first;
      say(piece, first);
    });
    document.addEventListener('pointerdown', function (e) { if (held && !week.contains(e.target)) release(); });

    // a mouse or pen can also drag a block; a finger taps, so the page still scrolls
    function dayAt(x, y) {
      var hits = document.elementsFromPoint(x, y);
      for (var i = 0; i < hits.length; i++) {
        if (drag.b.contains(hits[i])) continue;
        var d = hits[i].closest && hits[i].closest('.pc-day');
        if (d && week.contains(d)) return d;
      }
      return null;
    }
    week.addEventListener('pointerdown', function (e) {
      var b = e.target.closest('[data-move]');
      if (!b || e.button !== 0 || e.pointerType === 'touch') return;
      drag = { b: b, x: e.clientX, y: e.clientY, id: e.pointerId, on: false, over: null };
    });
    week.addEventListener('pointermove', function (e) {
      if (!drag || e.pointerId !== drag.id) return;
      var dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (!drag.on) {
        if (dx * dx + dy * dy < 36) return;
        drag.on = true;
        if (held !== drag.b) hold(drag.b);
        try { drag.b.setPointerCapture(e.pointerId); } catch (err) { /* the pointer already left */ }
        drag.b.classList.add('is-drag');
      }
      drag.b.style.transform = 'translate(' + px(dx) + ',' + px(dy) + ')';
      var over = dayAt(e.clientX, e.clientY);
      if (over !== drag.over) {
        if (drag.over) drag.over.classList.remove('is-over');
        drag.over = over;
        if (over && !over.classList.contains('is-no')) over.classList.add('is-over');
      }
    });
    function end(e, drop) {
      if (!drag || e.pointerId !== drag.id) return;
      var d = drag;
      drag = null;
      if (!d.on) return;
      skip = true;
      setTimeout(function () { skip = false; }, 0);
      d.b.classList.remove('is-drag');
      if (d.over) d.over.classList.remove('is-over');
      var w = drop && d.over ? why(d.b, d.over) : '';
      if (drop && d.over && (!w || d.over === dayOf(d.b))) { move(d.b, d.over); return; }
      flip([d.b], function () { d.b.style.transform = ''; release(); });
      if (w) { hint.textContent = w; say(piece, w); }
    }
    week.addEventListener('pointerup', function (e) { end(e, true); });
    week.addEventListener('pointercancel', function (e) { end(e, false); });

    names();
    return {
      build: function () {
        if (still()) return;
        var mark = one(week, '[data-due-mark]'), r0 = mark ? mark.getBoundingClientRect() : null, k = 0, s = 0;
        blocks().forEach(function (b) {
          if (one(b, '[data-n]') && r0) {
            // the problem set's sittings come out of its due date
            var r = b.getBoundingClientRect(), from = 'translate(' + px(r0.left - r.left) + ',' + px(r0.top - r.top) + ') scale(.7)';
            anim(b, [{ opacity: 0, transform: from }, { opacity: 1, transform: from, offset: .12 }, { opacity: 1, transform: 'none' }],
              { duration: 760, delay: 1000 + (k++) * 200 });
          } else {
            anim(b, [{ opacity: 0, transform: 'translateY(-.9em)' }, { opacity: 1, transform: 'none' }], { duration: 480, delay: 360 + (s++) * 170 });
          }
        });
      }
    };
  }

  /* ---------- start each piece once its window is on screen ---------- */
  var KINDS = { miss: miss, now: now, ahead: ahead };
  var pieces = all(root, '[data-piece]').map(function (el) {
    var make = KINDS[el.getAttribute('data-piece')], api = make && make(el);
    return api ? { el: el, api: api, on: false } : null;
  }).filter(Boolean);
  root.classList.add('pc-on');

  function go(P) {
    if (P.on) return;
    P.on = true;
    var w = one(P.el, '[data-build]');
    if (w) w.classList.add('is-in');
    P.api.build();
  }
  // js/site.js marks [data-build] as it arrives too; whichever comes first starts the piece
  document.addEventListener('kl:in', function (e) {
    var el = e.target && e.target.closest && e.target.closest('[data-piece]');
    pieces.forEach(function (P) { if (P.el === el) go(P); });
  });
  if (still() || !('IntersectionObserver' in window)) { pieces.forEach(go); return; }
  var io = new IntersectionObserver(function (es) {
    es.forEach(function (e) {
      if (!e.isIntersecting) return;
      pieces.forEach(function (P) { if (P.el.contains(e.target)) go(P); });
      io.unobserve(e.target);
    });
  }, { threshold: 0.3 });
  pieces.forEach(function (P) { var w = one(P.el, '[data-build]'); if (w) io.observe(w); else go(P); });
})();
