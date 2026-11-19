/* Knowlu app: the router (note D1, D2; plan §3). Hash routes #home, #tasks, #calendar, #classes, #class/<slug>, #lockin,
   #settings[/<section>]; anything else is Home. Only #center swaps: the sidebar, the band and the panel stay.

   KnowluRouter.add(name, mod)          a page registers itself: mod.mount(center, ctx), mod.update(state), mod.unmount().
                                        ctx = {arg, state, firstRun}: arg is the slug of #class/<slug> or the section of
                                        #settings/<section> ("" for none); state may be null (first run, or not yet read).
                                        update(state) runs on every newly painted revision, update(null) on a first-run answer.
   KnowluRouter.addSection(name, mod)   a Settings section registers itself; Settings finds it with section(name)
   KnowluRouter.section(name)           the section's module, else null
   KnowluRouter.view()                  the fixed route name now showing (telemetry's view, never a slug: I9)
   KnowluRouter.arg()                   the slug or section now showing
   KnowluRouter.go(route)               go to "tasks", "class/<slug>", "settings/data", ...
   KnowluRouter.start(center)           boot's, in the console only

   A route with no registered page (a placeholder's) leaves #center empty and throws nothing. Every lookup by a name
   that came from the address checks the table's own keys first, so "constructor" or "__proto__" never resolves.
   <html data-route="NAME"> says which page shows, for a page's own stylesheet (Lock in hides the sidebar with it).
   The tray's two globals live here: KNOWLU_OPEN_SETTINGS and KNOWLU_OPEN_REPORT. */
(function () {
  "use strict";
  var ROUTES = ["home", "tasks", "calendar", "classes", "class", "lockin", "settings"];
  /* the state view each page polls: only horizon views carry `list` (note D3) */
  var ALL_VIEW = ["tasks", "calendar", "classes", "class"];
  var KNOWN = Object.create(null);
  ROUTES.forEach(function (r) { KNOWN[r] = true; });
  var PAGES = Object.create(null);
  var SECTIONS = Object.create(null);
  var CENTER = null;
  var NAME = "home";
  var ARG = "";
  var CURRENT = null;
  var STARTED = false;
  var REDUCED = window.matchMedia ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;

  function parse(hash) {
    var raw = String(hash || "").replace(/^#/, "");
    var cut = raw.indexOf("/");
    var name = cut < 0 ? raw : raw.slice(0, cut);
    var arg = cut < 0 ? "" : raw.slice(cut + 1);
    if (!Object.prototype.hasOwnProperty.call(KNOWN, name)) { return { name: "home", arg: "" }; }
    try { arg = decodeURIComponent(arg); } catch (e) { arg = ""; }
    return { name: name, arg: arg };
  }
  function pageFor(name) {
    return Object.prototype.hasOwnProperty.call(PAGES, name) ? PAGES[name] : null;
  }
  function view() {
    return NAME;
  }
  function stateViewOf(name) { return ALL_VIEW.indexOf(name) >= 0 ? "all" : "today"; }

  function unmount() {
    var mod = CURRENT;
    CURRENT = null;
    if (mod && typeof mod.unmount === "function") {
      try { mod.unmount(); } catch (e) { window.console.error(e); }
    }
  }
  function mount(name, arg) {
    var mod = pageFor(name);
    CURRENT = null;
    if (!mod) {
      CENTER.textContent = "";
      return;
    }
    CENTER.textContent = "";
    CURRENT = mod;
    try {
      mod.mount(CENTER, { arg: arg, state: KnowluData.state(), firstRun: KnowluData.firstRun() });
    } catch (e) { window.console.error(e); }
    KnowluKit.watchSeen();
  }

  function change(r) {
    unmount();
    NAME = r.name;
    ARG = r.arg;
    document.documentElement.setAttribute("data-route", NAME);
    KnowluData.view(stateViewOf(NAME));
    mount(NAME, ARG);
    CENTER.scrollTop = 0;
    if (window.KnowluShell) { window.KnowluShell.routed(NAME, ARG); }
  }
  function apply() {
    var r = parse(location.hash);
    if (STARTED && r.name === NAME && r.arg === ARG) { return; }
    var first = !STARTED;
    STARTED = true;
    var root = document.documentElement;
    if (!first && document.startViewTransition && !(REDUCED && REDUCED.matches)) {
      root.classList.add("vt");
      var t = document.startViewTransition(function () { change(r); });
      t.finished.then(function () { root.classList.remove("vt"); }, function () { root.classList.remove("vt"); });
    } else {
      change(r);
    }
    ev("view_opened", null, "view");
  }
  function go(route) {
    var target = "#" + String(route || "home");
    if (location.hash === target) { apply(); } else { location.hash = target; }
  }

  KnowluData.on(function (st) {
    if (CURRENT && typeof CURRENT.update === "function") {
      try { CURRENT.update(st); } catch (e) { window.console.error(e); }
    }
    KnowluKit.watchSeen();
  });

  window.KnowluRouter = {
    add: function (name, mod) {
      if (!Object.prototype.hasOwnProperty.call(KNOWN, name) || !mod || typeof mod.mount !== "function") { return; }
      PAGES[name] = mod;
    },
    addSection: function (name, mod) {
      if (typeof name !== "string" || !name || !mod) { return; }
      SECTIONS[name] = mod;
    },
    section: function (name) {
      return Object.prototype.hasOwnProperty.call(SECTIONS, name) ? SECTIONS[name] : null;
    },
    view: view,
    arg: function () { return ARG; },
    go: go,
    start: function (center) {
      if (CENTER) { return; }
      CENTER = center;
      window.addEventListener("hashchange", apply);
      apply();
    }
  };

  /* the tray's two globals (tray.rs evaluates them by name) */
  window.KNOWLU_OPEN_SETTINGS = function () {
    if (!CENTER) { return; }
    go("settings");
  };
  window.KNOWLU_OPEN_REPORT = function () {
    if (!CENTER) { return; }
    go("settings/data");
    KnowluSheets.request("report");
  };
})();
