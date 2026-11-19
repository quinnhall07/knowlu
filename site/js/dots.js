/* Knowlu dots: the dot grid behind the hero and behind the price band. Gray dots on the section's own ground, tied
   together like a spring mesh.
   - Color. Under the grid lies a field of the four class colors that turns slowly on its own (the swirl, below).
     The grid is gray at rest. A dot shows the color it sits in only while something stirs it: the pointer's swell,
     a press and hold, a knock from the logo strip, the big ripple when someone joins. The more a dot is stirred the
     more color it takes, so nothing stirred is ever left a dirty gray.
   - The swell follows the pointer and leaves a short trail that fades where it was. Pressing and holding gathers the
     dots; typing an email sends a small gray ripple from the caret; joining sends a big one in color; clicking the
     logo strip under the hero knocks the hero's grid from below.
   - Pointing at or focusing a sign-up box clears the dots from under it and tints the ones around it red.
   - On the hero, when nobody is pointing, a soft swell wanders on its own under and beside the words, so a phone
     (which has no pointer) shows the colors too. It stops while the pointer is there or the form is in use.
   - The grid swells slowly on its own. Reduced motion draws one still frame.
   Colors come from each section's CSS (css/hero.css, css/price.css and the tokens) and are read again when the theme
   changes. The numbers are Quinn's settings from the prototype (spacing 22, size 1.2, reach 170, grow 55, push 25,
   neighbors 55, wobble 70, give 50, wave 30, speed 25, scale 60; swell, hold, typing and the sign-up box on). */
