/* Joined page: the state (?state=expired for an old link, otherwise confirmed) and the theme switch. Nothing is stored but the theme, and nothing is sent. */
(function () {
  'use strict';

  var root = document.documentElement;
  var params = new URLSearchParams(location.search);
  function each(sel, fn) { Array.prototype.forEach.call(document.querySelectorAll(sel), fn); }

  /* confirmed, or an old link */
  function show(state) {
    each('[data-jn-state]', function (el) { el.hidden = el.getAttribute('data-jn-state') !== state; });
    var h = document.querySelector('[data-jn-state="' + state + '"] h1');
    document.title = (h ? h.textContent : 'Knowlu') + ' · Knowlu';
  }
  var st = params.get('state');
  show(st === 'expired' ? st : 'ok');

  /* theme: the same switch and saved choice as the home page */
  var darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
  function current() { var t = root.dataset.theme; return t === 'light' || t === 'dark' ? t : (darkQuery.matches ? 'dark' : 'light'); }
  function setTheme(t, persist) {
    root.dataset.theme = t;
    if (persist) { try { localStorage.setItem('knowlu-theme', t); } catch (e) { /* this visit only */ } }
    each('[data-theme-toggle]', function (b) {
      var next = t === 'dark' ? 'Light' : 'Dark';
      b.textContent = next;
      b.setAttribute('aria-label', 'Switch to ' + next.toLowerCase() + ' theme');
    });
  }
  setTheme(current(), false);
  each('[data-theme-toggle]', function (b) { b.addEventListener('click', function () { setTheme(current() === 'dark' ? 'light' : 'dark', true); }); });
})();
