/* Knowlu app: the launch's account check and the upgrade sheet (note §5 "Sign-in, upgrade overlay, subscribe"; plan §5
   T20). Settings draws Account, Membership and Delete my data itself (T16); this file draws none of them.

   Once a launch, on the console only (KnowluBoot.onConsole), account_status is asked. An install that predates accounts
   (needs_account) gets the `upgrade` sheet, upgraded in place: never re-onboarded and never asked about a folder; it
   ends in attach_account, which writes the vault's cloud config beside the files already there. A signed-in vault is
   asked about privacy instead (KnowluReconsent.ask, reconsent.js).

   The sheet never traps anyone behind it (spec §5.1, D4): Not now and Escape dismiss it for the session, a sign-in that
   cannot reach the account service stands it down for the session, and the shell holds it back while Settings is the
   route. Two doors, as the wizard has: Continue with Google, then an emailed code; no password. Subscribe appears only
   for an account the service does not call entitled, opens the payment page and waits for the entitlement; it names
   no price (A2).

   window.KnowluAccount.after(fn) runs fn once the launch's account and privacy check has settled (at once if it has):
   mail's first open waits on it (M10). */
(function () {
  "use strict";

  /* account.rs's UNREACHABLE: every transport failure there reads "{UNREACHABLE} (…)" */
  var UNREACHABLE = "the account service could not be reached";
  var META = "Your work stays exactly where it is. Sign in and Knowlu picks up where it left off. Today's page is underneath and still works; ranking never needed an account.";

  var ACCOUNT = { checked: false, waiting: [] };
  var UPGRADE_DISMISSED = false;     /* Not now or Escape, for this session */
  var UPGRADE_UNREACHABLE = false;   /* the account service could not be reached, for this session */
  var UP_BUSY = false;               /* a sign-in is in flight: two presses are one browser tab */
  var UP = { root: null, api: null, done: false };
  var UPGRADE_SHEET = { render: renderUpgrade, close: onUpgradeClosed };

  function el(id) { return document.getElementById(id); }
  function say(text) {
    var e = el("up-error");
    if (e) { e.textContent = text || ""; }
  }

  /* ---- the entitlement: ok plus an active or trialing status; anything else, a refusal included, is not ---- */
  function checkEntitlement() {
    return KnowluData.call("entitlement_now", {}).then(function (r) {
      var yes = !!(r && r.ok && (r.status === "active" || r.status === "trialing"));
      return { entitled: yes, plan: yes && r.plan ? String(r.plan) : "" };
    });
  }
  function checkEntitled() {
    return checkEntitlement().then(function (r) { return r.entitled; });
  }

  /* ---- the launch's check ---- */
  function settled() {
    ACCOUNT.checked = true;
    var waiting = ACCOUNT.waiting;
    ACCOUNT.waiting = [];
    waiting.forEach(function (fn) {
      try { fn(); } catch (e) { window.console.error(e); }
    });
  }
  function checkAccount() {
    return KnowluData.call("account_status", {}).then(function (s) {
      maybeUpgrade(s);
      return window.KnowluReconsent ? KnowluReconsent.ask(s) : null;
    }).then(settled, settled);
  }
  function maybeUpgrade(s) {
    if (UPGRADE_DISMISSED || UPGRADE_UNREACHABLE || !s || !s.ok || !s.needs_account) { return; }
    KnowluSheets.request("upgrade", UPGRADE_SHEET);
  }

  /* ---- the sheet ---- */
  function upgradeHtml() {
    return '<div class="sheet-in sh up" aria-labelledby="up-h"><h2 class="sh-h" id="up-h" tabindex="-1">Knowlu now needs an account</h2>' +
      '<p class="sh-p">' + h(META) + "</p>" +
      '<div class="sh-checks">' +
      '<label class="sh-check"><input type="checkbox" id="up-18"><span class="sh-box" aria-hidden="true"></span><span>I am 18 or older</span></label>' +
      '<label class="sh-check"><input type="checkbox" id="up-terms"><span class="sh-box" aria-hidden="true"></span><span>I accept the terms and the privacy policy</span></label>' +
      '<p class="sh-hint">Read the <button class="sh-link" type="button" data-policy="terms">terms</button> and the <button class="sh-link" type="button" data-policy="privacy">privacy policy</button>.</p>' +
      "</div>" +
      '<div class="sh-doors"><button class="btn" type="button" id="up-google">Continue with Google</button>' +
      '<p class="sh-or">or get a code by email</p>' +
      '<div class="sh-line"><label class="sh-l" for="up-email">Email</label><input class="field" type="email" id="up-email" autocomplete="email" spellcheck="false">' +
      '<button class="obtn" type="button" id="up-magic">Email me a code</button></div>' +
      '<div class="sh-line" id="up-code-row" hidden><label class="sh-l" for="up-code">Code</label><input class="field" type="text" id="up-code" inputmode="numeric" autocomplete="one-time-code" spellcheck="false">' +
      '<button class="obtn" type="button" id="up-code-go">Sign in with the code</button></div>' +
      '<button class="btn" type="button" id="up-subscribe" hidden>Subscribe</button></div>' +
      '<p class="sh-err" id="up-error" role="status"></p>' +
      '<div class="sh-acts"><button class="tbtn decline" type="button" id="up-later">Not now</button></div></div>';
  }
  function renderUpgrade(host, api) {
    UP.api = api;
    UP.done = false;
    host.innerHTML = upgradeHtml();
    UP.root = host.querySelector(".up");
    UP.root.addEventListener("click", onUpgradeClick);
    el("up-h").focus({ preventScroll: true });
  }
  /* the shell runs this however the sheet closed: Not now, Escape, a stand-down or a finished upgrade */
  function onUpgradeClosed() {
    if (!UP.done) { UPGRADE_DISMISSED = true; }
    UP.root = null;
    UP.api = null;
    UP_BUSY = false;
  }
  function closeUpgrade() {
    if (UP.api) { UP.api.close(); }
  }
  /* a sign-in that could not reach the service stands the sheet down for this session; today's page is underneath */
  function upgradeUnreachable() {
    UPGRADE_UNREACHABLE = true;
    say("");
    closeUpgrade();
  }
  function upBusy(on) {
    UP_BUSY = on;
    ["up-google", "up-magic", "up-code-go"].forEach(function (id) {
      var b = el(id);
      if (b) { b.disabled = on; }
    });
  }
  function ticked() { return !!(el("up-18") && el("up-18").checked && el("up-terms") && el("up-terms").checked); }
  function refusedForGood(r) { return !r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0; }

  /* what both doors do once a session exists: ask first, and show Subscribe only on a not-entitled answer. Not now or
     Escape during the browser round trip closed the sheet: a late answer then does nothing. */
  function afterUpgradeSignIn() {
    if (!UP.root) { return null; }
    say("");
    el("up-code-row").hidden = true;
    return checkEntitled().then(function (yes) {
      if (!UP.root) { return null; }
      if (!yes) { el("up-subscribe").hidden = false; return null; }
      return finishUpgrade();
    });
  }
  /* an account the service already calls entitled goes straight through; otherwise the payment page, then a wait */
  function subscribe() {
    checkEntitled().then(function (yes) {
      if (yes) { return finishUpgrade(); }
      return KnowluData.call("open_checkout", { plan: "monthly" }).then(function (r) {
        if (!r.ok) { el("up-error").textContent = r.error || UNREACHABLE; return; }
        say("");
        var tries = 0;
        var tick = function () {
          if (!UP.root) { return; }
          tries += 1;
          checkEntitled().then(function (yes2) {
            if (!UP.root) { return; }
            if (yes2) { finishUpgrade(); return; }
            if (tries < 40) { setTimeout(tick, 3000); }
            else { say("Still not subscribed. When the payment page is done, press Subscribe again."); }
          });
        };
        setTimeout(tick, 3000);
      });
    });
  }
  function finishUpgrade() {
    return KnowluData.call("attach_account", {}).then(function (r) {
      if (!r.ok) { say(r.error || UNREACHABLE); return; }
      UP.done = true;
      closeUpgrade();
      KnowluData.refresh();
    });
  }

  function onUpgradeClick(e) {
    var policy = e.target.closest("[data-policy]");
    if (policy) {
      e.preventDefault();
      KnowluData.call("open_policy", { which: policy.getAttribute("data-policy") });
      return;
    }
    if (e.target.closest("#up-later")) {
      UPGRADE_DISMISSED = true;
      closeUpgrade();
      return;
    }
    if (e.target.closest("#up-google")) {
      if (UP_BUSY) { return; }
      if (!ticked()) { say("Tick both boxes to create an account."); return; }
      upBusy(true);
      say("Finish signing in, in your browser…");
      KnowluData.call("google_sign_in", { ageAttested: el("up-18").checked }).then(function (r) {
        upBusy(false);
        if (refusedForGood(r)) { upgradeUnreachable(); return null; }
        if (!r.ok) { say(r.error); return null; }
        return afterUpgradeSignIn();
      });
      return;
    }
    if (e.target.closest("#up-magic")) {
      if (UP_BUSY) { return; }
      if (!ticked()) { say("Tick both boxes to create an account."); return; }
      upBusy(true);
      KnowluData.call("send_magic_link", { email: el("up-email").value.trim(), ageAttested: el("up-18").checked }).then(function (r) {
        upBusy(false);
        if (!UP.root) { return; }
        if (refusedForGood(r)) { upgradeUnreachable(); return; }
        say(r.ok ? "We emailed you a code. Type it below." : r.error);
        el("up-code-row").hidden = !r.ok;
        if (r.ok) { el("up-code").focus(); }
      });
      return;
    }
    if (e.target.closest("#up-code-go")) {
      if (UP_BUSY) { return; }
      upBusy(true);
      KnowluData.call("verify_email_code", { email: el("up-email").value.trim(), code: el("up-code").value.trim() }).then(function (r) {
        upBusy(false);
        if (refusedForGood(r)) { upgradeUnreachable(); return null; }
        if (!r.ok) { say(r.error); return null; }
        return afterUpgradeSignIn();
      });
      return;
    }
    if (e.target.closest("#up-subscribe")) { subscribe(); }
  }

  window.KnowluAccount = {
    after: function (fn) {
      if (typeof fn !== "function") { return; }
      if (ACCOUNT.checked) { fn(); } else { ACCOUNT.waiting.push(fn); }
    }
  };

  KnowluBoot.onConsole(checkAccount);
})();