(function () {
  'use strict';

  var root = document.documentElement;
  var reducedQ = window.matchMedia('(prefers-reduced-motion: reduce)');
  var darkQ = window.matchMedia('(prefers-color-scheme: dark)');
  var TAU = Math.PI * 2;

  /* ---------- settings ---------- */
  var SPACING = 22;        /* px between dots */
  var SIZE = 1.2;          /* resting dot radius, px */
  var REACH = 170;         /* radius of the swell around the pointer, px */
  var GROW = 0.55;         /* how much the swell grows the dots */
  var PUSH = 0.25 * 6;     /* how hard the pointer pushes dots outward */
  var NEIGHBORS = 0.55;    /* how much a dot drags the dots beside it */
  var WOBBLE = 0.70;       /* how long a dot keeps moving */
  var GIVE = 0.50;         /* how far a dot gives */
  var WAVE = 0.30;         /* the grid's own slow swell */
  var WAVE_SPEED = 0.25;
  var WAVE_SCALE = 0.60;
  var TRAIL = 0.34;        /* seconds the swell lingers where the pointer was */
  var LEVELS = 16;         /* steps from gray to full class color */
  var TINTS = 6;           /* steps of the sign-up box's red tint */
  var SWIRL_SCALE = 620;   /* about how far apart the color field's waves are, px (smaller on a phone) */
  var SWIRL_TURN = 38;     /* seconds for the colors to come once around */
  var SWIRL_TWIST = 1.5;   /* how much the color arms curl around each meeting point */
  var SEAM = 15;           /* a color thins to gray this close to the next color, px */
  var WANDER_WAIT = 2.5;   /* seconds without a pointer before the hero's own swell starts */
  var WANDER_AMP = 0.8;    /* its strength beside the pointer's */

  function still() { return reducedQ.matches; }
  function rgb(c) { return 'rgb(' + c[0] + ',' + c[1] + ',' + c[2] + ')'; }
  function mix(a, b, t) {
    return [Math.round(a[0] + (b[0] - a[0]) * t), Math.round(a[1] + (b[1] - a[1]) * t), Math.round(a[2] + (b[2] - a[2]) * t)];
  }
  var probe = document.createElement('canvas').getContext('2d');
  function parse(v, fb) {
    v = String(v || '').trim();
    if (!v || !probe) return fb;
    probe.fillStyle = '#010203';
    probe.fillStyle = v;
    var s = probe.fillStyle;
    if (s === '#010203') return fb;
    if (s.charAt(0) === '#' && s.length === 7) return [parseInt(s.slice(1, 3), 16), parseInt(s.slice(3, 5), 16), parseInt(s.slice(5, 7), 16)];
    var m = /rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/.exec(s);
    return m ? [+m[1], +m[2], +m[3]] : fb;
  }

  /* ---------- the swirl ----------
     Five long waves cross the section, each in its own direction. Added up they give every spot a point on a dial:
     an angle and a strength. The dial's four quarters are the four classes, in the order of the rainbow (amber,
     green, blue, violet) so the colors that meet are never far apart. Where the sum passes through nothing all four
     colors meet, and because every wave moves on, the colors turn around those points like slow pinwheels, each one
     the other way from the ones beside it. The twist curls each color into an arm that trails behind the turn.
     Near the line between two colors a dot thins to gray, so no dot ever flips from one color to the next.
     Waves are [direction, length, weight, start, pace]; a pace a little off 1 lets the meeting points wander. */
  var WAVES = [[0.35, 1.0, 1.0, 0.4, 1.0], [1.45, 0.82, 0.9, 2.9, 1.07], [2.6, 1.18, 0.8, 5.1, 0.93],
    [3.9, 0.9, 0.85, 1.7, 1.12], [5.2, 1.3, 0.7, 3.6, 0.9]];
  var DIAL = [2, 1, 0, 3];   /* dial quarter to class: SPAN amber, CHEM green, CS blue, PSYC violet */
  function field(scale, seed) {
    var sum = 0, om = TAU / SWIRL_TURN;
    WAVES.forEach(function (w) { sum += w[2] * w[2]; });
    return WAVES.map(function (w, i) {
      var k = TAU / (scale * w[1]), dir = w[0] + seed;
      return { kx: k * Math.cos(dir), ky: k * Math.sin(dir), a: w[2] / Math.sqrt(sum), ph: w[3] + 2.4 * seed * (i + 1), om: om * w[4] };
    });
  }

  /* ---------- one grid: host is the section, cfg says what is special about it ---------- */
  function make(host, cfg) {
    var cv = host.querySelector(cfg.canvas);
    var ctx = cv && cv.getContext && cv.getContext('2d');
    if (!ctx) return null;

    var raf = 0, visible = true;
    var W = 0, H = 0, DPR = 1, small = false;
    var col = { paper: [255, 255, 255], dot: [107, 100, 108], hot: [195, 49, 55],
      cls: [[47, 85, 230], [20, 136, 95], [242, 171, 31], [122, 75, 209]] };
    var ptr = { cx: 0, cy: 0, on: false, touch: false, at: 0, down: false, last: 1.2 - WANDER_WAIT };
    var lens = { x: 0, y: 0, fresh: true, who: 0 };   /* who aims the swell: 0 nobody, 1 the pointer, 2 the hero's own wanderer */
    var boxes = [];
    var clock0 = performance.now();
    function now() { return (performance.now() - clock0) / 1000; }

    /* ---------- color, read from the section's CSS so the theme sets it ---------- */
    function readColours() {
      var cs = getComputedStyle(host);
      col.paper = parse(cs.backgroundColor, col.paper);
      col.dot = parse(cs.getPropertyValue(cfg.dot), col.dot);
      col.hot = parse(cs.getPropertyValue(cfg.hot), col.hot);
      col.cls = ['--c-cs', '--c-chem', '--c-span', '--c-psyc'].map(function (k, i) { return parse(cs.getPropertyValue(k), col.cls[i]); });
      AT = null;
    }

    /* ---------- geometry ---------- */
    var FW = field(SWIRL_SCALE, cfg.seed);
    function size() {
      var r = host.getBoundingClientRect();
      W = r.width; H = r.height; small = W < 600;
      /* sharp on a 2x screen, but never a backing store past about 5.6 million pixels: a big canvas costs every frame */
      DPR = Math.min(window.devicePixelRatio || 1, 2);
      if (W * H * DPR * DPR > 5.6e6) DPR = Math.max(1, Math.sqrt(5.6e6 / (W * H)));
      cv.width = Math.max(1, Math.round(W * DPR));
      cv.height = Math.max(1, Math.round(H * DPR));
      ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
      FW = field(small ? 0.62 * SWIRL_SCALE : SWIRL_SCALE, cfg.seed);
      measureBox();
      G = null; AT = null;
      if (blob.el) placeBlob();
    }
    /* the words and the form: the dots stay quieter behind them so the words stay easy to read. Each entry of
       cfg.quiet is one box (a list of selectors makes one box around all of them). The small words keep plain paper
       behind them too: tb holds their outlines, and a stirred dot stays gray there instead of taking a color. */
    var tb = [];
    function measureBox() {
      var r = host.getBoundingClientRect();
      function rel(b) { return { l: b.left - r.left, t: b.top - r.top, r: b.right - r.left, b: b.bottom - r.top }; }
      boxes = [];
      cfg.quiet.forEach(function (group) {
        var box = null;
        group.forEach(function (sel) {
          var el = host.querySelector(sel), b = el && el.getBoundingClientRect();
          if (!b || !b.width) return;
          var q = rel(b);
          box = box ? { l: Math.min(box.l, q.l), t: Math.min(box.t, q.t), r: Math.max(box.r, q.r), b: Math.max(box.b, q.b) } : q;
        });
        if (box) boxes.push(box);
      });
      tb = [];
      var rg = document.createRange();
      Array.prototype.forEach.call(host.querySelectorAll(cfg.plain), function (el) {
        if (!el.offsetParent && el.getClientRects().length === 0) return;
        rg.selectNodeContents(el);
        var b = rg.getBoundingClientRect();
        if (b.width && b.height) tb.push(rel(b));
      });
    }
    function plain(x, y) {
      var m = 1;
      for (var i = 0; i < tb.length; i++) {
        var b = tb[i], dx = Math.max(b.l - x, 0, x - b.r), dy = Math.max(b.t - y, 0, y - b.b);
        var d = Math.sqrt(dx * dx + dy * dy), v = d < 8 ? 0 : Math.min(1, (d - 8) / 36);
        if (v < m) m = v;
      }
      return m;
    }
    function quiet(x, y) {
      var m = 1;
      for (var i = 0; i < boxes.length; i++) {
        var b = boxes[i], dx = Math.max(b.l - x, 0, x - b.r), dy = Math.max(b.t - y, 0, y - b.b);
        var v = 0.4 + 0.6 * Math.min(Math.sqrt(dx * dx + dy * dy) / 56, 1);
        if (v < m) m = v;
      }
      return m;
    }
    function quietAll() {
      for (var k = 0; k < G.n; k++) {
        G.q[k] = 0.25 + 0.75 * quiet(G.hx[k], G.hy[k]);
        G.pq[k] = plain(G.hx[k], G.hy[k]);
      }
    }
    /* the form changed size (joined, or back): quiet the dots behind its new outline without moving any of them */
    function requiet() {
      measureBox();
      if (G) quietAll();
      kick();
    }

    /* which color a spot sits in right now (sw.c), and how far inside it: 0 on the line to the next color, 1 from
       SEAM px in (sw.e). Only stirred dots ask, a few hundred a frame at most. */
    var sw = { c: 0, e: 0 };
    function swirl(x, y, t) {
      var X = 0, Y = 0, Xx = 0, Xy = 0, Yx = 0, Yy = 0;
      for (var i = 0; i < FW.length; i++) {
        var w = FW[i], p = w.kx * x + w.ky * y + w.ph - w.om * t, s = w.a * Math.sin(p), c = w.a * Math.cos(p);
        X += c; Y += s; Xx -= s * w.kx; Xy -= s * w.ky; Yx += c * w.kx; Yy += c * w.ky;
      }
      var m2 = X * X + Y * Y + 1e-9, m = Math.sqrt(m2);
      /* the angle on the dial, curled by the twist, and how fast it changes across the page */
      var th = Math.atan2(Y, X) + SWIRL_TWIST * m;
      var gx = (X * Yx - Y * Xx) / m2 + SWIRL_TWIST * (X * Xx + Y * Yx) / m;
      var gy = (X * Yy - Y * Xy) / m2 + SWIRL_TWIST * (X * Xy + Y * Yy) / m;
      var u = th / TAU * 4;
      u -= Math.floor(u / 4) * 4;
      var q = u | 0;
      if (q > 3) q = 3;
      var f = u - q;
      if (f > 0.5) f = 1 - f;
      var e = f * (TAU / 4) / (Math.sqrt(gx * gx + gy * gy) + 1e-6) / SEAM;
      sw.c = DIAL[q]; sw.e = e >= 1 ? 1 : e * e * (3 - 2 * e);
    }

    /* ---------- the sign-up box ---------- */
    var dl = { amp: 0, acc: 0 };
    var blob = { hover: null, focus: null, el: null, amt: 0, cx: 0, cy: 0, ex: 0, ey: 0, rr: 24 };
    var BLOB_FALL = 52, BLOB_PAD = 8;
    function placeBlob() {
      var el = blob.el, b = el && el.getBoundingClientRect();
      if (!b || !b.width) return;
      /* the note under the box goes inside the shape too, so no dot sits on its words */
      var l = b.left, tp = b.top, rt = b.right, bt = b.bottom, f = el.closest('form'), msg = f && f.querySelector('.wl-msg');
      if (msg && el.classList.contains('wl-row') && msg.offsetHeight && msg.textContent.trim()) {
        var m = msg.getBoundingClientRect();
        l = Math.min(l, m.left); tp = Math.min(tp, m.top); rt = Math.max(rt, m.right); bt = Math.max(bt, m.bottom);
      }
      var r = host.getBoundingClientRect(), hw = (rt - l) / 2 + BLOB_PAD, hh = (bt - tp) / 2 + BLOB_PAD;
      blob.rr = Math.min(24, hw, hh);
      blob.cx = (l + rt) / 2 - r.left; blob.cy = (tp + bt) / 2 - r.top;
      blob.ex = hw - blob.rr; blob.ey = hh - blob.rr;
    }
    /* distance from a point to the box's rounded rectangle (negative inside); sn is the outward direction */
    var sn = { x: 0, y: 0 };
    function boxDist(px, py) {
      var sx = px < blob.cx ? -1 : 1, sy = py < blob.cy ? -1 : 1;
      var ax = Math.abs(px - blob.cx) - blob.ex, ay = Math.abs(py - blob.cy) - blob.ey;
      if (ax > 0 && ay > 0) {
        var o = Math.sqrt(ax * ax + ay * ay);
        sn.x = sx * ax / o; sn.y = sy * ay / o;
        return o - blob.rr;
      }
      if (ax > ay) { sn.x = sx; sn.y = 0; return ax - blob.rr; }
      sn.x = 0; sn.y = sy; return ay - blob.rr;
    }
    /* a sign-up row that has already sent its link is done: the dots stop parting around it (the invite row still does) */
    function rowOf(el) {
      var row = el && el.closest ? el.closest('.wl-row, .nx-row') : null;
      return row && row.classList.contains('wl-row') && row.closest('form.ok') ? null : row;
    }

    /* ---------- ripples: each runs its full course on its own clock and they add up ----------
       s is how strong. A ripple of 0.5 or more (a knock, joining) shows the class colors as it passes; a smaller one
       (typing) stays gray. Only past ten at once does the oldest fade early. */
    var rings = [];
    function ripple(x, y, s) {
      if (still()) return;
      var t = now();
      var r = { x: x, y: y, t: t, s: s, kill: -1,
        R: s > 1 ? 0.75 * Math.sqrt(W * W + H * H) : s > 0.5 ? Math.max(320, 2.1 * REACH) : Math.max(210, 1.35 * REACH),
        D: s > 1 ? 1.5 : s > 0.5 ? 0.85 : 0.6 };
      var live = 0, i;
      for (i = 0; i < rings.length; i++) if (rings[i].kill < 0) live++;
      for (i = 0; live >= 10 && i < rings.length; i++) if (rings[i].kill < 0) { rings[i].kill = t; live--; }
      rings.push(r);
      kick();
    }

    /* ---------- the mesh ---------- */
    var G = null;
    function buildGrid() {
      var s = SPACING;
      while ((Math.floor(W / s) + 1) * (Math.floor(H / s) + 1) > 16000) s += 1;
      var cols = Math.floor(W / s) + 1, rows = Math.floor(H / s) + 1, n = cols * rows;
      var ox = (W - (cols - 1) * s) / 2, oy = (H - (rows - 1) * s) / 2;
      G = { W: W, H: H, cols: cols, rows: rows, n: n, ringOn: false, asleep: true,
        hx: new Float32Array(n), hy: new Float32Array(n), ux: new Float32Array(n), uy: new Float32Array(n),
        vx: new Float32Array(n), vy: new Float32Array(n), q: new Float32Array(n), pq: new Float32Array(n),
        ht: new Float32Array(n),
        rb: new Float32Array(n), rc: new Float32Array(n), rx: new Float32Array(n), ry: new Float32Array(n) };
      for (var j = 0; j < rows; j++) {
        for (var i = 0; i < cols; i++) {
          var k = j * cols + i;
          G.hx[k] = ox + i * s; G.hy[k] = oy + j * s;
        }
      }
      quietAll();
    }
    /* the rings, worked out once a frame for every dot: rb is how lit a dot is, rc how much of that shows color,
       rx/ry how far the ring carries it */
    function ringPass(t) {
      var live = [];
      rings = rings.filter(function (r) {
        var age = t - r.t, p = age / r.D;
        if (p >= 1 || (r.kill >= 0 && t - r.kill >= 0.16)) return false;
        if (age < 0) return true;
        var env = Math.pow(1 - p, 1.5) * Math.min(1, age / 0.03 + 0.25);
        if (r.kill >= 0) env *= 1 - (t - r.kill) / 0.16;
        var rad = r.R * (1 - (1 - p) * (1 - p)), w = 12 + 26 * p;
        live.push({ x: r.x, y: r.y, rad: rad, w: w, env: env, s: r.s, col: r.s >= 0.5, A: Math.min(12, 7 * r.s),
          lo: Math.max(0, rad - 3 * w), hi: rad + 3 * w });
        return true;
      });
      if (!live.length && !G.ringOn) return false;
      var n = G.n, hx = G.hx, hy = G.hy, rb = G.rb, rc = G.rc, rx = G.rx, ry = G.ry;
      rb.fill(0); rc.fill(0); rx.fill(0); ry.fill(0);
      for (var i = 0; i < live.length; i++) {
        var L = live[i], lo2 = L.lo * L.lo, hi2 = L.hi * L.hi;
        for (var k = 0; k < n; k++) {
          var dx = hx[k] - L.x, dy = hy[k] - L.y, d2 = dx * dx + dy * dy;
          if (d2 < lo2 || d2 > hi2) continue;
          var d = Math.sqrt(d2), off = (d - L.rad) / L.w, g = Math.exp(-off * off), lit = L.s * L.env * g;
          rb[k] += lit;
          if (L.col) rc[k] += lit;
          if (d > 0.5) { var m = -off * g * L.A * L.env * 1.4 / d; rx[k] += dx * m; ry[k] += dy * m; }
        }
      }
      G.ringOn = live.length > 0;
      return G.ringOn;
    }
    /* ---------- a knock from outside: the logo strip under the hero shakes the mesh when it is clicked (js/strip.js) ----------
       A knock on the bottom edge at clientX: the dots near it jump away, swing and settle on their own springs, and a
       ring runs up the grid. s is how hard (1 is the usual knock). */
    function shock(clientX, s) {
      if (!(s > 0) || still()) return;
      if (!G || G.W !== W || G.H !== H) buildGrid();
      G.asleep = false;
      var x = clientX - host.getBoundingClientRect().left, y = H, R = Math.min(340, 0.6 * H), A = 5.5 * Math.min(2, s);
      for (var k = 0; k < G.n; k++) {
        var dx = G.hx[k] - x, dy = G.hy[k] - y, d2 = dx * dx + dy * dy;
        if (d2 >= R * R) continue;
        var d = Math.sqrt(d2), f = 1 - d / R;
        f = f * f * A / (d + 18);
        G.vx[k] += dx * f; G.vy[k] += dy * f;
      }
      ripple(x, y - 2, Math.min(1, 0.55 + 0.4 * s));
      kick();
    }

    var STIFF = 0.11 - 0.09 * GIVE;
    var BOX_PUSH = 0.6;   /* how hard the sign-up box moves the dots beside it aside */
    /* one or more 60 Hz steps; returns false once the mesh has come to rest with nothing pushing it */
    function stepDots(steps) {
      var n = G.n, cols = G.cols, rows = G.rows, hx = G.hx, hy = G.hy, ux = G.ux, uy = G.uy, vx = G.vx, vy = G.vy;
      var k = STIFF, c = 0.2 * NEIGHBORS, damp = 0.78 + 0.2 * WOBBLE;
      var amp = dl.amp, R = REACH, R2 = R * R;
      var push = PUSH * amp;
      var hold = ptr.down ? 1.6 * amp : 0, RH = R * 1.6, RH2 = RH * RH;
      var bs = BOX_PUSH * blob.amt, bReach = BLOB_FALL + blob.rr, lensOn = push !== 0 || hold > 0;
      var forced = lensOn || bs > 0.001;
      if (!forced && G.asleep) return false;
      var idx, st, i, j, peak = 0;
      for (st = 0; st < steps; st++) {
        for (j = 0, idx = 0; j < rows; j++) {
          for (i = 0; i < cols; i++, idx++) {
            var px = hx[idx] + ux[idx], py = hy[idx] + uy[idx], fx = 0, fy = 0;
            if (lensOn) {
              var dx = px - lens.x, dy = py - lens.y, d2 = dx * dx + dy * dy;
              if (d2 < R2) {
                var u = 1 - d2 / R2, f = u * u;
                fx += dx / R * push * f;
                fy += dy / R * push * f;
              }
              if (hold && d2 < RH2) {
                var d = Math.sqrt(d2), g = (1 - d / RH) * hold / (d + 24);
                fx -= dx * g; fy -= dy * g;
              }
            }
            if (bs > 0.001 && Math.abs(px - blob.cx) < blob.ex + bReach && Math.abs(py - blob.cy) < blob.ey + bReach) {
              var sd = boxDist(px, py);
              if (sd < BLOB_FALL) {
                var bf = sd <= 0 ? 1 : 1 - sd / BLOB_FALL;
                bf = bf * bf * bs;
                fx += sn.x * bf; fy += sn.y * bf;
              }
            }
            var lx = -4 * ux[idx], ly = -4 * uy[idx];
            if (i > 0) { lx += ux[idx - 1]; ly += uy[idx - 1]; }
            if (i < cols - 1) { lx += ux[idx + 1]; ly += uy[idx + 1]; }
            if (j > 0) { lx += ux[idx - cols]; ly += uy[idx - cols]; }
            if (j < rows - 1) { lx += ux[idx + cols]; ly += uy[idx + cols]; }
            vx[idx] = (vx[idx] - k * ux[idx] + c * lx + fx) * damp;
            vy[idx] = (vy[idx] - k * uy[idx] + c * ly + fy) * damp;
          }
        }
        for (idx = 0; idx < n; idx++) {
          ux[idx] += vx[idx]; uy[idx] += vy[idx];
          var m2 = ux[idx] * ux[idx] + uy[idx] * uy[idx];
          if (m2 > 3600) { var sc = 60 / Math.sqrt(m2); ux[idx] *= sc; uy[idx] *= sc; vx[idx] *= 0.5; vy[idx] *= 0.5; }
          var e = Math.abs(vx[idx]) + Math.abs(vy[idx]) + 0.05 * (Math.abs(ux[idx]) + Math.abs(uy[idx]));
          if (e > peak) peak = e;
        }
      }
      if (!forced && peak < 0.004) { ux.fill(0); uy.fill(0); vx.fill(0); vy.fill(0); G.asleep = true; return false; }
      G.asleep = false;
      return true;
    }
    /* reduced motion: no simulation, just where the pushes would leave each dot */
    function settleDots() {
      var k = STIFF, push = PUSH * dl.amp, R = REACH, R2 = R * R, bs = BOX_PUSH * blob.amt;
      for (var idx = 0; idx < G.n; idx++) {
        var dx = G.hx[idx] - lens.x, dy = G.hy[idx] - lens.y, d2 = dx * dx + dy * dy, x = 0, y = 0;
        if (d2 < R2) { var u = 1 - d2 / R2, f = u * u; x = dx / R * push * f / k; y = dy / R * push * f / k; }
        if (bs > 0.001) {
          var sd = boxDist(G.hx[idx], G.hy[idx]);
          if (sd < BLOB_FALL) { var bf = sd <= 0 ? 1 : 1 - sd / BLOB_FALL; bf = bf * bf * bs / k * 0.6; x += sn.x * bf; y += sn.y * bf; }
        }
        var m = Math.sqrt(x * x + y * y);
        if (m > 60) { x *= 60 / m; y *= 60 / m; }
        G.ux[idx] = x; G.uy[idx] = y; G.vx[idx] = 0; G.vy[idx] = 0;
      }
    }
    /* the grid's own slow swell: three travelling waves in different directions, so it never looks like one sine */
    var WS = (function () {
      var L = 260 + 640 * WAVE_SCALE, om = 0.3 + 2.0 * WAVE_SPEED;
      return [[0.35, 1, 1, 1], [2.1, 0.63, 1.27, 0.6], [4.0, 0.41, 1.61, 0.4]].map(function (w) {
        return { dx: Math.cos(w[0]), dy: Math.sin(w[0]), k: TAU / (L * w[1]), om: om * w[2], a: w[3] };
      });
    })();

    /* ---------- the fills ----------
       Ten grays from resting to a mid gray. First come those grays under each step of the sign-up box's red tint
       (step 0 is plain gray); then, for each of the 4 classes, LEVELS steps from each gray to the class color.
       Worked out once per color change; every frame fills one path per fill in use. */
    var CB = (TINTS + 1) * 10, NB = CB + 4 * LEVELS * 10;
    var AT = null;
    function atlas() {
      if (AT) return AT;
      var base = mix(col.paper, col.dot, cfg.shade), fills = [], grays = [], T, b, c, l;
      for (b = 0; b < 10; b++) grays.push(mix(base, col.dot, 0.6 * b / 9));
      for (T = 0; T <= TINTS; T++) for (b = 0; b < 10; b++) fills.push(rgb(mix(grays[b], col.hot, 0.5 * T / TINTS)));
      for (c = 0; c < 4; c++) for (l = 1; l <= LEVELS; l++) for (b = 0; b < 10; b++) fills.push(rgb(mix(grays[b], col.cls[c], l / LEVELS)));
      AT = { fills: fills };
      return AT;
    }

    function drawDots(t, dt, steps) {
      if (!G || G.W !== W || G.H !== H) buildGrid();
      var ringing = ringPass(t);
      if (still()) settleDots(); else stepDots(steps || 0);
      var fills = atlas().fills, paths = new Array(NB), b;
      var hx = G.hx, hy = G.hy, ux = G.ux, uy = G.uy, vx = G.vx, vy = G.vy, rb = G.rb, rc = G.rc, rx = G.rx, ry = G.ry;
      var q = G.q, pq = G.pq, ht = G.ht;
      var R2 = REACH * REACH, amp = dl.amp, keep = still() ? 0 : Math.exp(-dt / TRAIL);
      var held = ptr.down && amp > 0.01, RH = REACH * 1.6, RH2 = RH * RH;
      var wa = WAVE, ba = blob.amt > 0.01 ? blob.amt : 0, bReach = BLOB_FALL * 1.4 + blob.rr, asleep = G.asleep;
      for (var idx = 0; idx < G.n; idx++) {
        var x = hx[idx] + ux[idx], y = hy[idx] + uy[idx], z = 0, E = 0, Ib = 0, Ic = 0, T = 0;
        if (ringing) { x += rx[idx]; y += ry[idx]; }
        var ox = 0, oy = 0;
        for (var w = 0; w < 3; w++) {
          var wq = WS[w], ph = wq.k * (wq.dx * hx[idx] + wq.dy * hy[idx]) - wq.om * t, cs = wq.a * Math.cos(ph);
          z += wq.a * Math.sin(ph); ox += wq.dx * cs; oy += wq.dy * cs;
        }
        z /= 2; x += ox * 3 * wa; y += oy * 3 * wa;
        /* the swell: full where the pointer is now, fading where it was */
        var dx = x - lens.x, dy = y - lens.y, d2 = dx * dx + dy * dy, h = ht[idx] * keep;
        if (amp > 0 && d2 < R2) { var u = 1 - d2 / R2, kh = u * u * amp; if (kh > h) h = kh; }
        ht[idx] = h < 0.003 ? 0 : h;
        E = h * GROW;
        if (!asleep) {
          var m2 = ux[idx] * ux[idx] + uy[idx] * uy[idx];
          if (m2 > 0.25) E += Math.min(1, Math.sqrt(m2) / 16) * 0.6;
          /* a dot that is still moving counts as stirred, so a push reads as touch */
          var v2 = vx[idx] * vx[idx] + vy[idx] * vy[idx];
          if (v2 > 0.04) E += Math.min(1, Math.sqrt(v2) / 3) * 0.35;
        }
        if (held && d2 < RH2) E += (1 - Math.sqrt(d2) / RH) * 0.5 * amp;
        if (ringing && rb[idx] > 0.002) { Ib = Math.min(1, rb[idx]) * 0.75; Ic = Math.min(1, rc[idx]) * 0.75; }
        if (ba && Math.abs(hx[idx] - blob.cx) < blob.ex + bReach && Math.abs(hy[idx] - blob.cy) < blob.ey + bReach) {
          var sd = boxDist(hx[idx], hy[idx]), bt = sd <= 0 ? 1 : Math.max(0, 1 - sd / (BLOB_FALL * 1.4));
          if (bt > 0) { bt *= ba; T = (bt * bt * (3 - 2 * bt) * TINTS + 0.5) | 0; Ib += bt * 0.22; }
        }
        /* beside the small words a stirred dot stays small as well as gray, so nothing swells over them */
        var I = Math.min(1, (E * (0.4 + 0.6 * pq[idx]) + (z > 0 ? z * 0.3 * wa : 0)) * q[idx] + Ib);
        var r = SIZE * (1 + 2.8 * I) * (1 + 0.45 * z * wa), rd;
        b = T * 10 + ((I * 9 + 0.5) | 0);
        /* the class color: none until a dot is stirred a little, full once it is stirred well; quieter behind the
           words, and never over the sign-up box's own red */
        if (T === 0) {
          var ca = ((E * q[idx] + Ic) * pq[idx] - 0.055) / 0.3;
          if (ca > 0) {
            if (ca > 1) ca = 1;
            ca = ca * ca * (3 - 2 * ca);
            swirl(hx[idx], hy[idx], t);
            var lv = (ca * sw.e * LEVELS + 0.5) | 0;
            if (lv > 0) b = CB + (sw.c * LEVELS + lv - 1) * 10 + ((I * 9 + 0.5) | 0);
            /* on the line between two colors a dot is smaller as well as grayer, so the arms show in the size too */
            r *= 1 - 0.3 * ca * (1 - sw.e);
          }
        }
        /* the sign-up box: a dot under it is gone, and one at its edge is on its way out */
        if (ba && Math.abs(x - blob.cx) < blob.ex + blob.rr + 8 && Math.abs(y - blob.cy) < blob.ey + blob.rr + 8) {
          var si = boxDist(x, y);
          if (si < 6) { r *= 1 - ba * (si <= -4 ? 1 : (6 - si) / 10); if (r < 0.55) continue; }
        }
        rd = r * DPR;
        if (rd < 0.3) continue;
        var pa = paths[b] || (paths[b] = new Path2D());
        /* under about a device pixel and a half a circle and a square of the same area look alike, and a square is cheaper */
        if (rd < 1.6) { var hs = r * 0.886; pa.rect(x - hs, y - hs, 2 * hs, 2 * hs); }
        else { pa.moveTo(x + r, y); pa.arc(x, y, r, 0, TAU); }
      }
      for (b = 0; b < NB; b++) if (paths[b]) { ctx.fillStyle = fills[b]; ctx.fill(paths[b]); }
    }

    /* The hero's own swell, for when nobody is pointing: it swings like a slow pendulum under the words, from one
       side of them to the other and back in about a minute, a little wider or tighter each time, and never so far
       that the screen's edge cuts it. On a phone the words fill the width, so it runs to and fro along the strip of
       paper under the form. */
    function wander(t) {
      var b = boxes[0] || { l: 0.3 * W, t: 0.3 * H, r: 0.7 * W, b: 0.6 * H };
      var s = 1.25 * Math.sin(0.105 * t + 0.5), k = 1 + 0.12 * Math.sin(0.071 * t + 1.1);
      var cx = (b.l + b.r) / 2, mx = Math.min(0.9 * REACH, 0.22 * W);
      if (small) return { x: Math.max(mx, Math.min(W - mx, cx + 0.5 * W * Math.sin(s))), y: Math.min(H - 14, (b.b + H) / 2 + 10) };
      var cy = (b.t + b.b) / 2, rx = Math.min((b.r - b.l) / 2 + 130, W / 2 - mx) / 1.06, ry = (b.b - b.t) / 2 + 110;
      return { x: Math.max(mx, Math.min(W - mx, cx + k * rx * Math.sin(s))), y: Math.max(0.3 * H, Math.min(H - 24, cy + k * ry * Math.cos(s))) };
    }

    /* one tick of the inputs: the swell follows the pointer closely and is gone where the pointer left (the trail
       fades on its own; the swell never slides off somewhere); the box eases in and out over about a quarter of a second */
    function aimDots(t, dt) {
      var g = null, want = 0, who = 0;
      if (still()) {
        if (cfg.rest) { g = cfg.rest(W, H, small, boxes[0]); want = 1; }
      } else if (ptr.on && !blob.hover && (!ptr.touch || ptr.down || t - ptr.at < 1.2)) {
        var r = host.getBoundingClientRect();
        g = { x: ptr.cx - r.left, y: ptr.cy - r.top }; want = 1; who = 1;
      } else if (cfg.wander && !blob.hover && !blob.focus && (!ptr.on || ptr.touch) && t - ptr.last > WANDER_WAIT) {
        /* a mouse that rests on the section keeps its own swell; a finger that has lifted does not */
        g = wander(t); want = WANDER_AMP; who = 2;
      }
      var bwant = blob.hover || blob.focus ? 1 : 0;
      blob.el = blob.hover || blob.focus || blob.el;
      if (blob.el && (bwant || blob.amt > 0.002)) placeBlob();
      if (still() || lens.fresh) {
        if (g) { lens.x = g.x; lens.y = g.y; }
        lens.fresh = false; lens.who = who;
        dl.amp = want; blob.amt = bwant;
        return;
      }
      /* a new hand on the swell (the pointer arriving, or the wanderer taking over): it starts where that hand is */
      if (g && who !== lens.who) { lens.x = g.x; lens.y = g.y; dl.amp = 0; }
      else if (g) { var a = 1 - Math.exp(-dt / 0.028); lens.x += (g.x - lens.x) * a; lens.y += (g.y - lens.y) * a; }
      lens.who = who;
      dl.amp += (want - dl.amp) * (1 - Math.exp(-dt / (want > dl.amp ? (who === 2 ? 0.6 : 0.06) : 0.1)));
      if (Math.abs(want - dl.amp) < 0.002) dl.amp = want;
      blob.amt += (bwant - blob.amt) * (1 - Math.exp(-dt / 0.08));
      if (Math.abs(bwant - blob.amt) < 0.002) blob.amt = bwant;
    }

    /* ---------- the loop: runs only while the section is on screen and the tab is showing ---------- */
    var lastMs = 0;
    function frame() {
      raf = 0;
      var ms = performance.now();
      var dt = lastMs ? Math.min(0.05, Math.max(0, (ms - lastMs) / 1000)) : 1 / 60;
      lastMs = ms;
      var t = still() ? 0 : now();
      /* the mesh steps at a fixed 60 a second whatever the screen's rate, at most 3 a frame */
      dl.acc += dt;
      var steps = Math.min(3, Math.floor(dl.acc * 60 + 1e-6));
      dl.acc = steps === 3 ? 0 : dl.acc - steps / 60;
      aimDots(t, dt);
      ctx.clearRect(0, 0, W, H);
      drawDots(t, dt, steps);
      if (!still() && visible && !document.hidden) raf = requestAnimationFrame(frame);
      else { lastMs = 0; dl.acc = 0; }
    }
    function kick() { if (!raf) raf = requestAnimationFrame(frame); }

    /* ---------- input ---------- */
    host.addEventListener('pointermove', function (e) {
      /* a finger counts only while it is down (before the page takes it for a scroll) */
      if (e.pointerType === 'touch' && !ptr.down) return;
      ptr.cx = e.clientX; ptr.cy = e.clientY; ptr.on = true; ptr.touch = e.pointerType === 'touch';
      ptr.last = now();
      if (ptr.touch) ptr.at = ptr.last;
      var row = rowOf(e.target);
      blob.hover = !ptr.touch && row && host.contains(row) ? row : null;
      kick();
    });
    host.addEventListener('pointerleave', function (e) {
      if (e.pointerType === 'touch') return;
      ptr.on = false; blob.hover = null; ptr.last = now();
      kick();
    });
    host.addEventListener('pointerdown', function (e) {
      ptr.cx = e.clientX; ptr.cy = e.clientY; ptr.on = true; ptr.touch = e.pointerType === 'touch';
      ptr.at = ptr.last = now();
      if (!e.target.closest('form, a, button, input, label, summary, [role="table"], [data-hero-now]')) ptr.down = true;
      kick();
    });
    function release() {
      if (ptr.down) { ptr.last = now(); if (ptr.touch) ptr.at = ptr.last; }
      ptr.down = false; kick();
    }
    window.addEventListener('pointerup', release);
    window.addEventListener('pointercancel', release);

    /* typing in the section's email field sends a small ripple from where the caret is */
    var wlForm = cfg.form ? host.querySelector(cfg.form) : null;
    var email = wlForm && wlForm.querySelector('input[type="email"]');
    var meas = document.createElement('canvas').getContext('2d');
    if (email && meas) {
      email.addEventListener('input', function () {
        var r = host.getBoundingClientRect(), b = email.getBoundingClientRect(), cs = getComputedStyle(email);
        meas.font = cs.fontSize + ' ' + cs.fontFamily;
        var x = b.left - r.left + (parseFloat(cs.paddingLeft) || 0) + meas.measureText(email.value).width;
        ripple(Math.min(x, b.right - r.left - 6), b.top - r.top + b.height / 2, 0.45);
      });
    }
    /* the sign-up box keeps the dots parted while anything in its row has focus */
    if (wlForm) {
      wlForm.addEventListener('focusin', function (e) { blob.focus = rowOf(e.target); kick(); });
      wlForm.addEventListener('focusout', function () { blob.focus = null; kick(); });
    }
    /* joined: the parted dots ease back in and, in the section whose form was used, one big ripple runs out from it */
    document.addEventListener('kl:joined', function (e) {
      blob.focus = null; blob.hover = null;
      var from = e.detail && e.detail.form;
      if (wlForm && (!from || from === wlForm)) {
        var hr = host.getBoundingClientRect(), fr = wlForm.getBoundingClientRect();
        if (fr.width) ripple(fr.left - hr.left + fr.width / 2, fr.top - hr.top + Math.min(fr.height / 2, 40), 2.2);
        else ripple(W / 2, H / 2, 2.2);
      }
      requiet();
    });
    document.addEventListener('kl:unjoined', requiet);

    function retheme() { readColours(); kick(); }
    new MutationObserver(retheme).observe(root, { attributes: true, attributeFilter: ['data-theme'] });
    if (darkQ.addEventListener) darkQ.addEventListener('change', retheme);
    if (reducedQ.addEventListener) reducedQ.addEventListener('change', function () { lens.fresh = true; kick(); });
    if ('ResizeObserver' in window) new ResizeObserver(function () { size(); kick(); }).observe(host);
    if ('IntersectionObserver' in window) {
      new IntersectionObserver(function (es) { visible = es[es.length - 1].isIntersecting; if (visible) kick(); }).observe(host);
    }
    document.addEventListener('visibilitychange', function () { if (!document.hidden) kick(); });
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(function () { size(); kick(); });
    readColours();
    size();
    kick();
    return { shock: shock };
  }

  /* ---------- the two grids ---------- */
  var hero = document.querySelector('[data-hero]');
  var heroDots = hero && make(hero, {
    canvas: '[data-hero-fx]', dot: '--hero-dot', hot: '--hero-hot', shade: 0.42, seed: 0,
    quiet: [['.lockup', '.hero-now', 'form.wl']],
    plain: '.lockup .tag, .hero-now-i, form.wl .wl-msg, form.wl .wl-done > *',
    form: 'form.wl', wander: true,
    /* reduced motion: one still swell, to the right of the words on a wide screen and under them on a phone */
    rest: function (W, H, small, box) {
      return small ? { x: 0.62 * W, y: Math.min(H - 12, Math.max(0.86 * H, box ? box.b + 36 : 0)) } : { x: 0.76 * W, y: 0.7 * H };
    }
  });
  if (heroDots) window.klDots = heroDots;

  Array.prototype.forEach.call(document.querySelectorAll('[data-dots]'), function (band, i) {
    make(band, {
      canvas: '[data-dots-fx]', dot: '--dot', hot: '--dot-hot', shade: 0.6, seed: 1.9 + i,
      quiet: [['.pr-head'], ['form.wl']],
      plain: '.pr-head > *, form.wl .wl-msg, form.wl .wl-done > *',
      form: 'form.wl', wander: false, rest: null
    });
  });
})();
