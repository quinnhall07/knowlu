/* Knowlu app, Lock in (T9: note §4 "Lock in", D6; DESIGN.md "Lock in session").

   #lockin/<id>, view "today". The focus page: one assignment on its class's soft field, one clock beside it.
     The queue   Home's order (Do today's rows, then Recommended's), open rows only, from the chosen id (the route's
                 argument); an id not in it, or none, starts at the lead. Taken once, when the page opens: the page
                 never ranks and never reorders under the student.
     Done        the kit's check (KnowluKit.check: set_fields {progress: 100, status: "done"}), quiet (the beat's
                 Undo is the one Undo, so the kit shows no Done toast), from Done or the ring;
                 a refused check stays on the assignment. Then the beat: what is left, the finished one (checked, with
                 Undo, the kit's uncheck), the next one, and Start next in the next class's colour. It waits for Start
                 next. With nothing left, the closing page: what was done, the time focused, Home.
     The clock   25 minutes of focus, then "Break ready" and Break now; a 5-minute break with Skip break; Hide time
                 hides the figure and keeps its place. The clock, the break, the beat and the closing page are the
                 page's own: nothing is stored, nothing is sent but the check.
   End leaves for Home. The sidebar steps aside on this route (lockin.css). After each step the new primary control
   takes the focus: Start next on the beat, Done after Start next or Undo, Home on the closing page, Skip break or
   Hide time after the break's buttons. */
