/* Knowlu app: the boot (note §2, §3; plan §3). On DOMContentLoaded it asks launch_state which of three things this
   window is: the console over a resolved profile, the wizard (no profile yet) or the picker (more than one). The console
   path binds the console's listeners and starts the poll; the wizard and the picker render into #boot and nothing of the
   console runs. Any other mode is the picker; if launch_state cannot be read, the console boots (the shipped page's
   fallbacks).

   ui-ready: emitted once, with a plain window.__TAURI__.event.emit (never emitTo: main.rs listens with app.listen), after
   the theme class (prefs.js, in the head), DOMContentLoaded and one requestAnimationFrame raced with 300 ms; it is what
   tells main.rs to show the window (the one show path, note §3, N9).

   KnowluBoot.onConsole(fn)   a carried surface's launch check (week setup, updates, the account, mail's first open) runs
                              once the console has booted, in registration order; never on the wizard or picker path
   KnowluBoot.mode()          "console", "wizard", "picker", or "" before launch_state answers
   The wizard and the picker register themselves as window.KnowluWizard.start(launch, host) and
   window.KnowluPicker.render(launch, host); with neither registered, #boot stays empty. */
(function () {
  "use strict";
  var READY_SENT = false;
  var MODE = "";
  var HOOKS = [];
  var root = document.documentElement;

  function uiReady() {
    if (READY_SENT) { return; }
    READY_SENT = true;
    if (window.__TAURI__ && window.__TAURI__.event) {
      Promise.resolve(window.__TAURI__.event.emit("ui-ready")).catch(function () { /* main.rs's 3 s timer shows the window anyway */ });
    }
  }
  function afterFirstFrame(fn) {
    var done = false;
    var once = function () { if (!done) { done = true; fn(); } };
    window.requestAnimationFrame(once);
    setTimeout(once, 300);
  }

  function showBoot(mode) {
    MODE = mode;
    root.classList.add("mode-boot");
    root.classList.remove("booting");
    var host = document.getElementById("boot");
    host.hidden = false;
    host.textContent = "";
    return host;
  }
  function bootConsole() {
    MODE = "console";
    root.classList.remove("booting", "mode-boot");
    KnowluShell.start();
    KnowluShell.bindWindow();
    KnowluKit.startLook();
    KnowluRouter.start(document.getElementById("center"));
    KnowluData.start();
    HOOKS.forEach(function (fn) {
      try { fn(); } catch (e) { window.console.error(e); }
    });
  }
  function bootWizard(launch) {
    var host = showBoot("wizard");
    KnowluShell.bindWindow();
    if (window.KnowluWizard && typeof window.KnowluWizard.start === "function") {
      try { window.KnowluWizard.start(launch, host); } catch (e) { window.console.error(e); }
    }
  }
  function bootPicker(launch) {
    var host = showBoot("picker");
    KnowluShell.bindWindow();
    if (window.KnowluPicker && typeof window.KnowluPicker.render === "function") {
      try { window.KnowluPicker.render(launch, host); } catch (e) { window.console.error(e); }
    }
  }

  window.KnowluBoot = {
    onConsole: function (fn) {
      if (typeof fn !== "function") { return; }
      if (MODE === "console") { fn(); } else { HOOKS.push(fn); }
    },
    mode: function () { return MODE; }
  };

  document.addEventListener("DOMContentLoaded", function () {
    KnowluData.call("launch_state", {}).then(function (l) {
      if (!l || !l.mode || l.mode === "console") { bootConsole(); }
      else if (l.mode === "wizard") { bootWizard(l); }
      else { bootPicker(l); }
      afterFirstFrame(function () {
        uiReady();
        afterFirstFrame(function () { root.classList.remove("no-motion"); });
      });
    });
  });
})();
