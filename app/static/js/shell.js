/* Knowlu app: the shell's chrome (note §2, §3; plan §3). The sidebar (rail, peek, pin; the active classes), the top band
   (the clock, the window's three caption buttons), the Status button and list, and the three hosts a page or a carried
   surface renders into: the right panel, the one sheet and the re-consent layer.

   KnowluPanel.add(kind, mod)      a panel kind registers itself: mod.render(body, id, api) draws the panel's body and may
                                   call api.title(text), api.check(row) (the task's check beside the title) and api.close();
                                   mod.close() (optional) runs when it closes
   KnowluPanel.open(kind, id)      opens the right panel (#panel) with that kind; .close(); .isOpen()
   KnowluSheets.request(name, mod) asks for the one sheet (#sheet). Sheets come one at a time, in N5's order: sign-in and
                                   upgrade, then week setup, then mail progress, then the report; a request waits behind a
                                   sheet already open or of a higher rank. upgrade also waits while Settings is the route.
                                   mod.render(host, api) draws it; api.close() closes it; mod.close() (optional) runs then.
                                   A module defined with KnowluSheets.define(name, mod) is used when request names no mod.
                                   A name outside N5's list is refused: nothing else renders into #sheet.
   KnowluSheets.close(name)        closes that sheet (or forgets its waiting request); .isOpen()
   KnowluLayer.show(mod)           #layer: re-consent only; mod.render(host). Never modal, no Escape; it stands down while
                                   a sheet is open or Settings is the route, and comes back after. .hide() is the module's.
   KnowluSlots.deck(el)            Home's Waiting calls it; the deck module registers its renderer with addDeck(fn), and
                                   with none registered deck(el) draws nothing
   KnowluShell.routed(name)        the router's: marks the current page in the sidebar, closes the panel and Status
   KnowluShell.status()            redraws the Status dot and list; it also runs on every poll (KnowluData.onPoll), so
                                   grades_status and mail_status landing after the paint, or a poll that failed, show at once

   The Status dot is amber when the state carries journal warnings or unreadable notes, the error red when a source
   needs the student or the engine did not answer (P11, I1). Google's line shows what the last Settings visit read, or nothing: no launch, boot or
   poll path asks Google (P4). */