(function () {
  "use strict";

  var FOCUS_MS = 25 * 60000;
  var BREAK_MS = 5 * 60000;
  var TICK_MS = 250;
  var RING_SVG = '<svg viewBox="0 0 22 22" aria-hidden="true"><circle cx="11" cy="11" r="10"/><path d="M6.6 11.4l3 3 5.8-6.2"/></svg>';
  var END_HTML = '<a class="obtn lk-end" href="#home">End</a>';
  var HOME_HTML = '<a class="ibtn lk-home" href="#home">Home</a>';
  var BREAK_HTML = '<button class="obtn" type="button" data-lk-break>Break now</button>';
  var SKIP_HTML = '<button class="obtn" type="button" data-lk-skip>Skip break</button>';
  var LAYOUT_HTML = '<div class="lk" data-mode=""><div class="lk-stage" id="lk-stage"></div><p class="vh" aria-live="polite" id="lk-say"></p></div>';
  var NONE_HTML = '<div class="lk-set lk-none"><p class="lk-l lk-lbl">Nothing left today</p><div class="lk-l lk-acts">' + HOME_HTML + "</div></div>";
  var LABELS = { focus: "Focus", ready: "Break ready", "break": "Break" };

  var ROOT = null, TIMER = null, BUSY = false;
  var QUEUE = [], POS = 0, DONE = [], MODE = "focus", HIDE = false;
  var PHASE = "focus", LEFT_MS = FOCUS_MS, LAST = 0, FOCUSED_MS = 0;

  function $(id) { return document.getElementById(id); }
  /* the page's primary control after a step, so the keyboard never falls to the document */
  function focusOn(sel) {
    var el = ROOT ? ROOT.querySelector(sel) : null;
    if (el) { el.focus(); }
  }

  /* ---- the queue: Home's order, from the chosen id ---- */
  function homeOrder(st) {
    var out = [], seen = {};
    var add = function (r) {
      if (r && r.id && !seen[r.id]) { seen[r.id] = true; out.push(r); }
    };
    ((st && st.must_do && st.must_do.groups) || []).forEach(function (g) { (g.rows || []).forEach(add); });
    ((st && st.recommended && st.recommended.rows) || []).forEach(function (x) { add(x && x.row); });
    return out;
  }
  function queueFrom(st, id) {
    var open = homeOrder(st).filter(function (r) { return r.status !== "done"; });
    var at = -1;
    open.forEach(function (r, i) { if (at < 0 && r.id === id) { at = i; } });
    return at < 0 ? open : open.slice(at);
  }

  /* ---- the clock: the page's own, never stored ---- */
  function clockText(ms) {
    var s = Math.max(0, Math.ceil(ms / 1000));
    var m = Math.floor(s / 60), r = s % 60;
    return m + ":" + (r < 10 ? "0" : "") + r;
  }
  function clockSay(ms) {
    var s = Math.max(0, Math.ceil(ms / 1000));
    var m = Math.floor(s / 60), r = s % 60;
    return LABELS[PHASE] + ", " + m + (m === 1 ? " minute " : " minutes ") + r + (r === 1 ? " second left" : " seconds left");
  }
  function tick() {
    var now = Date.now();
    var dt = Math.max(0, now - LAST);
    LAST = now;
    if (PHASE === "ready") { return; }
    if (PHASE === "focus") { FOCUSED_MS += dt; }
    LEFT_MS -= dt;
    if (LEFT_MS <= 0) {
      if (PHASE === "focus") {
        PHASE = "ready";
        LEFT_MS = BREAK_MS;
        say("Break ready");
      } else {
        PHASE = "focus";
        LEFT_MS = FOCUS_MS;
        say("Focus");
      }
      drawTime();
      return;
    }
    paintClock();
  }
  function startClock() {
    stopClock();
    LAST = Date.now();
    TIMER = setInterval(tick, TICK_MS);
  }
  function stopClock() {
    if (TIMER) { clearInterval(TIMER); }
    TIMER = null;
  }
  function say(text) {
    var el = $("lk-say");
    if (el) { el.textContent = text; }
  }

  /* ---- the parts ---- */
  function slotOf(r) { return KnowluCourses.slot(KnowluCourses.match(r && r.course)); }
  function codeOf(r) {
    var k = KnowluCourses.match(r && r.course);
    return k ? k.title : String((r && r.course) || "");
  }
  function dueHtml(r) {
    return (r && r.due ? '<span class="lk-due"><i>Due</i> ' + h(KnowluFmt.due(r.due)) + "</span>" : "");
  }
  function codeHtml(r) {
    var code = codeOf(r);
    return (code ? '<span class="lk-code">' + h(code) + "</span>" : "");
  }
  function leftText() {
    var rest = QUEUE.slice(POS);
    var mins = 0;
    rest.forEach(function (r) { mins += (Number(r.hours) || 0) * 60; });
    return rest.length + " left" + (mins > 0 ? " · " + KnowluFmt.minutes(mins) : "");
  }
  function headHtml() {
    return '<div class="lk-head">' + END_HTML + "</div>";
  }
  function workHtml(r) {
    var slot = slotOf(r);
    var slotHtml = (slot ? " " + h(slot) : "");
    return '<div class="lk-set lk-work' + slotHtml + '">' +
      '<div class="lk-field' + slotHtml + '" aria-hidden="true"></div>' +
      '<p class="lk-l lk-lbl"><button class="ck lk-ring' + slotHtml + '" type="button" data-lk-done aria-pressed="false" aria-label="' + h("Done: " + (r.title || "")) + '">' + RING_SVG + "</button>" +
      codeHtml(r) + dueHtml(r) + "</p>" +
      '<h1 class="lk-l lk-main lk-name">' + h(r.title) + "</h1>" +
      '<div class="lk-l lk-acts"><button class="lk-do' + slotHtml + '" type="button" data-lk-done>Done</button></div></div>';
  }
  function seamHtml(was, nxt) {
    var wasSlot = slotOf(was), nxSlot = slotOf(nxt);
    return '<div class="lk-set lk-seam">' +
      '<p class="lk-l lk-lbl"><span class="lk-left">' + h(leftText()) + "</span></p>" +
      '<div class="lk-l lk-main lk-seam-main">' +
      '<p class="lk-line"><span class="ck lk-on' + (wasSlot ? " " + h(wasSlot) : "") + '" aria-hidden="true">' + RING_SVG + "</span>" +
      '<span class="vh">Done:</span><span class="lk-was">' + h(was.title) + "</span>" +
      '<button class="tbtn lk-undo" type="button" data-lk-undo>Undo</button></p>' +
      '<p class="lk-line"><span class="ck lk-ring' + (nxSlot ? " " + h(nxSlot) : "") + '" aria-hidden="true">' + RING_SVG + "</span>" +
      '<span class="vh">Next:</span>' + codeHtml(nxt) + dueHtml(nxt) + "</p>" +
      '<h1 class="lk-name">' + h(nxt.title) + "</h1></div></div>";
  }
  function doneRowHtml(r) {
    var slot = slotOf(r);
    return '<li class="lk-done-row"><span class="ck lk-on' + (slot ? " " + h(slot) : "") + '" aria-hidden="true">' + RING_SVG + "</span>" +
      '<span class="lk-done-nm">' + h(r.title) + '</span><span class="lk-done-code">' + h(codeOf(r)) + "</span></li>";
  }
  function closeHtml() {
    var rows = DONE.map(function (d) { return d.row; });
    return '<div class="lk-set lk-close">' +
      '<p class="lk-l lk-lbl"><span class="lk-done-n">' + h(rows.length + " done") + "</span></p>" +
      '<ul class="lk-l lk-main lk-done-list" aria-label="Done">' + rows.map(doneRowHtml).join("") + "</ul>" +
      '<p class="lk-r lk-lbl lk-clabel">Focused</p>' +
      '<p class="lk-r lk-main lk-spent">' + h(KnowluFmt.minutes(FOCUSED_MS / 60000)) + "</p>" +
      '<div class="lk-l lk-acts">' + HOME_HTML + "</div></div>";
  }
  function hideHtml() {
    return '<button class="obtn" type="button" data-lk-hide aria-pressed="' + (HIDE ? "true" : "false") + '">' + h(HIDE ? "Show time" : "Hide time") + "</button>";
  }
  function timeActsHtml() {
    if (MODE === "beat") {
      var nx = slotOf(QUEUE[POS]);
      return '<div class="lk-r lk-acts"><button class="lk-do lk-next' + (nx ? " " + h(nx) : "") + '" type="button" data-lk-next>Start next</button></div>';
    }
    return '<div class="lk-r lk-acts">' + hideHtml() + (PHASE === "ready" ? BREAK_HTML : "") + (PHASE === "break" ? SKIP_HTML : "") + "</div>";
  }
  function timeHtml() {
    return '<div class="lk-set lk-time">' +
      '<p class="lk-r lk-lbl lk-clabel">' + h(LABELS[PHASE]) + "</p>" +
      '<p class="lk-r lk-main lk-clock" role="timer" aria-label="' + h(clockSay(LEFT_MS)) + '">' + h(clockText(LEFT_MS)) + "</p>" +
      '<p class="lk-r lk-place">' + h(Math.min(POS + 1, QUEUE.length) + " of " + QUEUE.length + " today") + "</p>" +
      timeActsHtml() + "</div>";
  }

  /* ---- drawing ---- */
  function paintClock() {
    var c = ROOT ? ROOT.querySelector(".lk-clock") : null;
    if (!c) { return; }
    c.textContent = clockText(LEFT_MS);
    c.setAttribute("aria-label", clockSay(LEFT_MS));
  }
  function drawTime() {
    var t = ROOT ? ROOT.querySelector(".lk-time") : null;
    if (!t) { return; }
    var had = t.contains(document.activeElement);
    t.outerHTML = timeHtml();
    ROOT.setAttribute("data-phase", PHASE);
    if (had) { focusOn(PHASE === "ready" ? "[data-lk-break]" : "[data-lk-hide]"); }
  }
  function draw() {
    if (!ROOT) { return; }
    var stage = $("lk-stage");
    var cur = QUEUE[POS];
    ROOT.setAttribute("data-phase", PHASE);
    ROOT.toggleAttribute("data-hide", HIDE);
    if (!QUEUE.length) {
      ROOT.setAttribute("data-mode", "none");
      ROOT.removeAttribute("data-cur");
      stage.innerHTML = NONE_HTML;
      return;
    }
    if (MODE === "close") {
      ROOT.setAttribute("data-mode", "close");
      ROOT.removeAttribute("data-cur");
      stage.innerHTML = closeHtml();
      return;
    }
    ROOT.setAttribute("data-mode", MODE);
    ROOT.setAttribute("data-cur", cur.id);
    var leftHtml = MODE === "beat" ? seamHtml(DONE[DONE.length - 1].row, cur) : workHtml(cur);
    stage.innerHTML = headHtml() + leftHtml + timeHtml();
  }

  /* ---- what the student does ---- */
  function finish() {
    var cur = QUEUE[POS];
    if (!cur || BUSY || MODE !== "focus") { return; }
    BUSY = true;
    var ring = ROOT.querySelector(".lk-ring[data-lk-done]");
    if (ring) { ring.setAttribute("aria-pressed", "true"); }
    var old = typeof cur.progress_pct === "number" ? cur.progress_pct : 0;
    KnowluKit.check(cur, { quiet: true }).then(function (env) {
      BUSY = false;
      if (!ROOT || QUEUE[POS] !== cur) { return; }
      if (!env || !env.ok) {
        if (ring) { ring.setAttribute("aria-pressed", "false"); }
        return;
      }
      DONE.push({ row: cur, old: old });
      POS += 1;
      MODE = POS < QUEUE.length ? "beat" : "close";
      say(MODE === "beat" ? "Done. Next: " + QUEUE[POS].title : "All done");
      draw();
      focusOn(MODE === "beat" ? "[data-lk-next]" : ".lk-home");
    });
  }
  function undo() {
    var last = DONE[DONE.length - 1];
    if (!last || BUSY || MODE !== "beat") { return; }
    BUSY = true;
    KnowluKit.uncheck(last.row, last.old).then(function (env) {
      BUSY = false;
      if (!ROOT || !env || !env.ok) { return; }
      DONE.pop();
      POS -= 1;
      MODE = "focus";
      say(last.row.title);
      draw();
      focusOn(".lk-acts [data-lk-done]");
    });
  }
  function next() {
    if (MODE !== "beat") { return; }
    MODE = "focus";
    say(QUEUE[POS].title);
    draw();
    focusOn(".lk-acts [data-lk-done]");
  }
  function takeBreak() {
    PHASE = "break";
    LEFT_MS = BREAK_MS;
    LAST = Date.now();
    drawTime();
    focusOn("[data-lk-skip]");
  }
  function skipBreak() {
    PHASE = "focus";
    LEFT_MS = FOCUS_MS;
    LAST = Date.now();
    drawTime();
    focusOn("[data-lk-hide]");
  }
  function toggleHide() {
    HIDE = !HIDE;
    ROOT.toggleAttribute("data-hide", HIDE);
    var b = ROOT.querySelector("[data-lk-hide]");
    if (b) {
      b.setAttribute("aria-pressed", HIDE ? "true" : "false");
      b.textContent = HIDE ? "Show time" : "Hide time";
    }
  }
  function onClick(e) {
    var t = e.target;
    if (t.closest("[data-lk-done]")) { finish(); return; }
    if (t.closest("[data-lk-undo]")) { undo(); return; }
    if (t.closest("[data-lk-next]")) { next(); return; }
    if (t.closest("[data-lk-hide]")) { toggleHide(); return; }
    if (t.closest("[data-lk-break]")) { takeBreak(); return; }
    if (t.closest("[data-lk-skip]")) { skipBreak(); }
  }

  KnowluRouter.add("lockin", {
    mount: function (center, ctx) {
      center.innerHTML = LAYOUT_HTML;
      ROOT = center.querySelector(".lk");
      QUEUE = queueFrom(ctx.state, ctx.arg || "");
      POS = 0;
      DONE = [];
      MODE = "focus";
      HIDE = false;
      BUSY = false;
      PHASE = "focus";
      LEFT_MS = FOCUS_MS;
      FOCUSED_MS = 0;
      ROOT.addEventListener("click", onClick);
      draw();
      startClock();
    },
    update: function (st) {
      /* the queue was taken when the page opened; a new state only fills it in if it opened before the first poll */
      if (ROOT && !QUEUE.length && !DONE.length) {
        QUEUE = queueFrom(st, KnowluRouter.arg ? KnowluRouter.arg() : "");
        draw();
      }
    },
    unmount: function () {
      clearInterval(TIMER);
      TIMER = null;
      ROOT = null;
      QUEUE = [];
      DONE = [];
    }
  });
})();
