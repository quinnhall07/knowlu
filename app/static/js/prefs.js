/* Knowlu app: the preferences, applied before the first paint (v0.4.0 note D4).
   Loaded in <head>, blocking, before the first stylesheet, so the first frame already has the theme, the reading font,
   the text size and the rail. It keeps four of D4's six localStorage keys (the calendar's two are T11's) and no other
   storage at all.

     knowlu.theme   "system" (the default), "light" or "dark"; html[data-theme] always carries the resolved light or dark,
                    and a system choice follows the OS while the page is open
     knowlu.font    "default" or "dyslexic" (html[data-font="dyslexic"]: OpenDyslexic for every role but the wordmark)
     knowlu.text    "default" or "larger" (html[data-text="larger"]: the app drawn at 100% instead of 88%, D6)
     knowlu.pin     "1" when the sidebar is pinned open; otherwise it rests as the rail (html.rail)

   window.KnowluPrefs: theme(), setTheme(t), font(), setFont(f), text(), setText(t), pinned(), setPinned(on).
   setTheme also tells the window (getCurrentWindow().setTheme), so the frame Windows draws follows the page.
   It also marks <html> "booting" (the sidebar's contents wait for launch_state) and "no-motion" (nothing animates
   into its first place); boot.js lifts both. */
(function () {
  "use strict";
  var root = document.documentElement;
  var media = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;
  var THEMES = ["system", "light", "dark"];

  function get(key, fallback) {
    try { var v = window.localStorage.getItem(key); return v == null ? fallback : v; } catch (e) { return fallback; }
  }
  function put(key, value) {
    try { window.localStorage.setItem(key, value); } catch (e) { /* storage refused: the choice lasts this session */ }
  }
  function oneOf(v, list, fallback) { return list.indexOf(v) >= 0 ? v : fallback; }
  function resolved(t) { return t === "system" ? (media && media.matches ? "dark" : "light") : t; }
  function flag(name, on, value) { if (on) { root.setAttribute(name, value); } else { root.removeAttribute(name); } }

  var theme = oneOf(get("knowlu.theme", "system"), THEMES, "system");
  var font = oneOf(get("knowlu.font", "default"), ["default", "dyslexic"], "default");
  var text = oneOf(get("knowlu.text", "default"), ["default", "larger"], "default");
  var pinned = get("knowlu.pin", "0") === "1";

  root.setAttribute("data-theme", resolved(theme));
  flag("data-font", font === "dyslexic", "dyslexic");
  flag("data-text", text === "larger", "larger");
  root.classList.toggle("rail", !pinned);
  root.classList.add("no-motion", "booting");
  if (media && media.addEventListener) {
    media.addEventListener("change", function () { if (theme === "system") { root.setAttribute("data-theme", resolved(theme)); } });
  }

  window.KnowluPrefs = {
    theme: function () { return theme; },
    setTheme: function (t) {
      theme = oneOf(t, THEMES, "system");
      put("knowlu.theme", theme);
      root.setAttribute("data-theme", resolved(theme));
      try {
        window.__TAURI__.window.getCurrentWindow().setTheme(theme === "system" ? null : theme).catch(function () {});
      } catch (e) { /* no host window: the fixture harness */ }
    },
    font: function () { return font; },
    setFont: function (f) { font = oneOf(f, ["default", "dyslexic"], "default"); put("knowlu.font", font); flag("data-font", font === "dyslexic", "dyslexic"); },
    text: function () { return text; },
    setText: function (t) { text = oneOf(t, ["default", "larger"], "default"); put("knowlu.text", text); flag("data-text", text === "larger", "larger"); },
    pinned: function () { return pinned; },
    setPinned: function (on) { pinned = !!on; put("knowlu.pin", pinned ? "1" : "0"); root.classList.toggle("rail", !pinned); root.classList.remove("peek"); }
  };
  // A stored light or dark choice reaches the window's own frame at load too.
  if (theme !== "system") { window.KnowluPrefs.setTheme(theme); }
})();
