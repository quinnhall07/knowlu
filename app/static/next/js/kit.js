/* Knowlu app: the small parts every page shares (plan §3). Names defined here, and nowhere else:

   h(s)                          the one escaper (D5): & < > " ' become entities. Every innerHTML takes a module constant
                                 or a string built through h(); a page never sets innerHTML from anything else.
   ev(kind, id, objectKind, ms)  the one ui_event sender (D1, I9). view is the router's fixed route name, never a slug;
                                 id is a note id, objectKind a template's word; never a title, a body or mail text.
                                 A failure is silent and never blocks the write it rode in on.
   KnowluKit.owed()              the attributes a start control takes (every [data-start] control: Connect, Sync now,
                                 Resume, set-up) while the privacy update is owed: disabled, with the one line as its
                                 tooltip; "" otherwise (the shipped owedAttrs(), I5). KnowluKit.setOwed(on) is the
                                 re-consent module's switch; KnowluKit.isOwed() reads it; KnowluKit.OWED_LINE is the line.
   KnowluKit.owedHtml(controlHtml)  a disabled control takes neither the pointer nor focus, so its tip would never show:
                                 while the update is owed this wraps the control's markup in a focusable span that carries
                                 the line (the control passes the pointer to it, shell.css .owed); otherwise the markup as is.
                                 Use it around every [data-start] control: KnowluKit.owedHtml('<button data-start' +
                                 KnowluKit.owed() + '>...</button>').
   KnowluKit.checkHtml(row)      the one check control for a task row: a ring in the row's class colour; done, filled
                                 with it (A8; ink with no course entry). A press anywhere on the page writes through
                                 check / uncheck below, so a page only draws it.
   KnowluKit.check(row)          set_fields {progress: 100, status: "done"}, then a toast with Undo
   KnowluKit.uncheck(row, old)   set_fields {progress: old, status: "active"} (the Undo)
   KnowluKit.markHtml(course)    the class mark (.mk in the course's slot; ink with no entry)
   KnowluKit.toast(text, action, fn, isError)   a quiet line at the foot of the window, with one optional action
   KnowluKit.refusal(env)        the sentence for a refused envelope
   KnowluKit.watchSeen()         after a paint: object_seen once per [data-id] that stays 60% visible for 2 s
   KnowluKit.startLook()         the end-of-look stamp: mark_seen on blur and when the page is hidden (R37), never at boot
   Tooltips: any element with data-tip shows it on hover and focus after about 450 ms (60 ms once one has shown), and
   hides on a press, a scroll, Escape and blur (never title=, which draws the browser's own box; SHELL.md "Tooltips"). */
