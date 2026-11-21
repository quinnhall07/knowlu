/* Knowlu site: the theme switch, the nav, every waitlist form and building on scroll (the logo strip is js/strip.js).
   Nothing is sent from the page and nothing is stored except the theme (localStorage "knowlu-theme"); a waitlist request goes out from the visitor's own email app. */
(function () {
  'use strict';

  var root = document.documentElement;
  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
  var darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
  function each(sel, fn) { Array.prototype.forEach.call(document.querySelectorAll(sel), fn); }

  /* ---------- theme ---------- */
  function currentTheme() {
    var t = root.dataset.theme;
    return t === 'light' || t === 'dark' ? t : (darkQuery.matches ? 'dark' : 'light');
  }
  function labelTheme() {
    each('[data-theme-toggle]', function (b) {
      var next = currentTheme() === 'dark' ? 'Light' : 'Dark';
      b.textContent = next;
      b.setAttribute('aria-label', 'Switch to the ' + next.toLowerCase() + ' theme');
    });
  }
  each('[data-theme-toggle]', function (b) {
    b.addEventListener('click', function () {
      var next = currentTheme() === 'dark' ? 'light' : 'dark';
      root.dataset.theme = next;
      try { localStorage.setItem('knowlu-theme', next); } catch (e) { /* storage blocked: the theme lasts this visit */ }
      labelTheme();
    });
  });
  if (darkQuery.addEventListener) darkQuery.addEventListener('change', labelTheme);
  labelTheme();

  /* ---------- waitlist forms ----------
     A submit opens the visitor's own email app with the request to hello@knowlu.com; the page itself sends nothing. */
  var EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;
  var forms = Array.prototype.slice.call(document.querySelectorAll('form[data-wl]'));

  forms.forEach(function (form, i) {
    var w = {
      input: form.querySelector('input[type="email"]'),
      btn: form.querySelector('button[type="submit"]'),
      msg: form.querySelector('.wl-msg')
    };
    if (!w.input || !w.btn || !w.msg) return;
    w.idle = w.msg.textContent;
    w.msg.id = w.msg.id || 'wl-msg-' + i;
    w.input.setAttribute('aria-describedby', w.msg.id);
    form._wl = w;

    w.input.addEventListener('input', function () { if (form.classList.contains('err')) clearError(form); });
    form.addEventListener('submit', function (e) {
      e.preventDefault();
      submit(form);
    });
  });

  function clearError(form) {
    var w = form._wl;
    form.classList.remove('err');
    w.input.removeAttribute('aria-invalid');
    w.msg.textContent = w.idle;
  }
  function showError(form, text) {
    var w = form._wl;
    form.classList.add('err');
    w.input.setAttribute('aria-invalid', 'true');
    w.msg.textContent = text;
    w.input.focus();
  }
  function submit(form) {
    var w = form._wl, value = w.input.value.trim();
    if (!value) return showError(form, 'Enter your email to join.');
    if (!EMAIL.test(value)) return showError(form, 'That email doesn’t look right. Check it and try again.');
    clearError(form);
    location.href = 'mailto:hello@knowlu.com?subject=' + encodeURIComponent('Knowlu early access') +
      '&body=' + encodeURIComponent('Please add me to Knowlu early access: ' + value);
    w.msg.textContent = 'Send it and you’re on the list. Nothing opened? Write to hello@knowlu.com.';
  }

  /* "Get early access" in the nav: up to the hero form, focus in its email field */
  each('[data-focus-join]', function (link) {
    link.addEventListener('click', function (e) {
      var target = document.getElementById('join');
      if (!target || !target._wl) return;
      e.preventDefault();
      window.scrollTo({ top: 0, behavior: reduced.matches ? 'auto' : 'smooth' });
      target._wl.input.focus({ preventScroll: true });
    });
  });

  /* ---------- nav: clear over the top of the hero; solid from just before the big wordmark would slide under it ---------- */
  var nav = document.querySelector('[data-nav]');
  var heroLogo = document.querySelector('[data-hero] .lockup .wm-xl');
  if (nav && heroLogo) {
    var queued = false;
    var check = function () {
      queued = false;
      nav.classList.toggle('over', heroLogo.getBoundingClientRect().top > nav.offsetHeight + 20);
    };
    var queue = function () { if (!queued) { queued = true; window.requestAnimationFrame(check); } };
    window.addEventListener('scroll', queue, { passive: true });
    window.addEventListener('resize', queue);
    window.addEventListener('load', queue);
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(queue);
    check();
  }

  /* ---------- the phone menu: Menu shows and hides the section links under the bar ----------
     It closes on Escape (focus goes back to Menu), on a link, on a tap or click outside the bar, when focus leaves
     the bar, and when the screen grows wide enough for the links to sit in the bar (css/base.css, 760px). */
  var menuBtn = document.querySelector('[data-nav-menu]');
  var menu = menuBtn && document.getElementById(menuBtn.getAttribute('aria-controls'));
  if (nav && menuBtn && menu) {
    var isOpen = function () { return menuBtn.getAttribute('aria-expanded') === 'true'; };
    var setMenu = function (open, refocus) {
      if (open === isOpen()) return;
      menuBtn.setAttribute('aria-expanded', String(open));
      nav.classList.toggle('is-open', open);
      if (!open && refocus) menuBtn.focus();
    };
    menuBtn.addEventListener('click', function () { setMenu(!isOpen()); });
    menu.addEventListener('click', function (e) { if (e.target.closest('a')) setMenu(false); });
    document.addEventListener('click', function (e) { if (isOpen() && !nav.contains(e.target)) setMenu(false); });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && isOpen()) { e.preventDefault(); setMenu(false, true); }
    });
    nav.addEventListener('focusout', function (e) { if (e.relatedTarget && !nav.contains(e.relatedTarget)) setMenu(false); });
    var wide = window.matchMedia('(min-width: 760px)');
    var onWide = function () { if (wide.matches) setMenu(false); };
    if (wide.addEventListener) wide.addEventListener('change', onWide); else if (wide.addListener) wide.addListener(onWide);
  }

  /* ---------- building on scroll: .is-in once, at about 30% in view (or 30% of the screen for a tall one), then kl:in ---------- */
  var builds = Array.prototype.slice.call(document.querySelectorAll('[data-build]'));
  function arrive(el) {
    if (el.classList.contains('is-in') && el._klIn) return;
    el.classList.add('is-in');
    el._klIn = true;
    el.dispatchEvent(new CustomEvent('kl:in', { bubbles: true }));
  }
  if (reduced.matches) builds.forEach(function (el) { el.classList.add('is-in'); });
  if (builds.length && 'IntersectionObserver' in window) {
    var bio = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (!e.isIntersecting) return;
        var vh = (e.rootBounds && e.rootBounds.height) || window.innerHeight;
        if (e.intersectionRatio >= 0.3 || e.intersectionRect.height >= vh * 0.3) {
          arrive(e.target);
          bio.unobserve(e.target);
        }
      });
    }, { threshold: [0, 0.1, 0.2, 0.3, 0.5] });
    builds.forEach(function (el) { bio.observe(el); });
  } else {
    builds.forEach(arrive);
  }
})();