(function () {
  "use strict";
  var SHEET_RANK = { signin: 0, upgrade: 0, weeksetup: 1, mail: 2, report: 3 };
  var PANELS = Object.create(null);
  var SHEETS = Object.create(null);
  var EMPTY_SBAR = '<li class="sbar-empty">Everything is up to date.</li>';
  var PEEK_OPEN_MS = 350;   /* the pointer rests on the rail this long before the peek opens (SHELL.md) */
  var PEEK_CLOSE_MS = 400;  /* and the peek folds this long after it leaves, unless it comes back */
  var PEEK_TIMER = null;
  var $ = function (id) { return document.getElementById(id); };
  var root = document.documentElement;

  var ROUTE = "home";
  var OPEN_PANEL = null;
  var OPEN_SHEET = null;
  var WAITING = [];
  var LAYER_MOD = null;
  var DECK = null;
  var COURSE_KEY = null;

  function onSettings() { return ROUTE === "settings"; }

  /* ---- the sidebar ---- */
  function markCurrent() {
    document.querySelectorAll(".side a[data-route]").forEach(function (a) {
      if (a.getAttribute("data-route") === ROUTE) { a.setAttribute("aria-current", "page"); } else { a.removeAttribute("aria-current"); }
    });
    var slug = ROUTE === "class" ? KnowluRouter.arg() : null;
    document.querySelectorAll("#side-classes a[data-class]").forEach(function (a) {
      if (a.getAttribute("data-class") === slug) { a.setAttribute("aria-current", "page"); } else { a.removeAttribute("aria-current"); }
    });
  }
  function drawClasses(st) {
    var list = st && Array.isArray(st.courses) ? st.courses.filter(function (c) { return KnowluCourses.active(c); }) : [];
    var key = JSON.stringify(list.map(function (k) { return [k.slug, k.slot, k.title]; }));
    if (key === COURSE_KEY) { return; }
    COURSE_KEY = key;
    function itemHtml(c) {
      return '<li><a href="#class/' + h(encodeURIComponent(c.slug)) + '" class="' + h(c.slot) + '" data-class="' + h(c.slug) + '"><i class="mk" aria-hidden="true"></i><span>' + h(c.title) + "</span></a></li>";
    }
    $("side-classes").innerHTML = list.map(itemHtml).join("");
    markCurrent();
  }
  /* ---- the peek: the rail widened over the page. Keyboard focus and an open Status list hold it open; a mouse
     click that left focus inside does not. ---- */
  function fold() {
    clearTimeout(PEEK_TIMER);
    PEEK_TIMER = null;
    root.classList.remove("peek");
  }
  function holdsPeek() {
    return !!$("side").querySelector(":focus-visible") || $("sbar").getAttribute("data-open") !== null;
  }
  function bindSidebar() {
    var side = $("side");
    $("rail-toggle").setAttribute("aria-pressed", KnowluPrefs.pinned() ? "true" : "false");
    $("rail-toggle").addEventListener("click", function () {
      var on = !KnowluPrefs.pinned();
      KnowluPrefs.setPinned(on);
      $("rail-toggle").setAttribute("aria-pressed", on ? "true" : "false");
      $("rail-toggle").setAttribute("aria-label", on ? "Unpin sidebar" : "Pin sidebar");
    });
    side.addEventListener("pointerenter", function () {
      clearTimeout(PEEK_TIMER);
      PEEK_TIMER = null;
      if (!root.classList.contains("rail") || root.classList.contains("peek")) { return; }
      PEEK_TIMER = setTimeout(function () { root.classList.add("peek"); }, PEEK_OPEN_MS);
    });
    side.addEventListener("pointerleave", function () {
      clearTimeout(PEEK_TIMER);
      PEEK_TIMER = null;
      if (!root.classList.contains("peek") || holdsPeek()) { return; }
      PEEK_TIMER = setTimeout(fold, PEEK_CLOSE_MS);
    });
    side.addEventListener("focusin", function (e) {
      if (root.classList.contains("rail") && e.target.matches(":focus-visible")) { root.classList.add("peek"); }
    });
    side.addEventListener("focusout", function (e) {
      if (!side.contains(e.relatedTarget) && !side.matches(":hover") && !holdsPeek()) { fold(); }
    });
    $("classes-toggle").addEventListener("click", function () {
      var open = $("classes-toggle").getAttribute("aria-expanded") !== "true";
      $("classes-toggle").setAttribute("aria-expanded", open ? "true" : "false");
      $("side-classes").hidden = !open;
    });
    $("skip").addEventListener("click", function () { $("center").focus(); });
  }

  /* ---- Status: the dot and the list ---- */
  function engineDown() {
    var last = KnowluData.last("state");
    return !!(last && last.ok === false);
  }
  function statusRows(st) {
    var rows = [];
    if (engineDown()) { rows.push({ level: "error", name: "Knowlu", note: "The engine did not answer" }); }
    var srcs = st && st.issues_panel && Array.isArray(st.issues_panel.sources) ? st.issues_panel.sources : [];
    srcs.forEach(function (src) { rows.push({ level: "error", name: src.title, note: "Needs you" }); });
    var g = KnowluData.last("grades_status");
    if (g && g.ok && g.available && g.connected) {
      rows.push(g.signed_out ? { level: "error", name: "Grades", note: "Signed out" } : { level: "", name: "Grades", note: g.fetched_at ? KnowluFmt.read(g.fetched_at) : "Not read yet" });
    }
    var gs = KnowluData.last("google_status");
    if (gs) { rows.push({ level: gs.ok ? "" : "warn", name: "Google", note: gs.ok ? (gs.state === "connected" ? "Connected" : "Not connected") : "Could not check" }); }
    var warns = st && Array.isArray(st.warnings) ? st.warnings.length : 0;
    if (warns) { rows.push({ level: "warn", name: "Journal", note: warns === 1 ? "1 line skipped" : warns + " lines skipped" }); }
    var bad = st && Array.isArray(st.unreadable) ? st.unreadable.length : 0;
    if (bad) { rows.push({ level: "warn", name: "Notes", note: bad === 1 ? "1 note unreadable" : bad + " notes unreadable" }); }
    return rows;
  }
  function statusLevel(st) {
    var srcs = st && st.issues_panel && Array.isArray(st.issues_panel.sources) ? st.issues_panel.sources : [];
    var g = KnowluData.last("grades_status");
    if (engineDown() || srcs.length || (g && g.ok && g.connected && g.signed_out)) { return "error"; }
    if (st && ((Array.isArray(st.warnings) && st.warnings.length) || (Array.isArray(st.unreadable) && st.unreadable.length))) { return "warn"; }
    return "";
  }
  function rowHtml(r) {
    return '<li class="' + (r.level === "error" ? "bad" : "") + '"><b>' + (r.level ? '<i class="dot" data-level="' + h(r.level) + '" aria-hidden="true"></i>' : "") + h(r.name) + '</b><span class="ht">' + h(r.note) + "</span></li>";
  }
  function drawStatus() {
    var st = KnowluData.state();
    var level = statusLevel(st);
    var dot = $("status-dot");
    dot.hidden = !level;
    if (level) { dot.setAttribute("data-level", level); } else { dot.removeAttribute("data-level"); }
    $("status-toggle").setAttribute("aria-label", level === "error" ? "Status, a source needs a look" : level === "warn" ? "Status, something to check" : "Status");
    var rows = statusRows(st);
    $("sbar-list").innerHTML = rows.length ? rows.map(rowHtml).join("") : EMPTY_SBAR;
  }
  function setStatusOpen(open) {
    var bar = $("sbar");
    if (open) { drawStatus(); bar.setAttribute("data-open", ""); } else { bar.removeAttribute("data-open"); }
    $("status-toggle").setAttribute("aria-expanded", open ? "true" : "false");
    if (!open && root.classList.contains("peek") && !$("side").matches(":hover") && !holdsPeek()) { fold(); }
  }
  function bindStatus() {
    $("status-toggle").addEventListener("click", function () { setStatusOpen($("sbar").getAttribute("data-open") === null); });
    document.addEventListener("pointerdown", function (e) {
      if ($("sbar").getAttribute("data-open") === null) { return; }
      if (!e.target.closest("#sbar") && !e.target.closest("#status-toggle")) { setStatusOpen(false); }
    });
  }

  /* ---- the right panel ---- */
  function closePanel() {
    if (!OPEN_PANEL) { return; }
    var mod = OPEN_PANEL.mod;
    OPEN_PANEL = null;
    var p = $("panel");
    p.removeAttribute("data-open");
    p.setAttribute("inert", "");
    $("panel-body").textContent = "";
    $("panel-ck").textContent = "";
    $("panel-h").textContent = "";
    if (typeof mod.close === "function") { try { mod.close(); } catch (e) { window.console.error(e); } }
  }
  function openPanel(kind, id) {
    var mod = Object.prototype.hasOwnProperty.call(PANELS, kind) ? PANELS[kind] : null;
    if (!mod) { return false; }
    closePanel();
    OPEN_PANEL = { kind: kind, id: id, mod: mod };
    var p = $("panel");
    p.removeAttribute("inert");
    p.setAttribute("data-open", "");
    var api = {
      title: function (text) { $("panel-h").textContent = text == null ? "" : String(text); },
      check: function (row) {
        $("panel-ck").innerHTML = row && row.id ? KnowluKit.checkHtml(row) : "";
      },
      close: closePanel
    };
    try { mod.render($("panel-body"), id, api); } catch (e) { window.console.error(e); }
    KnowluKit.watchSeen();
    return true;
  }
  function bindPanel() {
    $("panel-close").addEventListener("click", closePanel);
    document.addEventListener("pointerdown", function (e) {
      if (!OPEN_PANEL || OPEN_SHEET) { return; }
      if (e.target.closest("#panel") || e.target.closest("[data-det]") || e.target.closest(".toasts")) { return; }
      closePanel();
    });
  }

  /* ---- the one sheet ---- */
  function rankOf(name) { return Object.prototype.hasOwnProperty.call(SHEET_RANK, name) ? SHEET_RANK[name] : -1; }
  function nextSheet() {
    var best = -1;
    for (var i = 0; i < WAITING.length; i++) {
      if (WAITING[i].name === "upgrade" && onSettings()) { continue; }
      if (best < 0 || rankOf(WAITING[i].name) < rankOf(WAITING[best].name)) { best = i; }
    }
    return best < 0 ? null : WAITING.splice(best, 1)[0];
  }
  /* #sheet is the one dialog, so it takes the name the sheet's own surface gives (.sheet-in's aria-labelledby): a role-less
     div's label names nothing. The last sheet's name goes first. */
  function showSheet(req) {
    OPEN_SHEET = req;
    var host = $("sheet");
    host.textContent = "";
    host.removeAttribute("aria-labelledby");
    host.hidden = false;
    $("center").setAttribute("inert", "");
    $("side").setAttribute("inert", "");
    $("panel").setAttribute("inert", "");
    syncLayer();
    try { req.mod.render(host, { close: function () { closeSheet(req.name); } }); } catch (e) { window.console.error(e); }
    var named = host.querySelector(".sheet-in[aria-labelledby]");
    if (named) { host.setAttribute("aria-labelledby", named.getAttribute("aria-labelledby")); }
  }
  function pump() {
    if (OPEN_SHEET) { return; }
    var req = nextSheet();
    if (req) { showSheet(req); }
  }
  function closeSheet(name) {
    WAITING = WAITING.filter(function (r) { return r.name !== name; });
    if (!OPEN_SHEET || OPEN_SHEET.name !== name) { return; }
    var mod = OPEN_SHEET.mod;
    OPEN_SHEET = null;
    var host = $("sheet");
    host.hidden = true;
    host.textContent = "";
    $("center").removeAttribute("inert");
    $("side").removeAttribute("inert");
    if (OPEN_PANEL) { $("panel").removeAttribute("inert"); }
    if (typeof mod.close === "function") { try { mod.close(); } catch (e) { window.console.error(e); } }
    pump();
    syncLayer();
  }

  /* ---- the re-consent layer ---- */
  function layerStandsDown() { return !!OPEN_SHEET || onSettings(); }
  function syncLayer() {
    var host = $("layer");
    if (!LAYER_MOD || layerStandsDown()) { host.hidden = true; return; }
    if (host.hidden) {
      host.textContent = "";
      host.hidden = false;
      try { LAYER_MOD.render(host); } catch (e) { window.console.error(e); }
    }
  }

  /* ---- Escape: the sheet first, then the panel, then Status. Never the layer: re-consent has no dismiss. ---- */
  function onEscape(e) {
    if (e.key !== "Escape") { return; }
    if (OPEN_SHEET) {
      if (OPEN_SHEET.mod.escape !== false) { closeSheet(OPEN_SHEET.name); }
      return;
    }
    if (OPEN_PANEL) { closePanel(); return; }
    if ($("sbar").getAttribute("data-open") !== null) { setStatusOpen(false); $("status-toggle").focus(); return; }
    if (root.classList.contains("peek")) { fold(); }
  }

  /* ---- the window's caption buttons (note §3) ---- */
  function host() {
    try { return window.__TAURI__ && window.__TAURI__.window ? window.__TAURI__.window.getCurrentWindow() : null; } catch (e) { return null; }
  }
  function syncMax(w) {
    w.isMaximized().then(function (m) {
      root.classList.toggle("maximized", !!m);
      var b = document.querySelector('.wc-b[data-wc="toggle"]');
      if (b) { b.setAttribute("aria-label", m ? "Restore" : "Maximize"); }
    }, function () {});
  }
  function bindWindow() {
    var w = host();
    if (!w) { return; }
    root.classList.add("frameless");
    document.querySelector(".wc").addEventListener("click", function (e) {
      var b = e.target.closest("[data-wc]");
      if (!b) { return; }
      var act = b.getAttribute("data-wc");
      if (act === "minimize") { w.minimize().catch(function () {}); }
      if (act === "toggle") { w.toggleMaximize().then(function () { syncMax(w); }, function () {}); }
      if (act === "close") { w.close().catch(function () {}); }
    });
    syncMax(w);
    if (typeof w.onResized === "function") { w.onResized(function () { syncMax(w); }).catch(function () {}); }
  }

  /* ---- the clock: the read model's now (D5), redrawn with each state ---- */
  function drawClock() {
    var st = KnowluData.state();
    var p = st && st.now ? KnowluFmt.parse(st.now) : null;
    var c = $("clock");
    c.textContent = "";
    if (!p) { return; }
    var a = document.createElement("span");
    a.textContent = KnowluFmt.date(p.d);
    var b = document.createElement("span");
    b.textContent = KnowluFmt.clock(p.d);
    c.appendChild(a);
    c.appendChild(b);
  }

  window.KnowluPanel = {
    add: function (kind, mod) { if (typeof kind === "string" && kind && mod && typeof mod.render === "function") { PANELS[kind] = mod; } },
    open: openPanel,
    close: closePanel,
    isOpen: function () { return !!OPEN_PANEL; }
  };
  window.KnowluSheets = {
    define: function (name, mod) { if (rankOf(name) >= 0 && mod && typeof mod.render === "function") { SHEETS[name] = mod; } },
    request: function (name, mod) {
      if (!Object.prototype.hasOwnProperty.call(SHEET_RANK, name)) { return false; }
      var m = mod || (Object.prototype.hasOwnProperty.call(SHEETS, name) ? SHEETS[name] : null);
      if (!m || typeof m.render !== "function") { return false; }
      if ((OPEN_SHEET && OPEN_SHEET.name === name) || WAITING.some(function (r) { return r.name === name; })) { return true; }
      WAITING.push({ name: name, mod: m });
      pump();
      return true;
    },
    close: closeSheet,
    isOpen: function () { return !!OPEN_SHEET; }
  };
  window.KnowluLayer = {
    show: function (mod) { if (mod && typeof mod.render === "function") { LAYER_MOD = mod; $("layer").hidden = true; syncLayer(); } },
    hide: function () { LAYER_MOD = null; var l = $("layer"); l.hidden = true; l.textContent = ""; }
  };
  window.KnowluSlots = {
    deck: function (el) {
      if (!el) { return; }
      if (typeof DECK !== "function") { el.textContent = ""; return; }
      try { DECK(el); } catch (e) { window.console.error(e); }
    },
    addDeck: function (fn) { if (typeof fn === "function") { DECK = fn; } }
  };
  window.KnowluShell = {
    routed: function (name) {
      ROUTE = name;
      markCurrent();
      closePanel();
      setStatusOpen(false);
      if (!$("side").matches(":hover")) { fold(); }
      pump();
      syncLayer();
    },
    status: drawStatus,
    start: function () {
      bindSidebar();
      bindStatus();
      bindPanel();
      document.addEventListener("keydown", onEscape);
      KnowluData.onPoll(drawStatus);
      KnowluData.on(function (st) {
        drawClasses(st);
        drawStatus();
        drawClock();
      });
    },
    bindWindow: bindWindow
  };
})();