(function () {
  "use strict";
  var ESC = { "&": "&amp;", "<": "&lt;", ">": "&gt;", "\x22": "&quot;", "\x27": "&#39;" };
  var OWED_LINE = "Accept the privacy update to use this";
  var CHECK_SVG = '<svg viewBox="0 0 22 22" aria-hidden="true"><circle cx="11" cy="11" r="10"/><path d="M6.6 11.4l3 3 5.8-6.2"/></svg>';
  var UNREACHABLE = "Knowlu could not reach its engine. Try again in a moment.";
  var OWED = false;

  function h(s) {
    return String(s == null ? "" : s).replace(/[&<>\x22\x27]/g, function (c) { return ESC[c]; });
  }

  function ev(kind, id, objectKind, ms) {
    KnowluData.call("ui_event", { action: kind, view: KnowluRouter.view(), objectId: id || null, objectKind: objectKind || null, ms: typeof ms === "number" ? ms : null });
  }

  function owed() { return OWED ? " disabled data-tip=\x22" + h(OWED_LINE) + "\x22" : ""; }
  function owedHtml(controlHtml) {
    if (!OWED) { return controlHtml; }
    return '<span class="owed" tabindex="0" data-tip="' + h(OWED_LINE) + '">' + controlHtml + "</span>";
  }

  function refusal(env) {
    if (env && typeof env.error === "string" && env.error) { return env.error; }
    return UNREACHABLE;
  }

  /* ---- toasts ---- */
  function toast(text, action, fn, isError) {
    var host = document.getElementById("toasts");
    if (!host) { return; }
    var t = document.createElement("div");
    t.className = "toast" + (isError ? " is-error" : "");
    t.setAttribute("role", isError ? "alert" : "status");
    var line = document.createElement("span");
    line.textContent = text;
    t.appendChild(line);
    var gone = function () { if (t.parentNode) { t.parentNode.removeChild(t); } };
    if (action && fn) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "tbtn";
      b.textContent = action;
      b.addEventListener("click", function () { gone(); fn(); });
      t.appendChild(b);
    }
    while (host.children.length >= 3) { host.removeChild(host.firstChild); }
    host.appendChild(t);
    setTimeout(gone, isError ? 8000 : 6000);
  }

  /* ---- the check control ---- */
  function markHtml(course) {
    var slot = KnowluCourses.slot(KnowluCourses.match(course));
    return '<i class="mk' + (slot ? " " + h(slot) : "") + '" aria-hidden="true"></i>';
  }
  function checkHtml(row) {
    var slot = KnowluCourses.slot(KnowluCourses.match(row.course));
    var on = row.status === "done";
    return '<button class="ck' + (slot ? " " + h(slot) : "") + '" type="button" data-ck="' + h(row.id) + '" data-progress="' + h(row.progress_pct == null ? 0 : row.progress_pct) +
      '" aria-pressed="' + (on ? "true" : "false") + '" aria-label="' + h((on ? "Not done: " : "Done: ") + (row.title || "")) + '">' + CHECK_SVG + "</button>";
  }
  document.addEventListener("click", function (e) {
    var b = e.target.closest ? e.target.closest("button.ck[data-ck]") : null;
    if (!b || b.disabled) { return; }
    e.stopPropagation();
    var row = { id: b.getAttribute("data-ck"), progress_pct: Number(b.getAttribute("data-progress")) || 0 };
    var was = b.getAttribute("aria-pressed") === "true";
    b.setAttribute("aria-pressed", was ? "false" : "true");
    b.disabled = true;
    var done = function (env) {
      b.disabled = false;
      if (!env || !env.ok) { b.setAttribute("aria-pressed", was ? "true" : "false"); }
    };
    if (was) { KIT.uncheck(row, 0).then(done); } else { KIT.check(row).then(done); }
  }, true);

  /* ---- seen: a 2 s dwell, once per id per launch ---- */
  var seenOnce = Object.create(null);
  var watched = new Set();
  var observer = window.IntersectionObserver ? new IntersectionObserver(function (entries) {
    entries.forEach(function (en) {
      var el = en.target, id = el.getAttribute("data-id");
      if (!id) { return; }
      if (!en.isIntersecting) { el.removeAttribute("data-seen-at"); return; }
      var at = Date.now();
      el.setAttribute("data-seen-at", String(at));
      if (seenOnce[id]) { return; }
      setTimeout(function () {
        if (seenOnce[id] || el.getAttribute("data-seen-at") !== String(at) || !el.isConnected) { return; }
        seenOnce[id] = true;
        ev("object_seen", id, el.getAttribute("data-kind"), 2000);
      }, 2000);
    });
  }, { threshold: 0.6 }) : null;
  function watchSeen() {
    if (!observer) { return; }
    watched.forEach(function (el) { if (!el.isConnected) { observer.unobserve(el); watched.delete(el); } });
    document.querySelectorAll("[data-id]").forEach(function (el) {
      if (!watched.has(el)) { observer.observe(el); watched.add(el); }
    });
  }

  /* ---- the end of a look: blur, or hidden. Invoked from this one place, never at boot (R37). ---- */
  function endOfLook() { KnowluData.call("mark_seen", {}); }
  var looking = false;
  function startLook() {
    if (looking) { return; }
    looking = true;
    window.addEventListener("blur", endOfLook);
    document.addEventListener("visibilitychange", function () { if (document.hidden) { endOfLook(); } });
  }

  /* ---- tooltips (data-tip) ---- */
  var TIP_DELAY_MS = 450;
  var TIP_AGAIN_MS = 60;
  var tip = null;
  var tipTimer = null;
  var tipFor = null;
  var tipShownAt = -Infinity;
  function showTip(el) {
    var words = el.getAttribute("data-tip");
    if (!words) { return; }
    if (!tip) { tip = document.createElement("div"); tip.className = "kn-tip"; tip.setAttribute("role", "tooltip"); document.body.appendChild(tip); }
    tip.textContent = words;
    var zoom = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--app-zoom")) || 1;
    var r = el.getBoundingClientRect();
    tip.setAttribute("data-open", "");
    var w = tip.offsetWidth * zoom, ht = tip.offsetHeight * zoom;
    var x = Math.max(8, Math.min(window.innerWidth - w - 8, r.left + r.width / 2 - w / 2));
    var y = r.top - ht - 8 < 8 ? r.bottom + 8 : r.top - ht - 8;
    tip.style.left = (x / zoom) + "px";
    tip.style.top = (y / zoom) + "px";
  }
  function hideTip() {
    clearTimeout(tipTimer);
    tipTimer = null;
    tipFor = null;
    if (tip && tip.hasAttribute("data-open")) { tip.removeAttribute("data-open"); tipShownAt = performance.now(); }
  }
  /* wait, then show: 450 ms, or 60 ms when a tip was showing a moment ago */
  function soonTip(el) {
    if (el === tipFor) { return; }
    hideTip();
    tipFor = el;
    var wait = performance.now() - tipShownAt < 600 ? TIP_AGAIN_MS : TIP_DELAY_MS;
    tipTimer = setTimeout(function () { if (tipFor === el && el.isConnected) { showTip(el); tipShownAt = performance.now(); } }, wait);
  }
  function tipTarget(e) { return e.target.closest ? e.target.closest("[data-tip]") : null; }
  document.addEventListener("pointerover", function (e) { var el = tipTarget(e); if (el) { soonTip(el); } else { hideTip(); } });
  document.addEventListener("focusin", function (e) { var el = tipTarget(e); if (el) { soonTip(el); } else { hideTip(); } });
  document.addEventListener("focusout", hideTip);
  document.addEventListener("pointerdown", hideTip, true);
  document.addEventListener("scroll", hideTip, true);
  document.addEventListener("keydown", function (e) { if (e.key === "Escape") { hideTip(); } });
  window.addEventListener("blur", hideTip);

  /* the API: check and uncheck are written out here, the one place a task is marked done */
  var KIT = {
    OWED_LINE: OWED_LINE,
    owed: owed,
    owedHtml: owedHtml,
    setOwed: function (on) { OWED = !!on; },
    isOwed: function () { return OWED; },
    check: function (row) {
      var old = typeof row.progress_pct === "number" && row.progress_pct < 100 ? row.progress_pct : 0;
      return KnowluData.call("set_fields", { id: row.id, fields: { progress: 100, status: "done" } }).then(function (env) {
        if (env && env.ok) {
          ev("edit_committed", row.id, "progress");
          toast("Done", "Undo", function () { KIT.uncheck(row, old); });
        } else {
          toast(refusal(env), null, null, true);
        }
        return env;
      });
    },
    uncheck: function (row, old) {
      var back = typeof old === "number" && old < 100 ? old : 0;
      return KnowluData.call("set_fields", { id: row.id, fields: { progress: back, status: "active" } }).then(function (env) {
        if (env && env.ok) { ev("edit_committed", row.id, "progress"); } else { toast(refusal(env), null, null, true); }
        return env;
      });
    },
    checkHtml: checkHtml,
    markHtml: markHtml,
    toast: toast,
    refusal: refusal,
    watchSeen: watchSeen,
    startLook: startLook
  };
  window.h = h;
  window.ev = ev;
  window.KnowluKit = KIT;
})();
