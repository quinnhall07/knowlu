/* The pilot's short form (index.html, section#pilot; css/price.css). A name, a school email and which of eight things
   the student has lived through go to a short Google Form, which emails Quinn so Quinn can write back.
   - Checking. A name and a school email that looks like one are needed; the list is optional. A wrong field gets its
     border and a sentence under it in the error color, and the first wrong field takes the focus.
   - Sending. One POST to the form's own action, with mode no-cors: the answer can't be read, so a fetch that
     resolves counts as sent and one that rejects (no network) asks the student to try again.
   - The honeypot (#pk-site) is hidden from people and screen readers. If it has text, the page shows the thank-you
     and sends nothing.
   - Sent. The form gives way to the thank-you, with the full application as the next step; the second way to the
     full application, at the foot of the steps, goes. A status line says the same to a screen reader, and the focus
     moves to the full application button.
   - Every link to the full application in the band carries the name and email once they are typed, so the student
     doesn't type them twice.
   Without this script (or without fetch) the form is a plain POST to the same address and ends on Google's thank-you
   page. Nothing is stored on the page. */
(function () {
  'use strict';

  var form = document.querySelector('[data-pf-form]');
  if (!form || !window.fetch || !window.URLSearchParams) return;

  var EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;
  var F_NAME = 'entry.2058486394', F_EMAIL = 'entry.1258614817', F_EXP = 'entry.1841072310';
  var P_NAME = 'entry.2017985301', P_EMAIL = 'entry.683863841';   /* the full application's prefill fields */

  var wrap = form.closest('[data-pf]');
  var band = form.closest('section') || document;
  var name = form.querySelector('[data-pf-name]');
  var email = form.querySelector('[data-pf-email]');
  var trap = form.querySelector('[data-pf-trap]');
  var send = form.querySelector('[data-pf-send]');
  var fail = form.querySelector('[data-pf-fail]');
  var done = wrap.querySelector('[data-pf-done]');
  var to = wrap.querySelector('[data-pf-to]');
  var say = wrap.querySelector('[data-pf-say]');
  var later = band.querySelector('[data-pf-later]');
  var fulls = Array.prototype.slice.call(band.querySelectorAll('[data-pf-full]'));
  var sendLabel = send.textContent;
  var sending = false;

  /* the page checks the fields itself, in words; with scripts off the browser's own checks stand */
  form.noValidate = true;

  /* ---------- the full application, with what was typed ---------- */
  fulls.forEach(function (a) { a.setAttribute('data-base', a.getAttribute('href')); });
  function prefill() {
    var n = name.value.trim(), e = email.value.trim();
    var add = (n ? '&' + P_NAME + '=' + encodeURIComponent(n) : '') + (EMAIL.test(e) ? '&' + P_EMAIL + '=' + encodeURIComponent(e) : '');
    fulls.forEach(function (a) { a.href = a.getAttribute('data-base') + add; });
  }

  /* ---------- checking ---------- */
  function problem(field) {
    var v = field.value.trim();
    if (field === name) return v ? '' : 'Enter your first and last name.';
    if (!v) return 'Enter your school email.';
    return EMAIL.test(v) ? '' : 'That email doesn’t look right. Check it and try again.';
  }
  function mark(field, text) {
    var msg = document.getElementById(field.getAttribute('aria-describedby'));
    if (text) field.setAttribute('aria-invalid', 'true'); else field.removeAttribute('aria-invalid');
    if (msg && msg.textContent !== text) msg.textContent = text;
  }
  [name, email].forEach(function (field) {
    field.addEventListener('input', function () {
      /* a field already marked wrong clears as soon as it is right; nothing new is marked while typing */
      if (field.getAttribute('aria-invalid') === 'true' && !problem(field)) mark(field, '');
      prefill();
    });
    field.addEventListener('change', prefill);
  });

  /* ---------- sending ---------- */
  function busy(on) {
    sending = on;
    send.textContent = on ? 'Sending…' : sendLabel;
    if (on) send.setAttribute('aria-disabled', 'true'); else send.removeAttribute('aria-disabled');
    if (on) form.setAttribute('aria-busy', 'true'); else form.removeAttribute('aria-busy');
  }
  function sent() {
    busy(false);
    prefill();
    /* the address, with a place to break after its @ and each dot, so a long one wraps cleanly on a phone */
    to.textContent = '';
    email.value.trim().split(/([@.])/).forEach(function (part) {
      if (!part) return;
      to.appendChild(document.createTextNode(part));
      if (part === '@' || part === '.') to.appendChild(document.createElement('wbr'));
    });
    form.hidden = true;
    done.hidden = false;
    if (later) later.hidden = true;
    var next = done.querySelector('[data-pf-full]');
    say.textContent = ['.pk-thx', '.pk-to', '.pk-next'].map(function (s) { return done.querySelector(s).textContent; }).join(' ');
    if (done.scrollIntoView) done.scrollIntoView({ block: 'nearest' });
    if (next) next.focus({ preventScroll: true });
    /* the dots behind the band ease back and send one ripple out from the thank-you (js/dots.js) */
    document.dispatchEvent(new CustomEvent('kl:joined', { detail: { form: form, at: done } }));
  }
  function failed() {
    busy(false);
    fail.textContent = 'That didn’t send. Check your connection and try again.';
  }

  form.addEventListener('submit', function (e) {
    e.preventDefault();
    if (sending) return;
    fail.textContent = '';
    var first = null;
    [name, email].forEach(function (field) {
      var p = problem(field);
      mark(field, p);
      if (p && !first) first = field;
    });
    if (first) { first.focus(); return; }
    if (trap && trap.value) { sent(); return; }

    var body = new URLSearchParams();
    body.append(F_NAME, name.value.trim());
    body.append(F_EMAIL, email.value.trim());
    Array.prototype.forEach.call(form.querySelectorAll('input[type="checkbox"][name="' + F_EXP + '"]:checked'), function (c) {
      body.append(F_EXP, c.value);
    });
    body.append('pageHistory', '0,1');
    busy(true);
    window.fetch(form.action, { method: 'POST', mode: 'no-cors', body: body }).then(sent, failed);
  });

  prefill();
})();
